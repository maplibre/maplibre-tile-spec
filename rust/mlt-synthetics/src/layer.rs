use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use mlt_core::GeometryValues;
use mlt_core::encoder::{
    ColumnKind, Encoder, EncoderConfig, ExplicitEncoder, IdWidth, IntEncoder, StagedId,
    StagedLayer, StagedProperty, StagedSharedDict, StrEncoding, StreamCtx, VertexBufferType,
};
use mlt_core::geo_types::{Coord, Geometry};
use mlt_core::wire::{LengthType, OffsetType, StreamType};

use crate::writer::{SynthErr, SynthResult, SynthWriter};

/// Create a layer with all geometry encoders set to `VarInt`.
pub fn geo_varint() -> Layer {
    Layer::new(IntEncoder::varint())
}

/// Create a layer with geometry encoders set to `VarInt` and RLE for the meta stream.
pub fn geo_varint_with_rle() -> Layer {
    Layer::new(IntEncoder::varint()).meta(IntEncoder::rle_varint())
}

/// Create a layer with all geometry encoders set to `FastPFOR`.
pub fn geo_fastpfor() -> Layer {
    Layer::new(IntEncoder::fastpfor())
}

/// Per-property encoding specification.
#[derive(Clone)]
enum PropConfig {
    /// Int/Bool/Float: `enc` is used for integer streams; Bool/Float auto-detect from type.
    Scalar(IntEncoder),
    /// String FSST encoding.
    StrFsst {
        sym_lengths: IntEncoder,
        dict_lengths: IntEncoder,
    },
    /// String FSST+Dictionary encoding.
    StrFsstDict {
        sym_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
    },
    /// String Dictionary (plain dict) encoding.
    StrDict {
        string_lengths: IntEncoder,
        offsets: IntEncoder,
    },
    /// Shared dictionary: `StrEncoding` for the corpus, per-suffix `IntEncoder` for offsets.
    SharedDict {
        dict_encoding: StrEncoding,
        item_encs: Vec<(String, IntEncoder)>,
    },
}

impl PropConfig {
    fn str_encoding(&self) -> StrEncoding {
        match self {
            Self::Scalar(_) => StrEncoding::Plain,
            Self::StrFsst { .. } => StrEncoding::Fsst,
            Self::StrFsstDict { .. } => StrEncoding::FsstDict,
            Self::StrDict { .. } => StrEncoding::Dict,
            Self::SharedDict { dict_encoding, .. } => *dict_encoding,
        }
    }

    /// Resolve the integer encoder for a property stream using wire `StreamType`.
    fn int_enc_for_stream_ctx(&self, ctx: &StreamCtx<'_>) -> IntEncoder {
        use LengthType as LT;
        use OffsetType as OT;
        use StreamType as ST;
        match self {
            Self::Scalar(e) => *e,
            Self::StrFsst {
                sym_lengths,
                dict_lengths,
            } => match ctx.stream_type {
                ST::Length(LT::Symbol) => *sym_lengths,
                _ => *dict_lengths,
            },
            Self::StrFsstDict {
                sym_lengths,
                dict_lengths,
                offsets,
            } => match ctx.stream_type {
                ST::Length(LT::Symbol) => *sym_lengths,
                ST::Offset(OT::String) => *offsets,
                _ => *dict_lengths,
            },
            Self::StrDict {
                string_lengths,
                offsets,
            } => match ctx.stream_type {
                ST::Offset(OT::String) => *offsets,
                _ => *string_lengths,
            },
            Self::SharedDict { item_encs, .. } => {
                // sub is the item suffix
                item_encs
                    .iter()
                    .find(|(k, _)| k == ctx.subname)
                    .map(|(_, e)| *e)
                    .or_else(|| item_encs.first().map(|(_, e)| *e))
                    .unwrap_or_else(IntEncoder::varint)
            }
        }
    }
}

