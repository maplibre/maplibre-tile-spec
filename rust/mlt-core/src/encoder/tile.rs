//! Staging: from the features a [`LayerSource`] provides to the columnar [`StagedLayer`] the
//! wire encoders write.

#[cfg(feature = "unstable-v2")]
use std::collections::BTreeMap;

use crate::PropValueRef;
use crate::decoder::GeometryValues;
#[cfg(any(test, feature = "__private"))]
use crate::encoder::SortStrategy;
#[cfg(any(test, feature = "__private"))]
use crate::encoder::model::CurveParams;
use crate::encoder::model::StagedLayer;
use crate::encoder::optimizer::{LayerStats, Presence, PropertyStats, SharedDictRole};
#[cfg(any(test, feature = "__private"))]
use crate::encoder::sort::sort_order;
use crate::encoder::source::{LayerSource, Order};
use crate::encoder::{EncoderConfig, StagedId, StagedProperty, StagedSharedDict};
#[cfg(feature = "unstable-v2")]
use crate::encoder::{
    StagedInterior, StagedLeaf, StagedList, StagedMValue, StagedNested, StagedNode, StagedStruct,
    StagedValues,
};
use crate::tile::PropKind;
#[cfg(any(test, feature = "__private"))]
use crate::tile::TileLayer;
#[cfg(feature = "unstable-v2")]
use crate::tile::{MValue, NestedKind, NestedValue, PropValue};

#[cfg(any(test, feature = "__private"))]
impl StagedLayer {
    /// Construct a [`StagedLayer`] from a row-oriented [`TileLayer`] using
    /// pre-computed layer statistics and curve parameters.
    ///
    /// `curve_params` is taken as a parameter (rather than recomputed here)
    /// so a single [`TileLayer::curve_params`] scan feeds every sort trial
    /// and the encoder's dictionary builders.
    ///
    /// When `tessellate` is `true`, polygon and multi-polygon geometries have
    /// their triangulation stored alongside the geometry.
    #[must_use]
    pub fn from_tile(
        source: &TileLayer,
        sort: SortStrategy,
        stats: &LayerStats,
        tessellate: bool,
        curve_params: CurveParams,
    ) -> Self {
        assert!(source.feature_count() != 0, "empty tile");
        let order = sort_order(source, sort, curve_params);
        let options = StageOptions {
            tessellate,
            widen_8bit: false,
        };
        stage_layer(source, &order, stats, options)
    }
}

/// How [`stage_layer`] stages a layer.
#[derive(Clone, Copy)]
pub(crate) struct StageOptions {
    /// Store the triangulation of polygons alongside their geometry.
    pub(crate) tessellate: bool,
    /// Stage 8-bit integer columns as 32-bit ones, for encoders that lack them.
    pub(crate) widen_8bit: bool,
}

impl From<EncoderConfig> for StageOptions {
    fn from(cfg: EncoderConfig) -> Self {
        Self {
            tessellate: cfg.tessellate(),
            widen_8bit: !cfg.allow_8bit_ints(),
        }
    }
}

