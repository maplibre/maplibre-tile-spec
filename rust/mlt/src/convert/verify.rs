//! Checks that an encoded layer decodes back to the layer it was encoded from.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use anyhow::{Context as _, Result as AnyResult, anyhow, bail, ensure};
use mlt_core::encoder::{EncoderConfig, wound_layer};
use mlt_core::geo_types::{Coord, Geometry, LineString, Polygon};
use mlt_core::{Decoder, MValue, NestedValue, Parser, PropValue, TileFeature, TileLayer};

/// Decode `encoded`, hand it to `restore`, and require the result to hold what `source` does.
///
/// The encoder chooses the feature order, each integer column's width and the winding of each polygon ring, so none of them counts.
/// Under [`EncoderConfig::vertex_sort`] it also chooses where each ring starts, the direction of each line and the order of each `MultiPoint`.
/// A null value is the same as no value.
/// A layer without features encodes to no bytes.
pub fn check_round_trip(
    source: &TileLayer,
    encoded: &[u8],
    config: EncoderConfig,
    restore: impl FnOnce(TileLayer) -> AnyResult<TileLayer>,
) -> AnyResult<()> {
    let mut layers = Parser::default().parse_layers(encoded)?.into_iter();
    let Some(layer) = layers.next() else {
        ensure!(
            source.features().is_empty(),
            "the encoded bytes hold no layer"
        );
        return Ok(());
    };
    ensure!(
        layers.next().is_none(),
        "the encoded bytes hold more than one layer"
    );
    let decoded = layer
        .into_tile(&mut Decoder::default())?
        .ok_or_else(|| anyhow!("the encoded layer has an unknown tag"))?;
    let vertices = if config.vertex_sort() {
        Vertices::AnyOrder
    } else {
        Vertices::Stored
    };
    same_layer(&wound_layer(source), &restore(decoded)?, vertices)
        .with_context(|| format!("layer {} does not decode back to its input", source.name()))
}

/// Which orders of a feature's vertices count as the same.
#[derive(Clone, Copy)]
enum Vertices {
    /// Only the stored order.
    Stored,
    /// Any rotation of a ring, either direction of a line and any order of a `MultiPoint`.
    AnyOrder,
}

fn same_layer(expected: &TileLayer, actual: &TileLayer, vertices: Vertices) -> AnyResult<()> {
    ensure!(
        expected.name() == actual.name(),
        "named {:?}, not {:?}",
        actual.name(),
        expected.name()
    );
    ensure!(
        expected.extent() == actual.extent(),
        "extent {}, not {}",
        actual.extent().get(),
        expected.extent().get()
    );
    ensure!(
        expected.z_axis() == actual.z_axis(),
        "z axis {:?}, not {:?}",
        actual.z_axis(),
        expected.z_axis()
    );
    ensure!(
        expected.feature_count() == actual.feature_count(),
        "{} features, not {}",
        actual.feature_count(),
        expected.feature_count()
    );
    for (e, a) in rows(expected, vertices)
        .iter()
        .zip(rows(actual, vertices).iter())
    {
        let id =
            e.id.map_or_else(|| "without an id".to_owned(), |id| id.to_string());
        ensure!(e.id == a.id, "feature {id} is missing");
        ensure!(
            e.geometry == a.geometry,
            "feature {id} has another geometry"
        );
        ensure!(
            e.z == a.z,
            "feature {id} has other z: {:?}, not {:?}",
            a.z,
            e.z
        );
        if let Some(difference) = first_difference(&e.values, &a.values) {
            bail!("feature {id} has other values: {difference}");
        }
    }
    Ok(())
}

/// One feature, its non-null values keyed by the kind of column and its name.
struct Row<'a> {
    id: Option<u64>,
    geometry: Geometry<i32>,
    /// Empty in a flat layer.
    z: Vec<i32>,
    values: BTreeMap<(Role, &'a str), Value<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    Property,
    MValue,
    Nested,
}

