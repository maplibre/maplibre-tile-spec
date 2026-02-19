//! TUI visualizer for MLT files using ratatui

mod rendering;
mod state;

use std::collections::HashSet;
use std::fs::canonicalize;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;
use std::{fs, thread};

use anyhow::bail;
use clap::Args;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use crossterm::execute;
use mlt_core::geojson::{Coordinate, FeatureCollection, Geometry};
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::parse_layers;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use rstar::{AABB, PointDistance, RTreeObject};

use crate::ls::{
    FileSortColumn, LsRow, MltFileInfo, analyze_tile_files, is_mvt_extension, is_tile_extension,
};
use crate::ui::rendering::files::{
    render_file_browser, render_file_filter_panel, render_file_info_panel,
};
use crate::ui::rendering::help::{render_error_popup, render_help_overlay};
use crate::ui::rendering::layers::{render_properties_panel, render_tree_panel};
use crate::ui::rendering::map::render_map_panel;
use crate::ui::state::{App, HoveredInfo, LayerGroup, ResizeHandle, TreeItem, ViewMode};

pub const CLR_POINT: Color = Color::Magenta;
pub const CLR_MULTI_POINT: Color = Color::LightMagenta;
pub const CLR_LINE: Color = Color::Cyan;
pub const CLR_MULTI_LINE: Color = Color::LightCyan;
pub const CLR_POLYGON: Color = Color::Blue;
pub const CLR_MULTI_POLYGON: Color = Color::LightBlue;
pub const CLR_INNER_RING: Color = Color::Red;
pub const CLR_BAD_WINDING: Color = Color::LightRed;
pub const CLR_EXTENT: Color = Color::DarkGray;
pub const CLR_SELECTED: Color = Color::Yellow;
pub const CLR_HOVERED: Color = Color::White;
pub const CLR_HOVERED_TREE: Color = Color::LightGreen;
pub const CLR_DIMMED: Color = Color::DarkGray;
pub const CLR_INNER_RING_SEL: Color = Color::Rgb(255, 150, 120);
pub const CLR_LABEL: Color = Color::Cyan;
pub const CLR_HINT: Color = Color::DarkGray;

pub const STYLE_SELECTED: Style = Style::new().fg(CLR_SELECTED).add_modifier(Modifier::BOLD);
pub const STYLE_LABEL: Style = Style::new().fg(CLR_LABEL);
pub const STYLE_BOLD: Style = Style::new().add_modifier(Modifier::BOLD);

#[derive(Args)]
pub struct UiArgs {
    /// Path to a tile file (.mlt, .mvt, .pbf) or directory
    path: PathBuf,
}

pub fn ui(args: &UiArgs) -> anyhow::Result<()> {
    let app = if args.path.is_dir() {
        let paths = find_tile_files(&args.path)?;
        if paths.is_empty() {
            bail!(
                "No tile files found in {}",
                canonicalize(&args.path)?.display()
            );
        }
        let base = args.path.clone();
        let files: Vec<_> = paths
            .iter()
            .map(|p| {
                (
                    p.clone(),
                    LsRow::Loading {
                        path: crate::ls::relative_path(p, &base),
                    },
                )
            })
            .collect();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(analyze_tile_files(&paths, &base, true));
        });
        App::new_file_browser(files, Some(rx))
    } else if args.path.is_file() {
        App::new_single_file(load_fc(&args.path)?, Some(args.path.clone()))
    } else {
        bail!("Path is not a file or directory");
    };
    run_app(app)
}

// --- Data loading ---

fn load_fc(path: &Path) -> anyhow::Result<FeatureCollection> {
    let buf = fs::read(path)?;
    if is_mvt_extension(path) {
        Ok(mvt_to_feature_collection(buf)?)
    } else {
        let mut layers = parse_layers(&buf)?;
        for layer in &mut layers {
            layer.decode_all()?;
        }
        Ok(FeatureCollection::from_layers(&layers)?)
    }
}

