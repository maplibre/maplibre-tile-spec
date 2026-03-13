//! Conversions between the decoding pipeline (`ParsedProperty<'_>`) and
//! the encoding pipeline (`StagedProperty`), plus helpers on `StagedProperty`.

use std::fmt;

use crate::MltError;
use crate::MltError::NotImplemented;
use crate::v01::{
    ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedStrings, PropertyKind, StagedProperty,
    StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};

// ── From<ParsedProperty<'_>> ─────────────────────────────────────────────────

impl<'a> From<ParsedProperty<'a>> for StagedProperty {
    fn from(p: ParsedProperty<'a>) -> Self {
        match p {
            ParsedProperty::Bool(s) => StagedProperty::Bool(StagedScalar::from(s)),
            ParsedProperty::I8(s) => StagedProperty::I8(StagedScalar::from(s)),
            ParsedProperty::U8(s) => StagedProperty::U8(StagedScalar::from(s)),
            ParsedProperty::I32(s) => StagedProperty::I32(StagedScalar::from(s)),
            ParsedProperty::U32(s) => StagedProperty::U32(StagedScalar::from(s)),
            ParsedProperty::I64(s) => StagedProperty::I64(StagedScalar::from(s)),
            ParsedProperty::U64(s) => StagedProperty::U64(StagedScalar::from(s)),
            ParsedProperty::F32(s) => StagedProperty::F32(StagedScalar::from(s)),
            ParsedProperty::F64(s) => StagedProperty::F64(StagedScalar::from(s)),
            ParsedProperty::Str(s) => StagedProperty::Str(StagedStrings::from(s)),
            ParsedProperty::SharedDict(sd) => {
                StagedProperty::SharedDict(StagedSharedDict::from(sd))
            }
        }
    }
}

impl<'a, T: Copy + PartialEq> From<ParsedScalar<'a, T>> for StagedScalar<T> {
    fn from(s: ParsedScalar<'a, T>) -> Self {
        StagedScalar {
            name: s.name.into_owned(),
            values: s.values,
        }
    }
}

impl<'a> From<ParsedStrings<'a>> for StagedStrings {
    fn from(s: ParsedStrings<'a>) -> Self {
        StagedStrings {
            name: s.name.into_owned(),
            lengths: s.lengths,
            data: s.data.into_owned(),
        }
    }
}

impl<'a> From<ParsedSharedDict<'a>> for StagedSharedDict {
    fn from(sd: ParsedSharedDict<'a>) -> Self {
        StagedSharedDict {
            prefix: sd.prefix.into_owned(),
            data: sd.data.into_owned(),
            items: sd
                .items
                .into_iter()
                .map(|item| StagedSharedDictItem {
                    suffix: item.suffix.into_owned(),
                    ranges: item.ranges,
                })
                .collect(),
        }
    }
}

// ── StagedProperty → ParsedProperty<'_> ─────────────────────────────────────

