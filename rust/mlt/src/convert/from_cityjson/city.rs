//! The part of a `CityJSON` file a mesh layer needs: each city object's surfaces at one `LoD`, in lon/lat/metres.

use std::collections::BTreeMap;

use anyhow::{Context as _, Result as AnyResult, bail};
use serde::Deserialize;
use serde_json::{Map, Value};

// CityJSON allows extension members ("+name") and more, so unknown fields are ignored, as for GeoJSON.
#[derive(Deserialize)]
struct File {
    #[serde(rename = "type")]
    kind: String,
    transform: Option<Transform>,
    metadata: Option<Metadata>,
    #[serde(rename = "CityObjects")]
    objects: BTreeMap<String, Object>,
    vertices: Vec<[i64; 3]>,
}

#[derive(Deserialize)]
struct Transform {
    scale: [f64; 3],
    translate: [f64; 3],
}

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename = "referenceSystem")]
    reference_system: Option<String>,
}

#[derive(Deserialize)]
struct Object {
    attributes: Option<Map<String, Value>>,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    geometry: Vec<Geometry>,
}

#[derive(Deserialize)]
struct Geometry {
    #[serde(rename = "type")]
    kind: String,
    lod: Option<Value>,
    #[serde(default)]
    boundaries: Value,
    semantics: Option<Semantics>,
}

#[derive(Deserialize)]
struct Semantics {
    surfaces: Vec<Map<String, Value>>,
    values: Value,
}

/// One polygon of a city object: its exterior ring, then its holes, each `[lon, lat, metres]`.
pub(super) struct Face {
    pub rings: Vec<Vec<[f64; 3]>>,
    /// The semantic surface type, such as `RoofSurface`, if the face has one.
    pub surface: Option<String>,
}

/// A city object with surfaces at the chosen `LoD`.
pub(super) struct CityObject {
    pub id: String,
    /// Its own attributes over those of its parents.
    pub attributes: Map<String, Value>,
    pub faces: Vec<Face>,
}

/// The objects of a parsed file, and how many were left out for having no surfaces at its `LoD`.
pub(super) struct City {
    pub lod: String,
    pub objects: Vec<CityObject>,
    pub skipped: usize,
}

/// How many levels of nesting a surface type's boundaries have above the surfaces themselves.
fn nesting(kind: &str) -> Option<usize> {
    match kind {
        "MultiSurface" | "CompositeSurface" => Some(0),
        "Solid" => Some(1),
        "MultiSolid" | "CompositeSolid" => Some(2),
        _ => None,
    }
}

fn lod_of(geometry: &Geometry) -> Option<String> {
    match geometry.lod.as_ref()? {
        Value::String(lod) => Some(lod.clone()),
        Value::Number(lod) => Some(lod.to_string()),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// Parse `text`, keeping each object's surface geometries at `lod`, or at the file's highest `LoD`.
pub(super) fn parse(text: &str, lod: Option<&str>) -> AnyResult<City> {
    let file: File = serde_json::from_str(text).context("parsing CityJSON")?;
    if file.kind != "CityJSON" {
        bail!("the file is of type {:?}, not CityJSON", file.kind);
    }
    check_crs(file.metadata.as_ref())?;
    let Some(transform) = file.transform else {
        bail!("the file has no transform, which CityJSON 1.1 and later require");
    };
    let lod = match lod {
        Some(lod) => lod.to_owned(),
        None => file
            .objects
            .values()
            .flat_map(|o| &o.geometry)
            .filter(|g| nesting(&g.kind).is_some())
            .filter_map(lod_of)
            .max_by(|a, b| {
                let key = |l: &str| l.parse::<f64>().unwrap_or(f64::NEG_INFINITY);
                key(a).total_cmp(&key(b))
            })
            .context("the file has no surface geometry with a LoD")?,
    };
    let position = |index: u32| -> AnyResult<[f64; 3]> {
        let v = usize::try_from(index)
            .ok()
            .and_then(|i| file.vertices.get(i))
            .with_context(|| format!("vertex {index} is out of range"))?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "CityJSON stores vertices as integers well below 2^53"
        )]
        Ok(std::array::from_fn(|axis| {
            (v[axis] as f64).mul_add(transform.scale[axis], transform.translate[axis])
        }))
    };

    let mut objects = Vec::new();
    let mut skipped = 0;
    for (id, object) in &file.objects {
        let mut faces = Vec::new();
        for geometry in &object.geometry {
            let Some(depth) = nesting(&geometry.kind) else {
                continue;
            };
            if lod_of(geometry).as_deref() != Some(lod.as_str()) {
                continue;
            }
            let values = geometry.semantics.as_ref().map(|s| &s.values);
            let mut surfaces = Vec::new();
            flatten(&geometry.boundaries, values, depth, &mut surfaces)
                .with_context(|| format!("reading the {} of object {id}", geometry.kind))?;
            for (rings, value) in surfaces {
                let surface = match (value, &geometry.semantics) {
                    (Some(value), Some(semantics)) => {
                        let surface = usize::try_from(value)
                            .ok()
                            .and_then(|i| semantics.surfaces.get(i))
                            .with_context(|| {
                                format!(
                                    "object {id} names semantic surface {value}, which it lacks"
                                )
                            })?;
                        surface
                            .get("type")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    }
                    _ => None,
                };
                let rings = rings
                    .iter()
                    .map(|ring| ring.iter().map(|&i| position(i)).collect())
                    .collect::<AnyResult<_>>()
                    .with_context(|| format!("reading object {id}"))?;
                faces.push(Face { rings, surface });
            }
        }
        if faces.is_empty() {
            skipped += 1;
            continue;
        }
        objects.push(CityObject {
            id: id.clone(),
            attributes: inherited_attributes(&file.objects, id),
            faces,
        });
    }
    Ok(City {
        lod,
        objects,
        skipped,
    })
}

