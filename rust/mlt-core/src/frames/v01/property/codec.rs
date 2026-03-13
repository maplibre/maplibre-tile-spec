use std::borrow::Cow;

use crate::Decodable as _;
use crate::MltError::{self, NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::decode::{Decode, DecodeInto as _};
use crate::encode::FromDecoded;
use crate::utils::apply_present;
use crate::v01::{
    DictionaryType, EncodedName, EncodedPresence, EncodedProperty, EncodedScalar, EncodedStream,
    EncodedStrings, LengthType, ParsedPresence, ParsedProperty, ParsedScalar, ParsedStrings,
    PresenceStream, Property, PropertyEncoder, RawPresence, RawProperty, ScalarEncoder,
    ScalarValueEncoder, StagedProperty, StrEncoder, encode_shared_dict_prop,
};

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: ParsedProperty<'static> = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Self =
            Self::from_decoded(&decoded, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(prop)
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
fn arbitrary_decoded_scalar<'a, T: arbitrary::Arbitrary<'a> + Copy + PartialEq>(
    u: &mut arbitrary::Unstructured<'a>,
) -> arbitrary::Result<ParsedScalar<'static, T>> {
    Ok(ParsedScalar {
        name: Cow::Owned(u.arbitrary()?),
        values: u.arbitrary()?,
    })
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for ParsedProperty<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range(0..=9)? {
            0 => Self::Bool(arbitrary_decoded_scalar(u)?),
            1 => Self::I8(arbitrary_decoded_scalar(u)?),
            2 => Self::U8(arbitrary_decoded_scalar(u)?),
            3 => Self::I32(arbitrary_decoded_scalar(u)?),
            4 => Self::U32(arbitrary_decoded_scalar(u)?),
            5 => Self::I64(arbitrary_decoded_scalar(u)?),
            6 => Self::U64(arbitrary_decoded_scalar(u)?),
            7 => Self::F32(arbitrary_decoded_scalar(u)?),
            8 => Self::F64(arbitrary_decoded_scalar(u)?),
            _ => Self::Str(u.arbitrary()?),
        })
    }
}

impl<'a> Property<'a> {
    #[inline]
    pub fn decode(self) -> Result<ParsedProperty<'a>, MltError> {
        Ok(match self {
            Self::Encoded(v) => v.decode_into()?,
            Self::Decoded(v) => v,
        })
    }

    pub fn decoded_property(&mut self) -> Result<&ParsedProperty<'a>, MltError> {
        Ok(self.materialize()?)
    }

    #[must_use]
    pub fn to_owned(&self) -> StagedProperty {
        match self {
            Self::Encoded(encoded) => StagedProperty::Encoded(encoded.to_owned()),
            Self::Decoded(decoded) => StagedProperty::Decoded(decoded.to_owned()),
        }
    }
}

