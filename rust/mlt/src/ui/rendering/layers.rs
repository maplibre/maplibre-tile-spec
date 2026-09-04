use std::collections::{BTreeMap, BTreeSet};

use mlt_core::GeometryType;
use mlt_core::geo_types::{
    Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::geojson::Feature;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Modifier, Span, Style};
use ratatui::widgets::{Paragraph, Wrap};
use usize_cast::IntoUsize as _;

use crate::ui::mbt::MbtTileData;
use crate::ui::rendering::scrollbar::{Scroll, render_hscrollbar, render_vscrollbar, wrapped_rows};
use crate::ui::state::{App, TreeItem, ViewMode};
use crate::ui::tile::{geometry_coord_count, polygon_coord_count};
use crate::ui::{
    CLR_HOVERED_TREE, STYLE_LABEL, STYLE_SELECTED, block_with_title, feature_suffix,
    geometry_color, geometry_type_name, is_ring_ccw, stat_line, sub_feature_suffix,
};

pub fn render_tree_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let lines: Vec<Line<'static>> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let (text, base) = match item {
                TreeItem::All => ("All".into(), None),
                TreeItem::Layer(li) => {
                    let g = &app.layer_groups[*li];
                    let n = g.feature_indices.len();
                    let first = g
                        .feature_indices
                        .first()
                        .map(|&gi| geometry_type_name(&app.tile.fc.features[gi].geometry));
                    let all_same = first.is_some_and(|ft| {
                        g.feature_indices
                            .iter()
                            .all(|&gi| geometry_type_name(&app.tile.fc.features[gi].geometry) == ft)
                    });
                    let label = if all_same && n > 0 {
                        format!("{}s", first.unwrap())
                    } else {
                        "features".into()
                    };

                    (
                        format!("  Layer: {} ({n} {label}, extent {})", g.name, g.extent),
                        None,
                    )
                }
                TreeItem::Feature { layer, feat } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    (
                        format!(
                            "    Feat {feat}: {}{}",
                            geometry_type_name(geom),
                            feature_suffix(geom)
                        ),
                        Some(geometry_color(geom)),
                    )
                }
                TreeItem::SubFeature { layer, feat, part } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    let n = match geom {
                        Geometry::<i32>::MultiPoint(_) => "Point",
                        Geometry::<i32>::MultiLineString(_) => "LineString",
                        Geometry::<i32>::MultiPolygon(_) => "Polygon",
                        Geometry::<i32>::Point(_)
                        | Geometry::<i32>::Line(_)
                        | Geometry::<i32>::LineString(_)
                        | Geometry::<i32>::Polygon(_)
                        | Geometry::<i32>::GeometryCollection(_)
                        | Geometry::<i32>::Rect(_)
                        | Geometry::<i32>::Triangle(_) => "Part",
                    };
                    (
                        format!("      Part {part}: {n}{}", sub_feature_suffix(geom, *part)),
                        Some(geometry_color(geom)),
                    )
                }
            };

            let style = if idx == app.selected_index {
                STYLE_SELECTED
            } else if app
                .hovered
                .as_ref()
                .is_some_and(|h| h.tree_idx == Some(idx))
            {
                Style::default()
                    .fg(CLR_HOVERED_TREE)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                base.map_or(Style::default(), |c| Style::default().fg(c))
            };
            Line::from(vec![
                Span::raw(if idx == app.selected_index {
                    ">> "
                } else {
                    "   "
                }),
                Span::styled(text, style),
            ])
        })
        .collect();

    let title = match app.mode {
        ViewMode::LayerOverview => {
            let name = app
                .current_file
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            format!("{name} - Enter/+/-:expand, Esc:back, h:help, q:quit")
        }
        ViewMode::FileBrowser | ViewMode::MbtilesMap => "Features".into(),
    };
    let inner_h = area.height.saturating_sub(2).into_usize();
    let inner_w = area.width.saturating_sub(2).into_usize();
    app.tree_inner_height = inner_h;
    let content_w = lines.iter().map(Line::width).max().unwrap_or(0);
    app.tree_scroll = app
        .tree_scroll
        .min(Scroll::max_pos(app.tree_items.len(), inner_h));
    app.tree_hscroll = app.tree_hscroll.min(Scroll::max_pos(content_w, inner_w));
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .scroll((app.tree_scroll, app.tree_hscroll));
    f.render_widget(para, area);
    render_vscrollbar(
        f,
        area,
        Scroll::new(app.tree_items.len(), inner_h, app.tree_scroll),
    );
    render_hscrollbar(f, area, Scroll::new(content_w, inner_w, app.tree_hscroll));
}

