//! Zero-copy per-feature view into a fully-decoded [`Layer01<Parsed>`].
//!
//! [`ParsedLayer01::iter_features`] yields one [`FeatureRef`] per feature.
//! [`FeatureRef::iter_properties`] exposes per-feature property values as flat
//! [`ColumnRef`] items; `SharedDict` columns are transparently expanded and null
//! values are skipped.

use std::fmt;

use geo_types::Geometry;

use crate::decoder::{Layer01, ParsedLayer01, ParsedProperty, ParsedScalar, RawProperty};
use crate::{Lazy, LazyParsed, MltResult, Parsed};

impl<'a> Layer01<'a, Lazy> {
    /// Iterate over the property column names of this layer, in order.
    ///
    /// Regular columns yield one [`PropName`]; `SharedDict` columns yield one name per
    /// sub-item.  Names are available even before any column data has been decoded.
    ///
    /// Pair with [`FeatureRef::iter_all_properties`] to associate per-feature
    /// values with their column names.
    pub fn iterate_prop_names(&self) -> impl Iterator<Item = PropName<'a>> + '_ {
        Layer01PropNamesIter::lazy(&self.properties)
    }
}

impl<'a> ParsedLayer01<'a> {
    /// Iterate over all features in this fully-decoded layer.
    ///
    /// Yields one [`FeatureRef`] per feature. Construction is infallible; individual
    /// `next()` calls return `MltResult<FeatureRef>` because geometry decoding can fail.
    /// The iterator implements [`ExactSizeIterator`], so `.len()` is available.
    #[must_use]
    pub fn iter_features(&self) -> impl ExactSizeIterator<Item = MltResult<FeatureRef<'_>>> + '_ {
        Layer01FeatureIter::new(self)
    }

    /// Iterate over the property column names of this layer, in order.
    /// See [`Layer01::iterate_prop_names`] for details.
    pub fn iterate_prop_names(&self) -> impl Iterator<Item = PropName<'a>> + '_ {
        Layer01PropNamesIter::parsed(&self.properties)
    }
}

/// Source of property columns, for either decode state.
enum PropSource<'a, 'p> {
    Parsed(&'a [ParsedProperty<'p>]),
    Lazy(&'a [LazyParsed<RawProperty<'p>, ParsedProperty<'p>>]),
}

/// Iterates the column names of a [`Layer01`]'s properties, for any decode state.
///
/// Returned by [`Layer01::iterate_prop_names`] and [`ParsedLayer01::iterate_prop_names`].
/// - Regular columns yield one [`PropName`] (the column name with an empty suffix).
/// - `SharedDict` columns yield one [`PropName`] per sub-item (`(prefix, suffix)`).
pub(crate) struct Layer01PropNamesIter<'a, 'p> {
    source: PropSource<'a, 'p>,
    col_idx: usize,
    dict_idx: usize,
}

impl<'a, 'p> Layer01PropNamesIter<'a, 'p> {
    fn parsed(props: &'a [ParsedProperty<'p>]) -> Self {
        Self {
            source: PropSource::Parsed(props),
            col_idx: 0,
            dict_idx: 0,
        }
    }

    fn lazy(props: &'a [LazyParsed<RawProperty<'p>, ParsedProperty<'p>>]) -> Self {
        Self {
            source: PropSource::Lazy(props),
            col_idx: 0,
            dict_idx: 0,
        }
    }
}

impl<'a> Iterator for Layer01PropNamesIter<'_, 'a> {
    type Item = PropName<'a>;

    fn next(&mut self) -> Option<PropName<'a>> {
        loop {
            let col_idx = self.col_idx;
            self.col_idx += 1;
            let name = match &self.source {
                PropSource::Parsed(props) => {
                    parsed_col_name(props.get(col_idx)?, &mut self.dict_idx)
                }
                PropSource::Lazy(props) => match props.get(col_idx)? {
                    LazyParsed::Raw(r) => raw_col_name(r, &mut self.dict_idx),
                    LazyParsed::Parsed(p) => parsed_col_name(p, &mut self.dict_idx),
                    LazyParsed::ParsingFailed => None,
                },
            };
            if self.dict_idx != 0 {
                self.col_idx -= 1; // SharedDict not yet exhausted: revisit this column
            }
            if let Some(n) = name {
                return Some(n);
            }
        }
    }
}