fn group_by_layer(fc: &FeatureCollection) -> Vec<LayerGroup> {
    let mut groups: Vec<LayerGroup> = Vec::new();
    for (i, f) in fc.features.iter().enumerate() {
        let name = f
            .properties
            .get("_layer")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let extent = f
            .properties
            .get("_extent")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(4096.0);
        if let Some(g) = groups.iter_mut().find(|g| g.name == name) {
            g.feature_indices.push(i);
        } else {
            groups.push(LayerGroup::new(name.to_string(), extent, vec![i]));
        }
    }
    groups
}

fn auto_expand(groups: &[LayerGroup]) -> Vec<bool> {
    if groups.len() == 1 {
        vec![true]
    } else {
        vec![false; groups.len()]
    }
}

fn find_tile_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let p = entry?.path();
                if p.is_dir() {
                    visit(&p, out)?;
                } else if is_tile_extension(&p) {
                    out.push(p);
                }
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(dir, &mut files)?;
    files.sort();
    Ok(files)
}

// --- Hit testing ---

fn point_in_rect(col: u16, row: u16, r: Rect) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

fn click_row_in_area(col: u16, row: u16, area: Rect, scroll: usize) -> Option<usize> {
    let top = area.y + 1;
    let bot = area.y + area.height.saturating_sub(1);
    (col >= area.x && col < area.x + area.width && row >= top && row < bot)
        .then(|| (row - top) as usize + scroll)
}

const HIGHLIGHT_SYMBOL_WIDTH: u16 = 3;
const COLUMN_SPACING: u16 = 1;

fn file_header_click_column(
    area: Rect,
    widths: &[Constraint; 5],
    col: u16,
    row: u16,
) -> Option<FileSortColumn> {
    if row != area.y + 1 {
        return None;
    }
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let resolved: Vec<u16> = widths
        .iter()
        .map(|c| match c {
            Constraint::Length(l) | Constraint::Min(l) => *l,
            _ => 0,
        })
        .collect();
    let mut x = inner.x + HIGHLIGHT_SYMBOL_WIDTH;
    let cols = [
        FileSortColumn::File,
        FileSortColumn::Size,
        FileSortColumn::EncPct,
        FileSortColumn::Layers,
        FileSortColumn::Features,
    ];
    for (i, &w) in resolved.iter().enumerate() {
        let end = if i == resolved.len() - 1 {
            inner.x + inner.width
        } else {
            x + w
        };
        if col >= x && col < end {
            return Some(cols[i]);
        }
        x = end + COLUMN_SPACING;
    }
    None
}

fn block_with_title(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title)
}

/// Helper to build `Span::styled(format!("{name}: "), STYLE_LABEL)` + raw value.
fn stat_line(name: &str, val: &dyn std::fmt::Display) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{name}: "), STYLE_LABEL),
        Span::raw(val.to_string()),
    ])
}

const DIVIDER_GRAB: u16 = 2;

fn divider_hit(col: u16, row: u16, left: Rect, tree: Rect) -> Option<ResizeHandle> {
    let dx = left.x + left.width;
    if col >= dx.saturating_sub(DIVIDER_GRAB)
        && col < dx.saturating_add(DIVIDER_GRAB)
        && row >= left.y
        && row < left.y + left.height
    {
        return Some(ResizeHandle::LeftRight);
    }
    let dy = tree.y + tree.height;
    if row >= dy.saturating_sub(DIVIDER_GRAB)
        && row < dy.saturating_add(DIVIDER_GRAB)
        && col >= left.x
        && col < left.x + left.width
    {
        return Some(ResizeHandle::FeaturesProperties);
    }
    None
}

// --- App loop ---

/// OSC 22: set mouse pointer shape
fn set_pointer_cursor(pointer: bool) {
    let seq: &[u8] = if pointer {
        b"\x1b]22;default\x1b\\"
    } else {
        b"\x1b]22;\x1b\\"
    };
    let _ = std::io::stdout().write_all(seq);
    let _ = std::io::stdout().flush();
}

fn run_app(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    set_pointer_cursor(true);
    let result = (|| {
        execute!(terminal.backend_mut(), EnableMouseCapture)?;
        run_app_loop(&mut terminal, &mut app)
    })();
    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
    set_pointer_cursor(false);
    ratatui::restore();
    result
}

