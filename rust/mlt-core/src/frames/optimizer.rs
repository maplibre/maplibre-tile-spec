use crate::frames::{EncodedLayer, LayerEncoder};
use crate::v01::{EncoderConfig, Tile01Encoder};
use crate::{MltError, MltResult, StagedLayer};

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
    /// sort + stream trialing on a [`crate::v01::TileLayer01`].
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
