//! Row-oriented "source form" for the optimizer.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! Conversion from [`TileLayer01`] to `StagedLayer01` is done via
//! `StagedLayer01::from_tile` (pre-computed `StringGroup` pairings produced by
//! `group_string_properties`) or the blanket [`From`] impl (no grouping).

use std::collections::HashMap;

use crate::decoder::{GeometryValues, IdValues, PropValue, TileFeature, TileLayer01};
use crate::encoder::model::StagedLayer01;
use crate::encoder::{SortStrategy, StagedProperty, StagedSharedDict, StringGroup};

impl StagedLayer01 {
    /// Construct a [`StagedLayer01`] from a row-oriented [`TileLayer01`], applying
    /// pre-computed `StringGroup` pairings to merge similar string columns into
    /// shared dictionaries.
    ///
    /// `groups` should be the output of `group_string_properties` called on the
    /// same [`TileLayer01`] source.  Because unique-value membership is
    /// row-order-independent, the same groups can be reused across sort trials.
    ///
    /// When `tessellate` is `true`, polygon and multi-polygon geometries have
    /// their triangulation stored alongside the geometry.
    #[must_use]
    #[hotpath::measure]
    pub fn from_tile(
        mut source: TileLayer01,
        sort: SortStrategy,
        groups: &[StringGroup],
        tessellate: bool,
    ) -> Self {
        assert!(!source.features.is_empty(), "empty tile");
        source.sort(sort);
        let mut geometry = if tessellate {
            GeometryValues::new_tessellated()
        } else {
            GeometryValues::default()
        };
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

fn build_scalar_column(name: String, col: usize, features: &mut [TileFeature]) -> StagedProperty {
    // Determine the variant by peeking at the first feature value.
    // Typed nulls (e.g. `PropValue::Bool(None)`) already carry the column type,
    // so no filtering is needed; only a fully-absent column returns `None` here.
    // Fall back to `Str` if every feature has no value for this column.
    let first_val = features.iter().find_map(|f| f.properties.get(col));

    // Collect optional values and check whether any are null. When no nulls
    // exist, use the non-optional staged variant (no presence stream written).
    macro_rules! scalar_col {
        ($opt_ctor:ident, $non_opt_ctor:ident, $ty:ty, $sv:ident) => {{
            let opt_values: Vec<Option<$ty>> = features
                .iter()
                .map(|f| {
                    if let Some(PropValue::$sv(v)) = f.properties.get(col) {
                        *v
                    } else {
                        None
                    }
                })
                .collect();
            if opt_values.iter().any(Option::is_none) {
                StagedProperty::$opt_ctor(name, opt_values)
            } else {
                StagedProperty::$non_opt_ctor(name, opt_values.into_iter().flatten().collect())
            }
        }};
    }

    match first_val {
        Some(PropValue::Bool(_)) => scalar_col!(opt_bool, bool, bool, Bool),
        Some(PropValue::I8(_)) => scalar_col!(opt_i8, i8, i8, I8),
        Some(PropValue::U8(_)) => scalar_col!(opt_u8, u8, u8, U8),
        Some(PropValue::I32(_)) => scalar_col!(opt_i32, i32, i32, I32),
        Some(PropValue::U32(_)) => scalar_col!(opt_u32, u32, u32, U32),
        Some(PropValue::I64(_)) => scalar_col!(opt_i64, i64, i64, I64),
        Some(PropValue::U64(_)) => scalar_col!(opt_u64, u64, u64, U64),
        Some(PropValue::F32(_)) => scalar_col!(opt_f32, f32, f32, F32),
        Some(PropValue::F64(_)) => scalar_col!(opt_f64, f64, f64, F64),
        Some(PropValue::Str(_)) | None => {
            let opt_values: Vec<Option<String>> = features
                .iter_mut()
                .map(|f| match f.properties.get_mut(col) {
                    Some(PropValue::Str(v)) => v.take(),
                    _ => None,
                })
                .collect();
            if opt_values.iter().any(Option::is_none) {
                StagedProperty::opt_str(name, opt_values)
            } else {
                StagedProperty::str(name, opt_values.into_iter().flatten())
            }
        }
    }
}

fn build_shared_dict(group: &StringGroup, features: &mut [TileFeature]) -> StagedProperty {
    let mut order: Vec<usize> = (0..group.columns.len()).collect();
    order.sort_by_key(|&i| group.columns[i].1);

    let columns = order.into_iter().map(|i| {
        let (suffix, col_idx) = &group.columns[i];
        let values: Vec<Option<String>> = features
            .iter_mut()
            .map(|f| match f.properties.get_mut(*col_idx) {
                Some(PropValue::Str(s)) => s.take(),
                _ => None,
            })
            .collect();
        (suffix.clone(), values)
    });

    StagedProperty::SharedDict(
        StagedSharedDict::new(group.prefix.clone(), columns).expect("StagedSharedDict succeed"),
    )
}

#[cfg(test)]
mod tests {
    use geo_types::Point;

    use super::*;
    use crate::Layer;
    use crate::decoder::GeometryValues;
    use crate::encoder::{Encoder, StagedLayer};
    use crate::test_helpers::{dec, parser};

    fn layer_tile(staged: StagedLayer01) -> TileLayer01 {
        let buf = StagedLayer::Tag01(staged)
            .encode_into(Encoder::default())
            .unwrap()
            .into_layer_bytes()
            .unwrap();
        let (_, layer) = Layer::from_bytes(&buf, &mut parser()).unwrap();
        let Layer::Tag01(lazy) = layer else { panic!() };
        let mut d = dec();
        lazy.decode_all(&mut d).unwrap().into_tile(&mut d).unwrap()
    }

    fn two_points() -> GeometryValues {
        let mut g = GeometryValues::default();
        g.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(0, 0)));
        g.push_geom(&geo_types::Geometry::<i32>::Point(Point::new(1, 1)));
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
            properties: vec![StagedProperty::opt_bool("flag", vec![None, Some(false)])],
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
            StagedProperty::opt_bool("b", vec![None, Some(true)]),
            StagedProperty::opt_i8("i8", vec![None, Some(-1)]),
            StagedProperty::opt_u8("u8", vec![None, Some(2)]),
            StagedProperty::opt_i32("i32", vec![None, Some(-3)]),
            StagedProperty::opt_u32("u32", vec![None, Some(4)]),
            StagedProperty::opt_i64("i64", vec![None, Some(-5)]),
            StagedProperty::opt_u64("u64", vec![None, Some(6)]),
            StagedProperty::opt_f32("f32", vec![None, Some(7.0)]),
            StagedProperty::opt_f64("f64", vec![None, Some(8.0)]),
            StagedProperty::opt_str("s", vec![None, Some("ok")]),
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