/// Compute percentage position (clamped to 10..=90) for drag resizing.
fn pct_at(pos: u16, origin: u16, span: u16) -> u16 {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pct =
        (f32::from(pos.saturating_sub(origin)) / f32::from(span.max(1)) * 100.0).round() as u16;
    pct.clamp(10, 90)
}

fn run_app_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    let mut map_area: Option<Rect> = None;
    let mut tree_area: Option<Rect> = None;
    let mut props_area: Option<Rect> = None;
    let mut left_area: Option<Rect> = None;
    let mut filter_area: Option<Rect> = None;
    let mut info_area: Option<Rect> = None;
    let mut file_left: Option<Rect> = None;
    let mut last_tree_click: Option<(Instant, usize)> = None;
    let mut last_file_click: Option<(Instant, usize)> = None;

    loop {
        if let Some(rows) = app.analysis_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            if rows.len() == app.mlt_files.len() {
                for (i, row) in rows.into_iter().enumerate() {
                    if let Some(e) = app.mlt_files.get_mut(i) {
                        e.1 = row;
                    }
                }
            }
            app.analysis_rx = None;
            app.rebuild_filtered_files();
        }

        if app.needs_redraw {
            app.needs_redraw = false;
            terminal.draw(|f| {
                match app.mode {
                    ViewMode::FileBrowser => {
                        let cols = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Percentage(app.file_left_pct),
                                Constraint::Percentage(100u16.saturating_sub(app.file_left_pct)),
                            ])
                            .split(f.area());
                        let right = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(cols[1]);
                        render_file_browser(f, cols[0], app);
                        render_file_filter_panel(f, right[0], app);
                        render_file_info_panel(f, right[1], app);
                        file_left = Some(cols[0]);
                        filter_area = Some(right[0]);
                        info_area = Some(right[1]);
                    }
                    ViewMode::LayerOverview => {
                        let cols = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Percentage(app.left_pct),
                                Constraint::Percentage(100u16.saturating_sub(app.left_pct)),
                            ])
                            .split(f.area());
                        let left = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Percentage(app.features_pct),
                                Constraint::Percentage(100u16.saturating_sub(app.features_pct)),
                            ])
                            .split(cols[0]);
                        render_tree_panel(f, left[0], app);
                        render_properties_panel(f, left[1], app);
                        render_map_panel(f, cols[1], app);
                        tree_area = Some(left[0]);
                        props_area = Some(left[1]);
                        left_area = Some(cols[0]);
                        map_area = Some(cols[1]);
                    }
                }
                if app.error_popup.is_some() {
                    render_error_popup(f, app);
                } else if app.show_help {
                    render_help_overlay(f, app);
                }
            })?;
        }

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    if app.error_popup.is_some() {
                        app.error_popup = None;
                        app.invalidate();
                        continue;
                    }
                    if app.show_help {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.help_scroll = app.help_scroll.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.help_scroll = app.help_scroll.saturating_add(1);
                            }
                            KeyCode::PageUp => {
                                app.help_scroll = app.help_scroll.saturating_sub(10);
                            }
                            KeyCode::PageDown => {
                                app.help_scroll = app.help_scroll.saturating_add(10);
                            }
                            KeyCode::Home => app.help_scroll = 0,
                            KeyCode::End => app.help_scroll = u16::MAX,
                            _ => app.show_help = false,
                        }
                        app.invalidate();
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Esc if app.handle_escape() => break,
                        KeyCode::Char('?') | KeyCode::F(1) => app.open_help(),
                        KeyCode::Char('h') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.open_help();
                        }
                        KeyCode::Enter => app.handle_enter(),
                        KeyCode::Char('+' | '=') | KeyCode::Right => app.handle_plus(),
                        KeyCode::Char('-') => app.handle_minus(),
                        KeyCode::Char('*') => app.handle_star(),
                        KeyCode::Up | KeyCode::Char('k') => app.move_up_by(1),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down_by(1),
                        KeyCode::Left => app.handle_left_arrow(),
                        KeyCode::PageUp => {
                            app.move_up_by(app.page_size().saturating_sub(1).max(1));
                        }
                        KeyCode::PageDown => {
                            app.move_down_by(app.page_size().saturating_sub(1).max(1));
                        }
                        KeyCode::Home => app.move_to_start(),
                        KeyCode::End => app.move_to_end(),
                        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.left_pct = app.left_pct.saturating_sub(5).max(10);
                            app.invalidate();
                        }
                        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.left_pct = (app.left_pct + 5).min(90);
                            app.invalidate();
                        }
                        KeyCode::Char('J') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.features_pct = app.features_pct.saturating_sub(5).max(10);
                            app.invalidate();
                        }
                        KeyCode::Char('K') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.features_pct = (app.features_pct + 5).min(90);
                            app.invalidate();
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) if app.error_popup.is_some() => {
                    if matches!(mouse.kind, MouseEventKind::Down(_)) {
                        app.error_popup = None;
                        app.invalidate();
                    }
                }
                Event::Mouse(mouse) if app.show_help => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.help_scroll = app.help_scroll.saturating_sub(1);
                        app.invalidate();
                    }
                    MouseEventKind::ScrollDown => {
                        app.help_scroll = app.help_scroll.saturating_add(1);
                        app.invalidate();
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Up(_) => {
                        if app.resizing.take().is_some() {
                            app.invalidate();
                        }
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        if let Some(handle) = app.resizing {
                            let area = terminal.get_frame().area();
                            let la = left_area.unwrap_or_default();
                            match handle {
                                ResizeHandle::LeftRight => {
                                    app.left_pct = pct_at(mouse.column, area.x, area.width);
                                }
                                ResizeHandle::FeaturesProperties => {
                                    app.features_pct = pct_at(mouse.row, la.y, la.height);
                                }
                                ResizeHandle::FileBrowserLeftRight => {
                                    app.file_left_pct = pct_at(mouse.column, area.x, area.width);
                                }
                            }
                            app.invalidate();
                            continue;
                        }
                        let prev = app.hovered.clone();
                        app.hovered = None;

                        if app.mode == ViewMode::LayerOverview {
                            let hover_ok = !matches!(
                                app.tree_items.get(app.selected_index),
                                Some(TreeItem::Feature { layer, feat })
                                    if !app.expanded_features.contains(&(*layer, *feat))
                            );
                            if hover_ok {
                                if let Some(area) = tree_area {
                                    if let Some(row) = click_row_in_area(
                                        mouse.column,
                                        mouse.row,
                                        area,
                                        app.tree_scroll as usize,
                                    ) {
                                        if let Some((l, f, p)) = app
                                            .tree_items
                                            .get(row)
                                            .and_then(TreeItem::layer_feat_part)
                                        {
                                            app.hovered = Some(HoveredInfo::new(row, l, f, p));
                                        }
                                    }
                                }
                            }
                            if app.hovered.is_none() {
                                if let Some(area) = map_area {
                                    if point_in_rect(mouse.column, mouse.row, area) {
                                        let b = app.get_bounds();
                                        let rx = f64::from(mouse.column - area.x)
                                            / f64::from(area.width);
                                        let ry =
                                            f64::from(mouse.row - area.y) / f64::from(area.height);
                                        let cx = b.0 + rx * (b.2 - b.0);
                                        let cy = b.3 - ry * (b.3 - b.1);
                                        app.find_hovered_feature(cx, cy, b);
                                    }
                                }
                            }
                        }
                        if app.hovered != prev {
                            app.invalidate();
                        }
                    }
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
                        let s = app.scroll_step();
                        let step = u16::try_from(s)?;
                        if app.mode == ViewMode::FileBrowser {
                            if filter_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                app.filter_scroll = scroll_by(app.filter_scroll, step, up);
                                app.invalidate();
                                continue;
                            }
                            if info_area.is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                continue;
                            }
                        }
                        if app.mode == ViewMode::LayerOverview {
                            if props_area.is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                app.properties_scroll = scroll_by(app.properties_scroll, step, up);
                                app.invalidate();
                                continue;
                            }
                            if let Some(area) = tree_area {
                                if point_in_rect(mouse.column, mouse.row, area) {
                                    if up {
                                        app.tree_scroll = app.tree_scroll.saturating_sub(step);
                                    } else {
                                        let inner = area.height.saturating_sub(2) as usize;
                                        let max = u16::try_from(
                                            app.tree_items.len().saturating_sub(inner),
                                        )?;
                                        app.tree_scroll =
                                            app.tree_scroll.saturating_add(step).min(max);
                                    }
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if map_area.is_some_and(|a| point_in_rect(mouse.column, mouse.row, a)) {
                                continue;
                            }
                        }
                        if up {
                            app.move_up_by(s);
                        } else {
                            app.move_down_by(s);
                        }
                    }
                    MouseEventKind::Down(_) => {
                        if app.mode == ViewMode::FileBrowser {
                            if let Some(fl) = file_left {
                                let dx = fl.x + fl.width;
                                if mouse.column >= dx.saturating_sub(DIVIDER_GRAB)
                                    && mouse.column < dx.saturating_add(DIVIDER_GRAB)
                                    && mouse.row >= fl.y
                                    && mouse.row < fl.y + fl.height
                                {
                                    app.resizing = Some(ResizeHandle::FileBrowserLeftRight);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(fa) = filter_area {
                                if point_in_rect(mouse.column, mouse.row, fa) {
                                    let row = (mouse.row.saturating_sub(fa.y + 1)) as usize
                                        + app.filter_scroll as usize;
                                    handle_filter_click(app, row);
                                    continue;
                                }
                            }
                            if let Some(ia) = info_area {
                                if point_in_rect(mouse.column, mouse.row, ia)
                                    && app.filtered_file_indices.is_empty()
                                    && !app.mlt_files.is_empty()
                                {
                                    let row = (mouse.row.saturating_sub(ia.y + 1)) as usize;
                                    if row == 2 {
                                        app.ext_filters.clear();
                                        app.geom_filters.clear();
                                        app.algo_filters.clear();
                                        app.rebuild_filtered_files();
                                    }
                                    continue;
                                }
                            }
                            if let Some(area) = app.file_table_area {
                                if app.data_loaded() {
                                    if let Some(widths) = app.file_table_widths {
                                        if let Some(c) = file_header_click_column(
                                            area,
                                            &widths,
                                            mouse.column,
                                            mouse.row,
                                        ) {
                                            app.handle_file_header_click(c);
                                            continue;
                                        }
                                    }
                                }
                                if let Some(row) = click_row_in_area(
                                    mouse.column,
                                    mouse.row,
                                    area,
                                    app.file_list_state.offset(),
                                ) {
                                    if row > 0 && row <= app.filtered_file_indices.len() {
                                        let r = row - 1;
                                        let dbl = last_file_click.is_some_and(|(t, prev)| {
                                            prev == r && t.elapsed().as_millis() < 400
                                        });
                                        last_file_click = Some((Instant::now(), r));
                                        app.selected_file_index = r;
                                        app.file_list_state.select(Some(r));
                                        app.invalidate_bounds();
                                        if dbl {
                                            app.handle_enter();
                                        }
                                    }
                                }
                            }
                        } else if app.mode == ViewMode::LayerOverview {
                            if let (Some(la), Some(ta)) = (left_area, tree_area) {
                                if let Some(h) = divider_hit(mouse.column, mouse.row, la, ta) {
                                    app.resizing = Some(h);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(area) = tree_area {
                                if let Some(row) = click_row_in_area(
                                    mouse.column,
                                    mouse.row,
                                    area,
                                    app.tree_scroll as usize,
                                ) {
                                    if row < app.tree_items.len() {
                                        let dbl = last_tree_click.is_some_and(|(t, prev)| {
                                            prev == row && t.elapsed().as_millis() < 400
                                        });
                                        last_tree_click = Some((Instant::now(), row));
                                        if let Some((l, f, p)) = app
                                            .tree_items
                                            .get(row)
                                            .and_then(TreeItem::layer_feat_part)
                                        {
                                            app.handle_feature_click(l, f, p, area.height);
                                        } else {
                                            app.selected_index = row;
                                            app.scroll_selected_into_view(
                                                area.height.saturating_sub(2) as usize,
                                            );
                                        }
                                        app.invalidate_bounds();
                                        if dbl {
                                            app.handle_enter();
                                        }
                                    }
                                }
                            }
                            if let Some(ref h) = app.hovered {
                                if let Some(ta) = tree_area {
                                    if map_area
                                        .is_some_and(|m| point_in_rect(mouse.column, mouse.row, m))
                                    {
                                        app.handle_feature_click(
                                            h.layer, h.feat, h.part, ta.height,
                                        );
                                        app.invalidate_bounds();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => app.invalidate(),
                _ => {}
            }
        }
    }
    Ok(())
}

/// Apply scroll delta: subtract if up, add if down.
fn scroll_by(val: u16, step: u16, up: bool) -> u16 {
    if up {
        val.saturating_sub(step)
    } else {
        val.saturating_add(step)
    }
}

// --- Filtering ---

fn handle_filter_click(app: &mut App, row: usize) {
    let exts = collect_extensions(&app.mlt_files);
    let geoms = collect_file_values(&app.mlt_files, MltFileInfo::geometries);
    let algos = collect_file_values(&app.mlt_files, MltFileInfo::algorithms);

    let ext_start = 3;
    let ext_end = ext_start + exts.len();
    let geom_start = ext_end + 2;
    let geom_end = geom_start + geoms.len();
    let algo_start = geom_end + 2;
    let algo_end = algo_start + algos.len();

    if row == 0 {
        app.ext_filters.clear();
        app.geom_filters.clear();
        app.algo_filters.clear();
    } else if row >= ext_start && row < ext_end {
        toggle_set(&mut app.ext_filters, &exts[row - ext_start]);
    } else if row >= geom_start && row < geom_end {
        toggle_set(&mut app.geom_filters, &geoms[row - geom_start]);
    } else if row >= algo_start && row < algo_end {
        toggle_set(&mut app.algo_filters, &algos[row - algo_start]);
    }
    app.rebuild_filtered_files();
}

fn toggle_set(set: &mut HashSet<String>, val: &str) {
    if !set.remove(val) {
        set.insert(val.to_string());
    }
}

fn geom_abbrev_to_full(abbrev: &str) -> &str {
    match abbrev {
        "Pt" => "Point",
        "Line" => "LineString",
        "Poly" => "Polygon",
        "MPt" => "MultiPoint",
        "MLine" => "MultiLineString",
        "MPoly" => "MultiPolygon",
        other => other,
    }
}

fn collect_file_values(files: &[(PathBuf, LsRow)], field: fn(&MltFileInfo) -> &str) -> Vec<String> {
    let mut set = HashSet::new();
    for (_, row) in files {
        if let LsRow::Info(info) = row {
            for v in field(info)
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                set.insert(v.to_string());
            }
        }
    }
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort();
    v
}

fn collect_extensions(files: &[(PathBuf, LsRow)]) -> Vec<String> {
    let mut set = HashSet::new();
    for (path, _) in files {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            set.insert(ext.to_lowercase());
        }
    }
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort();
    v
}

// --- Geometry helpers ---

fn geometry_type_name(geom: &Geometry) -> &'static str {
    match geom {
        Geometry::Point(_) => "Point",
        Geometry::LineString(_) => "LineString",
        Geometry::Polygon(_) => "Polygon",
        Geometry::MultiPoint(_) => "MultiPoint",
        Geometry::MultiLineString(_) => "MultiLineString",
        Geometry::MultiPolygon(_) => "MultiPolygon",
    }
}

fn sub_type_name(geom: &Geometry) -> &'static str {
    match geom {
        Geometry::MultiPoint(_) => "Point",
        Geometry::MultiLineString(_) => "LineString",
        Geometry::MultiPolygon(_) => "Polygon",
        _ => "Part",
    }
}

fn geometry_color(geom: &Geometry) -> Color {
    match geom {
        Geometry::Point(_) => CLR_POINT,
        Geometry::MultiPoint(_) => CLR_MULTI_POINT,
        Geometry::LineString(_) => CLR_LINE,
        Geometry::MultiLineString(_) => CLR_MULTI_LINE,
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) if has_nonstandard_winding(geom) => {
            CLR_BAD_WINDING
        }
        Geometry::Polygon(_) => CLR_POLYGON,
        Geometry::MultiPolygon(_) => CLR_MULTI_POLYGON,
    }
}

