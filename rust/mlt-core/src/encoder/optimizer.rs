use crate::decoder::TileLayer01;
use crate::encoder::{
    EncodeProperties as _, EncodedLayer, EncodedLayer01, GeometryEncoder, IdEncoder, IntEncoder,
    LayerEncoder, PropertyEncoder, SortStrategy, StagedLayer, StagedLayer01, StringGroup,
    group_string_properties, reorder_features, spatial_sort_likely_to_help,
};
use crate::{MltError, MltResult};

impl StagedLayer {
    /// Encode using a specific `LayerEncoder`, consuming `self` and producing [`EncodedLayer`].
    ///
    /// The `sort_strategy` in a `LayerEncoder::Tag01` is ignored here because sorting must
    /// happen before staging (on the `TileLayer01`). Use [`Tile01Encoder::encode`] for the
    /// full pipeline including sort.
    pub fn encode(self, encoder: LayerEncoder) -> MltResult<EncodedLayer> {
        match (self, encoder) {
            (Self::Tag01(t), LayerEncoder::Tag01(e)) => {
                Ok(EncodedLayer::Tag01(t.encode(e.stream)?))
            }
            (Self::Unknown(u), LayerEncoder::Unknown) => Ok(EncodedLayer::Unknown(u)),
            _ => Err(MltError::BadEncoderDataCombination),
        }
    }

    /// Automatically select the best encoders, consuming `self` and producing
    /// `(EncodedLayer, LayerEncoder)`.
    ///
    /// Sort strategy is [`SortStrategy::Unsorted`] in the returned encoder because sorting must
    /// happen before staging. Use [`Tile01Encoder::encode_auto`] for full
    /// sort + stream trialing on a [`crate::TileLayer01`].
    pub fn encode_auto(self, cfg: EncoderConfig) -> MltResult<(EncodedLayer, LayerEncoder)> {
        match self {
            Self::Tag01(t) => {
                let (encoded, stream_enc) = t.encode_auto(cfg)?;
                let tile_enc = Tile01Encoder {
                    stream: stream_enc,
                    ..Default::default()
                };
                Ok((EncodedLayer::Tag01(encoded), LayerEncoder::Tag01(tile_enc)))
            }
            Self::Unknown(u) => Ok((EncodedLayer::Unknown(u), LayerEncoder::Unknown)),
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
    pub fn encode_auto(
        self,
        cfg: EncoderConfig,
    ) -> MltResult<(EncodedLayer01, StagedLayer01Encoder)> {
        let (geometry, geom_enc) = self.geometry.encode_auto(cfg)?;
        let (id, id_enc) = match self.id {
            Some(parsed_id) => match parsed_id.encode_auto(cfg)? {
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
    /// String-property groups computed during optimization; empty when shared dicts are disabled.
    /// Stored so that [`encode`](Self::encode) uses identical grouping to the original trial run.
    pub str_groups: Vec<StringGroup>,
    /// Stream-level encoder settings applied after sorting and staging.
    pub stream: StagedLayer01Encoder,
}

impl Tile01Encoder {
    /// Reorder features in `tile` and stage it using the same string grouping
    /// that was computed when this encoder was produced by [`encode_auto`](Self::encode_auto).
    pub fn encode(&self, tile: &mut TileLayer01) -> StagedLayer01 {
        reorder_features(tile, self.sort_strategy);
        StagedLayer01::from_tile(tile.clone(), &self.str_groups)
    }

    /// Automatically select the best sort strategy and stream-level encoders by
    /// competitive trialing.
    pub fn encode_auto(
        tile: &TileLayer01,
        cfg: EncoderConfig,
    ) -> MltResult<(EncodedLayer01, Self)> {
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
        Self::encode_with(tile, &sort_by, cfg)
    }

    fn encode_with(
        tile: &TileLayer01,
        sort_by: &[SortStrategy],
        cfg: EncoderConfig,
    ) -> MltResult<(EncodedLayer01, Self)> {
        struct TrialResult {
            layer: EncodedLayer01,
            stream_enc: StagedLayer01Encoder,
            byte_count: usize,
            strategy: SortStrategy,
        }

        let str_groups = if cfg.allow_shared_dict {
            // String properties grouping should be the same regardless of the feature order
            group_string_properties(tile)
        } else {
            Vec::new()
        };

        let mut best: Option<TrialResult> = None;
        for &strategy in sort_by {
            let mut tile = tile.clone();
            reorder_features(&mut tile, strategy);

            let staged = StagedLayer01::from_tile(tile, &str_groups);
            let (encoded, stream_enc) = staged.encode_auto(cfg)?;

            // TODO: use Analyze instead of this
            let mut buf: Vec<u8> = Vec::new();
            encoded.write_to(&mut buf)?;
            let byte_count = buf.len();

            if best.as_ref().is_none_or(|b| byte_count < b.byte_count) {
                best = Some(TrialResult {
                    layer: encoded,
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
                str_groups,
                stream: best.stream_enc,
            },
        ))
    }
}