/// Yield the next [`PropName`] from a [`ParsedProperty`] column.
#[inline]
fn parsed_col_name<'p>(prop: &ParsedProperty<'p>, dict_idx: &mut usize) -> Option<PropName<'p>> {
    use ParsedProperty as P;
    match prop {
        P::Bool(s) => Some(PropName(s.name, "")),
        P::I8(s) => Some(PropName(s.name, "")),
        P::U8(s) => Some(PropName(s.name, "")),
        P::I32(s) => Some(PropName(s.name, "")),
        P::U32(s) => Some(PropName(s.name, "")),
        P::I64(s) => Some(PropName(s.name, "")),
        P::U64(s) => Some(PropName(s.name, "")),
        P::F32(s) => Some(PropName(s.name, "")),
        P::F64(s) => Some(PropName(s.name, "")),
        P::Str(s) => Some(PropName(s.name, "")),
        P::SharedDict(sd) => {
            if *dict_idx < sd.items.len() {
                let idx = *dict_idx;
                *dict_idx += 1;
                Some(PropName(sd.prefix, sd.items[idx].suffix))
            } else {
                *dict_idx = 0;
                None
            }
        }
    }
}

/// Yield the next [`PropName`] from a [`RawProperty`] column.  See [`parsed_col_name`].
#[inline]
fn raw_col_name<'p>(prop: &RawProperty<'p>, dict_idx: &mut usize) -> Option<PropName<'p>> {
    use RawProperty as P;
    match prop {
        P::Bool(s)
        | P::I8(s)
        | P::U8(s)
        | P::I32(s)
        | P::U32(s)
        | P::I64(s)
        | P::U64(s)
        | P::F32(s)
        | P::F64(s) => Some(PropName(s.name, "")),
        P::Str(s) => Some(PropName(s.name, "")),
        P::SharedDict(sd) => {
            if *dict_idx < sd.children.len() {
                let idx = *dict_idx;
                *dict_idx += 1;
                Some(PropName(sd.name, sd.children[idx].name))
            } else {
                *dict_idx = 0;
                None
            }
        }
    }
}

/// A zero-allocation two-part property name yielded by [`FeatureRef::iter_properties`].
///
/// The two parts concatenate on [`Display`](fmt::Display) as `"{}{}"`:
/// - For regular columns: `(column_name, "")` — zero allocation, second part always empty.
/// - For `SharedDict` sub-items: `(prefix, suffix)` — both borrow directly from layer data.
///
/// Structural [`PartialEq`] compares both parts independently.  Use [`PartialEq<str>`] or
/// [`PartialEq<&str>`] (also implemented) to compare against a plain `&str` as if the two
/// parts were concatenated.
#[derive(Debug, Clone, Copy)] // WARN: do not auto-derive PartialEq,Eq,Hash as it won't be correct
pub struct PropName<'a>(&'a str, &'a str);

impl fmt::Display for PropName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)?;
        f.write_str(self.1)
    }
}

impl PartialEq<PropName<'_>> for PropName<'_> {
    fn eq(&self, other: &PropName<'_>) -> bool {
        // Compare the concatenated strings byte-by-byte without allocating.
        let (a0, a1) = (self.0.as_bytes(), self.1.as_bytes());
        let a = a0.iter().chain(a1);
        let (b0, b1) = (other.0.as_bytes(), other.1.as_bytes());
        let b = b0.iter().chain(b1);
        let combined_len_eq = a0.len() + a1.len() == b0.len() + b1.len();
        combined_len_eq && a.eq(b)
    }
}

impl PartialEq<str> for PropName<'_> {
    /// Returns `true` if `other == self.0 + self.1`.
    fn eq(&self, other: &str) -> bool {
        other.strip_prefix(self.0) == Some(self.1)
    }
}

impl PartialEq<PropName<'_>> for str {
    fn eq(&self, other: &PropName<'_>) -> bool {
        other == self
    }
}

