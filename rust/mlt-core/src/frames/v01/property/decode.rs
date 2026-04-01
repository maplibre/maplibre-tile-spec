use crate::lazy_state::Decode;
use crate::utils::apply_present;
use crate::v01::{
    ParsedPresence, ParsedProperty, ParsedScalar, RawPresence, RawProperty, StagedProperty,
    StagedScalar, StagedStrings,
};
use crate::{Decoder, MltResult};

impl<'a, T: Copy + PartialEq> ParsedScalar<'a, T> {
    #[must_use]
    pub fn new(name: &'a str, values: Vec<Option<T>>) -> Self {
        Self { name, values }
    }

    pub fn from_parts(
        name: &'a str,
        presence: RawPresence<'a>,
        values: Vec<T>,
        dec: &mut Decoder,
    ) -> MltResult<Self> {
        Ok(Self {
            name,
            values: apply_present(presence, values, dec)?,
        })
    }
}

impl ParsedPresence {
    #[must_use]
    pub fn bools(&self, non_null_count: usize) -> Vec<bool> {
        self.0.clone().unwrap_or_else(|| vec![true; non_null_count])
    }

    #[must_use]
    pub fn feature_count(&self, non_null_count: usize) -> usize {
        self.0.as_ref().map_or(non_null_count, Vec::len)
    }
}

impl StagedProperty {
    #[must_use]
    pub fn bool(name: impl Into<String>, values: Vec<Option<bool>>) -> Self {
        Self::Bool(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i8(name: impl Into<String>, values: Vec<Option<i8>>) -> Self {
        Self::I8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u8(name: impl Into<String>, values: Vec<Option<u8>>) -> Self {
        Self::U8(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i32(name: impl Into<String>, values: Vec<Option<i32>>) -> Self {
        Self::I32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u32(name: impl Into<String>, values: Vec<Option<u32>>) -> Self {
        Self::U32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn i64(name: impl Into<String>, values: Vec<Option<i64>>) -> Self {
        Self::I64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn u64(name: impl Into<String>, values: Vec<Option<u64>>) -> Self {
        Self::U64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f32(name: impl Into<String>, values: Vec<Option<f32>>) -> Self {
        Self::F32(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn f64(name: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self::F64(StagedScalar {
            name: name.into(),
            values,
        })
    }
    #[must_use]
    pub fn str(name: impl Into<String>, values: Vec<Option<String>>) -> Self {
        Self::Str(StagedStrings::from_optional(name, values))
    }
}

impl<'a> Decode<ParsedProperty<'a>> for RawProperty<'a> {
    fn decode(self, decoder: &mut Decoder) -> MltResult<ParsedProperty<'a>> {
        RawProperty::decode(self, decoder)
    }
}

impl<'a> RawProperty<'a> {
    /// Decode into a [`ParsedProperty`], charging `dec` for every heap allocation.
    ///
    /// For scalar columns the output size is known from stream metadata, so
    /// the budget is charged *before* decoding.  For string and shared-dict
    /// columns the exact decoded size depends on compression, so the budget is
    /// charged *after* decoding based on actual allocation sizes.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<ParsedProperty<'a>> {
        /// Charge for the final `Vec<Option<T>>`, then decode the dense stream.
        /// `$decode_method` is the typed `RawStream` method for element type `$ty`.
        macro_rules! scalar_decode {
            ($variant:ident, $ty:ty, $decode_method:ident, $v:expr, $dec:expr) => {{
                ParsedProperty::$variant(ParsedScalar::from_parts(
                    $v.name,
                    $v.presence,
                    $v.data.$decode_method($dec)?,
                    $dec,
                )?)
            }};
        }

        Ok(match self {
            Self::Bool(v) => scalar_decode!(Bool, bool, decode_bools, v, dec),
            Self::I8(v) => scalar_decode!(I8, i8, decode_i8s, v, dec),
            Self::U8(v) => scalar_decode!(U8, u8, decode_u8s, v, dec),
            Self::I32(v) => scalar_decode!(I32, i32, decode_i32s, v, dec),
            Self::U32(v) => scalar_decode!(U32, u32, decode_u32s, v, dec),
            Self::I64(v) => scalar_decode!(I64, i64, decode_i64s, v, dec),
            Self::U64(v) => scalar_decode!(U64, u64, decode_u64s, v, dec),
            Self::F32(v) => scalar_decode!(F32, f32, decode_f32s, v, dec),
            Self::F64(v) => scalar_decode!(F64, f64, decode_f64s, v, dec),
            Self::Str(v) => ParsedProperty::Str(v.decode(dec)?),
            Self::SharedDict(v) => ParsedProperty::SharedDict(v.decode(dec)?),
        })
    }
}
