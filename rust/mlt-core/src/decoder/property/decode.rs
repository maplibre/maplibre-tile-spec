use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::decoder::{ParsedProperty, ParsedScalar, RawPresence, RawProperty};
use crate::utils::decode_presence;
use crate::{Decode, Decoder, MltResult};

impl<'a> RawPresence<'a> {
    /// Decode into a packed bitvector, or `None` for a non-optional column.
    ///
    /// This is the only place aware of every wire representation of presence;
    /// all downstream presence handling goes through the returned bits.
    pub(crate) fn decode_bits(
        self,
        dec: &mut Decoder,
    ) -> MltResult<Option<Cow<'a, BitSlice<u8, Lsb0>>>> {
        match self {
            Self::AllPresent => Ok(None),
            Self::Stream(s) => Ok(Some(s.decode_bitvec(dec)?)),
        }
    }

    /// Decode into one bool per feature, or `None` for a non-optional column.
    pub(crate) fn decode_bools(self, dec: &mut Decoder) -> MltResult<Option<Vec<bool>>> {
        match self {
            Self::AllPresent => Ok(None),
            Self::Stream(s) => Ok(Some(s.decode_bools(dec)?)),
        }
    }
}

impl<'a, T: Copy + PartialEq> ParsedScalar<'a, T> {
    pub fn from_parts(
        name: &'a str,
        presence: RawPresence<'a>,
        values: Vec<T>,
        dec: &mut Decoder,
    ) -> MltResult<Self> {
        let presence = decode_presence(presence, values, dec)?;
        Ok(Self { name, presence })
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
        use ParsedProperty as P;
        use ParsedScalar as S;

        Ok(match self {
            Self::Bool(v) => {
                let vals = v.data.decode_bools(dec)?;
                P::Bool(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::I8(v) => {
                let vals = v.data.decode_narrow::<i8, i32>(dec)?;
                P::I8(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::U8(v) => {
                let vals = v.data.decode_narrow::<u8, u32>(dec)?;
                P::U8(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::I32(v) => {
                let vals = v.data.decode_ints::<i32>(dec)?;
                P::I32(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::U32(v) => {
                let vals = v.data.decode_ints::<u32>(dec)?;
                P::U32(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::I64(v) => {
                let vals = v.data.decode_ints::<i64>(dec)?;
                P::I64(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::U64(v) => {
                let vals = v.data.decode_ints::<u64>(dec)?;
                P::U64(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::F32(v) => {
                let vals = v.data.decode_floats::<f32>(dec)?;
                P::F32(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::F64(v) => {
                let vals = v.data.decode_floats::<f64>(dec)?;
                P::F64(S::from_parts(v.name, v.presence, vals, dec)?)
            }
            Self::Str(v) => P::Str(v.decode(dec)?),
            Self::SharedDict(v) => P::SharedDict(v.decode(dec)?),
        })
    }
}
