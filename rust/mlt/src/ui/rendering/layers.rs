use mlt_core::geo_types::{
    Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::geojson::Feature;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Modifier, Span, Style};
use ratatui::widgets::{Paragraph, Wrap};

use crate::ui::mbt::MbtTileData;
use crate::ui::state::{App, TreeItem, ViewMode};
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
                        .map(|&gi| geometry_type_name(&app.fc.features[gi].geometry));
                    let all_same = first.is_some_and(|ft| {
                        g.feature_indices
                            .iter()
                            .all(|&gi| geometry_type_name(&app.fc.features[gi].geometry) == ft)
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
                        _ => "Part",
                    };
                    (
                        format!("      Part {part}: {n}{}", sub_feature_suffix(geom, *part)),
                        Some(geometry_color(geom)),
                    )
                }
            };

            let style = if idx == app.selected_index {
                STYLE_SELECTED
            } else if app.hovered.as_ref().is_some_and(|h| h.tree_idx == idx) {
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
    let inner = area.height.saturating_sub(2) as usize;
    app.tree_inner_height = inner;
    let max = u16::try_from(app.tree_items.len().saturating_sub(inner)).unwrap_or(0);
    app.tree_scroll = app.tree_scroll.min(max);
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .scroll((app.tree_scroll, 0));
    f.render_widget(para, area);
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
                    _ => v.to_string(),
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

fn render_properties_top(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let item = app.tree_items.get(app.selected_index);
    let hov = app.hovered.as_ref();
    let (title, lines): (String, Vec<Line<'static>>) = match item {
        None | Some(TreeItem::All | TreeItem::Layer(_)) => {
            if let Some(h) = hov {
                let key = (h.layer, h.feat);
                if app.last_properties_key != Some(key) {
                    app.properties_scroll = 0;
                    app.last_properties_key = Some(key);
                }
                (
                    format!("Properties (feat {}, hover)", h.feat),
                    feature_property_lines(app.feature(h.layer, h.feat)),
                )
            } else {
                app.last_properties_key = None;
                (
                    "Properties".into(),
                    vec![Line::from(
                        "Select or hover over a feature to view properties",
                    )],
                )
            }
        }
        Some(TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. }) => {
            let key = (*layer, *feat);
            if app.last_properties_key != Some(key) {
                app.properties_scroll = 0;
                app.last_properties_key = Some(key);
            }
            (
                format!("Properties (feat {feat})"),
                feature_property_lines(app.feature(*layer, *feat)),
            )
        }
    };
    let inner = area.height.saturating_sub(2);
    let max = u16::try_from(lines.len().saturating_sub(inner as usize)).unwrap_or(0);
    app.properties_scroll = app.properties_scroll.min(max);
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
}

fn info_point(lines: &mut Vec<Line<'static>>, p: Point<i32>) {
    lines.push(stat_line("Coords", &format!("{:?}", <[i32; 2]>::from(p))));
}

fn info_line_string(lines: &mut Vec<Line<'static>>, ls: &LineString<i32>) {
    lines.push(stat_line("Vertices", &ls.0.len()));
}

fn info_polygon(lines: &mut Vec<Line<'static>>, poly: &Polygon<i32>) {
    let total: usize =
        poly.exterior().0.len() + poly.interiors().iter().map(|r| r.0.len()).sum::<usize>();
    lines.push(stat_line("Vertices", &total));
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
    let total: usize = mpoly
        .iter()
        .flat_map(|p| {
            std::iter::once(p.exterior().0.len()).chain(p.interiors().iter().map(|r| r.0.len()))
        })
        .sum();
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
        _ => unreachable!("Unexpected geometry type {geom:?}"),
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
        _ => {}
    }
    lines
}

// ---------------------------------------------------------------------------
// MBTiles hover properties panel
// ---------------------------------------------------------------------------

/// Renders the left panel for `MbtilesMap` mode: shows hovered feature properties only.
pub fn render_mbtiles_hover_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let (title, lines) = mbt_hover_title_and_lines(app);
    let inner = area.height.saturating_sub(2) as usize;
    let max = u16::try_from(lines.len().saturating_sub(inner)).unwrap_or(0);
    app.properties_scroll = app.properties_scroll.min(max);
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
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
        "Properties — {} feat {} (tile {z}/{tx}/{ty})",
        group.name, h.feat_idx
    );
    (title, feature_property_lines(feat))
}

fn render_geometry_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
    let item = app.tree_items.get(app.selected_index);
    let hov = app.hovered.as_ref();

    let lines = match item {
        Some(TreeItem::SubFeature { layer, feat, part }) => {
            subpart_stats_lines(&app.feature(*layer, *feat).geometry, *part)
        }
        Some(TreeItem::Feature { layer, feat }) => {
            geometry_stats_lines(&app.feature(*layer, *feat).geometry)
        }
        _ => {
            if let Some(h) = hov {
                let geom = &app.feature(h.layer, h.feat).geometry;
                match h.part {
                    Some(p) => subpart_stats_lines(geom, p),
                    None => geometry_stats_lines(geom),
                }
            } else {
                vec![Line::from(
                    "Select or hover over a feature to view geometry info",
                )]
            }
        }
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("Geometry"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
