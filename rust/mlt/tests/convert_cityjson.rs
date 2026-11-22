//! End-to-end tests of `mlt convert` tiling `CityJSON` files into directories of v2 `Meshes` tiles.

#![cfg(not(feature = "hotpath"))]

mod city;

use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;

use mlt_core::{Decoder, Parser};
use serde_json::{Value, json};

use self::city::*;

/// Tiles `models` written once as `CityGML` and once as `CityJSON` with `args`, asserts that every zoom of `zooms` matches, and returns the `CityJSON` tiles.
#[track_caller]
fn tiled_alike(
    srs_name: &str,
    models: &[Model],
    args: &[&str],
    zooms: RangeInclusive<u8>,
) -> String {
    let dir = TempDir::new();
    let gml = dir.write("in.gml", &to_citygml(srs_name, models));
    let json = dir.write("in.city.json", &to_cityjson(srs_name, models));
    let (from_gml, from_json) = (dir.0.join("gml"), dir.0.join("json"));
    let args = [&["--mlt-version", "2", "--verify"][..], args].concat();
    convert(&gml, &from_gml, &args);
    convert(&json, &from_json, &args);
    let mut out = String::new();
    for zoom in zooms {
        let tiles_of_json = tiles(&from_json, zoom);
        assert_eq!(tiles_of_json, tiles(&from_gml, zoom), "zoom {zoom}");
        out += &tiles_of_json;
    }
    out
}

fn write_json(dir: &TempDir, name: &str, value: &Value) -> PathBuf {
    dir.write(name, &value.to_string())
}

/// A `CityJSON` file in Amersfoort / RD New + NAP height whose vertices are millimetres from a point in Delft.
fn delft(objects: &Value, vertices: &Value) -> Value {
    json!({
        "type": "CityJSON",
        "version": "2.0",
        "transform": { "scale": [0.001, 0.001, 0.001], "translate": [84600.0, 447_000.0, 0.0] },
        "metadata": { "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415" },
        "CityObjects": objects,
        "vertices": vertices,
    })
}

/// The corners of a box 10 m square from `x` to `top`, its foot at `bottom`, in millimetres east of the origin of [`delft`].
fn box_vertices(x: i64, bottom: i64, top: i64) -> Vec<Value> {
    let e = x + 10_000;
    vec![
        json!([x, 0, bottom]),
        json!([e, 0, bottom]),
        json!([e, 10_000, bottom]),
        json!([x, 10_000, bottom]),
        json!([x, 0, top]),
        json!([e, 0, top]),
        json!([e, 10_000, top]),
        json!([x, 10_000, top]),
    ]
}

/// The outer shell of a box over the 8 vertices from `first` of [`box_vertices`], each face counter-clockwise seen from outside.
fn box_shell(first: usize) -> Value {
    let v = |i: usize| first + i;
    json!([
        [[v(0), v(3), v(2), v(1)]],
        [[v(4), v(5), v(6), v(7)]],
        [[v(0), v(1), v(5), v(4)]],
        [[v(1), v(2), v(6), v(5)]],
        [[v(2), v(3), v(7), v(6)]],
        [[v(3), v(0), v(4), v(7)]],
    ])
}

/// The semantics of [`box_shell`]: its ground, its roof, then its four walls.
fn box_semantics() -> Value {
    json!({
        "surfaces": [{ "type": "GroundSurface" }, { "type": "RoofSurface" }, { "type": "WallSurface" }],
        "values": [[0, 1, 2, 2, 2, 2]],
    })
}

fn box_solid(lod: &str, first: usize) -> Value {
    json!({ "type": "Solid", "lod": lod, "boundaries": [box_shell(first)], "semantics": box_semantics() })
}