fn feature_property_lines(feat: &Feature) -> Vec<Line<'static>> {
    let lines: Vec<Line<'static>> = feat
        .properties
        .iter()
        .filter(|(k, _)| *k != "_layer" && *k != "_extent")
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k}: "), STYLE_LABEL),
                Span::raw(match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null
                    | serde_json::Value::Bool(_)
                    | serde_json::Value::Number(_)
                    | serde_json::Value::Array(_)
                    | serde_json::Value::Object(_) => v.to_string(),
                }),
            ])
        })
        .collect();
    if lines.is_empty() {
        vec![Line::from("(no properties)")]
    } else {
        lines
    }
}

fn value_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Layer count and per-layer feature counts for the whole tile.
fn tile_summary_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        stat_line("Layers", &app.layer_groups.len()),
        stat_line("Features", &app.tile.fc.features.len()),
    ];
    for g in &app.layer_groups {
        lines.push(Line::from(format!(
            "  {}: {} features",
            g.name,
            g.feature_indices.len()
        )));
    }
    lines
}

/// Feature count and property names with their value types for one layer, in name order.
fn layer_summary_lines(app: &App, layer: usize) -> Vec<Line<'static>> {
    let group = &app.layer_groups[layer];
    let mut types: BTreeMap<&str, BTreeSet<&'static str>> = BTreeMap::new();
    for &gi in &group.feature_indices {
        for (k, v) in &app.tile.fc.features[gi].properties {
            if k == "_layer" || k == "_extent" {
                continue;
            }
            types.entry(k).or_default().insert(value_type_name(v));
        }
    }
    let mut lines = vec![
        stat_line("Features", &group.feature_indices.len()),
        stat_line("Properties", &types.len()),
    ];
    for (name, kinds) in types {
        let kinds: Vec<&str> = kinds.into_iter().collect();
        lines.push(Line::from(vec![
            Span::styled(format!("  {name}: "), STYLE_LABEL),
            Span::raw(kinds.join(" | ")),
        ]));
    }
    lines
}

/// Geometry types with counts, over one layer or the whole tile, in type order.
fn geometry_count_lines(app: &App, layer: Option<usize>) -> Vec<Line<'static>> {
    let mut counts: BTreeMap<GeometryType, usize> = BTreeMap::new();
    let mut vertices = 0usize;
    let groups = app
        .layer_groups
        .iter()
        .enumerate()
        .filter(|(i, _)| layer.is_none_or(|l| l == *i));
    for (_, group) in groups {
        for &gi in &group.feature_indices {
            let geom = &app.tile.fc.features[gi].geometry;
            if let Ok(t) = GeometryType::try_from(geom) {
                *counts.entry(t).or_default() += 1;
            }
            vertices += geometry_coord_count(geom);
        }
    }
    let mut lines = vec![stat_line("Vertices", &vertices)];
    for (t, n) in counts {
        lines.push(stat_line(&t.to_string(), &n));
    }
    lines
}

pub fn render_properties_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(app.properties_pct),
            Constraint::Percentage(100u16.saturating_sub(app.properties_pct)),
        ])
        .split(area);

    render_properties_top(f, chunks[0], app);
    render_geometry_stats(f, chunks[1], app);
    chunks[1]
}

/// The hovered item when there is one, else the selection.
fn info_target(app: &App) -> (TreeItem, bool) {
    match app.hovered.as_ref() {
        Some(h) => (h.item.clone(), true),
        None => (app.selected_item().clone(), false),
    }
}

fn render_properties_top(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let (target, hover) = info_target(app);
    if app.last_properties_key.as_ref() != Some(&target) {
        app.properties_scroll = 0;
        app.geometry_scroll = 0;
        app.last_properties_key = Some(target.clone());
    }
    let suffix = if hover { ", hover" } else { "" };
    let (title, lines): (String, Vec<Line<'static>>) = match &target {
        TreeItem::All => ("Properties (all layers)".into(), tile_summary_lines(app)),
        TreeItem::Layer(l) => (
            format!("Properties (layer {}{suffix})", app.layer_groups[*l].name),
            layer_summary_lines(app, *l),
        ),
        TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. } => (
            format!("Properties (feat {feat}{suffix})"),
            feature_property_lines(app.feature(*layer, *feat)),
        ),
    };
    let inner_h = area.height.saturating_sub(2).into_usize();
    let inner_w = area.width.saturating_sub(2).into_usize();
    let rows = wrapped_rows(&lines, inner_w);
    app.properties_scroll = app.properties_scroll.min(Scroll::max_pos(rows, inner_h));
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
    render_vscrollbar(f, area, Scroll::new(rows, inner_h, app.properties_scroll));
}

