//! The harness of the `CityGML` and `CityJSON` tests: temp dirs, the binary, writers of both formats, and a summary of each tile.

#![allow(
    dead_code,
    reason = "each test crate uses its own share of the helpers"
)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use mlt_core::geo_types::Geometry;
use mlt_core::{Decoder, MValue, Parser, PropValue, TileFeature};
use serde_json::{Value, json};

pub const E: f64 = 566_100.0;
pub const N: f64 = 5_934_100.0;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mlt-convert-citygml-test-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn mlt_convert(input: &Path, output: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("convert")
        .arg(input)
        .arg(output)
        .args(args)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap()
}

#[track_caller]
pub fn convert(input: &Path, output: &Path, args: &[&str]) {
    let out = mlt_convert(input, output, args);
    assert!(
        out.status.success(),
        "mlt convert failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[track_caller]
pub fn stderr_of(dir: &TempDir, out: &Output) -> String {
    String::from_utf8(out.stderr.clone())
        .unwrap()
        .replace(dir.0.to_str().unwrap(), "[TMP]")
        .replace('\\', "/")
}

pub fn city_model(srs_name: &str, members: &[String]) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0" xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:gml="http://www.opengis.net/gml">
<gml:boundedBy><gml:Envelope srsName="{srs_name}" srsDimension="3"/></gml:boundedBy>
{}
</core:CityModel>
"#,
        members.concat()
    )
}

pub fn building(gml_id: &str, content: &[String]) -> String {
    format!(
        r#"<core:cityObjectMember><bldg:Building gml:id="{gml_id}">{}</bldg:Building></core:cityObjectMember>
"#,
        content.concat()
    )
}

pub fn surface(semantic: &str, lod: u8, polygons: &[String]) -> String {
    let mut members = String::new();
    for p in polygons {
        write!(members, "<gml:surfaceMember>{p}</gml:surfaceMember>").unwrap();
    }
    format!(
        "<bldg:boundedBy><bldg:{semantic}><bldg:lod{lod}MultiSurface><gml:MultiSurface>{members}\
         </gml:MultiSurface></bldg:lod{lod}MultiSurface></bldg:{semantic}></bldg:boundedBy>\n"
    )
}

