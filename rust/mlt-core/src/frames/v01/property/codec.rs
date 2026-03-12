use std::borrow::Cow;

use crate::Decodable as _;
use crate::MltError::{self, NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::decode::{FromEncoded, impl_decodable};
use crate::encode::{FromDecoded, impl_encodable};
use crate::utils::apply_present;
use crate::v01::{
    DecodedOptScalar, DecodedPresence, DecodedProperty, DecodedScalar, DecodedStrings,
    DictionaryType, EncodedPresence, EncodedProperty, LengthType, OwnedEncodedPresence,
    OwnedEncodedProperty, OwnedName, OwnedProperty, OwnedStream, PresenceStream, Property,
    PropertyEncoder, ScalarEncoder, ScalarValueEncoder, StrEncoder, decode_shared_dict,
    decode_strings, encode_shared_dict_prop,
};

impl_decodable!(Property<'a>, EncodedProperty<'a>, DecodedProperty<'a>);
impl_encodable!(
    OwnedProperty,
    DecodedProperty<'static>,
    OwnedEncodedProperty
);

/// FIXME: why should there be a default???
impl Default for OwnedEncodedProperty {
    fn default() -> Self {
        Self::Bool(
            OwnedName(String::default()),
            OwnedStream::empty_without_encoding(),
        )
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedEncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: DecodedProperty<'static> = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Self =
            Self::from_decoded(&decoded, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(prop)
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
fn arbitrary_decoded_scalar<'a, T: arbitrary::Arbitrary<'a> + Copy + PartialEq>(
    u: &mut arbitrary::Unstructured<'a>,
) -> arbitrary::Result<DecodedScalar<'static, T>> {
    Ok(DecodedScalar {
        name: Cow::Owned(u.arbitrary()?),
        values: u.arbitrary()?,
    })
}

#[cfg(all(not(test), feature = "arbitrary"))]
fn arbitrary_decoded_opt_scalar<'a, T: arbitrary::Arbitrary<'a> + Copy + PartialEq>(
    u: &mut arbitrary::Unstructured<'a>,
) -> arbitrary::Result<DecodedOptScalar<'static, T>> {
    Ok(DecodedOptScalar {
        name: Cow::Owned(u.arbitrary()?),
        values: u.arbitrary()?,
    })
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for DecodedProperty<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range(0..=19)? {
            0 => Self::Bool(arbitrary_decoded_scalar(u)?),
            1 => Self::BoolOpt(arbitrary_decoded_opt_scalar(u)?),
            2 => Self::I8(arbitrary_decoded_scalar(u)?),
            3 => Self::I8Opt(arbitrary_decoded_opt_scalar(u)?),
            4 => Self::U8(arbitrary_decoded_scalar(u)?),
            5 => Self::U8Opt(arbitrary_decoded_opt_scalar(u)?),
            6 => Self::I32(arbitrary_decoded_scalar(u)?),
            7 => Self::I32Opt(arbitrary_decoded_opt_scalar(u)?),
            8 => Self::U32(arbitrary_decoded_scalar(u)?),
            9 => Self::U32Opt(arbitrary_decoded_opt_scalar(u)?),
            10 => Self::I64(arbitrary_decoded_scalar(u)?),
            11 => Self::I64Opt(arbitrary_decoded_opt_scalar(u)?),
            12 => Self::U64(arbitrary_decoded_scalar(u)?),
            13 => Self::U64Opt(arbitrary_decoded_opt_scalar(u)?),
            14 => Self::F32(arbitrary_decoded_scalar(u)?),
            15 => Self::F32Opt(arbitrary_decoded_opt_scalar(u)?),
            16 => Self::F64(arbitrary_decoded_scalar(u)?),
            17 => Self::F64Opt(arbitrary_decoded_opt_scalar(u)?),
            _ => Self::Str(u.arbitrary()?),
        })
    }
}

impl<'a> Property<'a> {
    #[inline]
    pub fn decode(self) -> Result<DecodedProperty<'a>, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedProperty::from_encoded(v)?,
            Self::Decoded(v) => v,
        })
    }

    pub fn decoded_property(&mut self) -> Result<&DecodedProperty<'a>, MltError> {
        Ok(self.materialize()?)
    }
}

