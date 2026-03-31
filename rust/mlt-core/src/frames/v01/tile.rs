//! Row-oriented "source form" for the optimizer.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! Conversion from [`TileLayer01`] to [`StagedLayer01`] is done via
//! [`StagedLayer01::from_tile`] (pre-computed [`StringGroup`] pairings produced by
//! [`crate::v01::group_string_properties`]) or the blanket [`From`] impl (no grouping).

use std::collections::HashMap;

use crate::errors::AsMltError as _;
use crate::v01::{
    GeometryValues, IdValues, Layer01, ParsedLayer01, ParsedProperty, PropValue, PropValueRef,
    StagedLayer01, StagedProperty, StagedScalar, StagedSharedDict, StagedStrings, StringGroup,
    TileFeature, TileLayer01,
};
use crate::{Decoder, MltResult};

impl ParsedLayer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer01`], charging every
    /// heap allocation against `dec`.
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer01> {
        let names: Vec<String> = self.iterate_prop_names().map(|n| n.to_string()).collect();
        let col_nulls = typed_nulls(&self.properties);
        let mut features = dec.alloc::<TileFeature>(self.feature_count())?;
        for feat in self.iter_features() {
            let feat = feat?;
            let mut values = dec.alloc::<PropValue>(names.len())?;
            for (col_idx, value) in feat.iter_all_properties().enumerate() {
                values.push(match value {
                    Some(v) => prop_value_from_ref(v),
                    None => col_nulls[col_idx].clone(),
                });
            }

            charge_str_props(dec, &values)?;

            features.push(TileFeature {
                id: feat.id,
                geometry: feat.geometry,
                properties: values,
            });
        }

        Ok(TileLayer01 {
            name: self.name.to_string(),
            extent: self.extent,
            property_names: names,
            features,
        })
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.geometry.vector_types.len()
    }
}

impl Layer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer01`]
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer01> {
        self.decode_all(dec)?.into_tile(dec)
    }
}

/// Convert a [`PropValueRef`] (as yielded by [`crate::v01::FeatureRef::iter_all_properties`])
/// into an owned [`PropValue`].
fn prop_value_from_ref(value: PropValueRef<'_>) -> PropValue {
    match value {
        PropValueRef::Bool(v) => PropValue::Bool(Some(v)),
        PropValueRef::I8(v) => PropValue::I8(Some(v)),
        PropValueRef::U8(v) => PropValue::U8(Some(v)),
        PropValueRef::I32(v) => PropValue::I32(Some(v)),
        PropValueRef::U32(v) => PropValue::U32(Some(v)),
        PropValueRef::I64(v) => PropValue::I64(Some(v)),
        PropValueRef::U64(v) => PropValue::U64(Some(v)),
        PropValueRef::F32(v) => PropValue::F32(Some(v)),
        PropValueRef::F64(v) => PropValue::F64(Some(v)),
        PropValueRef::Str(s) => PropValue::Str(Some(s.to_string())),
    }
}