impl PartialEq<&str> for PropName<'_> {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<PropName<'_>> for &str {
    fn eq(&self, other: &PropName<'_>) -> bool {
        other == *self
    }
}

/// A borrowed, non-null per-feature property value.
///
/// Nullability is lifted to [`ColumnRef`]: only non-null values appear in
/// [`FeatureRef::iter_properties`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropValueRef<'a> {
    Bool(bool),
    I8(i8),
    U8(u8),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Str(&'a str),
}

macro_rules! impl_from_for_prop_value_ref {
    ($($ty:ty => $variant:ident),+ $(,)?) => {
        $(impl From<$ty> for PropValueRef<'_> {
            fn from(v: $ty) -> Self { Self::$variant(v) }
        })+
    };
}
impl_from_for_prop_value_ref!(
    bool => Bool, i8 => I8, u8 => U8,
    i32 => I32, u32 => U32,
    i64 => I64, u64 => U64,
    f32 => F32, f64 => F64,
);

impl<'a, T> ParsedScalar<'a, T>
where
    T: Copy + PartialEq,
    PropValueRef<'a>: From<T>,
{
    #[inline]
    fn value_at(&self, feat_idx: usize) -> Option<PropValueRef<'a>> {
        self.values[feat_idx].map(|v| PropValueRef::from(v))
    }

    #[inline]
    fn column_at(&self, idx: usize) -> Option<ColumnRef<'a>> {
        self.value_at(idx).map(|v| ColumnRef::new(self.name, v))
    }
}

/// A single non-null property value for one feature, yielded by [`FeatureRef::iter_properties`].
///
/// `name` is a [`PropName`] that displays as `"{prefix}{suffix}"`.
/// All borrows are zero-copy from the layer data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRef<'a> {
    pub name: PropName<'a>,
    pub value: PropValueRef<'a>,
}

impl<'a> ColumnRef<'a> {
    #[inline]
    pub(crate) fn new(name: &'a str, value: PropValueRef<'a>) -> Self {
        Self {
            name: PropName(name, ""),
            value,
        }
    }
    #[inline]
    pub(crate) fn new_sub(prefix: &'a str, suffix: &'a str, value: PropValueRef<'a>) -> Self {
        Self {
            name: PropName(prefix, suffix),
            value,
        }
    }
}

/// A single map feature returned by [`ParsedLayer01::iter_features`].
///
/// The internal columnar layout is private; use [`iter_properties`](Self::iter_properties)
/// and [`get_property`](Self::get_property) to access property values.
#[derive(Debug)]
pub struct FeatureRef<'a> {
    /// Optional feature ID.
    pub id: Option<u64>,
    /// Geometry in [`Geometry::<i32> `] form (owned, computed on demand by the iterator).
    pub geometry: Geometry<i32>,
    columns: &'a [ParsedProperty<'a>],
    index: usize,
}

impl<'a> FeatureRef<'a> {
    /// Iterate over every property slot for this feature, **values only**, in column order.
    ///
    /// Yields `Option<PropValueRef>`:
    /// - `Some(value)` — the slot contains a non-null value.
    /// - `None` — the slot is null / absent.
    ///
    /// The outer `Option` from [`Iterator::next`] signals end-of-iteration as usual.
    ///
    /// Use [`Layer01::iterate_prop_names`] to pair values with their column names.
    pub fn iter_all_properties(&self) -> impl Iterator<Item = Option<PropValueRef<'a>>> {
        FeatValuesIter {
            columns: self.columns,
            feat_idx: self.index,
            col_idx: 0,
            dict_idx: 0,
        }
    }

    /// Iterate over all non-null properties for this feature.
    ///
    /// `SharedDict` columns are transparently expanded into one [`ColumnRef`] per sub-item.
    /// Null / absent values are skipped entirely. The iterator is infallible.
    pub fn iter_properties(&self) -> impl Iterator<Item = ColumnRef<'a>> {
        FeatPropertyIter {
            columns: self.columns,
            feat_idx: self.index,
            col_idx: 0,
            dict_idx: 0,
        }
    }

    /// Look up a property by name, returning its value if present and non-null.
    ///
    /// For `SharedDict` columns the expected name is `"{prefix}{suffix}"`, matching
    /// the key used by [`iter_properties`](Self::iter_properties).
    #[must_use]
    pub fn get_property(&self, name: &str) -> Option<PropValueRef<'a>> {
        // TODO: determine if this is a perf issue
        self.iter_properties()
            .find(|col| col.name == name)
            .map(|col| col.value)
    }
}

