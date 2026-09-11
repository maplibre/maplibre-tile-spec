//! Vertex-scoped columns, which only tag `0x02` (v2) layers carry.

use std::borrow::Cow;
use std::ops::Range;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::decoder::{
    GeometryValues, ParsedStrings, RawFloats, RawPresence, RawScalar, RawStrings,
};
use crate::tile::{MValue, PropKind};
use crate::{Decode, DecodeState, Decoder, Lazy, MltError, MltResult};

/// An m-value column, parameterized by decode state, mirroring [`Property`](crate::decoder::Property).
pub type MValueColumn<'a, S = Lazy> =
    <S as DecodeState>::LazyOrParsed<RawMValue<'a>, ParsedMValue<'a>>;

/// A raw m-value column as read directly from the tile.
///
/// It reads the same streams a property column of the same data type reads, so it
/// holds the same containers. Neither an id nor a shared dictionary is one of them.
#[derive(Debug, Clone, PartialEq)]
pub enum RawMValue<'a> {
    Bool(RawScalar<'a>),
    I8(RawScalar<'a>),
    U8(RawScalar<'a>),
    I32(RawScalar<'a>),
    U32(RawScalar<'a>),
    I64(RawScalar<'a>),
    U64(RawScalar<'a>),
    F32(RawFloats<'a>),
    F64(RawFloats<'a>),
    Str(RawStrings<'a>),
}

/// A decoded m-value column: which features carry values, and the values they carry.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMValue<'a> {
    pub(crate) name: &'a str,
    /// One bit per feature, or [`None`] when every feature carries values.
    /// A feature whose bit is clear carries none at all, rather than one null per vertex.
    pub(crate) presence: Option<Cow<'a, BitSlice<u8, Lsb0>>>,
    pub(crate) values: MValues<'a>,
}

/// The values of one m-value column, one per vertex, flat across the features that carry them.
#[derive(Debug, Clone, PartialEq)]
pub enum MValues<'a> {
    Bool(Vec<bool>),
    I8(Vec<i8>),
    U8(Vec<u8>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Str(ParsedStrings<'a>),
}

impl MValues<'_> {
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Bool(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::U8(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::Str(v) => v.feature_count(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn kind(&self) -> PropKind {
        match self {
            Self::Bool(_) => PropKind::Bool,
            Self::I8(_) => PropKind::I8,
            Self::U8(_) => PropKind::U8,
            Self::I32(_) => PropKind::I32,
            Self::U32(_) => PropKind::U32,
            Self::I64(_) => PropKind::I64,
            Self::U64(_) => PropKind::U64,
            Self::F32(_) => PropKind::F32,
            Self::F64(_) => PropKind::F64,
            Self::Str(_) => PropKind::Str,
        }
    }
}

impl MValues<'_> {
    /// The values one feature owns, as the row model holds them, or its empty
    /// value when the feature owns none.
    ///
    /// `span` comes from [`ParsedMValue::spans`], so it is already within range;
    /// a span that is not is reported rather than panicked on.
    pub(crate) fn row(&self, name: &str, span: Option<Range<usize>>) -> MltResult<MValue> {
        let Some(span) = span else {
            return Ok(MValue::null(self.kind()));
        };
        let out_of_range = || MltError::MValueCountMismatch {
            name: name.to_string(),
            expected: span.end,
            actual: self.len(),
        };
        /// Copy one feature's run out of a column's flat values.
        macro_rules! run {
            ($values:expr) => {
                $values.get(span.clone()).ok_or_else(out_of_range)?.to_vec()
            };
        }
        Ok(match self {
            Self::Bool(v) => MValue::Bool(Some(run!(v))),
            Self::I8(v) => MValue::I8(Some(run!(v))),
            Self::U8(v) => MValue::U8(Some(run!(v))),
            Self::I32(v) => MValue::I32(Some(run!(v))),
            Self::U32(v) => MValue::U32(Some(run!(v))),
            Self::I64(v) => MValue::I64(Some(run!(v))),
            Self::U64(v) => MValue::U64(Some(run!(v))),
            Self::F32(v) => MValue::F32(Some(run!(v))),
            Self::F64(v) => MValue::F64(Some(run!(v))),
            Self::Str(v) => MValue::Str(Some(
                span.clone()
                    .map(|i| {
                        let i = u32::try_from(i)?;
                        Ok(v.get(i).ok_or_else(out_of_range)?.to_string())
                    })
                    .collect::<MltResult<Vec<String>>>()?,
            )),
        })
    }

    /// The heap bytes a row of this column takes, for the decoder's budget.
    pub(crate) fn row_bytes(&self, span: &Range<usize>) -> usize {
        match self {
            Self::Str(v) => span
                .clone()
                .filter_map(|i| u32::try_from(i).ok())
                .filter_map(|i| v.get(i))
                .map(str::len)
                .sum(),
            Self::Bool(_) | Self::I8(_) | Self::U8(_) => span.len(),
            Self::I32(_) | Self::U32(_) | Self::F32(_) => span.len() * 4,
            Self::I64(_) | Self::U64(_) | Self::F64(_) => span.len() * 8,
        }
    }
}

impl<'a> ParsedMValue<'a> {
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[must_use]
    pub fn values(&self) -> &MValues<'a> {
        &self.values
    }

    /// Whether feature `index` carries values.
    #[must_use]
    pub fn is_present(&self, index: usize) -> bool {
        self.presence
            .as_ref()
            .is_none_or(|bits| bits.get(index).as_deref().copied().unwrap_or(false))
    }

    /// Where each feature's values sit in this column, [`None`] for a feature carrying none.
    ///
    /// This is also the only check that the column and the geometry agree: the
    /// spans are cut from the geometry's per-feature vertex counts, so a column
    /// that does not end exactly where the last one does is rejected here.
    pub fn spans(&self, geometry: &GeometryValues) -> MltResult<Vec<Option<Range<usize>>>> {
        let mut spans = Vec::with_capacity(geometry.feature_count());
        let mut start = 0;
        for index in 0..geometry.feature_count() {
            if self.is_present(index) {
                let end = start + geometry.vertex_count(index)?;
                spans.push(Some(start..end));
                start = end;
            } else {
                spans.push(None);
            }
        }
        if start != self.values.len() {
            return Err(MltError::MValueCountMismatch {
                name: self.name.to_string(),
                expected: start,
                actual: self.values.len(),
            });
        }
        Ok(spans)
    }
}

impl<'a> Decode<ParsedMValue<'a>> for RawMValue<'a> {
    /// Decode into a [`ParsedMValue`], charging `dec` for every heap allocation.
    ///
    /// The presence bits are the feature's, so they never expand into the values
    /// the way a property column's nulls do.
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedMValue<'a>> {
        use MValues as M;

        Ok(match self {
            Self::Bool(v) => parsed(v.name, v.presence, M::Bool(v.data.decode_bools(dec)?), dec)?,
            Self::I8(v) => {
                let values = v.data.decode_narrow::<i8, i32>(dec)?;
                parsed(v.name, v.presence, M::I8(values), dec)?
            }
            Self::U8(v) => {
                let values = v.data.decode_narrow::<u8, u32>(dec)?;
                parsed(v.name, v.presence, M::U8(values), dec)?
            }
            Self::I32(v) => {
                let values = v.data.decode_ints::<i32>(dec)?;
                parsed(v.name, v.presence, M::I32(values), dec)?
            }
            Self::U32(v) => {
                let values = v.data.decode_ints::<u32>(dec)?;
                parsed(v.name, v.presence, M::U32(values), dec)?
            }
            Self::I64(v) => {
                let values = v.data.decode_ints::<i64>(dec)?;
                parsed(v.name, v.presence, M::I64(values), dec)?
            }
            Self::U64(v) => {
                let values = v.data.decode_ints::<u64>(dec)?;
                parsed(v.name, v.presence, M::U64(values), dec)?
            }
            Self::F32(v) => {
                let (name, presence) = (v.name, v.presence.clone());
                let values = v.decode_values::<f32>(dec)?;
                parsed(name, presence, M::F32(values), dec)?
            }
            Self::F64(v) => {
                let (name, presence) = (v.name, v.presence.clone());
                let values = v.decode_values::<f64>(dec)?;
                parsed(name, presence, M::F64(values), dec)?
            }
            Self::Str(mut v) => {
                // A string column expands its own nulls into its lengths, which
                // these presence bits are not: they are the feature's, not the value's.
                let presence = std::mem::take(&mut v.presence);
                let name = v.name;
                parsed(name, presence, M::Str(v.decode(dec)?), dec)?
            }
        })
    }
}

/// Assemble a column from its name, the presence bits over features, and its values.
fn parsed<'a>(
    name: &'a str,
    presence: RawPresence<'a>,
    values: MValues<'a>,
    dec: &mut Decoder,
) -> MltResult<ParsedMValue<'a>> {
    Ok(ParsedMValue {
        name,
        presence: presence.decode_bits(dec)?,
        values,
    })
}

impl RawMValue<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bool(v)
            | Self::I8(v)
            | Self::U8(v)
            | Self::I32(v)
            | Self::U32(v)
            | Self::I64(v)
            | Self::U64(v) => v.name,
            Self::F32(v) | Self::F64(v) => v.name,
            Self::Str(v) => v.name,
        }
    }
}