/// Stage the features of `source`, in `order`, using pre-computed layer statistics.
#[hotpath::measure]
pub(crate) fn stage_layer(
    source: &impl LayerSource,
    order: &Order,
    stats: &LayerStats,
    options: StageOptions,
) -> StagedLayer {
    let mut geometry = if options.tessellate {
        GeometryValues::new_tessellated()
    } else {
        GeometryValues::default()
    };
    geometry.reserve(
        source.feature_count(),
        order.iter().map(|f| source.coord_count(f)).sum(),
    );
    for f in order.iter() {
        source.push_geometry(f, &mut geometry);
    }
    #[cfg(feature = "unstable-v2")]
    if let Some(step) = source.v2_layout().z_step {
        let mut z = Vec::new();
        for f in order.iter() {
            let feature_z = source.z(f);
            match source.vertex_order(f) {
                Some(moved) if moved.len() == feature_z.len() => {
                    z.extend(moved.iter().map(|&i| feature_z[i]));
                }
                _ => z.extend_from_slice(feature_z),
            }
        }
        geometry
            .add_z(step, &z)
            .expect("the source holds one z per stored vertex");
    }

    let id = StagedId::from_optional_with_presence(
        order.iter().map(|f| source.id(f)),
        stats.id.as_ref(),
    );

    let property_count = source.property_count();
    let shared_dict_columns = shared_dict_columns(stats);
    let mut properties = Vec::with_capacity(property_count);
    for (col_idx, shared_cols) in shared_dict_columns.iter().enumerate().take(property_count) {
        let prop_analysis = stats
            .properties
            .get(col_idx)
            .expect("analysis matches source property columns");
        match prop_analysis.stats.shared_dict() {
            SharedDictRole::Owner(prefix) => {
                properties.push(build_shared_dict(
                    &prefix,
                    shared_cols,
                    source,
                    order,
                    stats,
                ));
            }
            SharedDictRole::Member(_) => {}
            SharedDictRole::None => {
                if let Some(prop) =
                    build_scalar_column(source, order, col_idx, prop_analysis, options.widen_8bit)
                {
                    properties.push(prop);
                }
            }
        }
    }

    StagedLayer {
        name: source.name().to_owned(),
        extent: source.extent(),
        id,
        geometry,
        properties,
        #[cfg(feature = "unstable-v2")]
        m_values: build_m_values(source, order),
        #[cfg(feature = "unstable-v2")]
        nested: build_nested(source, order),
    }
}

/// Gather each m-value column out of the features, in staging order.
///
/// A feature with no values for a column clears its presence bit and contributes
/// nothing to the values, which is the whole of how m-values are null.
#[cfg(feature = "unstable-v2")]
fn build_m_values(source: &impl LayerSource, order: &Order) -> Vec<StagedMValue> {
    /// Gather one column, naming the run and the staged column that hold its kind of values.
    macro_rules! column {
        ($index:expr, $variant:ident) => {{
            let (presence, values) = take_column(source, order, $index, |m_value| match m_value {
                MValue::$variant(run) => Some(run.as_deref()),
                _ => None,
            });
            (presence, StagedValues::$variant(values))
        }};
    }

    let layout = source.v2_layout();
    let mut columns = Vec::with_capacity(layout.m_value_names.len());
    for (index, (name, &kind)) in layout
        .m_value_names
        .iter()
        .zip(layout.m_value_kinds)
        .enumerate()
    {
        let (presence, values) = match kind {
            PropKind::Bool => column!(index, Bool),
            PropKind::I8 => column!(index, I8),
            PropKind::U8 => column!(index, U8),
            PropKind::I32 => column!(index, I32),
            PropKind::U32 => column!(index, U32),
            PropKind::I64 => column!(index, I64),
            PropKind::U64 => column!(index, U64),
            PropKind::F32 => column!(index, F32),
            PropKind::F64 => column!(index, F64),
            PropKind::Str => column!(index, Str),
        };
        // A column no feature is null on needs no mask at all.
        let presence = if presence.iter().all(|&p| p) {
            None
        } else {
            Some(presence)
        };
        columns.push(StagedMValue::new(name.clone(), presence, values));
    }
    columns
}

/// Column `index` of the features in staging order: which of them carry values, and the
/// values themselves, flat.
///
/// `run` projects a feature's value to its run of `T`, and is [`None`] only for a
/// value of another kind, which a source never holds.
#[cfg(feature = "unstable-v2")]
fn take_column<T: Clone>(
    source: &impl LayerSource,
    order: &Order,
    index: usize,
    run: fn(&MValue) -> Option<Option<&[T]>>,
) -> (Vec<bool>, Vec<T>) {
    let mut presence = Vec::with_capacity(source.feature_count());
    let mut values = Vec::new();
    for f in order.iter() {
        let Some(m_value) = source.m_value(f, index) else {
            presence.push(false);
            continue;
        };
        let run = run(m_value).expect("m-value kind matches its column");
        presence.push(run.is_some());
        let run = run.unwrap_or_default();
        match source.vertex_order(f) {
            Some(moved) if moved.len() == run.len() => {
                values.extend(moved.iter().map(|&i| run[i].clone()));
            }
            _ => values.extend_from_slice(run),
        }
    }
    (presence, values)
}

