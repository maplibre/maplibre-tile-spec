use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use geo_types::Coord;
use mlt_core::encoder::{
    Encoder, GeometryEncoder, IdEncoder, IntEncoder, PropertyEncoder, ScalarEncoder,
    SharedDictEncoder, SharedDictItemEncoder, StagedLayer01, StagedProperty, StagedSharedDict,
    StrEncoder, TessellationMode, VertexBufferType,
};
use mlt_core::geojson::Geom32;
use mlt_core::{GeometryValues, IdValues};

use crate::writer::{SynthErr, SynthResult};

/// Create a layer with all geometry encoders set to `VarInt`.
pub fn geo_varint() -> Layer {
    Layer::new(IntEncoder::varint())
}

/// Create a layer with geometry encoders set to `VarInt` and RLE for the meta (geometry types) stream.
pub fn geo_varint_with_rle() -> Layer {
    Layer::new(IntEncoder::varint()).meta(IntEncoder::rle_varint())
}

/// Create a layer with all geometry encoders set to `FastPFOR`.
pub fn geo_fastpfor() -> Layer {
    Layer::new(IntEncoder::fastpfor())
}

/// Layer builder: holds geometry encoder, geometry list, properties, extent, and IDs.
#[derive(Clone)]
pub struct Layer {
    geometry_encoder: GeometryEncoder,
    geometry_items: Vec<Geom32>,
    properties: Vec<StagedProperty>,
    prop_encoders: Vec<PropertyEncoder>,
    extent: Option<u32>,
    ids: Option<(Vec<Option<u64>>, IdEncoder)>,
}

impl Layer {
    fn new(default_geom_enc: IntEncoder) -> Self {
        Self {
            geometry_encoder: GeometryEncoder::all(default_geom_enc),
            geometry_items: vec![],
            properties: vec![],
            prop_encoders: vec![],
            extent: None,
            ids: None,
        }
    }

    pub fn force_presence_stream(&mut self) {
        for p in &mut self.prop_encoders {
            match p {
                PropertyEncoder::Scalar(v) => {
                    *v = v.forced_presence(true);
                }
                PropertyEncoder::SharedDict(v) => {
                    v.items.iter_mut().for_each(|vv| {
                        *vv = vv.forced_presence(true);
                    });
                }
            }
        }
    }

    /// Set encoding for parts length stream when rings are present.
    #[must_use]
    pub fn rings(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.rings(e);
        self
    }

    /// Set encoding for ring vertex-count stream.
    #[must_use]
    pub fn rings2(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.rings2(e);
        self
    }

    /// Set encoding for the geometry types (meta) stream.
    #[must_use]
    pub fn meta(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.meta(e);
        self
    }

    /// Set encoding for parts length stream when rings are not present.
    #[must_use]
    pub fn no_rings(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.no_rings(e);
        self
    }

    /// Set encoding for ring lengths when `geometry_offsets` absent.
    #[must_use]
    pub fn parts_ring(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.parts_ring(e);
        self
    }

    /// Set encoding for vertex offsets.
    #[must_use]
    pub fn vertex_offsets(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.vertex_offsets(e);
        self
    }

    /// Set encoding of the vertex buffer.
    #[must_use]
    pub fn vertex_buffer_type(mut self, v: VertexBufferType) -> Self {
        self.geometry_encoder.vertex_buffer_type(v);
        self
    }

    /// Add a geometry (uses [`geo_types::Geometry`] `From` impls: `Point`, `LineString`, etc.).
    #[must_use]
    pub fn geo(mut self, geometry: impl Into<Geom32>) -> Self {
        self.geometry_items.push(geometry.into());
        self
    }

    /// Add multiple geometries
    #[must_use]
    pub fn geos<T: Into<Geom32>, I: IntoIterator<Item = T>>(mut self, geometries: I) -> Self {
        for g in geometries {
            self = self.geo(g.into());
        }
        self
    }

    /// Enable polygon tessellation
    #[must_use]
    pub fn tessellate(mut self) -> Self {
        self.geometry_encoder.tessellation(TessellationMode::Earcut);
        self
    }