impl<'a, T: Copy + PartialEq> DecodedScalar<'a, T> {
    #[must_use]
    pub fn new(name: impl Into<Cow<'a, str>>, values: Vec<T>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    pub fn from_parts(name: impl Into<Cow<'a, str>>, values: Vec<T>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}

impl<'a, T: Copy + PartialEq> DecodedOptScalar<'a, T> {
    #[must_use]
    pub fn new(name: impl Into<Cow<'a, str>>, values: Vec<Option<T>>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    pub fn from_parts(
        name: impl Into<Cow<'a, str>>,
        presence: EncodedPresence,
        values: Vec<T>,
    ) -> Result<Self, MltError> {
        Ok(Self {
            name: name.into(),
            values: apply_present(presence.0, values)?,
        })
    }
}

impl DecodedPresence {
    #[must_use]
    pub fn bools(&self, non_null_count: usize) -> Vec<bool> {
        self.0.clone().unwrap_or_else(|| vec![true; non_null_count])
    }

    #[must_use]
    pub fn feature_count(&self, non_null_count: usize) -> usize {
        self.0.as_ref().map_or(non_null_count, Vec::len)
    }
}

impl From<Vec<bool>> for DecodedPresence {
    fn from(values: Vec<bool>) -> Self {
        if values.iter().all(|v| *v) {
            Self(None)
        } else {
            Self(Some(values))
        }
    }
}

impl From<Option<Vec<bool>>> for DecodedPresence {
    fn from(values: Option<Vec<bool>>) -> Self {
        match values {
            Some(values) => Self::from(values),
            None => Self::default(),
        }
    }
}

impl<'a> DecodedProperty<'a> {
    #[must_use]
    pub fn bool(name: impl Into<Cow<'a, str>>, values: Vec<bool>) -> Self {
        Self::Bool(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn bool_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<bool>>) -> Self {
        Self::BoolOpt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn i8(name: impl Into<Cow<'a, str>>, values: Vec<i8>) -> Self {
        Self::I8(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i8_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<i8>>) -> Self {
        Self::I8Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn u8(name: impl Into<Cow<'a, str>>, values: Vec<u8>) -> Self {
        Self::U8(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u8_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<u8>>) -> Self {
        Self::U8Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn i32(name: impl Into<Cow<'a, str>>, values: Vec<i32>) -> Self {
        Self::I32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i32_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<i32>>) -> Self {
        Self::I32Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn u32(name: impl Into<Cow<'a, str>>, values: Vec<u32>) -> Self {
        Self::U32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u32_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<u32>>) -> Self {
        Self::U32Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn i64(name: impl Into<Cow<'a, str>>, values: Vec<i64>) -> Self {
        Self::I64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i64_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<i64>>) -> Self {
        Self::I64Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn u64(name: impl Into<Cow<'a, str>>, values: Vec<u64>) -> Self {
        Self::U64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u64_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<u64>>) -> Self {
        Self::U64Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn f32(name: impl Into<Cow<'a, str>>, values: Vec<f32>) -> Self {
        Self::F32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn f32_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<f32>>) -> Self {
        Self::F32Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn f64(name: impl Into<Cow<'a, str>>, values: Vec<f64>) -> Self {
        Self::F64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn f64_opt(name: impl Into<Cow<'a, str>>, values: Vec<Option<f64>>) -> Self {
        Self::F64Opt(DecodedOptScalar::new(name, values))
    }
    #[must_use]
    pub fn str(name: impl Into<Cow<'a, str>>, values: Vec<Option<String>>) -> Self {
        let mut s = DecodedStrings::from(values);
        s.name = name.into();
        Self::Str(s)
    }
}

impl FromDecoded<'_> for Vec<OwnedEncodedProperty> {
    type Input = Vec<DecodedProperty<'static>>;
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
                    result.push(OwnedEncodedProperty::from_decoded(prop, enc)?);
                }
                PropertyEncoder::SharedDict(enc) => {
                    let DecodedProperty::SharedDict(shared_dict) = prop else {
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

impl FromDecoded<'_> for OwnedEncodedProperty {
    type Input = DecodedProperty<'static>;
    type Encoder = ScalarEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError> {
        use DecodedProperty as D;

        match (decoded, encoder.value) {
            (D::Bool(v), ScalarValueEncoder::Bool) => Ok(Self::Bool(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_bools(&v.values)?,
            )),
            (D::BoolOpt(v), ScalarValueEncoder::Bool) => Ok(Self::BoolOpt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_bools(&unapply_presence_opt(&v.values))?,
            )),
            (D::I8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I8(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_i8s(&v.values, enc)?,
            )),
            (D::I8Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I8Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_i8s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::U8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U8(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_u8s(&v.values, enc)?,
            )),
            (D::U8Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U8Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_u8s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::I32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_i32s(&v.values, enc)?,
            )),
            (D::I32Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I32Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_i32s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::U32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_u32s(&v.values, enc)?,
            )),
            (D::U32Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U32Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_u32s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::I64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_i64s(&v.values, enc)?,
            )),
            (D::I64Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I64Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_i64s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::U64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_u64s(&v.values, enc)?,
            )),
            (D::U64Opt(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U64Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_u64s(&unapply_presence_opt(&v.values), enc)?,
            )),
            (D::F32(v), ScalarValueEncoder::Float) => Ok(Self::F32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_f32(&v.values)?,
            )),
            (D::F32Opt(v), ScalarValueEncoder::Float) => Ok(Self::F32Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_f32(&unapply_presence_opt(&v.values))?,
            )),
            (D::F64(v), ScalarValueEncoder::Float) => Ok(Self::F64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedStream::encode_f64(&v.values)?,
            )),
            (D::F64Opt(v), ScalarValueEncoder::Float) => Ok(Self::F64Opt(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence(encoder.presence, &v.values)?),
                OwnedStream::encode_f64(&unapply_presence_opt(&v.values))?,
            )),
            (D::Str(v), ScalarValueEncoder::String(enc)) => Ok(Self::Str(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(encode_opt_presence_bools(
                    encoder.presence,
                    &v.presence_bools(),
                )?),
                match enc {
                    StrEncoder::Plain { string_lengths } => OwnedStream::encode_strings_with_type(
                        &v.dense_values(),
                        string_lengths,
                        LengthType::VarBinary,
                        DictionaryType::None,
                    )?,
                    StrEncoder::Fsst(enc) => OwnedStream::encode_strings_fsst_with_type(
                        &v.dense_values(),
                        enc,
                        DictionaryType::Single,
                    )?,
                },
            )),
            (D::SharedDict(..), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.into(), e.into()))?,
        }
    }
}

fn unapply_presence_opt<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

fn encode_opt_presence<T>(
    mode: PresenceStream,
    values: &[Option<T>],
) -> Result<Option<OwnedStream>, MltError> {
    if mode == PresenceStream::Present {
        let present_vec: Vec<bool> = values.iter().map(Option::is_some).collect();
        Ok(Some(OwnedStream::encode_presence(&present_vec)?))
    } else {
        Ok(None)
    }
}

fn encode_opt_presence_bools(
    mode: PresenceStream,
    presence_bools: &[bool],
) -> Result<Option<OwnedStream>, MltError> {
    if mode == PresenceStream::Present {
        Ok(Some(OwnedStream::encode_presence(presence_bools)?))
    } else {
        Ok(None)
    }
}

impl<'a> FromEncoded<'a> for DecodedProperty<'a> {
    type Input = EncodedProperty<'a>;

    fn from_encoded(v: EncodedProperty<'a>) -> Result<DecodedProperty<'a>, MltError> {
        use EncodedProperty as E;
        Ok(match v {
            E::Bool(name, data) => {
                Self::Bool(DecodedScalar::from_parts(name, data.decode_bools()?))
            }
            E::BoolOpt(name, presence, data) => Self::BoolOpt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_bools()?,
            )?),
            E::I8(name, data) => Self::I8(DecodedScalar::from_parts(name, data.decode_i8s()?)),
            E::I8Opt(name, presence, data) => Self::I8Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_i8s()?,
            )?),
            E::U8(name, data) => Self::U8(DecodedScalar::from_parts(name, data.decode_u8s()?)),
            E::U8Opt(name, presence, data) => Self::U8Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_u8s()?,
            )?),
            E::I32(name, data) => Self::I32(DecodedScalar::from_parts(name, data.decode_i32s()?)),
            E::I32Opt(name, presence, data) => Self::I32Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_i32s()?,
            )?),
            E::U32(name, data) => Self::U32(DecodedScalar::from_parts(name, data.decode_u32s()?)),
            E::U32Opt(name, presence, data) => Self::U32Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_u32s()?,
            )?),
            E::I64(name, data) => Self::I64(DecodedScalar::from_parts(name, data.decode_i64()?)),
            E::I64Opt(name, presence, data) => Self::I64Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_i64()?,
            )?),
            E::U64(name, data) => Self::U64(DecodedScalar::from_parts(name, data.decode_u64()?)),
            E::U64Opt(name, presence, data) => Self::U64Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_u64()?,
            )?),
            E::F32(name, data) => Self::F32(DecodedScalar::from_parts(name, data.decode_f32()?)),
            E::F32Opt(name, presence, data) => Self::F32Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_f32()?,
            )?),
            E::F64(name, data) => Self::F64(DecodedScalar::from_parts(name, data.decode_f64()?)),
            E::F64Opt(name, presence, data) => Self::F64Opt(DecodedOptScalar::from_parts(
                name,
                presence,
                data.decode_f64()?,
            )?),
            E::Str(name, presence, s) => Self::Str(decode_strings(name, presence, s)?),
            E::SharedDict(prefix, sd, children) => {
                Self::SharedDict(decode_shared_dict(prefix.0, &sd, &children)?)
            }
        })
    }
}
