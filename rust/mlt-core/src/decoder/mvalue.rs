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

/// An m-value column, parameterized by decode state.
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

macro_rules! impl_m_values {
    (
        scalar { $($sv:ident),* $(,)? }
        string { $($gv:ident),* $(,)? }
    ) => {
        impl MValues<'_> {
            /// How many values the column holds, over every feature that has any.
            #[must_use]
            pub fn len(&self) -> usize {
                match self {
                    $(Self::$sv(v) => v.len(),)*
                    // A string column counts its entries, not its bytes.
                    $(Self::$gv(v) => v.feature_count(),)*
                }
            }

            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            #[must_use]
            pub fn kind(&self) -> PropKind {
                match self {
                    $(Self::$sv(_) => PropKind::$sv,)*
                    $(Self::$gv(_) => PropKind::$gv,)*
                }
            }
        }
    };
}

with_kinds!(impl_m_values);

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
        let out_of_range = || MltError::MValueRunOutOfRange {
            name: name.to_string(),
            start: span.start,
            count: span.len(),
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
        is_present(self.presence.as_deref(), index)
    }

    /// Where each feature's values sit in this column, walked one feature at a time.
    ///
    /// The walk holds nothing per feature, so every column of a layer can be walked
    /// alongside its features without a span per feature per column.
    /// It does not check that the column ends where the geometry does, which
    /// [`Self::check_length`] is for.
    #[must_use]
    pub fn spans<'g>(&'g self, geometry: &'g GeometryValues) -> MValueSpans<'g> {
        MValueSpans {
            presence: self.presence.as_deref(),
            geometry,
            feature: 0,
            start: 0,
        }
    }

    /// Check that this column and the geometry it runs over agree.
    ///
    /// The values are cut into per-feature runs by the geometry's vertex counts, so a
    /// column that does not end exactly where the last feature's run does is rejected.
    pub fn check_length(&self, geometry: &GeometryValues) -> MltResult<()> {
        let mut expected = 0;
        for index in 0..geometry.feature_count() {
            if self.is_present(index) {
                expected += geometry.vertex_count(index)?;
            }
        }
        if expected != self.values.len() {
            return Err(MltError::MValueColumnLengthMismatch {
                name: self.name.to_string(),
                expected,
                actual: self.values.len(),
            });
        }
        Ok(())
    }
}

/// Whether the feature at `index` carries values, by a column's presence mask.
fn is_present(presence: Option<&BitSlice<u8, Lsb0>>, index: usize) -> bool {
    presence.is_none_or(|bits| bits.get(index).as_deref().copied().unwrap_or(false))
}

/// Each feature's run of values in one m-value column, in feature order.
///
/// A feature that carries no values yields [`None`] rather than an empty run.
#[derive(Debug, Clone)]
pub struct MValueSpans<'a> {
    presence: Option<&'a BitSlice<u8, Lsb0>>,
    geometry: &'a GeometryValues,
    feature: usize,
    start: usize,
}

impl Iterator for MValueSpans<'_> {
    type Item = MltResult<Option<Range<usize>>>;

    fn next(&mut self) -> Option<Self::Item> {
        let feature = self.feature;
        if feature >= self.geometry.feature_count() {
            return None;
        }
        self.feature += 1;
        if !is_present(self.presence, feature) {
            return Some(Ok(None));
        }
        let count = match self.geometry.vertex_count(feature) {
            Ok(count) => count,
            Err(err) => return Some(Err(err)),
        };
        let start = self.start;
        self.start += count;
        Some(Ok(Some(start..self.start)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.geometry.feature_count() - self.feature;
        (left, Some(left))
    }
}

impl ExactSizeIterator for MValueSpans<'_> {}

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

#[cfg(test)]
mod tests {
    use geo_types::{Coord, Geometry, LineString};

    use super::*;

    /// A line layer of one feature per entry, each with that many vertices.
    fn geometry(vertices: &[i32]) -> GeometryValues {
        let mut geometry = GeometryValues::default();
        for &count in vertices {
            let coords = (0..count).map(|i| Coord { x: i, y: i }).collect();
            geometry.push_geom(&Geometry::LineString(LineString::new(coords)));
        }
        geometry
    }

    fn column(presence: Option<&BitSlice<u8, Lsb0>>, values: Vec<i32>) -> ParsedMValue<'_> {
        ParsedMValue {
            name: "m",
            presence: presence.map(Cow::Borrowed),
            values: MValues::I32(values),
        }
    }

    #[test]
    fn every_feature_owns_the_run_its_vertex_count_asks_for() {
        let geometry = geometry(&[2, 3]);
        let column = column(None, vec![1, 2, 3, 4, 5]);
        let spans: Vec<_> = column
            .spans(&geometry)
            .collect::<MltResult<_>>()
            .expect("spans");
        assert_eq!(spans, [Some(0..2), Some(2..5)]);
        column.check_length(&geometry).expect("column length");
    }

    #[test]
    fn a_cleared_presence_bit_leaves_a_feature_out_of_the_runs() {
        let mask = [0b0000_0101u8];
        let geometry = geometry(&[2, 3, 2]);
        let column = column(Some(BitSlice::from_slice(&mask)), vec![1, 2, 3, 4]);
        let spans: Vec<_> = column
            .spans(&geometry)
            .collect::<MltResult<_>>()
            .expect("spans");
        assert_eq!(spans, [Some(0..2), None, Some(2..4)]);
        column.check_length(&geometry).expect("column length");
    }

    #[test]
    fn a_column_the_geometry_does_not_account_for_is_rejected() {
        let geometry = geometry(&[2, 2]);
        let err = column(None, vec![1, 2, 3, 4, 5])
            .check_length(&geometry)
            .unwrap_err();
        assert!(
            matches!(
                err,
                MltError::MValueColumnLengthMismatch {
                    ref name,
                    expected: 4,
                    actual: 5,
                } if name == "m"
            ),
            "{err:?}"
        );
    }

    #[test]
    fn a_run_past_the_end_of_a_column_reports_the_run_it_wanted() {
        let values = MValues::I32(vec![1, 2]);
        let err = values.row("m", Some(0..3)).unwrap_err();
        assert!(
            matches!(
                err,
                MltError::MValueRunOutOfRange {
                    ref name,
                    start: 0,
                    count: 3,
                    actual: 2,
                } if name == "m"
            ),
            "{err:?}"
        );
    }
}
