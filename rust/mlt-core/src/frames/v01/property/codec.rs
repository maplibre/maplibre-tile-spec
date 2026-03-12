use std::borrow::Cow;

use crate::Decodable as _;
use crate::MltError::{self, NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::decode::{Decode, DecodeInto as _};
use crate::encode::FromDecoded;
use crate::utils::apply_present;
use crate::v01::{
    DecodedPresence, DecodedProperty, DecodedScalar, DecodedStrings, DictionaryType,
    EncodedPresence, EncodedProperty, EncodedSharedDict, EncodedSharedDictChild, EncodedStrings,
    FsstData, LengthType, NameRef, OwnedEncodedPresence, OwnedEncodedProperty, OwnedName,
    OwnedProperty, OwnedStream, PlainData, PresenceStream, Property, PropertyEncoder,
    ScalarEncoder, ScalarValueEncoder, StrEncoder, decode_shared_dict, decode_strings,
    encode_shared_dict_prop,
};

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
impl<'a> arbitrary::Arbitrary<'a> for DecodedProperty<'static> {
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
    pub fn decode(self) -> Result<DecodedProperty<'a>, MltError> {
        Ok(match self {
            Self::Encoded(v) => v.decode_into()?,
            Self::Decoded(v) => v,
        })
    }

    pub fn decoded_property(&mut self) -> Result<&DecodedProperty<'a>, MltError> {
        Ok(self.materialize()?)
    }

    #[must_use]
    pub fn to_owned(&self) -> OwnedProperty {
        match self {
            Self::Encoded(encoded) => OwnedProperty::Encoded(encoded.to_owned()),
            Self::Decoded(decoded) => OwnedProperty::Decoded(decoded.to_owned()),
        }
    }
}