#[test]
fn the_zoom_tiers_of_a_tower_and_a_kiosk_match_their_citygml() {
    let models = [
        model(
            "tower",
            box_faces(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5]),
        ),
        model("kiosk", {
            let mut faces = box_faces(2, [E + 40.0, N + 10.0, 7.0], [E + 48.0, N + 18.0, 10.0]);
            faces.pop();
            faces
        }),
    ];
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &models, &["--min-zoom", "13", "--max-zoom", "16"], 13..=16),
        @"
    13/4323/2647 extent=262144
      id=0 gml_id=tower x=132046..133872 y=146724..148554 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
      id=1 buildings=1 x=135680..136704 y=146944..147968 z=300..300 vertices=4 roof=2 open roof_area=786432 up
    14/8647/5295 extent=131072
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=534360 outward
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    15/17294/10590 extent=65536
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=519940 outward
    16/34588/21180 extent=32768
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=519940 outward
    "
    );
}

#[test]
fn lod3_windows_and_installations_drop_out_of_lod2_as_in_citygml() {
    let mut faces = box_faces(3, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let window = [
        [E + 5.0, N, 3.0],
        [E + 8.0, N, 3.0],
        [E + 8.0, N, 6.0],
        [E + 5.0, N, 6.0],
    ];
    let hole: Vec<[f64; 3]> = window.iter().rev().copied().collect();
    faces[0] = face("WallSurface", 3, &faces[0].exterior).with_holes(&[&hole]);
    faces.insert(1, face("Window", 3, &window));
    let models = [model("house", faces).installation(box_faces(
        3,
        [E + 22.0, N, 0.0],
        [E + 30.0, N + 8.0, 3.0],
    ))];
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &models, &["--min-zoom", "15", "--max-zoom", "16"], 15..=16),
        @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..3685 y=15652..17494 z=0..1000 vertices=20 wall=22 roof=4 ground=4 window=2 closed roof_area=3769592 outward
    "
    );
}

#[test]
fn walls_roofs_and_grounds_each_pick_their_own_lods_as_in_citygml() {
    let lod = |lod: u8| box_faces(lod, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let window_hole = [
        [E + 5.0, N, 6.0],
        [E + 8.0, N, 6.0],
        [E + 8.0, N, 3.0],
        [E + 5.0, N, 3.0],
    ];
    let mut faces: Vec<Face> = lod(1).into_iter().take(4).collect();
    faces.push(face("WallSurface", 3, &lod(3)[0].exterior).with_holes(&[&window_hole]));
    faces.extend(lod(3).into_iter().skip(1).take(3));
    faces.extend(lod(2).into_iter().skip(4).take(1));
    faces.extend(lod(1).into_iter().skip(5));
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &[model("house", faces)], &["--min-zoom", "15", "--max-zoom", "16"], 15..=16),
        @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=12 wall=14 roof=2 ground=2 open roof_area=3249652 outward
    "
    );
}

#[test]
fn each_building_part_picks_its_own_highest_lod_as_in_citygml() {
    let faces = |lod: u8| box_faces(lod, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let models = [model("house", faces(3).into_iter().take(4).collect())
        .part(faces(2).into_iter().skip(5).collect())
        .part(faces(1).into_iter().skip(4).take(1).collect())];
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &models, &["--max-zoom", "16"], 16..=16),
        @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    "
    );
}

#[test]
fn lod2_keeps_courtyards_and_closes_skylights_as_in_citygml() {
    let faces = box_faces(3, [E, N, 0.0], [E + 30.0, N + 30.0, 10.0]);
    let courtyard = [
        [E + 10.0, N + 10.0, 10.0],
        [E + 20.0, N + 10.0, 10.0],
        [E + 20.0, N + 20.0, 10.0],
        [E + 10.0, N + 20.0, 10.0],
    ];
    let skylight = [
        [E + 2.0, N + 2.0, 10.0],
        [E + 5.0, N + 2.0, 10.0],
        [E + 2.0, N + 5.0, 10.0],
    ];
    let skylight_hole: Vec<[f64; 3]> = skylight.iter().rev().copied().collect();
    let courtyard_hole: Vec<[f64; 3]> = courtyard.iter().rev().copied().collect();
    let courtyard_ground: Vec<[f64; 3]> = courtyard.iter().map(|&[x, y, _]| [x, y, 0.0]).collect();
    let mut house: Vec<Face> = faces
        .iter()
        .take(4)
        .map(|f| face(f.semantic, 3, &f.exterior))
        .collect();
    house.push(
        face("RoofSurface", 3, &faces[4].exterior).with_holes(&[&courtyard_hole, &skylight_hole]),
    );
    house.push(face("Window", 3, &skylight));
    house.push(face("GroundSurface", 3, &faces[5].exterior).with_holes(&[&courtyard_ground]));
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &[model("house", house)], &["--min-zoom", "15", "--max-zoom", "16"], 15..=16),
        @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..3713 y=14749..17494 z=0..1000 vertices=12 wall=8 roof=8 open roof_area=6500193 outward
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..3713 y=14749..17494 z=0..1000 vertices=19 wall=8 roof=13 ground=8 window=1 open roof_area=6463735 outward
    "
    );
}

