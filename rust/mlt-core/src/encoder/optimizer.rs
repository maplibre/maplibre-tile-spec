use crate::MltResult;
use crate::decoder::TileLayer01;
#[cfg(feature = "__private")]
use crate::encoder::geometry::encode::encode_geometry;
#[cfg(feature = "__private")]
use crate::encoder::property::encode::write_properties;
#[cfg(feature = "__private")]
use crate::encoder::stream::IntEncoder;
use crate::encoder::{
    EncodeProperties as _, Encoder, EncoderConfig, SortStrategy, StagedLayer, StagedLayer01,
    group_string_properties, reorder_features, spatial_sort_likely_to_help,
};
#[cfg(feature = "__private")]
use crate::encoder::{ExplicitEncoder, IdWidth, VertexBufferType};

impl StagedLayer {
    /// Automatically encode and write `self` to `enc`.
    ///
    /// Sort strategy for Tag01 layers is [`SortStrategy::Unsorted`] here because
    /// sorting must happen before staging. Use [`encode_tile_layer`] for the full
    /// sort + stream trialing on a [`TileLayer01`].
    pub fn encode_into(self, enc: &mut Encoder) -> MltResult<()> {
        match self {
            Self::Tag01(t) => t.encode_into(enc),
            Self::Unknown(u) => u.write_to(enc),
        }
    }
}

#[cfg(feature = "__private")]
impl ExplicitEncoder {
    /// Use `enc` for all integer streams, plain string encoding, and `Vec2` vertex layout.
    #[must_use]
    pub fn all(enc: IntEncoder) -> Self {
        Self {
            override_id_width: Box::new(|w| w),
            vertex_buffer_type: VertexBufferType::Vec2,
            get_int_encoder: Box::new(move |_, _, _| enc),
            get_str_encoding: Box::new(|_, _| crate::encoder::StrEncoding::Plain),
            override_presence: Box::new(|_, _, _| false),
        }
    }

    /// Like [`Self::all`] but use `str_enc` for string property columns.
    #[must_use]
    pub fn all_with_str(enc: IntEncoder, str_enc: crate::encoder::StrEncoding) -> Self {
        Self {
            get_str_encoding: Box::new(move |_, _| str_enc),
            ..Self::all(enc)
        }
    }

    /// Use `id_enc` for the ID stream with a fixed `id_width`; `varint` for all other streams.
    ///
    /// Useful for tests that need to pin the exact ID encoding without caring about
    /// geometry or property streams.
    #[must_use]
    pub fn for_id(id_enc: IntEncoder, id_width: IdWidth) -> Self {
        Self {
            override_id_width: Box::new(move |_| id_width),
            get_int_encoder: Box::new(move |kind, _, _| {
                if kind == "id" {
                    id_enc
                } else {
                    IntEncoder::varint()
                }
            }),
            ..Self::all(IntEncoder::varint())
        }
    }
}

impl StagedLayer01 {
    /// Encode using an [`ExplicitEncoder`] and write directly to `enc`.
    ///
    /// All encoding choices are caller-specified. Used by synthetics and tests that require
    /// deterministic encoding. For automatic optimization, use [`encode_tile_layer`].
    #[cfg(feature = "__private")]
    pub fn encode_explicit(self, enc: &mut Encoder, cfg: &ExplicitEncoder) -> MltResult<()> {
        let Self {
            name,
            extent,
            id,
            geometry,
            properties,
        } = self;

        // ── ID column ────────────────────────────────────────────────────────
        let id_present = if let Some(ids) = id {
            ids.write_to_with(enc, cfg)?
        } else {
            false
        };

        // ── Geometry column ───────────────────────────────────────────────────
        encode_geometry(&geometry, cfg, enc)?;

        // ── Property columns ──────────────────────────────────────────────────
        let prop_count = write_properties(&properties, Some(cfg), enc)?;

        let col_count = u32::from(id_present) + 1 + prop_count;
        enc.write_header(&name, extent, col_count)?;

        Ok(())
    }

    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside [`encode_tile_layer`]: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    ///
    /// Column count is computed after encoding (because all-null / empty
    /// properties are omitted from the wire), so the header is written last
    /// within the logical ordering. Since [`Encoder`] accumulates
    /// `hdr`/`meta`/`data` in separate buffers the final byte order is always
    /// correct.
    ///
    /// Encoding configuration is read from [`enc.cfg`](Encoder::cfg).
    pub(crate) fn encode_into(self, enc: &mut Encoder) -> MltResult<()> {
        self.geometry.write_to(enc)?;

        // Write each column's type byte to enc.meta and data to enc.data directly.
        let id_present = if let Some(id) = self.id {
            id.write_to(enc)?
        } else {
            false
        };

        let prop_count = self.properties.write_to(enc)?;

        // Column count is only known after encoding.
        let col_count = u32::from(id_present)
            + 1 // geometry
            + prop_count;
        enc.write_header(&self.name, self.extent, col_count)?;

        Ok(())
    }
}

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

/// Encode a [`TileLayer01`] to bytes, automatically optimizing all encoding choices.
///
/// This is the primary encoding entry point. It:
/// 1. Determines which sort strategies to try based on `cfg`
/// 2. Tries each sort strategy, encoding and measuring the output size
/// 3. Returns the smallest encoding as a complete layer record (including tag and length prefix)
///
/// All encoding choices — sort order, per-stream integer encodings, string compression,
/// vertex buffer layout — are selected automatically to minimize output size.
pub fn encode_tile_layer(tile: &TileLayer01, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
    let mut sort_by = vec![SortStrategy::Unsorted];
    let try_spatial_sort = cfg.try_spatial_morton_sort || cfg.try_spatial_hilbert_sort;
    if try_spatial_sort
        && (tile.features.len() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(tile))
    {
        if cfg.try_spatial_morton_sort {
            sort_by.push(SortStrategy::SpatialMorton);
        }
        if cfg.try_spatial_hilbert_sort {
            sort_by.push(SortStrategy::SpatialHilbert);
        }
    }
    if cfg.try_id_sort {
        sort_by.push(SortStrategy::Id);
    }

    let str_groups = if cfg.allow_shared_dict {
        group_string_properties(tile)
    } else {
        Vec::new()
    };

    let mut best: Option<Encoder> = None;
    for strategy in sort_by {
        let mut tile = tile.clone();
        reorder_features(&mut tile, strategy);

        let staged = StagedLayer01::from_tile(tile, &str_groups);
        let mut enc = Encoder::new(cfg);
        staged.encode_into(&mut enc)?;

        if best
            .as_ref()
            .is_none_or(|b| enc.total_len() < b.total_len())
        {
            best = Some(enc);
        }
    }

    best.expect("non-empty best variant is set")
        .into_layer_bytes()
}
