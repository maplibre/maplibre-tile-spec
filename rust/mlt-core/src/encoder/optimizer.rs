use crate::MltResult;
use crate::decoder::TileLayer;
use crate::encoder::model::StagedLayer;
use crate::encoder::property::encode::write_properties;
use crate::encoder::{
    Encoder, EncoderConfig, SortStrategy, group_string_properties, spatial_sort_likely_to_help,
};

impl StagedLayer {
    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside `TileLayer::encode`: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    #[hotpath::measure]
    pub fn encode_into(self, mut enc: Encoder) -> MltResult<Encoder> {
        let Self {
            name,
            extent,
            id,
            geometry,
            properties,
        } = self;

        if let Some(ids) = id {
            ids.write_to(&mut enc)?;
        }
        geometry.write_to(&mut enc)?;
        write_properties(&properties, &mut enc)?;
        enc.write_header(&name, extent)?;

        Ok(enc)
    }
}

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

impl TileLayer {
    /// Encode a [`TileLayer`] to bytes, automatically optimizing all encoding choices.
    ///
    /// This is the primary encoding entry point. It:
    /// 1. Determines which sort strategies to try based on `cfg`
    /// 2. Tries each sort strategy, encoding and measuring the output size
    /// 3. Returns the smallest encoding as a complete layer record (including tag and length prefix)
    ///
    /// All encoding choices — sort order, per-stream integer encodings, string compression,
    /// vertex buffer layout — are selected automatically to minimize output size.
    #[hotpath::measure]
    pub fn encode(self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        if self.features.is_empty() {
            return Ok(Vec::new());
        }

        let mut sort_by = vec![SortStrategy::Unsorted];
        let try_spatial_sort = cfg.try_spatial_morton_sort || cfg.try_spatial_hilbert_sort;
        if try_spatial_sort
            && (self.features.len() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(&self))
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

        let groups = if cfg.allow_shared_dict {
            group_string_properties(&self)
        } else {
            Vec::new()
        };

        let (last, init) = sort_by.split_last().expect("at least one strategy");
        if init.is_empty() {
            StagedLayer::from_tile(self, *last, &groups, cfg.tessellate)
                .encode_into(Encoder::new(cfg))?
        } else {
            let mut enc: Encoder = {
                let first = init[0];
                StagedLayer::from_tile(self.clone(), first, &groups, cfg.tessellate)
                    .encode_into(Encoder::new(cfg))?
            };
            let mut best = enc.preserve_results();
            // Clone for all-but-last strategies
            for &sort in &init[1..] {
                let layer = StagedLayer::from_tile(self.clone(), sort, &groups, cfg.tessellate);
                enc = layer.encode_into(enc)?;
                if enc.total_len() < best.total_len() {
                    best = enc.preserve_results();
                }
            }
            // Last strategy: consume self, no clone
            let layer = StagedLayer::from_tile(self, *last, &groups, cfg.tessellate);
            enc = layer.encode_into(enc)?;
            if enc.total_len() < best.total_len() {
                best = enc.preserve_results();
            }
            best
        }
        .into_layer_bytes()
    }
}
