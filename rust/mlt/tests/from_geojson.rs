//! End-to-end tests of `mlt from-geojson <file> <dir>`: each writes a small
//! `GeoJSON` file, runs the binary, and decodes the tiles it wrote back through
//! `mlt-core` to check geometry (including z), properties, and tile placement.

// The command only exists in a v2 build, and hotpath appends its profile to
// stderr, which no exact snapshot can survive.
#![cfg(all(feature = "unstable-v2", not(feature = "hotpath")))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, Parser};
use serde_json::{Value, json};
use walkdir::WalkDir;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// A temp directory deleted on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("mlt-from-geojson-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Write `geojson` as `<file_name>` in a temp dir and run `mlt from-geojson` on it,
/// writing to `<out_name>` beside it.
///
/// CI exports `RUST_BACKTRACE=1`, and an inherited backtrace on stderr would break
/// every snapshot here.
fn run_into(
    file_name: &str,
    out_name: &str,
    geojson: &Value,
    args: &[&str],
) -> (TempDir, PathBuf, Output) {
    let dir = TempDir::new();
    let input = dir.0.join(file_name);
    fs::write(&input, serde_json::to_vec(geojson).unwrap()).unwrap();
    let output = dir.0.join(out_name);
    let out = Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("from-geojson")
        .arg(&input)
        .arg(&output)
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

#[track_caller]
fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "mlt from-geojson failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Run a successful conversion of `input.geojson`, returning the output dir.
fn run(geojson: &Value, args: &[&str]) -> (TempDir, PathBuf) {
    let (dir, output, out) = run_into("input.geojson", "out", geojson, args);
    assert_success(&out);
    (dir, output)
}

/// Run a conversion expected to fail, returning its stderr.
fn run_failing(geojson: &Value, args: &[&str]) -> String {
    let (dir, output, out) = run_into("input.geojson", "out", geojson, args);
    assert!(
        !out.status.success(),
        "mlt from-geojson unexpectedly succeeded"
    );
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

/// Decode one tile to a `GeoJSON` `FeatureCollection` value.
fn decode(output: &Path, tile: &str) -> Value {
    let buffer = fs::read(output.join(tile)).unwrap();
    let layers = Parser::default().parse_layers(&buffer).unwrap();
    let decoded = Decoder::default().decode_all(layers).unwrap();
    let fc = FeatureCollection::from_layers(decoded).unwrap();
    serde_json::to_value(&fc).unwrap()
}

fn features(fc: &Value) -> &Vec<Value> {
    fc["features"].as_array().unwrap()
}

/// The single feature named `name` (by its `name` property).
fn named<'a>(fc: &'a Value, name: &str) -> &'a Value {
    let matches: Vec<_> = features(fc)
        .iter()
        .filter(|f| f["properties"]["name"] == json!(name))
        .collect();
    assert_eq!(matches.len(), 1, "one feature named {name} in {fc}");
    matches[0]
}

fn coordinates(feature: &Value) -> &Value {
    &feature["geometry"]["coordinates"]
}

/// Every position of a geometry's coordinates, flattened.
fn positions(coords: &Value) -> Vec<Vec<i64>> {
    match coords.as_array().unwrap().first() {
        Some(Value::Number(_)) => vec![
            coords
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect(),
        ],
        Some(_) => coords
            .as_array()
            .unwrap()
            .iter()
            .flat_map(positions)
            .collect(),
        None => Vec::new(),
    }
}

fn feature(geometry: &Value, properties: &Value) -> Value {
    json!({ "type": "Feature", "geometry": geometry, "properties": properties })
}

fn collection(features: &[Value]) -> Value {
    json!({ "type": "FeatureCollection", "features": features })
}

