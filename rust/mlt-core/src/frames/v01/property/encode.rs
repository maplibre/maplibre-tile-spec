use crate::MltError::{
    EncodingInstructionCountMismatch, NotImplemented, UnsupportedPropertyEncoderCombination,
};
use crate::MltResult;
use crate::v01::{
    DictionaryType, EncodedName, EncodedPresence, EncodedProperty, EncodedScalar, EncodedScalarFam,
    EncodedStream, EncodedStrings, LengthType, PresenceStream, PropertyEncoder, Scalar,
    ScalarEncoder, ScalarValueEncoder, StagedProperty, StagedScalarFam, StrEncoder,
    encode_shared_dict_prop,
};

pub fn encode_properties(
    value: &[StagedProperty],
    encoders: Vec<PropertyEncoder>,
) -> MltResult<Vec<EncodedProperty>> {
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
    pub(crate) fn encode(value: &StagedProperty, encoder: ScalarEncoder) -> MltResult<Self> {
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
            (StagedProperty::Scalar(s), enc) => Ok(Self::Scalar(match (s, enc) {
                (Scalar::<StagedScalarFam>::Bool(v), ScalarValueEncoder::Bool) => {
                    Scalar::<EncodedScalarFam>::Bool(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_bools(&unapply_presence(&v.values))?,
                    ))
                }
                (Scalar::<StagedScalarFam>::I8(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::I8(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_i8s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::U8(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::U8(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_u8s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::I32(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::I32(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_i32s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::U32(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::U32(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_u32s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::I64(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::I64(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_i64s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::U64(v), ScalarValueEncoder::Int(enc)) => {
                    Scalar::<EncodedScalarFam>::U64(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_u64s(&unapply_presence(&v.values), enc)?,
                    ))
                }
                (Scalar::<StagedScalarFam>::F32(v), ScalarValueEncoder::Float) => {
                    Scalar::<EncodedScalarFam>::F32(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_f32(&unapply_presence(&v.values))?,
                    ))
                }
                (Scalar::<StagedScalarFam>::F64(v), ScalarValueEncoder::Float) => {
                    Scalar::<EncodedScalarFam>::F64(mk_scalar(
                        &v.name,
                        presence,
                        EncodedStream::encode_f64(&unapply_presence(&v.values))?,
                    ))
                }
                (_, e) => Err(UnsupportedPropertyEncoderCombination("scalar", e.into()))?,
            })),
            (StagedProperty::Str(v), ScalarValueEncoder::String(enc)) => {
                let dense_values = v.dense_values();
                Ok(Self::Str(EncodedStrings {
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
                }))
            }
            (StagedProperty::SharedDict(..), _) => Err(NotImplemented(
                "SharedDict cannot be encoded via ScalarEncoder",
            ))?,
            (v, e) => Err(UnsupportedPropertyEncoderCombination(v.into(), e.into()))?,
        }
    }
}

fn unapply_presence<T: Clone>(v: &[Option<T>]) -> Vec<T> {
    v.iter().filter_map(|x| x.as_ref()).cloned().collect()
}

impl StagedProperty {
    fn as_presence_stream(&self) -> MltResult<Vec<bool>> {
        Ok(match self {
            Self::Scalar(s) => s.presence_bools(),
            Self::Str(v) => v.presence_bools(),
            Self::SharedDict(..) => Err(NotImplemented("presence stream for shared dict"))?,
        })
    }

    /// Returns the column name regardless of variant.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Scalar(s) => s.name(),
            Self::Str(v) => &v.name,
            Self::SharedDict(v) => &v.prefix,
        }
    }
}
