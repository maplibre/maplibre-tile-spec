use arbitrary::{Arbitrary, Unstructured};
use mlt::convert::fields::FieldConfig;
use mlt::convert::verify::check_round_trip;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{
    MValue, MltError, NestedKind, NestedValue, PropKind, PropValue, TileFeature, TileLayer, ZStep,
};

use crate::fields_file::{SPLITS, fields_file, form_table, token};
use crate::z::geometry;

/// The names the generated layers and configs draw from, so that most configured names hit a column.
const NAMES: [&str; 6] = ["a", "b", "c", "m", "n", "x"];

/// A layer of packed string properties and a `--fields` file naming some of them, re-encoded as v2.
///
/// Whenever the config loads and parses the layer, its encoding must verify against the layer,
/// and the verifier must refuse the encoding once one value of the restored layer changes.
#[derive(Debug)]
pub struct FieldConfigInput {
    pub layer: TileLayer,
    pub config: String,
    pub encoder: EncoderConfig,
    pub mutation: Mutation,
}

/// One change to a decoded layer that the verifier must notice.
#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum Mutation {
    Id(u8),
    Geometry(u8),
    Z(u8),
    DropZStep,
    Property(u8, u8),
    DropFeature(u8),
}

impl Arbitrary<'_> for FieldConfigInput {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let layer = layer(u)?;
        let config = config(u, layer.name())?;
        // `mlt convert` refuses `--verify` with `--triangles-only`, which drops the outlines it compares.
        let encoder = EncoderConfig::arbitrary(u)?
            .with_wire_version(WireVersion::V02)
            .with_triangles_only(false);
        let mutation = u.arbitrary()?;
        Ok(Self {
            layer,
            config,
            encoder,
            mutation,
        })
    }
}

fn layer(u: &mut Unstructured<'_>) -> arbitrary::Result<TileLayer> {
    let mut out = TileLayer::builder("l", 4096).expect("a valid layer");
    let step = if u.arbitrary()? {
        let step = ZStep::new(u.int_in_range(ZStep::MIN_EXPONENT..=ZStep::MAX_EXPONENT)?)
            .expect("an exponent in range");
        out.set_z_step(step).expect("an empty layer");
        Some(step)
    } else {
        None
    };
    let mut properties = Vec::new();
    for name in &NAMES[..3] {
        if u.arbitrary()? {
            let packed = u.ratio(3, 4)?;
            let kind = if packed { PropKind::Str } else { PropKind::I64 };
            properties.push((out.add_property(*name, kind).expect("a new name"), packed));
        }
    }
    let m_value = if u.arbitrary()? {
        Some(out.add_m_value("m", PropKind::I32).expect("a new name"))
    } else {
        None
    };
    let nested = if u.arbitrary()? {
        let kind = NestedKind::list(NestedKind::Leaf(PropKind::U64));
        Some(out.add_nested("n", kind).expect("a new name"))
    } else {
        None
    };

    for _ in 0..u.int_in_range(0..=6u8)? {
        let geometry = geometry(u)?;
        let vertices = TileFeature::new(geometry.clone()).vertex_count();
        let mut row = out.feature(geometry);
        row.id(u.arbitrary::<Option<u8>>()?.map(u64::from));
        if step.is_some() {
            row.z((0..vertices)
                .map(|_| u.arbitrary())
                .collect::<arbitrary::Result<_>>()?)
                .expect("one z per vertex");
        }
        for &(key, is_packed) in &properties {
            let value = if is_packed {
                PropValue::Str(if u.arbitrary()? {
                    Some(packed(u)?)
                } else {
                    None
                })
            } else {
                PropValue::I64(u.arbitrary()?)
            };
            row.property(key, value).expect("a declared kind");
        }
        if let Some(key) = m_value {
            let values = if u.arbitrary()? {
                Some(
                    (0..vertices)
                        .map(|_| u.arbitrary())
                        .collect::<arbitrary::Result<_>>()?,
                )
            } else {
                None
            };
            row.m_value(key, MValue::I32(values))
                .expect("one value per vertex");
        }
        if let Some(key) = nested {
            let items: Option<Vec<u64>> = u.arbitrary()?;
            let value = items.map_or(NestedValue::List(None), |items| {
                NestedValue::list(
                    items
                        .into_iter()
                        .map(|v| NestedValue::Leaf(PropValue::U64(Some(v)))),
                )
            });
            row.nested(key, value).expect("a declared shape");
        }
        row.finish().expect("a complete row");
    }
    Ok(out.finish())
}

