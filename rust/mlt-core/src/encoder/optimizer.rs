use crate::MltResult;
use crate::decoder::TileLayer01;
use crate::encoder::{
    EncodeProperties as _, EncodedLayer, Encoder, GeometryEncoder, IdEncoder, PropertyEncoder,
    SortStrategy, StagedLayer, StagedLayer01, group_string_properties, reorder_features,
    spatial_sort_likely_to_help, write_properties_to,
};

impl StagedLayer {
    /// Automatically encode and write `self` to `enc`.
    ///
    /// Sort strategy for Tag01 layers is [`SortStrategy::Unsorted`] here because
    /// sorting must happen before staging. Use [`encode_tile_layer`] for the full
    /// sort + stream trialing on a [`TileLayer01`].
    pub fn encode_into(self, enc: &mut Encoder) -> MltResult<()> {
        match self {
            Self::Tag01(t) => t.encode_into(enc),
            Self::Unknown(u) => EncodedLayer::Unknown(u).write_to(enc),
        }
    }
}

/// Global encoder settings controlling which optimization strategies are attempted.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "enums would not model this better, not a state machine"
)]
pub struct EncoderConfig {
    /// Generate tessellation data for polygons and multi-polygons.
    pub tessellate: bool,
    /// Try sorting features by the Z-order (Morton) curve index of their first vertex.
    pub try_spatial_morton_sort: bool,
    /// Try sorting features by the Hilbert curve index of their first vertex.
    pub try_spatial_hilbert_sort: bool,
    /// Try sorting features by their feature ID in ascending order.
    pub try_id_sort: bool,
    /// Allow `FSST` string compression
    pub allow_fsst: bool,
    /// Allow `FastPFOR` integer compression
    pub allow_fpf: bool,
    /// Allow string grouping into shared dictionaries
    pub allow_shared_dict: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            tessellate: false,
            try_spatial_morton_sort: true,
            try_spatial_hilbert_sort: true,
            try_id_sort: true,
            allow_fsst: true,
            allow_fpf: true,
            allow_shared_dict: true,
        }
    }
}

impl StagedLayer01 {
    /// Encode using explicit per-stream encoders and write directly to `enc`.
    ///
    /// All encoding choices are caller-specified. Used by synthetics and tests that require
    /// deterministic encoding. For automatic optimization, use [`encode_tile_layer`].
    pub fn encode_with(
        self,
        enc: &mut Encoder,
        id_enc: IdEncoder,
        geometry: GeometryEncoder,
        properties: Vec<PropertyEncoder>,
    ) -> MltResult<()> {
        let id_present = if let Some(id) = self.id {
            id.write_to_with(enc, id_enc)?
        } else {
            false
        };
        self.geometry.write_to_with(enc, geometry)?;
        let prop_count = write_properties_to(&self.properties, properties, enc)?;

        let col_count = u32::from(id_present) + 1 + prop_count;
        enc.write_header(&self.name, self.extent, col_count)?;

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
    /// within the logical ordering — but since [`Encoder`] accumulates
    /// `hdr`/`meta`/`data` in separate buffers the final byte order is always
    /// correct.
    ///
    /// Encoding configuration is read from [`enc.cfg`](Encoder::cfg).
    pub(super) fn encode_into(self, enc: &mut Encoder) -> MltResult<()> {
        let cfg = enc.cfg;

        // Write each column's type byte to enc.meta and data to enc.data directly.
        let id_present = if let Some(id) = self.id {
            id.write_to(enc, cfg)?
        } else {
            false
        };
        self.geometry.write_to(enc, cfg)?;
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