/// A value with its storage type left out: integers by number, floats by their `f64` bits.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Value<'a> {
    Bool(bool),
    Int(i128),
    Float(Bits),
    Str(&'a str),
    List(Vec<Option<Self>>),
    Map(BTreeMap<&'a str, Self>),
}

/// An `f64` compared by its bits, so a NaN equals itself.
#[derive(Clone, Copy)]
struct Bits(f64);

impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Bits {}

impl PartialOrd for Bits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bits {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bits().cmp(&other.0.to_bits())
    }
}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> Value<'a> {
    /// `value`, or [`None`] if it is null.
    fn of(value: &'a PropValue) -> Option<Self> {
        Some(match value {
            PropValue::Bool(v) => Self::Bool((*v)?),
            PropValue::I8(v) => Self::Int((*v)?.into()),
            PropValue::U8(v) => Self::Int((*v)?.into()),
            PropValue::I32(v) => Self::Int((*v)?.into()),
            PropValue::U32(v) => Self::Int((*v)?.into()),
            PropValue::I64(v) => Self::Int((*v)?.into()),
            PropValue::U64(v) => Self::Int((*v)?.into()),
            PropValue::F32(v) => Self::Float(Bits(f64::from((*v)?))),
            PropValue::F64(v) => Self::Float(Bits((*v)?)),
            PropValue::Str(v) => Self::Str(v.as_deref()?),
        })
    }

    fn of_m_value(value: &'a MValue) -> Option<Self> {
        fn ints<T: Copy + Into<i128>>(v: &[T]) -> Value<'_> {
            Value::List(v.iter().map(|&x| Some(Value::Int(x.into()))).collect())
        }
        fn floats(v: impl Iterator<Item = f64>) -> Value<'static> {
            Value::List(v.map(|x| Some(Value::Float(Bits(x)))).collect())
        }
        Some(match value {
            MValue::Bool(v) => {
                Self::List(v.as_ref()?.iter().map(|&b| Some(Self::Bool(b))).collect())
            }
            MValue::I8(v) => ints(v.as_ref()?),
            MValue::U8(v) => ints(v.as_ref()?),
            MValue::I32(v) => ints(v.as_ref()?),
            MValue::U32(v) => ints(v.as_ref()?),
            MValue::I64(v) => ints(v.as_ref()?),
            MValue::U64(v) => ints(v.as_ref()?),
            MValue::F32(v) => floats(v.as_ref()?.iter().map(|&x| f64::from(x))),
            MValue::F64(v) => floats(v.as_ref()?.iter().copied()),
            MValue::Str(v) => Self::List(v.as_ref()?.iter().map(|s| Some(Self::Str(s))).collect()),
        })
    }

    /// A null inside a list stays a null item, never an empty list.
    fn of_nested(value: &'a NestedValue) -> Option<Self> {
        Some(match value {
            NestedValue::Leaf(v) => Self::of(v)?,
            NestedValue::List(items) => {
                Self::List(items.as_ref()?.iter().map(Self::of_nested).collect())
            }
            NestedValue::Map(entries) => Self::Map(
                entries
                    .as_ref()?
                    .iter()
                    .filter_map(|(k, v)| Some((k.as_str(), Self::of_nested(v)?)))
                    .collect(),
            ),
        })
    }
}

/// The features of `layer` in an order the encoder cannot change, each with its vertices in one order of those `vertices` allows.
fn rows(layer: &TileLayer, vertices: Vertices) -> Vec<Row<'_>> {
    let mut rows: Vec<(Shape, Row<'_>)> = layer
        .features()
        .iter()
        .map(|f| {
            let row = row(layer, f, vertices);
            (Shape::of(&row.geometry), row)
        })
        .collect();
    rows.sort_by(|(a_shape, a), (b_shape, b)| {
        (a.id, a_shape, &a.z, &a.values).cmp(&(b.id, b_shape, &b.z, &b.values))
    });
    rows.into_iter().map(|(_, row)| row).collect()
}

/// A geometry's coordinates, which unlike [`Geometry`] are ordered.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    Point(Xy),
    Line(Xy, Xy),
    LineString(Vec<Xy>),
    Polygon(Vec<Vec<Xy>>),
    MultiPoint(Vec<Xy>),
    MultiLineString(Vec<Vec<Xy>>),
    MultiPolygon(Vec<Vec<Vec<Xy>>>),
    Rect(Xy, Xy),
    Triangle(Xy, Xy, Xy),
    Collection(Vec<Self>),
}

type Xy = (i32, i32);

impl Shape {
    fn of(geometry: &Geometry<i32>) -> Self {
        let xy = |c: Coord<i32>| (c.x, c.y);
        let line = |l: &LineString<i32>| l.coords().copied().map(xy).collect::<Vec<_>>();
        let polygon = |p: &Polygon<i32>| {
            std::iter::once(p.exterior())
                .chain(p.interiors())
                .map(line)
                .collect::<Vec<_>>()
        };
        match geometry {
            Geometry::Point(p) => Self::Point(xy(p.0)),
            Geometry::Line(l) => Self::Line(xy(l.start), xy(l.end)),
            Geometry::LineString(l) => Self::LineString(line(l)),
            Geometry::Polygon(p) => Self::Polygon(polygon(p)),
            Geometry::MultiPoint(mp) => Self::MultiPoint(mp.iter().map(|p| xy(p.0)).collect()),
            Geometry::MultiLineString(ml) => Self::MultiLineString(ml.iter().map(line).collect()),
            Geometry::MultiPolygon(mp) => Self::MultiPolygon(mp.iter().map(polygon).collect()),
            Geometry::Rect(r) => Self::Rect(xy(r.min()), xy(r.max())),
            Geometry::Triangle(t) => Self::Triangle(xy(t.v1()), xy(t.v2()), xy(t.v3())),
            Geometry::GeometryCollection(gc) => Self::Collection(gc.iter().map(Self::of).collect()),
        }
    }
}

