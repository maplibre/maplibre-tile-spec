use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
#[cfg(feature = "unstable-v2")]
use usize_cast::IntoUsize as _;

#[cfg(feature = "unstable-v2")]
use crate::MltError;
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
        }
    }
}

impl<'a> RawFloats<'a> {
    /// Decode the column, reading whichever stream set its encoding uses.
    fn decode<T>(self, dec: &mut Decoder) -> MltResult<ParsedScalar<'a, T>>
    where
        T: Copy + PartialEq + num_traits::FromBytes + crate::codecs::float::FloatValue,
        for<'b> <T as num_traits::FromBytes>::Bytes: TryFrom<&'b [u8]>,
    {
        let values = match self.encoding {
            RawFloatsEncoding::Single(data) => data.decode_floats::<T>(dec)?,
            #[cfg(feature = "unstable-v2")]
            RawFloatsEncoding::Alp { params, data } => {
                let codes = data.decode_ints::<i64>(dec)?;
                dec.consume_items::<T>(codes.len())?;
                crate::codecs::alp::decode::<T>(&codes, params)
            }
            #[cfg(feature = "unstable-v2")]
            RawFloatsEncoding::Dictionary { codes, dictionary } => {
                let dictionary = dictionary.decode_floats::<T>(dec)?;
                let codes = codes.decode_ints::<u32>(dec)?;
                dec.consume_items::<T>(codes.len())?;
                codes
                    .into_iter()
                    .map(|code| {
                        dictionary
                            .get(code.into_usize())
                            .copied()
                            .ok_or(MltError::DictionaryCodeOutOfRange(code, dictionary.len()))
                    })
                    .collect::<MltResult<Vec<T>>>()?
            }
        };
        ParsedScalar::from_parts(self.name, self.presence, values, dec)
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
            Self::F32(v) => P::F32(v.decode::<f32>(dec)?),
            Self::F64(v) => P::F64(v.decode::<f64>(dec)?),
            Self::Str(v) => P::Str(v.decode(dec)?),
            Self::SharedDict(v) => P::SharedDict(v.decode(dec)?),
        })
    }
}

#[cfg(all(test, feature = "unstable-v2"))]
mod tests {
    use bytemuck::cast_slice;

    use super::*;
    use crate::decoder::{
        DictionaryType, IntEncoding, IntLogical, LogicalEncoding, PhysicalEncoding, RawStream,
        StreamMeta, StreamType, ValueKind,
    };
    use crate::test_helpers::dec;

    fn codes(values: &[u32]) -> RawStream<'_> {
        let encoding = IntEncoding::new(
            LogicalEncoding::Int(IntLogical::None),
            PhysicalEncoding::None,
        );
        let num = u32::try_from(values.len()).unwrap();
        let meta = StreamMeta::new(StreamType::Data(DictionaryType::None), encoding, num);
        RawStream::new(meta, cast_slice(values))
    }

    fn dictionary(values: &[f64]) -> RawStream<'_> {
        let num = u32::try_from(values.len()).unwrap();
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::Single),
            IntEncoding::none(ValueKind::Float),
            num,
        );
        RawStream::new(meta, cast_slice(values))
    }

    fn floats<'a>(codes: RawStream<'a>, dictionary: RawStream<'a>) -> RawFloats<'a> {
        RawFloats {
            name: "v",
            presence: RawPresence::AllPresent,
            encoding: RawFloatsEncoding::Dictionary { codes, dictionary },
        }
    }

    #[test]
    fn a_dictionary_column_expands_its_codes_bit_for_bit() {
        let dict = [1.5_f64, -0.0, f64::NAN];
        let column = floats(codes(&[2, 0, 1, 0]), dictionary(&dict));
        let parsed = column.decode::<f64>(&mut dec()).unwrap();

        let bits: Vec<u64> = parsed
            .presence
            .dense_values()
            .iter()
            .map(|v| v.to_bits())
            .collect();
        let expected: Vec<u64> = [dict[2], dict[0], dict[1], dict[0]]
            .iter()
            .map(|v| v.to_bits())
            .collect();
        assert_eq!(bits, expected);
    }

    #[test]
    fn a_code_past_the_end_of_the_dictionary_is_rejected() {
        let column = floats(codes(&[0, 3]), dictionary(&[1.0, 2.0, 3.0]));
        let err = column.decode::<f64>(&mut dec()).unwrap_err();
        assert!(
            matches!(err, MltError::DictionaryCodeOutOfRange(3, 3)),
            "{err:?}"
        );
    }
}
