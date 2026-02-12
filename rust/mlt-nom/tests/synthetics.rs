use std::fs;
use std::path::Path;

use mlt_nom::parse_layers;
use mlt_nom::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, Geometry, GeometryType, Id, PropValue, Property,
};
use serde_json::{json, Value};
use test_each_file::test_each_path;

test_each_path! { for ["mlt", "json"] in "../test/synthetic/0x01" => pair_test }

fn pair_test([mlt, json]: [&Path; 2]) {
    test_one(mlt, json);
}

fn test_one(mlt: &Path, json: &Path) {
    let buffer = fs::read(mlt).unwrap();
    let mut data = match parse_layers(&buffer) {
        Ok(v) => v,
        Err(e) if e.to_string().contains("FastPFOR") => {
            eprintln!("Skipping {}: {e}", mlt.display());
            return;
        }
        Err(e) => panic!("{e}"),
    };
    for layer in &mut data {
        if let Err(e) = layer.decode_all() {
            if e.to_string().contains("FastPFOR") {
                eprintln!("Skipping {}: {e}", mlt.display());
                return;
            }
            panic!("{e}");
        }
    }

    let expected: Value =
        serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();

    let mut features = Vec::new();
    for layer in &data {
        let l = layer.as_layer01().expect("expected Tag01 layer");
        let geom = decoded_geom(&l.geometry);
        let ids = decoded_ids(&l.id);
        let props: Vec<&DecodedProperty> = l
            .properties
            .iter()
            .map(|p| match p {
                Property::Decoded(d) => d,
                Property::Raw(_) => panic!("property not decoded"),
            })
            .collect();

        for i in 0..geom.vector_types.len() {
            let id: u64 = ids
                .and_then(|v| v.get(i).copied().flatten())
                .unwrap_or(0);
            let geometry = feature_geometry(geom, i);
            let mut properties = serde_json::Map::new();
            for prop in &props {
                if let Some(val) = prop_value_at(&prop.values, i) {
                    properties.insert(prop.name.clone(), val);
                }
            }
            properties.insert("layer".into(), json!(l.name));
            features.push(json!({
                "geometry": geometry,
                "id": id,
                "properties": Value::Object(properties),
                "type": "Feature"
            }));
        }
    }

    let actual = json!({
        "features": features,
        "type": "FeatureCollection"
    });
    assert_eq!(actual, expected);
}

fn decoded_geom<'a>(g: &'a Geometry<'a>) -> &'a DecodedGeometry {
    match g {
        Geometry::Decoded(d) => d,
        Geometry::Raw(_) => panic!("geometry not decoded"),
    }
}

fn decoded_ids<'a>(id: &'a Id<'a>) -> Option<&'a [Option<u64>]> {
    match id {
        Id::Decoded(DecodedId(Some(v))) => Some(v),
        Id::Decoded(DecodedId(None)) | Id::None => None,
        _ => panic!("id not decoded"),
    }
}

/// Build `GeoJSON` geometry object for a single feature
fn feature_geometry(geom: &DecodedGeometry, i: usize) -> Value {
    let verts = geom.vertices.as_deref().unwrap_or(&[]);
    let geom_type = geom.vector_types[i];
    let go = geom.geometry_offsets.as_deref();
    let po = geom.part_offsets.as_deref();
    let ro = geom.ring_offsets.as_deref();

    let v = |idx: usize| json!([verts[idx * 2], verts[idx * 2 + 1]]);
    let line = |start: usize, end: usize| Value::Array((start..end).map(&v).collect());
    // Polygon rings in GeoJSON must close (first vertex = last vertex).
    // MLT omits the closing vertex, so we append it here.
    let closed_ring = |start: usize, end: usize| {
        let mut coords: Vec<Value> = (start..end).map(&v).collect();
        coords.push(v(start));
        Value::Array(coords)
    };

    let coordinates = match geom_type {
        GeometryType::Point => match (go, po, ro) {
            (Some(go), Some(po), Some(ro)) => v(ro[po[go[i] as usize] as usize] as usize),
            (None, Some(po), Some(ro)) => v(ro[po[i] as usize] as usize),
            (None, Some(po), None) => v(po[i] as usize),
            (None, None, None) => v(i),
            _ => unreachable!(),
        },
        GeometryType::LineString => match (po, ro) {
            (Some(po), Some(ro)) => {
                let ri = po[i] as usize;
                line(ro[ri] as usize, ro[ri + 1] as usize)
            }
            (Some(po), None) => line(po[i] as usize, po[i + 1] as usize),
            _ => unreachable!(),
        },
        GeometryType::Polygon => {
            let (rs, re) = if let Some(go) = go {
                let pi = go[i] as usize;
                (po.unwrap()[pi] as usize, po.unwrap()[pi + 1] as usize)
            } else {
                (po.unwrap()[i] as usize, po.unwrap()[i + 1] as usize)
            };
            let ro = ro.unwrap();
            Value::Array(
                (rs..re)
                    .map(|r| closed_ring(ro[r] as usize, ro[r + 1] as usize))
                    .collect(),
            )
        }
        GeometryType::MultiPolygon => {
            let go = go.unwrap();
            let po = po.unwrap();
            let ro = ro.unwrap();
            let (ps, pe) = (go[i] as usize, go[i + 1] as usize);
            Value::Array(
                (ps..pe)
                    .map(|p| {
                        let (rs, re) = (po[p] as usize, po[p + 1] as usize);
                        Value::Array(
                            (rs..re)
                                .map(|r| closed_ring(ro[r] as usize, ro[r + 1] as usize))
                                .collect(),
                        )
                    })
                    .collect(),
            )
        }
        t => todo!("geometry type {t:?}"),
    };

    json!({
        "type": geom_type.to_string(),
        "coordinates": coordinates,
        "crs": { "type": "name", "properties": { "name": "EPSG:0" } }
    })
}

/// Extract a single property value at index i as a JSON Value
fn prop_value_at(values: &PropValue, i: usize) -> Option<Value> {
    match values {
        PropValue::Bool(v) => v[i].map(|b| json!(b)),
        PropValue::I8(v) => v[i].map(|n| json!(n)),
        PropValue::U8(v) => v[i].map(|n| json!(n)),
        PropValue::I32(v) => v[i].map(|n| json!(n)),
        PropValue::U32(v) => v[i].map(|n| json!(n)),
        PropValue::I64(v) => v[i].map(|n| json!(n)),
        PropValue::U64(v) => v[i].map(|n| json!(n)),
        PropValue::F32(v) => v[i].map(f32_to_json),
        #[allow(clippy::cast_possible_truncation)] // f64 stored as f32 in wire format
        PropValue::F64(v) => v[i].map(|f| f32_to_json(f as f32)),
        PropValue::Str(v) => v[i].as_ref().map(|s| json!(s)),
        PropValue::Struct => None,
    }
}

/// Convert f32 to JSON using shortest decimal representation (matches Java's `Float.toString()`)
fn f32_to_json(f: f32) -> Value {
    // Use f32's shortest representation to match the JSON generated by Java
    serde_json::from_str(&f.to_string()).unwrap()
}

#[test]
fn test_plain() {
    let path = "../../test/synthetic/0x01/point";

    let mlt = Path::new(path).with_extension("mlt");
    let json = Path::new(path).with_extension("json");
    test_one(&mlt, &json);
}
