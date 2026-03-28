use crate::v01::{
    FsstStrEncoder, IntEncoder, PresenceStream, PropertyEncoder, ScalarEncoder, ScalarValueEncoder,
    StrEncoder,
};

impl ScalarEncoder {
    #[must_use]
    pub fn str(presence: PresenceStream, string_lengths: IntEncoder) -> Self {
        let enc = StrEncoder::Plain { string_lengths };
        Self {
            presence,
            value: ScalarValueEncoder::String(enc),
        }
    }
    /// Create a property encoder with integer encoding
    #[must_use]
    pub fn int(presence: PresenceStream, enc: IntEncoder) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Int(enc),
        }
    }
    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn str_fsst(
        presence: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::Fsst(FsstStrEncoder {
                symbol_lengths,
                dict_lengths,
            })),
        }
    }

    /// Create a property encoder with deduplicated plain dictionary string encoding.
    /// Encodes unique strings once; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_dict(
        presence: PresenceStream,
        string_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::Dict {
                string_lengths,
                offsets,
            }),
        }
    }

    /// Create a property encoder with deduplicated FSST dictionary string encoding.
    /// FSST-compresses unique strings; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_fsst_dict(
        presence: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::FsstDict {
                fsst: FsstStrEncoder {
                    symbol_lengths,
                    dict_lengths,
                },
                offsets,
            }),
        }
    }
    /// Create a property encoder for boolean values
    #[must_use]
    pub fn bool(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Bool,
        }
    }
    /// Create a property encoder for float values
    #[must_use]
    pub fn float(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Float,
        }
    }
}

/// FIXME: uncertain why we need this, delete?
impl From<ScalarEncoder> for PropertyEncoder {
    fn from(encoder: ScalarEncoder) -> Self {
        Self::Scalar(encoder)
    }
}
