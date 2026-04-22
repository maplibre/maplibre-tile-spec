use crate::decoder::{ParsedProperty, ParsedScalar, RawPresence, RawProperty};
use crate::utils::{Presence, decode_presence};
use crate::{Decode, Decoder, MltResult};

impl<'a, T: Copy + PartialEq> ParsedScalar<'a, T> {
    pub fn from_parts(
        name: &'a str,
        presence: RawPresence<'a>,
        values: Vec<T>,
        dec: &mut Decoder,
    ) -> MltResult<Self> {
        let presence = decode_presence(presence, values.len(), dec)?;
        Ok(Self {
            name,
            presence,
            values,
        })
    }

    /// Total number of features (present and absent).
    #[inline]
    #[must_use]
    pub fn feature_count(&self) -> usize {
        match &self.presence {
            Presence::AllPresent => self.values.len(),
            Presence::Bits(bits) => bits.len(),
        }
    }

    /// Returns the value for feature `idx`, or `None` if absent or out of bounds.
    #[inline]
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<T> {
        match &self.presence {
            Presence::AllPresent => self.values.get(idx).copied(),
            Presence::Bits(bits) => {
                if *bits.get(idx)? {
                    Some(self.values[bits[..idx].count_ones()])
                } else {
                    None
                }
            }
        }
    }

    /// Expand into a `Vec<Option<T>>` with one entry per feature.
    ///
    /// Allocates a new vector; prefer [`ParsedScalar::get`] for single-feature access.
    #[must_use]
    pub fn materialize(&self) -> Vec<Option<T>> {
        match &self.presence {
            Presence::AllPresent => self.values.iter().copied().map(Some).collect(),
            Presence::Bits(bits) => {
                let mut dense = self.values.iter().copied();
                bits.iter()
                    .by_vals()
                    .map(|present| if present { dense.next() } else { None })
                    .collect()
            }
        }
    }

    /// Return the backing dense values slice (present entries only).
    #[inline]
    #[must_use]
    pub fn dense_values(&self) -> &[T] {
        &self.values
    }
}

impl<'a> Decode<ParsedProperty<'a>> for RawProperty<'a> {
    /// Decode into a [`ParsedProperty`], charging `dec` for every heap allocation.
    ///
    /// For scalar columns the output size is known from stream metadata, so
    /// the budget is charged *before* decoding.  For string and shared-dict
    /// columns the exact decoded size depends on compression, so the budget is
    /// charged *after* decoding based on actual allocation sizes.
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedProperty<'a>> {
        /// Decode the dense value stream and wrap it with the presence bitmap.
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