/// Shred each nested column out of the features, in staging order.
///
/// A map is shredded as a struct here, whichever of the two the writer then picks.
#[cfg(feature = "unstable-v2")]
fn build_nested(source: &impl LayerSource, order: &Order) -> Vec<StagedNested> {
    let layout = source.v2_layout();
    let mut columns = Vec::with_capacity(layout.nested_names.len());
    for (index, (name, kind)) in layout
        .nested_names
        .iter()
        .zip(layout.nested_kinds)
        .enumerate()
    {
        let inputs: Vec<Option<&NestedValue>> =
            order.iter().map(|f| source.nested(f, index)).collect();
        let StagedNode::Interior(root) = shred(kind, &inputs) else {
            unreachable!("a nested column's root is never a leaf")
        };
        columns.push(StagedNested::new(name.clone(), root));
    }
    columns
}

/// Shred one node's worth of values, one per value its parent hands it.
///
/// An input that is missing or null clears the node's presence bit and contributes
/// nothing below it, which is the whole of how a nested value is null.
#[cfg(feature = "unstable-v2")]
fn shred(kind: &NestedKind, inputs: &[Option<&NestedValue>]) -> StagedNode {
    let present = |value: Option<&NestedValue>| value.is_some_and(|v| !v.is_null());
    let mask: Vec<bool> = inputs.iter().map(|value| present(*value)).collect();
    let presence = if mask.iter().all(|&bit| bit) {
        None
    } else {
        Some(mask)
    };
    let dense: Vec<&NestedValue> = inputs
        .iter()
        .copied()
        .filter(|value| present(*value))
        .flatten()
        .collect();

    match kind {
        NestedKind::Leaf(leaf) => {
            StagedNode::Leaf(StagedLeaf::new(presence, leaf_values(*leaf, &dense)))
        }
        NestedKind::List(element) => {
            let mut lengths = Vec::with_capacity(dense.len());
            let mut items: Vec<Option<&NestedValue>> = Vec::new();
            for value in &dense {
                let NestedValue::List(Some(list)) = value else {
                    continue;
                };
                lengths.push(u32::try_from(list.len()).unwrap_or(u32::MAX));
                items.extend(list.iter().map(Some));
            }
            StagedNode::Interior(StagedInterior::List(StagedList::new(
                presence,
                lengths,
                shred(element, &items),
            )))
        }
        NestedKind::Map(fields) => {
            let entries: Vec<&BTreeMap<String, NestedValue>> = dense
                .iter()
                .filter_map(|value| match value {
                    NestedValue::Map(entries) => entries.as_ref(),
                    NestedValue::Leaf(_) | NestedValue::List(_) => None,
                })
                .collect();
            let fields = fields
                .iter()
                .map(|(key, kind)| {
                    let values: Vec<Option<&NestedValue>> =
                        entries.iter().map(|entry| entry.get(key)).collect();
                    (key.clone(), shred(kind, &values))
                })
                .collect::<Vec<_>>();
            StagedNode::Interior(StagedInterior::Struct(StagedStruct::new(presence, fields)))
        }
    }
}