pub fn pos_list(ring: &[[f64; 3]]) -> String {
    ring.iter()
        .chain(&ring[..1])
        .map(|[x, y, z]| format!("{x} {y} {z}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn polygon(exterior: &[[f64; 3]], holes: &[&[[f64; 3]]]) -> String {
    let mut out = format!(
        "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>{}</gml:posList></gml:LinearRing></gml:exterior>",
        pos_list(exterior)
    );
    for hole in holes {
        write!(
            out,
            "<gml:interior><gml:LinearRing><gml:posList>{}</gml:posList></gml:LinearRing></gml:interior>",
            pos_list(hole)
        )
        .unwrap();
    }
    out + "</gml:Polygon>"
}

pub fn cuboid_faces(min: [f64; 3], max: [f64; 3]) -> Vec<(&'static str, Vec<[f64; 3]>)> {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    vec![
        (
            "WallSurface",
            vec![[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        ),
        (
            "WallSurface",
            vec![[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
        ),
        (
            "WallSurface",
            vec![[x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1]],
        ),
        (
            "WallSurface",
            vec![[x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1]],
        ),
        (
            "RoofSurface",
            vec![[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        ),
        (
            "GroundSurface",
            vec![[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
        ),
    ]
}

pub fn surfaces(lod: u8, faces: &[(&str, Vec<[f64; 3]>)]) -> String {
    faces
        .iter()
        .map(|(semantic, ring)| surface(semantic, lod, &[polygon(ring, &[])]))
        .collect()
}

pub fn cuboid(lod: u8, min: [f64; 3], max: [f64; 3]) -> String {
    surfaces(lod, &cuboid_faces(min, max))
}

pub fn tiles(output: &Path, zoom: u8) -> String {
    let mut paths: Vec<(u32, u32, PathBuf)> = Vec::new();
    let Ok(columns) = fs::read_dir(output.join(zoom.to_string())) else {
        return String::new();
    };
    for column in columns {
        let column = column.unwrap().path();
        let x: u32 = column
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        for tile in fs::read_dir(&column).unwrap() {
            let tile = tile.unwrap().path();
            let y: u32 = tile.file_stem().unwrap().to_str().unwrap().parse().unwrap();
            paths.push((x, y, tile));
        }
    }
    paths.sort();
    let mut out = String::new();
    for (x, y, path) in paths {
        let bytes = fs::read(path).unwrap();
        let mut layers = Parser::default().parse_layers(&bytes).unwrap();
        assert_eq!(layers.len(), 1);
        let layer = layers.pop().unwrap();
        let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
        writeln!(out, "{zoom}/{x}/{y} extent={}", layer.extent().get()).unwrap();
        for feature in layer.features() {
            writeln!(out, "  {}", describe(layer.property_names(), feature)).unwrap();
        }
    }
    out
}

pub fn describe(names: &[String], feature: &TileFeature) -> String {
    let mut out = match feature.id() {
        Some(id) => format!("id={id}"),
        None => "id=-".to_owned(),
    };
    for (name, value) in names.iter().zip(feature.properties()) {
        if let PropValue::Str(Some(value)) = value {
            write!(out, " {name}={value}").unwrap();
        } else if let PropValue::U32(Some(value)) = value {
            write!(out, " {name}={value}").unwrap();
        } else if let Some(value) = value.value_ref() {
            write!(out, " {name}={value:?}").unwrap();
        }
    }
    let Geometry::MultiPoint(points) = feature.geometry() else {
        panic!("{:?}", feature.geometry());
    };
    let xyz: Vec<[i64; 3]> = points
        .iter()
        .zip(feature.z())
        .map(|(p, &z)| [i64::from(p.x()), i64::from(p.y()), i64::from(z)])
        .collect();
    let range = |i: usize| {
        let lo = xyz.iter().map(|p| p[i]).min().unwrap();
        let hi = xyz.iter().map(|p| p[i]).max().unwrap();
        format!("{lo}..{hi}")
    };
    write!(
        out,
        " x={} y={} z={} vertices={}",
        range(0),
        range(1),
        range(2),
        xyz.len()
    )
    .unwrap();
    let MValue::Str(Some(parts)) = &feature.m_values()[0] else {
        panic!("{:?}", feature.m_values());
    };
    for part in ["wall", "roof", "ground", "window", "door"] {
        let count = parts.iter().filter(|p| *p == part).count();
        if count > 0 {
            write!(out, " {part}={count}").unwrap();
        }
    }
    let triangles = feature.triangles().unwrap();
    assert_eq!(triangles.len(), parts.len());
    let mut forward: Vec<[u32; 2]> = triangles
        .iter()
        .flat_map(|&[a, b, c]| [[a, b], [b, c], [c, a]])
        .collect();
    let mut backward: Vec<[u32; 2]> = forward.iter().map(|&[a, b]| [b, a]).collect();
    forward.sort_unstable();
    backward.sort_unstable();
    out.push_str(if forward == backward {
        " closed"
    } else {
        " open"
    });
    if forward.windows(2).any(|pair| pair[0] == pair[1]) {
        out.push_str(" inconsistent");
    }
    let len = i64::try_from(xyz.len()).unwrap();
    let below = [
        xyz.iter().map(|p| p[0]).sum::<i64>() / len,
        -xyz.iter().map(|p| p[1]).sum::<i64>() / len,
        xyz.iter().map(|p| p[2]).min().unwrap(),
    ];
    let (mut volume, mut up, mut roof_area) = (0_i128, 0_i128, 0_i128);
    for (t, part) in triangles.iter().zip(parts) {
        let [a, b, c] = t.map(|v| {
            let [x, y, z] = xyz[v as usize];
            [x, -y, z].map(i128::from)
        });
        let [a, b, c] = [a, b, c].map(|p| [0, 1, 2].map(|i| p[i] - i128::from(below[i])));
        volume += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
        let doubled_area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        up += doubled_area;
        if part == "roof" {
            roof_area += doubled_area.abs();
        }
    }
    write!(out, " roof_area={}", roof_area / 2).unwrap();
    out.push_str(match (volume.signum(), up.signum()) {
        (1, _) => " outward",
        (-1, _) => " inward",
        (_, 1) => " up",
        (_, -1) => " down",
        _ => " flat",
    });
    out
}

/// A polygon of a building: its semantic surface, or `""` for none, its LOD, its exterior and its holes.
pub struct Face {
    pub semantic: &'static str,
    pub lod: u8,
    pub exterior: Vec<[f64; 3]>,
    pub holes: Vec<Vec<[f64; 3]>>,
}

pub fn face(semantic: &'static str, lod: u8, exterior: &[[f64; 3]]) -> Face {
    Face {
        semantic,
        lod,
        exterior: exterior.to_vec(),
        holes: Vec::new(),
    }
}

impl Face {
    pub fn with_holes(mut self, holes: &[&[[f64; 3]]]) -> Self {
        self.holes = holes.iter().map(|h| h.to_vec()).collect();
        self
    }
}

/// The faces of [`cuboid_faces`] at `lod`.
pub fn box_faces(lod: u8, min: [f64; 3], max: [f64; 3]) -> Vec<Face> {
    cuboid_faces(min, max)
        .into_iter()
        .map(|(semantic, ring)| face(semantic, lod, &ring))
        .collect()
}

/// A building, bridge or tunnel, written alike in `CityGML` and `CityJSON`.
pub struct Model {
    pub kind: &'static str,
    pub id: &'static str,
    pub faces: Vec<Face>,
    pub parts: Vec<Vec<Face>>,
    pub installations: Vec<Vec<Face>>,
}

pub fn model(id: &'static str, faces: Vec<Face>) -> Model {
    Model {
        kind: "Building",
        id,
        faces,
        parts: Vec::new(),
        installations: Vec::new(),
    }
}

impl Model {
    pub fn kind(mut self, kind: &'static str) -> Self {
        self.kind = kind;
        self
    }

    pub fn part(mut self, faces: Vec<Face>) -> Self {
        self.parts.push(faces);
        self
    }

    pub fn installation(mut self, faces: Vec<Face>) -> Self {
        self.installations.push(faces);
        self
    }
}

fn gml_faces(faces: &[Face]) -> String {
    faces
        .iter()
        .map(|f| {
            let holes: Vec<&[[f64; 3]]> = f.holes.iter().map(Vec::as_slice).collect();
            let polygon = polygon(&f.exterior, &holes);
            if f.semantic.is_empty() {
                format!(
                    "<bldg:lod{lod}MultiSurface><gml:MultiSurface><gml:surfaceMember>{polygon}\
                     </gml:surfaceMember></gml:MultiSurface></bldg:lod{lod}MultiSurface>\n",
                    lod = f.lod
                )
            } else {
                surface(f.semantic, f.lod, &[polygon])
            }
        })
        .collect()
}

/// `models` as a `CityGML` file in `srs_name`, each part and installation nested in its building.
pub fn to_citygml(srs_name: &str, models: &[Model]) -> String {
    let members: Vec<String> = models
        .iter()
        .map(|m| {
            let mut content = gml_faces(&m.faces);
            for part in &m.parts {
                write!(
                    content,
                    "<bldg:consistsOf{kind}Part><bldg:{kind}Part>{}</bldg:{kind}Part></bldg:consistsOf{kind}Part>",
                    gml_faces(part),
                    kind = m.kind
                )
                .unwrap();
            }
            for installation in &m.installations {
                write!(
                    content,
                    "<bldg:outer{kind}Installation><bldg:{kind}Installation>{}</bldg:{kind}Installation></bldg:outer{kind}Installation>",
                    gml_faces(installation),
                    kind = m.kind
                )
                .unwrap();
            }
            format!(
                "<core:cityObjectMember><bldg:{kind} gml:id=\"{id}\">{content}</bldg:{kind}></core:cityObjectMember>\n",
                kind = m.kind,
                id = m.id
            )
        })
        .collect();
    city_model(srs_name, &members)
}

const STEP: f64 = 1e-9;

/// The vertices of a `CityJSON` file, stored in steps of [`STEP`] from `origin`, fine enough for degrees as well as metres.
struct Vertices {
    origin: [f64; 3],
    list: Vec<[i64; 3]>,
}

impl Vertices {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "test coordinates in steps fit an i64"
    )]
    fn index(&mut self, p: [f64; 3]) -> usize {
        let v = [0, 1, 2].map(|i| ((p[i] - self.origin[i]) / STEP).round() as i64);
        self.list.iter().position(|&w| w == v).unwrap_or_else(|| {
            self.list.push(v);
            self.list.len() - 1
        })
    }

    /// One `MultiSurface` per run of faces of one LOD, so the faces keep their order.
    fn geometries(&mut self, faces: &[Face]) -> Value {
        let mut geometries = Vec::new();
        for run in faces.chunk_by(|a, b| a.lod == b.lod) {
            let mut types: Vec<&str> = Vec::new();
            let mut boundaries = Vec::new();
            let mut values = Vec::new();
            for f in run {
                let rings: Vec<Vec<usize>> = std::iter::once(&f.exterior)
                    .chain(&f.holes)
                    .map(|ring| ring.iter().map(|&p| self.index(p)).collect())
                    .collect();
                boundaries.push(rings);
                values.push(if f.semantic.is_empty() {
                    Value::Null
                } else {
                    let i = types
                        .iter()
                        .position(|t| *t == f.semantic)
                        .unwrap_or_else(|| {
                            types.push(f.semantic);
                            types.len() - 1
                        });
                    json!(i)
                });
            }
            let surfaces: Vec<Value> = types.iter().map(|t| json!({ "type": t })).collect();
            geometries.push(json!({
                "type": "MultiSurface",
                "lod": run[0].lod.to_string(),
                "boundaries": boundaries,
                "semantics": { "surfaces": surfaces, "values": values },
            }));
        }
        Value::Array(geometries)
    }
}

/// `models` as a `CityJSON` 2.0 file in `crs`, each part and installation a child object of its building.
pub fn to_cityjson(crs: &str, models: &[Model]) -> String {
    let origin = models
        .iter()
        .flat_map(|m| &m.faces)
        .flat_map(|f| &f.exterior)
        .fold([f64::INFINITY; 3], |o, p| {
            [0, 1, 2].map(|i| o[i].min(p[i].floor()))
        });
    let mut vertices = Vertices {
        origin,
        list: Vec::new(),
    };
    let mut objects = serde_json::Map::new();
    for m in models {
        let mut children = Vec::new();
        let mut child_objects = Vec::new();
        let kinds = m
            .parts
            .iter()
            .map(|p| ("Part", p))
            .chain(m.installations.iter().map(|i| ("Installation", i)));
        for (n, (suffix, faces)) in kinds.enumerate() {
            let id = format!("{}-{n}", m.id);
            children.push(id.clone());
            child_objects.push((
                id,
                json!({
                    "type": format!("{}{suffix}", m.kind),
                    "parents": [m.id],
                    "geometry": vertices.geometries(faces),
                }),
            ));
        }
        let mut root = json!({ "type": m.kind, "geometry": vertices.geometries(&m.faces) });
        if !children.is_empty() {
            root["children"] = json!(children);
        }
        objects.insert(m.id.to_owned(), root);
        objects.extend(child_objects);
    }
    json!({
        "type": "CityJSON",
        "version": "2.0",
        "transform": { "scale": [STEP, STEP, STEP], "translate": origin },
        "metadata": { "referenceSystem": crs },
        "CityObjects": objects,
        "vertices": vertices.list,
    })
    .to_string()
}