    /// Add a scalar property.
    #[must_use]
    pub fn add_prop(mut self, encoder: ScalarEncoder, prop: StagedProperty) -> Self {
        self.properties.push(prop);
        self.prop_encoders.push(PropertyEncoder::Scalar(encoder));
        self
    }

    /// Add a shared dictionary with its child columns.
    ///
    /// Use [`SharedDict::new`] to create the builder, then add columns with
    /// [`SharedDict::column`], and pass it to this method.
    #[must_use]
    pub fn add_shared_dict(mut self, shared_dict: SharedDict) -> Self {
        let name = shared_dict.name;
        let encoder = shared_dict.encoder;
        let dict = StagedSharedDict::new(name, shared_dict.items)
            .expect("shared dict builder should be valid");
        self.properties.push(StagedProperty::SharedDict(dict));
        self.prop_encoders.push(encoder.into());
        self
    }

    /// Set the tile extent.
    #[must_use]
    pub fn extent(mut self, extent: u32) -> Self {
        self.extent = Some(extent);
        self
    }

    /// Set feature IDs.
    #[must_use]
    pub fn ids(mut self, ids: Vec<Option<u64>>, encoder: IdEncoder) -> Self {
        self.ids = Some((ids, encoder));
        self
    }

    pub fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    pub fn encode_to_bytes(self) -> SynthResult<Vec<u8>> {
        let Self {
            geometry_encoder,
            geometry_items,
            properties,
            prop_encoders,
            extent,
            ids,
        } = self;

        let (id_values, id_encoder) = match ids {
            Some((v, e)) => (Some(v), e),
            None => (None, IdEncoder::default()),
        };

        let mut geometry = match geometry_encoder.tessellation {
            TessellationMode::Earcut => GeometryValues::new_tessellated(),
            TessellationMode::None => GeometryValues::default(),
        };
        for geom in &geometry_items {
            geometry.push_geom(geom);
        }

        let mut enc = Encoder::default();
        StagedLayer01 {
            name: "layer1".to_string(),
            extent: extent.unwrap_or(80),
            id: id_values.map(IdValues),
            geometry,
            properties,
        }
        .encode_with(&mut enc, id_encoder, geometry_encoder, prop_encoders)?;
        enc.into_layer_bytes().map_err(SynthErr::Mlt)
    }
}

/// Builder for a shared dictionary struct column with multiple string sub-properties.
///
/// Use [`SharedDict::new`] to create the builder, add columns with [`SharedDict::column`],
/// then pass it to [`Layer::add_shared_dict`].
pub struct SharedDict {
    name: String,
    encoder: SharedDictEncoder,
    items: Vec<(String, Vec<Option<String>>)>,
}

impl SharedDict {
    /// Create a new shared dictionary builder.
    ///
    /// # Arguments
    /// * `name` - The name for the property (e.g., `"name:"` for `"name:de"`, `"name:en"`).
    /// * `dict_encoder` - The string encoder for the shared dictionary (plain or FSST).
    #[must_use]
    pub fn new(name: impl Into<String>, dict_encoder: StrEncoder) -> Self {
        Self {
            name: name.into(),
            encoder: SharedDictEncoder {
                dict_encoder,
                items: vec![],
            },
            items: vec![],
        }
    }

    /// Add a child column to the shared dictionary.
    ///
    /// # Arguments
    /// * `suffix` - The suffix name for this child (e.g., `"de"` for `"name:de"`).
    /// * `offsets` - The integer encoder for the offset-index stream.
    /// * `values` - The string values for each feature.
    #[must_use]
    pub fn column(
        self,
        suffix: impl Into<String>,
        offsets: IntEncoder,
        values: impl IntoIterator<Item = Option<String>>,
    ) -> Self {
        self.column_with_enc(SharedDictItemEncoder::new(offsets), suffix, values)
    }

    fn column_with_enc(
        mut self,
        enc: SharedDictItemEncoder,
        suffix: impl Into<String>,
        values: impl IntoIterator<Item = Option<String>>,
    ) -> Self {
        self.encoder.items.push(enc);
        let suffix = suffix.into();
        let values: Vec<Option<String>> = values.into_iter().collect();
        self.items.push((suffix, values));
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