fn multi_part_count(geom: &Geometry) -> usize {
    match geom {
        Geometry::MultiPoint(v) => v.len(),
        Geometry::MultiLineString(v) => v.len(),
        Geometry::MultiPolygon(v) => v.len(),
        _ => 0,
    }
}

fn feature_suffix(geom: &Geometry) -> String {
    let n = multi_part_count(geom);
    if n > 0 {
        return format!(" ({n} parts)");
    }
    match geom {
        Geometry::LineString(c) => format!(" ({}v)", c.len()),
        Geometry::Polygon(rings) => {
            let total: usize = rings.iter().map(Vec::len).sum();
            if rings.len() > 1 {
                format!(" ({total}v, {} rings)", rings.len())
            } else {
                format!(" ({total}v)")
            }
        }
        _ => String::new(),
    }
}

fn sub_feature_suffix(geom: &Geometry, part: usize) -> String {
    match geom {
        Geometry::MultiLineString(v) => v
            .get(part)
            .map_or(String::new(), |l| format!(" ({}v)", l.len())),
        Geometry::MultiPolygon(v) => v.get(part).map_or(String::new(), |p| {
            let total: usize = p.iter().map(Vec::len).sum();
            if p.len() > 1 {
                format!(" ({total}v, {} rings)", p.len())
            } else {
                format!(" ({total}v)")
            }
        }),
        _ => String::new(),
    }
}