fn row<'a>(layer: &'a TileLayer, feature: &'a TileFeature, vertices: Vertices) -> Row<'a> {
    let mut values = BTreeMap::new();
    for (name, value) in layer.property_names().iter().zip(feature.properties()) {
        if let Some(value) = Value::of(value) {
            values.insert((Role::Property, name.as_str()), value);
        }
    }
    for (name, value) in layer.m_value_names().iter().zip(feature.m_values()) {
        if let Some(value) = Value::of_m_value(value) {
            values.insert((Role::MValue, name.as_str()), value);
        }
    }
    for (name, value) in layer.nested_names().iter().zip(feature.nested()) {
        if let Some(value) = Value::of_nested(value) {
            values.insert((Role::Nested, name.as_str()), value);
        }
    }
    let mut z = feature.z().to_vec();
    let geometry = match vertices {
        Vertices::Stored => feature.geometry().clone(),
        Vertices::AnyOrder => {
            let mut per_vertex: Vec<&mut Vec<Option<Value<'a>>>> = values
                .iter_mut()
                .filter(|((role, _), _)| *role == Role::MValue)
                .filter_map(|(_, value)| match value {
                    Value::List(items) => Some(items),
                    Value::Bool(_)
                    | Value::Int(_)
                    | Value::Float(_)
                    | Value::Str(_)
                    | Value::Map(_) => None,
                })
                .collect();
            let mut least = LeastOrder {
                z: &z,
                per_vertex: &per_vertex,
                order: Vec::new(),
            };
            let geometry = least.geometry(feature.geometry());
            let order = least.order;
            if order.len() == z.len() {
                z = order.iter().map(|&i| z[i]).collect();
            }
            for items in &mut per_vertex {
                if order.len() == items.len() {
                    let mut taken: Vec<_> = items.drain(..).map(Some).collect();
                    items.extend(
                        order
                            .iter()
                            .map(|&i| taken[i].take().expect("each vertex is stored once")),
                    );
                }
            }
            geometry
        }
    };
    Row {
        id: feature.id(),
        geometry,
        z,
        values,
    }
}

/// One vertex with everything held for it, compared to pick one of the orders [`Vertices::AnyOrder`] allows.
type Key<'v, 'a> = (Xy, Option<i32>, Vec<Option<&'v Option<Value<'a>>>>);

/// Puts each part of a geometry in its least order by [`Key`], recording where each stored vertex came from.
struct LeastOrder<'v, 'a> {
    z: &'v [i32],
    per_vertex: &'v [&'v mut Vec<Option<Value<'a>>>],
    /// Entry `i` is the source position of the vertex stored at `i`.
    order: Vec<usize>,
}

