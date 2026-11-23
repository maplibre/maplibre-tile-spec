//! The harness of the `CityGML` tests: temp dirs, the binary, a `CityGML` writer, and a summary of each tile.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use mlt_core::geo_types::Geometry;
use mlt_core::{Decoder, MValue, Parser, PropValue, TileFeature};

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
