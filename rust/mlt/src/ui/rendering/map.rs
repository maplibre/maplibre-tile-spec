use std::collections::HashSet;

use mlt_core::geo_types::{Coord, Geometry, Polygon};
use mlt_core::geojson::FeatureCollection;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Span, Style};
use ratatui::style::Color;
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};

use crate::ui::mbt::{MbtHoveredInfo, MbtTileData, TileTransform};
use crate::ui::state::{App, LayerGroup, TreeItem};
use crate::ui::{
    CLR_CONTEXT_FEATURE, CLR_CONTEXT_LAYER, CLR_DIMMED, CLR_EXTENT, CLR_HOVERED, CLR_INNER_RING,
    CLR_INNER_RING_SEL, CLR_POLYGON, CLR_SELECTED, block_with_title, coord_f64, geometry_color,
    is_ring_ccw,
};

/// How a geometry is colored.
#[derive(Debug, Clone, Copy)]
enum Paint {
    /// Geometry-type colors, polygon rings by winding.
    Natural,
    /// One color, with holes in the highlighted-hole tint.
    Highlight(Color),
    /// One color for everything.
    Flat(Color),
}

/// Draws the selected scope in color, the rest as gray context, and the hovered item in white.
pub fn render_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let sel = app.selected_item();
    let ext = app.extent();
    let (x0, y0, x1, y1) = app.calculate_bounds();
    let hov = app.hovered.as_ref().map(|h| &h.item);
    let fc = &app.tile.fc;

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

            match sel {
                TreeItem::All => {
                    let hov_layer = match hov {
                        Some(TreeItem::Layer(l)) => Some(*l),
                        _ => None,
                    };
                    for (li, group) in app.layer_groups.iter().enumerate() {
                        if Some(li) == hov_layer {
                            continue;
                        }
                        draw_group(ctx, fc, group, Paint::Natural);
                    }
                    if let Some(l) = hov_layer {
                        draw_group(ctx, fc, &app.layer_groups[l], Paint::Highlight(CLR_HOVERED));
                    }
                }
                TreeItem::Layer(l) => {
                    draw_other_layers(ctx, app, *l);
                    let hov_feat = match hov {
                        Some(TreeItem::Feature { layer, feat }) if layer == l => Some(*feat),
                        _ => None,
                    };
                    let group = &app.layer_groups[*l];
                    for (fi, &gi) in group.feature_indices.iter().enumerate() {
                        if Some(fi) != hov_feat {
                            draw_feature(
                                ctx,
                                &fc.features[gi].geometry,
                                Paint::Natural,
                                None,
                                None,
                            );
                        }
                    }
                    if let Some(fi) = hov_feat {
                        let geom = &app.feature(*l, fi).geometry;
                        draw_feature(ctx, geom, Paint::Highlight(CLR_HOVERED), None, None);
                    }
                }
                TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. } => {
                    draw_other_layers(ctx, app, *layer);
                    let group = &app.layer_groups[*layer];
                    for (fi, &gi) in group.feature_indices.iter().enumerate() {
                        if fi != *feat {
                            let paint = Paint::Flat(CLR_CONTEXT_FEATURE);
                            draw_feature(ctx, &fc.features[gi].geometry, paint, None, None);
                        }
                    }
                    let sel_part = match sel {
                        TreeItem::SubFeature { part, .. } => Some(*part),
                        TreeItem::All | TreeItem::Layer(_) | TreeItem::Feature { .. } => None,
                    };
                    let hov_part = match hov {
                        Some(TreeItem::SubFeature {
                            layer: hl,
                            feat: hf,
                            part,
                        }) if hl == layer && hf == feat => Some(*part),
                        _ => None,
                    };
                    let whole_hovered = matches!(
                        hov,
                        Some(TreeItem::Feature { layer: hl, feat: hf }) if hl == layer && hf == feat
                    );
                    let geom = &app.feature(*layer, *feat).geometry;
                    let paint = if whole_hovered {
                        Paint::Highlight(CLR_HOVERED)
                    } else {
                        Paint::Natural
                    };
                    draw_feature(ctx, geom, paint, sel_part, hov_part);
                }
            }
        });

    f.render_widget(canvas, area);
}