fn info_point(lines: &mut Vec<Line<'static>>, p: Point<i32>) {
    lines.push(stat_line("Coords", &format!("{:?}", <[i32; 2]>::from(p))));
}

fn info_line_string(lines: &mut Vec<Line<'static>>, ls: &LineString<i32>) {
    lines.push(stat_line("Vertices", &ls.0.len()));
}

fn info_polygon(lines: &mut Vec<Line<'static>>, poly: &Polygon<i32>) {
    lines.push(stat_line("Vertices", &polygon_coord_count(poly)));
    lines.push(stat_line("Rings", &(1 + poly.interiors().len())));
    let ext = &poly.exterior().0;
    let w = if is_ring_ccw(ext) { "CCW" } else { "CW" };
    lines.push(Line::from(format!("  Ring 0: {}v, {w}", ext.len())));
    for (i, ring) in poly.interiors().iter().enumerate() {
        let w = if is_ring_ccw(&ring.0) { "CCW" } else { "CW" };
        lines.push(Line::from(format!(
            "  Ring {}: {}v, {w}",
            i + 1,
            ring.0.len()
        )));
    }
}

fn info_multi_point(lines: &mut Vec<Line<'static>>, pts: &MultiPoint<i32>) {
    lines.push(stat_line("Points", &pts.0.len()));
}

fn info_multi_line_string(lines: &mut Vec<Line<'static>>, mls: &MultiLineString<i32>) {
    let total: usize = mls.iter().map(|ls| ls.0.len()).sum();
    lines.push(stat_line("Parts", &mls.0.len()));
    lines.push(stat_line("Vertices", &total));
}

fn info_multi_polygon(lines: &mut Vec<Line<'static>>, mpoly: &MultiPolygon<i32>) {
    let total: usize = mpoly.iter().map(polygon_coord_count).sum();
    let total_rings: usize = mpoly.iter().map(|p| 1 + p.interiors().len()).sum();
    lines.push(stat_line("Parts", &mpoly.0.len()));
    lines.push(stat_line("Total vertices", &total));
    lines.push(stat_line("Total rings", &total_rings));
}

fn geometry_stats_lines(geom: &Geometry<i32>) -> Vec<Line<'static>> {
    let mut lines = vec![stat_line("Type", &geometry_type_name(geom))];
    match geom {
        Geometry::<i32>::Point(p) => info_point(&mut lines, *p),
        Geometry::<i32>::LineString(ls) => info_line_string(&mut lines, ls),
        Geometry::<i32>::Polygon(poly) => info_polygon(&mut lines, poly),
        Geometry::<i32>::MultiPoint(pts) => info_multi_point(&mut lines, pts),
        Geometry::<i32>::MultiLineString(mls) => info_multi_line_string(&mut lines, mls),
        Geometry::<i32>::MultiPolygon(mpoly) => info_multi_polygon(&mut lines, mpoly),
        Geometry::<i32>::Line(_)
        | Geometry::<i32>::GeometryCollection(_)
        | Geometry::<i32>::Rect(_)
        | Geometry::<i32>::Triangle(_) => {
            unreachable!("Unexpected geometry type {geom:?}")
        }
    }
    lines
}

fn subpart_stats_lines(geom: &Geometry<i32>, part: usize) -> Vec<Line<'static>> {
    let mut lines = vec![stat_line(
        "Component",
        &format!("part #{} of a {}", part, geometry_type_name(geom)),
    )];
    match geom {
        Geometry::<i32>::MultiPoint(pts) => {
            if let Some(p) = pts.0.get(part) {
                lines.push(stat_line("Type", &"Point"));
                info_point(&mut lines, *p);
            }
        }
        Geometry::<i32>::MultiLineString(mls) => {
            if let Some(ls) = mls.0.get(part) {
                lines.push(stat_line("Type", &"LineString"));
                info_line_string(&mut lines, ls);
            }
        }
        Geometry::<i32>::MultiPolygon(mpoly) => {
            if let Some(poly) = mpoly.0.get(part) {
                lines.push(stat_line("Type", &"Polygon"));
                info_polygon(&mut lines, poly);
            }
        }
        Geometry::<i32>::Point(_)
        | Geometry::<i32>::Line(_)
        | Geometry::<i32>::LineString(_)
        | Geometry::<i32>::Polygon(_)
        | Geometry::<i32>::GeometryCollection(_)
        | Geometry::<i32>::Rect(_)
        | Geometry::<i32>::Triangle(_) => {}
    }
    lines
}