#[test]
fn touching_houses_merge_into_blocks_as_in_citygml() {
    let models = [
        model("a", box_faces(2, [E, N, 0.0], [E + 30.0, N + 30.0, 8.0])),
        model(
            "b",
            box_faces(2, [E + 30.0, N, 0.0], [E + 60.0, N + 30.0, 9.0]),
        ),
        model(
            "c",
            box_faces(2, [E + 60.0, N, 0.0], [E + 90.0, N + 30.0, 25.0]),
        ),
        model(
            "d",
            box_faces(2, [E, N + 90.0, 0.0], [E + 30.0, N + 120.0, 6.0]),
        ),
    ];
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &models, &["--min-zoom", "12", "--max-zoom", "15"], 12..=13),
        @"
    12/2161/1323 extent=524288
      id=2 buildings=1 x=399360..402432 y=407552..410624 z=0..2500 vertices=10 wall=8 roof=2 open roof_area=9437184 outward
      id=3 buildings=1 x=394240..397312 y=399360..402432 z=600..600 vertices=4 roof=2 open roof_area=7864320 up
      id=0 buildings=2 x=394240..399360 y=407552..410624 z=860..860 vertices=4 roof=2 open roof_area=15728640 up
    13/4323/2647 extent=262144
      id=3 gml_id=d x=132160..134899 y=137700..140445 z=0..600 vertices=8 wall=8 roof=2 open roof_area=7313051 outward
      id=1 gml_id=b x=134747..137486 y=145859..148604 z=0..900 vertices=8 wall=8 roof=2 open roof_area=7313051 outward
      id=0 gml_id=a x=132046..134785 y=145821..148566 z=0..800 vertices=8 wall=8 roof=2 open roof_area=7313051 outward
      id=2 gml_id=c x=137448..140187 y=145897..148642 z=0..2500 vertices=8 wall=8 roof=2 open roof_area=7313051 outward
    "
    );
}

#[test]
fn faces_without_semantics_are_roofs_walls_and_grounds_by_their_facing_as_in_citygml() {
    let faces = box_faces(2, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0])
        .into_iter()
        .map(|f| face("", 2, &f.exterior))
        .collect();
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &[model("shed", faces)], &["--max-zoom", "16"], 16..=16),
        @"
    16/34588/21180 extent=32768
      id=0 gml_id=shed x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    "
    );
}

#[test]
fn bridges_and_tunnels_with_their_parts_tile_as_in_citygml() {
    let models = [
        model(
            "bridge",
            box_faces(2, [E, N, 5.0], [E + 40.0, N + 8.0, 7.0]),
        )
        .kind("Bridge")
        .part(box_faces(
            2,
            [E + 18.0, N + 2.0, 0.0],
            [E + 22.0, N + 6.0, 5.0],
        )),
        model(
            "tunnel",
            box_faces(2, [E + 60.0, N, -10.0], [E + 90.0, N + 10.0, -4.0]),
        )
        .kind("Tunnel"),
    ];
    insta::assert_snapshot!(
        tiled_alike("EPSG:25832", &models, &["--max-zoom", "16"], 16..=16),
        @"
    16/34588/21180 extent=32768
      id=0 gml_id=bridge x=974..4586 y=16735..17507 z=0..700 vertices=16 wall=16 roof=4 ground=4 closed roof_area=2729023 outward
      id=1 gml_id=tunnel x=6376..9089 y=16630..17570 z=0..600 vertices=8 wall=8 roof=2 ground=2 closed roof_area=2436326 outward
    "
    );
}

