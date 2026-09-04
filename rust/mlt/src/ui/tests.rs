use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::geojson::{Feature, FeatureCollection};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use serde_json::{Value, json};

use super::{
    MouseState, PanelAreas, SCAN_FLAGS, UiArgs, build_app, collect_extensions,
    collect_file_algorithms, collect_file_geometries, extent_from_fc, find_tile_files,
    handle_filter_click, handle_key, handle_mouse, load_fc, parse_center_tile_xyz, render_frame,
    tick,
};
use crate::ls::{FileSortColumn, LsRow, analyze_tile_files};
use crate::ui::mbt::{MbtHoveredInfo, MbtTileData, MbtilesState};
use crate::ui::state::{App, HoveredInfo, ResizeHandle, TreeItem, ViewMode};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;

fn render(app: &mut App) -> String {
    render_sized(app, WIDTH, HEIGHT)
}

fn render_sized(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut areas = PanelAreas::default();
    terminal.draw(|f| render_frame(f, app, &mut areas)).unwrap();
    terminal.backend().to_string()
}

fn press(app: &mut App, code: KeyCode) {
    assert!(
        !handle_key(app, KeyEvent::new(code, KeyModifiers::NONE)),
        "{code:?} should not quit"
    );
}

fn quits(app: &mut App, code: KeyCode) -> bool {
    handle_key(app, KeyEvent::new(code, KeyModifiers::NONE))
}

fn c(x: i32, y: i32) -> Coord<i32> {
    Coord { x, y }
}

fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> LineString<i32> {
    LineString(vec![c(x0, y0), c(x1, y0), c(x1, y1), c(x0, y1)])
}

fn feature(layer: &str, geometry: impl Into<Geometry<i32>>, props: &[(&str, Value)]) -> Feature {
    let mut properties: BTreeMap<String, Value> = props
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect();
    properties.insert("_layer".into(), json!(layer));
    properties.insert("_extent".into(), json!(4096));
    Feature {
        geometry: geometry.into(),
        id: None,
        properties,
        ty: "Feature".into(),
    }
}

/// Three layers covering every geometry type, with a hole in the first polygon.
fn sample_fc() -> FeatureCollection {
    let features = vec![
        feature(
            "water",
            Polygon::new(
                square(500, 500, 3500, 3500),
                vec![LineString(vec![
                    c(1500, 1500),
                    c(1500, 2500),
                    c(2500, 2500),
                    c(2500, 1500),
                ])],
            ),
            &[("class", json!("lake")), ("name", json!("Loch"))],
        ),
        feature(
            "water",
            MultiPolygon(vec![
                Polygon::new(square(200, 200, 800, 800), vec![]),
                Polygon::new(square(3300, 3300, 3900, 3900), vec![]),
            ]),
            &[("class", json!("pond"))],
        ),
        feature(
            "roads",
            LineString(vec![c(0, 2048), c(4096, 2048)]),
            &[("class", json!("primary")), ("oneway", json!(true))],
        ),
        feature(
            "roads",
            MultiLineString(vec![
                LineString(vec![c(2048, 0), c(2048, 4096)]),
                LineString(vec![c(0, 0), c(4096, 4096)]),
            ]),
            &[("class", json!("path"))],
        ),
        feature(
            "poi",
            Point(c(1000, 3000)),
            &[("name", json!("Cafe")), ("rank", json!(3))],
        ),
        feature(
            "poi",
            MultiPoint(vec![Point(c(3000, 1000)), Point(c(3200, 1200))]),
            &[("name", json!("Twin peaks"))],
        ),
    ];
    FeatureCollection {
        features,
        ty: "FeatureCollection".into(),
    }
}

fn sample_app() -> App {
    App::new_single_file(sample_fc(), Some(PathBuf::from("sample.mlt")))
}

/// Fixture paths relative to the package directory, where cargo runs unit tests.
fn test_dir(rel: &str) -> PathBuf {
    PathBuf::from("../../test").join(rel)
}

fn fixtures_dir() -> PathBuf {
    test_dir("fixtures/simple")
}

/// File browser over `test/fixtures/simple` with every file analyzed.
fn file_browser_app() -> App {
    browser_over(fixtures_dir())
}

fn browser_over(base: PathBuf) -> App {
    let paths = find_tile_files(&base).unwrap();
    let files = analyze_tile_files(&paths, &base, SCAN_FLAGS);
    App::new_file_browser(files, None, base)
}

fn analyze_row(path: PathBuf, base: &std::path::Path) -> LsRow {
    analyze_tile_files(&[path], base, SCAN_FLAGS).remove(0)
}

