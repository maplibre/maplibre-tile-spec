use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::decoder::{
    ParsedProperty, ParsedScalar, RawFloats, RawFloatsEncoding, RawPresence, RawProperty,
};
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
            #[cfg(feature = "unstable-v2")]
            Self::Bitfield(bits) => Ok(Some(Cow::Borrowed(bits))),
            // The last reader takes the bits rather than copying them, which for a
            // column's own field - the common case - is every reader.
            #[cfg(feature = "unstable-v2")]
            Self::Decoded(bits) => Ok(Some(Cow::Owned(
                std::rc::Rc::try_unwrap(bits).unwrap_or_else(|bits| (*bits).clone()),
            ))),
        }
    }
}

impl<'a> RawFloats<'a> {
    /// Decode the column as its encoding says.
    fn decode<T>(self, dec: &mut Decoder) -> MltResult<ParsedScalar<'a, T>>
    where
        T: Copy + PartialEq + num_traits::FromBytes + crate::codecs::float::FloatValue,
        for<'b> <T as num_traits::FromBytes>::Bytes: TryFrom<&'b [u8]>,
    {
        let (name, presence) = (self.name, self.presence.clone());
        let values = self.decode_values::<T>(dec)?;
        ParsedScalar::from_parts(name, presence, values, dec)
    }

    /// Decode just the values, as the encoding says.
    pub(crate) fn decode_values<T>(self, dec: &mut Decoder) -> MltResult<Vec<T>>
    where
        T: Copy + PartialEq + num_traits::FromBytes + crate::codecs::float::FloatValue,
        for<'b> <T as num_traits::FromBytes>::Bytes: TryFrom<&'b [u8]>,
    {
        Ok(match self.encoding {
            RawFloatsEncoding::Single(data) => data.decode_floats::<T>(dec)?,
            #[cfg(feature = "unstable-v2")]
            RawFloatsEncoding::Alp { params, data } => {
                let codes = data.decode_alp_codes(params, dec)?;
                dec.consume_items::<T>(codes.len())?;
                crate::codecs::alp::decode::<T>(&codes, params.scale)
            }
        })
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
    #[hotpath::measure]
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
            Self::F32(v) => P::F32(v.decode::<f32>(dec)?),
            Self::F64(v) => P::F64(v.decode::<f64>(dec)?),
            Self::Str(v) => P::Str(v.decode(dec)?),
            Self::SharedDict(v) => P::SharedDict(v.decode(dec)?),
        })
    }
}
