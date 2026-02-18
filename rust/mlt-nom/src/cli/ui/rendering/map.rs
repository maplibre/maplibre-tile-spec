use mlt_nom::geojson::{Coordinate, Geometry};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Span, Style};
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};

use crate::cli::ui::{
    App, TreeItem, block_with_title, coord_f64, geometry_color, is_ring_ccw, part_color,
};

pub fn render_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let selected = app.get_selected_item();
    let extent = app.get_extent();
    let (x_min, y_min, x_max, y_max) = app.calculate_bounds();

    let canvas = Canvas::default()
        .block(block_with_title("Map View"))
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(|ctx| {
            ctx.draw(&Rectangle {
                x: 0.0,
                y: 0.0,
                width: extent,
                height: extent,
                color: Color::DarkGray,
            });

            let hovered = app.hovered.as_ref();
            let draw_feat = |ctx: &mut Context<'_>, gi: usize| {
                let geom = &app.fc.features[gi].geometry;
                let base = geometry_color(geom);
                let is_hovered = hovered.is_some_and(|h| app.global_idx(h.layer, h.feat) == gi);
                let sel_part = match selected {
                    TreeItem::SubFeature { layer, feat, part }
                        if app.global_idx(*layer, *feat) == gi =>
                    {
                        Some(*part)
                    }
                    _ => None,
                };
                let hov_part = hovered
                    .and_then(|h| (app.global_idx(h.layer, h.feat) == gi).then_some(h.part)?);
                draw_feature(ctx, geom, base, is_hovered, sel_part, hov_part);
            };

            match selected {
                TreeItem::All => {
                    for gi in 0..app.fc.features.len() {
                        draw_feat(ctx, gi);
                    }
                }
                TreeItem::Layer(l) => {
                    for &gi in &app.layer_groups[*l].feature_indices {
                        draw_feat(ctx, gi);
                    }
                }
                TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. } => {
                    draw_feat(ctx, app.global_idx(*layer, *feat));
                }
            }
        });

    f.render_widget(canvas, area);
}

fn draw_feature(
    ctx: &mut Context<'_>,
    geom: &Geometry,
    base_color: Color,
    is_hovered: bool,
    selected_part: Option<usize>,
    hovered_part: Option<usize>,
) {
    let color = if is_hovered { Color::White } else { base_color };
    match geom {
        Geometry::Point(c) => draw_point(ctx, *c, color),
        Geometry::LineString(coords) => draw_line(ctx, coords, color),
        Geometry::Polygon(rings) => draw_polygon(ctx, rings, is_hovered, color),
        Geometry::MultiPoint(coords) => {
            for (i, c) in coords.iter().enumerate() {
                draw_point(ctx, *c, part_color(selected_part, hovered_part, i, color));
            }
        }
        Geometry::MultiLineString(lines) => {
            for (i, coords) in lines.iter().enumerate() {
                draw_line(
                    ctx,
                    coords,
                    part_color(selected_part, hovered_part, i, color),
                );
            }
        }
        Geometry::MultiPolygon(polys) => {
            for (i, rings) in polys.iter().enumerate() {
                let pc = part_color(selected_part, hovered_part, i, color);
                draw_polygon(ctx, rings, matches!(pc, Color::White | Color::Yellow), pc);
            }
        }
    }
}

fn draw_point(ctx: &mut Context<'_>, c: Coordinate, color: Color) {
    let [x, y] = coord_f64(c);
    ctx.print(x, y, Span::styled("×", Style::default().fg(color)));
}

fn draw_line(ctx: &mut Context<'_>, coords: &[Coordinate], color: Color) {
    for w in coords.windows(2) {
        let [x1, y1] = coord_f64(w[0]);
        let [x2, y2] = coord_f64(w[1]);
        ctx.draw(&CanvasLine::new(x1, y1, x2, y2, color));
    }
}

fn draw_ring(ctx: &mut Context<'_>, ring: &[Coordinate], color: Color) {
    draw_line(ctx, ring, color);
    if let (Some(&last), Some(&first)) = (ring.last(), ring.first()) {
        let [lx, ly] = coord_f64(last);
        let [fx, fy] = coord_f64(first);
        ctx.draw(&CanvasLine::new(lx, ly, fx, fy, color));
    }
}

fn draw_polygon(
    ctx: &mut Context<'_>,
    rings: &[Vec<Coordinate>],
    highlighted: bool,
    fallback: Color,
) {
    for ring in rings {
        let color = if !highlighted {
            ring_color(ring)
        } else if is_ring_ccw(ring) {
            fallback
        } else {
            Color::Rgb(255, 150, 120)
        };
        draw_ring(ctx, ring, color);
    }
}

fn ring_color(ring: &[Coordinate]) -> Color {
    if is_ring_ccw(ring) {
        Color::Blue
    } else {
        Color::Red
    }
}
