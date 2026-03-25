//! Zero-copy per-feature view into a fully-decoded [`Layer01<Parsed>`].
//!
//! [`Layer01FeatureIter`] is returned by [`Layer01::iter_features`]
//! and yields one [`FeatureRef`] per feature.  [`FeatureRef::iter_properties`] exposes
//! per-feature property values as flat [`ColumnRef`] items; `SharedDict` columns are
//! transparently expanded and null values are skipped.

use std::fmt;

use serde_json::Value;

use crate::geojson::Geom32;
use crate::utils::{f32_to_json, f64_to_json};
use crate::v01::{Layer01, ParsedProperty};
use crate::{MltResult, Parsed};

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
        // This is probably ok for performance as it shouldn't be in the hot path
        self.to_string().as_str() == other
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

impl From<PropValueRef<'_>> for Value {
    fn from(v: PropValueRef<'_>) -> Value {
        match v {
            PropValueRef::Bool(v) => Value::Bool(v),
            PropValueRef::I8(v) => Value::from(v),
            PropValueRef::U8(v) => Value::from(v),
            PropValueRef::I32(v) => Value::from(v),
            PropValueRef::U32(v) => Value::from(v),
            PropValueRef::I64(v) => Value::from(v),
            PropValueRef::U64(v) => Value::from(v),
            PropValueRef::F32(v) => f32_to_json(v),
            PropValueRef::F64(v) => f64_to_json(v),
            PropValueRef::Str(s) => Value::String(s.to_string()),
        }
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

/// A single map feature returned by [`Layer01FeatureIter`].
///
/// The internal columnar layout is private; use [`iter_properties`](Self::iter_properties)
/// and [`get_property`](Self::get_property) to access property values.
#[derive(Debug)]
pub struct FeatureRef<'a> {
    /// Optional feature ID.
    pub id: Option<u64>,
    /// Geometry in [`Geom32`] form (owned, computed on demand by the iterator).
    pub geometry: Geom32,
    columns: &'a [ParsedProperty<'a>],
    index: usize,
}

impl<'a> FeatureRef<'a> {
    /// Iterate over all non-null properties for this feature.
    ///
    /// `SharedDict` columns are transparently expanded into one [`ColumnRef`] per sub-item.
    /// Null / absent values are skipped entirely. The iterator is infallible.
    #[must_use]
    pub fn iter_properties(&self) -> FeatPropertyIter<'a> {
        FeatPropertyIter {
            columns: self.columns,
            feat_idx: self.index,
            col_idx: 0,
            sub_idx: 0,
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

/// Iterator over the non-null properties of a single feature.
///
/// Returned by [`FeatureRef::iter_properties`]. `SharedDict` columns are expanded
/// in-place without any heap allocation.
pub struct FeatPropertyIter<'a> {
    columns: &'a [ParsedProperty<'a>],
    feat_idx: usize,
    col_idx: usize,
    /// Sub-item cursor within the current `SharedDict` column; 0 for all other types.
    sub_idx: usize,
}

impl<'a> Iterator for FeatPropertyIter<'a> {
    type Item = ColumnRef<'a>;

    fn next(&mut self) -> Option<ColumnRef<'a>> {
        macro_rules! scalar {
            ($s:expr, $variant:ident) => {
                $s.values[self.feat_idx].map(|v| ColumnRef {
                    name: PropName($s.name, ""),
                    value: PropValueRef::$variant(v),
                })
            };
        }
        loop {
            let prop = self.columns.get(self.col_idx)?;
            self.col_idx += 1;
            let res = match prop {
                ParsedProperty::SharedDict(dict) => {
                    // iterate over SharedDict subitems - undo the idx+=1 until done with shared dict
                    self.col_idx -= 1;
                    while self.sub_idx < dict.items.len() {
                        let item = &dict.items[self.sub_idx];
                        self.sub_idx += 1;
                        if let Some(s) = item.get(dict, self.feat_idx) {
                            return Some(ColumnRef {
                                name: PropName(dict.prefix, item.suffix),
                                value: PropValueRef::Str(s),
                            });
                        }
                    }
                    self.col_idx += 1;
                    self.sub_idx = 0;
                    None
                }
                ParsedProperty::Bool(s) => scalar!(s, Bool),
                ParsedProperty::I8(s) => scalar!(s, I8),
                ParsedProperty::U8(s) => scalar!(s, U8),
                ParsedProperty::I32(s) => scalar!(s, I32),
                ParsedProperty::U32(s) => scalar!(s, U32),
                ParsedProperty::I64(s) => scalar!(s, I64),
                ParsedProperty::U64(s) => scalar!(s, U64),
                ParsedProperty::F32(s) => scalar!(s, F32),
                ParsedProperty::F64(s) => scalar!(s, F64),
                ParsedProperty::Str(s) => {
                    let i = u32::try_from(self.feat_idx).unwrap_or(u32::MAX);
                    s.get(i).map(|val| ColumnRef {
                        name: PropName(s.name, ""),
                        value: PropValueRef::Str(val),
                    })
                }
            };
            if let Some(v) = res {
                return Some(v);
            }
        }
    }
}