fn is_entry_visible(layer: usize, feat: usize, sel: &TreeItem) -> bool {
    match sel {
        TreeItem::All => true,
        TreeItem::Layer(l) => *l == layer,
        TreeItem::Feature { layer: l, feat: f }
        | TreeItem::SubFeature {
            layer: l, feat: f, ..
        } => *l == layer && *f == feat,
    }
}

fn part_color(sel: Option<usize>, hov: Option<usize>, idx: usize, base: Color) -> Color {
    if sel == Some(idx) {
        CLR_SELECTED
    } else if hov == Some(idx) {
        CLR_HOVERED
    } else if sel.is_some() || hov.is_some() {
        CLR_DIMMED
    } else {
        base
    }
}

// --- Winding ---

fn ring_signed_area(ring: &[Coordinate]) -> f64 {
    let mut area = 0.0;
    for w in ring.windows(2) {
        let [x1, y1] = coord_f64(w[0]);
        let [x2, y2] = coord_f64(w[1]);
        area += (x2 - x1) * (y2 + y1);
    }
    if let (Some(&last), Some(&first)) = (ring.last(), ring.first()) {
        let [lx, ly] = coord_f64(last);
        let [fx, fy] = coord_f64(first);
        area += (fx - lx) * (fy + ly);
    }
    area
}

