//! End-to-end tests of `mlt convert <file>.city.json <dir>`: each writes a small
//! `CityJSON` file, runs the binary, and decodes the mesh tiles it wrote back through
//! `mlt-core`.

// hotpath appends its profile to stderr, which no exact snapshot can survive.
#![cfg(not(feature = "hotpath"))]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use mlt_core::{Decoder, MValue, Parser};
use serde_json::{Value, json};
use walkdir::WalkDir;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// A temp directory deleted on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mlt-convert-cityjson-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Write `cityjson` as `input.city.json` in a temp dir and run `mlt convert` on it into `out`.
///
/// CI exports `RUST_BACKTRACE=1`, and an inherited backtrace on stderr would break every snapshot here.
fn run_into(cityjson: &Value, args: &[&str]) -> (TempDir, PathBuf, Output) {
    let dir = TempDir::new();
    let input = dir.0.join("input.city.json");
    fs::write(&input, serde_json::to_vec(cityjson).unwrap()).unwrap();
    let output = dir.0.join("out");
    let out = Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("convert")
        .arg(&input)
        .arg(&output)
        .args(["--mlt-version", "2"])
        .args(args)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();
    (dir, output, out)
}

/// The stderr of `out`, with the temp dir it ran in shown as `[TMP]`.
fn stderr_of(dir: &TempDir, out: &Output) -> String {
    String::from_utf8(out.stderr.clone())
        .unwrap()
        .replace(dir.0.to_str().unwrap(), "[TMP]")
}

/// Run a successful conversion, returning every tile it wrote rendered by [`render`], then its stderr.
fn run(cityjson: &Value, args: &[&str]) -> String {
    let (dir, output, out) = run_into(cityjson, args);
    assert!(
        out.status.success(),
        "mlt convert failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    format!("{}---\n{}", render(&output), stderr_of(&dir, &out))
}

/// Run a conversion expected to fail, returning its stderr.
fn run_failing(cityjson: &Value, args: &[&str]) -> String {
    let (dir, output, out) = run_into(cityjson, args);
    assert!(!out.status.success(), "mlt convert unexpectedly succeeded");
    assert_eq!(mlt_files(&output), Vec::<String>::new());
    stderr_of(&dir, &out)
}

/// Every `.mlt` under `dir` as a `z/x/y.mlt` string, sorted.
fn mlt_files(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "mlt"))
        .map(|e| {
            let rel = e.path().strip_prefix(dir).unwrap();
            rel.to_string_lossy().replace('\\', "/")
        })
        .collect();
    out.sort();
    out
}

/// Each tile's meshes: properties, sorted vertices, and the triangle count per surface.
fn render(output: &Path) -> String {
    let mut text = String::new();
    for tile in mlt_files(output) {
        let buffer = fs::read(output.join(&tile)).unwrap();
        let mut dec = Decoder::default();
        for layer in Parser::default().parse_layers(&buffer).unwrap() {
            let layer = layer.into_meshes(&mut dec).unwrap().expect("a mesh layer");
            writeln!(text, "{tile} {} {:?}", layer.name(), layer.z_axis()).unwrap();
            for feature in layer.features() {
                let properties: Vec<String> = layer
                    .property_names()
                    .iter()
                    .zip(feature.properties())
                    .filter_map(|(name, value)| Some(format!("{name}={:?}", value.value_ref()?)))
                    .collect();
                writeln!(text, "  {}", properties.join(" ")).unwrap();
                let mut vertices = feature.mesh().vertices().to_vec();
                vertices.sort_unstable();
                let vertices: Vec<String> = vertices
                    .iter()
                    .map(|[x, y, z]| format!("{x},{y},{z}"))
                    .collect();
                writeln!(text, "  vertices {}", vertices.join(" ")).unwrap();
                let mut surfaces = BTreeMap::<String, usize>::new();
                for (name, values) in layer.m_value_names().iter().zip(feature.m_values()) {
                    if let MValue::Str(Some(values)) = values {
                        for value in values {
                            *surfaces.entry(format!("{name}={value}")).or_default() += 1;
                        }
                    }
                }
                write!(text, "  {} triangles", feature.mesh().triangles().len()).unwrap();
                for (surface, count) in surfaces {
                    write!(text, " {surface} x{count}").unwrap();
                }
                writeln!(text).unwrap();
            }
        }
    }
    text
}

