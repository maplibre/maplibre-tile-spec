//! Zero-copy per-feature view into a fully-decoded [`Layer01<Parsed>`].
//!
//! [`ParsedLayer01::iter_features`] yields one [`FeatureRef`] per feature via
//! [`LendingIterator`].  [`FeatureRef::iter_properties`] exposes per-feature
//! property values as flat [`ColumnRef`] items; `SharedDict` columns are
//! transparently expanded and null values are skipped.
//!
//! # Iterator model
//!
//! Feature iteration uses [`LendingIterator`] rather than [`std::iter::Iterator`].
//! This allows the iterator to reuse an internal buffer across steps — the
//! [`FeatureRef`] borrows its property values from that buffer — eliminating a
//! per-feature `Vec` allocation.
//!
//! The consequence is that each [`FeatureRef`] must be dropped before calling
//! [`LendingIterator::next`] again, so standard adapters like `.map()` and
//! `.collect()` are **not** available directly.  Use a `while let` loop instead:

use std::fmt;

use geo_types::Geometry;

use crate::decoder::{Layer01, ParsedLayer01, ParsedProperty, ParsedScalar, RawProperty};
use crate::{Lazy, LazyParsed, MltResult, Parsed};

/// A minimal lending (streaming) iterator trait.
///
/// Unlike [`std::iter::Iterator`], the item type may borrow from the iterator
/// itself, enabling zero-allocation iteration where the inner buffer is reused
/// across steps.
///
/// Use a `while let` loop to drive the iterator:
/// ```ignore
/// let mut iter = layer.iter_features();
/// while let Some(feat) = iter.next() {
///     let feat = feat?;
///     /* use feat here — it borrows from iter */
/// }
/// ```
pub trait LendingIterator {
    /// The type of each element, which may borrow from `self`.
    type Item<'this>
    where
        Self: 'this;

    /// Advance the iterator, returning the next element or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

impl<'a> Layer01<'a, Lazy> {
    /// Iterate over the property column names of this layer, in order.
    ///
    /// Regular columns yield one [`PropName`]; `SharedDict` columns yield one name per
    /// sub-item.  Names are available even before any column data has been decoded.
    ///
    /// Pair with [`FeatureRef::iter_all_properties`] to associate per-feature
    /// values with their column names.
    pub fn iterate_prop_names(&self) -> impl Iterator<Item = PropName<'a>> + '_ {
        let props = &self.properties;
        let mut col_idx = 0;
        let mut dict_idx = 0;
        std::iter::from_fn(move || {
            loop {
                let idx = col_idx;
                col_idx += 1;
                let name = match props.get(idx)? {
                    LazyParsed::Raw(r) => raw_col_name(r, &mut dict_idx),
                    LazyParsed::Parsed(p) => parsed_col_name(p, &mut dict_idx),
                    LazyParsed::ParsingFailed => None,
                };
                if dict_idx != 0 {
                    col_idx -= 1;
                }
                if let Some(n) = name {
                    return Some(n);
                }
            }
        })
    }
}

impl<'a> ParsedLayer01<'a> {
    /// Iterate over all features in this fully-decoded layer via a [`LendingIterator`].
    ///
    /// Yields one `MltResult<`[`FeatureRef`]`>` per feature. Geometry decoding can
    /// fail, hence the `Result` wrapper.
    ///
    /// ```text
    /// let mut iter = parsed.iter_features();
    /// while let Some(feat) = iter.next() {
    ///     let feat = feat?;
    ///     for col in feat.iter_properties() {
    ///        // or use iter_all_properties() to include Nones
    ///     }
    /// }
    /// ```
    ///
    /// All inner iterators — [`FeatureRef::iter_properties`],
    /// [`FeatureRef::iter_all_properties`], and the name iterators — implement the
    /// standard [`std::iter::Iterator`] trait and compose normally.
    #[must_use]
    pub fn iter_features(&self) -> Layer01FeatureIter<'_, 'a> {
        Layer01FeatureIter::new(self)
    }