fn draw_group(ctx: &mut Context<'_>, fc: &FeatureCollection, group: &LayerGroup, paint: Paint) {
    for &gi in &group.feature_indices {
        draw_feature(ctx, &fc.features[gi].geometry, paint, None, None);
    }
}

/// Every layer except `except`, in the darker context gray.
fn draw_other_layers(ctx: &mut Context<'_>, app: &App, except: usize) {
    for (li, group) in app.layer_groups.iter().enumerate() {
        if li != except {
            draw_group(ctx, &app.tile.fc, group, Paint::Flat(CLR_CONTEXT_LAYER));
        }
    }
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
                draw_feature(ctx, &feat.geometry, Paint::Natural, None, None);
            }
        });

    f.render_widget(canvas, area);
}

fn flat_color(paint: Paint, geom: &Geometry<i32>) -> Color {
    match paint {
        Paint::Natural => geometry_color(geom),
        Paint::Highlight(c) | Paint::Flat(c) => c,
    }
}

/// Paint of part `idx` of a multi-geometry given the selected and hovered parts.
fn part_paint(paint: Paint, sel: Option<usize>, hov: Option<usize>, idx: usize) -> Paint {
    if sel == Some(idx) {
        Paint::Highlight(CLR_SELECTED)
    } else if hov == Some(idx) {
        Paint::Highlight(CLR_HOVERED)
    } else if sel.is_some() || hov.is_some() {
        Paint::Flat(CLR_DIMMED)
    } else {
        paint
    }
}

fn draw_feature(
    ctx: &mut Context<'_>,
    geom: &Geometry<i32>,
    paint: Paint,
    sel_part: Option<usize>,
    hov_part: Option<usize>,
) {
    match geom {
        Geometry::<i32>::Point(p) => draw_point(ctx, p.0, flat_color(paint, geom)),
        Geometry::<i32>::LineString(ls) => draw_line(ctx, &ls.0, flat_color(paint, geom)),
        Geometry::<i32>::Polygon(poly) => draw_polygon(ctx, poly, paint),
        Geometry::<i32>::MultiPoint(pts) => {
            for (i, p) in pts.iter().enumerate() {
                let pc = flat_color(part_paint(paint, sel_part, hov_part, i), geom);
                draw_point(ctx, p.0, pc);
            }
        }
        Geometry::<i32>::MultiLineString(lines) => {
            for (i, ls) in lines.iter().enumerate() {
                let pc = flat_color(part_paint(paint, sel_part, hov_part, i), geom);
                draw_line(ctx, &ls.0, pc);
            }
        }
        Geometry::<i32>::MultiPolygon(polys) => {
            for (i, poly) in polys.iter().enumerate() {
                draw_polygon(ctx, poly, part_paint(paint, sel_part, hov_part, i));
            }
        }
        Geometry::<i32>::Line(_)
        | Geometry::<i32>::GeometryCollection(_)
        | Geometry::<i32>::Rect(_)
        | Geometry::<i32>::Triangle(_) => {}
    }
}

fn draw_point(ctx: &mut Context<'_>, c: Coord<i32>, color: Color) {
    let [x, y] = coord_f64(c);
    ctx.print(x, y, Span::styled("×", Style::default().fg(color)));
}

fn draw_line(ctx: &mut Context<'_>, coords: &[Coord<i32>], color: Color) {
    for w in coords.windows(2) {
        let [x1, y1] = coord_f64(w[0]);
        let [x2, y2] = coord_f64(w[1]);
        ctx.draw(&CanvasLine::new(x1, y1, x2, y2, color));
    }
}

fn draw_ring(ctx: &mut Context<'_>, ring: &[Coord<i32>], color: Color) {
    draw_line(ctx, ring, color);
    if let (Some(&last), Some(&first)) = (ring.last(), ring.first()) {
        let [lx, ly] = coord_f64(last);
        let [fx, fy] = coord_f64(first);
        ctx.draw(&CanvasLine::new(lx, ly, fx, fy, color));
    }
}