fn is_ring_ccw(ring: &[Coordinate]) -> bool {
    ring_signed_area(ring) < 0.0
}

fn has_nonstandard_winding(geom: &Geometry) -> bool {
    let check = |rings: &[Vec<Coordinate>]| {
        rings.first().is_some_and(|r| !is_ring_ccw(r))
            || rings.iter().skip(1).any(|r| is_ring_ccw(r))
    };
    match geom {
        Geometry::Polygon(rings) => check(rings),
        Geometry::MultiPolygon(polys) => polys.iter().any(|p| check(p)),
        _ => false,
    }
}

// --- Spatial index ---

struct GeometryIndexEntry {
    layer: usize,
    feat: usize,
    part: Option<usize>,
    vertices: Vec<[f64; 2]>,
}

impl RTreeObject for GeometryIndexEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        if self.vertices.is_empty() {
            return AABB::from_point([0.0, 0.0]);
        }
        let (min_x, min_y, max_x, max_y) = self.vertices.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(ax, ay, bx, by), v| (ax.min(v[0]), ay.min(v[1]), bx.max(v[0]), by.max(v[1])),
        );
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

impl PointDistance for GeometryIndexEntry {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        self.vertices
            .iter()
            .map(|v| {
                let dx = v[0] - point[0];
                let dy = v[1] - point[1];
                dx * dx + dy * dy
            })
            .fold(f64::INFINITY, f64::min)
    }
}

#[must_use]
pub fn coord_f64(c: Coordinate) -> [f64; 2] {
    [f64::from(c[0]), f64::from(c[1])]
}
