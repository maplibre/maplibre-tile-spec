use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::widgets::{Paragraph, Wrap};
use mlt_nom::geojson::{Feature, Geometry};
use crate::cli::ui::{block_with_title, feature_suffix, geometry_color, geometry_type_name, is_ring_ccw, sub_feature_suffix, sub_type_name, App, TreeItem, ViewMode};

pub fn render_tree_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let lines: Vec<Line<'static>> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let (text, base_color) = match item {
                TreeItem::All => ("All".to_string(), None),
                TreeItem::Layer(li) => {
                    let group = &app.layer_groups[*li];
                    let n = group.feature_indices.len();
                    let first_type = group
                        .feature_indices
                        .first()
                        .map(|&gi| geometry_type_name(&app.fc.features[gi].geometry));
                    let all_same = first_type.is_some_and(|ft| {
                        group
                            .feature_indices
                            .iter()
                            .all(|&gi| geometry_type_name(&app.fc.features[gi].geometry) == ft)
                    });
                    let label = if all_same && n > 0 {
                        format!("{}s", first_type.unwrap())
                    } else {
                        "features".to_string()
                    };
                    (
                        format!(
                            "  Layer: {} ({n} {label}, extent {})",
                            group.name, group.extent
                        ),
                        None,
                    )
                }
                TreeItem::Feature { layer, feat } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    let suffix = feature_suffix(geom);
                    (
                        format!("    Feat {feat}: {}{suffix}", geometry_type_name(geom)),
                        Some(geometry_color(geom)),
                    )
                }
                TreeItem::SubFeature { layer, feat, part } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    let suffix = sub_feature_suffix(geom, *part);
                    (
                        format!("      Part {part}: {}{suffix}", sub_type_name(geom)),
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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if app.hovered.as_ref().is_some_and(|h| h.tree_idx == idx) {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                base_color.map_or(Style::default(), |c| Style::default().fg(c))
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
            format!("{name} - Enter/+/-:expand, Esc:back, q:quit. Drag dividers to resize")
        }
        ViewMode::FileBrowser => "Features".to_string(),
    };
    let inner_height = area.height.saturating_sub(2) as usize;
    app.tree_inner_height = inner_height;
    let max_scroll = u16::try_from(app.tree_items.len().saturating_sub(inner_height)).unwrap_or(0);
    app.tree_scroll = app.tree_scroll.min(max_scroll);
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .scroll((app.tree_scroll, 0));
    f.render_widget(para, area);
}

fn format_property_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}

fn feature_property_lines(feat: &Feature) -> Vec<Line<'static>> {
    let lines: Vec<Line<'static>> = feat
        .properties
        .iter()
        .filter(|(k, _)| *k != "_layer" && *k != "_extent")
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k}: "), Style::default().fg(Color::Cyan)),
                Span::raw(format_property_value(v)),
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
    let hovered = app.hovered.as_ref();
    let (title, lines): (String, Vec<Line<'static>>) = match item {
        None | Some(TreeItem::All | TreeItem::Layer(_)) => {
            if let Some(h) = hovered {
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
                    "Properties".to_string(),
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
    let inner_height = area.height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len().saturating_sub(inner_height as usize)).unwrap_or(0);
    app.properties_scroll = app.properties_scroll.min(max_scroll);
    let para = Paragraph::new(lines)
        .block(block_with_title(title))
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
}

fn geometry_stats_lines(geom: &Geometry) -> Vec<Line<'static>> {
    let cyan = Style::default().fg(Color::Cyan);
    let stat = |name: &str, val: &dyn std::fmt::Display| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{name}: "), cyan),
            Span::raw(val.to_string()),
        ])
    };

    let mut lines = vec![stat("Type", &geometry_type_name(geom))];
    match geom {
        Geometry::Point(c) => {
            lines.push(stat("Coords", &format!("[{}, {}]", c[0], c[1])));
        }
        Geometry::MultiPoint(pts) => {
            lines.push(stat("Points", &pts.len()));
        }
        Geometry::LineString(c) => {
            lines.push(stat("Vertices", &c.len()));
        }
        Geometry::MultiLineString(v) => {
            lines.push(stat("Parts", &v.len()));
            let total: usize = v.iter().map(Vec::len).sum();
            lines.push(stat("Vertices", &total));
        }
        Geometry::Polygon(rings) => {
            let total: usize = rings.iter().map(Vec::len).sum();
            lines.push(stat("Vertices", &total));
            lines.push(stat("Rings", &rings.len()));
            for (i, ring) in rings.iter().enumerate() {
                let w = if is_ring_ccw(ring) { "CCW" } else { "CW" };
                lines.push(Line::from(format!("  Ring {i}: {}v, {w}", ring.len())));
            }
        }
        Geometry::MultiPolygon(polys) => {
            lines.push(stat("Parts", &polys.len()));
            let total: usize = polys.iter().flat_map(|p| p.iter()).map(Vec::len).sum();
            lines.push(stat("Total vertices", &total));
            let total_rings: usize = polys.iter().map(Vec::len).sum();
            lines.push(stat("Total rings", &total_rings));
        }
    }

    lines
}

fn subpart_stats_lines(geom: &Geometry, part: usize) -> Vec<Line<'static>> {
    let cyan = Style::default().fg(Color::Cyan);
    let stat = |name: &str, val: &dyn std::fmt::Display| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{name}: "), cyan),
            Span::raw(val.to_string()),
        ])
    };

    let mut lines = vec![stat(
        "Part",
        &format!("#{part} of {}", geometry_type_name(geom)),
    )];
    match geom {
        Geometry::MultiPoint(pts) => {
            if let Some(&[x, y]) = pts.get(part) {
                lines.push(stat("Type", &"Point"));
                lines.push(stat("Coords", &format!("[{x}, {y}]")));
            }
        }
        Geometry::MultiLineString(v) => {
            if let Some(line) = v.get(part) {
                lines.push(stat("Type", &"LineString"));
                lines.push(stat("Vertices", &line.len()));
            }
        }
        Geometry::MultiPolygon(polys) => {
            if let Some(rings) = polys.get(part) {
                lines.push(stat("Type", &"Polygon"));
                let total: usize = rings.iter().map(Vec::len).sum();
                lines.push(stat("Vertices", &total));
                lines.push(stat("Rings", &rings.len()));
                for (i, ring) in rings.iter().enumerate() {
                    let w = if is_ring_ccw(ring) { "CCW" } else { "CW" };
                    lines.push(Line::from(format!("  Ring {i}: {}v, {w}", ring.len())));
                }
            }
        }
        _ => {}
    }
    lines
}

fn render_geometry_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
    let item = app.tree_items.get(app.selected_index);
    let hovered = app.hovered.as_ref();

    let lines = match item {
        Some(TreeItem::SubFeature { layer, feat, part }) => {
            let geom = &app.feature(*layer, *feat).geometry;
            subpart_stats_lines(geom, *part)
        }
        Some(TreeItem::Feature { layer, feat }) => {
            geometry_stats_lines(&app.feature(*layer, *feat).geometry)
        }
        _ => {
            if let Some(h) = hovered {
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