/// Gather a leaf's non-null values into the staged column that holds its type.
#[cfg(feature = "unstable-v2")]
fn leaf_values(kind: PropKind, dense: &[&NestedValue]) -> StagedValues {
    /// Pick the values of one type out of the leaves that hold them.
    macro_rules! leaf {
        ($variant:ident) => {
            StagedValues::$variant(
                dense
                    .iter()
                    .filter_map(|value| match value {
                        NestedValue::Leaf(PropValue::$variant(Some(v))) => Some(v.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };
    }
    match kind {
        PropKind::Bool => leaf!(Bool),
        PropKind::I8 => leaf!(I8),
        PropKind::U8 => leaf!(U8),
        PropKind::I32 => leaf!(I32),
        PropKind::U32 => leaf!(U32),
        PropKind::I64 => leaf!(I64),
        PropKind::U64 => leaf!(U64),
        PropKind::F32 => leaf!(F32),
        PropKind::F64 => leaf!(F64),
        PropKind::Str => leaf!(Str),
    }
}

fn shared_dict_columns(stats: &LayerStats) -> Vec<Vec<usize>> {
    let mut columns = vec![Vec::new(); stats.properties.len()];
    for (col_idx, prop) in stats.properties.iter().enumerate() {
        match prop.stats.shared_dict() {
            SharedDictRole::Owner(_) => columns[col_idx].push(col_idx),
            SharedDictRole::Member(owner_col) => columns[owner_col].push(col_idx),
            SharedDictRole::None => {}
        }
    }
    columns
}

fn build_scalar_column(
    source: &impl LayerSource,
    order: &Order,
    col: usize,
    analysis: &PropertyStats,
    widen_8bit: bool,
) -> Option<StagedProperty> {
    let PropertyStats { presence, stats } = analysis;
    if *presence == Presence::AllNull {
        return None;
    }
    let name = source.property_name(col).to_owned();
    let values = order.iter().map(|f| source.property(f, col));

    // Presence is precomputed before sort trials; this pass only gathers values
    // in the selected row order. `$convert` widens or narrows to the staged type.
    macro_rules! column {
        ($opt_ctor:ident, $ctor:ident, $variant:ident, $convert:expr) => {{
            let values = values.map(|v| match v {
                Some(PropValueRef::$variant(v)) => Some($convert(v)),
                _ => None,
            });
            Some(if *presence == Presence::AllPresent {
                let values = values.map(|v| v.expect("analysis guarantees present typed values"));
                StagedProperty::$ctor(name, values.collect::<Vec<_>>())
            } else {
                StagedProperty::$opt_ctor(name, values)
            })
        }};
    }

    match source.property_kinds()[col] {
        PropKind::Bool => column!(opt_bool, bool, Bool, |v| v),
        PropKind::I8 if widen_8bit => column!(opt_i32, i32, I8, i32::from),
        PropKind::I8 => column!(opt_i8, i8, I8, |v| v),
        PropKind::U8 if widen_8bit => column!(opt_u32, u32, U8, u32::from),
        PropKind::U8 => column!(opt_u8, u8, U8, |v| v),
        PropKind::I32 => column!(opt_i32, i32, I32, |v| v),
        PropKind::U32 => column!(opt_u32, u32, U32, |v| v),
        // `u32` is tried before `i32` since `values_fit_u32` requires `min >= 0`.
        PropKind::I64 if stats.values_fit_u32() => column!(opt_u32, u32, I64, narrow),
        PropKind::I64 if stats.values_fit_i32() => column!(opt_i32, i32, I64, narrow),
        PropKind::I64 => column!(opt_i64, i64, I64, |v| v),
        PropKind::U64 if stats.values_fit_u32() => column!(opt_u32, u32, U64, narrow),
        PropKind::U64 => column!(opt_u64, u64, U64, |v| v),
        PropKind::F32 => column!(opt_f32, f32, F32, |v| v),
        PropKind::F64 => column!(opt_f64, f64, F64, |v| v),
        PropKind::Str => column!(opt_str, str, Str, |v| v),
    }
}

/// Narrowing a 64-bit column to 32 bits makes the `FastPFOR` codec (32-bit only) eligible.
fn narrow<S, D>(value: S) -> D
where
    D: TryFrom<S>,
    D::Error: std::fmt::Debug,
{
    D::try_from(value).expect("analyzed range guarantees value fits narrowed type")
}

fn build_shared_dict(
    prefix: &str,
    shared_dict_columns: &[usize],
    source: &impl LayerSource,
    order: &Order,
    analysis: &LayerStats,
) -> StagedProperty {
    let columns = shared_dict_columns.iter().copied().map(|col_idx| {
        let name = source.property_name(col_idx);
        let suffix = name.strip_prefix(prefix).unwrap_or(name).to_owned();
        let values = order
            .iter()
            .map(move |f| match source.property(f, col_idx) {
                Some(PropValueRef::Str(s)) => Some(s),
                _ => None,
            });
        let presence = analysis.properties[col_idx].presence;
        (suffix, values, presence)
    });

    StagedProperty::SharedDict(
        StagedSharedDict::new(prefix.to_owned(), columns).expect("StagedSharedDict succeed"),
    )
}

#[cfg(test)]
mod tests {
    use geo_types::Point;

    use super::*;
    use crate::decoder::GeometryValues;
    use crate::encoder::{Codecs, Encoder};
    use crate::test_helpers::{dec, parser};
    use crate::{Layer, PropValue};

    fn layer_tile(staged: StagedLayer) -> TileLayer {
        let mut codecs = Codecs::default();
        let buf = staged
            .encode_into(Encoder::default(), &mut codecs)
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
        let tile = layer_tile(
            StagedLayer::new(
                "t",
                4096,
                StagedId::None,
                two_points(),
                vec![StagedProperty::opt_bool("flag", vec![None, Some(false)])],
            )
            .unwrap(),
        );

        assert_eq!(tile.property_names(), &["flag"]);
        // Null slot -> typed null matching the column type
        assert_eq!(tile.features()[0].properties()[0], PropValue::Bool(None));
        // Non-null value after the null must not be dropped
        assert_eq!(
            tile.features()[1].properties()[0],
            PropValue::Bool(Some(false))
        );
    }

    /// Every scalar type must produce a typed null for null slots and a typed
    /// non-null value for present slots, even when the first feature is null.
    #[test]
    fn null_first_feature_across_types() {
        let props = vec![
            StagedProperty::opt_bool("b", vec![None, Some(true)]),
            StagedProperty::opt_i32("i32", vec![None, Some(-3)]),
            StagedProperty::opt_u32("u32", vec![None, Some(4)]),
            StagedProperty::opt_i64("i64", vec![None, Some(-5)]),
            StagedProperty::opt_u64("u64", vec![None, Some(6)]),
            StagedProperty::opt_f32("f32", vec![None, Some(7.0)]),
            StagedProperty::opt_f64("f64", vec![None, Some(8.0)]),
            StagedProperty::opt_str("s", vec![None, Some("ok")]),
        ];
        let tile =
            layer_tile(StagedLayer::new("t", 4096, StagedId::None, two_points(), props).unwrap());

        // Feature 0: every column is null -> typed null for each column
        let n = tile.features()[0].properties();
        assert_eq!(n[0], PropValue::Bool(None));
        assert_eq!(n[1], PropValue::I32(None));
        assert_eq!(n[2], PropValue::U32(None));
        assert_eq!(n[3], PropValue::I64(None));
        assert_eq!(n[4], PropValue::U64(None));
        assert_eq!(n[5], PropValue::F32(None));
        assert_eq!(n[6], PropValue::F64(None));
        assert_eq!(n[7], PropValue::Str(None));

        // Feature 1: every column has its typed non-null value
        let p = tile.features()[1].properties();
        assert_eq!(p[0], PropValue::Bool(Some(true)));
        assert_eq!(p[1], PropValue::I32(Some(-3)));
        assert_eq!(p[2], PropValue::U32(Some(4)));
        assert_eq!(p[3], PropValue::I64(Some(-5)));
        assert_eq!(p[4], PropValue::U64(Some(6)));
        assert_eq!(p[5], PropValue::F32(Some(7.0)));
        assert_eq!(p[6], PropValue::F64(Some(8.0)));
        assert_eq!(p[7], PropValue::Str(Some("ok".into())));
    }
}
