use geo_types::Polygon;
use mlt_core::geojson::{Coord32, Geom32};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Span, Style};
use ratatui::style::Color;
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};

use crate::ui::state::{App, TreeItem};
use crate::ui::{
    CLR_EXTENT, CLR_HOVERED, CLR_INNER_RING, CLR_INNER_RING_SEL, CLR_POLYGON, CLR_SELECTED,
    block_with_title, coord_f64, geometry_color, is_ring_ccw, part_color,
};

pub fn render_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let sel = app.selected_item();
    let ext = app.extent();
    let (x0, y0, x1, y1) = app.calculate_bounds();

    let canvas = Canvas::default()
        .block(block_with_title("Map View"))
        .x_bounds([x0, x1])
        .y_bounds([y0, y1])
        .paint(|ctx| {
            ctx.draw(&Rectangle {
                x: 0.0,
                y: 0.0,
                width: ext,
                height: ext,
                color: CLR_EXTENT,
            });

            let hov = app.hovered.as_ref();
            let draw_feat = |ctx: &mut Context<'_>, gi: usize| {
                let geom = &app.fc.features[gi].geometry;
                let base = geometry_color(geom);
                let is_hov = hov.is_some_and(|h| app.global_idx(h.layer, h.feat) == gi);
                let sel_part = match sel {
                    TreeItem::SubFeature { layer, feat, part }
                        if app.global_idx(*layer, *feat) == gi =>
                    {
                        Some(*part)
                    }
                    _ => None,
                };
                let hov_part =
                    hov.and_then(|h| (app.global_idx(h.layer, h.feat) == gi).then_some(h.part)?);
                draw_feature(ctx, geom, base, is_hov, sel_part, hov_part);
            };

            match sel {
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
    geom: &Geom32,
    base: Color,
    is_hov: bool,
    sel_part: Option<usize>,
    hov_part: Option<usize>,
) {
    let color = if is_hov { CLR_HOVERED } else { base };
    match geom {
        Geom32::Point(p) => draw_point(ctx, p.0, color),
        Geom32::LineString(ls) => draw_line(ctx, &ls.0, color),
        Geom32::Polygon(poly) => draw_polygon(ctx, poly, is_hov, color),
        Geom32::MultiPoint(pts) => {
            for (i, p) in pts.iter().enumerate() {
                draw_point(ctx, p.0, part_color(sel_part, hov_part, i, color));
            }
        }
        Geom32::MultiLineString(lines) => {
            for (i, ls) in lines.iter().enumerate() {
                draw_line(ctx, &ls.0, part_color(sel_part, hov_part, i, color));
            }
        }
        Geom32::MultiPolygon(polys) => {
            for (i, poly) in polys.iter().enumerate() {
                let pc = part_color(sel_part, hov_part, i, color);
                draw_polygon(ctx, poly, matches!(pc, CLR_HOVERED | CLR_SELECTED), pc);
            }
        }
        _ => {}
    }
}

fn draw_point(ctx: &mut Context<'_>, c: Coord32, color: Color) {
    let [x, y] = coord_f64(c);
    ctx.print(x, y, Span::styled("×", Style::default().fg(color)));
}

fn draw_line(ctx: &mut Context<'_>, coords: &[Coord32], color: Color) {
    for w in coords.windows(2) {
        let [x1, y1] = coord_f64(w[0]);
        let [x2, y2] = coord_f64(w[1]);
        ctx.draw(&CanvasLine::new(x1, y1, x2, y2, color));
    }
}

fn draw_ring(ctx: &mut Context<'_>, ring: &[Coord32], color: Color) {
    draw_line(ctx, ring, color);
    if let (Some(&last), Some(&first)) = (ring.last(), ring.first()) {
        let [lx, ly] = coord_f64(last);
        let [fx, fy] = coord_f64(first);
        ctx.draw(&CanvasLine::new(lx, ly, fx, fy, color));
    }
}

fn ring_color(ring: &[Coord32], highlighted: bool, fallback: Color) -> Color {
    if !highlighted {
        if is_ring_ccw(ring) {
            CLR_POLYGON
        } else {
            CLR_INNER_RING
        }
    } else if is_ring_ccw(ring) {
        fallback
    } else {
        CLR_INNER_RING_SEL
    }
}

fn draw_polygon(ctx: &mut Context<'_>, poly: &Polygon<i32>, highlighted: bool, fallback: Color) {
    let ext = &poly.exterior().0;
    draw_ring(ctx, ext, ring_color(ext, highlighted, fallback));
    for ring in poly.interiors() {
        draw_ring(ctx, &ring.0, ring_color(&ring.0, highlighted, fallback));
    }
}