/// Layer builder for synthetic tile generation.
#[derive(Clone)]
pub struct Layer {
    /// Default encoder for all geometry streams.
    default_geo_enc: IntEncoder,
    /// Per-stream overrides; key is the stream name (e.g. `"meta"`, `"rings"`).
    geo_stream_overrides: HashMap<&'static str, IntEncoder>,
    vertex_buffer_type: VertexBufferType,
    tessellate: bool,
    /// When `true`, emit a presence stream even for all-present columns.
    force_presence: bool,
    /// Geometry stream names that must be written even when their data is empty.
    /// See [`ExplicitEncoder::force_stream`] for details.
    force_empty_streams: HashSet<&'static str>,
    geometry_items: Vec<Geometry<i32>>,
    props: Vec<(StagedProperty, PropConfig)>,
    extent: Option<u32>,
    ids: Option<(Vec<Option<u64>>, IdWidth, IntEncoder)>,
}

impl Layer {
    fn new(default_enc: IntEncoder) -> Self {
        Self {
            default_geo_enc: default_enc,
            geo_stream_overrides: HashMap::new(),
            vertex_buffer_type: VertexBufferType::Vec2,
            tessellate: false,
            force_presence: false,
            force_empty_streams: HashSet::new(),
            geometry_items: vec![],
            props: vec![],
            extent: None,
            ids: None,
        }
    }

    #[must_use]
    pub fn meta(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("meta", e);
        self
    }
    #[must_use]
    pub fn rings(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("rings", e);
        self
    }
    #[must_use]
    pub fn rings2(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("rings2", e);
        self
    }
    #[must_use]
    pub fn no_rings(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("no_rings", e);
        self
    }
    #[must_use]
    pub fn parts_ring(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("parts_ring", e);
        self
    }
    #[must_use]
    pub fn vertex_offsets(mut self, e: IntEncoder) -> Self {
        self.geo_stream_overrides.insert("vertex_offsets", e);
        self
    }
    #[must_use]
    pub fn vertex_buffer_type(mut self, v: VertexBufferType) -> Self {
        self.vertex_buffer_type = v;
        self
    }
    #[must_use]
    pub fn tessellate(mut self) -> Self {
        self.tessellate = true;
        self
    }

    /// Force a geometry stream to be written even when its data is empty.
    ///
    /// `name` is the geometry stream name as used internally by the encoder
    /// (e.g. `"triangles_indexes"`, `"geometries"`, `"rings"`, …).
    ///
    /// Useful for producing byte-for-byte output that matches Java's encoder when a
    /// normally-empty stream must still appear in the wire format.
    #[must_use]
    pub fn force_empty_stream(mut self, name: &'static str) -> Self {
        self.force_empty_streams.insert(name);
        self
    }

    #[must_use]
    pub fn geo(mut self, geometry: impl Into<Geometry<i32>>) -> Self {
        self.geometry_items.push(geometry.into());
        self
    }

    #[must_use]
    pub fn geos<T: Into<Geometry<i32>>, I: IntoIterator<Item = T>>(
        mut self,
        geometries: I,
    ) -> Self {
        for g in geometries {
            self = self.geo(g.into());
        }
        self
    }

    /// Add a bool, integer, or float property.
    ///
    /// `enc` is used for integer stream encoding; Bool and Float columns ignore it.
    #[must_use]
    pub fn add_prop(mut self, enc: IntEncoder, prop: StagedProperty) -> Self {
        self.props.push((prop, PropConfig::Scalar(enc)));
        self
    }

    /// Add an FSST-compressed string property.
    #[must_use]
    pub fn add_prop_str_fsst(
        mut self,
        sym_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        prop: StagedProperty,
    ) -> Self {
        self.props.push((
            prop,
            PropConfig::StrFsst {
                sym_lengths,
                dict_lengths,
            },
        ));
        self
    }

    /// Add a Dictionary (plain dict) string property.
    #[must_use]
    pub fn add_prop_str_dict(
        mut self,
        string_lengths: IntEncoder,
        offsets: IntEncoder,
        prop: StagedProperty,
    ) -> Self {
        self.props.push((
            prop,
            PropConfig::StrDict {
                string_lengths,
                offsets,
            },
        ));
        self
    }