impl StagedProperty {
    /// Borrow this staged property as a `ParsedProperty` that borrows from `self`.
    ///
    /// Used internally to bridge into the existing encoder infrastructure that
    /// expects `&[ParsedProperty<'_>]`.
    #[must_use]
    pub fn as_parsed(&self) -> ParsedProperty<'_> {
        use std::borrow::Cow;
        match self {
            Self::Bool(s) => ParsedProperty::Bool(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::I8(s) => ParsedProperty::I8(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::U8(s) => ParsedProperty::U8(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::I32(s) => ParsedProperty::I32(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::U32(s) => ParsedProperty::U32(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::I64(s) => ParsedProperty::I64(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::U64(s) => ParsedProperty::U64(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::F32(s) => ParsedProperty::F32(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::F64(s) => ParsedProperty::F64(ParsedScalar {
                name: Cow::Borrowed(&s.name),
                values: s.values.clone(),
            }),
            Self::Str(s) => {
                use crate::v01::ParsedStrings;
                ParsedProperty::Str(ParsedStrings {
                    name: Cow::Borrowed(&s.name),
                    lengths: s.lengths.clone(),
                    data: Cow::Borrowed(&s.data),
                })
            }
            Self::SharedDict(sd) => {
                use crate::v01::{ParsedSharedDict, ParsedSharedDictItem};
                ParsedProperty::SharedDict(ParsedSharedDict {
                    prefix: Cow::Borrowed(&sd.prefix),
                    data: Cow::Borrowed(&sd.data),
                    items: sd
                        .items
                        .iter()
                        .map(|item| ParsedSharedDictItem {
                            suffix: Cow::Borrowed(&item.suffix),
                            ranges: item.ranges.clone(),
                        })
                        .collect(),
                })
            }
        }
    }
}

// ── helpers on StagedProperty ────────────────────────────────────────────────

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

    /// Create a string column from optional owned string values.
    #[must_use]
    pub fn str(name: impl Into<String>, values: Vec<Option<String>>) -> Self {
        let mut lengths = Vec::with_capacity(values.len());
        let mut data = String::new();
        for v in values {
            if let Some(s) = v {
                lengths.push(i32::try_from(data.len() + s.len()).unwrap_or(i32::MAX));
                data.push_str(&s);
            } else {
                let cur = i32::try_from(data.len()).unwrap_or(i32::MAX);
                lengths.push(!cur);
            }
        }
        Self::Str(StagedStrings {
            name: name.into(),
            lengths,
            data,
        })
    }

    /// Column name (borrowed from the owned `String`).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(s) => &s.name,
            Self::I8(s) => &s.name,
            Self::U8(s) => &s.name,
            Self::I32(s) => &s.name,
            Self::U32(s) => &s.name,
            Self::I64(s) => &s.name,
            Self::U64(s) => &s.name,
            Self::F32(s) => &s.name,
            Self::F64(s) => &s.name,
            Self::Str(s) => &s.name,
            Self::SharedDict(sd) => &sd.prefix,
        }
    }

    /// Build the per-feature presence (non-null) boolean stream.
    pub(crate) fn as_presence_stream(&self) -> Result<Vec<bool>, MltError> {
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
            Self::Str(v) => v.lengths.iter().map(|&l| l >= 0).collect(),
            Self::SharedDict(..) => Err(NotImplemented("presence stream for shared dict"))?,
        })
    }

    /// Broad property kind (used to select encoder class).
    #[must_use]
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::Bool(_) => PropertyKind::Bool,
            Self::I8(_)
            | Self::U8(_)
            | Self::I32(_)
            | Self::U32(_)
            | Self::I64(_)
            | Self::U64(_) => PropertyKind::Integer,
            Self::F32(_) | Self::F64(_) => PropertyKind::Float,
            Self::Str(_) => PropertyKind::String,
            Self::SharedDict(_) => PropertyKind::SharedDict,
        }
    }

    /// Static string name of the variant (for error messages).
    #[must_use]
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::I8(_) => "i8",
            Self::U8(_) => "u8",
            Self::I32(_) => "i32",
            Self::U32(_) => "u32",
            Self::I64(_) => "i64",
            Self::U64(_) => "u64",
            Self::F32(_) => "f32",
            Self::F64(_) => "f64",
            Self::Str(_) => "str",
            Self::SharedDict(_) => "shared_dict",
        }
    }
}

// ── Debug impls ──────────────────────────────────────────────────────────────

impl<T: Copy + PartialEq + fmt::Debug> fmt::Debug for StagedScalar<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StagedScalar")
            .field("name", &self.name)
            .field("values", &self.values)
            .finish()
    }
}

impl fmt::Debug for StagedProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(s) => f.debug_tuple("Bool").field(s).finish(),
            Self::I8(s) => f.debug_tuple("I8").field(s).finish(),
            Self::U8(s) => f.debug_tuple("U8").field(s).finish(),
            Self::I32(s) => f.debug_tuple("I32").field(s).finish(),
            Self::U32(s) => f.debug_tuple("U32").field(s).finish(),
            Self::I64(s) => f.debug_tuple("I64").field(s).finish(),
            Self::U64(s) => f.debug_tuple("U64").field(s).finish(),
            Self::F32(s) => f.debug_tuple("F32").field(s).finish(),
            Self::F64(s) => f.debug_tuple("F64").field(s).finish(),
            Self::Str(s) => f.debug_tuple("Str").field(s).finish(),
            Self::SharedDict(sd) => f.debug_tuple("SharedDict").field(sd).finish(),
        }
    }
}