/// A string written the way packed fields are: numbers and short words between separators.
fn packed(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    let tokens = (0..u.int_in_range(0..=5u8)?)
        .map(|_| token(u))
        .collect::<arbitrary::Result<Vec<_>>>()?;
    let separator = *u.choose(&[",", ";", " ", "|", "é", "+", "-", "1", ""])?;
    if separator.is_empty() {
        let mut s = String::new();
        for (i, token) in tokens.iter().enumerate() {
            if i > 0 && !token.starts_with(['+', '-']) && u.arbitrary()? {
                s.push('+');
            }
            s.push_str(token);
        }
        return Ok(s);
    }
    Ok(tokens.join(separator))
}

/// A `--fields` file with a form for some of [`NAMES`], mostly under `layer`.
fn config(u: &mut Unstructured<'_>, layer: &str) -> arbitrary::Result<String> {
    let mut forms = toml::Table::new();
    for name in NAMES {
        if u.arbitrary()? {
            forms.insert(name.to_owned(), form(u)?.into());
        }
    }
    let layer = if u.ratio(7, 8)? { layer } else { "other" };
    Ok(fields_file(layer, forms))
}

fn form(u: &mut Unstructured<'_>) -> arbitrary::Result<toml::Table> {
    let split = if u.ratio(1, 8)? {
        *u.choose(&["", ",,"])?
    } else {
        *u.choose(&SPLITS)?
    };
    let kind = *u.choose(&["i32", "u32", "i64", "u64", "str", "str", "f32"])?;
    let running_sum = if u.arbitrary()? {
        Some(u.arbitrary()?)
    } else {
        None
    };
    let into = if u.arbitrary()? {
        Some(*u.choose(&["list", "m-value", "m-value", "nested"])?)
    } else {
        None
    };
    Ok(form_table(split, kind, running_sum, into))
}

impl FieldConfigInput {
    pub fn fuzz(self) {
        let Ok(fields) = self.config.parse::<FieldConfig>() else {
            return;
        };
        let Ok(parsed) = fields.apply(self.layer.clone()) else {
            return;
        };
        let encoded = match parsed.encode(self.encoder) {
            Ok(encoded) => encoded,
            Err(MltError::NotImplemented(_) | MltError::MValuesNeedVertexCounts(_)) => return,
            Err(e) => panic!("a parsed layer should encode: {e}"),
        };
        check_round_trip(&self.layer, &encoded, |decoded| fields.restore(decoded))
            .expect("a parsed layer should verify");

        let mut mutated = false;
        let result = check_round_trip(&self.layer, &encoded, |decoded| {
            let restored = fields.restore(decoded)?;
            Ok(match mutate(&restored, self.mutation) {
                Some(changed) => {
                    mutated = true;
                    changed
                }
                None => restored,
            })
        });
        if mutated {
            assert!(result.is_err(), "the verifier missed {:?}", self.mutation);
        }
    }
}

