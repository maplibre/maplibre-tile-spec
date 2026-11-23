//! A `CityJSON` file read into the city model: each building, bridge and tunnel with its parts, installations and semantic surfaces.

use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::{Context as _, Result as AnyResult, bail};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::model::{
    Lod, ObjectKind, OwnedPolygon, Part, Rings, Semantic, SourceDocument, SourceObject,
};

// CityJSON allows extension members ("+name") and more, so unknown fields are ignored, as for GeoJSON.
#[derive(Deserialize)]
struct File {
    #[serde(rename = "type")]
    kind: String,
    transform: Option<Transform>,
    metadata: Option<Metadata>,
    #[serde(rename = "CityObjects", deserialize_with = "in_file_order")]
    objects: Vec<(String, Object)>,
    vertices: Vec<[f64; 3]>,
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
    #[serde(rename = "type")]
    kind: String,
    attributes: Option<Map<String, Value>>,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    geometry: Vec<Geometry>,
}

/// A ring of vertex indices.
type Ring = Vec<usize>;
/// A surface: its exterior ring, then its holes.
type Surface = Vec<Ring>;
type Shell = Vec<Surface>;

/// Each surface's index into [`Semantics::surfaces`], nested as the boundaries are, any level of it `null`.
type Values1 = Vec<Option<usize>>;
type Values2 = Vec<Option<Values1>>;
type Values3 = Vec<Option<Values2>>;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Geometry {
    MultiSurface(Geometric<Shell, Values1>),
    CompositeSurface(Geometric<Shell, Values1>),
    Solid(Geometric<Vec<Shell>, Values2>),
    MultiSolid(Geometric<Vec<Vec<Shell>>, Values3>),
    CompositeSolid(Geometric<Vec<Vec<Shell>>, Values3>),
    /// Points, curves and geometry instances, which hold no surface of their own.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(bound = "B: Deserialize<'de>, V: Deserialize<'de>")]
struct Geometric<B, V> {
    lod: LodValue,
    boundaries: B,
    semantics: Option<Semantics<V>>,
}

#[derive(Deserialize)]
#[serde(bound = "V: Deserialize<'de>")]
struct Semantics<V> {
    surfaces: Vec<SemanticSurface>,
    values: Option<V>,
}

#[derive(Deserialize)]
struct SemanticSurface {
    #[serde(rename = "type")]
    kind: String,
}

/// A LOD as `CityJSON` 1.1 and later write it, `"2.2"`, or as 1.0 did, `2.2`.
#[derive(Deserialize)]
#[serde(untagged)]
enum LodValue {
    Text(String),
    Number(serde_json::Number),
}

impl LodValue {
    fn lod(&self) -> AnyResult<Lod> {
        match self {
            Self::Text(text) => text.parse(),
            Self::Number(number) => number.to_string().parse(),
        }
    }
}

/// The members of a JSON object in the order the file lists them.
fn in_file_order<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(String, Object)>, D::Error> {
    struct Members;
    impl<'de> Visitor<'de> for Members {
        type Value = Vec<(String, Object)>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an object of city objects")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut members = Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some(member) = map.next_entry()? {
                members.push(member);
            }
            Ok(members)
        }
    }
    d.deserialize_map(Members)
}

/// The buildings, bridges and tunnels of the `CityJSON` text `bytes`, in the CRS of its `metadata.referenceSystem`.
/// Each takes in the surfaces of its parts and installations, and the attributes of its own.
pub fn parse(bytes: &[u8]) -> AnyResult<SourceDocument> {
    let file: File = serde_json::from_slice(bytes).context("parsing CityJSON")?;
    if file.kind != "CityJSON" {
        bail!("the file is of type {:?}, not CityJSON", file.kind);
    }
    let (scale, translate) = file
        .transform
        .map_or(([1.0; 3], [0.0; 3]), |t| (t.scale, t.translate));
    let reader = Reader {
        vertices: &file.vertices,
        scale,
        translate,
    };
    let index: HashMap<&str, usize> = file
        .objects
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.as_str(), i))
        .collect();
    let mut children: Vec<Vec<usize>> = file
        .objects
        .iter()
        .map(|(_, o)| {
            o.children
                .iter()
                .filter_map(|c| index.get(c.as_str()).copied())
                .collect()
        })
        .collect();
    // CityJSON 1.0 files may name a part's parent without naming the part among the parent's children.
    for (i, (_, object)) in file.objects.iter().enumerate() {
        for parent in &object.parents {
            if let Some(&p) = index.get(parent.as_str())
                && !children[p].contains(&i)
            {
                children[p].push(i);
            }
        }
    }

    let mut city_objects = Vec::new();
    for (root, (id, object)) in file.objects.iter().enumerate() {
        // A building may be a member of a group, but one inside another building is a part of it.
        let in_building = object.parents.iter().any(|p| {
            index
                .get(p.as_str())
                .is_some_and(|&p| ObjectKind::of(&file.objects[p].1.kind).is_some())
        });
        if ObjectKind::of(&object.kind) != Some(ObjectKind::Root) || in_building {
            continue;
        }
        let mut source = SourceObject {
            id: Some(id.clone()),
            polygons: Vec::new(),
            installations: HashSet::new(),
            terrain_points: Vec::new(),
            attributes: object.attributes.clone().map_or_else(Map::new, flatten),
        };
        // Each object of the building, depth first, numbered as its owner.
        let mut seen = HashSet::from([root]);
        let mut stack = vec![root];
        let mut owner = 0;
        while let Some(at) = stack.pop() {
            let (id, object) = &file.objects[at];
            if at != root {
                match ObjectKind::of(&object.kind) {
                    Some(ObjectKind::Installation) => {
                        source.installations.insert(owner);
                    }
                    Some(ObjectKind::Root | ObjectKind::Part) => {}
                    // Rooms, storeys, furniture and the like are not seen from outside.
                    None => continue,
                }
            }
            reader
                .object(object, owner, &mut source.polygons)
                .with_context(|| format!("reading object {id}"))?;
            owner += 1;
            for &child in children[at].iter().rev() {
                if seen.insert(child) {
                    stack.push(child);
                }
            }
        }
        if !source.polygons.is_empty() {
            city_objects.push(source);
        }
    }
    Ok(SourceDocument {
        crs: file.metadata.and_then(|m| m.reference_system),
        lon_first: true,
        city_objects,
    })
}