/// Build a flat list of typed null [`PropValue`]s, one per logical column position
/// as yielded by [`crate::v01::FeatureRef::iter_all_properties`].
///
/// Each scalar column contributes one entry with its specific null variant (e.g.
/// `PropValue::Bool(None)`).  A `SharedDict` column expands to one `PropValue::Str(None)`
/// entry per sub-item.
fn typed_nulls(properties: &[ParsedProperty<'_>]) -> Vec<PropValue> {
    use ParsedProperty as PP;
    use PropValue as PV;
    let mut nulls = Vec::new();
    for prop in properties {
        match prop {
            PP::Bool(_) => nulls.push(PV::Bool(None)),
            PP::I8(_) => nulls.push(PV::I8(None)),
            PP::U8(_) => nulls.push(PV::U8(None)),
            PP::I32(_) => nulls.push(PV::I32(None)),
            PP::U32(_) => nulls.push(PV::U32(None)),
            PP::I64(_) => nulls.push(PV::I64(None)),
            PP::U64(_) => nulls.push(PV::U64(None)),
            PP::F32(_) => nulls.push(PV::F32(None)),
            PP::F64(_) => nulls.push(PV::F64(None)),
            PP::Str(_) => nulls.push(PV::Str(None)),
            PP::SharedDict(d) => {
                for _ in &d.items {
                    nulls.push(PV::Str(None));
                }
            }
        }
    }
    nulls
}

// ── TileLayer01 → StagedLayer01 ─────────────────────────────────────────────

impl StagedLayer01 {
    /// Construct a [`StagedLayer01`] from a row-oriented [`TileLayer01`], applying
    /// pre-computed [`StringGroup`] pairings to merge similar string columns into
    /// shared dictionaries.
    ///
    /// `groups` should be the output of [`crate::v01::group_string_properties`] called on the
    /// same [`TileLayer01`] source.  Because unique-value membership is
    /// row-order-independent, the same groups can be reused across sort trials.
    #[must_use]
    pub fn from_tile(mut source: TileLayer01, groups: &[StringGroup]) -> Self {
        let mut geometry = GeometryValues::default();
        for f in &source.features {
            geometry.push_geom(&f.geometry);
        }

        let id = if source.features.iter().any(|f| f.id.is_some()) {
            Some(IdValues(source.features.iter().map(|f| f.id).collect()))
        } else {
            None
        };

        let col_to_group: HashMap<_, _> = groups
            .iter()
            .flat_map(|g| g.columns.iter().map(move |(_, i)| (*i, g)))
            .collect();

        // first col_idx of each group → the group (emit the SharedDict here)
        let mut group_start: HashMap<_, _> = groups.iter().map(|g| (g.columns[0].1, g)).collect();

        let mut properties = Vec::with_capacity(source.property_names.len());
        for (col_idx, name) in source.property_names.into_iter().enumerate() {
            if let Some(g) = group_start.remove(&col_idx) {
                properties.push(build_shared_dict(g, &mut source.features));
            } else if !col_to_group.contains_key(&col_idx) {
                properties.push(build_scalar_column(name, col_idx, &mut source.features));
            } // else this column is part of a group we already consumed
        }

        Self {
            name: source.name,
            extent: source.extent,
            id,
            geometry,
            properties,
        }
    }
}

/// Convenience conversion — equivalent to [`StagedLayer01::from_tile`] with no grouping.
impl From<TileLayer01> for StagedLayer01 {
    fn from(source: TileLayer01) -> Self {
        Self::from_tile(source, &[])
    }
}

fn build_scalar_column(name: String, col: usize, features: &mut [TileFeature]) -> StagedProperty {
    // Determine the variant by peeking at the first feature value.
    // Typed nulls (e.g. `PropValue::Bool(None)`) already carry the column type,
    // so no filtering is needed; only a fully-absent column returns `None` here.
    // Fall back to `Str` if every feature has no value for this column.
    let first_val = features.iter().find_map(|f| f.properties.get(col));

    macro_rules! scalar_col {
        ($variant:ident, $ty:ty, $sv:ident) => {{
            let values: Vec<Option<$ty>> = features
                .iter()
                .map(|f| {
                    if let Some(PropValue::$sv(v)) = f.properties.get(col) {
                        *v
                    } else {
                        None
                    }
                })
                .collect();
            StagedProperty::$variant(StagedScalar { name, values })
        }};
    }

    match first_val {
        Some(PropValue::Bool(_)) => scalar_col!(Bool, bool, Bool),
        Some(PropValue::I8(_)) => scalar_col!(I8, i8, I8),
        Some(PropValue::U8(_)) => scalar_col!(U8, u8, U8),
        Some(PropValue::I32(_)) => scalar_col!(I32, i32, I32),
        Some(PropValue::U32(_)) => scalar_col!(U32, u32, U32),
        Some(PropValue::I64(_)) => scalar_col!(I64, i64, I64),
        Some(PropValue::U64(_)) => scalar_col!(U64, u64, U64),
        Some(PropValue::F32(_)) => scalar_col!(F32, f32, F32),
        Some(PropValue::F64(_)) => scalar_col!(F64, f64, F64),
        Some(PropValue::Str(_)) | None => {
            let values: Vec<Option<String>> = features
                .iter_mut()
                .map(|f| match f.properties.get_mut(col) {
                    Some(PropValue::Str(v)) => v.take(),
                    _ => None,
                })
                .collect();
            StagedProperty::Str(StagedStrings::from_optional(name, values))
        }
    }
}

fn build_shared_dict(group: &StringGroup, features: &mut [TileFeature]) -> StagedProperty {
    let mut order: Vec<usize> = (0..group.columns.len()).collect();
    order.sort_by_key(|&i| group.columns[i].1);

    let columns = order.into_iter().map(|i| {
        let (suffix, col_idx) = &group.columns[i];
        let values: Vec<Option<String>> = features
            .iter()
            .map(|f| match f.properties.get(*col_idx) {
                Some(PropValue::Str(s)) => s.clone(),
                _ => None,
            })
            .collect();
        (suffix.clone(), values)
    });

    StagedProperty::SharedDict(
        StagedSharedDict::new(group.prefix.clone(), columns).expect("StagedSharedDict succeed"),
    )
}

/// Charge `dec` for the heap bytes of owned `String` values inside `PropValue::Str`.
fn charge_str_props(dec: &mut Decoder, props: &[PropValue]) -> MltResult<()> {
    let str_bytes = props
        .iter()
        .filter_map(|p| {
            if let PropValue::Str(Some(s)) = p {
                Some(s.len())
            } else {
                None
            }
        })
        .try_fold(0u32, |acc, n| {
            acc.checked_add(u32::try_from(n).or_overflow()?)
                .or_overflow()
        })?;
    if str_bytes > 0 {
        dec.consume(str_bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use geo_types::Point;

    use super::*;
    use crate::geojson::Geom32;
    use crate::test_helpers::{dec, parser};
    use crate::v01::{GeometryValues, StagedLayer01, StagedProperty};
    use crate::{EncodedLayer, Layer};

    fn layer_tile(staged: StagedLayer01) -> TileLayer01 {
        let (enc, _) = staged.encode_auto().unwrap();
        let mut buf = Vec::new();
        EncodedLayer::Tag01(enc).write_to(&mut buf).unwrap();
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let mut d = dec();
        lazy.decode_all(&mut d).unwrap().into_tile(&mut d).unwrap()
    }

    fn two_points() -> GeometryValues {
        let mut g = GeometryValues::default();
        g.push_geom(&Geom32::Point(Point::new(0, 0)));
        g.push_geom(&Geom32::Point(Point::new(1, 1)));
        g
    }

    /// `into_tile` must produce a **typed** null (e.g. `PropValue::Bool(None)`)
    /// for null slots, matching the column's actual type, even when the **first**
    /// feature is null.
    #[test]
    fn null_first_feature_preserves_later_typed_value() {
        let tile = layer_tile(StagedLayer01 {
            name: "t".into(),
            extent: 4096,
            id: None,
            geometry: two_points(),
            properties: vec![StagedProperty::bool("flag", vec![None, Some(false)])],
        });

        assert_eq!(tile.property_names, vec!["flag"]);
        // Null slot → typed null matching the column type
        assert_eq!(tile.features[0].properties[0], PropValue::Bool(None));
        // Non-null value after the null must not be dropped
        assert_eq!(tile.features[1].properties[0], PropValue::Bool(Some(false)));
    }

    /// Every scalar type must produce a typed null for null slots and a typed
    /// non-null value for present slots, even when the first feature is null.
    #[test]
    fn null_first_feature_across_types() {
        let props = vec![
            StagedProperty::bool("b", vec![None, Some(true)]),
            StagedProperty::i8("i8", vec![None, Some(-1)]),
            StagedProperty::u8("u8", vec![None, Some(2)]),
            StagedProperty::i32("i32", vec![None, Some(-3)]),
            StagedProperty::u32("u32", vec![None, Some(4)]),
            StagedProperty::i64("i64", vec![None, Some(-5)]),
            StagedProperty::u64("u64", vec![None, Some(6)]),
            StagedProperty::f32("f32", vec![None, Some(7.0)]),
            StagedProperty::f64("f64", vec![None, Some(8.0)]),
            StagedProperty::str("s", vec![None, Some("ok".into())]),
        ];
        let tile = layer_tile(StagedLayer01 {
            name: "t".into(),
            extent: 4096,
            id: None,
            geometry: two_points(),
            properties: props,
        });

        // Feature 0: every column is null → typed null for each column
        let n = &tile.features[0].properties;
        assert_eq!(n[0], PropValue::Bool(None));
        assert_eq!(n[1], PropValue::I8(None));
        assert_eq!(n[2], PropValue::U8(None));
        assert_eq!(n[3], PropValue::I32(None));
        assert_eq!(n[4], PropValue::U32(None));
        assert_eq!(n[5], PropValue::I64(None));
        assert_eq!(n[6], PropValue::U64(None));
        assert_eq!(n[7], PropValue::F32(None));
        assert_eq!(n[8], PropValue::F64(None));
        assert_eq!(n[9], PropValue::Str(None));

        // Feature 1: every column has its typed non-null value
        let p = &tile.features[1].properties;
        assert_eq!(p[0], PropValue::Bool(Some(true)));
        assert_eq!(p[1], PropValue::I8(Some(-1)));
        assert_eq!(p[2], PropValue::U8(Some(2)));
        assert_eq!(p[3], PropValue::I32(Some(-3)));
        assert_eq!(p[4], PropValue::U32(Some(4)));
        assert_eq!(p[5], PropValue::I64(Some(-5)));
        assert_eq!(p[6], PropValue::U64(Some(6)));
        assert_eq!(p[7], PropValue::F32(Some(7.0)));
        assert_eq!(p[8], PropValue::F64(Some(8.0)));
        assert_eq!(p[9], PropValue::Str(Some("ok".into())));
    }
}