impl TryFrom<OwnedEncodedProperty> for DecodedProperty<'static> {
    type Error = MltError;

    fn try_from(encoded: OwnedEncodedProperty) -> Result<Self, MltError> {
        let borrowed: EncodedProperty<'_> = match &encoded {
            OwnedEncodedProperty::Bool(name, pres, data) => EncodedProperty::Bool(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::I8(name, pres, data) => EncodedProperty::I8(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::U8(name, pres, data) => EncodedProperty::U8(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::I32(name, pres, data) => EncodedProperty::I32(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::U32(name, pres, data) => EncodedProperty::U32(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::I64(name, pres, data) => EncodedProperty::I64(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::U64(name, pres, data) => EncodedProperty::U64(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::F32(name, pres, data) => EncodedProperty::F32(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::F64(name, pres, data) => EncodedProperty::F64(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                data.as_borrowed(),
            ),
            OwnedEncodedProperty::Str(name, pres, strings) => EncodedProperty::Str(
                NameRef(&name.0),
                EncodedPresence(pres.0.as_ref().map(|s| s.as_borrowed())),
                match strings {
                    crate::v01::OwnedEncodedStrings::Plain(d) => EncodedStrings::Plain(PlainData {
                        lengths: d.lengths.as_borrowed(),
                        data: d.data.as_borrowed(),
                    }),
                    crate::v01::OwnedEncodedStrings::Dictionary {
                        plain_data,
                        offsets,
                    } => EncodedStrings::Dictionary {
                        plain_data: PlainData {
                            lengths: plain_data.lengths.as_borrowed(),
                            data: plain_data.data.as_borrowed(),
                        },
                        offsets: offsets.as_borrowed(),
                    },
                    crate::v01::OwnedEncodedStrings::FsstPlain(d) => {
                        EncodedStrings::FsstPlain(FsstData {
                            symbol_lengths: d.symbol_lengths.as_borrowed(),
                            symbol_table: d.symbol_table.as_borrowed(),
                            lengths: d.lengths.as_borrowed(),
                            corpus: d.corpus.as_borrowed(),
                        })
                    }
                    crate::v01::OwnedEncodedStrings::FsstDictionary { fsst_data, offsets } => {
                        EncodedStrings::FsstDictionary {
                            fsst_data: FsstData {
                                symbol_lengths: fsst_data.symbol_lengths.as_borrowed(),
                                symbol_table: fsst_data.symbol_table.as_borrowed(),
                                lengths: fsst_data.lengths.as_borrowed(),
                                corpus: fsst_data.corpus.as_borrowed(),
                            },
                            offsets: offsets.as_borrowed(),
                        }
                    }
                },
            ),
            OwnedEncodedProperty::SharedDict(name, dict, children) => EncodedProperty::SharedDict(
                NameRef(&name.0),
                match dict {
                    crate::v01::OwnedEncodedSharedDict::Plain(d) => {
                        EncodedSharedDict::Plain(PlainData {
                            lengths: d.lengths.as_borrowed(),
                            data: d.data.as_borrowed(),
                        })
                    }
                    crate::v01::OwnedEncodedSharedDict::FsstPlain(d) => {
                        EncodedSharedDict::FsstPlain(FsstData {
                            symbol_lengths: d.symbol_lengths.as_borrowed(),
                            symbol_table: d.symbol_table.as_borrowed(),
                            lengths: d.lengths.as_borrowed(),
                            corpus: d.corpus.as_borrowed(),
                        })
                    }
                },
                children
                    .iter()
                    .map(|c| EncodedSharedDictChild {
                        name: NameRef(&c.name.0),
                        presence: EncodedPresence(c.presence.0.as_ref().map(|s| s.as_borrowed())),
                        data: c.data.as_borrowed(),
                    })
                    .collect(),
            ),
        };
        Ok(<DecodedProperty<'_> as Decode<EncodedProperty<'_>>>::decode(borrowed)?.to_owned())
    }
}

impl TryFrom<OwnedProperty> for DecodedProperty<'static> {
    type Error = MltError;

    fn try_from(owned: OwnedProperty) -> Result<Self, MltError> {
        match owned {
            OwnedProperty::Encoded(encoded) => DecodedProperty::try_from(encoded),
            OwnedProperty::Decoded(decoded) => Ok(decoded.to_owned()),
        }
    }
}

impl<'a, T: Copy + PartialEq> DecodedScalar<'a, T> {
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
    pub fn bool(name: impl Into<Cow<'a, str>>, values: Vec<Option<bool>>) -> Self {
        Self::Bool(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i8(name: impl Into<Cow<'a, str>>, values: Vec<Option<i8>>) -> Self {
        Self::I8(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u8(name: impl Into<Cow<'a, str>>, values: Vec<Option<u8>>) -> Self {
        Self::U8(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i32(name: impl Into<Cow<'a, str>>, values: Vec<Option<i32>>) -> Self {
        Self::I32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u32(name: impl Into<Cow<'a, str>>, values: Vec<Option<u32>>) -> Self {
        Self::U32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn i64(name: impl Into<Cow<'a, str>>, values: Vec<Option<i64>>) -> Self {
        Self::I64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn u64(name: impl Into<Cow<'a, str>>, values: Vec<Option<u64>>) -> Self {
        Self::U64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn f32(name: impl Into<Cow<'a, str>>, values: Vec<Option<f32>>) -> Self {
        Self::F32(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn f64(name: impl Into<Cow<'a, str>>, values: Vec<Option<f64>>) -> Self {
        Self::F64(DecodedScalar::new(name, values))
    }
    #[must_use]
    pub fn str(name: impl Into<Cow<'a, str>>, values: Vec<Option<String>>) -> Self {
        let mut s = DecodedStrings::from(values);
        s.name = name.into();
        Self::Str(s)
    }
}

impl DecodedProperty<'_> {
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
        let presence = if encoder.presence == PresenceStream::Present {
            let present_vec: Vec<bool> = decoded.as_presence_stream()?;
            Some(OwnedStream::encode_presence(&present_vec)?)
        } else {
            None
        };

        match (decoded, encoder.value) {
            (D::Bool(v), ScalarValueEncoder::Bool) => Ok(Self::Bool(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_bools(&unapply_presence(&v.values))?,
            )),
            (D::I8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I8(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i8s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U8(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U8(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u8s(&unapply_presence(&v.values), enc)?,
            )),
            (D::I32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i32s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U32(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u32s(&unapply_presence(&v.values), enc)?,
            )),
            (D::I64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::I64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_i64s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U64(v), ScalarValueEncoder::Int(enc)) => Ok(Self::U64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_u64s(&unapply_presence(&v.values), enc)?,
            )),
            (D::F32(v), ScalarValueEncoder::Float) => Ok(Self::F32(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_f32(&unapply_presence(&v.values))?,
            )),
            (D::F64(v), ScalarValueEncoder::Float) => Ok(Self::F64(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence.clone()),
                OwnedStream::encode_f64(&unapply_presence(&v.values))?,
            )),
            (D::Str(v), ScalarValueEncoder::String(enc)) => Ok(Self::Str(
                OwnedName(v.name.as_ref().to_string()),
                OwnedEncodedPresence(presence),
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

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl<'a> Decode<EncodedProperty<'a>> for DecodedProperty<'a> {
    fn decode(v: EncodedProperty<'a>) -> Result<DecodedProperty<'a>, MltError> {
        use EncodedProperty as E;
        Ok(match v {
            E::Bool(name, presence, data) => Self::Bool(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::I8(name, presence, data) => Self::I8(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::U8(name, presence, data) => Self::U8(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::I32(name, presence, data) => Self::I32(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::U32(name, presence, data) => Self::U32(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::I64(name, presence, data) => Self::I64(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::U64(name, presence, data) => Self::U64(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::F32(name, presence, data) => Self::F32(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::F64(name, presence, data) => Self::F64(DecodedScalar::from_parts(
                name,
                presence,
                data.decode_into()?,
            )?),
            E::Str(name, presence, s) => Self::Str(decode_strings(name, presence, s)?),
            E::SharedDict(prefix, sd, children) => {
                Self::SharedDict(decode_shared_dict(prefix.0, &sd, &children)?)
            }
        })
    }
}