#[test]
fn layer_overview_starts_on_all_layers() {
    let mut app = sample_app();
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│>> All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│     Layer: roads (2 feature││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢲⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢒⡲⡆     │"
    "│     Layer: poi (2 features,││     ⢸                           ⢸                 ⡤⠤⠤⠤⠤⠤⠤⢤⡔⠁ ⡇     │"
    "│                            ││     ⢸                           ⢸                 ⡇    ⡠⠊⠁⡇  ⡇     │"
    "│                            ││     ⢸      ⢀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣸⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣇⣀⣀⠔⠊   ⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸                 ⣧⣒⣹⣀⣀⣀⣀⣀⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸              ⢀⠤⠊  ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ×             ⢸            ⣀⠔⠁    ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸         ⢀⡠⠊       ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸       ⢀⠔⠁         ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡖⠒⠒⠒⠒⠒⠒⢺⠒⠒⠒⠒⠒⡲⡎⠁           ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡇      ⢸  ⢀⠔⠊ ⡇            ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸      ⢸             ⡇      ⢸⡠⠒⠁   ⡇            ⢸        ⡇     │"
    "┌Properties──────────────────┐│     ⢸⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⡏⠉⠉⠉⠉⢉⠭⢻⠉⠉⠉⠉⠉⠉⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⡇     │"
    "│Select or hover over a      ││     ⢸      ⢸             ⡇  ⡠⠔⠁ ⢸      ⡇            ⢸        ⡇     │"
    "│feature to view properties  ││     ⢸      ⢸             ⣇⡠⠊    ⢸      ⡇            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸           ⢀⠔⠉⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠁            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸         ⡠⠊⠁        ⢸               ×   ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ⢀⠔⠉           ⢸            ×      ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸    ⡠⠒⠁             ⢸                   ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸  ⢸⠉⠉⠉⢹⠉⢉⠭⢻                ⢸                   ⢸        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸  ⢸   ⡸⠴⠥⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼        ⡇     │"
    "│Select or hover over a      ││     ⢸  ⢸⢀⡠⠊    ⢸                ⢸                            ⡇     │"
    "│feature to view geometry    ││     ⢸ ⢀⠜⠓⠒⠒⠒⠒⠒⠒⠚                ⢸                            ⡇     │"
    "│info                        ││     ⠸⠮⠥⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                            ││                                                                    │"
    "│                            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn keys_expand_a_layer_and_select_a_feature() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│   All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│>>     Feat 0: Polygon (10v,││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⡆     │"
    "│       Feat 1: MultiPolygon ││     ⢸                                                        ⡇     │"
    "│     Layer: roads (2 feature││     ⢸                                                        ⡇     │"
    "│     Layer: poi (2 features,││     ⢸      ⢀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡖⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⡆            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡇             ⡇            ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸      ⢸             ⡇             ⡇            ⢸        ⡇     │"
    "┌Properties (feat 0)─────────┐│     ⢸      ⢸             ⡇             ⡇            ⢸        ⡇     │"
    "│class: lake                 ││     ⢸      ⢸             ⡇             ⡇            ⢸        ⡇     │"
    "│name: Loch                  ││     ⢸      ⢸             ⡇             ⡇            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠁            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                                        ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸      ⢸                                        ⢸        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸      ⠸⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼        ⡇     │"
    "│Type: Polygon               ││     ⢸                                                        ⡇     │"
    "│Vertices: 10                ││     ⢸                                                        ⡇     │"
    "│Rings: 2                    ││     ⠸⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│  Ring 0: 5v, CCW           ││                                                                    │"
    "│  Ring 1: 5v, CW            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn keys_drill_into_a_multipolygon_part() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│   All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│       Feat 0: Polygon (10v,││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⡆     │"
    "│       Feat 1: MultiPolygon ││     ⢸                                             ⡤⠤⠤⠤⠤⠤⠤⠤⡄  ⡇     │"
    "│>>       Part 0: Polygon (5v││     ⢸                                             ⡇       ⡇  ⡇     │"
    "│         Part 1: Polygon (5v││     ⢸                                             ⡇       ⡇  ⡇     │"
    "│     Layer: roads (2 feature││     ⢸                                             ⣇⣀⣀⣀⣀⣀⣀⣀⡇  ⡇     │"
    "│     Layer: poi (2 features,││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸                                                        ⡇     │"
    "┌Properties (feat 1)─────────┐│     ⢸                                                        ⡇     │"
    "│class: pond                 ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸  ⢸⠉⠉⠉⠉⠉⠉⠉⢹                                             ⡇     │"
    "┌Geometry────────────────────┐│     ⢸  ⢸       ⢸                                             ⡇     │"
    "│Component: part #0 of a     ││     ⢸  ⢸       ⢸                                             ⡇     │"
    "│MultiPolygon                ││     ⢸  ⠘⠒⠒⠒⠒⠒⠒⠒⠚                                             ⡇     │"
    "│Type: Polygon               ││     ⠸⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│Vertices: 5                 ││                                                                    │"
    "│Rings: 1                    ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn star_expands_every_layer_and_end_jumps_to_the_last_row() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Char('*'));
    press(&mut app, KeyCode::End);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│   All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│       Feat 0: Polygon (10v,││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⡆     │"
    "│       Feat 1: MultiPolygon ││     ⢸                                                        ⡇     │"
    "│     Layer: roads (2 feature││     ⢸                                                        ⡇     │"
    "│       Feat 0: LineString (2││     ⢸                                                        ⡇     │"
    "│       Feat 1: MultiLineStri││     ⢸                                                        ⡇     │"
    "│     Layer: poi (2 features,││     ⢸                                                        ⡇     │"
    "│       Feat 0: Point        ││     ⢸                                                        ⡇     │"
    "│>>     Feat 1: MultiPoint (2││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸                                                        ⡇     │"
    "┌Properties (feat 1)─────────┐│     ⢸                                                        ⡇     │"
    "│name: Twin peaks            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                           ×            ⡇     │"
    "│                            ││     ⢸                                        ×               ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸                                                        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸                                                        ⡇     │"
    "│Type: MultiPoint            ││     ⢸                                                        ⡇     │"
    "│Points: 2                   ││     ⢸                                                        ⡇     │"
    "│                            ││     ⠸⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                            ││                                                                    │"
    "│                            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn hovering_the_map_highlights_the_nearest_feature() {
    let mut app = sample_app();
    let bounds = app.get_bounds();
    app.find_hovered_feature(1000.0, 3000.0, bounds);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│>> All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│     Layer: roads (2 feature││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢲⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢒⡲⡆     │"
    "│     Layer: poi (2 features,││     ⢸                           ⢸                 ⡤⠤⠤⠤⠤⠤⠤⢤⡔⠁ ⡇     │"
    "│                            ││     ⢸                           ⢸                 ⡇    ⡠⠊⠁⡇  ⡇     │"
    "│                            ││     ⢸      ⢀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣸⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣇⣀⣀⠔⠊   ⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸                 ⣧⣒⣹⣀⣀⣀⣀⣀⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸              ⢀⠤⠊  ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ×             ⢸            ⣀⠔⠁    ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸         ⢀⡠⠊       ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸       ⢀⠔⠁         ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡖⠒⠒⠒⠒⠒⠒⢺⠒⠒⠒⠒⠒⡲⡎⠁           ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸             ⡇      ⢸  ⢀⠔⠊ ⡇            ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸      ⢸             ⡇      ⢸⡠⠒⠁   ⡇            ⢸        ⡇     │"
    "┌Properties (feat 0, hover)──┐│     ⢸⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⡏⠉⠉⠉⠉⢉⠭⢻⠉⠉⠉⠉⠉⠉⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⡇     │"
    "│name: Cafe                  ││     ⢸      ⢸             ⡇  ⡠⠔⠁ ⢸      ⡇            ⢸        ⡇     │"
    "│rank: 3                     ││     ⢸      ⢸             ⣇⡠⠊    ⢸      ⡇            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸           ⢀⠔⠉⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠁            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸         ⡠⠊⠁        ⢸               ×   ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ⢀⠔⠉           ⢸            ×      ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸    ⡠⠒⠁             ⢸                   ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸  ⢸⠉⠉⠉⢹⠉⢉⠭⢻                ⢸                   ⢸        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸  ⢸   ⡸⠴⠥⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼        ⡇     │"
    "│Type: Point                 ││     ⢸  ⢸⢀⡠⠊    ⢸                ⢸                            ⡇     │"
    "│Coords: [1000, 3000]        ││     ⢸ ⢀⠜⠓⠒⠒⠒⠒⠒⠒⠚                ⢸                            ⡇     │"
    "│                            ││     ⠸⠮⠥⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                            ││                                                                    │"
    "│                            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn help_overlay_lists_the_layer_overview_keys() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Char('?'));
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│>> All            ┌Help (↑/↓/scroll to navigate, any other key to close)───────┐                  │"
    "│     Layer: water │Keyboard                                                    │                  │"
    "│     Layer: roads │  ?  h  F1            Toggle this help                      │⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢒⡲⡆     │"
    "│     Layer: poi (2│  q  Ctrl+c           Quit                                  │ ⡤⠤⠤⠤⠤⠤⠤⢤⡔⠁ ⡇     │"
    "│                  │  Esc                 Back to file browser                  │ ⡇    ⡠⠊⠁⡇  ⡇     │"
    "│                  │  Up/Down  j/k        Navigate feature tree                 │⣀⣇⣀⣀⠔⠊   ⡇  ⡇     │"
    "│                  │  PageUp/PageDown     Scroll by page                        │ ⣧⣒⣹⣀⣀⣀⣀⣀⡇  ⡇     │"
    "│                  │  Home/End            Jump to first/last                    │⠊  ⢸        ⡇     │"
    "│                  │  Enter               Expand/collapse layer or feature      │   ⢸        ⡇     │"
    "│                  │  +  =  Right         Expand selected node                  │   ⢸        ⡇     │"
    "│                  │  -                   Collapse (or jump to parent)          │   ⢸        ⡇     │"
    "│                  │  *                   Expand/collapse all layers            │   ⢸        ⡇     │"
    "│                  │  Left                Jump to parent node                   │   ⢸        ⡇     │"
    "└──────────────────│  Ctrl+h / Ctrl+l     Resize left/right split               │   ⢸        ⡇     │"
    "┌Properties────────│  Shift+J / Shift+K   Resize top/bottom split               │⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⡇     │"
    "│Select or hover ov│                                                            │   ⢸        ⡇     │"
    "│feature to view pr│Mouse                                                       │   ⢸        ⡇     │"
    "│                  │  Click tree item     Select (drill into level)             │   ⢸        ⡇     │"
    "│                  │  Double-click        Expand/collapse                       │   ⢸        ⡇     │"
    "│                  │  Hover tree/map      Highlight geometry                    │   ⢸        ⡇     │"
    "│                  │  Click on map        Select hovered feature                │   ⢸        ⡇     │"
    "└──────────────────│  Scroll panels       Scroll tree/properties                │   ⢸        ⡇     │"
    "┌Geometry──────────│  Drag dividers       Resize panels                         │⠤⠤⠤⠼        ⡇     │"
    "│Select or hover ov│                                                            │            ⡇     │"
    "│feature to view ge│Map Colors                                                  │            ⡇     │"
    "│info              │  Magenta             Point                                 │⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                  │  Light magenta       MultiPoint                            │                  │"
    "│                  └────────────────────────────────────────────────────────────┘                  │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
    press(&mut app, KeyCode::Char('x'));
    assert!(!app.show_help);
}

#[test]
fn error_popup_shows_the_message_until_a_key_is_pressed() {
    let mut app = sample_app();
    app.error_popup = Some(("broken.mlt".into(), "unexpected end of buffer".into()));
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌sample.mlt - Enter/+/-:expan┐┌Map View────────────────────────────────────────────────────────────┐"
    "│>> All                      ││                                                                    │"
    "│     Layer: water (2 feature││                                                                    │"
    "│     Layer: roads (2 feature││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢲⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⢒⡲⡆     │"
    "│     Layer: poi (2 features,││     ⢸                           ⢸                 ⡤⠤⠤⠤⠤⠤⠤⢤⡔⠁ ⡇     │"
    "│                            ││     ⢸                           ⢸                 ⡇    ⡠⠊⠁⡇  ⡇     │"
    "│                            ││     ⢸      ⢀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣸⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣇⣀⣀⠔⠊   ⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸                 ⣧⣒⣹⣀⣀⣀⣀⣀⡇  ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸              ⢀⠤⠊  ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ×             ⢸            ⣀⠔⠁    ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸         ⢀⡠⠊       ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸                    ⢸       ⢀⠔⠁         ⢸        ⡇     │"
    "│         ┌ Unable to open broken.mlt ───────────────────────────────────────────────────┐   ⡇     │"
    "│         │                                                                              │   ⡇     │"
    "└─────────│                           unexpected end of buffer                           │   ⡇     │"
    "┌Propertie│                                                                              │⠉⠉⠉⡇     │"
    "│Select or│                                                                              │   ⡇     │"
    "│feature t└──────────────────────────────────────────────────────────────any key to close┘   ⡇     │"
    "│                            ││     ⢸      ⢸           ⢀⠔⠉⠉⠉⠉⠉⠉⠉⢹⠉⠉⠉⠉⠉⠉⠁            ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸         ⡠⠊⠁        ⢸               ×   ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸      ⢀⠔⠉           ⢸            ×      ⢸        ⡇     │"
    "│                            ││     ⢸      ⢸    ⡠⠒⠁             ⢸                   ⢸        ⡇     │"
    "└────────────────────────────┘│     ⢸  ⢸⠉⠉⠉⢹⠉⢉⠭⢻                ⢸                   ⢸        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸  ⢸   ⡸⠴⠥⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⢼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼        ⡇     │"
    "│Select or hover over a      ││     ⢸  ⢸⢀⡠⠊    ⢸                ⢸                            ⡇     │"
    "│feature to view geometry    ││     ⢸ ⢀⠜⠓⠒⠒⠒⠒⠒⠒⠚                ⢸                            ⡇     │"
    "│info                        ││     ⠸⠮⠥⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠼⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                            ││                                                                    │"
    "│                            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
    press(&mut app, KeyCode::Enter);
    assert!(app.error_popup.is_none());
}

#[test]
fn quit_keys() {
    let mut app = sample_app();
    assert!(
        quits(&mut app, KeyCode::Esc),
        "Esc quits without a file list"
    );
    assert!(quits(&mut app, KeyCode::Char('q')));
    assert!(handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    ));
}

#[test]
fn file_browser_lists_the_directory() {
    let mut app = file_browser_app();
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌MLT Files (6 found) - ↑/↓ navigate, Enter open, h help, q quit Click┐┌Tile Preview────────────────┐"
    "│   File                         Size   Enc % Layers   Features Notes││Select a tile file (.mlt / .│"
    "│>> line-boolean.mvt              40B      -       1          1      ││                            │"
    "│   multiline-boolean.mvt         46B      -       1          1      ││                            │"
    "│   multipoint-boolean.mvt        37B      -       1          1      ││                            │"
    "│   multipolygon-boolean.mvt      65B      -       1          1      ││                            │"
    "│   point-boolean.mvt             35B      -       1          1      ││                            │"
    "│   polygon-boolean.mvt           41B      -       1          1      ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌Filter (click to toggle)────┐"
    "│                                                                    ││[Reset filters]             │"
    "│                                                                    ││                            │"
    "│                                                                    ││Extensions:                 │"
    "│                                                                    ││  [ ] mvt                   │"
    "│                                                                    ││                            │"
    "│                                                                    ││Geometry Types:             │"
    "│                                                                    ││  [ ] Point                 │"
    "│                                                                    ││  [ ] LineString            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌File Info───────────────────┐"
    "│                                                                    ││File: line-boolean.mvt      │"
    "│                                                                    ││Size: 40B  raw MLT file size│"
    "│                                                                    ││Encoding: -  MLT / (data +  │"
    "│                                                                    ││metadata)                   │"
    "│                                                                    ││Data: -  decoded payload    │"
    "│                                                                    ││size                        │"
    "│                                                                    ││Metadata: -  encoding       │"
    "│                                                                    ││overhead                    │"
    "└────────────────────────────────────────────────────────────────────┘└────────────────────────────┘"
    "#);
}

#[test]
fn file_browser_previews_the_selected_tile() {
    let mut app = file_browser_app();
    press(&mut app, KeyCode::Down);
    let path = app.get_selected_file().unwrap().path().to_path_buf();
    let fc = load_fc(&path).unwrap();
    app.preview_extent = extent_from_fc(&fc);
    app.preview_fc = Some(fc);
    app.preview_tile_path = Some(path);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌MLT Files (6 found) - ↑/↓ navigate, Enter open, h help, q quit Click┐┌Tile Preview────────────────┐"
    "│   File                         Size   Enc % Layers   Features Notes││⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹│"
    "│   line-boolean.mvt              40B      -       1          1      ││⡇                          ⢸│"
    "│>> multiline-boolean.mvt         46B      -       1          1      ││⡇                          ⢸│"
    "│   multipoint-boolean.mvt        37B      -       1          1      ││⡇                          ⢸│"
    "│   multipolygon-boolean.mvt      65B      -       1          1      ││⡇                          ⢸│"
    "│   point-boolean.mvt             35B      -       1          1      ││⡇                          ⢸│"
    "│   polygon-boolean.mvt           41B      -       1          1      ││⡇                          ⢸│"
    "│                                                                    ││⣇⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣸│"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌Filter (click to toggle)────┐"
    "│                                                                    ││[Reset filters]             │"
    "│                                                                    ││                            │"
    "│                                                                    ││Extensions:                 │"
    "│                                                                    ││  [ ] mvt                   │"
    "│                                                                    ││                            │"
    "│                                                                    ││Geometry Types:             │"
    "│                                                                    ││  [ ] Point                 │"
    "│                                                                    ││  [ ] LineString            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌File Info───────────────────┐"
    "│                                                                    ││File: multiline-boolean.mvt │"
    "│                                                                    ││Size: 46B  raw MLT file size│"
    "│                                                                    ││Encoding: -  MLT / (data +  │"
    "│                                                                    ││metadata)                   │"
    "│                                                                    ││Data: -  decoded payload    │"
    "│                                                                    ││size                        │"
    "│                                                                    ││Metadata: -  encoding       │"
    "│                                                                    ││overhead                    │"
    "└────────────────────────────────────────────────────────────────────┘└────────────────────────────┘"
    "#);
}

#[test]
fn file_browser_sorts_by_a_clicked_header() {
    let mut app = file_browser_app();
    app.handle_file_header_click(FileSortColumn::Size);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌MLT Files (6 found) - ↑/↓ navigate, Enter open, h help, q quit Click┐┌Tile Preview────────────────┐"
    "│   File                         Size   Enc % Layers   Features Notes││Select a tile file (.mlt / .│"
    "│   point-boolean.mvt             35B      -       1          1      ││                            │"
    "│   multipoint-boolean.mvt        37B      -       1          1      ││                            │"
    "│>> line-boolean.mvt              40B      -       1          1      ││                            │"
    "│   polygon-boolean.mvt           41B      -       1          1      ││                            │"
    "│   multiline-boolean.mvt         46B      -       1          1      ││                            │"
    "│   multipolygon-boolean.mvt      65B      -       1          1      ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌Filter (click to toggle)────┐"
    "│                                                                    ││[Reset filters]             │"
    "│                                                                    ││                            │"
    "│                                                                    ││Extensions:                 │"
    "│                                                                    ││  [ ] mvt                   │"
    "│                                                                    ││                            │"
    "│                                                                    ││Geometry Types:             │"
    "│                                                                    ││  [ ] Point                 │"
    "│                                                                    ││  [ ] LineString            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌File Info───────────────────┐"
    "│                                                                    ││File: line-boolean.mvt      │"
    "│                                                                    ││Size: 40B  raw MLT file size│"
    "│                                                                    ││Encoding: -  MLT / (data +  │"
    "│                                                                    ││metadata)                   │"
    "│                                                                    ││Data: -  decoded payload    │"
    "│                                                                    ││size                        │"
    "│                                                                    ││Metadata: -  encoding       │"
    "│                                                                    ││overhead                    │"
    "└────────────────────────────────────────────────────────────────────┘└────────────────────────────┘"
    "#);
}

#[test]
fn filter_click_narrows_the_file_list() {
    let mut app = file_browser_app();
    let first_geometry_row = 3 + collect_extensions(&app.files).len() + 2;
    handle_filter_click(&mut app, first_geometry_row);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌MLT Files (1/6 found) - ↑/↓ navigate, Enter open, h help, q quit Cli┐┌Tile Preview────────────────┐"
    "│   File                         Size   Enc % Layers   Features Notes││Select a tile file (.mlt / .│"
    "│>> point-boolean.mvt             35B      -       1          1      ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    ││                            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌Filter (click to toggle)────┐"
    "│                                                                    ││[Reset filters]             │"
    "│                                                                    ││                            │"
    "│                                                                    ││Extensions:                 │"
    "│                                                                    ││  [ ] mvt                   │"
    "│                                                                    ││                            │"
    "│                                                                    ││Geometry Types:             │"
    "│                                                                    ││  [x] Point                 │"
    "│                                                                    ││  [ ] LineString            │"
    "│                                                                    │└────────────────────────────┘"
    "│                                                                    │┌File Info───────────────────┐"
    "│                                                                    ││File: point-boolean.mvt     │"
    "│                                                                    ││Size: 35B  raw MLT file size│"
    "│                                                                    ││Encoding: -  MLT / (data +  │"
    "│                                                                    ││metadata)                   │"
    "│                                                                    ││Data: -  decoded payload    │"
    "│                                                                    ││size                        │"
    "│                                                                    ││Metadata: -  encoding       │"
    "│                                                                    ││overhead                    │"
    "└────────────────────────────────────────────────────────────────────┘└────────────────────────────┘"
    "#);
}

#[test]
fn enter_opens_a_file_and_escape_returns_to_the_browser() {
    let mut app = file_browser_app();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.mode, ViewMode::LayerOverview);
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌line-boolean.mvt - Enter/+/-┐┌Map View────────────────────────────────────────────────────────────┐"
    "│>> All                      ││                                                                    │"
    "│     Layer: layer (1 LineStr││                                                                    │"
    "│       Feat 0: LineString (3││     ⢰⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⠒⡆     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸                                                        ⡇     │"
    "┌Properties──────────────────┐│     ⢸                                                        ⡇     │"
    "│Select or hover over a      ││     ⢸                                                        ⡇     │"
    "│feature to view properties  ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "│                            ││     ⢸                                                        ⡇     │"
    "└────────────────────────────┘│     ⢸                                                        ⡇     │"
    "┌Geometry────────────────────┐│     ⢸                                                        ⡇     │"
    "│Select or hover over a      ││     ⢸                                                        ⡇     │"
    "│feature to view geometry    ││     ⢸                                                        ⡇     │"
    "│info                        ││     ⠸⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠤⠇     │"
    "│                            ││                                                                    │"
    "│                            ││                                                                    │"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, ViewMode::FileBrowser);
}