#[test]
fn a_geographic_crs_is_read_longitude_first_like_a_citygml_one_latitude_first() {
    let dir = TempDir::new();
    let tokyo_lat_lon = model(
        "tokyo",
        box_faces(2, [35.68, 139.76, 3.0], [35.6802, 139.7603, 23.0]),
    );
    let gml = dir.write("in.gml", &to_citygml("EPSG:4326", &[tokyo_lat_lon]));
    let lon_lat = |faces: Vec<Face>| {
        faces
            .into_iter()
            .map(|f| {
                let ring: Vec<[f64; 3]> = f.exterior.iter().map(|&[a, b, c]| [b, a, c]).collect();
                face(f.semantic, f.lod, &ring)
            })
            .collect()
    };
    let tokyo_lon_lat = model(
        "tokyo",
        lon_lat(box_faces(
            2,
            [35.68, 139.76, 3.0],
            [35.6802, 139.7603, 23.0],
        )),
    );
    let json = dir.write(
        "in.city.json",
        &to_cityjson(
            "https://www.opengis.net/def/crs/EPSG/0/4326",
            &[tokyo_lon_lat],
        ),
    );
    let (from_gml, from_json) = (dir.0.join("gml"), dir.0.join("json"));
    convert(&gml, &from_gml, &["--mlt-version", "2", "--max-zoom", "16"]);
    convert(
        &json,
        &from_json,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&from_json, 16), @"
    16/58210/25806 extent=32768
      id=0 gml_id=tokyo x=17418..19208 y=28432..29901 z=0..2000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=2629510 outward
    ");
    assert_eq!(tiles(&from_json, 16), tiles(&from_gml, 16));
}

