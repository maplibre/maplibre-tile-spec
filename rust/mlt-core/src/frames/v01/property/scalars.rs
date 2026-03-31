use crate::v01::{
    FsstStrEncoder, IntEncoder, PropertyEncoder, ScalarEncoder, ScalarValueEncoder, StrEncoder,
};

impl ScalarEncoder {
    fn new(value: ScalarValueEncoder) -> Self {
        Self {
            value,
            #[cfg(feature = "__private")]
            forced_presence: false,
        }
    }
    #[must_use]
    pub fn str(string_lengths: IntEncoder) -> Self {
        let enc = StrEncoder::Plain { string_lengths };
        Self::new(ScalarValueEncoder::String(enc))
    }
    /// Create a property encoder with integer encoding
    #[must_use]
    pub fn int(enc: IntEncoder) -> Self {
        Self::new(ScalarValueEncoder::Int(enc))
    }
    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn str_fsst(symbol_lengths: IntEncoder, dict_lengths: IntEncoder) -> Self {
        Self::new(ScalarValueEncoder::String(StrEncoder::Fsst(
            FsstStrEncoder {
                symbol_lengths,
                dict_lengths,
            },
        )))
    }

    /// Create a property encoder with deduplicated plain dictionary string encoding.
    /// Encodes unique strings once; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_dict(string_lengths: IntEncoder, offsets: IntEncoder) -> Self {
        Self::new(ScalarValueEncoder::String(StrEncoder::Dict {
            string_lengths,
            offsets,
        }))
    }

    /// Create a property encoder with deduplicated FSST dictionary string encoding.
    /// FSST-compresses unique strings; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_fsst_dict(
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self::new(ScalarValueEncoder::String(StrEncoder::FsstDict {
            fsst: FsstStrEncoder {
                symbol_lengths,
                dict_lengths,
            },
            offsets,
        }))
    }
    /// Create a property encoder for boolean values
    #[must_use]
    pub fn bool() -> Self {
        Self::new(ScalarValueEncoder::Bool)
    }
    /// Create a property encoder for float values
    #[must_use]
    pub fn float() -> Self {
        Self::new(ScalarValueEncoder::Float)
    }
}

/// FIXME: uncertain why we need this, delete?
impl From<ScalarEncoder> for PropertyEncoder {
    fn from(encoder: ScalarEncoder) -> Self {
        Self::Scalar(encoder)
    }
}