fn mbtiles_app() -> App {
    let path = test_dir("fixtures/omt.max1.mbtiles");
    let mbt = MbtilesState::new(path.clone());
    App::new_mbtiles(mbt, path)
}

fn mbt(app: &mut App) -> &mut MbtilesState {
    app.mbt_state.as_mut().unwrap()
}

#[test]
fn mbtiles_map_renders_the_world_tile() {
    let mut app = mbtiles_app();
    mbt(&mut app).wait_for_visible_tiles();
    assert!(mbt(&mut app).take_loader_fatal().is_none());
    assert!(matches!(
        mbt(&mut app).tiles.get(&(0, 0, 0)),
        Some(MbtTileData::Loaded { .. })
    ));
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌Properties──────────────────┐┌World Map - 0/0/0 - zoom 0.0  drag=pan  hover=info  q/Esc quit──────┐"
    "│Hover over a feature to     ││⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹│"
    "│inspect properties          ││⡇×               ⢀⣠⠤⠤⠤⡄  ⣠⣴⡏⠒⠤⡀                                    ⢸│"
    "│                            ││⡇               ⣴⡊⡁ ⢀⣶⡯⠭⠾⠽⠉⠲⢭⠻⢴⡶     ⢀⡀   ⣀⣀       ⢠⡀              ⢸│"
    "│                            ││⡇           ⢀⡄⣤⣼⣤⣸⠍⢠⣺⡵⠊      ⢉⡟    ⠐⡝⢓⡟⠁  ⠈⠁       ⠱⣱⢄             ⢸│"
    "│                            ││⡇         ⠠⣊⠿⣧⣬⣽⣷⣿⣥⠊⠙⠷⠤⡀     ⢾⡗     ⠙⠋⠋     ⢀⢤⡶   ⢀⡤⡨⠛⠦⡄   ⢀⠤⣀     ⢸│"
    "│                            ││⡇         ⡰⣳⠯⢽⣧⢿⡿⡿⢽⡧⡀  ⢣⡀   ⢰⣾⠃            ⢠⡟⠁⢠⢤⣄⡖⠋   ⠐⠗⠦⠖⣄⣀⡿⢍⡁    ⢸│"
    "│                            ││⡯⣀⢠⡖⠉⠉⠑⢲⠔⠚⠚⢻⣯⣤⣽⣻⡜⣿⢲⢲⡉⢇⡀⢘⢧  ⣠⡿⠟      ⢀⡴⡾⣟⠢⢄⣤⣀⠭⠖⠺⢹⡅         ⠉⠉  ⠈⠙⠒⠲⠖⢺│"
    "│                            ││⡗⠻⢝⠿ ⢀ ⢸  ⠘⢛⣄×  ⢠⠼⣷⣒⡧⢽⠃⠈⢿⣠⡞⠉ ⠹⠭⠃  ⢀⡰⡵⣵⠟⢸⣽⠛⠉    ⠉   ×           ⣀⢀ ⢀⣺│"
    "│                            ││⡇ ⠈⣙⡶⠞⠉⠙⠺⣆  ⠉⠒⠲⢀⠳⢄⣀⡣⠈⠚⢄⡀⠈⠉      ×⣦⠈⢲⣷⣪⣿⢏⠉     ⣀⣀⡀     ⣀    ⣠⢔⠒⠛⡖⡯⠉⠁⢸│"
    "│                            ││⡇        ⠻⢦⡤⠤×⠤⠽⢄⣤⡘⠃⢀⣠⣤⣿⡄       ⠻×××⣽⣦⣿⣞⣓⡢⢰⣖⠒⡛ ⢉⣉⢒×⡒⠖⠾⠥⢤⡜⠹⢄⣌⢿⠄ ⠛⠁  ⢸│"
    "│                            ││⡇         ⢸ ⠰ ×  ⠘⠛⢻⠽⠙⠋   ×     ×⣹⣖⣻⣻⡿⣿×⣛⣗⣾×⣟⡷⣧⣾⡾⠃ ⠉⠒×⠚⠉⣤×⡞⢊⡾⠃     ⢸│"
    "│                            ││⡇×         ⠙⣶⣒×⣤⠤⠤⣴⣊           ⢠⣮⠟ ⢻⠓⠲×⠚⣿×⠼⣄⣀⣿×⡻×⢤⣤⣠⣄   ⢹⠈⠛⠋⠁      ⢸│"
    "│                            ││⡇   ⠰       ⠈⠁⠣⢜⢴⣶⡊⠿⠶⠤        ⢰⣟⣸⣉×⡚×⠑⡞⠒×⣧⠤⣬⠿⠉⠘⢳ ⡔⠚⢫⣿×⣶⠊⢽⡀         ⢸│"
    "│                            ││⡇         ×      ⠈⢓×⢛⣭⣽⣤⣄     ⠈⠙⠻⠿⠿⢼⣻×⠿⢮×⢬⡿⠋   ⠈⠚⠇ ⠠⣿⣝⣁⡴⣟⣿⡀ ×      ⢸│"
    "│                            ││⡇                 ⠘×⢿⣀⡀×⠈⠉⠉⡆       ⠈⢟⠧⣄⣿⣍⣏ ⡀  ×     ⠈⠻⠭⠽⢿⡿⣛×⢷⢶⡓⢤⡀  ⢸│"
    "│                            ││⡇                  ⠈⢹⣦×⢆ ⣀⡜         ⢳⢲×⣯⢿⠜⣎⠇           ⡤⠤⠊×⠉⠊⠣⡀ ⠠⡅ ⢺│"
    "│                            ││⡇                   ⢸× ⣯⠜           ⠘⣞⣉⠾⠃ ⠈            ⢣⣀⡤⠤⣤⡀ ⡸   ⣀⢸│"
    "│                            ││⡇        ×         ⢀⣟⣰⠎⠁      ×                             ⠑⠿⠁  ⣠⡾⢿│"
    "│                            ││⡇                  ⠸⣯⡏⠠⠄                      ⠐⠂                 ⠉ ⢸│"
    "│                            ││⡇                   ⠉⠉                            ×                ⢸│"
    "│                            ││⡇                    ⢀⡤⠂                   ⢀⣀     ⡀  ⡀⢀⣀⢀⣀⣀⣄       ⢸│"
    "│                            ││⡇                   ⡰⡟⡆         ⣀⣀⣀⡤⣠⠤⣄⡠⠒⠖⠊⠉ ⠉⢩⢦⠴⠉⠉⠉⠉⠈⠁ ⠉  ⠈⠉⠙⠒⠤⣀⣀ ⢸│"
    "│                            ││⡇      ⣀⣀⣀⣦⣦⢤⡀⣸⡿⠴⠒⠴⠬⠿⠃⡕       ⢠⡎⠉⠉            ⠈⠁                 ⡤⠃⢸│"
    "│                            ││⡇   ⡤⡤⠚      ⠈ ⠁   ⢰⡒⠊  ⢀⣀ ⣀⠔⠒⠁  ×                              ⢸  ⢸│"
    "│                            ││⡇  ⠹⠕⠒⢄            ⢏⡀⢀⣠⢀⠎⢸ ⠧⢆                                  ⢀⠞⠁ ⢸│"
    "│                            ││⡇   ⠐⡗⠁             ⠈⠚⢍⢈⡩⢥⠖⠊⠁                                  ⠈⡆  ⢸│"
    "│                            ││⡇ ⡀  ⣇                ⠈⠊                                        ⠈⢢ ⢸│"
    "│                            ││⣗⣋⣑⣄⣀⣈⣒⣆⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣋⣺│"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn mbtiles_hover_describes_the_nearest_feature() {
    let mut app = mbtiles_app();
    mbt(&mut app).wait_for_visible_tiles();
    let vertex = {
        let Some(MbtTileData::Loaded { geo_index, .. }) = mbt(&mut app).tiles.get(&(0, 0, 0))
        else {
            panic!("world tile should be loaded");
        };
        geo_index.iter().next().unwrap().vertices[0]
    };
    mbt(&mut app).find_hovered(vertex[0], vertex[1]);
    assert!(mbt(&mut app).hovered.is_some());
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌Properties - place feat 5 (t┐┌World Map - 0/0/0 - zoom 0.0  drag=pan  hover=info  q/Esc quit──────┐"
    "│class: continent            ││⡏⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹│"
    "│name: Oceania               ││⡇×               ⢀⣠⠤⠤⠤⡄  ⣠⣴⡏⠒⠤⡀                                    ⢸│"
    "│name:ar: أوقيانوسيا         ││⡇               ⣴⡊⡁ ⢀⣶⡯⠭⠾⠽⠉⠲⢭⠻⢴⡶     ⢀⡀   ⣀⣀       ⢠⡀              ⢸│"
    "│name:be: Акіянія            ││⡇           ⢀⡄⣤⣼⣤⣸⠍⢠⣺⡵⠊      ⢉⡟    ⠐⡝⢓⡟⠁  ⠈⠁       ⠱⣱⢄             ⢸│"
    "│name:ca: Oceania            ││⡇         ⠠⣊⠿⣧⣬⣽⣷⣿⣥⠊⠙⠷⠤⡀     ⢾⡗     ⠙⠋⠋     ⢀⢤⡶   ⢀⡤⡨⠛⠦⡄   ⢀⠤⣀     ⢸│"
    "│name:cs: Oceánie            ││⡇         ⡰⣳⠯⢽⣧⢿⡿⡿⢽⡧⡀  ⢣⡀   ⢰⣾⠃            ⢠⡟⠁⢠⢤⣄⡖⠋   ⠐⠗⠦⠖⣄⣀⡿⢍⡁    ⢸│"
    "│name:da: Oceanien           ││⡯⣀⢠⡖⠉⠉⠑⢲⠔⠚⠚⢻⣯⣤⣽⣻⡜⣿⢲⢲⡉⢇⡀⢘⢧  ⣠⡿⠟      ⢀⡴⡾⣟⠢⢄⣤⣀⠭⠖⠺⢹⡅         ⠉⠉  ⠈⠙⠒⠲⠖⢺│"
    "│name:de: Ozeanien           ││⡗⠻⢝⠿ ⢀ ⢸  ⠘⢛⣄×  ⢠⠼⣷⣒⡧⢽⠃⠈⢿⣠⡞⠉ ⠹⠭⠃  ⢀⡰⡵⣵⠟⢸⣽⠛⠉    ⠉   ×           ⣀⢀ ⢀⣺│"
    "│name:el: Ωκεανία            ││⡇ ⠈⣙⡶⠞⠉⠙⠺⣆  ⠉⠒⠲⢀⠳⢄⣀⡣⠈⠚⢄⡀⠈⠉      ×⣦⠈⢲⣷⣪⣿⢏⠉     ⣀⣀⡀     ⣀    ⣠⢔⠒⠛⡖⡯⠉⠁⢸│"
    "│name:en: Oceania            ││⡇        ⠻⢦⡤⠤×⠤⠽⢄⣤⡘⠃⢀⣠⣤⣿⡄       ⠻×××⣽⣦⣿⣞⣓⡢⢰⣖⠒⡛ ⢉⣉⢒×⡒⠖⠾⠥⢤⡜⠹⢄⣌⢿⠄ ⠛⠁  ⢸│"
    "│name:eo: Oceanio            ││⡇         ⢸ ⠰ ×  ⠘⠛⢻⠽⠙⠋   ×     ×⣹⣖⣻⣻⡿⣿×⣛⣗⣾×⣟⡷⣧⣾⡾⠃ ⠉⠒×⠚⠉⣤×⡞⢊⡾⠃     ⢸│"
    "│name:es: Oceanía            ││⡇×         ⠙⣶⣒×⣤⠤⠤⣴⣊           ⢠⣮⠟ ⢻⠓⠲×⠚⣿×⠼⣄⣀⣿×⡻×⢤⣤⣠⣄   ⢹⠈⠛⠋⠁      ⢸│"
    "│name:fi: Oseania            ││⡇   ⠰       ⠈⠁⠣⢜⢴⣶⡊⠿⠶⠤        ⢰⣟⣸⣉×⡚×⠑⡞⠒×⣧⠤⣬⠿⠉⠘⢳ ⡔⠚⢫⣿×⣶⠊⢽⡀         ⢸│"
    "│name:fr: Océanie            ││⡇         ×      ⠈⢓×⢛⣭⣽⣤⣄     ⠈⠙⠻⠿⠿⢼⣻×⠿⢮×⢬⡿⠋   ⠈⠚⠇ ⠠⣿⣝⣁⡴⣟⣿⡀ ×      ⢸│"
    "│name:fy: Oseaanje           ││⡇                 ⠘×⢿⣀⡀×⠈⠉⠉⡆       ⠈⢟⠧⣄⣿⣍⣏ ⡀  ×     ⠈⠻⠭⠽⢿⡿⣛×⢷⢶⡓⢤⡀  ⢸│"
    "│name:ga: An Aigéine         ││⡇                  ⠈⢹⣦×⢆ ⣀⡜         ⢳⢲×⣯⢿⠜⣎⠇           ⡤⠤⠊×⠉⠊⠣⡀ ⠠⡅ ⢺│"
    "│name:hi: ओशिआनिया           ││⡇                   ⢸× ⣯⠜           ⠘⣞⣉⠾⠃ ⠈            ⢣⣀⡤⠤⣤⡀ ⡸   ⣀⢸│" Hidden by multi-width symbols: [(12, " "), (15, " "), (17, " ")]
    "│name:hr: Oceanija           ││⡇        ×         ⢀⣟⣰⠎⠁      ×                             ⠑⠿⠁  ⣠⡾⢿│"
    "│name:hu: Óceánia            ││⡇                  ⠸⣯⡏⠠⠄                      ⠐⠂                 ⠉ ⢸│"
    "│name:is: Eyjaálfa           ││⡇                   ⠉⠉                            ×                ⢸│"
    "│name:it: Oceania            ││⡇                    ⢀⡤⠂                   ⢀⣀     ⡀  ⡀⢀⣀⢀⣀⣀⣄       ⢸│"
    "│name:kn: ಒಷ್ಯಾನಿಯ             ││⡇                   ⡰⡟⡆         ⣀⣀⣀⡤⣠⠤⣄⡠⠒⠖⠊⠉ ⠉⢩⢦⠴⠉⠉⠉⠉⠈⠁ ⠉  ⠈⠉⠙⠒⠤⣀⣀ ⢸│" Hidden by multi-width symbols: [(13, " ")]
    "│name:ku: Okyanûsya          ││⡇      ⣀⣀⣀⣦⣦⢤⡀⣸⡿⠴⠒⠴⠬⠿⠃⡕       ⢠⡎⠉⠉            ⠈⠁                 ⡤⠃⢸│"
    "│name:la: Oceania            ││⡇   ⡤⡤⠚      ⠈ ⠁   ⢰⡒⠊  ⢀⣀ ⣀⠔⠒⠁  ×                              ⢸  ⢸│"
    "│name:latin: Oceania         ││⡇  ⠹⠕⠒⢄            ⢏⡀⢀⣠⢀⠎⢸ ⠧⢆                                  ⢀⠞⠁ ⢸│"
    "│name:lt: Okeanija           ││⡇   ⠐⡗⠁             ⠈⠚⢍⢈⡩⢥⠖⠊⠁                                  ⠈⡆  ⢸│"
    "│name:nl: Oceanië            ││⡇ ⡀  ⣇                ⠈⠊                                        ⠈⢢ ⢸│"
    "│name:no: Oseania            ││⣗⣋⣑⣄⣀⣈⣒⣆⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣋⣺│"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn mbtiles_center_tile_zoom_and_pan_move_the_viewport() {
    let mut app = mbtiles_app();
    mbt(&mut app).set_viewport_to_tile(1, 0, 0).unwrap();
    assert_eq!(mbt(&mut app).zoom_level(), 1);
    mbt(&mut app).zoom_wheel_at(0.25, 0.25, true);
    mbt(&mut app).zoom_wheel_at(0.25, 0.25, true);
    assert_eq!(mbt(&mut app).zoom_level(), 2);
    mbt(&mut app).pan_by_pixels(100, 50, -50, 0);
    assert!(
        mbt(&mut app).vp_x0 > 0.0,
        "panning left moves the viewport east"
    );
    mbt(&mut app).wait_for_visible_tiles();
    insta::assert_snapshot!(render(&mut app), @r#"
    "┌Properties──────────────────┐┌World Map - 2/1/1 - zoom 2.0  drag=pan  hover=info  q/Esc quit──────┐"
    "│Hover over a feature to     ││⡇⢠⡪⠜      ⠰⡁  ⣀⠤⠒⡡⠊                                 ⣼⠃             ⢸│"
    "│inspect properties          ││⡧⠓⡣⡄     ⢀⠔⠁⢰⠭⣤⠒⠊                                  ⡰⢹              ⢸│"
    "│                            ││⡇⢸⠈⠙⠄   ⡠⠊   ⠉⠒⠭⣶⣤⣀                              ⢀ ⡇⠸⡀             ⢸│"
    "│                            ││⣷⠊     ⠈⢒⠆    ⠲⡛⠫⡍                              ⢰⠙⣤⡣⠔⠓             ⢸│"
    "│                            ││⡏⠉⠒⠒⠊⠉⠑⠊⠁      ⠑⠼⠶⠖⠚⠛⠭⣒⢄                        ⠸⠊⢈⠎⢢              ⢸│"
    "│                            ││⡏⠉⠒⠊⠉⠉⠉⢢               ⢹⢆                         ⢸ ⡬⠆             ⢸│"
    "│                            ││⡗⠤⠤⠔⠒⠒⠤⠊                ⠹⣆                    ⢀⣀⡀⢠⠼⢀⡸⠄             ⢸│"
    "│                            ││⡇⡤⢔⠆⡠⠔⡄⡖⠒⢢               ⠹⣆                   ⠸⡀⠈⠁⣀⡨⠆              ⢸│"
    "│                            ││⡟ ⠘⠜  ⠈⡎⣒⠶⢗               ⢹⢆⢀                  ⢧ ⣠⣘⡄               ⢸│"
    "│                            ││⣇⡀     ⠉   ⠉⣆⢤           ⠠⣃⢌⠛⡄               ⢀⠖⠉⢠⡀⠑⡅               ⢸│"
    "│                            ││⡗⠥⣀⣀    ⢀     ⠉⠒⢄          ⡖⠳⢼               ⢠⠃⢀⣜⣑⣤⡃               ⢸│"
    "│                            ││⣇  ⢸⠉⠛⢖⠊⠁⠑⢄     ⠈⣆        ⠘⠢⠔⣺               ⡬⢍⡹⡻⠝⠉                ⢸│"
    "│                            ││⡇⢱⢀⠎  ⡜   ⡭⡒⢄   ⠉⠒⠤⡀       ⢀⠔⠁⡇            ⡷⠴⠕⠒⠉                   ⢸│"
    "│                            ││⡇⠈⠊⣀ ⢀⡸  ⠘⠴⠁⢀⠇     ⠘⠤⡄     ⢎  ⡇        ⢀⠤⣀⡼⠁                       ⢸│"
    "│                            ││⡏⠉⢹⡭⡉⠉⠉⠉⠉⣉⣉⣉⡏⠉⠉⠉⠹⡉⠫⡉⡩⠋⠉⠉⠉⠉⠉⠫⡉⢝⠋⠉⠉⠉⠉⠉⠉⢉⡽⠝⠛⠉⠉⠉⠉⠉⠉⠉⠉⠽⢍⡙⠝⠉⠛⠉⠹⡉⠉⠉⠉⠉⠉⠉⠉⠉⠉⢹│"
    "│                            ││⣇⡠⢳⠁⠈⡑⢢⡀⠐⠥⠤⠴⠤⡀   ⠈⢢⠈        ⠸⡈⢢     ⢠⠇           ⠈⢛⣅⡀ ⢀⣀⠤⠊         ⢸│"
    "│                            ││⡇ ⠉⠣⢊⢬⠍⢉ ⢀⣀ ⡀⠘⠤⣀⡑⡦⢄⠇         ⢣⡱    ⢠⠏               ⠈⠉⠁            ⢸│"
    "│                            ││⡇    ⠁ ⠘⠃⡇ ⠉⠈⠢⣀⣀⠈⠙⠂           ⠫⢕⣤⣄⡀⣼                               ⢸│"
    "│                            ││⡇        ⡸     ⢸  ⢠⢢             ⠈⠚⠃                               ⢸│"
    "│                            ││⡇       ⠈⢢     ⠈⠣⠒⠁ ⠣⡀                                         ⢰⢒⣖⡀⢸│"
    "│                            ││⡗⠤⢄⡀     ⢈⠆          ⠵⣀                                        ⢇×⣰⡁⢸│"
    "│                            ││⡇  ⠈⠉⠑⡆⠠⡔⠊             ⠑⠢⡀                                  ⣀⠤⣖⣪⠒⢄⠘⣼│"
    "│                            ││⡇     ⢇ ⢱                ⢱                                  ⣱ ⣠⢃⣏⠁ ⢸│"
    "│                            ││⡇      ⠑⠁        ⢀⣀⣀⣀⣀⣀⠤⡪⣳⡁                                  ⠉ ⣨⠽⡡⣤⢼│"
    "│                            ││⣧⠴⣢⣀⡀          ⣠⡴⠕⢒⠖⠉⠂ ⢴⠁⠈⠉⣽                                    ⣤⠒⠵⢹│"
    "│                            ││⡗⠙⣤⠿⣷⡢⢄⡀      ⢊⠏⢱ ⠈⠶⣖⣄⢆⠉⠉⠙⠋⠓⠁                                    ⠉⢢⢸│"
    "│                            ││⡇⢐⢯⠊⢠⣻⡜⢙⣠⣤⡤⠊⠉⠉⠁⡤⠒⠉⣟⡡⠔⠊⠁                                      ⡠⠤⠤⣀⣀⣎⢸│"
    "│                            ││⡇⠈⠦⠃ ⠾⠶⠟⠋   ⢀⣀⠼⠄                                            ⠘⡶⢴    ⢹│"
    "└────────────────────────────┘└────────────────────────────────────────────────────────────────────┘"
    "#);
}

fn render_with_areas(app: &mut App, width: u16, height: u16) -> PanelAreas {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut areas = PanelAreas::default();
    terminal.draw(|f| render_frame(f, app, &mut areas)).unwrap();
    areas
}

/// Feed one mouse event against the panels of a `width` x `height` screen.
fn send(
    app: &mut App,
    areas: &PanelAreas,
    clicks: &mut MouseState,
    (width, height): (u16, u16),
    kind: MouseEventKind,
    column: u16,
    row: u16,
) {
    let event = MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse(app, areas, Rect::new(0, 0, width, height), event, clicks).unwrap();
}

/// Screen cell that shows tile coordinate `(x, y)` on the map.
fn map_cell(map: Rect, bounds: (f64, f64, f64, f64), x: f64, y: f64) -> (u16, u16) {
    let rx = (x - bounds.0) / (bounds.2 - bounds.0);
    let ry = (bounds.3 - y) / (bounds.3 - bounds.1);
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let col = map.x + (rx * f64::from(map.width)) as u16;
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let row = map.y + (ry * f64::from(map.height)) as u16;
    (col, row)
}

const SCREEN: (u16, u16) = (WIDTH, HEIGHT);
const LEFT: MouseEventKind = MouseEventKind::Down(MouseButton::Left);

#[test]
fn clicking_a_tree_row_selects_it_and_a_double_click_expands_it() {
    let mut app = sample_app();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let tree = areas.tree.unwrap();
    let mut clicks = MouseState::default();
    let row = tree.y + 1 + 2;
    send(&mut app, &areas, &mut clicks, SCREEN, LEFT, tree.x + 4, row);
    assert_eq!(app.selected_item(), &TreeItem::Layer(1));
    let rows_before = app.tree_items.len();
    send(&mut app, &areas, &mut clicks, SCREEN, LEFT, tree.x + 4, row);
    assert!(
        app.tree_items.len() > rows_before,
        "double-click expands the layer"
    );
}

#[test]
fn moving_over_the_map_hovers_and_clicking_selects() {
    let mut app = sample_app();
    let screen = (160, 100);
    let areas = render_with_areas(&mut app, screen.0, screen.1);
    let map = areas.map.unwrap();
    let (col, row) = map_cell(map, app.get_bounds(), 1000.0, 3000.0);
    let mut clicks = MouseState::default();
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        MouseEventKind::Moved,
        col,
        row,
    );
    assert_eq!(
        app.hovered.as_ref().map(|h| (h.layer, h.feat, h.part)),
        Some((2, 0, None))
    );
    send(&mut app, &areas, &mut clicks, screen, LEFT, col, row);
    assert_eq!(app.selected_item(), &TreeItem::Layer(2));
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        MouseEventKind::Moved,
        map.x + 1,
        map.y + 1,
    );
    assert!(app.hovered.is_none(), "empty map space clears the hover");
}

#[test]
fn the_wheel_scrolls_the_panel_under_the_cursor() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Char('*'));
    let screen = (60, 16);
    let areas = render_with_areas(&mut app, screen.0, screen.1);
    let tree = areas.tree.unwrap();
    let props = areas.props.unwrap();
    let mut clicks = MouseState::default();
    let down = MouseEventKind::ScrollDown;
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        down,
        tree.x + 2,
        tree.y + 2,
    );
    assert!(app.tree_scroll > 0);
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        down,
        props.x + 2,
        props.y + 1,
    );
    assert!(app.properties_scroll > 0);
}