impl<'a, T: Copy + PartialEq> ParsedScalar<'a, T> {
    #[must_use]
    pub fn new(name: impl Into<Cow<'a, str>>, values: Vec<Option<T>>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    pub fn from_parts(
        name: impl Into<Cow<'a, str>>,
        presence: RawPresence<'a>,
        values: Vec<T>,
    ) -> Result<Self, MltError> {
        Ok(Self {
            name: name.into(),
            values: apply_present(presence.0, values)?,
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

impl From<Vec<bool>> for ParsedPresence {
    fn from(values: Vec<bool>) -> Self {
        if values.iter().all(|v| *v) {
            Self(None)
        } else {
            Self(Some(values))
        }
    }
}

impl From<Option<Vec<bool>>> for ParsedPresence {
    fn from(values: Option<Vec<bool>>) -> Self {
        match values {
            Some(values) => Self::from(values),
            None => Self::default(),
        }
    }
}

impl<'a> ParsedProperty<'a> {
    #[must_use]
    pub fn bool(name: impl Into<Cow<'a, str>>, values: Vec<Option<bool>>) -> Self {
        Self::Bool(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i8(name: impl Into<Cow<'a, str>>, values: Vec<Option<i8>>) -> Self {
        Self::I8(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u8(name: impl Into<Cow<'a, str>>, values: Vec<Option<u8>>) -> Self {
        Self::U8(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i32(name: impl Into<Cow<'a, str>>, values: Vec<Option<i32>>) -> Self {
        Self::I32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u32(name: impl Into<Cow<'a, str>>, values: Vec<Option<u32>>) -> Self {
        Self::U32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i64(name: impl Into<Cow<'a, str>>, values: Vec<Option<i64>>) -> Self {
        Self::I64(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u64(name: impl Into<Cow<'a, str>>, values: Vec<Option<u64>>) -> Self {
        Self::U64(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn f32(name: impl Into<Cow<'a, str>>, values: Vec<Option<f32>>) -> Self {
        Self::F32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn f64(name: impl Into<Cow<'a, str>>, values: Vec<Option<f64>>) -> Self {
        Self::F64(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn str(name: impl Into<Cow<'a, str>>, values: Vec<Option<String>>) -> Self {
        let mut s = ParsedStrings::from(values);
        s.name = name.into();
        Self::Str(s)
    }
}

impl ParsedProperty<'_> {
    pub(super) fn as_presence_stream(&self) -> Result<Vec<bool>, MltError> {
        Ok(match self {
            Self::Bool(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I8(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U8(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::I64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::U64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::F32(v) => v.values.iter().map(Option::is_some).collect(),
            Self::F64(v) => v.values.iter().map(Option::is_some).collect(),
            Self::Str(v) => v.presence_bools(),
            Self::SharedDict(..) => Err(NotImplemented("presence stream for shared dict"))?,
        })
    }
}

impl FromDecoded<'_> for Vec<EncodedProperty> {
    type Input = Vec<ParsedProperty<'static>>;
    type Encoder = Vec<PropertyEncoder>;

    fn from_decoded(properties: &Self::Input, encoders: Self::Encoder) -> Result<Self, MltError> {
        if properties.len() != encoders.len() {
            return Err(MltError::EncodingInstructionCountMismatch {
                input_len: properties.len(),
                config_len: encoders.len(),
            });
        }

        let mut result = Vec::with_capacity(properties.len());

        for (prop, encoder) in properties.iter().zip(encoders) {
            match encoder {
                PropertyEncoder::Scalar(enc) => {
                    result.push(EncodedProperty::from_decoded(prop, enc)?);
                }
                PropertyEncoder::SharedDict(enc) => {
                    let ParsedProperty::SharedDict(shared_dict) = prop else {
                        return Err(UnsupportedPropertyEncoderCombination(
                            prop.into(),
                            "shared_dict",
                        ));
                    };
                    result.push(encode_shared_dict_prop(shared_dict, &enc)?);
                }
            }
        }

        Ok(result)
    }
}

impl FromDecoded<'_> for EncodedProperty {
    type Input = ParsedProperty<'static>;
    type Encoder = ScalarEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError> {
        use ParsedProperty as D;
        let presence = if encoder.presence == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.as_presence_stream()?;
            Some(EncodedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        let mk_scalar =
            |name: &str, presence: Option<EncodedStream>, data: EncodedStream| EncodedScalar {
                name: EncodedName(name.to_string()),
                presence: EncodedPresence(presence),
                data,
            };

        match (decoded, encoder.value) {
            (D::Bool(v), ScalarValueEncoder::Bool) => Ok(Self::Bool(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_bools(&unapply_presence(&v.values))?,
            ))),
            (D::I8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I8(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i8s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::U8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U8(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u8s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::I32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i32s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::U32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u32s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::I64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i64s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::U64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u64s(&unapply_presence(&v.values), enc)?,
            ))),
            (D::F32(v), ScalarValueEncoder::Float) => Ok(Self::F32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_f32(&unapply_presence(&v.values))?,
            ))),
            (D::F64(v), ScalarValueEncoder::Float) => Ok(Self::F64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_f64(&unapply_presence(&v.values))?,
            ))),
            (D::Str(v), ScalarValueEncoder::String(enc)) => Ok(Self::Str(EncodedStrings {
                name: EncodedName(v.name.as_ref().to_string()),
                presence: EncodedPresence(presence),
                encoding: match enc {
                    StrEncoder::Plain { string_lengths } => {
                        EncodedStream::encode_strings_with_type(
                            &v.dense_values(),
                            string_lengths,
                            LengthType::VarBinary,
                            DictionaryType::None,
                        )?
                    }
                    StrEncoder::Fsst(enc) => EncodedStream::encode_strings_fsst_with_type(
                        &v.dense_values(),
                        enc,
                        DictionaryType::Single,
                    )?,
                },
            })),
            (D::SharedDict(..), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.into(), e.into()))?,
        }
    }
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl<'a> TryFrom<RawProperty<'a>> for ParsedProperty<'a> {
    type Error = MltError;

    fn try_from(raw: RawProperty<'a>) -> Result<Self, MltError> {
        ParsedProperty::decode(raw)
    }
}

impl<'a> Decode<RawProperty<'a>> for ParsedProperty<'a> {
    fn decode(v: RawProperty<'a>) -> Result<ParsedProperty<'a>, MltError> {
        use RawProperty as E;
        Ok(match v {
            E::Bool(s) => Self::Bool(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::I8(s) => Self::I8(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::U8(s) => Self::U8(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::I32(s) => Self::I32(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::U32(s) => Self::U32(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::I64(s) => Self::I64(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::U64(s) => Self::U64(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::F32(s) => Self::F32(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::F64(s) => Self::F64(ParsedScalar::from_parts(
                s.name,
                s.presence,
                s.data.decode_into()?,
            )?),
            E::Str(s) => Self::Str(s.into_decoded()?),
            E::SharedDict(s) => Self::SharedDict(s.into_decoded()?),
        })
    }
}
