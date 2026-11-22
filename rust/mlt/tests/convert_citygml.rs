//! End-to-end tests of `mlt convert` tiling `CityGML` files into directories of v2 `Meshes` tiles.

#![cfg(not(feature = "hotpath"))]

mod city;

use std::fs;

use mlt_core::geo_types::Geometry;
use mlt_core::{Decoder, HeightMode, MValue, Parser, ZAxis, ZStep};

use self::city::*;

#[test]
fn each_building_becomes_one_meshes_feature_per_zoom_with_heights_above_its_ground() {
    let dir = TempDir::new();
    let input = dir.write(
        "hamburg.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "tower",
                    &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
                ),
                building(
                    "kiosk",
                    &[surfaces(
                        2,
                        &cuboid_faces([E + 40.0, N + 10.0, 7.0], [E + 48.0, N + 18.0, 10.0])[..5],
                    )],
                ),
            ],
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
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=519940 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=519940 outward
    ");
}

#[test]
fn a_tile_holds_one_meshes_layer_named_after_the_input_with_centimetre_heights_above_ground() {
    let dir = TempDir::new();
    let input = dir.write(
        "hamburg.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "tower",
                &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    let bytes = fs::read(output.join("16/34588/21180.mlt")).unwrap();
    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .pop()
        .unwrap();
    let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    assert_eq!(layer.name(), "hamburg");
    assert!(layer.is_meshes());
    assert_eq!(
        layer.z_axis(),
        Some(ZAxis {
            step: ZStep::new(-2).unwrap(),
            mode: HeightMode::RelativeToGround,
        })
    );
    assert_eq!(layer.property_names(), ["gml_id"]);
    assert_eq!(layer.m_value_names(), ["part"]);
}

#[test]
fn lower_zooms_keep_3d_meshes_of_tall_buildings_and_merge_small_ones_from_three_zooms_below() {
    let dir = TempDir::new();
    let input = dir.write(
        "hamburg.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "tower",
                    &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
                ),
                building(
                    "kiosk",
                    &[surfaces(
                        2,
                        &cuboid_faces([E + 40.0, N + 10.0, 7.0], [E + 48.0, N + 18.0, 10.0])[..5],
                    )],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 14), @"
    14/8647/5295 extent=131072
      id=1 gml_id=kiosk x=4588..5318 y=15883..16615 z=0..300 vertices=8 wall=8 roof=2 open roof_area=534360 outward
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    ");
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 gml_id=tower x=132046..133872 y=146724..148554 z=0..6000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
      id=1 buildings=1 x=135680..136704 y=146944..147968 z=300..300 vertices=4 roof=2 open roof_area=786432 up
    ");
}

#[test]
fn a_polygon_given_twice_is_welded_into_one_set_of_triangles() {
    let dir = TempDir::new();
    let roof = polygon(
        &[
            [E, N, 10.0],
            [E + 20.0, N, 10.0],
            [E + 20.0, N + 20.0, 10.0],
            [E, N + 20.0, 10.0],
        ],
        &[],
    );
    let input = dir.write(
        "twice.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    surfaces(
                        2,
                        &cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])[..4],
                    ),
                    surface("RoofSurface", 2, &[roof.clone(), roof]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    ");
}

