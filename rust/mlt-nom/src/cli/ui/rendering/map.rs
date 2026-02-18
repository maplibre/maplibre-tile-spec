use crate::cli::ui::{App, TreeItem, block_with_title, draw_feature, geometry_color};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Color;
use ratatui::widgets::canvas::{Canvas, Context, Rectangle};

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
