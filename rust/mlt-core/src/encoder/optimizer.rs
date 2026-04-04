use crate::MltResult;
use crate::decoder::TileLayer01;
use crate::encoder::property::encode::write_properties;
use crate::encoder::{
    Encoder, EncoderConfig, SortStrategy, StagedLayer, StagedLayer01, group_string_properties,
    reorder_features, spatial_sort_likely_to_help,
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
            Self::Unknown(u) => u.write_to(enc),
        }
    }
}

impl StagedLayer01 {
    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside [`encode_tile_layer`]: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    pub fn encode_into(self, enc: &mut Encoder) -> MltResult<()> {
        let Self {
            name,
            extent,
            id,
            geometry,
            properties,
        } = self;

        if let Some(ids) = id {
            ids.write_to(enc)?;
        }
        geometry.write_to(enc)?;
        write_properties(&properties, enc)?;
        enc.write_header(&name, extent)?;

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