#[test]
fn dragging_the_divider_resizes_the_split() {
    let mut app = sample_app();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let left = areas.left.unwrap();
    let mut clicks = MouseState::default();
    let x = left.x + left.width;
    send(&mut app, &areas, &mut clicks, SCREEN, LEFT, x, left.y + 5);
    assert_eq!(app.resizing, Some(ResizeHandle::LeftRight));
    let drag = MouseEventKind::Drag(MouseButton::Left);
    send(&mut app, &areas, &mut clicks, SCREEN, drag, 50, left.y + 5);
    assert_eq!(app.left_pct, 50);
    let up = MouseEventKind::Up(MouseButton::Left);
    send(&mut app, &areas, &mut clicks, SCREEN, up, 50, left.y + 5);
    assert!(app.resizing.is_none());
}

#[test]
fn mbtiles_mouse_pans_and_zooms() {
    let mut app = mbtiles_app();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let map = areas.map.unwrap();
    let mut clicks = MouseState::default();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        map.x + 10,
        map.y + 10,
    );
    assert!(mbt(&mut app).map_drag_last.is_some());
    let drag = MouseEventKind::Drag(MouseButton::Left);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        drag,
        map.x + 20,
        map.y + 10,
    );
    assert!(
        mbt(&mut app).vp_x0 < 0.0,
        "dragging east moves the viewport west"
    );
    let up = MouseEventKind::Up(MouseButton::Left);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        up,
        map.x + 20,
        map.y + 10,
    );
    assert!(mbt(&mut app).map_drag_last.is_none());
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        MouseEventKind::ScrollUp,
        map.x + 10,
        map.y + 10,
    );
    assert!((mbt(&mut app).zoom_f - 0.5).abs() < 1e-9);
}