// ── Per-feature property iterators ───────────────────────────────────────────

/// Iterates every property slot of a feature as `Option<`[`PropValueRef`]`>`.
///
/// Returned by [`FeatureRef::iter_all_properties`].
///
/// The [`Iterator::Item`] is `Option<PropValueRef>`:
/// - `Some(v)` — non-null value.
/// - `None` — null / absent slot.
///
/// Pair with [`Layer01::iterate_prop_names`] to associate values with column names.
pub(crate) struct FeatValuesIter<'a> {
    columns: &'a [ParsedProperty<'a>],
    feat_idx: usize,
    col_idx: usize,
    dict_idx: usize,
}

impl<'a> Iterator for FeatValuesIter<'a> {
    type Item = Option<PropValueRef<'a>>;

    fn next(&mut self) -> Option<Option<PropValueRef<'a>>> {
        use ParsedProperty as PP;

        loop {
            let prop = self.columns.get(self.col_idx)?;
            self.col_idx += 1;
            let idx = self.feat_idx;
            return Some(match prop {
                PP::Bool(s) => s.value_at(idx),
                PP::I8(s) => s.value_at(idx),
                PP::U8(s) => s.value_at(idx),
                PP::I32(s) => s.value_at(idx),
                PP::U32(s) => s.value_at(idx),
                PP::I64(s) => s.value_at(idx),
                PP::U64(s) => s.value_at(idx),
                PP::F32(s) => s.value_at(idx),
                PP::F64(s) => s.value_at(idx),
                PP::Str(s) => s.value_at(idx),
                PP::SharedDict(s) => {
                    let result = s.value_at(idx, &mut self.dict_idx);
                    if self.dict_idx != 0 {
                        // Not exhausted (null sub-item or more items remain): stay on this column.
                        self.col_idx -= 1;
                        return Some(result.map(|(_, v)| v));
                    }
                    continue; // exhausted all sub-items
                }
            });
        }
    }
}

/// Iterates the non-null properties of a feature as [`ColumnRef`] (name + value).
///
/// Returned by [`FeatureRef::iter_properties`]. `SharedDict` columns are expanded
/// in-place without any heap allocation.
pub(crate) struct FeatPropertyIter<'a> {
    columns: &'a [ParsedProperty<'a>],
    feat_idx: usize,
    col_idx: usize,
    dict_idx: usize,
}

impl<'a> Iterator for FeatPropertyIter<'a> {
    type Item = ColumnRef<'a>;

    fn next(&mut self) -> Option<ColumnRef<'a>> {
        use ParsedProperty as PP;

        loop {
            let prop = self.columns.get(self.col_idx)?;
            self.col_idx += 1;
            let idx = self.feat_idx;
            if let Some(col) = match &prop {
                PP::Bool(s) => s.column_at(idx),
                PP::I8(s) => s.column_at(idx),
                PP::U8(s) => s.column_at(idx),
                PP::I32(s) => s.column_at(idx),
                PP::U32(s) => s.column_at(idx),
                PP::I64(s) => s.column_at(idx),
                PP::U64(s) => s.column_at(idx),
                PP::F32(s) => s.column_at(idx),
                PP::F64(s) => s.column_at(idx),
                PP::Str(s) => s.column_at(idx),
                PP::SharedDict(s) => {
                    let result = s.column_at(idx, &mut self.dict_idx);
                    if self.dict_idx != 0 {
                        // Not exhausted (null sub-item or more items remain): stay on this column.
                        self.col_idx -= 1;
                    }
                    if result.is_none() {
                        continue;
                    }
                    result
                }
            } {
                return Some(col);
            }
        }
    }
}

/// Iterator over the features of a fully-decoded [`Layer01<Parsed>`].
///
/// Returned by [`ParsedLayer01::iter_features`].
/// Geometry construction is the only fallible step; all property access is infallible.
pub(crate) struct Layer01FeatureIter<'layer, 'data> {
    layer: &'layer Layer01<'data, Parsed>,
    index: usize,
}