impl<'a> LeastOrder<'_, 'a> {
    fn key(&self, at: usize, c: Coord<i32>) -> Key<'_, 'a> {
        (
            (c.x, c.y),
            self.z.get(at).copied(),
            self.per_vertex.iter().map(|items| items.get(at)).collect(),
        )
    }

    fn geometry(&mut self, geometry: &Geometry<i32>) -> Geometry<i32> {
        let polygon = |this: &mut Self, p: &Polygon<i32>| {
            let exterior = this.ring(p.exterior());
            let interiors = p.interiors().iter().map(|r| this.ring(r)).collect();
            Polygon::new(exterior, interiors)
        };
        match geometry {
            Geometry::Point(_) => {
                self.order.push(self.order.len());
                geometry.clone()
            }
            Geometry::LineString(l) => Geometry::LineString(self.line(l)),
            Geometry::Polygon(p) => Geometry::Polygon(polygon(self, p)),
            Geometry::MultiPoint(mp) => {
                let base = self.order.len();
                let mut at: Vec<usize> = (0..mp.0.len()).collect();
                at.sort_by(|&i, &j| {
                    self.key(base + i, mp.0[i].0)
                        .cmp(&self.key(base + j, mp.0[j].0))
                });
                self.order.extend(at.iter().map(|&i| base + i));
                Geometry::MultiPoint(at.iter().map(|&i| mp.0[i]).collect())
            }
            Geometry::MultiLineString(ml) => {
                Geometry::MultiLineString(ml.iter().map(|l| self.line(l)).collect())
            }
            Geometry::MultiPolygon(mp) => {
                Geometry::MultiPolygon(mp.iter().map(|p| polygon(self, p)).collect())
            }
            Geometry::Line(_)
            | Geometry::Rect(_)
            | Geometry::Triangle(_)
            | Geometry::GeometryCollection(_) => geometry.clone(),
        }
    }

    /// The line in whichever direction has the lesser keys.
    fn line(&mut self, line: &LineString<i32>) -> LineString<i32> {
        let base = self.order.len();
        let keys: Vec<_> = (line.0.iter().enumerate())
            .map(|(i, &c)| self.key(base + i, c))
            .collect();
        let at: Vec<usize> = if keys.iter().rev().lt(keys.iter()) {
            (0..line.0.len()).rev().collect()
        } else {
            (0..line.0.len()).collect()
        };
        self.order.extend(at.iter().map(|&i| base + i));
        LineString(at.iter().map(|&i| line.0[i]).collect())
    }

    /// The ring of 3 or more stored vertices rotated to start where its keys are least, closed again if it was.
    fn ring(&mut self, ring: &LineString<i32>) -> LineString<i32> {
        let coords = &ring.0;
        let closed = coords.len() > 1 && coords.first() == coords.last();
        let len = if closed {
            coords.len() - 1
        } else {
            coords.len()
        };
        let base = self.order.len();
        let keys: Vec<_> = (coords[..len].iter().enumerate())
            .map(|(i, &c)| self.key(base + i, c))
            .collect();
        let rotated = |start: usize| (0..len).map(move |i| (start + i) % len);
        let start = if len >= 3 { least_rotation(&keys) } else { 0 };
        self.order.extend(rotated(start).map(|i| base + i));
        let mut out: Vec<_> = rotated(start).map(|i| coords[i]).collect();
        if closed {
            out.extend(out.first().copied());
        }
        LineString(out)
    }
}

/// The earliest start of the least rotation of `keys`, in linear time.
fn least_rotation<T: Ord>(keys: &[T]) -> usize {
    let n = keys.len();
    let (mut i, mut j, mut k) = (0, 1, 0);
    while i < n && j < n && k < n {
        match keys[(i + k) % n].cmp(&keys[(j + k) % n]) {
            Ordering::Equal => k += 1,
            Ordering::Greater => {
                i += k + 1;
                if i == j {
                    i += 1;
                }
                k = 0;
            }
            Ordering::Less => {
                j += k + 1;
                if i == j {
                    j += 1;
                }
                k = 0;
            }
        }
    }
    i.min(j)
}