#[test]
fn help_and_error_popup_take_the_mouse() {
    let mut app = sample_app();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let mut clicks = MouseState::default();
    press(&mut app, KeyCode::Char('?'));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        MouseEventKind::ScrollDown,
        10,
        10,
    );
    assert_eq!(app.help_scroll, 1);
    press(&mut app, KeyCode::Char('x'));
    app.error_popup = Some(("broken.mlt".into(), "nope".into()));
    send(&mut app, &areas, &mut clicks, SCREEN, LEFT, 10, 10);
    assert!(app.error_popup.is_none());
}

fn ui_args(path: PathBuf, center_tile: Option<&str>) -> UiArgs {
    UiArgs {
        path,
        center_tile: center_tile.map(str::to_string),
    }
}

#[test]
fn build_app_picks_the_mode_from_the_path() {
    let browser = build_app(&ui_args(fixtures_dir(), None)).unwrap();
    assert_eq!(browser.mode, ViewMode::FileBrowser);
    let file = build_app(&ui_args(fixtures_dir().join("line-boolean.mvt"), None)).unwrap();
    assert_eq!(file.mode, ViewMode::LayerOverview);
    let archive = test_dir("fixtures/omt.max1.mbtiles");
    let mut map = build_app(&ui_args(archive.clone(), Some("1/1/0"))).unwrap();
    assert_eq!(map.mode, ViewMode::MbtilesMap);
    assert_eq!(mbt(&mut map).center_tile_xyz(), (1, 1, 0));
    assert!(build_app(&ui_args(fixtures_dir(), Some("1/0/0"))).is_err());
    assert!(build_app(&ui_args(archive, Some("1/9/0"))).is_err());
    assert!(build_app(&ui_args(test_dir("nope"), None)).is_err());
}