#[test]
fn a_flat_collection_round_trips_at_z0() {
    let geojson = collection(&[
        json!({
            "type": "Feature", "id": 7,
            "geometry": { "type": "Point", "coordinates": [0, 0] },
            "properties": { "name": "origin", "rank": 5, "score": 3.5, "active": true,
                            "note": null }
        }),
        feature(
            &json!({ "type": "LineString", "coordinates": [[0, 0], [0, 60]] }),
            &json!({ "name": "edge", "rank": -2 }),
        ),
        feature(
            &json!({ "type": "Polygon", "coordinates": [[[-10, -10], [10, -10], [10, 10], [-10, 10], [-10, -10]]] }),
            &json!({ "name": "box" }),
        ),
    ]);
    let (_dir, output) = run(&geojson, &["--max-zoom", "0", "--layer", "places"]);

    assert_eq!(mlt_files(&output), ["0/0/0.mlt"]);
    let fc = decode(&output, "0/0/0.mlt");
    assert_eq!(features(&fc).len(), 3);
    for f in features(&fc) {
        assert_eq!(f["properties"]["_layer"], json!("places"));
        assert_eq!(f["properties"]["_extent"], json!(4096));
        assert!(f["properties"].get("_z_step").is_none(), "{f}");
    }

    let origin = named(&fc, "origin");
    assert_eq!(origin["id"], json!(7));
    assert_eq!(coordinates(origin), &json!([2048, 2048]));
    assert_eq!(origin["properties"]["rank"], json!(5));
    assert_eq!(origin["properties"]["score"], json!(3.5));
    assert_eq!(origin["properties"]["active"], json!(true));
    assert!(origin["properties"]["note"].is_null());

    let edge = named(&fc, "edge");
    assert!(edge.get("id").is_none_or(Value::is_null));
    assert_eq!(coordinates(edge), &json!([[2048, 2048], [2048, 1189]]));
    assert_eq!(edge["properties"]["rank"], json!(-2));
    assert!(edge["properties"]["score"].is_null());

    let bbox = named(&fc, "box");
    let ring = &coordinates(bbox)[0];
    assert_eq!(ring.as_array().unwrap().len(), 5);
    assert_eq!(ring[0], ring[4]);
    assert!(
        positions(ring)
            .iter()
            .all(|p| p.iter().all(|c| (1934..=2162).contains(c))),
        "{ring}"
    );
}

#[test]
fn the_layer_name_defaults_to_the_file_stem() {
    let geojson = collection(&[feature(
        &json!({ "type": "Point", "coordinates": [0, 0] }),
        &json!({}),
    )]);
    let (_dir, output, out) = run_into("places.geojson", "out", &geojson, &["--max-zoom", "0"]);
    assert_success(&out);
    let fc = decode(&output, "0/0/0.mlt");
    assert_eq!(features(&fc)[0]["properties"]["_layer"], json!("places"));
}

#[test]
fn a_point_on_a_shared_tile_corner_is_written_once() {
    let geojson = collection(&[feature(
        &json!({ "type": "Point", "coordinates": [0, 0] }),
        &json!({ "name": "corner" }),
    )]);
    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    assert_eq!(mlt_files(&output), ["1/1/1.mlt"]);
    let fc = decode(&output, "1/1/1.mlt");
    assert_eq!(coordinates(named(&fc, "corner")), &json!([0, 0]));
}

#[test]
fn a_multi_point_is_split_between_the_tiles_its_points_fall_in() {
    let geojson = collection(&[feature(
        &json!({ "type": "MultiPoint", "coordinates": [[-90, 45], [90, 45], [90, -45]] }),
        &json!({ "name": "mp" }),
    )]);
    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    assert_eq!(mlt_files(&output), ["1/0/0.mlt", "1/1/0.mlt", "1/1/1.mlt"]);
    let fc = decode(&output, "1/0/0.mlt");
    assert_eq!(named(&fc, "mp")["geometry"]["type"], json!("MultiPoint"));
    assert_eq!(coordinates(named(&fc, "mp")), &json!([[2048, 2947]]));
}

#[test]
fn a_line_crossing_tiles_is_clipped_into_each_with_the_buffer() {
    // The equator is the boundary between the two z1 rows, so the line lands in all four tiles.
    let geojson = collection(&[feature(
        &json!({ "type": "LineString", "coordinates": [[-90, 0], [90, 0]] }),
        &json!({ "name": "equator" }),
    )]);
    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    assert_eq!(
        mlt_files(&output),
        ["1/0/0.mlt", "1/0/1.mlt", "1/1/0.mlt", "1/1/1.mlt"]
    );
    let fc = decode(&output, "1/0/0.mlt");
    assert_eq!(
        coordinates(named(&fc, "equator")),
        &json!([[2048, 4096], [4160, 4096]])
    );
    let fc = decode(&output, "1/1/1.mlt");
    assert_eq!(
        coordinates(named(&fc, "equator")),
        &json!([[-64, 0], [2048, 0]])
    );
}