/// The attributes of an object, nested arrays and objects written as JSON text, as property columns hold no nesting.
fn flatten(attributes: Map<String, Value>) -> Map<String, Value> {
    attributes
        .into_iter()
        .map(|(name, value)| match value {
            Value::Array(_) | Value::Object(_) => (name, Value::String(value.to_string())),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => (name, value),
        })
        .collect()
}

/// The vertices of a file and the transform that places them.
struct Reader<'a> {
    vertices: &'a [[f64; 3]],
    scale: [f64; 3],
    translate: [f64; 3],
}

impl Reader<'_> {
    /// Appends the polygons of the surface geometries of `object` other than its LOD 0 footprints to `out`, each owned by `owner`.
    /// The inner shells of a solid bound voids, which no one sees, so they are left out.
    fn object(&self, object: &Object, owner: u32, out: &mut Vec<OwnedPolygon>) -> AnyResult<()> {
        for geometry in &object.geometry {
            let (lod, surfaces) = match geometry {
                Geometry::MultiSurface(g) | Geometry::CompositeSurface(g) => {
                    let values = g.semantics.as_ref().and_then(|s| s.values.as_ref());
                    let surfaces: Vec<_> = g
                        .boundaries
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (s, value(values, i)))
                        .collect();
                    (&g.lod, semantic(surfaces, g.semantics.as_ref())?)
                }
                Geometry::Solid(g) => {
                    let values = g.semantics.as_ref().and_then(|s| s.values.as_ref());
                    let shell_values = values.and_then(|v| v.first()).and_then(Option::as_ref);
                    let surfaces: Vec<_> = g
                        .boundaries
                        .first()
                        .into_iter()
                        .flatten()
                        .enumerate()
                        .map(|(i, s)| (s, value(shell_values, i)))
                        .collect();
                    (&g.lod, semantic(surfaces, g.semantics.as_ref())?)
                }
                Geometry::MultiSolid(g) | Geometry::CompositeSolid(g) => {
                    let values = g.semantics.as_ref().and_then(|s| s.values.as_ref());
                    let mut surfaces = Vec::new();
                    for (solid, shells) in g.boundaries.iter().enumerate() {
                        let shell_values = values
                            .and_then(|v| v.get(solid))
                            .and_then(Option::as_ref)
                            .and_then(|v| v.first())
                            .and_then(Option::as_ref);
                        for (i, s) in shells.first().into_iter().flatten().enumerate() {
                            surfaces.push((s, value(shell_values, i)));
                        }
                    }
                    (&g.lod, semantic(surfaces, g.semantics.as_ref())?)
                }
                Geometry::Other => continue,
            };
            let lod = lod.lod()?;
            if lod.major() == 0 {
                continue;
            }
            for (surface, part) in surfaces {
                let mut rings = Rings::default();
                for (r, ring) in surface.iter().enumerate() {
                    let positions = ring
                        .iter()
                        .map(|&i| self.position(i))
                        .collect::<AnyResult<Vec<_>>>()?;
                    rings.push(positions, r > 0, false);
                }
                if let Some(polygon) = rings.finish(part) {
                    out.push(OwnedPolygon {
                        owner,
                        lod,
                        polygon,
                    });
                }
            }
        }
        Ok(())
    }

    fn position(&self, index: usize) -> AnyResult<[f64; 3]> {
        let v = self
            .vertices
            .get(index)
            .with_context(|| format!("vertex {index} is out of range"))?;
        Ok(std::array::from_fn(|axis| {
            v[axis].mul_add(self.scale[axis], self.translate[axis])
        }))
    }
}

/// Each of `surfaces` with the part its semantic surface names, those of hidden semantic surfaces left out.
fn semantic<'s, V>(
    surfaces: Vec<(&'s Surface, Option<usize>)>,
    semantics: Option<&Semantics<V>>,
) -> AnyResult<Vec<(&'s Surface, Option<Part>)>> {
    let mut out = Vec::with_capacity(surfaces.len());
    for (surface, value) in surfaces {
        let semantic = match (value, semantics) {
            (Some(i), Some(semantics)) => {
                let surface = semantics.surfaces.get(i).with_context(|| {
                    format!("a surface names semantic surface {i}, which its geometry lacks")
                })?;
                Semantic::of(&surface.kind)
            }
            _ => None,
        };
        match semantic {
            Some(Semantic::Seen(part)) => out.push((surface, Some(part))),
            Some(Semantic::Hidden) => {}
            None => out.push((surface, None)),
        }
    }
    Ok(out)
}

/// The semantic value of surface `i` of `values`, if it has one.
fn value(values: Option<&Values1>, i: usize) -> Option<usize> {
    values.and_then(|v| v.get(i).copied().flatten())
}