/// Triangle count line for a tessellated feature or one of its parts.
fn tessellation_line(
    app: &App,
    layer: usize,
    feat: usize,
    part: Option<usize>,
) -> Option<Line<'static>> {
    let tess = app.tessellation(layer, feat)?;
    let n: usize = match part {
        Some(p) => tess.get(p)?.len(),
        None => tess.iter().map(Vec::len).sum(),
    };
    Some(stat_line("Triangles", &n))
}

// ---------------------------------------------------------------------------
// MBTiles hover properties panel
// ---------------------------------------------------------------------------

/// Renders the left panel for `MbtilesMap` mode: shows hovered feature properties only.
pub fn render_mbtiles_hover_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let (title, lines) = mbt_hover_title_and_lines(app);
    let inner_h = area.height.saturating_sub(2).into_usize();
    let inner_w = area.width.saturating_sub(2).into_usize();
    let rows = wrapped_rows(&lines, inner_w);
    app.properties_scroll = app.properties_scroll.min(Scroll::max_pos(rows, inner_h));
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
    render_vscrollbar(f, area, Scroll::new(rows, inner_h, app.properties_scroll));
}

fn mbt_hover_title_and_lines(app: &App) -> (String, Vec<Line<'static>>) {
    let Some(ref mbt) = app.mbt_state else {
        return ("Properties".into(), vec![Line::from("No mbtiles loaded")]);
    };
    let Some(ref h) = mbt.hovered else {
        return (
            "Properties".into(),
            vec![Line::from("Hover over a feature to inspect properties")],
        );
    };
    let tile_entry = mbt.tiles.get(&h.tile);
    let Some(MbtTileData::Loaded {
        fc, layer_groups, ..
    }) = tile_entry
    else {
        let msg: String = match tile_entry {
            Some(MbtTileData::Empty) => "Tile empty (no vector data)".into(),
            Some(MbtTileData::Error(e)) => {
                let snippet: String = e.chars().take(160).collect();
                format!("Tile error: {snippet}")
            }
            None | Some(MbtTileData::Loading | MbtTileData::Loaded { .. }) => {
                "Tile loading…".into()
            }
        };
        return ("Properties".into(), vec![Line::from(msg)]);
    };
    let Some(group) = layer_groups.get(h.layer_idx) else {
        return ("Properties".into(), vec![Line::from("(feature not found)")]);
    };
    let Some(&gi) = group.feature_indices.get(h.feat_idx) else {
        return ("Properties".into(), vec![Line::from("(feature not found)")]);
    };
    let feat = &fc.features[gi];
    let (z, tx, ty) = h.tile;
    let title = format!(
        "Properties - {} feat {} (tile {z}/{tx}/{ty})",
        group.name, h.feat_idx
    );
    (title, feature_property_lines(feat))
}

fn render_geometry_stats(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let (target, _) = info_target(app);
    let (title, lines) = match target {
        TreeItem::All => (
            "Geometry (all layers)".to_string(),
            geometry_count_lines(app, None),
        ),
        TreeItem::Layer(l) => (
            format!("Geometry (layer {})", app.layer_groups[l].name),
            geometry_count_lines(app, Some(l)),
        ),
        TreeItem::Feature { layer, feat } => {
            let mut lines = geometry_stats_lines(&app.feature(layer, feat).geometry);
            lines.extend(tessellation_line(app, layer, feat, None));
            ("Geometry".to_string(), lines)
        }
        TreeItem::SubFeature { layer, feat, part } => {
            let mut lines = subpart_stats_lines(&app.feature(layer, feat).geometry, part);
            lines.extend(tessellation_line(app, layer, feat, Some(part)));
            ("Geometry".to_string(), lines)
        }
    };

    let inner_h = area.height.saturating_sub(2).into_usize();
    let inner_w = area.width.saturating_sub(2).into_usize();
    let rows = wrapped_rows(&lines, inner_w);
    app.geometry_scroll = app.geometry_scroll.min(Scroll::max_pos(rows, inner_h));
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: false })
        .scroll((app.geometry_scroll, 0));
    f.render_widget(para, area);
    render_vscrollbar(f, area, Scroll::new(rows, inner_h, app.geometry_scroll));
}