#[test]
fn a_polygon_spanning_tiles_is_clipped_to_each_buffered_tile() {
    let geojson = collection(&[feature(
        &json!({ "type": "Polygon", "coordinates": [[[-90, -45], [90, -45], [90, 45], [-90, 45], [-90, -45]]] }),
        &json!({ "name": "big" }),
    )]);
    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    assert_eq!(
        mlt_files(&output),
        ["1/0/0.mlt", "1/0/1.mlt", "1/1/0.mlt", "1/1/1.mlt"]
    );
    for tile in mlt_files(&output) {
        let fc = decode(&output, &tile);
        let big = named(&fc, "big");
        assert_eq!(big["geometry"]["type"], json!("Polygon"));
        let ring = &coordinates(big)[0];
        assert_eq!(ring.as_array().unwrap().len(), 5, "{tile}: {ring}");
        for p in positions(ring) {
            assert!(p.iter().all(|c| (-64..=4160).contains(c)), "{tile}: {p:?}");
        }
    }
}

#[test]
fn a_line_split_by_a_tile_decodes_as_a_multi_line_string() {
    // The line leaves tile 1/0/0 to the east and comes back, so two parts remain there.
    let geojson = collection(&[feature(
        &json!({ "type": "LineString", "coordinates": [[-45, 45], [45, 45], [45, 20], [-45, 20]] }),
        &json!({ "name": "hook" }),
    )]);
    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    let fc = decode(&output, "1/0/0.mlt");
    let hook = named(&fc, "hook");
    assert_eq!(hook["geometry"]["type"], json!("MultiLineString"));
    assert_eq!(coordinates(hook).as_array().unwrap().len(), 2);
}

#[test]
fn every_zoom_from_min_to_max_is_written() {
    let geojson = collection(&[feature(
        &json!({ "type": "Point", "coordinates": [10, 10] }),
        &json!({}),
    )]);
    let (_dir, output) = run(&geojson, &["--max-zoom", "2"]);
    assert_eq!(mlt_files(&output), ["0/0/0.mlt", "1/1/0.mlt", "2/2/1.mlt"]);
}

#[test]
fn a_null_geometry_is_skipped() {
    let geojson = collection(&[
        json!({ "type": "Feature", "geometry": null, "properties": { "name": "ghost" } }),
        feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({ "name": "real" }),
        ),
    ]);
    let (_dir, output) = run(&geojson, &["--max-zoom", "0"]);
    assert_eq!(features(&decode(&output, "0/0/0.mlt")).len(), 1);
}