    /// Iterate over the property column names of this layer, in order.
    /// See [`Layer01::iterate_prop_names`] for details.
    pub fn iterate_prop_names(&self) -> impl Iterator<Item = PropName<'a>> + '_ {
        Layer01PropNamesIter::new(&self.properties)
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

/// A single non-null property value for one feature, yielded by [`FeatureRef::iter_properties`].
///
/// `name` is a [`PropName`] that displays as `"{prefix}{suffix}"`.
/// All borrows are zero-copy from the layer data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRef<'a> {
    pub name: PropName<'a>,
    pub value: PropValueRef<'a>,
}

/// A single map feature returned by [`ParsedLayer01::iter_features`].
///
/// Borrows `values` from the outer [`Layer01FeatureIter`] buffer — it must be
/// dropped before calling [`LendingIterator::next`] again.
#[derive(Debug)]
pub struct FeatureRef<'feat, 'layer: 'feat> {
    /// Optional feature ID.
    pub id: Option<u64>,
    /// Geometry in [`Geometry<i32>`] form (owned, decoded on demand by the iterator).
    pub geometry: Geometry<i32>,
    /// Borrowed slice of column descriptors from the layer; used to yield column names.
    columns: &'layer [ParsedProperty<'layer>],
    /// Per-feature values in column order, one per slot (scalar, string, or `SharedDict`
    /// sub-item).  Borrowed from the iterator's reused buffer — no allocation per feature.
    values: &'feat [Option<PropValueRef<'layer>>],
}

impl<'feat, 'layer: 'feat> FeatureRef<'feat, 'layer> {
    /// Iterate over every property slot for this feature, **values only**, in column order.
    ///
    /// Yields `Option<PropValueRef>`:
    /// - `Some(value)` — the slot contains a non-null value.
    /// - `None` — the slot is null / absent.
    ///
    /// Use [`Layer01::iterate_prop_names`] to pair values with their column names.
    pub fn iter_all_properties(&self) -> impl Iterator<Item = Option<PropValueRef<'layer>>> + '_ {
        self.values.iter().copied()
    }

    /// Iterate over all non-null properties for this feature.
    ///
    /// `SharedDict` columns are transparently expanded into one [`ColumnRef`] per sub-item.
    /// Null / absent values are skipped entirely. The iterator is infallible.
    pub fn iter_properties(&self) -> impl Iterator<Item = ColumnRef<'layer>> + '_ {
        Layer01PropNamesIter::new(self.columns)
            .zip(self.values.iter().copied())
            .filter_map(|(name, opt_val)| opt_val.map(|value| ColumnRef { name, value }))
    }

    /// Look up a property by name, returning its value if present and non-null.
    ///
    /// For `SharedDict` columns the expected name is `"{prefix}{suffix}"`, matching
    /// the key used by [`iter_properties`](Self::iter_properties).
    #[must_use]
    pub fn get_property(&self, name: &str) -> Option<PropValueRef<'layer>> {
        self.iter_properties()
            .find(|col| col.name == name)
            .map(|col| col.value)
    }
}

// ── Column name helpers ───────────────────────────────────────────────────────

/// Iterates the property column names of a fully-decoded [`ParsedLayer01`].
///
/// Regular columns yield one [`PropName`]; `SharedDict` columns yield one name per
/// sub-item (`(prefix, suffix)`).
pub(crate) struct Layer01PropNamesIter<'a, 'p> {
    props: &'a [ParsedProperty<'p>],
    col_idx: usize,
    dict_idx: usize,
}

