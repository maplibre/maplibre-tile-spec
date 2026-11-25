//! Staging: from the features a [`LayerSource`] provides to the columnar [`StagedLayer`] the
//! wire encoders write.

#[cfg(feature = "unstable-v2")]
use std::collections::BTreeMap;

#[cfg(feature = "unstable-v2")]
use compact_str::CompactString;

use crate::PropValueRef;
#[cfg(feature = "unstable-v2")]
use crate::decoder::GeometryLayout;
use crate::decoder::GeometryValues;
#[cfg(any(test, feature = "__private"))]
use crate::encoder::SortStrategy;
use crate::encoder::geometry::GeometryStager;
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
#[cfg(any(test, feature = "__private"))]
use crate::tile::TileLayer;
#[cfg(feature = "unstable-v2")]
use crate::tile::{NestedKind, NestedValue, PropValue};
use crate::tile::{PropKind, VertexMoves};

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
            vertex_sort: false,
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
    /// Start each feature's rings, lines and points near where the previous feature ended.
    pub(crate) vertex_sort: bool,
}

impl StageOptions {
    /// Stage for `cfg`, with the triangles of polygons when `tessellate`.
    pub(crate) fn new(cfg: EncoderConfig, tessellate: bool) -> Self {
        Self {
            tessellate,
            widen_8bit: !cfg.allow_8bit_ints(),
            vertex_sort: cfg.vertex_sort(),
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
    #[cfg(feature = "unstable-v2")]
    let layout = source.v2_layout();
    let mut geometry = match () {
        // Its z is added below, with every other layer's.
        #[cfg(feature = "unstable-v2")]
        () if layout.meshes => GeometryValues {
            layout: GeometryLayout::Meshes,
            ..GeometryValues::default()
        },
        () if options.tessellate => GeometryValues::new_tessellated(),
        () => GeometryValues::default(),
    };
    geometry.reserve(
        source.feature_count(),
        order.iter().map(|f| source.coord_count(f)).sum(),
    );
    #[cfg(feature = "unstable-v2")]
    let track = source.v2_layout().z_axis.is_some() || !source.m_value_kinds().is_empty();
    #[cfg(not(feature = "unstable-v2"))]
    let track = false;
    let mut stager = GeometryStager::new(&mut geometry)
        .vertex_sorted(options.vertex_sort)
        .tracked(track);
    #[cfg_attr(
        not(feature = "unstable-v2"),
        expect(unused_variables, reason = "only v2 holds values per vertex")
    )]
    let moves: Vec<VertexMoves> = order
        .iter()
        .map(|f| {
            source.push_geometry(f, &mut stager);
            stager.finish_feature()
        })
        .collect();
    #[cfg(feature = "unstable-v2")]
    if let Some(axis) = layout.z_axis {
        let mut z = Vec::new();
        for (f, moves) in order.iter().zip(&moves) {
            moves.extend(&mut z, source.z(f), |&z| z);
        }
        geometry
            .add_z(axis, &z)
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
        m_values: build_m_values(source, order, &moves),
        #[cfg(feature = "unstable-v2")]
        nested: build_nested(source, order),
    }
}

/// Gather each m-value column out of the features, in staging order.
///
/// A feature with no values for a column clears its presence bit and contributes
/// nothing to the values, which is the whole of how m-values are null.
#[cfg(feature = "unstable-v2")]
fn build_m_values(
    source: &impl LayerSource,
    order: &Order,
    moves: &[VertexMoves],
) -> Vec<StagedMValue> {
    let kinds = source.m_value_kinds();
    let layout = source.v2_layout();
    let mut columns = Vec::with_capacity(kinds.len());
    for (index, &kind) in kinds.iter().enumerate() {
        let scope = layout.m_value_scope(index);
        let mut values = StagedValues::empty(kind);
        let presence: Vec<bool> = order
            .iter()
            .zip(moves)
            .map(|(f, moves)| source.push_m_values(f, index, moves, &mut values))
            .collect();
        // A column no feature is null on needs no mask at all.
        let presence = if presence.iter().all(|&p| p) {
            None
        } else {
            Some(presence)
        };
        columns.push(
            StagedMValue::new(source.m_value_name(index), presence, values).with_scope(scope),
        );
    }
    columns
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
        NestedKind::Leaf(leaf) => StagedNode::Leaf(StagedLeaf::new(
            presence,
            narrow_leaf(leaf_values(*leaf, &dense)),
        )),
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
            let entries: Vec<&BTreeMap<CompactString, NestedValue>> = dense
                .iter()
                .filter_map(|value| match value {
                    NestedValue::Map(entries) => entries.as_ref(),
                    NestedValue::Leaf(_) | NestedValue::List(_) => None,
                })
                .collect();
            let fields = fields
                .iter()
                .map(|(key, kind)| {
                    let values: Vec<Option<&NestedValue>> = entries
                        .iter()
                        .map(|entry| entry.get(key.as_str()))
                        .collect();
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

/// Narrow a 64-bit leaf to 32 bits when every value fits, as [`build_scalar_column`] does.
///
/// An empty leaf keeps its declared type, since there are no values to narrow it by.
#[cfg(feature = "unstable-v2")]
fn narrow_leaf(values: StagedValues) -> StagedValues {
    use StagedValues as V;

    fn fit<S: Copy, D: TryFrom<S>>(values: &[S]) -> Option<Vec<D>> {
        values.iter().map(|&v| D::try_from(v).ok()).collect()
    }
    match values {
        // `u32` is tried before `i32`, matching `build_scalar_column`.
        V::I64(v) if !v.is_empty() => fit(&v)
            .map(V::U32)
            .or_else(|| fit(&v).map(V::I32))
            .unwrap_or(V::I64(v)),
        V::U64(v) if !v.is_empty() => fit(&v).map_or(V::U64(v), V::U32),
        values @ (V::Bool(_)
        | V::I8(_)
        | V::U8(_)
        | V::I32(_)
        | V::U32(_)
        | V::I64(_)
        | V::U64(_)
        | V::F32(_)
        | V::F64(_)
        | V::Str(_)) => values,
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
    use crate::encoder::{Codecs, Encoder, EncoderConfig, Network, Profile};
    use crate::test_helpers::{dec, parser};
    use crate::{Layer, PropValue};

    fn layer_tile(staged: StagedLayer) -> TileLayer {
        let mut codecs = Codecs::default();
        let buf = staged
            .encode_into(
                Encoder::new(EncoderConfig::from_profile(Profile::Native(
                    Network::Cellular4G,
                ))),
                &mut codecs,
            )
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