#[test]
fn a_bare_feature_or_geometry_is_rejected() {
    let stderr = run_failing(
        &feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({}),
        ),
        &["--max-zoom", "0"],
    );
    insta::assert_snapshot!(stderr, @"
    Error: reading [TMP]/input.geojson

    Caused by:
        Expected GeoJSON type `FeatureCollection`, found `Feature`
    ");
    let stderr = run_failing(
        &json!({ "type": "Point", "coordinates": [0, 0] }),
        &["--max-zoom", "0"],
    );
    insta::assert_snapshot!(stderr, @"
    Error: reading [TMP]/input.geojson

    Caused by:
        Expected GeoJSON type `FeatureCollection`, found `Geometry`
    ");
}

#[test]
fn an_empty_collection_writes_no_tiles() {
    let (dir, output, out) = run_into(
        "input.geojson",
        "out",
        &collection(&[]),
        &["--max-zoom", "3"],
    );
    assert_success(&out);
    assert_eq!(mlt_files(&output), Vec::<String>::new());
    let stderr = stderr_of(&dir, &out);
    insta::assert_snapshot!(stderr, @"No features with a geometry to tile in [TMP]/input.geojson");
}

#[test]
fn features_too_small_for_the_zoom_write_no_tiles_and_say_so() {
    // A 1 m square is far below a z0 grid unit (about 10 km), so it collapses.
    let geojson = collection(&[feature(
        &json!({ "type": "Polygon", "coordinates": [[[0, 0], [0.00001, 0], [0.00001, 0.00001], [0, 0]]] }),
        &json!({}),
    )]);
    let (dir, output, out) = run_into("input.geojson", "out", &geojson, &["--max-zoom", "0"]);
    assert_success(&out);
    assert_eq!(mlt_files(&output), Vec::<String>::new());
    let stderr = stderr_of(&dir, &out);
    insta::assert_snapshot!(stderr, @"[TMP]/input.geojson: none of its 1 feature(s) spans a grid unit at zooms 0..=0, so no tile was written; raise --max-zoom");
}

/// 2000 points spread evenly over the four z1 tiles, each with a property key of
/// its own. A decoder lays out every column for every feature, so the z0 tile
/// holding all of them needs about 2000 x 2000 values (~90 MiB) and outgrows the
/// default decoder budget (64 MiB), while each z1 tile holds a quarter of the
/// features and fits.
fn too_heavy_at_z0() -> Value {
    let features: Vec<Value> = (0..2000)
        .map(|i| {
            let lon = if i % 2 == 0 { -90 } else { 90 };
            let lat = if i % 4 < 2 { -45 } else { 45 };
            feature(
                &json!({ "type": "Point", "coordinates": [lon, lat] }),
                &json!({ format!("p{i}"): i }),
            )
        })
        .collect();
    collection(&features)
}

#[test]
fn a_tile_too_heavy_to_decode_fails_naming_the_min_zoom_that_fits() {
    let geojson = too_heavy_at_z0();
    let (dir, output, out) = run_into("input.geojson", "out", &geojson, &["--max-zoom", "1"]);
    assert!(
        !out.status.success(),
        "mlt from-geojson unexpectedly succeeded"
    );
    let stderr = stderr_of(&dir, &out);
    insta::assert_snapshot!(stderr, @"Error: tile 0/0/0 needs more memory to decode than mlt-core's default decoder budget allows, so it and any such tile at a lower zoom were not written; pass --min-zoom 1");
    assert_eq!(
        mlt_files(&output),
        ["1/0/0.mlt", "1/0/1.mlt", "1/1/0.mlt", "1/1/1.mlt"]
    );

    let (_dir, output) = run(&geojson, &["--min-zoom", "1", "--max-zoom", "1"]);
    assert_eq!(
        mlt_files(&output),
        ["1/0/0.mlt", "1/0/1.mlt", "1/1/0.mlt", "1/1/1.mlt"]
    );
}

#[test]
fn a_tile_too_heavy_to_decode_at_the_max_zoom_asks_for_a_higher_max_zoom() {
    let stderr = run_failing(&too_heavy_at_z0(), &["--max-zoom", "0"]);
    insta::assert_snapshot!(stderr, @"Error: tile 0/0/0 needs more memory to decode than mlt-core's default decoder budget allows, so it and any such tile at a lower zoom were not written; pass --min-zoom 1 and a --max-zoom of at least 1");
}

#[test]
fn a_string_id_is_rejected_naming_the_feature() {
    let geojson = collection(&[
        feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({}),
        ),
        json!({ "type": "Feature", "id": "way/42", "geometry": { "type": "Point", "coordinates": [1, 1] }, "properties": {} }),
    ]);
    let stderr = run_failing(&geojson, &["--max-zoom", "0"]);
    insta::assert_snapshot!(stderr, @r#"
    Error: reading the id of feature 1

    Caused by:
        id "way/42" is a string, but an MLT id must be a non-negative integer; drop it or move it to a property
    "#);
}

#[test]
fn an_invalid_position_is_rejected_naming_the_feature() {
    let geojson = collection(&[
        feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({}),
        ),
        feature(
            &json!({ "type": "Point", "coordinates": [181.5, 10] }),
            &json!({}),
        ),
    ]);
    let stderr = run_failing(&geojson, &["--max-zoom", "0"]);
    insta::assert_snapshot!(stderr, @"
    Error: projecting feature 1

    Caused by:
        position [181.5, 10.0] has a longitude outside -180..=180
    ");
}

#[test]
fn a_min_zoom_above_the_max_zoom_is_rejected() {
    let stderr = run_failing(&collection(&[]), &["--min-zoom", "5", "--max-zoom", "2"]);
    insta::assert_snapshot!(stderr, @"Error: --min-zoom (5) must be <= --max-zoom (2)");
    let stderr = run_failing(&collection(&[]), &["--max-zoom", "31"]);
    insta::assert_snapshot!(stderr, @"Error: --max-zoom (31) must be <= 30");
}

#[test]
fn an_archive_output_is_rejected() {
    let (dir, output, out) = run_into(
        "input.geojson",
        "out.pmtiles",
        &collection(&[]),
        &["--max-zoom", "0"],
    );
    assert!(!out.status.success());
    assert!(!output.exists());
    let stderr = stderr_of(&dir, &out);
    insta::assert_snapshot!(stderr, @"Error: from-geojson writes a directory of z/x/y.mlt tiles; archive output is not supported yet, got: [TMP]/out.pmtiles");
}

#[test]
fn mixing_flat_and_3d_positions_is_rejected() {
    let geojson = collection(&[
        feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({}),
        ),
        feature(
            &json!({ "type": "Point", "coordinates": [0, 10, 5] }),
            &json!({}),
        ),
    ]);
    let stderr = run_failing(&geojson, &["--max-zoom", "0"]);
    insta::assert_snapshot!(stderr, @"
    Error: projecting feature 1

    Caused by:
        position [0.0, 10.0, 5.0] has an altitude, but the first position [0.0, 0.0] has none; an MLT layer is either flat or 3D, so make them all [lon, lat] or all [lon, lat, alt]
    ");
}

mod z {
    use super::*;

    #[test]
    fn altitudes_round_trip_on_the_z_step_grid() {
        let geojson = collection(&[
            feature(
                &json!({ "type": "Point", "coordinates": [0, 0, 12.3] }),
                &json!({ "name": "p" }),
            ),
            feature(
                &json!({ "type": "LineString", "coordinates": [[0, 0, -4], [0, 60, 350]] }),
                &json!({ "name": "l" }),
            ),
            feature(
                &json!({ "type": "Polygon", "coordinates": [[[-10, -10, 1], [10, -10, 2], [10, 10, 3], [-10, 10, 4], [-10, -10, 1]]] }),
                &json!({ "name": "poly" }),
            ),
        ]);
        let (_dir, output) = run(&geojson, &["--max-zoom", "0", "--z-step", "-1"]);
        let fc = decode(&output, "0/0/0.mlt");
        for f in features(&fc) {
            assert_eq!(f["properties"]["_z_step"], json!(-1), "{f}");
        }
        // z = (metres + 10000) * 10 on the decimetre grid.
        assert_eq!(coordinates(named(&fc, "p")), &json!([2048, 2048, 100_123]));
        assert_eq!(
            coordinates(named(&fc, "l")),
            &json!([[2048, 2048, 99_960], [2048, 1189, 103_500]])
        );
        let ring = &coordinates(named(&fc, "poly"))[0];
        let zs: Vec<i64> = positions(ring).iter().map(|p| p[2]).collect();
        assert_eq!(zs.len(), 5);
        assert_eq!(zs[0], zs[4]);
        let mut distinct = zs[..4].to_vec();
        distinct.sort_unstable();
        assert_eq!(distinct, [100_010, 100_020, 100_030, 100_040]);
    }

    #[test]
    fn a_clipped_line_interpolates_its_altitude_at_the_cut() {
        // From 0 m at 90°W to 1000 m at 90°E: tile 1/0/0 keeps the line up to its
        // buffered edge at world x 4160, 51.5625% of the way, and tile 1/1/0 from
        // world x 4032, 48.4375% of the way.
        let geojson = collection(&[feature(
            &json!({ "type": "LineString", "coordinates": [[-90, 0, 0], [90, 0, 1000]] }),
            &json!({ "name": "ramp" }),
        )]);
        let (_dir, output) = run(
            &geojson,
            &["--min-zoom", "1", "--max-zoom", "1", "--z-step", "0"],
        );
        let fc = decode(&output, "1/0/0.mlt");
        assert_eq!(
            coordinates(named(&fc, "ramp")),
            &json!([[2048, 4096, 10_000], [4160, 4096, 10_516]])
        );
        let fc = decode(&output, "1/1/0.mlt");
        assert_eq!(
            coordinates(named(&fc, "ramp")),
            &json!([[-64, 4096, 10_484], [2048, 4096, 11_000]])
        );
    }

    #[test]
    fn altitudes_without_a_z_step_are_rejected() {
        let geojson = collection(&[feature(
            &json!({ "type": "Point", "coordinates": [0, 0, 5] }),
            &json!({}),
        )]);
        let stderr = run_failing(&geojson, &["--max-zoom", "0"]);
        insta::assert_snapshot!(stderr, @"Error: the GeoJSON positions carry an altitude, so --z-step is required (the power of ten of the z grid's step in metres, -3..=4)");
    }

    #[test]
    fn a_z_step_without_altitudes_is_rejected() {
        let geojson = collection(&[feature(
            &json!({ "type": "Point", "coordinates": [0, 0] }),
            &json!({}),
        )]);
        let stderr = run_failing(&geojson, &["--max-zoom", "0", "--z-step", "0"]);
        insta::assert_snapshot!(stderr, @"Error: --z-step needs [lon, lat, alt] positions, but the GeoJSON positions have no altitude");
    }

    #[test]
    fn a_z_step_outside_the_grid_is_rejected() {
        let geojson = collection(&[feature(
            &json!({ "type": "Point", "coordinates": [0, 0, 5] }),
            &json!({}),
        )]);
        let stderr = run_failing(&geojson, &["--max-zoom", "0", "--z-step", "5"]);
        insta::assert_snapshot!(stderr, @"Error: a z step of 10^5 m is outside 10^-3..=10^4 m");
    }
}
