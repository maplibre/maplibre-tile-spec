use super::model::{
    EncodedName, EncodedPresence, EncodedProperty, EncodedScalar, EncodedStrings, PresenceKind,
    PropertyEncoder, ScalarEncoder, ScalarValueEncoder, StagedProperty, StrEncoder,
};
use super::strings::encode_shared_dict_prop;
use crate::MltError::{
    EncodingInstructionCountMismatch, NotImplemented, UnsupportedPropertyEncoderCombination,
};
use crate::MltResult;
use crate::encoder::EncodedStream;
use crate::v01::{DictionaryType, LengthType};

pub fn encode_properties(
    value: &[StagedProperty],
    encoders: Vec<PropertyEncoder>,
) -> MltResult<Vec<Option<EncodedProperty>>> {
    if value.len() != encoders.len() {
        return Err(EncodingInstructionCountMismatch {
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
    pub(crate) fn encode(
        value: &StagedProperty,
        encoder: ScalarEncoder,
    ) -> MltResult<Option<Self>> {
        use PresenceKind as Kind;
        use StagedProperty as D;

        let kind = value.presence();

        #[cfg(feature = "__private")]
        let kind = if encoder.forced_presence {
            Kind::Mixed
        } else {
            kind
        };

        let presence = match kind {
            Kind::Mixed => Some(EncodedStream::encode_presence(
                &value.as_presence_stream()?,
            )?),
            Kind::AllPresent => None,
            Kind::Empty | Kind::AllNull => return Ok(None),
        };

        let mk_scalar =
            |name: &str, presence: Option<EncodedStream>, data: EncodedStream| EncodedScalar {
                name: EncodedName(name.to_string()),
                presence: EncodedPresence(presence),
                data,
            };

        Ok(Some(match (value, encoder.value) {
            (D::Bool(v), ScalarValueEncoder::Bool) => Self::Bool(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_bools(&unapply_presence(&v.values))?,
            )),
            (D::I8(v), ScalarValueEncoder::Int(enc)) => Self::I8(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i8s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U8(v), ScalarValueEncoder::Int(enc)) => Self::U8(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u8s(&unapply_presence(&v.values), enc)?,
            )),
            (D::I32(v), ScalarValueEncoder::Int(enc)) => Self::I32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i32s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U32(v), ScalarValueEncoder::Int(enc)) => Self::U32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u32s(&unapply_presence(&v.values), enc)?,
            )),
            (D::I64(v), ScalarValueEncoder::Int(enc)) => Self::I64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_i64s(&unapply_presence(&v.values), enc)?,
            )),
            (D::U64(v), ScalarValueEncoder::Int(enc)) => Self::U64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_u64s(&unapply_presence(&v.values), enc)?,
            )),
            (D::F32(v), ScalarValueEncoder::Float) => Self::F32(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_f32(&unapply_presence(&v.values))?,
            )),
            (D::F64(v), ScalarValueEncoder::Float) => Self::F64(mk_scalar(
                &v.name,
                presence,
                EncodedStream::encode_f64(&unapply_presence(&v.values))?,
            )),
            (D::Str(v), ScalarValueEncoder::String(enc)) => {
                let dense_values = v.dense_values();
                Self::Str(EncodedStrings {
                    name: EncodedName(v.name.clone()),
                    presence: EncodedPresence(presence),
                    encoding: match enc {
                        StrEncoder::Plain { string_lengths } => {
                            EncodedStream::encode_strings_with_type(
                                &dense_values,
                                string_lengths,
                                LengthType::VarBinary,
                                DictionaryType::None,
                            )?
                        }
                        StrEncoder::Dict {
                            string_lengths,
                            offsets,
                        } => EncodedStream::encode_strings_dict(
                            &dense_values,
                            string_lengths,
                            offsets,
                        )?,
                        StrEncoder::Fsst(enc) => EncodedStream::encode_strings_fsst_with_type(
                            &dense_values,
                            enc,
                            DictionaryType::Single,
                        )?,
                        StrEncoder::FsstDict { fsst, offsets } => {
                            EncodedStream::encode_strings_fsst_dict(&dense_values, fsst, offsets)?
                        }
                    },
                })
            }
            (D::SharedDict(..), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.into(), e.into()))?,
        }))
    }
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

fn presence_of_options<T>(values: &[Option<T>]) -> PresenceKind {
    let mut has_null = false;
    let mut has_present = false;
    for v in values {
        if v.is_none() {
            has_null = true;
        } else {
            has_present = true;
        }
        if has_null && has_present {
            return PresenceKind::Mixed;
        }
    }
    match (has_null, has_present) {
        (false, false) => PresenceKind::Empty,
        (false, true) => PresenceKind::AllPresent,
        (true, false) => PresenceKind::AllNull,
        (true, true) => unreachable!("early return handles Mixed"),
    }
}

impl StagedProperty {
    fn as_presence_stream(&self) -> MltResult<Vec<bool>> {
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

    #[must_use]
    pub fn presence(&self) -> PresenceKind {
        match self {
            Self::Bool(v) => presence_of_options(&v.values),
            Self::I8(v) => presence_of_options(&v.values),
            Self::U8(v) => presence_of_options(&v.values),
            Self::I32(v) => presence_of_options(&v.values),
            Self::U32(v) => presence_of_options(&v.values),
            Self::I64(v) => presence_of_options(&v.values),
            Self::U64(v) => presence_of_options(&v.values),
            Self::F32(v) => presence_of_options(&v.values),
            Self::F64(v) => presence_of_options(&v.values),
            Self::Str(v) => v.presence(),
            Self::SharedDict(..) => PresenceKind::Empty,
        }
    }

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