impl<'a, 'p> Layer01PropNamesIter<'a, 'p> {
    pub(crate) fn new(props: &'a [ParsedProperty<'p>]) -> Self {
        Self {
            props,
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
            let name = parsed_col_name(self.props.get(col_idx)?, &mut self.dict_idx);
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

/// A boxed per-column-slot value iterator yielding one `Option<`[`PropValueRef`]`>` per feature.
type ColValIter<'l> = Box<dyn Iterator<Item = Option<PropValueRef<'l>>> + 'l>;

/// Build one [`ColValIter`] per property column "slot" from a decoded column slice.
///
/// - Scalar and string columns contribute one slot each.
/// - `SharedDict` columns contribute one slot per sub-item.
fn build_col_iters<'p>(columns: &'p [ParsedProperty<'p>]) -> Vec<ColValIter<'p>> {
    use ParsedProperty as PP;
    let mut iters: Vec<ColValIter<'p>> = Vec::new();
    for col in columns {
        match col {
            PP::Bool(s) => iters.push(scalar_col_iter(s)),
            PP::I8(s) => iters.push(scalar_col_iter(s)),
            PP::U8(s) => iters.push(scalar_col_iter(s)),
            PP::I32(s) => iters.push(scalar_col_iter(s)),
            PP::U32(s) => iters.push(scalar_col_iter(s)),
            PP::I64(s) => iters.push(scalar_col_iter(s)),
            PP::U64(s) => iters.push(scalar_col_iter(s)),
            PP::F32(s) => iters.push(scalar_col_iter(s)),
            PP::F64(s) => iters.push(scalar_col_iter(s)),
            PP::Str(strings) => {
                let data: &'p str = strings.data.as_ref();
                let lengths: &'p [i32] = &strings.lengths;
                let mut curr_end: u32 = 0;
                let mut feat_idx = 0usize;
                iters.push(Box::new(std::iter::from_fn(move || {
                    let &end_i32 = lengths.get(feat_idx)?;
                    feat_idx += 1;
                    if end_i32 >= 0 {
                        let start = curr_end as usize;
                        curr_end = end_i32.cast_unsigned();
                        Some(data.get(start..curr_end as usize).map(PropValueRef::Str))
                    } else {
                        // Null slot: curr_end unchanged (null encodes the current byte offset).
                        Some(None)
                    }
                })));
            }
            PP::SharedDict(dict) => {
                for item in &dict.items {
                    let dict_ref: &'p _ = dict;
                    let item_ref: &'p _ = item;
                    let mut feat_idx = 0usize;
                    iters.push(Box::new(std::iter::from_fn(move || {
                        if feat_idx >= item_ref.ranges.len() {
                            return None;
                        }
                        let idx = feat_idx;
                        feat_idx += 1;
                        Some(item_ref.get(dict_ref, idx).map(PropValueRef::Str))
                    })));
                }
            }
        }
    }
    iters
}

/// Build a boxed value iterator for a single scalar property column.
fn scalar_col_iter<'p, T>(scalar: &'p ParsedScalar<'p, T>) -> ColValIter<'p>
where
    T: Copy + PartialEq,
    PropValueRef<'p>: From<T>,
{
    Box::new(scalar.iter_optional().map(|o| o.map(PropValueRef::from)))
}

/// Iterator over the features of a fully-decoded [`Layer01<Parsed>`].
///
/// Returned by [`ParsedLayer01::iter_features`]. Implements [`LendingIterator`]:
/// advance with `while let Some(feat) = iter.next()`.
///
/// Holds one O(1)-per-step cursor per property column slot. On each step the
/// per-column cursors are advanced and their results written into a reused
/// `values_buf` — yielding a [`FeatureRef`] that borrows that buffer with no
/// per-feature heap allocation.
pub struct Layer01FeatureIter<'layer, 'data: 'layer> {
    layer: &'layer Layer01<'data, Parsed>,
    index: usize,
    feature_count: usize,
    /// ID iterator, `None` when the layer has no ID column.
    id_iter: Option<crate::utils::PresenceOptIter<'layer, u64>>,
    /// One boxed value iterator per column slot (scalar, string, or `SharedDict` sub-item).
    col_iters: Vec<ColValIter<'layer>>,
    /// Reused buffer: filled on each `next()` call, borrowed by the yielded [`FeatureRef`].
    values_buf: Vec<Option<PropValueRef<'layer>>>,
}

impl<'layer, 'data: 'layer> Layer01FeatureIter<'layer, 'data> {
    fn new(layer: &'layer Layer01<'data, Parsed>) -> Self {
        let col_iters = build_col_iters(&layer.properties);
        let cap = col_iters.len();
        Self {
            layer,
            index: 0,
            feature_count: layer.feature_count(),
            id_iter: layer.id.as_ref().map(|id| id.iter_optional()),
            col_iters,
            values_buf: Vec::with_capacity(cap),
        }
    }

    /// Number of features not yet yielded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.feature_count - self.index
    }

    /// Returns `true` if all features have been yielded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index >= self.feature_count
    }
}