/// Iterator over the features of a fully-decoded [`Layer01<Parsed>`].
///
/// Returned by [`Layer01::iter_features`].
/// Geometry construction is the only fallible step; all property access is infallible.
///
/// The two lifetime parameters are `'layer` (lifetime of the borrow of the layer) and
/// `'data` (lifetime of the data the layer borrows from, e.g. the input buffer).
/// In most call sites both can be elided as `Layer01FeatureIter<'_, '_>`.
pub struct Layer01FeatureIter<'layer, 'data> {
    layer: &'layer Layer01<'data, Parsed>,
    index: usize,
}

impl<'layer, 'data> Layer01FeatureIter<'layer, 'data> {
    pub(crate) fn new(layer: &'layer Layer01<'data, Parsed>) -> Self {
        Self { layer, index: 0 }
    }
}

impl<'layer> Iterator for Layer01FeatureIter<'layer, '_> {
    type Item = MltResult<FeatureRef<'layer>>;

    fn next(&mut self) -> Option<Self::Item> {
        let count = self.layer.geometry.vector_types.len();
        if self.index >= count {
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
    use crate::test_helpers::{dec, parser};
    use crate::v01::{
        GeometryValues, IdValues, StagedLayer01, StagedProperty, StagedStrings,
        build_staged_shared_dict,
    };
    use crate::{EncodedLayer, Layer};

    fn layer_buf(staged: StagedLayer01) -> Vec<u8> {
        let (enc, _) = staged.encode_auto().unwrap();
        let mut buf = Vec::new();
        EncodedLayer::Tag01(enc).write_to(&mut buf).unwrap();
        buf
    }

    fn three_points() -> GeometryValues {
        let mut g = GeometryValues::default();
        g.push_geom(&Geom32::Point(Point::new(1, 2)));
        g.push_geom(&Geom32::Point(Point::new(3, 4)));
        g.push_geom(&Geom32::Point(Point::new(5, 6)));
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
        assert_eq!(geoms[0], Geom32::Point(Point::new(1, 2)));
        assert_eq!(geoms[1], Geom32::Point(Point::new(3, 4)));
        assert_eq!(geoms[2], Geom32::Point(Point::new(5, 6)));
    }

    #[test]
    fn null_scalar_values_are_skipped() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::u32("n", vec![Some(1), None, Some(3)])],
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
    }

    #[test]
    fn null_string_values_are_skipped() {
        let buf = layer_buf(StagedLayer01 {
            name: "test".into(),
            extent: 4096,
            id: None,
            geometry: three_points(),
            properties: vec![StagedProperty::str(
                "label",
                vec![Some("foo".into()), None, Some("bar".into())],
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
                StagedProperty::bool("flag", vec![Some(true), Some(false), None]),
                StagedProperty::i32("score", vec![None, Some(-5), Some(7)]),
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
            properties: vec![StagedProperty::u32("x", vec![Some(1), Some(2), Some(3)])],
        });
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let parsed = lazy.decode_all(&mut dec()).unwrap();

        let feat = parsed.iter_features().next().unwrap().unwrap();
        assert_eq!(feat.get_property("no_such_column"), None);
    }

    #[test]
    fn shared_dict_columns_are_expanded() {
        let shared_dict = build_staged_shared_dict(
            "addr:",
            vec![
                (
                    "city".into(),
                    StagedStrings::from(vec![Some("Paris".into()), Some("Rome".into()), None]),
                ),
                (
                    "zip".into(),
                    StagedStrings::from(vec![Some("75001".into()), None, Some("00100".into())]),
                ),
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
    }
}