/// `layer` with `mutation` applied, or `None` if `layer` has nothing it applies to.
fn mutate(layer: &TileLayer, mutation: Mutation) -> Option<TileLayer> {
    let features = layer.features();
    let pick = |i: u8| (!features.is_empty()).then(|| usize::from(i) % features.len());
    let target = match mutation {
        Mutation::DropZStep => {
            layer.z_step()?;
            None
        }
        Mutation::Id(i)
        | Mutation::Geometry(i)
        | Mutation::Z(i)
        | Mutation::Property(i, _)
        | Mutation::DropFeature(i) => Some(pick(i)?),
    };
    let drop_z = matches!(mutation, Mutation::DropZStep);
    let property = if let Mutation::Property(_, p) = mutation {
        let columns = layer.property_names().len();
        Some(usize::from(p) % (columns > 0).then_some(columns)?)
    } else {
        None
    };
    if matches!(mutation, Mutation::Z(_)) && features[target?].z().is_empty() {
        return None;
    }

    let mut out = if drop_z {
        TileLayer::builder(layer.name(), layer.extent().get()).expect("a valid layer")
    } else {
        layer.builder_like()
    };
    let property_keys = out.add_properties_like(layer).expect("unique names");
    let m_keys = out.add_m_values_like(layer).expect("unique names");
    let nested_keys = out.add_nested_like(layer).expect("unique names");

    for (index, feature) in features.iter().enumerate() {
        let here = target == Some(index);
        if here && matches!(mutation, Mutation::DropFeature(_)) {
            continue;
        }
        let geometry = if here && matches!(mutation, Mutation::Geometry(_)) {
            shifted(feature.geometry())
        } else {
            feature.geometry().clone()
        };
        let mut row = out.feature(geometry);
        row.id(match (here, mutation) {
            (true, Mutation::Id(_)) => feature.id().map_or(Some(0), |id| id.checked_add(1)),
            _ => feature.id(),
        });
        if !drop_z && !feature.z().is_empty() {
            let mut z = feature.z().to_vec();
            if here && matches!(mutation, Mutation::Z(_)) {
                z[0] = z[0].wrapping_add(1);
            }
            row.z(z).expect("one z per vertex");
        }
        for (i, (key, value)) in property_keys.iter().zip(feature.properties()).enumerate() {
            let value = if here && property == Some(i) {
                changed(value)
            } else {
                value.clone()
            };
            row.property(*key, value).expect("a declared kind");
        }
        for (key, value) in m_keys.iter().zip(feature.m_values()) {
            row.m_value(*key, value.clone()).expect("a declared kind");
        }
        for (key, value) in nested_keys.iter().zip(feature.nested()) {
            row.nested(*key, value.clone()).expect("a declared shape");
        }
        row.finish().expect("a complete row");
    }
    Some(out.finish())
}

/// `value` with another value, or a value where it was null.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the generated layers hold only strings and integers"
)]
fn changed(value: &PropValue) -> PropValue {
    match value {
        PropValue::Str(s) => PropValue::Str(Some(
            s.as_ref().map_or_else(String::new, |s| s.clone() + "x"),
        )),
        PropValue::I64(v) => PropValue::I64(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        PropValue::U64(v) => PropValue::U64(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        PropValue::I32(v) => PropValue::I32(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        PropValue::U32(v) => PropValue::U32(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        PropValue::I8(v) => PropValue::I8(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        PropValue::U8(v) => PropValue::U8(Some(v.map_or(0, |v| v.wrapping_add(1)))),
        other => panic!("{other:?} is not generated"),
    }
}

/// `geometry` moved one unit along x.
fn shifted(geometry: &Geometry<i32>) -> Geometry<i32> {
    let coord = |c: &Coord<i32>| Coord {
        x: c.x.wrapping_add(1),
        y: c.y,
    };
    let line = |l: &LineString<i32>| LineString(l.0.iter().map(coord).collect());
    let polygon = |p: &Polygon<i32>| {
        Polygon::new(line(p.exterior()), p.interiors().iter().map(line).collect())
    };
    match geometry {
        Geometry::Point(p) => Geometry::Point(Point(coord(&p.0))),
        Geometry::LineString(l) => Geometry::LineString(line(l)),
        Geometry::Polygon(p) => Geometry::Polygon(polygon(p)),
        Geometry::MultiPoint(mp) => {
            Geometry::MultiPoint(MultiPoint(mp.iter().map(|p| Point(coord(&p.0))).collect()))
        }
        Geometry::MultiLineString(ml) => {
            Geometry::MultiLineString(MultiLineString(ml.iter().map(line).collect()))
        }
        Geometry::MultiPolygon(mp) => {
            Geometry::MultiPolygon(MultiPolygon(mp.iter().map(polygon).collect()))
        }
        Geometry::Line(_)
        | Geometry::GeometryCollection(_)
        | Geometry::Rect(_)
        | Geometry::Triangle(_) => unreachable!("the decoder produces none"),
    }
}
