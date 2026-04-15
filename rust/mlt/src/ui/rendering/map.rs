use std::collections::HashSet;

use geo_types::Polygon;
use mlt_core::geojson::FeatureCollection;
use mlt_core::{Coord32, Geom32};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Span, Style};
use ratatui::style::Color;
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};

use crate::ui::mbt::{MbtHoveredInfo, MbtTileData, TileTransform};
use crate::ui::state::{App, LayerGroup, TreeItem};
use crate::ui::{
    CLR_DIMMED, CLR_EXTENT, CLR_HOVERED, CLR_INNER_RING, CLR_INNER_RING_SEL, CLR_POLYGON,
    CLR_SELECTED, block_with_title, coord_f64, geometry_color, is_ring_ccw, part_color,
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
                width: f64::from(ext),
                height: f64::from(ext),
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

/// Full-tile preview for file browser (all layers, no r-tree/mouse).
pub fn render_tile_preview(f: &mut Frame<'_>, area: Rect, fc: &FeatureCollection, extent: u32) {
    let canvas = Canvas::default()
        .block(block_with_title("Tile Preview"))
        .x_bounds([0.0, f64::from(extent)])
        .y_bounds([0.0, f64::from(extent)])
        .paint(|ctx| {
            ctx.draw(&Rectangle {
                x: 0.0,
                y: 0.0,
                width: f64::from(extent),
                height: f64::from(extent),
                color: CLR_EXTENT,
            });
            for feat in &fc.features {
                draw_feature(
                    ctx,
                    &feat.geometry,
                    geometry_color(&feat.geometry),
                    false,
                    None,
                    None,
                );
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

// ---------------------------------------------------------------------------
// MBTiles world-map rendering
// ---------------------------------------------------------------------------

/// Draw all layers from a decoded tile (`data_tile` is the MVT source key used for hover match).
fn draw_mbtiles_loaded_tile_layers(
    ctx: &mut Context<'_>,
    hovered: Option<&MbtHoveredInfo>,
    data_tile: (u8, u32, u32),
    fc: &FeatureCollection,
    extent: u32,
    layer_groups: &[LayerGroup],
    vy0: f64,
    vy1: f64,
) {
    let (tz, tx, ty) = data_tile;
    let transform = TileTransform::new(tz, tx, ty, extent);
    let hov_gi = hovered.and_then(|h| {
        if h.tile != (tz, tx, ty) {
            return None;
        }
        layer_groups
            .get(h.layer_idx)
            .and_then(|g| g.feature_indices.get(h.feat_idx))
            .copied()
    });
    for group in layer_groups {
        for &gi in &group.feature_indices {
            let feat = &fc.features[gi];
            let base = geometry_color(&feat.geometry);
            let is_hov = hov_gi == Some(gi);
            let color = if is_hov { CLR_HOVERED } else { base };
            draw_geom_world(ctx, &feat.geometry, &transform, vy0, vy1, color);
        }
    }
}

/// Render the interactive world map for an .mbtiles file.
///
/// World coordinate space: x ∈ [0,1] west→east, y ∈ [0,1] north→south.
///
/// `y_bounds` must be `[min_y, max_y]` with `min_y < max_y` for ratatui's `Painter` clip math.
/// The painter maps **larger** world Y toward the **top** of the widget, so we reflect each
/// geographic `wy` with [`mbt_screen_y`] to get north-up on screen.
pub fn render_mbtiles_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(ref mbt) = app.mbt_state else {
        return;
    };
    let visible = mbt.visible_tiles();
    let (cz, cx, cy) = mbt.center_tile_xyz();
    let title = format!(
        "World Map — {cz}/{cx}/{cy} — zoom {:.1}  drag=pan  hover=info  q/Esc quit",
        mbt.zoom_f
    );

    let canvas = Canvas::default()
        .block(block_with_title(title))
        .x_bounds([mbt.vp_x0, mbt.vp_x1])
        .y_bounds([mbt.vp_y0, mbt.vp_y1])
        .paint(|ctx| {
            let vy0 = mbt.vp_y0;
            let vy1 = mbt.vp_y1;

            // Under native resolution: draw each loaded ancestor at most once (world-aligned).
            let mut overzoom_drawn: HashSet<(u8, u32, u32)> = HashSet::new();
            for &(tz, tx, ty) in &visible {
                if matches!(
                    mbt.tiles.get(&(tz, tx, ty)),
                    Some(MbtTileData::Loaded { .. })
                ) {
                    continue;
                }
                let Some((sz, sx, sy)) = mbt.find_overzoom_source(tz, tx, ty) else {
                    continue;
                };
                if !overzoom_drawn.insert((sz, sx, sy)) {
                    continue;
                }
                let Some(MbtTileData::Loaded {
                    fc,
                    extent,
                    layer_groups,
                    ..
                }) = mbt.tiles.get(&(sz, sx, sy))
                else {
                    continue;
                };
                draw_mbtiles_loaded_tile_layers(
                    ctx,
                    mbt.hovered.as_ref(),
                    (sz, sx, sy),
                    fc,
                    *extent,
                    layer_groups,
                    vy0,
                    vy1,
                );
            }

            for &(tz, tx, ty) in &visible {
                let n = f64::from(1u32 << tz);
                let x0 = f64::from(tx) / n;
                let y0 = f64::from(ty) / n;
                let x1 = f64::from(tx + 1) / n;
                let y1 = f64::from(ty + 1) / n;

                // Draw tile border.
                draw_world_rect_vp(ctx, vy0, vy1, x0, y0, x1, y1, CLR_EXTENT);

                let Some(tile_data) = mbt.tiles.get(&(tz, tx, ty)) else {
                    continue;
                };

                match tile_data {
                    MbtTileData::Loading => {
                        let cx = f64::midpoint(x0, x1);
                        let cy = f64::midpoint(y0, y1);
                        let sy = mbt_screen_y(vy0, vy1, cy);
                        ctx.print(cx, sy, Span::styled("…", Style::default().fg(CLR_DIMMED)));
                    }
                    MbtTileData::Loaded { fc, extent, layer_groups, .. } => {
                        draw_mbtiles_loaded_tile_layers(
                            ctx,
                            mbt.hovered.as_ref(),
                            (tz, tx, ty),
                            fc,
                            *extent,
                            layer_groups,
                            vy0,
                            vy1,
                        );
                    }
                    MbtTileData::Empty | MbtTileData::Error(_) => {}
                }
            }
        });

    f.render_widget(canvas, area);
}

/// Map geographic world Y to canvas Y so north is at the top of the map widget.
#[inline]
fn mbt_screen_y(vp_y0: f64, vp_y1: f64, wy: f64) -> f64 {
    vp_y0 + vp_y1 - wy
}

/// Draw the four edges of a world-coordinate tile rectangle (north-up).
fn draw_world_rect_vp(
    ctx: &mut Context<'_>,
    vp_y0: f64,
    vp_y1: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
) {
    let s0 = mbt_screen_y(vp_y0, vp_y1, y0);
    let s1 = mbt_screen_y(vp_y0, vp_y1, y1);
    ctx.draw(&CanvasLine::new(x0, s0, x1, s0, color));
    ctx.draw(&CanvasLine::new(x0, s1, x1, s1, color));
    ctx.draw(&CanvasLine::new(x0, s0, x0, s1, color));
    ctx.draw(&CanvasLine::new(x1, s0, x1, s1, color));
}

/// Draw a geometry in world coordinates using the provided tile transform (north-up on canvas).
fn draw_geom_world(
    ctx: &mut Context<'_>,
    geom: &Geom32,
    t: &TileTransform,
    vp_y0: f64,
    vp_y1: f64,
    color: Color,
) {
    match geom {
        Geom32::Point(p) => {
            let [wx, wy] = t.to_world(p.0);
            let sy = mbt_screen_y(vp_y0, vp_y1, wy);
            ctx.print(wx, sy, Span::styled("×", Style::default().fg(color)));
        }
        Geom32::LineString(ls) => draw_world_line(ctx, &ls.0, t, vp_y0, vp_y1, color),
        Geom32::Polygon(poly) => {
            draw_world_ring(ctx, &poly.exterior().0, t, vp_y0, vp_y1, color);
            for ring in poly.interiors() {
                draw_world_ring(ctx, &ring.0, t, vp_y0, vp_y1, color);
            }
        }
        Geom32::MultiPoint(mp) => {
            for p in mp.iter() {
                let [wx, wy] = t.to_world(p.0);
                let sy = mbt_screen_y(vp_y0, vp_y1, wy);
                ctx.print(wx, sy, Span::styled("×", Style::default().fg(color)));
            }
        }
        Geom32::MultiLineString(mls) => {
            for ls in mls.iter() {
                draw_world_line(ctx, &ls.0, t, vp_y0, vp_y1, color);
            }
        }
        Geom32::MultiPolygon(mpoly) => {
            for poly in mpoly.iter() {
                draw_world_ring(ctx, &poly.exterior().0, t, vp_y0, vp_y1, color);
                for ring in poly.interiors() {
                    draw_world_ring(ctx, &ring.0, t, vp_y0, vp_y1, color);
                }
            }
        }
        _ => {}
    }
}

fn draw_world_line(
    ctx: &mut Context<'_>,
    coords: &[Coord32],
    t: &TileTransform,
    vp_y0: f64,
    vp_y1: f64,
    color: Color,
) {
    for w in coords.windows(2) {
        let [xa, ya] = t.to_world(w[0]);
        let [xb, yb] = t.to_world(w[1]);
        let sa = mbt_screen_y(vp_y0, vp_y1, ya);
        let sb = mbt_screen_y(vp_y0, vp_y1, yb);
        ctx.draw(&CanvasLine::new(xa, sa, xb, sb, color));
    }
}

fn draw_world_ring(
    ctx: &mut Context<'_>,
    ring: &[Coord32],
    t: &TileTransform,
    vp_y0: f64,
    vp_y1: f64,
    color: Color,
) {
    draw_world_line(ctx, ring, t, vp_y0, vp_y1, color);
    if let (Some(&last), Some(&first)) = (ring.last(), ring.first()) {
        let [lx, ly] = t.to_world(last);
        let [fx, fy] = t.to_world(first);
        ctx.draw(&CanvasLine::new(
            lx,
            mbt_screen_y(vp_y0, vp_y1, ly),
            fx,
            mbt_screen_y(vp_y0, vp_y1, fy),
            color,
        ));
    }
}