impl<'layer, 'data> Layer01FeatureIter<'layer, 'data> {
    fn new(layer: &'layer Layer01<'data, Parsed>) -> Self {
        Self { layer, index: 0 }
    }
}

impl<'layer> Iterator for Layer01FeatureIter<'layer, '_> {
    type Item = MltResult<FeatureRef<'layer>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.layer.feature_count() {
            return None;
        }
        let index = self.index;
        self.index += 1;

        Some(Ok(FeatureRef {
            id: self
                .layer
                .id
                .as_ref()
                .and_then(|ids| ids.0.get(index).copied().flatten()),
            geometry: match self.layer.geometry.to_geojson(index) {
                Ok(g) => g,
                Err(e) => return Some(Err(e)),
            },
            columns: &self.layer.properties,
            index,
        }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .layer
            .geometry
            .vector_types
            .len()
            .saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Layer01FeatureIter<'_, '_> {}

#[cfg(test)]
mod tests {
    use geo_types::Point;
    use serde_json::Value;

    use super::*;
    use crate::Layer;
    use crate::decoder::{GeometryValues, IdValues};
    use crate::encoder::model::{StagedLayer, StagedLayer01};
    use crate::encoder::{Encoder, StagedProperty, StagedSharedDict};
    use crate::test_helpers::{dec, parser};

    fn layer_buf(staged: StagedLayer01) -> Vec<u8> {
        StagedLayer::Tag01(staged)
            .encode_into(Encoder::default())
            .unwrap()
            .into_layer_bytes()
            .unwrap()
    }

    fn three_points() -> GeometryValues {
        let mut g = GeometryValues::default();
        g.push_geom(&Geometry::<i32>::Point(Point::new(1, 2)));
        g.push_geom(&Geometry::<i32>::Point(Point::new(3, 4)));
        g.push_geom(&Geometry::<i32>::Point(Point::new(5, 6)));
        g
    }

    fn empty_layer(name: &str) -> StagedLayer01 {
        StagedLayer01 {
            name: name.to_string(),
            extent: 4096,
            id: None,
            geometry: GeometryValues::default(),
            properties: vec![],
        }
    }

    #[test]
    fn prop_name_display_concatenates_parts() {
        assert_eq!(PropName("addr:", "city").to_string(), "addr:city");
        assert_eq!(PropName("name", "").to_string(), "name");
        assert_eq!(PropName("", "").to_string(), "");
    }

    #[test]
    fn prop_name_eq_str_matches_concatenation() {
        assert_eq!(PropName("addr:", "city"), "addr:city");
        assert_eq!("addr:city", PropName("addr:", "city"));
        assert_ne!(PropName("addr:", "city"), "addr:");
        assert_ne!(PropName("addr:", "city"), "city");
        assert_eq!(PropName("name", ""), "name");
    }

    #[test]
    fn prop_name_structural_eq_is_part_wise() {
        assert_eq!(PropName("a", "b"), PropName("a", "b"));
        assert_eq!(PropName("ab", ""), PropName("a", "b"));
    }

    #[test]
    fn prop_name_eq_prop_name_semantic_equality() {
        assert_eq!(PropName("ab", ""), PropName("a", "b"));
        assert_eq!(PropName("", "ab"), PropName("a", "b"));
        assert_eq!(PropName("abc", "def"), PropName("ab", "cdef"));
        assert_eq!(PropName("a", "bcdef"), PropName("abcde", "f"));

        assert_ne!(PropName("a", "b"), PropName("a", "c"));
        assert_ne!(PropName("a", "b"), PropName("ab", "c"));
        assert_ne!(PropName("abc", ""), PropName("ab", ""));
    }

    #[test]
    fn prop_value_ref_scalars_convert_to_json() {
        assert_eq!(Value::from(PropValueRef::Bool(true)), Value::Bool(true));
        assert_eq!(Value::from(PropValueRef::Bool(false)), Value::Bool(false));
        assert_eq!(Value::from(PropValueRef::I8(-1)), Value::from(-1_i8));
        assert_eq!(Value::from(PropValueRef::U8(255)), Value::from(255_u8));
        assert_eq!(
            Value::from(PropValueRef::I32(-1000)),
            Value::from(-1000_i32)
        );
        assert_eq!(Value::from(PropValueRef::U32(1000)), Value::from(1000_u32));
        assert_eq!(
            Value::from(PropValueRef::I64(i64::MIN)),
            Value::from(i64::MIN)
        );
        assert_eq!(
            Value::from(PropValueRef::U64(u64::MAX)),
            Value::from(u64::MAX)
        );
        assert_eq!(
            Value::from(PropValueRef::Str("hello")),
            Value::String("hello".into())
        );
    }

    #[test]
    fn prop_value_ref_float_finite_is_number() {
        assert!(matches!(
            Value::from(PropValueRef::F32(1.5)),
            Value::Number(_)
        ));
        assert!(matches!(
            Value::from(PropValueRef::F64(2.5)),
            Value::Number(_)
        ));
    }

    #[test]
    fn prop_value_ref_float_non_finite_becomes_string_sentinel() {
        assert_eq!(
            Value::from(PropValueRef::F32(f32::NAN)),
            Value::String("f32::NAN".into())
        );
        assert_eq!(
            Value::from(PropValueRef::F32(f32::INFINITY)),
            Value::String("f32::INFINITY".into())
        );
        assert_eq!(
            Value::from(PropValueRef::F64(f64::NAN)),
            Value::String("f64::NAN".into())
        );
        assert_eq!(
            Value::from(PropValueRef::F64(f64::NEG_INFINITY)),
            Value::String("f64::NEG_INFINITY".into())
        );
    }

    #[test]
    fn empty_layer_yields_no_features() {
        let buf = layer_buf(empty_layer("empty"));
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else {
            panic!("expected Tag01")
        };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        assert_eq!(parsed.iter_features().count(), 0);
        assert_eq!(parsed.iter_features().len(), 0);
        assert_eq!(parsed.iter_features().size_hint(), (0, Some(0)));
    }

    #[test]
    fn size_hint_and_exact_size_decrease_with_each_next() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();
        assert_eq!(iter.len(), 3);
        iter.next().unwrap().unwrap();
        assert_eq!(iter.len(), 2);
        iter.next().unwrap().unwrap();
        assert_eq!(iter.len(), 1);
        iter.next().unwrap().unwrap();
        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn feature_ids_are_preserved() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: Some(IdValues(vec![Some(100), None, Some(200)])),
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let ids: Vec<_> = parsed.iter_features().map(|r| r.unwrap().id).collect();
        assert_eq!(ids, [Some(100), None, Some(200)]);
    }

    #[test]
    fn geometry_values_match_input() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let geoms: Vec<_> = parsed
            .iter_features()
            .map(|r| r.unwrap().geometry)
            .collect();
        assert_eq!(geoms[0], Geometry::<i32>::Point(Point::new(1, 2)));
        assert_eq!(geoms[1], Geometry::<i32>::Point(Point::new(3, 4)));
        assert_eq!(geoms[2], Geometry::<i32>::Point(Point::new(5, 6)));
    }

    #[test]
    fn null_scalar_values_are_skipped() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::opt_u32("n", vec![Some(1), None, Some(3)])],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feats: Vec<_> = parsed.iter_features().map(|r| r.unwrap()).collect();

        let cols0: Vec<_> = feats[0].iter_properties().collect();
        assert_eq!(cols0.len(), 1);
        assert_eq!(cols0[0].name, PropName("n", ""));
        assert_eq!(cols0[0].name, "n");
        assert_eq!(cols0[0].value, PropValueRef::U32(1));

        assert!(feats[1].iter_properties().next().is_none());

        assert_eq!(feats[2].get_property("n"), Some(PropValueRef::U32(3)));

        // iter_all_properties yields Option<PropValueRef> (values only, no names)
        let all0: Vec<_> = feats[0].iter_all_properties().collect();
        assert_eq!(all0, [Some(PropValueRef::U32(1))]);
        let all1: Vec<_> = feats[1].iter_all_properties().collect();
        assert_eq!(all1, [None]); // null slot
        let all2: Vec<_> = feats[2].iter_all_properties().collect();
        assert_eq!(all2, [Some(PropValueRef::U32(3))]);

        // iterate_prop_names yields names from the layer
        let names: Vec<_> = parsed.iterate_prop_names().map(|n| n.to_string()).collect();
        assert_eq!(names, ["n"]);
    }

    #[test]
    fn null_string_values_are_skipped() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::opt_str(
                "label",
                vec![Some("foo"), None, Some("bar")],
            )],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feats: Vec<_> = parsed.iter_features().map(|r| r.unwrap()).collect();
        assert_eq!(
            feats[0].get_property("label"),
            Some(PropValueRef::Str("foo"))
        );
        assert_eq!(feats[1].get_property("label"), None);
        assert_eq!(
            feats[2].get_property("label"),
            Some(PropValueRef::Str("bar"))
        );
    }

    #[test]
    fn multiple_columns_independently_nullable() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![
                StagedProperty::opt_bool("flag", vec![Some(true), Some(false), None]),
                StagedProperty::opt_i32("score", vec![None, Some(-5), Some(7)]),
            ],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feats: Vec<_> = parsed.iter_features().map(|r| r.unwrap()).collect();

        // feat 0: flag=true, score=null → 1 property
        assert_eq!(feats[0].iter_properties().count(), 1);
        assert_eq!(
            feats[0].get_property("flag"),
            Some(PropValueRef::Bool(true))
        );
        assert_eq!(feats[0].get_property("score"), None);

        // feat 1: flag=false, score=-5 → 2 properties
        assert_eq!(feats[1].iter_properties().count(), 2);
        assert_eq!(
            feats[1].get_property("flag"),
            Some(PropValueRef::Bool(false))
        );
        assert_eq!(feats[1].get_property("score"), Some(PropValueRef::I32(-5)));

        // feat 2: flag=null, score=7 → 1 property
        assert_eq!(feats[2].iter_properties().count(), 1);
        assert_eq!(feats[2].get_property("flag"), None);
        assert_eq!(feats[2].get_property("score"), Some(PropValueRef::I32(7)));
    }

    #[test]
    fn get_property_absent_column_returns_none() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::u32("x", vec![1, 2, 3])],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feat = parsed.iter_features().next().unwrap().unwrap();
        assert_eq!(feat.get_property("no_such_column"), None);
    }

    #[test]
    fn shared_dict_columns_are_expanded() {
        let shared_dict = StagedSharedDict::new(
            "addr:",
            [
                ("city", vec![Some("Paris"), Some("Rome"), None]),
                ("zip", vec![Some("75001"), None, Some("00100")]),
            ],
        )
        .unwrap();

        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::SharedDict(shared_dict)],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feats: Vec<_> = parsed.iter_features().map(|r| r.unwrap()).collect();

        // feat 0: city=Paris, zip=75001
        assert_eq!(
            feats[0].get_property("addr:city"),
            Some(PropValueRef::Str("Paris"))
        );
        assert_eq!(
            feats[0].get_property("addr:zip"),
            Some(PropValueRef::Str("75001"))
        );
        assert_eq!(feats[0].iter_properties().count(), 2);

        // feat 1: city=Rome, zip=null
        assert_eq!(
            feats[1].get_property("addr:city"),
            Some(PropValueRef::Str("Rome"))
        );
        assert_eq!(feats[1].get_property("addr:zip"), None);
        assert_eq!(feats[1].iter_properties().count(), 1);

        // feat 2: city=null, zip=00100
        assert_eq!(feats[2].get_property("addr:city"), None);
        assert_eq!(
            feats[2].get_property("addr:zip"),
            Some(PropValueRef::Str("00100"))
        );

        // iter_all_properties: values only (no names); SharedDict expands to two slots
        let all: Vec<_> = feats[1].iter_all_properties().collect();
        assert_eq!(all, [Some(PropValueRef::Str("Rome")), None]);

        // iterate_prop_names: names from the layer, in order
        let names: Vec<_> = parsed.iterate_prop_names().map(|n| n.to_string()).collect();
        assert_eq!(names, ["addr:city", "addr:zip"]);
    }
}