#[test]
fn center_tile_specs_are_validated() {
    assert_eq!(parse_center_tile_xyz(" 3 / 2 / 1 ").unwrap(), (3, 2, 1));
    for bad in ["1/2", "z/0/0", "1/x/0", "1/0/y", "40/0/0", "1/2/0"] {
        assert!(parse_center_tile_xyz(bad).is_err(), "{bad}");
    }
}

fn tick_until(app: &mut App, done: impl Fn(&App) -> bool) {
    for _ in 0..500 {
        tick(app);
        if done(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("condition not reached");
}

#[test]
fn tick_loads_previews() {
    let mut app = file_browser_app();
    tick_until(&mut app, |a| a.preview_fc.is_some());
    let first = app.preview_tile_path.clone().unwrap();
    press(&mut app, KeyCode::Down);
    tick_until(&mut app, |a| a.preview_tile_path.as_ref() != Some(&first));
    assert!(app.preview_fc.is_some());
}

#[test]
fn tick_surfaces_a_loader_failure_as_a_popup() {
    let path = test_dir("missing.mbtiles");
    let mut app = App::new_mbtiles(MbtilesState::new(path.clone()), path);
    tick_until(&mut app, |a| a.error_popup.is_some());
    let (title, msg) = app.error_popup.clone().unwrap();
    assert!(title.ends_with("missing.mbtiles"));
    assert!(msg.contains("failed"), "{msg}");
    let state = mbt(&mut app);
    state.request_tile(1, 0, 0);
    assert!(
        matches!(state.tiles.get(&(1, 0, 0)), Some(MbtTileData::Error(_))),
        "a request after the loader died is an error, not a hang"
    );
}

#[test]
fn tick_pumps_mbtiles_results() {
    let mut app = mbtiles_app();
    tick_until(&mut app, |a| {
        a.mbt_state
            .as_ref()
            .is_some_and(|m| matches!(m.tiles.get(&(0, 0, 0)), Some(MbtTileData::Loaded { .. })))
    });
    assert!(app.needs_redraw);
}

#[test]
fn help_scrolls_with_the_keyboard() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Char('?'));
    press(&mut app, KeyCode::Down);
    assert_eq!(app.help_scroll, 1);
    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.help_scroll, 11);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.help_scroll, 10);
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.help_scroll, 0);
    press(&mut app, KeyCode::End);
    assert_eq!(app.help_scroll, u16::MAX);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.help_scroll, 0);
    assert!(app.show_help);
}