#[test]
fn a_3d_bag_building_is_one_feature_of_its_parts_lod_2_2_with_the_buildings_attributes() {
    let dir = TempDir::new();
    let mut vertices = box_vertices(0, -500, 6000);
    vertices.extend(box_vertices(0, -500, 9500));
    let input = write_json(
        &dir,
        "delft.city.json",
        &delft(
            &json!({
                "NL.IMBAG.Pand.1": {
                    "type": "Building",
                    "attributes": {
                        "b3_h_maaiveld": -0.4,
                        "oorspronkelijkbouwjaar": 1931,
                        "status": "Pand in gebruik",
                        "b3_val3dity_lod22": [102, 203],
                        "address": { "street": "Markt" },
                        "b3_bouwlagen": null,
                    },
                    "children": ["NL.IMBAG.Pand.1-0"],
                    "geometry": [{ "type": "MultiSurface", "lod": "0", "boundaries": [[[0, 1, 2, 3]]] }],
                },
                "NL.IMBAG.Pand.1-0": {
                    "type": "BuildingPart",
                    "parents": ["NL.IMBAG.Pand.1"],
                    "attributes": { "ignored": true },
                    "geometry": [box_solid("1.2", 0), box_solid("2.2", 8)],
                },
            }),
            &Value::Array(vertices),
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &[
            "--mlt-version",
            "2",
            "--min-zoom",
            "15",
            "--max-zoom",
            "16",
            "--verify",
        ],
    );
    insta::assert_snapshot!(format!("{}{}", tiles(&output, 15), tiles(&output, 16)), @r#"
    15/16781/10822 extent=65536
      id=0 gml_id=NL.IMBAG.Pand.1 status=Pand in gebruik b3_h_maaiveld=F64(-0.4) oorspronkelijkbouwjaar=1931 b3_val3dity_lod22=[102,203] address={"street":"Markt"} x=1696..2576 y=47941..48825 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=756184 outward
    16/33562/21645 extent=32768
      id=0 gml_id=NL.IMBAG.Pand.1 status=Pand in gebruik b3_h_maaiveld=F64(-0.4) oorspronkelijkbouwjaar=1931 b3_val3dity_lod22=[102,203] address={"street":"Markt"} x=1696..2576 y=15173..16057 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=756184 outward
    "#);
    let bytes = fs::read(output.join("16/33562/21645.mlt")).unwrap();
    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .pop()
        .unwrap();
    let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    assert_eq!(layer.name(), "delft");
    assert_eq!(
        layer.property_names(),
        [
            "gml_id",
            "status",
            "b3_h_maaiveld",
            "oorspronkelijkbouwjaar",
            "b3_val3dity_lod22",
            "address"
        ]
    );
    assert_eq!(layer.m_value_names(), ["part"]);
}

#[test]
fn a_ground_attribute_sets_the_height_the_mesh_stands_on() {
    let dir = TempDir::new();
    let input = write_json(
        &dir,
        "delft.city.json",
        &delft(
            &json!({
                "pand": {
                    "type": "Building",
                    "attributes": { "b3_h_maaiveld": 1.5 },
                    "geometry": [box_solid("2.2", 0)],
                },
            }),
            &Value::Array(box_vertices(0, -500, 9500)),
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &[
            "--mlt-version",
            "2",
            "--max-zoom",
            "16",
            "--ground-attribute",
            "b3_h_maaiveld",
        ],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/33562/21645 extent=32768
      id=0 gml_id=pand b3_h_maaiveld=F64(1.5) x=1696..2576 y=15173..16057 z=-200..800 vertices=8 wall=8 roof=2 ground=2 closed roof_area=756184 outward
    ");
}

#[test]
fn lod_pins_the_one_lod_read_at_every_zoom() {
    let dir = TempDir::new();
    let mut vertices = box_vertices(0, 0, 6000);
    vertices.extend(box_vertices(0, 0, 9000));
    let input = write_json(
        &dir,
        "delft.city.json",
        &delft(
            &json!({ "pand": { "type": "Building", "geometry": [box_solid("1.2", 0), box_solid("2.2", 8)] } }),
            &Value::Array(vertices),
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &[
            "--mlt-version",
            "2",
            "--min-zoom",
            "15",
            "--max-zoom",
            "16",
            "--lod",
            "1.2",
        ],
    );
    insta::assert_snapshot!(format!("{}{}", tiles(&output, 15), tiles(&output, 16)), @"
    15/16781/10822 extent=65536
      id=0 gml_id=pand x=1696..2576 y=47941..48825 z=0..600 vertices=8 wall=8 roof=2 open roof_area=756184 outward
    16/33562/21645 extent=32768
      id=0 gml_id=pand x=1696..2576 y=15173..16057 z=0..600 vertices=8 wall=8 roof=2 ground=2 closed roof_area=756184 outward
    ");
    let empty = dir.0.join("empty");
    let out = mlt_convert(
        &input,
        &empty,
        &["--mlt-version", "2", "--max-zoom", "16", "--lod", "3"],
    );
    assert!(out.status.success());
    insta::assert_snapshot!(stderr_of(&dir, &out), @"
    note: no tiles were written (the input held no 3D buildings at LoD 3, or none reached the minimum pixel size at zooms 0..=16)
    [TMP]/delft.city.json -> [TMP]/empty: wrote 0 tile(s) holding 0 building(s) of 1 file(s) at zooms 0..=16
    ");
}

#[test]
fn a_solid_tiles_its_outer_shell_and_a_multi_solid_each_solid() {
    let dir = TempDir::new();
    let mut vertices = box_vertices(0, 0, 9000);
    vertices.extend(box_vertices(0, 2000, 4000));
    vertices.extend(box_vertices(20_000, 0, 3000));
    vertices.extend(box_vertices(35_000, 0, 6000));
    let mut void = box_shell(8);
    void.as_array_mut().unwrap().reverse();
    let input = write_json(
        &dir,
        "delft.city.json",
        &delft(
            &json!({
                "hollow": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2.2",
                        "boundaries": [box_shell(0), void],
                        "semantics": {
                            "surfaces": [{ "type": "GroundSurface" }, { "type": "RoofSurface" }, { "type": "WallSurface" }],
                            "values": [[0, 1, 2, 2, 2, 2], [2, 2, 2, 2, 2, 2]],
                        },
                    }],
                },
                "pair": {
                    "type": "Building",
                    "geometry": [{
                        "type": "MultiSolid",
                        "lod": "2.2",
                        "boundaries": [[box_shell(16)], [box_shell(24)]],
                        "semantics": {
                            "surfaces": [{ "type": "GroundSurface" }, { "type": "RoofSurface" }, { "type": "WallSurface" }],
                            "values": [[[0, 1, 2, 2, 2, 2]], null],
                        },
                    }],
                },
            }),
            &Value::Array(vertices),
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/33562/21645 extent=32768
      id=1 gml_id=pair x=3433..5617 y=15130..16032 z=0..600 vertices=16 wall=16 roof=4 ground=4 closed roof_area=1514098 outward
      id=0 gml_id=hollow x=1696..2576 y=15173..16057 z=0..900 vertices=8 wall=8 roof=2 ground=2 closed roof_area=756184 outward
    ");
}

#[test]
fn closure_surfaces_rooms_instances_and_objects_other_than_buildings_are_left_out() {
    let dir = TempDir::new();
    let mut vertices = box_vertices(0, 0, 9000);
    vertices.extend(box_vertices(1000, 0, 3000));
    vertices.extend([
        json!([30_000, 0, 0]),
        json!([40_000, 0, 0]),
        json!([40_000, 10_000, 0]),
    ]);
    let input = write_json(
        &dir,
        "delft.city.json",
        &delft(
            &json!({
                "group": { "type": "CityObjectGroup", "children": ["house"] },
                "house": {
                    "type": "Building",
                    "parents": ["group"],
                    "children": ["room"],
                    "geometry": [
                        {
                            "type": "MultiSurface",
                            "lod": "2.2",
                            "boundaries": [[[0, 1, 5, 4]], [[1, 2, 6, 5]]],
                            "semantics": {
                                "surfaces": [{ "type": "WallSurface" }, { "type": "ClosureSurface" }],
                                "values": [0, 1],
                            },
                        },
                        { "type": "GeometryInstance", "template": 0, "boundaries": [0], "transformationMatrix": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1] },
                    ],
                },
                "room": { "type": "BuildingRoom", "parents": ["house"], "geometry": [box_solid("2.2", 8)] },
                "terrain": {
                    "type": "TINRelief",
                    "geometry": [{ "type": "CompositeSurface", "lod": "1", "boundaries": [[[16, 17, 18]]] }],
                },
            }),
            &Value::Array(vertices),
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/33562/21645 extent=32768
      id=0 gml_id=house x=1708..2576 y=16044..16057 z=0..900 vertices=4 wall=2 open roof_area=0 outward
    ");
}

#[test]
fn a_cityjson_1_0_file_without_a_transform_and_with_numeric_lods_reads_alike() {
    let dir = TempDir::new();
    let transformed = delft(
        &json!({ "pand": { "type": "Building", "geometry": [box_solid("2.2", 0)] } }),
        &Value::Array(box_vertices(0, 0, 9000)),
    );
    let mut plain = transformed.clone();
    plain["version"] = json!("1.0");
    plain.as_object_mut().unwrap().remove("transform");
    plain["metadata"]["referenceSystem"] = json!("urn:ogc:def:crs:EPSG::7415");
    plain["CityObjects"]["pand"]["geometry"][0]["lod"] = json!(2.2);
    plain["vertices"] = transformed["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v: Vec<f64> = serde_json::from_value(v.clone()).unwrap();
            json!([
                84600.0 + v[0] / 1000.0,
                447_000.0 + v[1] / 1000.0,
                v[2] / 1000.0
            ])
        })
        .collect();
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    convert(
        &write_json(&dir, "a.city.json", &transformed),
        &a,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    convert(
        &write_json(&dir, "b.city.json", &plain),
        &b,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    assert_eq!(tiles(&b, 16), tiles(&a, 16));
}

#[test]
fn a_directory_of_citygml_and_cityjson_files_tiles_into_one_layer() {
    let dir = TempDir::new();
    let tower = model(
        "tower",
        box_faces(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5]),
    );
    let kiosk = model(
        "kiosk",
        box_faces(2, [E + 40.0, N + 10.0, 7.0], [E + 48.0, N + 18.0, 10.0]),
    );
    dir.write("hamburg/a.gml", &to_citygml("EPSG:25832", &[tower]));
    dir.write(
        "hamburg/b.city.json",
        &to_cityjson("https://www.opengis.net/def/crs/EPSG/0/25832", &[kiosk]),
    );
    dir.write("hamburg/notes.txt", "not a city model");
    let output = dir.0.join("out");
    let out = mlt_convert(
        &dir.0.join("hamburg"),
        &output,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    assert!(out.status.success(), "{}", stderr_of(&dir, &out));
    insta::assert_snapshot!(stderr_of(&dir, &out), @"[TMP]/hamburg -> [TMP]/out: wrote 17 tile(s) holding 2 building(s) of 2 file(s) at zooms 0..=16");
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 ground=2 closed roof_area=519940 outward
    ");
}

#[test]
fn a_cityjson_file_naming_the_wrong_utm_zone_is_read_in_that_of_the_others() {
    let dir = TempDir::new();
    let house = |id| {
        [model(
            id,
            box_faces(2, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0]),
        )]
    };
    dir.write("in/a.city.json", &to_cityjson("EPSG:25832", &house("a")));
    dir.write("in/b.city.json", &to_cityjson("EPSG:25832", &house("b")));
    dir.write("in/c.city.json", &to_cityjson("EPSG:25833", &house("c")));
    let output = dir.0.join("out");
    let out = mlt_convert(
        &dir.0.join("in"),
        &output,
        &[
            "--mlt-version",
            "2",
            "--max-zoom",
            "16",
            "--fix-mislabelled-crs",
        ],
    );
    assert!(out.status.success(), "{}", stderr_of(&dir, &out));
    insta::assert_snapshot!(stderr_of(&dir, &out), @"
    warning: [TMP]/in/c.city.json names EPSG:25833, but its coordinates lie near the other files in EPSG:25832, so it is read in that
    [TMP]/in -> [TMP]/out: wrote 5 tile(s) holding 3 building(s) of 3 file(s) at zooms 0..=16
    ");
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=a x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=1 gml_id=b x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=2 gml_id=c x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    ");
}

#[test]
fn a_broken_cityjson_file_fails_naming_what_is_wrong() {
    let dir = TempDir::new();
    let house = || {
        delft(
            &json!({ "house": { "type": "Building", "attributes": {}, "geometry": [box_solid("2.2", 0)] } }),
            &Value::Array(box_vertices(0, 0, 9000)),
        )
    };
    let mut cases: Vec<(&str, Value, &[&str])> = Vec::new();
    let mut feature = house();
    feature["type"] = json!("CityJSONFeature");
    cases.push(("of another type", feature, &[]));
    let mut far_vertex = house();
    far_vertex["CityObjects"]["house"]["geometry"][0]["boundaries"][0][0][0][0] = json!(99);
    cases.push(("naming a missing vertex", far_vertex, &[]));
    let mut far_semantic = house();
    far_semantic["CityObjects"]["house"]["geometry"][0]["semantics"]["values"] =
        json!([[0, 1, 2, 2, 2, 7]]);
    cases.push(("naming a missing semantic surface", far_semantic, &[]));
    let mut bad_lod = house();
    bad_lod["CityObjects"]["house"]["geometry"][0]["lod"] = json!("two");
    cases.push(("with an unreadable lod", bad_lod, &[]));
    let mut no_crs = house();
    no_crs.as_object_mut().unwrap().remove("metadata");
    cases.push(("without a CRS", no_crs, &[]));
    let mut local_crs = house();
    local_crs["metadata"]["referenceSystem"] = json!("urn:example:local");
    cases.push(("in a CRS without an EPSG code", local_crs, &[]));
    let mut reserved = house();
    reserved["CityObjects"]["house"]["attributes"] = json!({ "gml_id": "x" });
    cases.push(("with an attribute named gml_id", reserved, &[]));
    cases.push((
        "without the ground attribute",
        house(),
        &["--ground-attribute", "b3_h_maaiveld"],
    ));
    cases.push(("with an unreadable --lod", house(), &["--lod", "high"]));
    let errors: Vec<String> = cases
        .into_iter()
        .map(|(case, cityjson, args)| {
            let input = write_json(&dir, "house.city.json", &cityjson);
            let output = dir.0.join("out");
            let args = [&["--mlt-version", "2", "--max-zoom", "16"][..], args].concat();
            let out = mlt_convert(&input, &output, &args);
            assert!(!out.status.success(), "{case}");
            assert!(!output.exists(), "{case}");
            format!("{case}:\n{}", stderr_of(&dir, &out))
        })
        .collect();
    insta::assert_snapshot!(errors.join("---\n"), @r#"
    of another type:
    Error: reading [TMP]/house.city.json

    Caused by:
        the file is of type "CityJSONFeature", not CityJSON
    ---
    naming a missing vertex:
    Error: reading [TMP]/house.city.json

    Caused by:
        0: reading object house
        1: vertex 99 is out of range
    ---
    naming a missing semantic surface:
    Error: reading [TMP]/house.city.json

    Caused by:
        0: reading object house
        1: a surface names semantic surface 7, which its geometry lacks
    ---
    with an unreadable lod:
    Error: reading [TMP]/house.city.json

    Caused by:
        0: reading object house
        1: "two" is not a LoD such as 2 or 2.2
    ---
    without a CRS:
    Error: reading [TMP]/house.city.json

    Caused by:
        the file names no CRS in its referenceSystem
    ---
    in a CRS without an EPSG code:
    Error: reading [TMP]/house.city.json

    Caused by:
        the referenceSystem "urn:example:local" names no EPSG code
    ---
    with an attribute named gml_id:
    Error: object house has an attribute named gml_id, which convert writes a column of its own under
    ---
    without the ground attribute:
    Error: reading [TMP]/house.city.json

    Caused by:
        object house has no numeric b3_h_maaiveld attribute
    ---
    with an unreadable --lod:
    Error: reading --lod

    Caused by:
        "high" is not a LoD such as 2 or 2.2
    "#);
}

#[test]
fn a_cityjson_input_rejects_flags_it_cannot_honour() {
    let dir = TempDir::new();
    let input = dir.write(
        "hamburg.city.json",
        &to_cityjson(
            "EPSG:25832",
            &[model(
                "tower",
                box_faces(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5]),
            )],
        ),
    );
    let output = dir.0.join("out");
    let archive = dir.0.join("out.pmtiles");
    let errors: Vec<String> = [
        (&output, &["--max-zoom", "16"][..]),
        (&output, &["--mlt-version", "2"][..]),
        (
            &output,
            &["--mlt-version", "2", "--max-zoom", "16", "--tessellate"][..],
        ),
        (
            &output,
            &[
                "--mlt-version",
                "2",
                "--max-zoom",
                "16",
                "--height-mode",
                "absolute",
            ][..],
        ),
        (&archive, &["--mlt-version", "2", "--max-zoom", "16"][..]),
    ]
    .iter()
    .map(|(output, args)| {
        let out = mlt_convert(&input, output, args);
        assert!(!out.status.success());
        stderr_of(&dir, &out)
    })
    .collect();
    assert!(!output.exists() && !archive.exists());
    insta::assert_snapshot!(errors.join("---\n"), @"
    Error: tiling a CityJSON input writes MLT version 2, so it needs --mlt-version 2
    ---
    Error: tiling a CityJSON input requires --max-zoom
    ---
    Error: --tessellate cuts 2D polygons, so it does not apply to a CityJSON mesh input
    ---
    Error: a CityJSON input stores heights above each building's ground, so it takes no --height-mode
    ---
    Error: a CityJSON input is tiled into a directory of z/x/y.mlt tiles; archive output is not supported yet, got: [TMP]/out.pmtiles
    ");
}

#[test]
fn city_model_flags_are_rejected_for_other_inputs() {
    let dir = TempDir::new();
    let input = dir.write(
        "points.geojson",
        r#"{"type":"FeatureCollection","features":[]}"#,
    );
    let out = mlt_convert(
        &input,
        &dir.0.join("out"),
        &["--mlt-version", "2", "--max-zoom", "4", "--lod", "2"],
    );
    assert!(!out.status.success());
    insta::assert_snapshot!(stderr_of(&dir, &out), @"Error: --lod tiles a CityGML or CityJSON input, but the input is neither a .gml, .xml or .city.json file nor a directory of them: [TMP]/points.geojson");
}