fn ring_color(ring: &[Coord<i32>], paint: Paint) -> Color {
    let ccw = is_ring_ccw(ring);
    match paint {
        Paint::Natural if ccw => CLR_POLYGON,
        Paint::Natural => CLR_INNER_RING,
        Paint::Highlight(c) if ccw => c,
        Paint::Highlight(_) => CLR_INNER_RING_SEL,
        Paint::Flat(c) => c,
    }
}

fn draw_polygon(ctx: &mut Context<'_>, poly: &Polygon<i32>, paint: Paint) {
    let ext = &poly.exterior().0;
    draw_ring(ctx, ext, ring_color(ext, paint));
    for ring in poly.interiors() {
        draw_ring(ctx, &ring.0, ring_color(&ring.0, paint));
    }
}

// ---------------------------------------------------------------------------
// MBTiles world-map rendering
// ---------------------------------------------------------------------------

/// Draw all layers from a decoded tile (`data_tile` is the MVT source key used for hover match).
#[allow(clippy::too_many_arguments)] // Canvas draw context + tile payload + viewport Y range.
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
/// World coordinate space: `x ∈ [0, 1]` west->east, `y ∈ [0, 1]` north->south.
///
/// `y_bounds` must be `[min_y, max_y]` with `min_y < max_y` for Ratatui's `Painter` clip math.
/// The painter maps **larger** world Y toward the **top** of the widget, so we reflect each
/// geographic `wy` with [`mbt_screen_y`] to get north-up on screen.
pub fn render_mbtiles_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(ref mbt) = app.mbt_state else {
        return;
    };
    let visible = mbt.visible_tiles();
    let (cz, cx, cy) = mbt.center_tile_xyz();
    let title = format!(
        "World Map - {cz}/{cx}/{cy} - zoom {:.1}  drag=pan  hover=info  q/Esc quit",
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
                    MbtTileData::Loaded {
                        fc,
                        extent,
                        layer_groups,
                        ..
                    } => {
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
#[allow(clippy::too_many_arguments)] // Ratatui canvas line API; args map 1:1 to world corners + style.
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
    geom: &Geometry<i32>,
    t: &TileTransform,
    vp_y0: f64,
    vp_y1: f64,
    color: Color,
) {
    match geom {
        Geometry::<i32>::Point(p) => {
            let [wx, wy] = t.to_world(p.0);
            let sy = mbt_screen_y(vp_y0, vp_y1, wy);
            ctx.print(wx, sy, Span::styled("×", Style::default().fg(color)));
        }
        Geometry::<i32>::LineString(ls) => draw_world_line(ctx, &ls.0, t, vp_y0, vp_y1, color),
        Geometry::<i32>::Polygon(poly) => {
            draw_world_ring(ctx, &poly.exterior().0, t, vp_y0, vp_y1, color);
            for ring in poly.interiors() {
                draw_world_ring(ctx, &ring.0, t, vp_y0, vp_y1, color);
            }
        }
        Geometry::<i32>::MultiPoint(mp) => {
            for p in mp.iter() {
                let [wx, wy] = t.to_world(p.0);
                let sy = mbt_screen_y(vp_y0, vp_y1, wy);
                ctx.print(wx, sy, Span::styled("×", Style::default().fg(color)));
            }
        }
        Geometry::<i32>::MultiLineString(mls) => {
            for ls in mls.iter() {
                draw_world_line(ctx, &ls.0, t, vp_y0, vp_y1, color);
            }
        }
        Geometry::<i32>::MultiPolygon(mpoly) => {
            for poly in mpoly.iter() {
                draw_world_ring(ctx, &poly.exterior().0, t, vp_y0, vp_y1, color);
                for ring in poly.interiors() {
                    draw_world_ring(ctx, &ring.0, t, vp_y0, vp_y1, color);
                }
            }
        }
        Geometry::<i32>::Line(_)
        | Geometry::<i32>::GeometryCollection(_)
        | Geometry::<i32>::Rect(_)
        | Geometry::<i32>::Triangle(_) => {}
    }
}

fn draw_world_line(
    ctx: &mut Context<'_>,
    coords: &[Coord<i32>],
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
    ring: &[Coord<i32>],
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
