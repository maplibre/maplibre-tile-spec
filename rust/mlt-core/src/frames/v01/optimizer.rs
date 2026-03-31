use crate::MltResult;
use crate::v01::sort::{reorder_features, spatial_sort_likely_to_help};
use crate::v01::{
    EncodeProperties as _, EncodedLayer01, GeometryEncoder, IdEncoder, IntEncoder, PropertyEncoder,
    SortStrategy, StagedLayer01, TileLayer01, group_string_properties,
};

impl StagedLayer01 {
    /// Encode using a specific [`StagedLayer01Encoder`], consuming `self` and producing [`EncodedLayer01`].
    pub fn encode(self, encoder: StagedLayer01Encoder) -> MltResult<EncodedLayer01> {
        let geometry = self.geometry.encode(encoder.geometry)?;

        let id = match self.id {
            Some(id) => id.encode(encoder.id)?,
            None => None,
        };

        let properties = self
            .properties
            .encode(encoder.properties)?
            .into_iter()
            .flatten()
            .collect();

        Ok(EncodedLayer01 {
            name: self.name,
            extent: self.extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        })
    }

    /// Automatically select the best stream-level encoders by competitive trialing.
    ///
    /// This method does **not** attempt different sort strategies; call
    /// [`Tile01Encoder::encode_auto`] instead when sort optimization is also desired.
    pub fn encode_auto(self) -> MltResult<(EncodedLayer01, StagedLayer01Encoder)> {
        let (geometry, geom_enc) = self.geometry.encode_auto()?;
        let (id, id_enc) = match self.id {
            Some(parsed_id) => match parsed_id.encode_auto()? {
                Some((enc_id, enc)) => (Some(enc_id), enc),
                None => (None, IdEncoder::default()),
            },
            None => (None, IdEncoder::default()),
        };
        let (properties, props_enc) = self.properties.encode_auto()?;
        let properties: Vec<_> = properties.into_iter().flatten().collect();

        let stream_encoder = StagedLayer01Encoder {
            id: id_enc,
            properties: props_enc,
            geometry: geom_enc,
        };

        let layer = EncodedLayer01 {
            name: self.name,
            extent: self.extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        };

        Ok((layer, stream_encoder))
    }
}

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

/// Candidate sort strategies evaluated during automatic competitive trialing.
const TRIAL_STRATEGIES: [SortStrategy; 3] = [
    SortStrategy::Unsorted,
    SortStrategy::SpatialMorton,
    SortStrategy::Id,
];

/// Stream-level encoder configuration for a v01 layer.
///
/// Produced by any of the three optimization paths (manual, automatic, or profile-driven)
/// and consumed by [`StagedLayer01::encode`].  Sort ordering is handled separately by
/// [`Tile01Encoder`] before this stage.
#[derive(Debug, Clone)]
pub struct StagedLayer01Encoder {
    /// ID encoder.  Always present; used only when the layer actually contains ID data.
    pub id: IdEncoder,
    pub geometry: GeometryEncoder,
    pub properties: Vec<PropertyEncoder>,
}

impl Default for StagedLayer01Encoder {
    fn default() -> Self {
        Self {
            id: IdEncoder::default(),
            geometry: GeometryEncoder::all(IntEncoder::varint()),
            properties: Vec::new(),
        }
    }
}

/// Entry-point encoder that converts a [`TileLayer01`] into a [`StagedLayer01`] and
/// reorders features according to a [`SortStrategy`].
///
/// Holds both the sort strategy and the stream-level encoder settings, so the
/// complete configuration used for a tile can be captured and replayed.
///
/// For automatic sort-strategy selection, use [`Tile01Encoder::encode_auto`].
#[derive(Debug, Clone, Default)]
pub struct Tile01Encoder {
    /// How to reorder features before columnar staging.
    /// [`SortStrategy::Unsorted`] (the default) preserves the original feature order.
    pub sort_strategy: SortStrategy,
    /// Stream-level encoder settings applied after sorting and staging.
    pub stream: StagedLayer01Encoder,
}

impl Tile01Encoder {
    /// Reorder features in `tile` and stage it using the same `MinHash`-based
    /// string grouping that was applied when this encoder was produced.
    pub fn encode(&self, tile: &mut TileLayer01) -> StagedLayer01 {
        reorder_features(tile, self.sort_strategy);
        let str_groups = group_string_properties(tile);
        StagedLayer01::from_tile(tile.clone(), &str_groups)
    }

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    pub fn encode_auto(tile: &TileLayer01) -> MltResult<(EncodedLayer01, Self)> {
        Self::encode_with(
            tile,
            if tile.features.len() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(tile) {
                &TRIAL_STRATEGIES
            } else {
                &[SortStrategy::Unsorted, SortStrategy::Id]
            },
        )
    }

    fn encode_with(
        tile: &TileLayer01,
        sort_by: &[SortStrategy],
    ) -> MltResult<(EncodedLayer01, Self)> {
        struct TrialResult {
            layer: EncodedLayer01,
            stream_enc: StagedLayer01Encoder,
            byte_count: usize,
            strategy: SortStrategy,
        }

        // String properties grouping should be the same regardless of the feature order
        let str_groups = group_string_properties(tile);
        let mut best: Option<TrialResult> = None;

        for &strategy in sort_by {
            let mut tile = tile.clone();
            reorder_features(&mut tile, strategy);

            let staged = StagedLayer01::from_tile(tile, &str_groups);
            let (layer, stream_enc) = staged.encode_auto()?;

            // TODO: use Analyze instead of this
            let mut buf: Vec<u8> = Vec::new();
            layer.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer,
                    stream_enc,
                    byte_count,
                    strategy,
                });
            }
        }

        let best = best.expect("non-empty best variant is set");
        Ok((
            best.layer,
            Self {
                sort_strategy: best.strategy,
                stream: best.stream_enc,
            },
        ))
    }
}