/// One z0 grid unit in degrees of longitude, so integer vertices land on integer grid positions.
const Z0_UNIT: f64 = 360.0 / 4096.0;

/// A `CityJSON` file in EPSG:4326 whose vertices are scaled by one z0 grid unit and one metre.
fn city(objects: &Value, vertices: &Value) -> Value {
    json!({
        "type": "CityJSON",
        "version": "2.0",
        "transform": { "scale": [Z0_UNIT, Z0_UNIT, 1.0], "translate": [0.0, 0.0, 0.0] },
        "metadata": { "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/4326" },
        "CityObjects": objects,
        "vertices": vertices,
    })
}

/// The corners of a box `width` grid units wide, 10 units deep and 10 m tall, with its south-west foot at `(x, y)`.
fn cube_vertices(x: i64, y: i64, width: i64) -> Value {
    let (e, n) = (x + width, y + 10);
    json!([
        [x, y, 0],
        [e, y, 0],
        [e, n, 0],
        [x, n, 0],
        [x, y, 10],
        [e, y, 10],
        [e, n, 10],
        [x, n, 10],
    ])
}

/// A closed cube solid over [`cube_vertices`], each face counter-clockwise seen from outside.
fn cube_solid(lod: &str) -> Value {
    json!({
        "type": "Solid",
        "lod": lod,
        "boundaries": [[
            [[0, 3, 2, 1]], [[4, 5, 6, 7]],
            [[0, 1, 5, 4]], [[1, 2, 6, 5]], [[2, 3, 7, 6]], [[3, 0, 4, 7]],
        ]],
        "semantics": {
            "surfaces": [
                { "type": "GroundSurface" }, { "type": "RoofSurface" }, { "type": "WallSurface" },
            ],
            "values": [[0, 1, 2, 2, 2, 2]],
        },
    })
}