/// Accept only EPSG:4326, whose z is taken as metres above sea level, as MLT's absolute heights are.
fn check_crs(metadata: Option<&Metadata>) -> AnyResult<()> {
    let crs = metadata.and_then(|m| m.reference_system.as_deref());
    let is_wgs84 =
        crs.is_some_and(|crs| crs.ends_with("/EPSG/0/4326") || crs.ends_with("EPSG::4326"));
    if !is_wgs84 {
        let named = crs.map_or_else(
            || "no metadata.referenceSystem".to_owned(),
            |crs| format!("reference system {crs}"),
        );
        bail!(
            "the file has {named}, but convert reads CityJSON in EPSG:4326 \
             (lon, lat, metres above sea level); reproject it first, e.g. with `cjio <in> reproject 4326 save <out>`"
        );
    }
    Ok(())
}

/// Walk `boundaries` down `depth` levels to its surfaces, pairing each with its semantic value.
fn flatten(
    boundaries: &Value,
    values: Option<&Value>,
    depth: usize,
    out: &mut Vec<(Vec<Vec<u32>>, Option<u64>)>,
) -> AnyResult<()> {
    let Value::Array(items) = boundaries else {
        bail!("boundaries are not nested arrays");
    };
    let values = match values {
        Some(Value::Array(values)) => Some(values),
        Some(Value::Null) | None => None,
        Some(_) => bail!("semantic values are not nested arrays"),
    };
    for (index, item) in items.iter().enumerate() {
        let value = values.and_then(|v| v.get(index));
        if depth == 0 {
            let rings: Vec<Vec<u32>> = serde_json::from_value(item.clone())
                .context("a surface is not rings of vertex indices")?;
            out.push((rings, value.and_then(Value::as_u64)));
        } else {
            flatten(item, value, depth - 1, out)?;
        }
    }
    Ok(())
}

/// The attributes of `id`, over those of its parents, over those of theirs.
fn inherited_attributes(objects: &BTreeMap<String, Object>, id: &str) -> Map<String, Value> {
    let mut chain = vec![id];
    let mut at = 0;
    while let Some(&current) = chain.get(at) {
        for parent in objects
            .get(current)
            .map_or(&[][..], |o| o.parents.as_slice())
        {
            if !chain.contains(&parent.as_str()) {
                chain.push(parent);
            }
        }
        at += 1;
    }
    let mut attributes = Map::new();
    for current in chain.iter().rev() {
        if let Some(own) = objects.get(*current).and_then(|o| o.attributes.as_ref()) {
            attributes.extend(own.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }
    attributes
}