#[test]
fn keys_resize_the_splits_and_page_through_the_tree() {
    let mut app = sample_app();
    let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    let shift = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT);
    assert!(!handle_key(&mut app, ctrl('h')));
    assert_eq!(app.left_pct, 25);
    assert!(!handle_key(&mut app, ctrl('l')));
    assert_eq!(app.left_pct, 30);
    assert!(!handle_key(&mut app, shift('J')));
    assert_eq!(app.features_pct, 45);
    assert!(!handle_key(&mut app, shift('K')));
    assert_eq!(app.features_pct, 50);

    press(&mut app, KeyCode::Char('*'));
    render_sized(&mut app, 60, 16);
    press(&mut app, KeyCode::PageDown);
    assert!(app.selected_index > 0);
    press(&mut app, KeyCode::End);
    assert_eq!(app.selected_index, app.tree_items.len() - 1);
    assert!(app.tree_scroll > 0, "the selection scrolls into view");
    press(&mut app, KeyCode::PageUp);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.tree_scroll, 0);
}

#[test]
fn plus_minus_and_left_walk_the_tree() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    assert!(app.tree_items.len() > 4, "+ expands the layer");
    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    assert!(app.expanded_features.contains(&(0, 1)));
    press(&mut app, KeyCode::Down);
    assert_eq!(
        app.selected_item(),
        &TreeItem::SubFeature {
            layer: 0,
            feat: 1,
            part: 0
        }
    );
    press(&mut app, KeyCode::Char('-'));
    assert_eq!(
        app.selected_item(),
        &TreeItem::Feature { layer: 0, feat: 1 }
    );
    assert!(
        !app.expanded_features.contains(&(0, 1)),
        "- on a part collapses the feature"
    );
    press(&mut app, KeyCode::Char('-'));
    assert_eq!(
        app.selected_item(),
        &TreeItem::Layer(0),
        "- on a feature collapses the layer"
    );
    press(&mut app, KeyCode::Char('-'));
    assert_eq!(
        app.selected_item(),
        &TreeItem::Layer(0),
        "- on a collapsed layer is a no-op"
    );

    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Left);
    assert_eq!(
        app.selected_item(),
        &TreeItem::Feature { layer: 0, feat: 1 }
    );
    press(&mut app, KeyCode::Left);
    assert_eq!(app.selected_item(), &TreeItem::Layer(0));
    press(&mut app, KeyCode::Left);
    assert_eq!(app.selected_item(), &TreeItem::All);
    press(&mut app, KeyCode::Left);
    assert_eq!(
        app.mode,
        ViewMode::LayerOverview,
        "no browser to go back to"
    );
    press(&mut app, KeyCode::Char('+'));
    assert_eq!(app.selected_item(), &TreeItem::All, "+ on All is a no-op");

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('-'));
    assert_eq!(
        app.selected_item(),
        &TreeItem::Feature { layer: 0, feat: 1 }
    );
    assert!(
        !app.expanded_features.contains(&(0, 1)),
        "- collapses expanded parts first"
    );
    press(&mut app, KeyCode::Char('*'));
    press(&mut app, KeyCode::Char('*'));
    assert_eq!(app.tree_items.len(), 4, "* twice collapses every layer");

    let mut app = file_browser_app();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Left);
    assert_eq!(
        app.mode,
        ViewMode::FileBrowser,
        "Left on All returns to the browser"
    );
}

#[test]
fn file_browser_keys_page_and_bad_rows_show_a_popup() {
    let mut app = file_browser_app();
    render(&mut app);
    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.selected_file_index, 5);
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.selected_file_index, 0);
    press(&mut app, KeyCode::End);
    assert_eq!(app.selected_file_index, 5);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.selected_file_index, 0);

    let base = fixtures_dir();
    let rows = vec![
        analyze_row(base.join("broken.mvt"), &base),
        LsRow::Loading {
            path: base.join("missing.mvt"),
        },
    ];
    let mut app = App::new_file_browser(rows, None, base);
    assert!(matches!(app.files[0], LsRow::Error { size: None, .. }));
    let screen = render(&mut app);
    assert!(screen.contains("Select a file to view"), "{screen}");
    press(&mut app, KeyCode::Enter);
    assert!(app.error_popup.is_some(), "an error row opens its message");
    app.error_popup = None;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(
        app.error_popup.is_some(),
        "a file that fails to decode opens its message"
    );
}

#[test]
fn header_clicks_sort_every_column_both_ways() {
    let mut app = file_browser_app();
    let first = |app: &App| {
        app.files[0]
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    };
    app.handle_file_header_click(FileSortColumn::File);
    assert_eq!(first(&app), "line-boolean.mvt");
    app.handle_file_header_click(FileSortColumn::File);
    assert_eq!(first(&app), "polygon-boolean.mvt");
    app.handle_file_header_click(FileSortColumn::Size);
    assert_eq!(first(&app), "point-boolean.mvt");
    app.handle_file_header_click(FileSortColumn::Size);
    assert_eq!(first(&app), "multipolygon-boolean.mvt");
    for col in [
        FileSortColumn::EncPct,
        FileSortColumn::Layers,
        FileSortColumn::Features,
    ] {
        app.handle_file_header_click(col);
        assert_eq!(app.filtered_file_indices.len(), 6, "{col:?}");
    }

    let base = fixtures_dir();
    let mut rows = analyze_tile_files(
        &[
            base.join("line-boolean.mvt"),
            base.join("point-boolean.mvt"),
        ],
        &base,
        SCAN_FLAGS,
    );
    rows.push(LsRow::Error {
        path: base.join("b.mvt"),
        size: Some(9),
        error: "bad".into(),
    });
    rows.push(analyze_row(base.join("a.mvt"), &base));
    let mut app = App::new_file_browser(rows, None, base);
    app.handle_file_header_click(FileSortColumn::Size);
    let names: Vec<String> = app
        .files
        .iter()
        .map(|r| r.path().file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        names,
        ["point-boolean.mvt", "line-boolean.mvt", "a.mvt", "b.mvt"]
    );
    app.handle_file_header_click(FileSortColumn::File);
    app.handle_file_header_click(FileSortColumn::Layers);
    assert!(
        matches!(app.files[2], LsRow::Error { .. }),
        "error rows sort after info rows"
    );

    let (tx, rx) = mpsc::channel::<Vec<LsRow>>();
    let mut app = App::new_file_browser(Vec::new(), Some(rx), fixtures_dir());
    app.handle_file_header_click(FileSortColumn::Size);
    drop(tx);
}

#[test]
fn empty_loading_and_filtered_out_browser_states_render() {
    let mut app = file_browser_app();
    let path = app.get_selected_file().unwrap().path().to_path_buf();
    app.preview_load_requested = Some(path);
    assert!(render(&mut app).contains("Loading…"));

    app.ext_filters.insert("xyz".into());
    app.rebuild_filtered_files();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    assert!(render(&mut app).contains("No files match"));
    let info = areas.info.unwrap();
    let mut clicks = MouseState::default();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        info.x + 2,
        info.y + 3,
    );
    assert_eq!(
        app.filtered_file_indices.len(),
        6,
        "[Reset filters] in the info panel"
    );
}

#[test]
fn filter_clicks_toggle_extensions_and_algorithms() {
    let mut app = file_browser_app();
    handle_filter_click(&mut app, 3);
    assert!(app.ext_filters.contains("mvt"));
    assert_eq!(app.filtered_file_indices.len(), 6);
    handle_filter_click(&mut app, 3);
    assert!(app.ext_filters.is_empty(), "a second click toggles it off");

    let mut app = browser_over(test_dir("synthetic/0x01-rust"));
    assert!(app.files.len() > 100);
    let algos = collect_file_algorithms(&app.files);
    assert!(!algos.is_empty());
    let geoms = collect_file_geometries(&app.files).len();
    let exts = collect_extensions(&app.files).len();
    let first_algo_row = 3 + exts + 2 + geoms + 2;
    handle_filter_click(&mut app, first_algo_row);
    assert_eq!(app.algo_filters.len(), 1);
    assert!(app.filtered_file_indices.len() < app.files.len());
    render(&mut app);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.mode,
        ViewMode::LayerOverview,
        "an MLT opens from the browser"
    );
}

