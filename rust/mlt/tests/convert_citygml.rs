//! End-to-end tests of `mlt convert` tiling a `CityGML` file into a directory of v2 `Meshes` tiles.

// hotpath appends its profile to stderr, which the error checks read.
#![cfg(not(feature = "hotpath"))]

use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mlt_core::geo_types::Geometry;
use mlt_core::{Decoder, HeightMode, MValue, Parser, PropValue, TileFeature, ZAxis, ZStep};

/// Two boxes in Hamburg: `tower`, 60 m high with a ground surface, and `kiosk`, 3 m high without.
const BUILDINGS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/fixtures/meshes/buildings.gml"
);
const TILED: [&str; 6] = ["--mlt-version", "2", "--min-zoom", "15", "--max-zoom", "16"];

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// A temp directory deleted on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mlt-convert-citygml-test-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Run `mlt convert <input> <output> <args>`, returning its stderr when it fails.
fn convert(input: &Path, output: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new(env!("CARGO_BIN_EXE_mlt"))
        .arg("convert")
        .arg(input)
        .arg(output)
        .args(args)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// Half a cell of the LOD2 mesh at zoom 15 in Hamburg in 1 cm steps: half of half a pixel of a
/// 512-pixel tile about 727 m across.
const LOD2_HALF_CELL: i32 = 36;

#[test]
fn each_building_becomes_one_mesh_per_zoom_with_parts_and_heights_above_ground() {
    let dir = TempDir::new();
    let output = dir.0.join("out");
    convert(Path::new(BUILDINGS), &output, &TILED).unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    let [tower, kiosk] = &features(&output, "16/34588/21180")[..] else {
        panic!("2 features at zoom 16");
    };
    assert_box(tower, "tower", 6000..=6000, [8, 2, 2]);
    assert_box(kiosk, "kiosk", 300..=300, [8, 2, 0]);
    // The highest LOD of the boxes is LOD2, which zoom 15 clusters in cells of about 0.7 m.
    let [tower, kiosk] = &features(&output, "15/17294/10590")[..] else {
        panic!("2 features at zoom 15");
    };
    let near = |top: i32| top - LOD2_HALF_CELL..=top + LOD2_HALF_CELL;
    assert_box(tower, "tower", near(6000), [8, 2, 2]);
    assert_box(kiosk, "kiosk", near(300), [8, 2, 0]);
}

#[test]
fn lower_zooms_hold_blocks_merged_from_three_below_the_highest_but_for_landmarks() {
    let dir = TempDir::new();
    let output = dir.0.join("out");
    let zooms = ["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"];
    convert(Path::new(BUILDINGS), &output, &zooms).unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 4);
    let [tower, kiosk] = &features(&output, "14/8647/5295")[..] else {
        panic!("2 features at zoom 14");
    };
    assert_box(tower, "tower", 6000..=6000, [8, 2, 0]);
    assert_box(kiosk, "kiosk", 300..=300, [8, 2, 0]);
    // Zoom 13 is three below the highest: the 60 m tower stays a block of its own, and the 3 m
    // kiosk is merged, alone, into the cells of about 5.7 m its ground covers half of: a roof
    // without walls, lower than a pixel.
    let [tower, kiosk] = &features(&output, "13/4323/2647")[..] else {
        panic!("2 features at zoom 13");
    };
    assert_box(tower, "tower", 6000..=6000, [8, 2, 0]);
    assert_eq!(
        kiosk.properties(),
        [PropValue::Str(None), PropValue::U32(Some(1))]
    );
    let mut heights = kiosk.z().to_vec();
    heights.sort_unstable();
    heights.dedup();
    assert_eq!(heights, [300]);
}

/// The features of the `buildings` layer of the tile `z/x/y` under `output`, by id.
fn features(output: &Path, tile: &str) -> Vec<TileFeature> {
    let bytes = fs::read(output.join(format!("{tile}.mlt"))).unwrap();
    let mut layers = Parser::default().parse_layers(&bytes).unwrap();
    assert_eq!(layers.len(), 1, "{tile}");
    let layer = layers.pop().unwrap();
    let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    assert_eq!(layer.name(), "buildings");
    let ground = ZAxis {
        step: ZStep::new(-2).unwrap(),
        mode: HeightMode::RelativeToGround,
    };
    assert_eq!(layer.z_axis(), Some(ground));
    let names = layer.property_names();
    assert!(
        names == ["gml_id"] || names == ["gml_id", "buildings"],
        "{names:?}"
    );
    assert_eq!(layer.m_value_names(), ["part"]);
    let mut features = layer.features().to_vec();
    features.sort_by_key(TileFeature::id);
    features
}