#[test]
fn an_inside_out_closed_box_is_turned_outward() {
    let dir = TempDir::new();
    let inside_out: Vec<(&str, Vec<[f64; 3]>)> =
        cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])
            .into_iter()
            .map(|(semantic, ring)| (semantic, ring.into_iter().rev().collect()))
            .collect();
    let input = dir.write(
        "inside_out.gml",
        &city_model(
            "EPSG:25832",
            &[building("box", &[surfaces(2, &inside_out)])],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=box x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    ");
}

#[test]
fn an_inside_out_open_box_keeps_its_winding() {
    let dir = TempDir::new();
    let inside_out_without_ground: Vec<(&str, Vec<[f64; 3]>)> =
        cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])
            .into_iter()
            .filter(|(semantic, _)| *semantic != "GroundSurface")
            .map(|(semantic, ring)| (semantic, ring.into_iter().rev().collect()))
            .collect();
    let input = dir.write(
        "inside_out.gml",
        &city_model(
            "EPSG:25832",
            &[building("box", &[surfaces(2, &inside_out_without_ground)])],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=box x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 inward
    ");
}

#[test]
fn a_tower_stands_on_its_lowest_ground_ring_rather_than_an_elevated_underside_over_its_centre() {
    let dir = TempDir::new();
    let underside = polygon(
        &[
            [E + 9.0, N + 9.0, 225.5],
            [E + 9.0, N + 11.0, 225.5],
            [E + 11.0, N + 11.0, 225.5],
            [E + 11.0, N + 9.0, 225.5],
        ],
        &[],
    );
    let input = dir.write(
        "tower.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "tower",
                &[
                    cuboid(2, [E, N, 20.5], [E + 20.0, N + 20.0, 300.4]),
                    surface("GroundSurface", 2, &[underside]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=tower x=974..2800 y=15652..17482 z=0..27990 vertices=12 wall=8 roof=2 ground=4 open roof_area=3249652 outward
    ");
}

#[test]
fn a_terrain_intersection_sets_the_ground_of_a_building() {
    let dir = TempDir::new();
    let terrain = format!(
        "<bldg:lod2TerrainIntersection><gml:MultiCurve><gml:curveMember><gml:LineString>\
         <gml:posList srsDimension=\"3\">{} 20.674 {} 20.674</gml:posList></gml:LineString>\
         </gml:curveMember></gml:MultiCurve></bldg:lod2TerrainIntersection>",
        format_args!("{E} {N}"),
        format_args!("{} {N}", E + 20.0),
    );
    let input = dir.write(
        "terrain.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[cuboid(2, [E, N, 20.0], [E + 20.0, N + 20.0, 60.0]), terrain],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=-67..3933 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    ");
}

#[test]
fn two_zooms_below_a_small_building_becomes_a_block_of_its_ground_outline() {
    let dir = TempDir::new();
    let ground = [
        [E, N, 0.0],
        [E, N + 10.0, 0.0],
        [E + 10.0, N + 10.0, 0.0],
        [E + 10.0, N, 0.0],
    ];
    let reversed: Vec<[f64; 3]> = ground.iter().rev().copied().collect();
    let input = dir.write(
        "block.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    surfaces(
                        2,
                        &cuboid_faces([E, N, 0.0], [E + 10.0, N + 10.0, 10.0])[..5],
                    ),
                    surface(
                        "GroundSurface",
                        2,
                        &[polygon(&ground, &[]), polygon(&reversed, &[])],
                    ),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "14", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 14), @"
    14/8647/5295 extent=131072
      id=0 gml_id=house x=974..1887 y=16554..17469 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=812413 outward
    ");
}

#[test]
fn two_zooms_below_a_building_without_ground_becomes_its_bounding_box_and_one_under_two_and_a_half_pixels_is_left_out()
 {
    let dir = TempDir::new();
    let input = dir.write(
        "blocks.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "shed",
                    &[surfaces(
                        2,
                        &cuboid_faces([E, N, 0.0], [E + 10.0, N + 10.0, 5.0])[..5],
                    )],
                ),
                building(
                    "hut",
                    &[surfaces(
                        2,
                        &cuboid_faces([E + 30.0, N, 0.0], [E + 35.0, N + 5.0, 3.0])[..5],
                    )],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "14", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 14), @"
    14/8647/5295 extent=131072
      id=0 gml_id=shed x=974..1887 y=16554..17469 z=0..500 vertices=8 wall=8 roof=2 open roof_area=835395 outward
    ");
}

#[test]
fn a_file_naming_the_wrong_utm_zone_is_read_in_that_of_the_others() {
    let dir = TempDir::new();
    let hamburg = |srs_name: &str, gml_id: &str| {
        city_model(
            srs_name,
            &[building(
                gml_id,
                &[cuboid(2, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0])],
            )],
        )
    };
    dir.write("in/a.gml", &hamburg("EPSG:25832", "a"));
    dir.write("in/b.gml", &hamburg("EPSG:25832", "b"));
    dir.write("in/c.gml", &hamburg("EPSG:25833", "c"));
    dir.write(
        "in/d.gml",
        &city_model(
            "EPSG:25833",
            &[building(
                "berlin",
                &[cuboid(
                    2,
                    [390_000.0, 5_820_000.0, 0.0],
                    [390_020.0, 5_820_020.0, 10.0],
                )],
            )],
        ),
    );
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
    assert!(out.status.success());
    insta::assert_snapshot!(stderr_of(&dir, &out), @"
    warning: [TMP]/in/c.gml names EPSG:25833, but its coordinates lie near the other files in EPSG:25832, so it is read in that
    [TMP]/in -> [TMP]/out: wrote 10 tile(s) holding 4 building(s) of 4 file(s) at zooms 0..=16
    ");
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=a x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=1 gml_id=b x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
      id=2 gml_id=c x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    16/35203/21493 extent=32768
      id=3 gml_id=berlin x=17601..19398 y=3769..5570 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3098255 outward
    ");
}

#[test]
fn a_building_keeps_its_highest_lod_with_its_holes_and_leaves_out_closure_surfaces() {
    let dir = TempDir::new();
    let square = [
        [E, N, 10.0],
        [E + 20.0, N, 10.0],
        [E + 20.0, N + 20.0, 10.0],
        [E, N + 20.0, 10.0],
    ];
    let hole = [
        [E + 4.0, N + 4.0, 10.0],
        [E + 4.0, N + 8.0, 10.0],
        [E + 8.0, N + 4.0, 10.0],
    ];
    let input = dir.write(
        "lods.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "roof",
                &[
                    surface("RoofSurface", 1, &[polygon(&square, &[])]),
                    surface("RoofSurface", 2, &[polygon(&square, &[&hole])]),
                    surface("ClosureSurface", 2, &[polygon(&square, &[])]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=roof x=974..2800 y=15652..17482 z=0..0 vertices=7 roof=7 open roof_area=3184660 up
    ");
}

#[test]
fn each_building_part_picks_its_own_highest_lod() {
    let dir = TempDir::new();
    let part = format!(
        "<bldg:consistsOfBuildingPart><bldg:BuildingPart>{}</bldg:BuildingPart></bldg:consistsOfBuildingPart>",
        surfaces(
            2,
            &cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])[5..]
        ),
    );
    let walls = surfaces(
        3,
        &cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])[..4],
    );
    let input = dir.write(
        "parts.gml",
        &city_model(
            "EPSG:25832",
            &[format!(
                "<core:cityObjectMember><bldg:Building>{walls}{part}</bldg:Building></core:cityObjectMember>"
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 ground=2 open roof_area=0 outward
    ");
}

#[test]
fn lod2_closes_the_holes_of_lod3_walls_and_leaves_out_windows_and_installations() {
    let dir = TempDir::new();
    let wall = [
        [E, N, 0.0],
        [E + 20.0, N, 0.0],
        [E + 20.0, N, 10.0],
        [E, N, 10.0],
    ];
    let window = [
        [E + 5.0, N, 3.0],
        [E + 8.0, N, 3.0],
        [E + 8.0, N, 6.0],
        [E + 5.0, N, 6.0],
    ];
    let hole: Vec<[f64; 3]> = window.iter().rev().copied().collect();
    let opening = format!(
        "<bldg:opening><bldg:Window><bldg:lod3MultiSurface><gml:MultiSurface><gml:surfaceMember>{}\
         </gml:surfaceMember></gml:MultiSurface></bldg:lod3MultiSurface></bldg:Window></bldg:opening>",
        polygon(&window, &[]),
    );
    let walled = format!(
        "<bldg:boundedBy><bldg:WallSurface><bldg:lod3MultiSurface><gml:MultiSurface><gml:surfaceMember>{}\
         </gml:surfaceMember></gml:MultiSurface></bldg:lod3MultiSurface>{opening}</bldg:WallSurface></bldg:boundedBy>",
        polygon(&wall, &[&hole]),
    );
    let installation = format!(
        "<bldg:outerBuildingInstallation><bldg:BuildingInstallation>{}</bldg:BuildingInstallation></bldg:outerBuildingInstallation>",
        cuboid(3, [E + 22.0, N, 0.0], [E + 30.0, N + 8.0, 3.0]),
    );
    let input = dir.write(
        "lod3.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    walled,
                    surfaces(
                        3,
                        &cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])[1..],
                    ),
                    installation,
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "15", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..3685 y=15652..17494 z=0..1000 vertices=20 wall=22 roof=4 ground=4 window=2 closed roof_area=3769592 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    ");
}

#[test]
fn walls_roofs_and_grounds_each_pick_their_own_highest_lod_and_lod2() {
    let dir = TempDir::new();
    let faces = cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let window = [
        [E + 5.0, N, 3.0],
        [E + 8.0, N, 3.0],
        [E + 8.0, N, 6.0],
        [E + 5.0, N, 6.0],
    ];
    let hole: Vec<[f64; 3]> = window.iter().rev().copied().collect();
    let input = dir.write(
        "lods.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    surfaces(1, &faces[..4]),
                    surface("WallSurface", 3, &[polygon(&faces[0].1, &[&hole])]),
                    surfaces(3, &faces[1..4]),
                    surfaces(2, &faces[4..5]),
                    surfaces(1, &faces[5..]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "15", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=12 wall=14 roof=2 ground=2 open roof_area=3249652 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    ");
}

#[test]
fn footprints_and_lower_surfaces_without_semantics_leave_semantic_surfaces_alone() {
    let dir = TempDir::new();
    let faces = cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let footprint = format!(
        "<bldg:lod0FootPrint><gml:MultiSurface><gml:surfaceMember>{}</gml:surfaceMember>\
         </gml:MultiSurface></bldg:lod0FootPrint>",
        polygon(&faces[5].1, &[]),
    );
    let solid = format!(
        "<bldg:lod1MultiSurface><gml:MultiSurface><gml:surfaceMember>{}</gml:surfaceMember>\
         </gml:MultiSurface></bldg:lod1MultiSurface>",
        polygon(&faces[4].1, &[]),
    );
    let input = dir.write(
        "lods.gml",
        &city_model(
            "EPSG:25832",
            &[building("house", &[footprint, solid, surfaces(2, &faces)])],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    ");
}

#[test]
fn an_orientable_surface_of_negative_orientation_reverses_its_polygon() {
    let dir = TempDir::new();
    let faces = cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0]);
    let downward_roof: Vec<[f64; 3]> = faces[4].1.iter().rev().copied().collect();
    let reversed_roof = format!(
        "<bldg:boundedBy><bldg:RoofSurface><bldg:lod2MultiSurface><gml:MultiSurface><gml:surfaceMember>\
         <gml:OrientableSurface orientation=\"-\"><gml:baseSurface>{}</gml:baseSurface></gml:OrientableSurface>\
         </gml:surfaceMember></gml:MultiSurface></bldg:lod2MultiSurface></bldg:RoofSurface></bldg:boundedBy>",
        polygon(&downward_roof, &[]),
    );
    let input = dir.write(
        "orientable.gml",
        &city_model(
            "EPSG:25832",
            &[building("slab", &[reversed_roof, surfaces(2, &faces[5..])])],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=slab x=974..2800 y=15652..17482 z=0..1000 vertices=8 roof=2 ground=2 open roof_area=3249652 outward
    ");
}

#[test]
fn lod2_keeps_courtyards_in_roofs_and_grounds_while_closing_skylights() {
    let dir = TempDir::new();
    let faces = cuboid_faces([E, N, 0.0], [E + 30.0, N + 30.0, 10.0]);
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
    let window = format!(
        "<bldg:opening><bldg:Window><bldg:lod3MultiSurface><gml:MultiSurface><gml:surfaceMember>{}\
         </gml:surfaceMember></gml:MultiSurface></bldg:lod3MultiSurface></bldg:Window></bldg:opening>",
        polygon(&skylight, &[]),
    );
    let roof = format!(
        "<bldg:boundedBy><bldg:RoofSurface><bldg:lod3MultiSurface><gml:MultiSurface><gml:surfaceMember>{}\
         </gml:surfaceMember></gml:MultiSurface></bldg:lod3MultiSurface>{window}</bldg:RoofSurface></bldg:boundedBy>",
        polygon(&faces[4].1, &[&courtyard_hole, &skylight_hole]),
    );
    let input = dir.write(
        "courtyard.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    surfaces(3, &faces[..4]),
                    roof,
                    surface(
                        "GroundSurface",
                        3,
                        &[polygon(&faces[5].1, &[&courtyard_ground])],
                    ),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "15", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..3713 y=14749..17494 z=0..1000 vertices=19 wall=8 roof=13 ground=8 window=1 open roof_area=6463735 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..3713 y=14749..17494 z=0..1000 vertices=12 wall=8 roof=8 open roof_area=6500193 outward
    ");
}

#[test]
fn polygons_without_area_unreadable_coordinates_and_2d_rings_are_left_out() {
    let dir = TempDir::new();
    let line = format!(
        "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>{E} {N} 0 {} {N} 0 {E} {N} 0</gml:posList>\
         </gml:LinearRing></gml:exterior></gml:Polygon>",
        E + 10.0
    );
    let not_a_number = format!(
        "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>{E} {N} 0 {} {N} NaN {E} {} 0 {E} {N} 0</gml:posList>\
         </gml:LinearRing></gml:exterior></gml:Polygon>",
        E + 10.0,
        N + 10.0
    );
    let shifted = format!(
        "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>{E} {N} 0 x {N} 0 {E} {} 0 {E} {N} 0</gml:posList>\
         </gml:LinearRing></gml:exterior></gml:Polygon>",
        N + 10.0
    );
    let flat = format!(
        "<gml:Polygon srsDimension=\"2\"><gml:exterior><gml:LinearRing><gml:posList>{E} {N} {} {N} {E} {} {E} {N}</gml:posList>\
         </gml:LinearRing></gml:exterior></gml:Polygon>",
        E + 10.0,
        N + 10.0
    );
    let input = dir.write(
        "broken.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "nothing",
                &[surface(
                    "WallSurface",
                    2,
                    &[line, not_a_number, shifted, flat],
                )],
            )],
        ),
    );
    let output = dir.0.join("out");
    let out = mlt_convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    assert!(out.status.success());
    insta::assert_snapshot!(stderr_of(&dir, &out), @"
    note: no tiles were written (the input held no 3D buildings, or none reached the minimum pixel size at zooms 0..=16)
    [TMP]/broken.gml -> [TMP]/out: wrote 0 tile(s) holding 0 building(s) of 1 file(s) at zooms 0..=16
    ");
}

#[test]
fn coordinates_may_be_separated_by_commas() {
    let dir = TempDir::new();
    let ring = [
        [E, N, 10.0],
        [E + 20.0, N, 10.0],
        [E + 20.0, N + 20.0, 10.0],
        [E, N + 20.0, 10.0],
        [E, N, 10.0],
    ];
    let coordinates: Vec<String> = ring
        .iter()
        .map(|[x, y, z]| format!("{x},{y},{z}"))
        .collect();
    let roof = format!(
        "<gml:Polygon><gml:outerBoundaryIs><gml:LinearRing><gml:coordinates>{}</gml:coordinates>\
         </gml:LinearRing></gml:outerBoundaryIs></gml:Polygon>",
        coordinates.join(" ")
    );
    let input = dir.write(
        "commas.gml",
        &city_model(
            "EPSG:25832",
            &[building("roof", &[surface("RoofSurface", 2, &[roof])])],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=roof x=974..2800 y=15652..17482 z=0..0 vertices=4 roof=2 open roof_area=3249652 up
    ");
}

#[test]
fn broken_xml_fails_with_its_position() {
    let dir = TempDir::new();
    let unclosed = dir.write("unclosed.gml", "<a><b></a>");
    let mismatched = dir.write("mismatched.gml", "<a><b></c></a>");
    let output = dir.0.join("out");
    let errors: Vec<String> = [unclosed, mismatched]
        .iter()
        .map(|input| {
            let out = mlt_convert(input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
            assert!(!out.status.success());
            stderr_of(&dir, &out)
        })
        .collect();
    insta::assert_snapshot!(errors.join("---\n"), @"
    Error: reading [TMP]/unclosed.gml

    Caused by:
        invalid XML at byte 10: ill-formed document: expected `</b>`, but `</a>` was found
    ---
    Error: reading [TMP]/mismatched.gml

    Caused by:
        invalid XML at byte 10: ill-formed document: expected `</b>`, but `</c>` was found
    ");
}

#[test]
fn every_spelling_of_an_epsg_code_reads_alike() {
    let dir = TempDir::new();
    let tower = building(
        "tower",
        &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
    );
    let input = dir.write(
        "plain.gml",
        &city_model("EPSG:25832", std::slice::from_ref(&tower)),
    );
    let plain = dir.0.join("plain");
    convert(&input, &plain, &["--mlt-version", "2", "--max-zoom", "16"]);
    for srs_name in [
        "urn:ogc:def:crs:EPSG::25832",
        "http://www.opengis.net/def/crs/EPSG/0/25832",
        "urn:ogc:def:crs,crs:EPSG::25832,crs:EPSG::7837",
        "http://www.opengis.net/def/crs-compound?1=http://www.opengis.net/def/crs/EPSG/0/25832&amp;2=http://www.opengis.net/def/crs/EPSG/0/7837",
        "EPSG:5555",
        "urn:adv:crs:ETRS89_UTM32*DE_DHHN2016_NH",
    ] {
        let input = dir.write(
            "spelled.gml",
            &city_model(srs_name, std::slice::from_ref(&tower)),
        );
        let spelled = dir.0.join("spelled");
        convert(
            &input,
            &spelled,
            &["--mlt-version", "2", "--max-zoom", "16"],
        );
        assert_eq!(tiles(&spelled, 16), tiles(&plain, 16), "{srs_name}");
        fs::remove_dir_all(&spelled).unwrap();
    }
}

#[test]
fn an_srs_name_without_an_epsg_code_or_of_an_unknown_one_is_rejected() {
    let dir = TempDir::new();
    let tower = building(
        "tower",
        &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
    );
    let output = dir.0.join("out");
    let errors: Vec<String> = [
        "urn:example:local",
        "EPSG:",
        "urn:adv:crs:ETRS89_UTM60000",
        "urn:adv:crs:ETRS89_UTM0",
        "EPSG:1",
    ]
    .iter()
    .map(|srs_name| {
        let input = dir.write(
            "local.gml",
            &city_model(srs_name, std::slice::from_ref(&tower)),
        );
        let out = mlt_convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
        assert!(!out.status.success());
        stderr_of(&dir, &out)
    })
    .collect();
    assert!(!output.exists());
    insta::assert_snapshot!(errors.join("---\n"), @r#"
    Error: reading [TMP]/local.gml

    Caused by:
        the srsName "urn:example:local" names no EPSG code
    ---
    Error: reading [TMP]/local.gml

    Caused by:
        the srsName "EPSG:" names no EPSG code
    ---
    Error: reading [TMP]/local.gml

    Caused by:
        the srsName "urn:adv:crs:ETRS89_UTM60000" names no EPSG code
    ---
    Error: reading [TMP]/local.gml

    Caused by:
        the srsName "urn:adv:crs:ETRS89_UTM0" names no EPSG code
    ---
    Error: reading [TMP]/local.gml

    Caused by:
        EPSG:1 is not a CRS this build knows: Projection not found
    "#);
}

#[test]
fn a_geographic_crs_is_read_latitude_first() {
    let dir = TempDir::new();
    let tokyo = building(
        "tokyo",
        &[cuboid(2, [35.68, 139.76, 3.0], [35.6802, 139.7603, 23.0])],
    );
    let jgd2011 = dir.write(
        "jgd2011.gml",
        &city_model(
            "http://www.opengis.net/def/crs/EPSG/0/6697",
            std::slice::from_ref(&tokyo),
        ),
    );
    let wgs84 = dir.write(
        "wgs84.gml",
        &city_model("EPSG:4326", std::slice::from_ref(&tokyo)),
    );
    let from_jgd2011 = dir.0.join("jgd2011");
    let from_wgs84 = dir.0.join("wgs84");
    convert(
        &jgd2011,
        &from_jgd2011,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    convert(
        &wgs84,
        &from_wgs84,
        &["--mlt-version", "2", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&from_jgd2011, 16), @"
    16/58210/25806 extent=32768
      id=0 gml_id=tokyo x=17418..19208 y=28432..29901 z=0..2000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=2629510 outward
    ");
    assert_eq!(tiles(&from_wgs84, 16), tiles(&from_jgd2011, 16));
}

#[test]
fn heights_in_us_survey_feet_are_scaled_to_metres() {
    let dir = TempDir::new();
    let input = dir.write(
        "new_york.gml",
        &city_model(
            "EPSG:2263",
            &[building(
                "tower",
                &[cuboid(
                    2,
                    [987_000.0, 212_000.0, 0.0],
                    [987_050.0, 212_050.0, 100.0],
                )],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/19298/24631 extent=32768
      id=0 gml_id=tower x=16970..18047 y=21371..22452 z=0..3048 vertices=8 wall=8 roof=2 ground=2 closed roof_area=1163696 outward
    ");
}

#[test]
fn a_flat_roof_cut_into_cells_is_simplified_to_its_outline() {
    let dir = TempDir::new();
    let cells: Vec<String> = (0..16)
        .map(|c| {
            let (x, y) = (E + f64::from(c % 4) * 5.0, N + f64::from(c / 4) * 5.0);
            polygon(
                &[
                    [x, y, 10.0],
                    [x + 5.0, y, 10.0],
                    [x + 5.0, y + 5.0, 10.0],
                    [x, y + 5.0, 10.0],
                ],
                &[],
            )
        })
        .collect();
    let input = dir.write(
        "cells.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    surfaces(
                        2,
                        &cuboid_faces([E, N, 0.0], [E + 20.0, N + 20.0, 10.0])[..4],
                    ),
                    surface("RoofSurface", 2, &cells),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3249652 outward
    ");
}

#[test]
fn the_seam_between_two_parts_keeps_the_area_of_each() {
    let dir = TempDir::new();
    let cell = |c: u32| {
        let (x, y) = (
            1_110_000.0 + f64::from(c % 4) * 5.0,
            7_100_000.0 + f64::from(c / 4) * 5.0,
        );
        polygon(
            &[
                [x, y, 10.0],
                [x + 5.0, y, 10.0],
                [x + 5.0, y + 5.0, 10.0],
                [x, y + 5.0, 10.0],
            ],
            &[],
        )
    };
    let west: Vec<String> = (0..16).filter(|c| c % 4 < 2).map(cell).collect();
    let east: Vec<String> = (0..16).filter(|c| c % 4 >= 2).map(cell).collect();
    let input = dir.write(
        "seam.gml",
        &city_model(
            "EPSG:3857",
            &[building(
                "slab",
                &[
                    surface("WallSurface", 2, &west),
                    surface("RoofSurface", 2, &east),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(&input, &output, &["--mlt-version", "2", "--max-zoom", "16"]);
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34583/21157 extent=32768
      id=0 gml_id=slab x=7199..8271 y=3359..4431 z=0..0 vertices=6 wall=2 roof=2 open roof_area=574592 up
    ");
    let bytes = fs::read(output.join("16/34583/21157.mlt")).unwrap();
    let layer = Parser::default()
        .parse_layers(&bytes)
        .unwrap()
        .pop()
        .unwrap();
    let layer = layer.into_tile(&mut Decoder::default()).unwrap().unwrap();
    let [slab] = layer.features() else {
        panic!("{:?}", layer.features());
    };
    let Geometry::MultiPoint(points) = slab.geometry() else {
        panic!("{:?}", slab.geometry());
    };
    let MValue::Str(Some(parts)) = &slab.m_values()[0] else {
        panic!("{:?}", slab.m_values());
    };
    let corners = |part: &str| -> Vec<[i64; 2]> {
        slab.triangles()
            .unwrap()
            .iter()
            .zip(parts)
            .filter(|(_, p)| *p == part)
            .flat_map(|(t, _)| {
                t.map(|v| {
                    [
                        i64::from(points.0[v as usize].x()),
                        i64::from(points.0[v as usize].y()),
                    ]
                })
            })
            .collect()
    };
    let doubled_area = |part: &str| -> i64 {
        corners(part)
            .chunks(3)
            .map(|t| {
                ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1])
                    - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]))
                    .abs()
            })
            .sum()
    };
    let bounds = |part: &str| -> [i64; 4] {
        let corners = corners(part);
        [
            corners.iter().map(|c| c[0]).min().unwrap(),
            corners.iter().map(|c| c[0]).max().unwrap(),
            corners.iter().map(|c| c[1]).min().unwrap(),
            corners.iter().map(|c| c[1]).max().unwrap(),
        ]
    };
    let [wall_west, wall_east, wall_north, wall_south] = bounds("wall");
    let [roof_west, roof_east, roof_north, roof_south] = bounds("roof");
    assert_eq!(wall_east, roof_west);
    assert_eq!(
        doubled_area("wall"),
        2 * (wall_east - wall_west) * (wall_south - wall_north)
    );
    assert_eq!(
        doubled_area("roof"),
        2 * (roof_east - roof_west) * (roof_south - roof_north)
    );
}

#[test]
fn a_detached_sliver_is_pruned_at_the_highest_zoom_only() {
    let dir = TempDir::new();
    let sliver = polygon(
        &[
            [E + 30.0, N, 10.0],
            [E + 30.01, N, 10.0],
            [E + 30.0, N + 0.01, 10.0],
        ],
        &[],
    );
    let input = dir.write(
        "sliver.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "house",
                &[
                    cuboid(2, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0]),
                    surface("RoofSurface", 2, &[sliver]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "15", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=house x=974..2800 y=15652..17482 z=0..1000 vertices=8 wall=8 roof=2 ground=2 closed roof_area=3249652 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=house x=974..3676 y=15652..17494 z=0..1000 vertices=11 wall=8 roof=3 open roof_area=3249653 outward
    ");
}

#[test]
fn a_steep_spire_stays_a_roof_down_the_zooms() {
    let dir = TempDir::new();
    let apex = [E + 4.0, N + 4.0, 65.0];
    let eaves = [
        [E, N, 40.0],
        [E + 8.0, N, 40.0],
        [E + 8.0, N + 8.0, 40.0],
        [E, N + 8.0, 40.0],
    ];
    let spire: Vec<String> = (0..4)
        .map(|i| polygon(&[eaves[i], eaves[(i + 1) % 4], apex], &[]))
        .collect();
    let input = dir.write(
        "spire.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "church",
                &[
                    surfaces(2, &cuboid_faces([E, N, 0.0], [E + 8.0, N + 8.0, 40.0])[..4]),
                    surface("RoofSurface", 2, &spire),
                    surfaces(2, &cuboid_faces([E, N, 0.0], [E + 8.0, N + 8.0, 40.0])[5..]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "14", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 16), @"
    16/34588/21180 extent=32768
      id=0 gml_id=church x=974..1705 y=16735..17467 z=0..6500 vertices=9 wall=8 roof=4 ground=2 closed roof_area=519951 outward
    ");
    insta::assert_snapshot!(tiles(&output, 15), @"
    15/17294/10590 extent=65536
      id=0 gml_id=church x=974..1705 y=16735..17467 z=0..6500 vertices=9 wall=8 roof=4 open roof_area=519951 outward
    ");
    insta::assert_snapshot!(tiles(&output, 14), @"
    14/8647/5295 extent=131072
      id=0 gml_id=church x=974..1705 y=16735..17467 z=0..6500 vertices=9 wall=8 roof=4 open roof_area=519951 outward
    ");
}

#[test]
fn touching_buildings_of_one_height_band_merge_into_one_block_at_their_mean_top() {
    let dir = TempDir::new();
    let input = dir.write(
        "terrace.gml",
        &city_model(
            "EPSG:25832",
            &[
                building("six", &[cuboid(2, [E, N, 0.0], [E + 40.0, N + 20.0, 6.0])]),
                building(
                    "eight",
                    &[cuboid(2, [E + 40.0, N, 0.0], [E + 80.0, N + 20.0, 8.0])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=2 x=132096..139264 y=146432..148992 z=0..703 vertices=14 wall=14 roof=5 open roof_area=15335424 outward
    ");
}

#[test]
fn a_taller_band_stays_its_own_block_walled_only_above_its_neighbour() {
    let dir = TempDir::new();
    let input = dir.write(
        "bands.gml",
        &city_model(
            "EPSG:25832",
            &[
                building("low", &[cuboid(2, [E, N, 0.0], [E + 40.0, N + 20.0, 6.0])]),
                building(
                    "tall",
                    &[cuboid(2, [E + 40.0, N, 0.0], [E + 80.0, N + 20.0, 25.0])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=1 x=132096..135680 y=146432..148480 z=0..600 vertices=8 wall=6 roof=2 open roof_area=7340032 outward
      id=1 buildings=1 x=135680..139264 y=146432..148992 z=0..2500 vertices=12 wall=10 roof=3 open roof_area=8126464 outward
    ");
}

#[test]
fn merged_blocks_have_no_walls_lower_than_a_pixel() {
    let dir = TempDir::new();
    let input = dir.write(
        "low_walls.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "lower",
                    &[cuboid(2, [E, N, 0.0], [E + 40.0, N + 20.0, 9.5])],
                ),
                building(
                    "higher",
                    &[cuboid(2, [E + 40.0, N, 0.0], [E + 80.0, N + 20.0, 10.5])],
                ),
                building(
                    "shed",
                    &[cuboid(2, [E + 120.0, N, 0.0], [E + 160.0, N + 20.0, 1.5])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=1 x=132096..135680 y=146432..148480 z=0..950 vertices=8 wall=6 roof=2 open roof_area=7340032 outward
      id=1 buildings=1 x=135680..139264 y=146432..148992 z=0..1050 vertices=10 wall=8 roof=3 open roof_area=8126464 outward
      id=2 buildings=1 x=142848..146432 y=146944..148992 z=150..150 vertices=4 roof=2 open roof_area=7340032 up
    ");
}

#[test]
fn a_staircase_of_cells_is_simplified_to_its_slope() {
    let dir = TempDir::new();
    let wedge = [[E, N, 0.0], [E + 80.0, N, 0.0], [E, N + 80.0, 0.0]];
    let walls: Vec<String> = (0..3)
        .map(|i| {
            let [a, b] = [wedge[i], wedge[(i + 1) % 3]];
            polygon(&[a, b, [b[0], b[1], 5.0], [a[0], a[1], 5.0]], &[])
        })
        .collect();
    let roof: Vec<[f64; 3]> = wedge.iter().map(|&[x, y, _]| [x, y, 5.0]).collect();
    let ground: Vec<[f64; 3]> = wedge.iter().rev().copied().collect();
    let input = dir.write(
        "wedge.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "wedge",
                &[
                    surface("WallSurface", 2, &walls),
                    surface("RoofSurface", 2, &[polygon(&roof, &[])]),
                    surface("GroundSurface", 2, &[polygon(&ground, &[])]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=1 x=132096..139264 y=141312..148992 z=500..500 vertices=7 roof=5 open roof_area=24379392 up
    ");
}

#[test]
fn a_courtyard_is_a_hole_in_the_merged_roof_walled_inside() {
    let dir = TempDir::new();
    let input = dir.write(
        "courtyard.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "north",
                    &[cuboid(2, [E, N, 0.0], [E + 60.0, N + 20.0, 9.0])],
                ),
                building(
                    "south",
                    &[cuboid(2, [E, N + 40.0, 0.0], [E + 60.0, N + 60.0, 9.0])],
                ),
                building(
                    "west",
                    &[cuboid(2, [E, N + 20.0, 0.0], [E + 20.0, N + 40.0, 9.0])],
                ),
                building(
                    "east",
                    &[cuboid(
                        2,
                        [E + 40.0, N + 20.0, 0.0],
                        [E + 60.0, N + 40.0, 9.0],
                    )],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=4 x=132096..137728 y=142848..148480 z=0..900 vertices=18 wall=18 roof=9 open roof_area=29229056 outward
    ");
}

#[test]
fn two_courtyards_touching_at_a_corner_each_keep_their_hole() {
    let dir = TempDir::new();
    let ring = |x: f64, y: f64, name: &str| -> Vec<String> {
        vec![
            building(
                &format!("{name}-n"),
                &[cuboid(2, [x, y, 0.0], [x + 60.0, y + 20.0, 9.0])],
            ),
            building(
                &format!("{name}-s"),
                &[cuboid(2, [x, y + 40.0, 0.0], [x + 60.0, y + 60.0, 9.0])],
            ),
            building(
                &format!("{name}-w"),
                &[cuboid(2, [x, y + 20.0, 0.0], [x + 20.0, y + 40.0, 9.0])],
            ),
            building(
                &format!("{name}-e"),
                &[cuboid(
                    2,
                    [x + 40.0, y + 20.0, 0.0],
                    [x + 60.0, y + 40.0, 9.0],
                )],
            ),
        ]
    };
    let input = dir.write(
        "figure_eight.gml",
        &city_model(
            "EPSG:25832",
            &[ring(E, N + 60.0, "nw"), ring(E + 60.0, N, "se")].concat(),
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=8 x=132096..142848 y=137728..148992 z=0..900 vertices=34 wall=34 roof=19 open roof_area=59506688 outward
    ");
}

#[test]
fn a_building_under_half_a_merged_cell_is_left_out() {
    let dir = TempDir::new();
    let input = dir.write(
        "hut.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "hut",
                &[cuboid(2, [E, N, 0.0], [E + 2.0, N + 2.0, 3.0])],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"");
}

#[test]
fn hillside_houses_in_different_bands_stay_apart_and_keep_their_downhill_foundations() {
    let dir = TempDir::new();
    let input = dir.write(
        "hillside.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "uphill",
                    &[cuboid(2, [E, N, 15.0], [E + 40.0, N + 20.0, 27.0])],
                ),
                building(
                    "downhill",
                    &[cuboid(2, [E + 40.0, N, 0.0], [E + 80.0, N + 20.0, 12.0])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=1 x=132096..135680 y=146432..148480 z=-300..1200 vertices=10 wall=8 roof=2 open roof_area=7340032 outward
      id=1 buildings=1 x=135680..139264 y=146432..148992 z=0..1200 vertices=10 wall=8 roof=3 open roof_area=8126464 outward
    ");
}

#[test]
fn every_triangle_of_an_l_shaped_block_faces_out() {
    let dir = TempDir::new();
    let input = dir.write(
        "l.gml",
        &city_model(
            "EPSG:25832",
            &[
                building("long", &[cuboid(2, [E, N, 0.0], [E + 80.0, N + 40.0, 9.0])]),
                building(
                    "short",
                    &[cuboid(2, [E, N + 40.0, 0.0], [E + 40.0, N + 80.0, 9.0])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=2 x=132096..139264 y=141312..148992 z=0..900 vertices=16 wall=16 roof=6 open roof_area=39321600 outward
    ");
}

#[test]
fn a_hall_on_a_small_pier_keeps_its_whole_roof_when_merged() {
    let dir = TempDir::new();
    let canopy = [
        [E, N, 18.0],
        [E + 60.0, N, 18.0],
        [E + 60.0, N + 40.0, 18.0],
        [E, N + 40.0, 18.0],
    ];
    let pier = [
        [E, N, 0.0],
        [E, N + 5.0, 0.0],
        [E + 5.0, N + 5.0, 0.0],
        [E + 5.0, N, 0.0],
    ];
    let input = dir.write(
        "hall.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "hall",
                &[
                    surface("RoofSurface", 2, &[polygon(&canopy, &[])]),
                    surface("GroundSurface", 2, &[polygon(&pier, &[])]),
                ],
            )],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=0 buildings=1 x=132096..137728 y=144896..148480 z=0..1800 vertices=8 wall=8 roof=2 open roof_area=20185088 outward
    ");
}

#[test]
fn a_footprint_too_wide_for_one_raster_is_left_out_while_its_neighbour_is_merged_on_its_own() {
    let dir = TempDir::new();
    let input = dir.write(
        "giant.gml",
        &city_model(
            "EPSG:25832",
            &[
                building(
                    "giant",
                    &[cuboid(
                        2,
                        [E - 6000.0, N - 6000.0, 0.0],
                        [E + 6000.0, N + 6000.0, 10.0],
                    )],
                ),
                building(
                    "house",
                    &[cuboid(2, [E, N, 0.0], [E + 20.0, N + 20.0, 10.0])],
                ),
            ],
        ),
    );
    let output = dir.0.join("out");
    convert(
        &input,
        &output,
        &["--mlt-version", "2", "--min-zoom", "13", "--max-zoom", "16"],
    );
    insta::assert_snapshot!(tiles(&output, 13), @"
    13/4323/2647 extent=262144
      id=1 buildings=1 x=132096..134144 y=146432..148480 z=0..1000 vertices=8 wall=8 roof=2 open roof_area=3670016 outward
    ");
}

#[test]
fn a_citygml_input_rejects_flags_it_cannot_honour() {
    let dir = TempDir::new();
    let input = dir.write(
        "hamburg.gml",
        &city_model(
            "EPSG:25832",
            &[building(
                "tower",
                &[cuboid(2, [E, N, 6.5], [E + 20.0, N + 20.0, 66.5])],
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
    Error: tiling a CityGML input writes MLT version 2, so it needs --mlt-version 2
    ---
    Error: tiling a CityGML input requires --max-zoom
    ---
    Error: --tessellate cuts 2D polygons, so it does not apply to a CityGML mesh input
    ---
    Error: a CityGML input stores heights above each building's ground, so it takes no --height-mode
    ---
    Error: a CityGML input is tiled into a directory of z/x/y.mlt tiles; archive output is not supported yet, got: [TMP]/out.pmtiles
    ");
}