#[test]
fn file_browser_dividers_and_side_panels_take_the_mouse() {
    let mut app = file_browser_app();
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let mut clicks = MouseState::default();
    let drag = MouseEventKind::Drag(MouseButton::Left);
    let up = MouseEventKind::Up(MouseButton::Left);
    let fl = areas.file_left.unwrap();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        fl.x + fl.width,
        fl.y + 5,
    );
    assert_eq!(app.resizing, Some(ResizeHandle::FileBrowserLeftRight));
    send(&mut app, &areas, &mut clicks, SCREEN, drag, 60, fl.y + 5);
    assert_eq!(app.file_left_pct, 60);
    send(&mut app, &areas, &mut clicks, SCREEN, up, 60, fl.y + 5);

    let pa = areas.preview.unwrap();
    let fa = areas.filter.unwrap();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        pa.x + 2,
        pa.y + pa.height,
    );
    assert_eq!(app.resizing, Some(ResizeHandle::FileBrowserPreviewFilter));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        drag,
        pa.x + 2,
        pa.y + pa.height + 4,
    );
    assert_ne!(app.file_preview_pct, 33);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        up,
        pa.x + 2,
        pa.y + pa.height + 4,
    );
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        fa.x + 2,
        fa.y + fa.height,
    );
    assert_eq!(app.resizing, Some(ResizeHandle::FileBrowserFilterInfo));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        drag,
        fa.x + 2,
        fa.y + fa.height - 3,
    );
    assert_ne!(app.file_filter_pct, 33);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        up,
        fa.x + 2,
        fa.y + fa.height - 3,
    );

    let down = MouseEventKind::ScrollDown;
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        down,
        fa.x + 2,
        fa.y + 3,
    );
    assert!(app.filter_scroll > 0);
    let info = areas.info.unwrap();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        down,
        info.x + 2,
        info.y + 3,
    );
    assert!(app.file_info_scroll > 0);
    let table = app.file_table_area.unwrap();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        down,
        table.x + 5,
        table.y + 5,
    );
    assert!(
        app.selected_file_index > 0,
        "the wheel over the table moves the selection"
    );
}

#[test]
fn layer_view_dividers_tree_hover_and_map_wheel() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Char('*'));
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let tree = areas.tree.unwrap();
    let geom = areas.geom.unwrap();
    let map = areas.map.unwrap();
    let mut clicks = MouseState::default();
    let drag = MouseEventKind::Drag(MouseButton::Left);
    let up = MouseEventKind::Up(MouseButton::Left);

    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        MouseEventKind::Moved,
        tree.x + 4,
        tree.y + 3,
    );
    assert_eq!(app.hovered, Some(HoveredInfo::new(2, 0, 0, None)));
    assert!(render(&mut app).contains("Properties (feat 0, hover)"));

    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        tree.x + 2,
        tree.y + tree.height,
    );
    assert_eq!(app.resizing, Some(ResizeHandle::FeaturesProperties));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        drag,
        tree.x + 2,
        tree.y + tree.height + 3,
    );
    assert_ne!(app.features_pct, 50);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        up,
        tree.x + 2,
        tree.y + tree.height + 3,
    );

    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        geom.x + 2,
        geom.y,
    );
    assert_eq!(app.resizing, Some(ResizeHandle::PropertiesGeometry));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        drag,
        geom.x + 2,
        geom.y + 3,
    );
    assert_ne!(app.properties_pct, 50);
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        up,
        geom.x + 2,
        geom.y + 3,
    );

    let before = app.selected_index;
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        MouseEventKind::ScrollDown,
        map.x + 5,
        map.y + 5,
    );
    assert_eq!(
        app.selected_index, before,
        "the wheel over the map does nothing"
    );
}

#[test]
fn mbtiles_hover_and_side_panel_wheel_through_the_mouse() {
    let mut app = mbtiles_app();
    mbt(&mut app).wait_for_visible_tiles();
    let screen = (160, 100);
    let areas = render_with_areas(&mut app, screen.0, screen.1);
    let map = areas.map.unwrap();
    let left = areas.left.unwrap();
    let mut clicks = MouseState::default();
    let vertex = {
        let Some(MbtTileData::Loaded { geo_index, .. }) = mbt(&mut app).tiles.get(&(0, 0, 0))
        else {
            panic!("world tile");
        };
        geo_index.iter().next().unwrap().vertices[0]
    };
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let col = map.x + (vertex[0] * f64::from(map.width)) as u16;
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let row = map.y + (vertex[1] * f64::from(map.height)) as u16;
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        MouseEventKind::Moved,
        col,
        row,
    );
    assert!(mbt(&mut app).hovered.is_some());
    send(
        &mut app,
        &areas,
        &mut clicks,
        screen,
        MouseEventKind::ScrollDown,
        left.x + 2,
        left.y + 2,
    );
    assert!(app.properties_scroll > 0);
}

#[test]
fn mbtiles_hover_panel_explains_every_tile_state() {
    let mut app = mbtiles_app();
    let hover = |tile, layer_idx, feat_idx| MbtHoveredInfo {
        tile,
        layer_idx,
        feat_idx,
    };
    mbt(&mut app).hovered = Some(hover((5, 0, 0), 0, 0));
    assert!(render(&mut app).contains("Tile loading"));
    mbt(&mut app).tiles.insert((5, 0, 0), MbtTileData::Empty);
    assert!(render(&mut app).contains("Tile empty"));
    mbt(&mut app)
        .tiles
        .insert((5, 0, 0), MbtTileData::Error("boom".into()));
    assert!(render(&mut app).contains("Tile error: boom"));
    mbt(&mut app).request_tile_with_ancestors(0, 0, 0);
    mbt(&mut app).hovered = None;
    render(&mut app);
    mbt(&mut app).wait_for_visible_tiles();
    mbt(&mut app).hovered = Some(hover((0, 0, 0), 99, 0));
    assert!(render(&mut app).contains("(feature not found)"));
    mbt(&mut app).hovered = Some(hover((0, 0, 0), 0, 99_999));
    assert!(render(&mut app).contains("(feature not found)"));
    press(&mut app, KeyCode::Char('?'));
    assert!(render(&mut app).contains("Zoom ±0.5 levels"));
}

#[test]
fn mbtiles_viewport_edges_and_pruning_at_depth() {
    let mut app = mbtiles_app();
    let state = mbt(&mut app);
    assert!(state.set_viewport_to_tile(1, 5, 0).is_err());
    assert!(state.set_viewport_to_tile(40, 0, 0).is_err());
    state.zoom_wheel_at(0.5, 0.5, false);
    assert!(state.zoom_f.abs() < 1e-9, "cannot zoom out past the world");
    state.pan_by_pixels(0, 0, 5, 5);
    assert!(state.vp_x0.abs() < 1e-9, "a zero-sized map does not pan");
    state.set_viewport_to_tile(2, 1, 1).unwrap();
    state.wait_for_visible_tiles();
    state.find_hovered(0.3, 0.3);
    for x in 0..300u32 {
        state.tiles.insert((9, x, 511), MbtTileData::Empty);
    }
    state.tiles.insert((5, 31, 31), MbtTileData::Empty);
    state.hovered = Some(MbtHoveredInfo {
        tile: (5, 31, 31),
        layer_idx: 0,
        feat_idx: 0,
    });
    state.prune_tile_cache_if_needed();
    assert!(
        state.hovered.is_none(),
        "a hover on a pruned tile is dropped"
    );
    assert!(state.tiles.contains_key(&(0, 0, 0)), "ancestors survive");
    assert!(!state.tiles.contains_key(&(5, 31, 31)));
}

#[test]
fn help_renders_for_the_file_browser() {
    let mut app = file_browser_app();
    press(&mut app, KeyCode::Char('?'));
    assert!(render(&mut app).contains("Double-click row"));
}

#[test]
fn enter_and_tree_clicks_drill_through_features_and_parts() {
    let mut app = sample_app();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('+'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(
        app.expanded_features.contains(&(0, 1)),
        "Enter expands a multi feature"
    );
    press(&mut app, KeyCode::Enter);
    assert!(
        !app.expanded_features.contains(&(0, 1)),
        "Enter again collapses it"
    );
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.tree_items.len(),
        6,
        "Enter on a plain polygon is a no-op"
    );

    press(&mut app, KeyCode::Left);
    assert_eq!(app.selected_item(), &TreeItem::Layer(0));
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    let tree = areas.tree.unwrap();
    let mut clicks = MouseState::default();
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        tree.x + 6,
        tree.y + 1 + 3,
    );
    assert_eq!(
        app.selected_item(),
        &TreeItem::Feature { layer: 0, feat: 1 }
    );
    assert!(
        app.expanded_features.contains(&(0, 1)),
        "clicking a multi feature opens its parts"
    );
    let areas = render_with_areas(&mut app, WIDTH, HEIGHT);
    std::thread::sleep(Duration::from_millis(450));
    send(
        &mut app,
        &areas,
        &mut clicks,
        SCREEN,
        LEFT,
        tree.x + 8,
        tree.y + 1 + 4,
    );
    assert_eq!(
        app.selected_item(),
        &TreeItem::SubFeature {
            layer: 0,
            feat: 1,
            part: 0
        }
    );
    assert_eq!(app.tree_items[4].layer_feat_part(), Some((0, 1, Some(0))));

    let mut app = mbtiles_app();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::PageDown);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.selected_index, 0, "the map view has no tree to move in");
}