    /// Add an FSST+Dictionary string property.
    #[must_use]
    pub fn add_prop_str_fsst_dict(
        mut self,
        sym_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
        prop: StagedProperty,
    ) -> Self {
        self.props.push((
            prop,
            PropConfig::StrFsstDict {
                sym_lengths,
                dict_lengths,
                offsets,
            },
        ));
        self
    }

    /// Add a shared dictionary column.
    #[must_use]
    pub fn add_shared_dict(mut self, shared_dict: SharedDict) -> Self {
        let dict_encoding = shared_dict.dict_encoding;
        let item_encs: Vec<(String, IntEncoder)> = shared_dict
            .items
            .iter()
            .map(|(suffix, enc, _, _)| (suffix.clone(), *enc))
            .collect();
        let is_optional_flags: Vec<bool> =
            shared_dict.items.iter().map(|(_, _, _, f)| *f).collect();
        let mut dict = StagedSharedDict::new(
            shared_dict.name,
            shared_dict
                .items
                .into_iter()
                .map(|(suffix, _, vals, _)| (suffix, vals)),
        )
        .expect("shared dict builder should be valid");
        for (item, is_optional) in dict.items.iter_mut().zip(is_optional_flags) {
            if is_optional {
                item.set_presence(true);
            }
        }
        self.props.push((
            StagedProperty::SharedDict(dict),
            PropConfig::SharedDict {
                dict_encoding,
                item_encs,
            },
        ));
        self
    }

    /// Encode and then either verify against the reference dir (non-rust files) or write to the
    /// output dir (`-rust`-suffixed files). Delegates to [`SynthWriter::write`].
    ///
    /// When `force_empty_streams` is non-empty, also emits a `_ns` ("no forced stream")
    /// sibling — but only when removing the forced-empty-stream flag **actually changes the
    /// encoded output**.  For some geometry configurations (e.g. Multi* types where the
    /// GEOMETRIES stream is already non-empty) the flag is a no-op; emitting the sibling in
    /// those cases would produce duplicate MLT files and fail the uniqueness check.
    pub fn write(self, w: &mut SynthWriter, name: impl AsRef<str>) {
        if !self.force_empty_streams.is_empty() {
            let forced_bytes = self.clone().encode_to_bytes().ok();
            let mut ns_layer = self.clone();
            ns_layer.force_empty_streams.clear();
            let ns_bytes = ns_layer.clone().encode_to_bytes().ok();
            if forced_bytes != ns_bytes {
                let name = if let Some(prefix) = name.as_ref().strip_suffix("-rust") {
                    format!("{prefix}_ns-rust")
                } else {
                    format!("{}_ns", name.as_ref())
                };
                w.write(ns_layer, name);
            }
        }
        w.write(self, name);
    }

    #[must_use]
    pub fn extent(mut self, extent: u32) -> Self {
        self.extent = Some(extent);
        self
    }

    /// Set feature IDs with explicit encoding.
    #[must_use]
    pub fn ids(mut self, ids: Vec<Option<u64>>, id_width: IdWidth, int_enc: IntEncoder) -> Self {
        self.ids = Some((ids, id_width, int_enc));
        self
    }