/// `feature` is the box `gml_id`: 8 welded corners at heights `0` and one in `top`, in 1 cm steps
/// above its ground, and `[wall, roof, ground]` triangles, each counter-clockwise in (x, -y, z)
/// seen from outside and facing the way its part says.
fn assert_box(feature: &TileFeature, gml_id: &str, top: RangeInclusive<i32>, parts: [usize; 3]) {
    let [name, rest @ ..] = feature.properties() else {
        panic!("{gml_id}: no properties");
    };
    assert_eq!(*name, PropValue::Str(Some(gml_id.into())));
    assert!(rest.iter().all(PropValue::is_null), "{gml_id}: {rest:?}");
    let mut heights = feature.z().to_vec();
    heights.sort_unstable();
    heights.dedup();
    let [0, high] = heights[..] else {
        panic!("{gml_id}: heights {heights:?}");
    };
    assert!(top.contains(&high), "{gml_id}: top {high}");
    let Geometry::MultiPoint(points) = feature.geometry() else {
        panic!("{gml_id}: {:?}", feature.geometry());
    };
    let xyz: Vec<[f64; 3]> = points
        .iter()
        .zip(feature.z())
        .map(|(p, &z)| [f64::from(p.x()), -f64::from(p.y()), f64::from(z)])
        .collect();
    assert_eq!(xyz.len(), 8, "{gml_id}");
    let MValue::Str(Some(names)) = &feature.m_values()[0] else {
        panic!("{gml_id}: {:?}", feature.m_values());
    };
    let count = |part: &str| names.iter().filter(|p| *p == part).count();
    let counts = [count("wall"), count("roof"), count("ground")];
    assert_eq!(counts, parts, "{gml_id}");

    let triangles = feature.triangles().unwrap();
    assert_eq!(triangles.len(), names.len(), "{gml_id}");
    let centre = [0, 1, 2].map(|i| xyz.iter().map(|p| p[i]).sum::<f64>() / 8.0);
    for (triangle, part) in triangles.iter().zip(names) {
        let [first, second, third] = triangle.map(|i| xyz[i as usize]);
        let [along, across] = [second, third].map(|c| [0, 1, 2].map(|i| c[i] - first[i]));
        let normal = [
            along[1] * across[2] - along[2] * across[1],
            along[2] * across[0] - along[0] * across[2],
            along[0] * across[1] - along[1] * across[0],
        ];
        let outwards: f64 = (0..3).map(|i| normal[i] * (first[i] - centre[i])).sum();
        assert!(outwards > 0.0, "{gml_id}: {triangle:?} faces inwards");
        let facing = match normal[2] {
            up if up > 0.0 => "roof",
            down if down < 0.0 => "ground",
            _ => "wall",
        };
        assert_eq!(part, facing, "{gml_id}: {triangle:?}");
    }
}

#[test]
fn a_file_naming_the_wrong_utm_zone_is_read_in_that_of_the_others() {
    let dir = TempDir::new();
    let input = dir.0.join("in");
    let output = dir.0.join("out");
    fs::create_dir(&input).unwrap();
    let xml = fs::read_to_string(BUILDINGS).unwrap();
    fs::write(input.join("a.gml"), &xml).unwrap();
    fs::write(input.join("b.gml"), &xml).unwrap();
    fs::write(input.join("c.gml"), xml.replace("EPSG:25832", "EPSG:25833")).unwrap();
    convert(&input, &output, &TILED).unwrap();
    for zoom in ["15", "16"] {
        assert_eq!(
            fs::read_dir(output.join(zoom)).unwrap().count(),
            1,
            "{zoom}"
        );
    }
    let bytes = fs::read(output.join("16/34588/21180.mlt")).unwrap();
    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .pop()
        .unwrap();
    let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    assert_eq!(layer.feature_count(), 6);
}

#[test]
fn citygml_input_rejects_flags_it_cannot_honour_and_srs_names_it_cannot_read() {
    let dir = TempDir::new();
    let input = Path::new(BUILDINGS);
    let output = dir.0.join("out");
    let archive = dir.0.join("out.pmtiles");
    let unknown_crs = dir.0.join("local.gml");
    let xml = fs::read_to_string(input).unwrap();
    fs::write(&unknown_crs, xml.replace("EPSG:25832", "urn:example:local")).unwrap();
    let verify = [&TILED[..], &["--verify"]].concat();
    let height_mode = [&TILED[..], &["--height-mode", "absolute"]].concat();
    for (input, output, args, expected) in [
        (input, &output, &TILED[2..], "needs --mlt-version 2"),
        (input, &output, &TILED[..4], "requires --max-zoom"),
        (
            input,
            &output,
            &verify[..],
            "--verify does not compare meshes",
        ),
        (input, &output, &height_mode[..], "takes no --height-mode"),
        (
            input,
            &archive,
            &TILED[..],
            "archive output is not supported",
        ),
        (
            unknown_crs.as_path(),
            &output,
            &TILED[..],
            "names no EPSG code",
        ),
    ] {
        let err = convert(input, output, args).unwrap_err();
        assert!(err.contains(expected), "{expected}: {err}");
    }
    assert!(!output.exists() && !archive.exists());
}