impl<'layer> LendingIterator for Layer01FeatureIter<'layer, '_> {
    type Item<'this>
        = MltResult<FeatureRef<'this, 'layer>>
    where
        Self: 'this;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        let index = self.index;
        if index >= self.feature_count {
            return None;
        }
        self.index += 1;

        // Advance all per-feature cursors unconditionally, even if geometry decode fails,
        // so that IDs and property values remain aligned with geometry indices.
        let id = self.id_iter.as_mut().and_then(Iterator::next).flatten();
        self.values_buf.clear();
        self.values_buf
            .extend(self.col_iters.iter_mut().map(|it| it.next().flatten()));

        Some(
            self.layer
                .geometry
                .to_geojson(index)
                .map(|geometry| FeatureRef {
                    id,
                    geometry,
                    columns: &self.layer.properties,
                    values: &self.values_buf,
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use geo_types::Point;
    use serde_json::Value;

    use super::*;
    use crate::Layer;
    use crate::decoder::GeometryValues;
    use crate::encoder::model::StagedLayer;
    use crate::encoder::{Codecs, Encoder, Presence, StagedId, StagedProperty, StagedSharedDict};
    use crate::test_helpers::{dec, parser};

    fn layer_buf(staged: StagedLayer) -> Vec<u8> {
        staged
            .encode_into(Encoder::default(), &mut Codecs::default())
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

    fn empty_layer(name: &str) -> StagedLayer {
        StagedLayer {
            name: name.to_string(),
            extent: 4096,
            id: StagedId::None,
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

        let iter = parsed.iter_features();
        assert_eq!(iter.len(), 0);
        assert!(iter.is_empty());
        assert_eq!(parsed.iter_features().len(), 0);
    }

    #[test]
    fn len_decreases_with_each_next() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
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
        assert!(iter.is_empty());
        assert!(iter.next().is_none());
    }

    #[test]
    fn feature_ids_are_preserved() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::from_optional(vec![Some(100), None, Some(200)]),
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut ids = Vec::new();
        let mut iter = parsed.iter_features();
        while let Some(r) = iter.next() {
            ids.push(r.unwrap().id);
        }
        assert_eq!(ids, [Some(100), None, Some(200)]);
    }

    #[test]
    fn geometry_values_match_input() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut geoms = Vec::new();
        let mut iter = parsed.iter_features();
        while let Some(r) = iter.next() {
            geoms.push(r.unwrap().geometry);
        }
        assert_eq!(geoms[0], Geometry::<i32>::Point(Point::new(1, 2)));
        assert_eq!(geoms[1], Geometry::<i32>::Point(Point::new(3, 4)));
        assert_eq!(geoms[2], Geometry::<i32>::Point(Point::new(5, 6)));
    }

    #[test]
    fn null_scalar_values_are_skipped() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![StagedProperty::opt_u32("n", vec![Some(1), None, Some(3)])],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();

        {
            let feat = iter.next().unwrap().unwrap();
            let cols: Vec<_> = feat.iter_properties().collect();
            assert_eq!(cols.len(), 1);
            assert_eq!(cols[0].name, PropName("n", ""));
            assert_eq!(cols[0].name, "n");
            assert_eq!(cols[0].value, PropValueRef::U32(1));
            let all: Vec<_> = feat.iter_all_properties().collect();
            assert_eq!(all, [Some(PropValueRef::U32(1))]);
        }
        {
            let feat = iter.next().unwrap().unwrap();
            assert!(feat.iter_properties().next().is_none());
            let all: Vec<_> = feat.iter_all_properties().collect();
            assert_eq!(all, [None]);
        }
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.get_property("n"), Some(PropValueRef::U32(3)));
            let all: Vec<_> = feat.iter_all_properties().collect();
            assert_eq!(all, [Some(PropValueRef::U32(3))]);
        }

        let names: Vec<_> = parsed.iterate_prop_names().map(|n| n.to_string()).collect();
        assert_eq!(names, ["n"]);
    }

    #[test]
    fn null_string_values_are_skipped() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![StagedProperty::opt_str(
                "label",
                vec![Some("foo"), None, Some("bar")],
            )],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.get_property("label"), Some(PropValueRef::Str("foo")));
        }
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.get_property("label"), None);
        }
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.get_property("label"), Some(PropValueRef::Str("bar")));
        }
    }

    #[test]
    fn multiple_columns_independently_nullable() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![
                StagedProperty::opt_bool("flag", vec![Some(true), Some(false), None]),
                StagedProperty::opt_i32("score", vec![None, Some(-5), Some(7)]),
            ],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();

        // feat 0: flag=true, score=null → 1 property
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.iter_properties().count(), 1);
            assert_eq!(feat.get_property("flag"), Some(PropValueRef::Bool(true)));
            assert_eq!(feat.get_property("score"), None);
        }
        // feat 1: flag=false, score=-5 → 2 properties
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.iter_properties().count(), 2);
            assert_eq!(feat.get_property("flag"), Some(PropValueRef::Bool(false)));
            assert_eq!(feat.get_property("score"), Some(PropValueRef::I32(-5)));
        }
        // feat 2: flag=null, score=7 → 1 property
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.iter_properties().count(), 1);
            assert_eq!(feat.get_property("flag"), None);
            assert_eq!(feat.get_property("score"), Some(PropValueRef::I32(7)));
        }
    }

    #[test]
    fn geometry_error_does_not_misalign_ids() {
        use crate::decoder::GeometryType;

        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::from_optional(vec![Some(10), Some(20), Some(30)]),
            geometry: three_points(),
            properties: vec![],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let mut parsed = lazy.decode_all(&mut dec()).unwrap();

        // Corrupt feature 1's geometry type: Point → LineString.
        // A LineString requires part_offsets, which are absent here, so
        // to_geojson(1) will return Err(NoPartOffsets).
        parsed.geometry.vector_types[1] = GeometryType::LineString;

        let mut iter = parsed.iter_features();

        // Feature 0: valid Point, id = Some(10)
        let feat0 = iter.next().unwrap().unwrap();
        assert_eq!(feat0.id, Some(10));

        // Feature 1: geometry error — iterator still advances ID cursor
        assert!(iter.next().unwrap().is_err());

        // Feature 2: valid Point, id must be Some(30), not Some(20)
        let feat2 = iter.next().unwrap().unwrap();
        assert_eq!(
            feat2.id,
            Some(30),
            "id cursor was not advanced on geometry error"
        );

        assert!(iter.next().is_none());
    }

    #[test]
    fn get_property_absent_column_returns_none() {
        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![StagedProperty::u32("x", vec![1, 2, 3])],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();
        let feat = iter.next().unwrap().unwrap();
        assert_eq!(feat.get_property("no_such_column"), None);
    }

    #[test]
    fn shared_dict_columns_are_expanded() {
        let shared_dict = StagedSharedDict::new(
            "addr:",
            [
                (
                    "city",
                    vec![Some("Paris"), Some("Rome"), None],
                    Presence::Mixed,
                ),
                (
                    "zip",
                    vec![Some("75001"), None, Some("00100")],
                    Presence::Mixed,
                ),
            ],
        )
        .unwrap();

        let buf = layer_buf(StagedLayer {
            name: "test".into(),
            extent: 4096,
            id: StagedId::None,
            geometry: three_points(),
            properties: vec![StagedProperty::SharedDict(shared_dict)],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let mut iter = parsed.iter_features();

        // feat 0: city=Paris, zip=75001
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(
                feat.get_property("addr:city"),
                Some(PropValueRef::Str("Paris"))
            );
            assert_eq!(
                feat.get_property("addr:zip"),
                Some(PropValueRef::Str("75001"))
            );
            assert_eq!(feat.iter_properties().count(), 2);
        }
        // feat 1: city=Rome, zip=null
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(
                feat.get_property("addr:city"),
                Some(PropValueRef::Str("Rome"))
            );
            assert_eq!(feat.get_property("addr:zip"), None);
            assert_eq!(feat.iter_properties().count(), 1);
            // iter_all_properties: values only (no names); SharedDict expands to two slots
            let all: Vec<_> = feat.iter_all_properties().collect();
            assert_eq!(all, [Some(PropValueRef::Str("Rome")), None]);
        }
        // feat 2: city=null, zip=00100
        {
            let feat = iter.next().unwrap().unwrap();
            assert_eq!(feat.get_property("addr:city"), None);
            assert_eq!(
                feat.get_property("addr:zip"),
                Some(PropValueRef::Str("00100"))
            );
        }

        let names: Vec<_> = parsed.iterate_prop_names().map(|n| n.to_string()).collect();
        assert_eq!(names, ["addr:city", "addr:zip"]);
    }
}