/// The first column whose value differs, described, or [`None`] if every value matches.
fn first_difference(
    expected: &BTreeMap<(Role, &str), Value<'_>>,
    actual: &BTreeMap<(Role, &str), Value<'_>>,
) -> Option<String> {
    let show = |v: Option<&Value<'_>>| v.map_or_else(|| "nothing".to_owned(), |v| format!("{v:?}"));
    expected
        .keys()
        .chain(actual.keys())
        .map(|key| (key, expected.get(key), actual.get(key)))
        .find(|(_, e, a)| e != a)
        .map(|(key, e, a)| format!("{:?} {} is {}, not {}", key.0, key.1, show(a), show(e)))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use mlt_core::PropKind;
    use mlt_core::geo_types::{MultiPoint, Point};

    use super::*;

    fn layer_of(kind: PropKind, value: PropValue, id: u64) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        let key = layer.add_property("v", kind).unwrap();
        let mut row = layer.feature(Point::new(1, 2).into());
        row.id(Some(id));
        row.property(key, value).unwrap();
        row.finish().unwrap();
        layer.finish()
    }

    #[test]
    fn an_integer_column_may_change_its_width() {
        let wide = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        let narrow = layer_of(PropKind::U32, PropValue::U32(Some(7)), 1);
        same_layer(&wide, &narrow, Vertices::Stored).unwrap();
    }

    fn layer_with_z(step: Option<i8>, z: Vec<i32>) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        if let Some(step) = step {
            layer
                .set_z_axis(mlt_core::ZStep::new(step).unwrap())
                .unwrap();
        }
        let mut row = layer.feature(Point::new(1, 2).into());
        row.id(Some(1));
        if !z.is_empty() {
            row.z(z).unwrap();
        }
        row.finish().unwrap();
        layer.finish()
    }

    #[test]
    fn a_dropped_z_step_is_reported() {
        let a = layer_with_z(Some(0), vec![100]);
        let b = layer_with_z(None, vec![]);
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @"z axis None, not Some(ZAxis { step: ZStep(0), mode: Absolute })");
    }

    #[test]
    fn a_changed_z_names_the_feature() {
        let a = layer_with_z(Some(0), vec![100]);
        let b = layer_with_z(Some(0), vec![101]);
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @"feature 1 has other z: [101], not [100]");
    }

    #[test]
    fn a_rewound_ring_carries_its_z_and_m_values_along() {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        layer.set_z_axis(mlt_core::ZStep::new(0).unwrap()).unwrap();
        let key = layer.add_m_value("m", PropKind::I32).unwrap();
        let clockwise = Polygon::new(
            LineString::from(vec![(0, 0), (0, 9), (9, 9), (9, 0), (0, 0)]),
            vec![],
        );
        let mut row = layer.feature(clockwise.into());
        row.z(vec![10, 20, 30, 40]).unwrap();
        row.m_value(key, MValue::I32(Some(vec![1, 2, 3, 4])))
            .unwrap();
        row.finish().unwrap();
        let layer = layer.finish();
        let config = EncoderConfig::from_profile(mlt_core::encoder::Profile::Native(
            mlt_core::encoder::Network::Cellular4G,
        ))
        .with_wire_version(mlt_core::encoder::WireVersion::V02);
        let encoded = layer.clone().encode(config).unwrap();
        check_round_trip(&layer, &encoded, config, Ok).unwrap();
    }

    #[test]
    fn a_vertex_sorted_layer_verifies_with_its_z_and_m_values() {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        layer.set_z_axis(mlt_core::ZStep::new(0).unwrap()).unwrap();
        let key = layer.add_m_value("m", PropKind::I32).unwrap();
        let features: [(Geometry<i32>, Vec<i32>); 4] = [
            (Point::new(90, 10).into(), vec![1]),
            (
                Polygon::new(
                    LineString::from(vec![(0, 0), (0, 100), (100, 100), (100, 0), (0, 0)]),
                    vec![],
                )
                .into(),
                vec![2, 3, 4, 5],
            ),
            (
                LineString::from(vec![(500, 500), (1, 1)]).into(),
                vec![6, 7],
            ),
            (
                MultiPoint::from(vec![(30, 30), (2, 2), (20, 20)]).into(),
                vec![8, 9, 10],
            ),
        ];
        for (geometry, z) in features {
            let m = z.iter().map(|z| z * 10).collect();
            let mut row = layer.feature(geometry);
            row.z(z).unwrap();
            row.m_value(key, MValue::I32(Some(m))).unwrap();
            row.finish().unwrap();
        }
        let layer = layer.finish();
        let vertex_sorted = EncoderConfig::from_profile(mlt_core::encoder::Profile::Native(
            mlt_core::encoder::Network::Cellular4G,
        ))
        .with_wire_version(mlt_core::encoder::WireVersion::V02)
        .with_vertex_sort(true);
        let stored_order = vertex_sorted.with_vertex_sort(false);
        let encoded = layer.clone().encode(vertex_sorted).unwrap();
        check_round_trip(&layer, &encoded, vertex_sorted, Ok).unwrap();
        insta::assert_snapshot!(
            format!("{:#}", check_round_trip(&layer, &encoded, stored_order, Ok).unwrap_err()),
            @"layer l does not decode back to its input: feature without an id has another geometry"
        );
    }

    fn layer_of_geometries(features: Vec<(Geometry<i32>, Vec<i32>)>) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        layer.set_z_axis(mlt_core::ZStep::new(0).unwrap()).unwrap();
        for (geometry, z) in features {
            let mut row = layer.feature(geometry);
            row.z(z).unwrap();
            row.finish().unwrap();
        }
        layer.finish()
    }

    #[test]
    fn a_rotated_ring_a_reversed_line_and_reordered_points_are_the_same_in_any_order() {
        let stored = layer_of_geometries(vec![
            (
                Polygon::new(
                    LineString::from(vec![(0, 0), (9, 0), (9, 9), (0, 9), (0, 0)]),
                    vec![],
                )
                .into(),
                vec![1, 2, 3, 4],
            ),
            (
                LineString::from(vec![(0, 0), (5, 5), (9, 0)]).into(),
                vec![5, 6, 7],
            ),
            (MultiPoint::from(vec![(1, 1), (2, 2)]).into(), vec![8, 9]),
        ]);
        let reordered = layer_of_geometries(vec![
            (
                Polygon::new(
                    LineString::from(vec![(9, 9), (0, 9), (0, 0), (9, 0), (9, 9)]),
                    vec![],
                )
                .into(),
                vec![3, 4, 1, 2],
            ),
            (
                LineString::from(vec![(9, 0), (5, 5), (0, 0)]).into(),
                vec![7, 6, 5],
            ),
            (MultiPoint::from(vec![(2, 2), (1, 1)]).into(), vec![9, 8]),
        ]);
        same_layer(&stored, &reordered, Vertices::AnyOrder).unwrap();
        insta::assert_snapshot!(
            same_layer(&stored, &reordered, Vertices::Stored).unwrap_err(),
            @"feature without an id has another geometry"
        );
    }

    #[test]
    fn a_rotated_ring_of_100_000_equal_vertices_is_the_same_in_any_order() {
        let ring = |at: usize| {
            let mut coords = vec![Coord { x: 0, y: 0 }; 100_000];
            coords[at] = Coord { x: 9, y: 9 };
            coords.push(coords[0]);
            Polygon::new(LineString(coords), vec![]).into()
        };
        let stored = layer_of_geometries(vec![(ring(0), vec![0; 100_000])]);
        let rotated = layer_of_geometries(vec![(ring(50_000), vec![0; 100_000])]);
        same_layer(&stored, &rotated, Vertices::AnyOrder).unwrap();
    }

    #[test]
    fn a_two_vertex_ring_may_not_swap_its_vertices() {
        let stored = layer_of_geometries(vec![(
            Polygon::new(LineString::from(vec![(0, 0), (9, 9), (0, 0)]), vec![]).into(),
            vec![1, 2],
        )]);
        let swapped = layer_of_geometries(vec![(
            Polygon::new(LineString::from(vec![(9, 9), (0, 0), (9, 9)]), vec![]).into(),
            vec![2, 1],
        )]);
        insta::assert_snapshot!(
            same_layer(&stored, &swapped, Vertices::AnyOrder).unwrap_err(),
            @"feature without an id has another geometry"
        );
    }

    #[test]
    fn a_rotated_ring_must_carry_its_z_along() {
        let stored = layer_of_geometries(vec![(
            Polygon::new(
                LineString::from(vec![(0, 0), (9, 0), (9, 9), (0, 9), (0, 0)]),
                vec![],
            )
            .into(),
            vec![1, 2, 3, 4],
        )]);
        let rotated_without_z = layer_of_geometries(vec![(
            Polygon::new(
                LineString::from(vec![(9, 9), (0, 9), (0, 0), (9, 0), (9, 9)]),
                vec![],
            )
            .into(),
            vec![1, 2, 3, 4],
        )]);
        insta::assert_snapshot!(
            same_layer(&stored, &rotated_without_z, Vertices::AnyOrder).unwrap_err(),
            @"feature without an id has other z: [3, 4, 1, 2], not [1, 2, 3, 4]"
        );
    }

    fn layer_with_nested(kind: mlt_core::NestedKind, value: NestedValue) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        let key = layer.add_nested("n", kind).unwrap();
        let mut row = layer.feature(Point::new(1, 2).into());
        row.id(Some(1));
        row.nested(key, value).unwrap();
        row.finish().unwrap();
        layer.finish()
    }

    #[test]
    fn a_null_list_item_is_not_an_empty_list() {
        use mlt_core::NestedKind;
        let null_item = layer_with_nested(
            NestedKind::list(NestedKind::Leaf(PropKind::U64)),
            NestedValue::list([NestedValue::Leaf(PropValue::U64(None))]),
        );
        let empty_item = layer_with_nested(
            NestedKind::list(NestedKind::list(NestedKind::Leaf(PropKind::U64))),
            NestedValue::list([NestedValue::list([])]),
        );
        insta::assert_snapshot!(
            same_layer(&null_item, &empty_item, Vertices::Stored).unwrap_err(),
            @"feature 1 has other values: Nested n is List([Some(List([]))]), not List([None])"
        );
    }

    #[test]
    fn a_changed_value_names_the_feature_and_column() {
        let a = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        let b = layer_of(PropKind::U64, PropValue::U64(Some(8)), 1);
        insta::assert_snapshot!(
            same_layer(&a, &b, Vertices::Stored).unwrap_err(),
            @"feature 1 has other values: Property v is Int(8), not Int(7)"
        );
    }

    #[test]
    fn a_string_is_not_a_number() {
        let a = layer_of(PropKind::Str, PropValue::Str(Some("7".into())), 1);
        let b = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        insta::assert_snapshot!(
            same_layer(&a, &b, Vertices::Stored).unwrap_err(),
            @r#"feature 1 has other values: Property v is Int(7), not Str("7")"#
        );
    }

    #[test]
    fn a_changed_id_is_a_missing_feature() {
        let a = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        let b = layer_of(PropKind::U64, PropValue::U64(Some(7)), 2);
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @"feature 1 is missing");
    }

    #[test]
    fn a_renamed_layer_is_reported() {
        let a = TileLayer::builder("a", 4096).unwrap().finish();
        let b = TileLayer::builder("b", 4096).unwrap().finish();
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @r#"named "b", not "a""#);
    }

    #[test]
    fn a_changed_extent_is_reported() {
        let a = TileLayer::builder("l", 4096).unwrap().finish();
        let b = TileLayer::builder("l", 512).unwrap().finish();
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @"extent 512, not 4096");
    }

    #[test]
    fn a_dropped_feature_is_reported() {
        let a = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        let b = TileLayer::builder("l", 4096).unwrap().finish();
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @"0 features, not 1");
    }

    #[test]
    fn a_value_turned_null_is_reported_as_nothing() {
        let a = layer_of(PropKind::U64, PropValue::U64(Some(7)), 1);
        let b = layer_of(PropKind::U64, PropValue::U64(None), 1);
        insta::assert_snapshot!(
            same_layer(&a, &b, Vertices::Stored).unwrap_err(),
            @"feature 1 has other values: Property v is nothing, not Int(7)"
        );
    }

    fn layer_of_points(points: &[(Option<u64>, i32)]) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        for &(id, x) in points {
            let mut row = layer.feature(Point::new(x, 0).into());
            row.id(id);
            row.finish().unwrap();
        }
        layer.finish()
    }

    #[test]
    fn features_in_another_order_are_the_same_layer() {
        let a = layer_of_points(&[(Some(1), 10), (Some(2), 20)]);
        let b = layer_of_points(&[(Some(2), 20), (Some(1), 10)]);
        same_layer(&a, &b, Vertices::Stored).unwrap();
    }

    #[test]
    fn features_without_ids_in_another_order_are_the_same_layer() {
        let a = layer_of_points(&[(None, 10), (None, 20)]);
        let b = layer_of_points(&[(None, 20), (None, 10)]);
        same_layer(&a, &b, Vertices::Stored).unwrap();
    }

    #[test]
    fn a_nan_equals_itself() {
        let a = layer_of(PropKind::F64, PropValue::F64(Some(f64::NAN)), 1);
        let b = layer_of(PropKind::F64, PropValue::F64(Some(f64::NAN)), 1);
        same_layer(&a, &b, Vertices::Stored).unwrap();
    }

    #[test]
    fn a_float_column_may_change_its_width() {
        let wide = layer_of(PropKind::F64, PropValue::F64(Some(1.5)), 1);
        let narrow = layer_of(PropKind::F32, PropValue::F32(Some(1.5)), 1);
        same_layer(&wide, &narrow, Vertices::Stored).unwrap();
    }

    #[test]
    fn a_change_in_every_property_kind_is_reported() {
        let changes = [
            (
                PropKind::Bool,
                PropValue::Bool(Some(true)),
                PropValue::Bool(Some(false)),
            ),
            (
                PropKind::I8,
                PropValue::I8(Some(-1)),
                PropValue::I8(Some(1)),
            ),
            (PropKind::U8, PropValue::U8(Some(1)), PropValue::U8(Some(2))),
            (
                PropKind::I32,
                PropValue::I32(Some(-1)),
                PropValue::I32(Some(1)),
            ),
            (
                PropKind::U32,
                PropValue::U32(Some(1)),
                PropValue::U32(Some(2)),
            ),
            (
                PropKind::I64,
                PropValue::I64(Some(-1)),
                PropValue::I64(Some(1)),
            ),
            (
                PropKind::U64,
                PropValue::U64(Some(1)),
                PropValue::U64(Some(2)),
            ),
            (
                PropKind::F32,
                PropValue::F32(Some(0.5)),
                PropValue::F32(Some(0.25)),
            ),
            (
                PropKind::F64,
                PropValue::F64(Some(0.5)),
                PropValue::F64(Some(0.25)),
            ),
            (
                PropKind::Str,
                PropValue::Str(Some("a".into())),
                PropValue::Str(Some("b".into())),
            ),
        ];
        let mut report = String::new();
        for (kind, a, b) in changes {
            let err = same_layer(
                &layer_of(kind, a, 1),
                &layer_of(kind, b, 1),
                Vertices::Stored,
            )
            .unwrap_err();
            writeln!(report, "{}: {err}", <&str>::from(kind)).unwrap();
        }
        insta::assert_snapshot!(report, @r#"
        bool: feature 1 has other values: Property v is Bool(false), not Bool(true)
        i8: feature 1 has other values: Property v is Int(1), not Int(-1)
        u8: feature 1 has other values: Property v is Int(2), not Int(1)
        i32: feature 1 has other values: Property v is Int(1), not Int(-1)
        u32: feature 1 has other values: Property v is Int(2), not Int(1)
        i64: feature 1 has other values: Property v is Int(1), not Int(-1)
        u64: feature 1 has other values: Property v is Int(2), not Int(1)
        f32: feature 1 has other values: Property v is Float(0.25), not Float(0.5)
        f64: feature 1 has other values: Property v is Float(0.25), not Float(0.5)
        str: feature 1 has other values: Property v is Str("b"), not Str("a")
        "#);
    }

    fn layer_with_m_value(kind: PropKind, value: MValue) -> TileLayer {
        let mut layer = TileLayer::builder("l", 4096).unwrap();
        let key = layer.add_m_value("m", kind).unwrap();
        let mut row = layer.feature(Point::new(1, 2).into());
        row.id(Some(1));
        row.m_value(key, value).unwrap();
        row.finish().unwrap();
        layer.finish()
    }

    #[test]
    fn a_change_in_every_m_value_kind_is_reported() {
        let changes = [
            (
                PropKind::Bool,
                MValue::Bool(Some(vec![true])),
                MValue::Bool(Some(vec![false])),
            ),
            (
                PropKind::I8,
                MValue::I8(Some(vec![-1])),
                MValue::I8(Some(vec![1])),
            ),
            (
                PropKind::U8,
                MValue::U8(Some(vec![1])),
                MValue::U8(Some(vec![2])),
            ),
            (
                PropKind::I32,
                MValue::I32(Some(vec![-1])),
                MValue::I32(Some(vec![1])),
            ),
            (
                PropKind::U32,
                MValue::U32(Some(vec![1])),
                MValue::U32(Some(vec![2])),
            ),
            (
                PropKind::I64,
                MValue::I64(Some(vec![-1])),
                MValue::I64(Some(vec![1])),
            ),
            (
                PropKind::U64,
                MValue::U64(Some(vec![1])),
                MValue::U64(Some(vec![2])),
            ),
            (
                PropKind::F32,
                MValue::F32(Some(vec![0.5])),
                MValue::F32(Some(vec![0.25])),
            ),
            (
                PropKind::F64,
                MValue::F64(Some(vec![0.5])),
                MValue::F64(Some(vec![0.25])),
            ),
            (
                PropKind::Str,
                MValue::Str(Some(vec!["a".into()])),
                MValue::Str(Some(vec!["b".into()])),
            ),
        ];
        let mut report = String::new();
        for (kind, a, b) in changes {
            let err = same_layer(
                &layer_with_m_value(kind, a),
                &layer_with_m_value(kind, b),
                Vertices::Stored,
            )
            .unwrap_err();
            writeln!(report, "{}: {err}", <&str>::from(kind)).unwrap();
        }
        insta::assert_snapshot!(report, @r#"
        bool: feature 1 has other values: MValue m is List([Some(Bool(false))]), not List([Some(Bool(true))])
        i8: feature 1 has other values: MValue m is List([Some(Int(1))]), not List([Some(Int(-1))])
        u8: feature 1 has other values: MValue m is List([Some(Int(2))]), not List([Some(Int(1))])
        i32: feature 1 has other values: MValue m is List([Some(Int(1))]), not List([Some(Int(-1))])
        u32: feature 1 has other values: MValue m is List([Some(Int(2))]), not List([Some(Int(1))])
        i64: feature 1 has other values: MValue m is List([Some(Int(1))]), not List([Some(Int(-1))])
        u64: feature 1 has other values: MValue m is List([Some(Int(2))]), not List([Some(Int(1))])
        f32: feature 1 has other values: MValue m is List([Some(Float(0.25))]), not List([Some(Float(0.5))])
        f64: feature 1 has other values: MValue m is List([Some(Float(0.25))]), not List([Some(Float(0.5))])
        str: feature 1 has other values: MValue m is List([Some(Str("b"))]), not List([Some(Str("a"))])
        "#);
    }

    #[test]
    fn a_changed_map_entry_is_reported() {
        use mlt_core::NestedKind;
        let kind = NestedKind::map([("k", NestedKind::Leaf(PropKind::U64))]);
        let a = layer_with_nested(
            kind.clone(),
            NestedValue::map([("k", NestedValue::Leaf(PropValue::U64(Some(1))))]),
        );
        let b = layer_with_nested(
            kind,
            NestedValue::map([("k", NestedValue::Leaf(PropValue::U64(Some(2))))]),
        );
        insta::assert_snapshot!(same_layer(&a, &b, Vertices::Stored).unwrap_err(), @r#"feature 1 has other values: Nested n is Map({"k": Int(2)}), not Map({"k": Int(1)})"#);
    }

    #[test]
    fn a_null_map_entry_is_a_missing_one() {
        use mlt_core::NestedKind;
        let kind = NestedKind::map([("k", NestedKind::Leaf(PropKind::U64))]);
        let null_entry = layer_with_nested(
            kind.clone(),
            NestedValue::map([("k", NestedValue::Leaf(PropValue::U64(None)))]),
        );
        let no_entry = layer_with_nested(kind, NestedValue::map::<String>([]));
        same_layer(&null_entry, &no_entry, Vertices::Stored).unwrap();
    }
}
