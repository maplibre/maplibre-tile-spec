use mlt_core::geojson;
use mlt_core::geojson::{Feature, Geometry};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Modifier, Span, Style};
use ratatui::widgets::{Paragraph, Wrap};

use crate::ui::state::{App, TreeItem, ViewMode};
use crate::ui::{
    CLR_HOVERED_TREE, STYLE_LABEL, STYLE_SELECTED, block_with_title, feature_suffix,
    geometry_color, geometry_type_name, is_ring_ccw, stat_line, sub_feature_suffix, sub_type_name,
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
                    (
                        format!(
                            "      Part {part}: {}{}",
                            sub_type_name(geom),
                            sub_feature_suffix(geom, *part)
                        ),
                        Some(geometry_color(geom)),
                    )
                }
            };

            let prefix = if idx == app.selected_index {
                ">> "
            } else {
                "   "
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
            Line::from(vec![Span::raw(prefix), Span::styled(text, style)])
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
        ViewMode::FileBrowser => "Features".into(),
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
            let val = match v {
                serde_json::Value::String(s) => s.clone(),
                _ => v.to_string(),
            };
            Line::from(vec![
                Span::styled(format!("{k}: "), STYLE_LABEL),
                Span::raw(val),
            ])
        })
        .collect();
    if lines.is_empty() {
        vec![Line::from("(no properties)")]
    } else {
        lines
    }
}

pub fn render_properties_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_properties_top(f, chunks[0], app);
    render_geometry_stats(f, chunks[1], app);
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
                        "Select a feature or hover over map to view properties",
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

fn geometry_stats_lines(geom: &Geometry) -> Vec<Line<'static>> {
    let mut lines = vec![stat_line("Type", &geometry_type_name(geom))];
    match geom {
        Geometry::Point(c) => {
            lines.push(stat_line("Coords", &format!("[{}, {}]", c[0], c[1])));
        }
        Geometry::MultiPoint(pts) => {
            lines.push(stat_line("Points", &pts.len()));
        }
        Geometry::LineString(c) => {
            lines.push(stat_line("Vertices", &c.len()));
        }
        Geometry::MultiLineString(v) => {
            lines.push(stat_line("Parts", &v.len()));
            let total: usize = v.iter().map(Vec::len).sum();
            lines.push(stat_line("Vertices", &total));
        }
        Geometry::Polygon(rings) => push_ring_stats(&mut lines, rings),
        Geometry::MultiPolygon(polys) => {
            lines.push(stat_line("Parts", &polys.len()));
            let total: usize = polys.iter().flat_map(|p| p.iter()).map(Vec::len).sum();
            lines.push(stat_line("Total vertices", &total));
            let total_rings: usize = polys.iter().map(Vec::len).sum();
            lines.push(stat_line("Total rings", &total_rings));
        }
    }
    lines
}

fn subpart_stats_lines(geom: &Geometry, part: usize) -> Vec<Line<'static>> {
    let mut lines = vec![stat_line(
        "Part",
        &format!("#{part} of {}", geometry_type_name(geom)),
    )];
    match geom {
        Geometry::MultiPoint(pts) => {
            if let Some(&[x, y]) = pts.get(part) {
                lines.push(stat_line("Type", &"Point"));
                lines.push(stat_line("Coords", &format!("[{x}, {y}]")));
            }
        }
        Geometry::MultiLineString(v) => {
            if let Some(line) = v.get(part) {
                lines.push(stat_line("Type", &"LineString"));
                lines.push(stat_line("Vertices", &line.len()));
            }
        }
        Geometry::MultiPolygon(polys) => {
            if let Some(rings) = polys.get(part) {
                lines.push(stat_line("Type", &"Polygon"));
                push_ring_stats(&mut lines, rings);
            }
        }
        _ => {}
    }
    lines
}

fn push_ring_stats(lines: &mut Vec<Line<'static>>, rings: &[Vec<geojson::Coordinate>]) {
    let total: usize = rings.iter().map(Vec::len).sum();
    lines.push(stat_line("Vertices", &total));
    lines.push(stat_line("Rings", &rings.len()));
    for (i, ring) in rings.iter().enumerate() {
        let w = if is_ring_ccw(ring) { "CCW" } else { "CW" };
        lines.push(Line::from(format!("  Ring {i}: {}v, {w}", ring.len())));
    }
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
                vec![Line::from("Select a feature to view geometry details")]
            }
        }
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("Geometry"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