#[test]
fn a_cube_solid_becomes_one_closed_mesh_with_its_surfaces_per_triangle() {
    let cityjson = city(
        &json!({
            "house": {
                "type": "Building",
                "attributes": { "year": 1990 },
                "geometry": [cube_solid("2.2")],
            },
        }),
        &cube_vertices(0, 0, 10),
    );
    insta::assert_snapshot!(run(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @r#"
    0/0/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      year=U32(1990) id=Str("house")
      vertices 2048,2038,0 2048,2038,10 2048,2048,0 2048,2048,10 2058,2038,0 2058,2038,10 2058,2048,0 2058,2048,10
      12 triangles surface=GroundSurface x2 surface=RoofSurface x2 surface=WallSurface x8
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 1 tile(s) holding 1 mesh(es) of LoD 2.2 at zooms 0..=0
    strict mesh rules: dropped 0 degenerate triangle(s), flipped 0 inside-out mesh(es)
    "#);
}

#[test]
fn a_solid_across_a_tile_edge_is_cut_into_an_open_mesh_per_buffered_tile() {
    let cityjson = city(
        &json!({ "hall": { "type": "Building", "geometry": [cube_solid("2.2")] } }),
        &cube_vertices(-50, 40, 100),
    );
    insta::assert_snapshot!(run(&cityjson, &["--min-zoom", "1", "--max-zoom", "1", "--z-step", "0"]), @r#"
    1/0/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      id=Str("hall")
      vertices 3996,3996,0 3996,3996,10 3996,4016,0 3996,4016,10 4160,3996,0 4160,3996,2 4160,3996,10 4160,4000,10 4160,4012,0 4160,4016,0 4160,4016,2 4160,4016,10
      14 triangles surface=GroundSurface x3 surface=RoofSurface x3 surface=WallSurface x8
    1/1/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      id=Str("hall")
      vertices -64,3996,0 -64,3996,8 -64,3996,10 -64,4000,0 -64,4012,10 -64,4016,0 -64,4016,8 -64,4016,10 100,3996,0 100,3996,10 100,4016,0 100,4016,10
      14 triangles surface=GroundSurface x3 surface=RoofSurface x3 surface=WallSurface x8
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 2 tile(s) holding 2 mesh(es) of LoD 2.2 at zooms 1..=1
    strict mesh rules: dropped 0 degenerate triangle(s), flipped 0 inside-out mesh(es)
    "#);
}

#[test]
fn an_inside_out_solid_is_turned_over_and_counted() {
    let inside_out = json!({
        "type": "Solid",
        "lod": "2.2",
        "boundaries": [[
            [[1, 2, 3, 0]], [[7, 6, 5, 4]],
            [[4, 5, 1, 0]], [[5, 6, 2, 1]], [[6, 7, 3, 2]], [[7, 4, 0, 3]],
        ]],
    });
    let cityjson = city(
        &json!({ "house": { "type": "Building", "geometry": [inside_out] } }),
        &cube_vertices(0, 0, 10),
    );
    insta::assert_snapshot!(run(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @r#"
    0/0/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      id=Str("house")
      vertices 2048,2038,0 2048,2038,10 2048,2048,0 2048,2048,10 2058,2038,0 2058,2038,10 2058,2048,0 2058,2048,10
      12 triangles
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 1 tile(s) holding 1 mesh(es) of LoD 2.2 at zooms 0..=0
    strict mesh rules: dropped 0 degenerate triangle(s), flipped 1 inside-out mesh(es)
    "#);
}

#[test]
fn a_sliver_that_rounds_onto_a_line_is_dropped_and_counted() {
    let mut cityjson = city(
        &json!({
            "canopy": {
                "type": "Building",
                "geometry": [{
                    "type": "MultiSurface",
                    "lod": "2.2",
                    "boundaries": [[[0, 1, 2]], [[0, 3, 1]]],
                }],
            },
        }),
        &json!([[0, 0, 0], [1000, 0, 0], [1000, 1, 0], [0, -1000, 0]]),
    );
    cityjson["transform"]["scale"] = json!([Z0_UNIT / 100.0, Z0_UNIT / 100.0, 1.0]);
    insta::assert_snapshot!(run(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @r#"
    0/0/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      id=Str("canopy")
      vertices 2048,2048,0 2048,2058,0 2058,2048,0
      1 triangles
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 1 tile(s) holding 1 mesh(es) of LoD 2.2 at zooms 0..=0
    strict mesh rules: dropped 1 degenerate triangle(s), flipped 0 inside-out mesh(es)
    "#);
}

#[test]
fn a_3d_bag_shaped_part_takes_the_highest_lod_and_its_parents_ground_height() {
    let cityjson = city(
        &json!({
            "pand": {
                "type": "Building",
                "attributes": { "b3_h_maaiveld": 2.0, "status": "in gebruik" },
                "children": ["pand-0"],
                "geometry": [{
                    "type": "MultiSurface",
                    "lod": "0",
                    "boundaries": [[[0, 1, 2, 3]]],
                }],
            },
            "pand-0": {
                "type": "BuildingPart",
                "parents": ["pand"],
                "geometry": [cube_solid("1.2"), cube_solid("2.2")],
            },
        }),
        &cube_vertices(0, 0, 10),
    );
    insta::assert_snapshot!(
        run(&cityjson, &[
            "--max-zoom", "0", "--z-step", "0",
            "--height-mode", "relative-to-ground", "--ground-attribute", "b3_h_maaiveld",
        ]),
        @r#"
    0/0/0.mlt input ZAxis { step: ZStep(0), mode: RelativeToGround }
      b3_h_maaiveld=F64(2.0) status=Str("in gebruik") id=Str("pand-0")
      vertices 2048,2038,-2 2048,2038,8 2048,2048,-2 2048,2048,8 2058,2038,-2 2058,2038,8 2058,2048,-2 2058,2048,8
      12 triangles surface=GroundSurface x2 surface=RoofSurface x2 surface=WallSurface x8
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 1 tile(s) holding 1 mesh(es) of LoD 2.2 at zooms 0..=0
    skipped 1 object(s) with no surfaces at LoD 2.2
    strict mesh rules: dropped 0 degenerate triangle(s), flipped 0 inside-out mesh(es)
    "#,
    );
}

#[test]
fn a_projected_reference_system_is_rejected_with_a_reproject_hint() {
    let mut cityjson = city(
        &json!({ "house": { "type": "Building", "geometry": [cube_solid("2.2")] } }),
        &cube_vertices(0, 0, 10),
    );
    cityjson["metadata"]["referenceSystem"] = json!("https://www.opengis.net/def/crs/EPSG/0/7415");
    insta::assert_snapshot!(run_failing(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @"
    Error: reading [TMP]/input.city.json

    Caused by:
        the file has reference system https://www.opengis.net/def/crs/EPSG/0/7415, but convert reads CityJSON in EPSG:4326 (lon, lat, metres above sea level); reproject it first, e.g. with `cjio <in> reproject 4326 save <out>`
    ");
}

#[test]
fn a_face_with_a_hole_is_triangulated_around_it() {
    let cityjson = city(
        &json!({
            "courtyard": {
                "type": "Building",
                "geometry": [{
                    "type": "MultiSurface",
                    "lod": "2.2",
                    "boundaries": [[[0, 1, 2, 3], [4, 7, 6, 5]]],
                }],
            },
        }),
        &json!([
            [0, 0, 0],
            [10, 0, 0],
            [10, 10, 0],
            [0, 10, 0],
            [3, 3, 0],
            [7, 3, 0],
            [7, 7, 0],
            [3, 7, 0],
        ]),
    );
    insta::assert_snapshot!(run(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @r#"
    0/0/0.mlt input ZAxis { step: ZStep(0), mode: Absolute }
      id=Str("courtyard")
      vertices 2048,2038,0 2048,2048,0 2051,2041,0 2051,2045,0 2055,2041,0 2055,2045,0 2058,2038,0 2058,2048,0
      8 triangles
    ---
    [TMP]/input.city.json -> [TMP]/out: wrote 1 tile(s) holding 1 mesh(es) of LoD 2.2 at zooms 0..=0
    strict mesh rules: dropped 0 degenerate triangle(s), flipped 0 inside-out mesh(es)
    "#);
}

#[test]
fn an_object_without_the_ground_attribute_is_rejected_by_name() {
    let cityjson = city(
        &json!({ "house": { "type": "Building", "geometry": [cube_solid("2.2")] } }),
        &cube_vertices(0, 0, 10),
    );
    insta::assert_snapshot!(
        run_failing(&cityjson, &[
            "--max-zoom", "0", "--z-step", "0",
            "--height-mode", "relative-to-ground", "--ground-attribute", "b3_h_maaiveld",
        ]),
        @"Error: object house has no numeric b3_h_maaiveld attribute",
    );
}

#[test]
fn an_attribute_named_like_the_surface_column_is_rejected_up_front() {
    let cityjson = city(
        &json!({
            "house": {
                "type": "Building",
                "attributes": { "surface": 120.5 },
                "geometry": [cube_solid("2.2")],
            },
        }),
        &cube_vertices(0, 0, 10),
    );
    insta::assert_snapshot!(run_failing(&cityjson, &["--max-zoom", "0", "--z-step", "0"]), @"Error: object house has an attribute named surface, which convert stores each triangle's semantic surface type in");
}

#[test]
fn a_tile_too_heavy_to_decode_is_skipped_naming_the_min_zoom_that_fits() {
    let mut objects = serde_json::Map::new();
    let mut vertices = Vec::new();
    for i in 0..2000_i64 {
        let lon = if i % 2 == 0 { -90 } else { 89 };
        let lat = if i % 4 < 2 { -45 } else { 44 };
        let first = vertices.len();
        vertices.extend([
            json!([lon, lat, 0]),
            json!([lon + 1, lat, 0]),
            json!([lon, lat + 1, 0]),
        ]);
        objects.insert(
            format!("o{i}"),
            json!({
                "type": "Building",
                "attributes": { format!("p{i}"): i },
                "geometry": [{
                    "type": "MultiSurface",
                    "lod": "2.2",
                    "boundaries": [[[first, first + 1, first + 2]]],
                }],
            }),
        );
    }
    let mut cityjson = city(&Value::Object(objects), &Value::Array(vertices));
    cityjson["transform"]["scale"] = json!([1.0, 1.0, 1.0]);
    let (dir, output, out) = run_into(&cityjson, &["--max-zoom", "1", "--z-step", "0"]);
    assert!(!out.status.success(), "mlt convert unexpectedly succeeded");
    insta::assert_snapshot!(stderr_of(&dir, &out), @"Error: tile 0/0/0 needs more memory to decode than mlt-core's default decoder budget allows, so it and any such tile at a lower zoom were not written; pass --min-zoom 1");
    assert_eq!(
        mlt_files(&output),
        ["1/0/0.mlt", "1/0/1.mlt", "1/1/0.mlt", "1/1/1.mlt"]
    );
}
