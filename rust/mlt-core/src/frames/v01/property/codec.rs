// TODO: break this into decode.rs and encode.rs

use std::mem::size_of;

use crate::Decoder;
use crate::MltError::{self, NotImplemented, UnsupportedPropertyEncoderCombination};
use crate::enc_dec::Decode;
use crate::errors::AsMltError as _;
use crate::utils::apply_present;
use crate::v01::{
    DictionaryType, EncodedName, EncodedPresence, EncodedProperty, EncodedScalar, EncodedStream,
    EncodedStrings, LengthType, ParsedPresence, ParsedProperty, ParsedScalar, PresenceStream,
    PropertyEncoder, RawPresence, RawProperty, ScalarEncoder, ScalarValueEncoder, StagedProperty,
    StagedScalar, StagedStrings, StrEncoder, encode_shared_dict_prop,
};

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: StagedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Self =
            Self::encode(&decoded, encoder).map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(prop)
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let values: Vec<Option<u32>> = u.arbitrary()?;
        Ok(Self::u32("prop", values))
    }
}

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
    ) -> Result<Self, MltError> {
        Ok(Self {
            name,
            values: apply_present(presence.0, values, dec)?,
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

impl<'a> ParsedProperty<'a> {
    #[must_use]
    pub fn bool(name: &'a str, values: Vec<Option<bool>>) -> Self {
        Self::Bool(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i8(name: &'a str, values: Vec<Option<i8>>) -> Self {
        Self::I8(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u8(name: &'a str, values: Vec<Option<u8>>) -> Self {
        Self::U8(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i32(name: &'a str, values: Vec<Option<i32>>) -> Self {
        Self::I32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u32(name: &'a str, values: Vec<Option<u32>>) -> Self {
        Self::U32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn i64(name: &'a str, values: Vec<Option<i64>>) -> Self {
        Self::I64(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn u64(name: &'a str, values: Vec<Option<u64>>) -> Self {
        Self::U64(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn f32(name: &'a str, values: Vec<Option<f32>>) -> Self {
        Self::F32(ParsedScalar::new(name, values))
    }
    #[must_use]
    pub fn f64(name: &'a str, values: Vec<Option<f64>>) -> Self {
        Self::F64(ParsedScalar::new(name, values))
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
        let mut s = StagedStrings::from(values);
        s.name = name.into();
        Self::Str(s)
    }
}

impl StagedProperty {
    fn as_presence_stream(&self) -> Result<Vec<bool>, MltError> {
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

pub(crate) fn encode_properties(
    value: &[StagedProperty],
    encoders: Vec<PropertyEncoder>,
) -> Result<Vec<EncodedProperty>, MltError> {
    if value.len() != encoders.len() {
        return Err(MltError::EncodingInstructionCountMismatch {
            input_len: value.len(),
            config_len: encoders.len(),
        });
    }

    let mut result = Vec::with_capacity(value.len());

    for (prop, encoder) in value.iter().zip(encoders) {
        match encoder {
            PropertyEncoder::Scalar(enc) => {
                result.push(EncodedProperty::encode(prop, enc)?);
            }
            PropertyEncoder::SharedDict(enc) => {
                let StagedProperty::SharedDict(shared_dict) = prop else {
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

impl EncodedProperty {
    pub(crate) fn encode(value: &StagedProperty, encoder: ScalarEncoder) -> Result<Self, MltError> {
        use StagedProperty as D;
        let presence = if encoder.presence == PresenceStream::Present {
            let present_vec: Vec<bool> = value.as_presence_stream()?;
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

        match (value, encoder.value) {
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
                name: EncodedName(v.name.clone()),
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

impl<'a> Decode<ParsedProperty<'a>> for RawProperty<'a> {
    fn decode(self, decoder: &mut Decoder) -> Result<ParsedProperty<'a>, MltError> {
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
    pub fn decode(self, dec: &mut Decoder) -> Result<ParsedProperty<'a>, MltError> {
        /// Charge for the final `Vec<Option<T>>`, then decode the dense stream.
        /// `$decode_method` is the typed `RawStream` method for element type `$ty`.
        macro_rules! scalar_decode {
            ($variant:ident, $ty:ty, $decode_method:ident, $s:expr) => {{
                let s = $s;
                let feature_count = s
                    .presence
                    .0
                    .as_ref()
                    .map_or(s.data.meta.num_values, |p| p.meta.num_values);
                dec.consume(
                    feature_count
                        .saturating_mul(u32::try_from(size_of::<Option<$ty>>()).or_overflow()?),
                )?;
                ParsedProperty::$variant(ParsedScalar::from_parts(
                    s.name,
                    s.presence,
                    s.data.$decode_method(dec)?,
                    dec,
                )?)
            }};
        }

        Ok(match self {
            Self::Bool(s) => scalar_decode!(Bool, bool, decode_bools, s),
            Self::I8(s) => scalar_decode!(I8, i8, decode_i8s, s),
            Self::U8(s) => scalar_decode!(U8, u8, decode_u8s, s),
            Self::I32(s) => scalar_decode!(I32, i32, decode_i32s, s),
            Self::U32(s) => scalar_decode!(U32, u32, decode_u32s, s),
            Self::I64(s) => scalar_decode!(I64, i64, decode_i64s, s),
            Self::U64(s) => scalar_decode!(U64, u64, decode_u64s, s),
            Self::F32(s) => scalar_decode!(F32, f32, decode_f32s, s),
            Self::F64(s) => scalar_decode!(F64, f64, decode_f64s, s),
            Self::Str(s) => ParsedProperty::Str(s.decode(dec)?),
            Self::SharedDict(s) => ParsedProperty::SharedDict(s.decode(dec)?),
        })
    }
}

impl StagedProperty {
    /// Returns the column name regardless of variant.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v) => &v.name,
            Self::I8(v) => &v.name,
            Self::U8(v) => &v.name,
            Self::I32(v) => &v.name,
            Self::U32(v) => &v.name,
            Self::I64(v) => &v.name,
            Self::U64(v) => &v.name,
            Self::F32(v) => &v.name,
            Self::F64(v) => &v.name,
            Self::Str(v) => &v.name,
            Self::SharedDict(v) => &v.prefix,
        }
    }
}

// Keep `arbitrary::Arbitrary` for StagedStrings (used in fuzz and tests via the arbitrary feature)
#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self::from(u.arbitrary::<Vec<Option<String>>>()?))
    }
}