    pub fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    pub fn encode_to_bytes(self) -> SynthResult<Vec<u8>> {
        let Self {
            default_geo_enc,
            geo_stream_overrides,
            vertex_buffer_type,
            tessellate,
            force_presence,
            force_empty_streams,
            geometry_items,
            props,
            extent,
            ids,
        } = self;

        let enc_cfg = EncoderConfig {
            tessellate,
            ..EncoderConfig::default()
        };

        let mut geometry = if enc_cfg.tessellate {
            GeometryValues::new_tessellated()
        } else {
            GeometryValues::default()
        };
        for geom in &geometry_items {
            geometry.push_geom(geom);
        }

        let (id_values, id_enc_spec) = match ids {
            Some((v, id_width, int_enc)) => (Some(v), Some((id_width, int_enc))),
            None => (None, None),
        };

        // Build name→PropConfig map for the ExplicitEncoder callbacks.
        let prop_map: HashMap<String, PropConfig> = props
            .iter()
            .map(|(p, c)| (p.name().to_string(), c.clone()))
            .collect();

        let id_width_spec = id_enc_spec.as_ref().map(|(w, _)| *w);
        let id_int_enc = id_enc_spec.map(|(_, e)| e);

        let cfg = ExplicitEncoder {
            override_id_width: match id_width_spec {
                Some(w) => Box::new(move |_| w),
                None => Box::new(|w| w),
            },
            vertex_buffer_type,
            force_stream: Box::new(move |ctx: &StreamCtx<'_>| {
                ctx.kind == ColumnKind::Geometry && force_empty_streams.contains(ctx.name)
            }),
            get_int_encoder: {
                let prop_map = prop_map.clone();
                Box::new(move |ctx: &StreamCtx<'_>| match ctx.kind {
                    ColumnKind::Id => id_int_enc.unwrap_or_else(IntEncoder::varint),
                    ColumnKind::Geometry => geo_stream_overrides
                        .get(ctx.name)
                        .copied()
                        .unwrap_or(default_geo_enc),
                    ColumnKind::Property => prop_map
                        .get(ctx.name)
                        .map_or_else(IntEncoder::varint, |c| c.int_enc_for_stream_ctx(ctx)),
                })
            },
            get_str_encoding: {
                Box::new(move |name: &str| {
                    prop_map
                        .get(name)
                        .map_or(StrEncoding::Plain, PropConfig::str_encoding)
                })
            },
            override_presence: Box::new(move |_| force_presence),
        };

        StagedLayer {
            name: "layer1".to_string(),
            extent: extent.unwrap_or(80),
            id: id_values.map(StagedId::from_optional),
            geometry,
            properties: props.into_iter().map(|(p, _)| p).collect(),
        }
        .encode_into(Encoder::with_explicit(enc_cfg, cfg))?
        .into_layer_bytes()
        .map_err(SynthErr::Mlt)
    }
}

/// Builder for a shared dictionary struct column with multiple string sub-properties.
pub struct SharedDict {
    name: String,
    dict_encoding: StrEncoding,
    /// `(suffix, encoder, values, is_optional)`
    items: Vec<(String, IntEncoder, Vec<Option<String>>, bool)>,
}

impl SharedDict {
    /// Create a new shared dictionary builder.
    ///
    /// # Arguments
    /// * `name` - The name for the property (e.g., `"name:"` for `"name:de"`, `"name:en"`).
    /// * `dict_encoding` - The string encoding for the shared dictionary corpus (plain or FSST).
    #[must_use]
    pub fn new(name: impl Into<String>, dict_encoding: StrEncoding) -> Self {
        Self {
            name: name.into(),
            dict_encoding,
            items: vec![],
        }
    }

    /// Add a non-optional child column (no presence stream will be written).
    #[must_use]
    pub fn col<S: Into<String>>(
        mut self,
        suffix: impl Into<String>,
        offsets: IntEncoder,
        values: impl IntoIterator<Item = S>,
    ) -> Self {
        self.items.push((
            suffix.into(),
            offsets,
            values.into_iter().map(|v| Some(v.into())).collect(),
            false,
        ));
        self
    }

    /// Add an optional child column (a presence stream is always written).
    #[must_use]
    pub fn opt(
        mut self,
        suffix: impl Into<String>,
        offsets: IntEncoder,
        values: impl IntoIterator<Item = Option<String>>,
    ) -> Self {
        self.items
            .push((suffix.into(), offsets, values.into_iter().collect(), true));
        self
    }
}

/// Morton (Z-order) curve: de-interleave index bits into x/y (even/odd bits).
/// Produces a 4×4 complete Morton block (16 points, scale 8).
pub fn morton_curve() -> Vec<Coord<i32>> {
    let num_points = 16usize;
    let scale = 8_i32;
    let morton_bits = 4u32;
    let mut curve = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let i = i32::try_from(i).unwrap();
        let mut x = 0_i32;
        let mut y = 0_i32;
        for b in 0..morton_bits {
            x |= ((i >> (2 * b)) & 1) << b;
            y |= ((i >> (2 * b + 1)) & 1) << b;
        }
        curve.push(crate::c(x * scale, y * scale));
    }
    curve
}
