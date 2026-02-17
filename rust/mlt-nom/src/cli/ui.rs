//! TUI visualizer for MLT files using ratatui

use std::collections::HashSet;
use std::fs::canonicalize;
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
use serde_json::Value as JsonValue;
use mlt_nom::geojson::{Coordinate, Feature, FeatureCollection, Geometry};
use mlt_nom::parse_layers;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap};

use crate::cli::ls::{FileSortColumn, LsRow, analyze_mlt_files, row_cells};

#[derive(Args)]
pub struct UiArgs {
    /// Path to the MLT file or directory
    path: PathBuf,
}

pub fn ui(args: &UiArgs) -> anyhow::Result<()> {
    let app = if args.path.is_dir() {
        let paths = find_mlt_files(&args.path)?;
        if paths.is_empty() {
            let path = canonicalize(&args.path)?;
            bail!("No .mlt files found in {}", path.display());
        }
        let base_path = args.path.clone();
        let paths_for_analysis = paths.clone();
        let loading_rows: Vec<LsRow> = paths
            .iter()
            .map(|p| LsRow::Loading {
                path: crate::cli::ls::relative_path(p, &base_path),
            })
            .collect();
        let file_entries: Vec<_> = paths.into_iter().zip(loading_rows).collect();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let rows = analyze_mlt_files(&paths_for_analysis, &base_path);
            let _ = tx.send(rows);
        });
        App::new_file_browser(file_entries, Some(rx))
    } else if args.path.is_file() {
        App::new_single_file(load_fc(&args.path)?, Some(args.path.clone()))
    } else {
        bail!("Path is not a file or directory");
    };
    run_app(app)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViewMode {
    FileBrowser,
    LayerOverview,
}

/// Which divider is being dragged for resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    /// Vertical divider between left panel (features/props) and map.
    LeftRight,
    /// Horizontal divider between features list and properties panel.
    FeaturesProperties,
}

/// Represents a selectable item in the tree view.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeItem {
    AllFeatures,
    /// Index into `App::layer_groups`.
    Layer(usize),
    /// `layer` = layer group index, `feat` = local feature index within that group.
    Feature {
        layer: usize,
        feat: usize,
    },
    /// Sub-part of a multi-geometry feature.
    SubFeature {
        layer: usize,
        feat: usize,
        part: usize,
    },
}

/// A group of features belonging to the same MLT layer.
struct LayerGroup {
    name: String,
    extent: f64,
    /// Global indices into `FeatureCollection::features`.
    feature_indices: Vec<usize>,
}

/// Application state for the visualizer.
struct App {
    mode: ViewMode,
    // File browser: (path for loading, ls row for display)
    mlt_files: Vec<(PathBuf, LsRow)>,
    selected_file_index: usize,
    file_list_state: TableState,
    /// Receiver for async analysis results; dropped when analysis completes
    analysis_rx: Option<mpsc::Receiver<Vec<LsRow>>>,
    /// Sort state for file table: (column, ascending). None = default order.
    file_sort: Option<(FileSortColumn, bool)>,
    /// Area of the file table (for header click detection).
    file_table_area: Option<Rect>,
    /// Column constraints used for the file table (for header click detection).
    file_table_widths: Option<[Constraint; 6]>,
    // Current file
    current_file: Option<PathBuf>,
    fc: FeatureCollection,
    layer_groups: Vec<LayerGroup>,
    // Tree
    tree_items: Vec<TreeItem>,
    selected_index: usize,
    list_state: ListState,
    hovered_item: Option<usize>,
    // Expansion
    expanded_layers: Vec<bool>,
    expanded_features: HashSet<(usize, usize)>,
    // Scroll acceleration
    last_scroll_time: Instant,
    scroll_speed: usize,
    // Rendering
    needs_redraw: bool,
    cached_bounds: Option<(f64, f64, f64, f64)>,
    cached_bounds_key: usize,
    // Resizable splits (percentages, 10..90)
    left_pct: u16,
    features_pct: u16,
    resizing: Option<ResizeHandle>,
    /// Scroll offset for properties panel.
    properties_scroll: u16,
    /// (layer, feat) of last feature we showed properties for; used to reset scroll on selection change.
    last_properties_key: Option<(usize, usize)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: ViewMode::FileBrowser,
            mlt_files: Vec::new(),
            selected_file_index: 0,
            file_list_state: TableState::default(),
            analysis_rx: None,
            file_sort: None,
            file_table_area: None,
            file_table_widths: None,
            current_file: None,
            fc: FeatureCollection {
                features: Vec::new(),
                ty: "FeatureCollection".into(),
            },
            layer_groups: Vec::new(),
            tree_items: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            hovered_item: None,
            expanded_layers: Vec::new(),
            expanded_features: HashSet::new(),
            last_scroll_time: Instant::now(),
            scroll_speed: 1,
            needs_redraw: true,
            cached_bounds: None,
            cached_bounds_key: usize::MAX,
            left_pct: 30,
            features_pct: 50,
            resizing: None,
            properties_scroll: 0,
            last_properties_key: None,
        }
    }
}

impl App {
    fn new_file_browser(
        mlt_files: Vec<(PathBuf, LsRow)>,
        analysis_rx: Option<mpsc::Receiver<Vec<LsRow>>>,
    ) -> Self {
        let mut file_list_state = TableState::default();
        file_list_state.select(Some(0));
        Self {
            mlt_files,
            file_list_state,
            analysis_rx,
            ..Self::default()
        }
    }

    fn new_single_file(fc: FeatureCollection, file_path: Option<PathBuf>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let layer_groups = group_by_layer(&fc);
        let expanded_layers = auto_expand(&layer_groups);
        let mut app = Self {
            mode: ViewMode::LayerOverview,
            current_file: file_path,
            list_state,
            expanded_layers,
            layer_groups,
            fc,
            ..Self::default()
        };
        app.build_tree_items();
        app
    }

    fn handle_file_header_click(&mut self, col: FileSortColumn) {
        let data_loaded = self.analysis_rx.is_none()
            && !self
                .mlt_files
                .iter()
                .any(|(_, r)| matches!(r, LsRow::Loading { .. }));
        if !data_loaded {
            return;
        }
        let selected_path = self
            .mlt_files
            .get(self.selected_file_index)
            .map(|(p, _)| p.clone());
        let (new_col, asc) = match self.file_sort {
            Some((c, a)) if c == col => (col, !a),
            _ => (col, true),
        };
        self.file_sort = Some((new_col, asc));
        self.mlt_files.sort_by(|a, b| file_cmp(a, b, new_col, asc));
        if let Some(path) = selected_path {
            if let Some(idx) = self.mlt_files.iter().position(|(p, _)| p == &path) {
                self.selected_file_index = idx;
                self.file_list_state.select(Some(idx));
            }
        }
        self.invalidate();
    }

    fn load_file(&mut self, path: &Path) -> anyhow::Result<()> {
        self.fc = load_fc(path)?;
        self.layer_groups = group_by_layer(&self.fc);
        self.current_file = Some(path.to_path_buf());
        self.mode = ViewMode::LayerOverview;
        self.expanded_layers = auto_expand(&self.layer_groups);
        self.expanded_features.clear();
        self.build_tree_items();
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.invalidate_bounds();
        Ok(())
    }

    /// Resolve a `(layer, feat)` pair to a global feature index.
    fn global_idx(&self, layer: usize, feat: usize) -> usize {
        self.layer_groups[layer].feature_indices[feat]
    }

    /// Get the `Feature` for a tree-item's `(layer, feat)` pair.
    fn feature(&self, layer: usize, feat: usize) -> &Feature {
        &self.fc.features[self.global_idx(layer, feat)]
    }

    fn get_extent(&self) -> f64 {
        self.layer_groups.first().map_or(4096.0, |g| g.extent)
    }

    fn get_selected_item(&self) -> &TreeItem {
        &self.tree_items[self.selected_index]
    }

    fn build_tree_items(&mut self) {
        self.tree_items.clear();
        self.tree_items.push(TreeItem::AllFeatures);

        for (li, group) in self.layer_groups.iter().enumerate() {
            self.tree_items.push(TreeItem::Layer(li));

            if !self.expanded_layers.get(li).copied().unwrap_or(false) {
                continue;
            }
            for (fi, &gi) in group.feature_indices.iter().enumerate() {
                self.tree_items.push(TreeItem::Feature {
                    layer: li,
                    feat: fi,
                });

                if self.expanded_features.contains(&(li, fi)) {
                    let n = multi_part_count(&self.fc.features[gi].geometry);
                    for part in 0..n {
                        self.tree_items.push(TreeItem::SubFeature {
                            layer: li,
                            feat: fi,
                            part,
                        });
                    }
                }
            }
        }
    }

    fn scroll_step(&mut self) -> usize {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_scroll_time).as_millis();
        self.last_scroll_time = now;
        self.scroll_speed = match elapsed_ms {
            0..50 => (self.scroll_speed + 1).min(20),
            50..120 => self.scroll_speed.max(2),
            _ => 1,
        };
        self.scroll_speed
    }

    fn move_up(&mut self) {
        self.move_up_by(1);
    }

    fn move_up_by(&mut self, n: usize) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                self.selected_file_index = self.selected_file_index.saturating_sub(n);
                self.file_list_state.select(Some(self.selected_file_index));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                self.selected_index = self.selected_index.saturating_sub(n);
                self.list_state.select(Some(self.selected_index));
                if self.selected_index != prev {
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn move_down(&mut self) {
        self.move_down_by(1);
    }

    fn move_down_by(&mut self, n: usize) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                let max = self.mlt_files.len().saturating_sub(1);
                self.selected_file_index = self.selected_file_index.saturating_add(n).min(max);
                self.file_list_state.select(Some(self.selected_file_index));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                let max = self.tree_items.len().saturating_sub(1);
                self.selected_index = self.selected_index.saturating_add(n).min(max);
                self.list_state.select(Some(self.selected_index));
                if self.selected_index != prev {
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn move_to_start(&mut self) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                self.selected_file_index = 0;
                self.file_list_state.select(Some(0));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                self.selected_index = 0;
                self.list_state.select(Some(0));
                if self.selected_index != prev {
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn move_to_end(&mut self) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                let max = self.mlt_files.len().saturating_sub(1);
                self.selected_file_index = max;
                self.file_list_state.select(Some(max));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                let max = self.tree_items.len().saturating_sub(1);
                self.selected_index = max;
                self.list_state.select(Some(max));
                if self.selected_index != prev {
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn handle_enter(&mut self) -> anyhow::Result<()> {
        match self.mode {
            ViewMode::FileBrowser => {
                let path = self
                    .mlt_files
                    .get(self.selected_file_index)
                    .map(|(p, _)| p.clone());
                if let Some(path) = path {
                    self.load_file(&path)?;
                }
            }
            ViewMode::LayerOverview => match self.tree_items.get(self.selected_index) {
                Some(TreeItem::Layer(li)) => {
                    let li = *li;
                    if li < self.expanded_layers.len() {
                        self.expanded_layers[li] = !self.expanded_layers[li];
                        self.build_tree_items();
                        self.invalidate();
                    }
                }
                Some(TreeItem::Feature { layer, feat }) => {
                    let key = (*layer, *feat);
                    if multi_part_count(&self.feature(key.0, key.1).geometry) > 0 {
                        if !self.expanded_features.remove(&key) {
                            self.expanded_features.insert(key);
                        }
                        self.build_tree_items();
                        self.invalidate();
                    }
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_plus(&mut self) {
        if self.mode != ViewMode::LayerOverview {
            return;
        }
        match self.tree_items.get(self.selected_index) {
            Some(TreeItem::Layer(li)) => {
                let li = *li;
                if li < self.expanded_layers.len() && !self.expanded_layers[li] {
                    self.expanded_layers[li] = true;
                    self.build_tree_items();
                    self.invalidate();
                }
            }
            Some(TreeItem::Feature { layer, feat }) => {
                let key = (*layer, *feat);
                if multi_part_count(&self.feature(key.0, key.1).geometry) > 0
                    && !self.expanded_features.contains(&key)
                {
                    self.expanded_features.insert(key);
                    self.build_tree_items();
                    self.invalidate();
                }
            }
            _ => {}
        }
    }

    fn handle_minus(&mut self) {
        if self.mode != ViewMode::LayerOverview {
            return;
        }
        match self.tree_items.get(self.selected_index).cloned() {
            Some(TreeItem::Layer(li)) => {
                if li < self.expanded_layers.len() && self.expanded_layers[li] {
                    self.expanded_layers[li] = false;
                    self.rebuild_and_clamp();
                }
            }
            Some(TreeItem::Feature { layer, feat }) => {
                if self.expanded_features.remove(&(layer, feat)) {
                    self.rebuild_and_clamp();
                } else if layer < self.expanded_layers.len() && self.expanded_layers[layer] {
                    self.expanded_layers[layer] = false;
                    self.rebuild_and_select(|it| matches!(it, TreeItem::Layer(l) if *l == layer));
                }
            }
            Some(TreeItem::SubFeature { layer, feat, .. }) => {
                self.expanded_features.remove(&(layer, feat));
                self.rebuild_and_select(|it| {
                    matches!(it, TreeItem::Feature { layer: l, feat: f } if *l == layer && *f == feat)
                });
            }
            _ => {}
        }
    }

    fn handle_star(&mut self) {
        if self.mode != ViewMode::LayerOverview {
            return;
        }
        let new_state = !self.expanded_layers.iter().all(|&e| e);
        self.expanded_layers.fill(new_state);
        self.rebuild_and_clamp();
    }

    fn handle_escape(&mut self) -> bool {
        match self.mode {
            ViewMode::FileBrowser => true,
            ViewMode::LayerOverview if self.mlt_files.is_empty() => true,
            ViewMode::LayerOverview => {
                self.mode = ViewMode::FileBrowser;
                // Do not clear fc/layer_groups/tree_items: dropping large feature collections
                // is slow. Data is overwritten on next load_file. File list (mlt_files) is
                // already in memory.
                self.invalidate_bounds();
                false
            }
        }
    }

    fn handle_left_arrow(&mut self) {
        if self.mode != ViewMode::LayerOverview {
            return;
        }
        let Some(item) = self.tree_items.get(self.selected_index).cloned() else {
            return;
        };
        let target = match item {
            TreeItem::SubFeature { layer, feat, .. } => self.tree_items.iter().position(|t| {
                matches!(t, TreeItem::Feature { layer: l, feat: f } if *l == layer && *f == feat)
            }),
            TreeItem::Feature { layer, .. } => {
                self.tree_items.iter().position(|t| matches!(t, TreeItem::Layer(l) if *l == layer))
            }
            TreeItem::Layer(_) => Some(0),
            TreeItem::AllFeatures => {
                if !self.mlt_files.is_empty() { self.mode = ViewMode::FileBrowser; }
                return;
            }
        };
        if let Some(idx) = target {
            if idx != self.selected_index {
                self.selected_index = idx;
                self.list_state.select(Some(idx));
                self.invalidate_bounds();
            }
        }
    }

    fn rebuild_and_clamp(&mut self) {
        self.build_tree_items();
        self.selected_index = self
            .selected_index
            .min(self.tree_items.len().saturating_sub(1));
        self.list_state.select(Some(self.selected_index));
        self.invalidate_bounds();
    }

    fn rebuild_and_select(&mut self, pred: impl Fn(&TreeItem) -> bool) {
        self.build_tree_items();
        if let Some(idx) = self.tree_items.iter().position(pred) {
            self.selected_index = idx;
        }
        self.list_state.select(Some(self.selected_index));
        self.invalidate_bounds();
    }

    fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    fn invalidate_bounds(&mut self) {
        self.cached_bounds = None;
        self.invalidate();
    }

    fn get_bounds(&mut self) -> (f64, f64, f64, f64) {
        if self.cached_bounds_key != self.selected_index || self.cached_bounds.is_none() {
            self.cached_bounds = Some(self.calculate_bounds());
            self.cached_bounds_key = self.selected_index;
        }
        self.cached_bounds.unwrap()
    }

    fn calculate_bounds(&self) -> (f64, f64, f64, f64) {
        let selected = self.get_selected_item();
        let extent = self.get_extent();
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);

        let mut update = |x: f64, y: f64| {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        };

        match selected {
            TreeItem::AllFeatures => {
                for feat in &self.fc.features {
                    for_each_coord(&feat.geometry, &mut update);
                }
            }
            TreeItem::Layer(l) => {
                for &gi in &self.layer_groups[*l].feature_indices {
                    for_each_coord(&self.fc.features[gi].geometry, &mut update);
                }
            }
            TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. } => {
                for_each_coord(&self.feature(*layer, *feat).geometry, &mut update);
            }
        }

        min_x = min_x.min(0.0);
        min_y = min_y.min(0.0);
        max_x = max_x.max(extent);
        max_y = max_y.max(extent);
        let px = (max_x - min_x) * 0.1;
        let py = (max_y - min_y) * 0.1;
        (min_x - px, min_y - py, max_x + px, max_y + py)
    }

    fn find_hovered_feature(&mut self, canvas_x: f64, canvas_y: f64, bounds: (f64, f64, f64, f64)) {
        let selected = self.get_selected_item().clone();
        let threshold = (bounds.2 - bounds.0).max(bounds.3 - bounds.1) * 0.02;
        let early_exit = threshold * threshold * 0.01;
        let mut best: Option<(usize, f64)> = None;

        for (idx, item) in self.tree_items.iter().enumerate() {
            let dist = match item {
                TreeItem::Feature { layer, feat } => {
                    if self.expanded_features.contains(&(*layer, *feat)) {
                        continue;
                    }
                    let visible = match &selected {
                        TreeItem::AllFeatures => true,
                        TreeItem::Layer(l) => *layer == *l,
                        TreeItem::Feature {
                            layer: sl,
                            feat: sf,
                        } => *layer == *sl && *feat == *sf,
                        TreeItem::SubFeature { .. } => false,
                    };
                    if !visible {
                        continue;
                    }
                    nearest_dist(
                        &self.feature(*layer, *feat).geometry,
                        canvas_x,
                        canvas_y,
                        threshold,
                    )
                }
                TreeItem::SubFeature { layer, feat, part } => {
                    let visible = match &selected {
                        TreeItem::Feature {
                            layer: sl,
                            feat: sf,
                        }
                        | TreeItem::SubFeature {
                            layer: sl,
                            feat: sf,
                            ..
                        } => *layer == *sl && *feat == *sf,
                        _ => false,
                    };
                    if !visible {
                        continue;
                    }
                    nearest_dist_sub(
                        &self.feature(*layer, *feat).geometry,
                        *part,
                        canvas_x,
                        canvas_y,
                        threshold,
                    )
                }
                _ => continue,
            };
            if let Some(d) = dist {
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((idx, d));
                    if d < early_exit {
                        break;
                    }
                }
            }
        }
        self.hovered_item = best.map(|(idx, _)| idx);
    }
}

fn load_fc(path: &Path) -> anyhow::Result<FeatureCollection> {
    let buffer = fs::read(path)?;
    let mut layers = parse_layers(&buffer)?;
    for layer in &mut layers {
        layer.decode_all()?;
    }
    Ok(FeatureCollection::from_layers(&layers)?)
}

fn group_by_layer(fc: &FeatureCollection) -> Vec<LayerGroup> {
    let mut groups: Vec<LayerGroup> = Vec::new();
    for (i, feat) in fc.features.iter().enumerate() {
        let name = feat
            .properties
            .get("_layer")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let extent = feat
            .properties
            .get("_extent")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(4096.0);
        if let Some(group) = groups.iter_mut().find(|g| g.name == name) {
            group.feature_indices.push(i);
        } else {
            groups.push(LayerGroup {
                name: name.to_string(),
                extent,
                feature_indices: vec![i],
            });
        }
    }
    groups
}

fn auto_expand(layer_groups: &[LayerGroup]) -> Vec<bool> {
    if layer_groups.len() == 1 {
        vec![true]
    } else {
        vec![false; layer_groups.len()]
    }
}

fn find_mlt_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    visit(&path, out)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("mlt") {
                    out.push(path);
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

fn click_row_in_area(col: u16, row: u16, area: Rect, scroll_offset: usize) -> Option<usize> {
    let top = area.y + 1;
    let bot = area.y + area.height.saturating_sub(1);
    (col >= area.x && col < area.x + area.width && row >= top && row < bot)
        .then(|| (row - top) as usize + scroll_offset)
}

/// Returns which column was clicked (0-5) if the click is in the header row.
fn file_header_click_column(
    area: Rect,
    widths: &[Constraint; 6],
    mouse_col: u16,
    mouse_row: u16,
) -> Option<FileSortColumn> {
    if mouse_row != area.y + 1 {
        return None;
    }
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(*widths)
        .split(inner);
    for (i, chunk) in chunks.iter().enumerate() {
        if mouse_col >= chunk.x && mouse_col < chunk.x + chunk.width {
            return Some(match i {
                0 => FileSortColumn::File,
                1 => FileSortColumn::Size,
                2 => FileSortColumn::EncPct,
                3 => FileSortColumn::Layers,
                4 => FileSortColumn::Features,
                5 => FileSortColumn::Geometry,
                _ => return None,
            });
        }
    }
    None
}

fn block_with_title(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title)
}

/// Hit zone width/height for divider grab (pixels each side of boundary).
const DIVIDER_GRAB: u16 = 2;

fn divider_hit(
    col: u16,
    row: u16,
    left_panel: Rect,
    tree_area: Rect,
) -> Option<ResizeHandle> {
    let left_right_divider = left_panel.x + left_panel.width;
    if col >= left_right_divider.saturating_sub(DIVIDER_GRAB)
        && col < left_right_divider.saturating_add(DIVIDER_GRAB)
        && row >= left_panel.y
        && row < left_panel.y + left_panel.height
    {
        return Some(ResizeHandle::LeftRight);
    }
    let feats_props_divider = tree_area.y + tree_area.height;
    if row >= feats_props_divider.saturating_sub(DIVIDER_GRAB)
        && row < feats_props_divider.saturating_add(DIVIDER_GRAB)
        && col >= left_panel.x
        && col < left_panel.x + left_panel.width
    {
        return Some(ResizeHandle::FeaturesProperties);
    }
    None
}

fn run_app(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(terminal.backend_mut(), EnableMouseCapture)?;
        run_app_loop(&mut terminal, &mut app)
    })();
    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn run_app_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    let mut map_area: Option<Rect> = None;
    let mut tree_area: Option<Rect> = None;
    let mut properties_area: Option<Rect> = None;
    let mut left_panel_area: Option<Rect> = None;
    let mut last_tree_click: Option<(Instant, usize)> = None;
    let mut last_file_click: Option<(Instant, usize)> = None;

    loop {
        // Check for async analysis results
        let rows = app.analysis_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(rows) = rows {
            if rows.len() == app.mlt_files.len() {
                for (i, row) in rows.into_iter().enumerate() {
                    if let Some(entry) = app.mlt_files.get_mut(i) {
                        entry.1 = row;
                    }
                }
            }
            app.analysis_rx = None;
            app.invalidate();
        }

        if app.needs_redraw {
            app.needs_redraw = false;
            terminal.draw(|f| match app.mode {
                ViewMode::FileBrowser => render_file_browser(f, app),
                ViewMode::LayerOverview => {
                    let right_pct = 100u16.saturating_sub(app.left_pct);
                    let props_pct = 100u16.saturating_sub(app.features_pct);
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(app.left_pct),
                            Constraint::Percentage(right_pct),
                        ])
                        .split(f.area());
                    let left_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(app.features_pct),
                            Constraint::Percentage(props_pct),
                        ])
                        .split(chunks[0]);
                    render_tree_panel(f, left_chunks[0], app);
                    render_properties_panel(f, left_chunks[1], app);
                    render_map_panel(f, chunks[1], &app);
                    tree_area = Some(left_chunks[0]);
                    properties_area = Some(left_chunks[1]);
                    left_panel_area = Some(chunks[0]);
                    map_area = Some(chunks[1]);
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
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Esc if app.handle_escape() => break,
                        KeyCode::Enter => app.handle_enter()?,
                        KeyCode::Char('+' | '=') | KeyCode::Right => app.handle_plus(),
                        KeyCode::Char('-') => app.handle_minus(),
                        KeyCode::Char('*') => app.handle_star(),
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Left => app.handle_left_arrow(),
                        KeyCode::PageUp => app.move_up_by(10),
                        KeyCode::PageDown => app.move_down_by(10),
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
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Up(_) => {
                        if app.resizing.take().is_some() {
                            app.invalidate();
                        }
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        if let Some(handle) = app.resizing {
                            let area = terminal.get_frame().area();
                            let left = left_panel_area.unwrap_or(Rect::default());
                            match handle {
                                ResizeHandle::LeftRight => {
                                    let pct = (mouse.column.saturating_sub(area.x) as f32
                                        / area.width.max(1) as f32
                                        * 100.0)
                                        .round() as u16;
                                    app.left_pct = pct.clamp(10, 90);
                                }
                                ResizeHandle::FeaturesProperties => {
                                    let pct = (mouse.row.saturating_sub(left.y) as f32
                                        / left.height.max(1) as f32
                                        * 100.0)
                                        .round() as u16;
                                    app.features_pct = pct.clamp(10, 90);
                                }
                            }
                            app.invalidate();
                            continue;
                        }
                        let prev = app.hovered_item;
                        app.hovered_item = None;

                        let hover_disabled = matches!(
                            app.tree_items.get(app.selected_index),
                            Some(TreeItem::Feature { layer, feat })
                                if !app.expanded_features.contains(&(*layer, *feat))
                        );

                        if app.mode == ViewMode::LayerOverview && !hover_disabled {
                            if let Some(area) = tree_area {
                                if let Some(row) = click_row_in_area(
                                    mouse.column,
                                    mouse.row,
                                    area,
                                    app.list_state.offset(),
                                ) {
                                    if row < app.tree_items.len()
                                        && matches!(
                                            app.tree_items[row],
                                            TreeItem::Feature { .. } | TreeItem::SubFeature { .. }
                                        )
                                    {
                                        app.hovered_item = Some(row);
                                    }
                                }
                            }
                            if app.hovered_item.is_none() {
                                if let Some(area) = map_area {
                                    if mouse.column >= area.x
                                        && mouse.column < area.x + area.width
                                        && mouse.row >= area.y
                                        && mouse.row < area.y + area.height
                                    {
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
                        if app.hovered_item != prev {
                            app.invalidate();
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        let s = app.scroll_step();
                        if app.mode == ViewMode::LayerOverview {
                            if let Some(area) = properties_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    app.properties_scroll =
                                        app.properties_scroll.saturating_sub(s as u16);
                                    app.invalidate();
                                    continue;
                                }
                            }
                        }
                        app.move_up_by(s);
                    }
                    MouseEventKind::ScrollDown => {
                        let s = app.scroll_step();
                        if app.mode == ViewMode::LayerOverview {
                            if let Some(area) = properties_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    app.properties_scroll = app.properties_scroll.saturating_add(s as u16);
                                    app.invalidate();
                                    continue;
                                }
                            }
                        }
                        app.move_down_by(s);
                    }
                    MouseEventKind::Down(_) => {
                        if app.mode == ViewMode::FileBrowser {
                            let area = terminal.get_frame().area();
                            let data_loaded = app.analysis_rx.is_none()
                                && !app
                                    .mlt_files
                                    .iter()
                                    .any(|(_, r)| matches!(r, LsRow::Loading { .. }));
                            if data_loaded {
                                if let Some(widths) = app.file_table_widths {
                                    if let Some(col) = file_header_click_column(
                                        area,
                                        &widths,
                                        mouse.column,
                                        mouse.row,
                                    ) {
                                        app.handle_file_header_click(col);
                                    }
                                }
                            }
                            if let Some(row) = click_row_in_area(
                                mouse.column,
                                mouse.row,
                                area,
                                app.file_list_state.offset(),
                            ) {
                                if row > 0 && row <= app.mlt_files.len() {
                                    let data_row = row - 1;
                                    let dbl = last_file_click.is_some_and(|(t, r)| {
                                        r == data_row && t.elapsed().as_millis() < 400
                                    });
                                    last_file_click = Some((Instant::now(), data_row));
                                    app.selected_file_index = data_row;
                                    app.file_list_state.select(Some(data_row));
                                    app.invalidate_bounds();
                                    if dbl {
                                        app.handle_enter()?;
                                    }
                                }
                            }
                        } else if app.mode == ViewMode::LayerOverview {
                            if let (Some(left), Some(tree)) =
                                (left_panel_area, tree_area)
                            {
                                if let Some(handle) = divider_hit(
                                    mouse.column,
                                    mouse.row,
                                    left,
                                    tree,
                                ) {
                                    app.resizing = Some(handle);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(area) = tree_area {
                                if let Some(row) = click_row_in_area(
                                    mouse.column,
                                    mouse.row,
                                    area,
                                    app.list_state.offset(),
                                ) {
                                    if row < app.tree_items.len() {
                                        let dbl = last_tree_click.is_some_and(|(t, r)| {
                                            r == row && t.elapsed().as_millis() < 400
                                        });
                                        last_tree_click = Some((Instant::now(), row));
                                        app.selected_index = row;
                                        app.list_state.select(Some(row));
                                        app.invalidate_bounds();
                                        if dbl {
                                            app.handle_enter()?;
                                        }
                                    }
                                }
                            }
                            if let Some(hovered) = app.hovered_item {
                                if let Some(area) = map_area {
                                    if mouse.column >= area.x
                                        && mouse.column < area.x + area.width
                                        && mouse.row >= area.y
                                        && mouse.row < area.y + area.height
                                    {
                                        app.selected_index = hovered;
                                        app.list_state.select(Some(hovered));
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

fn file_cmp(
    a: &(PathBuf, LsRow),
    b: &(PathBuf, LsRow),
    col: FileSortColumn,
    asc: bool,
) -> std::cmp::Ordering {
    let key = |r: &LsRow| -> (String, usize, f64) {
        match (r, col) {
            (LsRow::Info(i), FileSortColumn::File) => (i.path().to_string(), 0, 0.0),
            (LsRow::Info(i), FileSortColumn::Size) => (String::new(), i.size(), 0.0),
            (LsRow::Info(i), FileSortColumn::EncPct) => (String::new(), 0, i.encoding_pct()),
            (LsRow::Info(i), FileSortColumn::Layers) => (String::new(), i.layers(), 0.0),
            (LsRow::Info(i), FileSortColumn::Features) => (String::new(), i.features(), 0.0),
            (LsRow::Info(i), FileSortColumn::Geometry) => (i.geometries().to_string(), 0, 0.0),
            (LsRow::Error { path, .. }, FileSortColumn::File) => (path.clone(), 0, 0.0),
            (LsRow::Loading { path }, FileSortColumn::File) => (path.clone(), 0, 0.0),
            (LsRow::Error { path, .. }, _) => (path.clone(), 0, 0.0),
            (LsRow::Loading { path }, _) => (path.clone(), 0, 0.0),
        }
    };
    let (sa, na, fa) = key(&a.1);
    let (sb, nb, fb) = key(&b.1);
    let ord = sa.cmp(&sb).then(na.cmp(&nb)).then(fa.total_cmp(&fb));
    if asc { ord } else { ord.reverse() }
}

fn render_file_browser(f: &mut Frame<'_>, app: &mut App) {
    let area = f.area();
    app.file_table_area = Some(area);

    let data_loaded = app.analysis_rx.is_none()
        && !app
            .mlt_files
            .iter()
            .any(|(_, r)| matches!(r, LsRow::Loading { .. }));

    let files = &app.mlt_files;

    let header_cells = vec![
        Cell::from("File"),
        Cell::from("Size"),
        Cell::from("Enc %"),
        Cell::from("Layers"),
        Cell::from("Features"),
        Cell::from("Geometry"),
    ];
    let header = Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = files
        .iter()
        .map(|(_, row)| {
            let cells = row_cells(row);
            Row::new(vec![
                Cell::from(cells[0].clone()),
                Cell::from(cells[1].clone()),
                Cell::from(cells[2].clone()),
                Cell::from(cells[3].clone()),
                Cell::from(cells[4].clone()),
                Cell::from(cells[5].clone()),
            ])
        })
        .collect();

    let file_col_width = app
        .mlt_files
        .iter()
        .map(|(_, r)| row_cells(r)[0].chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let file_col_width = (file_col_width as u16).min(200);

    let widths = [
        Constraint::Length(file_col_width),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Min(8),
    ];
    app.file_table_widths = Some(widths);

    let sort_hint = if data_loaded {
        " Click header to sort"
    } else {
        ""
    };
    let title = format!(
        "MLT Files ({} found) - ↑/↓ navigate, Enter open, q quit{}",
        app.mlt_files.len(),
        sort_hint
    );
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(block_with_title(title))
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(table, f.area(), &mut app.file_list_state);
}

fn render_tree_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let (text, base_color) = match item {
                TreeItem::AllFeatures => ("All Features".to_string(), None),
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
                    (format!("  Layer: {} ({n} {label}, extent {})", group.name, group.extent), None)
                }
                TreeItem::Feature { layer, feat } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    let color = Some(geometry_color(geom));
                    let suffix = feature_suffix(geom);
                    (
                        format!("    Feat {feat}: {}{suffix}", geometry_type_name(geom)),
                        color,
                    )
                }
                TreeItem::SubFeature { layer, feat, part } => {
                    let geom = &app.feature(*layer, *feat).geometry;
                    let color = Some(geometry_color(geom));
                    let suffix = sub_feature_suffix(geom, *part);
                    (
                        format!("      Part {part}: {}{suffix}", sub_type_name(geom)),
                        color,
                    )
                }
            };

            let style = if idx == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if Some(idx) == app.hovered_item {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::UNDERLINED)
            } else if let Some(c) = base_color {
                Style::default().fg(c)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(text, style)))
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
    let list = List::new(items).block(block_with_title(title));
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn format_property_value(v: &JsonValue) -> String {
    match v {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(a) => serde_json::to_string(a).unwrap_or_else(|_| "[]".into()),
        JsonValue::Object(o) => serde_json::to_string(o).unwrap_or_else(|_| "{}".into()),
    }
}

fn render_properties_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let selected = app.selected_index;
    let item = app.tree_items.get(selected);
    let hovered = app.hovered_item.and_then(|i| app.tree_items.get(i));
    let (title, lines): (String, Vec<Line<'static>>) = match item {
        None | Some(TreeItem::AllFeatures) | Some(TreeItem::Layer(_)) => {
            match hovered {
                Some(TreeItem::Feature { layer, feat }) | Some(TreeItem::SubFeature { layer, feat, .. }) => {
                    let key = (*layer, *feat);
                    if app.last_properties_key != Some(key) {
                        app.properties_scroll = 0;
                        app.last_properties_key = Some(key);
                    }
                    let feat_ref = app.feature(*layer, *feat);
                    let mut prop_lines: Vec<Line<'static>> = feat_ref
                        .properties
                        .iter()
                        .filter(|(k, _)| *k != "_layer" && *k != "_extent")
                        .map(|(k, v)| {
                            let val_str = format_property_value(v);
                            Line::from(vec![
                                Span::styled(format!("{k}: "), Style::default().fg(Color::Cyan)),
                                Span::raw(val_str),
                            ])
                        })
                        .collect();
                    if prop_lines.is_empty() {
                        prop_lines.push(Line::from(Span::raw("(no properties)")));
                    }
                    (
                        format!("Properties (feat {feat}, hover)"),
                        prop_lines,
                    )
                }
                _ => {
                    app.last_properties_key = None;
                    (
                        "Properties".to_string(),
                        vec![Line::from(Span::raw("Select a feature or hover over map to view properties"))],
                    )
                }
            }
        }
        Some(TreeItem::Feature { layer, feat }) | Some(TreeItem::SubFeature { layer, feat, .. }) => {
            let key = (*layer, *feat);
            if app.last_properties_key != Some(key) {
                app.properties_scroll = 0;
                app.last_properties_key = Some(key);
            }
            let feat_ref = app.feature(*layer, *feat);
            let mut prop_lines: Vec<Line<'static>> = feat_ref
                .properties
                .iter()
                .filter(|(k, _)| *k != "_layer" && *k != "_extent")
                .map(|(k, v)| {
                    let val_str = format_property_value(v);
                    Line::from(vec![
                        Span::styled(format!("{k}: "), Style::default().fg(Color::Cyan)),
                        Span::raw(val_str),
                    ])
                })
                .collect();
            if prop_lines.is_empty() {
                prop_lines.push(Line::from(Span::raw("(no properties)")));
            }
            (
                format!("Properties (feat {feat})"),
                prop_lines,
            )
        }
    };
    let block = block_with_title(title);
    let inner_height = area.height.saturating_sub(2);
    let max_scroll = lines.len().saturating_sub(inner_height as usize) as u16;
    app.properties_scroll = app.properties_scroll.min(max_scroll);
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
}

fn render_map_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
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

            let hovered = app.hovered_item.and_then(|i| app.tree_items.get(i));

            // Collect global feature indices to draw
            let draw_feat = |ctx: &mut Context<'_>, gi: usize| {
                let geom = &app.fc.features[gi].geometry;
                let base = geometry_color(geom);
                let is_hovered = hovered.is_some_and(|h| match h {
                    TreeItem::Feature { layer, feat } => app.global_idx(*layer, *feat) == gi,
                    _ => false,
                });
                let sel_part = match selected {
                    TreeItem::SubFeature { layer, feat, part }
                        if app.global_idx(*layer, *feat) == gi =>
                    {
                        Some(*part)
                    }
                    _ => None,
                };
                let hov_part = hovered.and_then(|h| match h {
                    TreeItem::SubFeature { layer, feat, part }
                        if app.global_idx(*layer, *feat) == gi =>
                    {
                        Some(*part)
                    }
                    _ => None,
                });
                draw_feature(ctx, geom, base, is_hovered, sel_part, hov_part);
            };

            match selected {
                TreeItem::AllFeatures => {
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
                let highlight = matches!(pc, Color::White | Color::Yellow);
                draw_polygon(ctx, rings, highlight, pc);
            }
        }
    }
}

fn draw_point(ctx: &mut Context<'_>, c: Coordinate, color: Color) {
    ctx.print(
        f64::from(c[0]),
        f64::from(c[1]),
        Span::styled("×", Style::default().fg(color)),
    );
}

fn draw_line(ctx: &mut Context<'_>, coords: &[Coordinate], color: Color) {
    for w in coords.windows(2) {
        let (x1, y1) = (f64::from(w[0][0]), f64::from(w[0][1]));
        let (x2, y2) = (f64::from(w[1][0]), f64::from(w[1][1]));
        ctx.draw(&CanvasLine::new(x1, y1, x2, y2, color));
    }
}

fn draw_ring(ctx: &mut Context<'_>, ring: &[Coordinate], color: Color) {
    draw_line(ctx, ring, color);
    if let (Some(last), Some(first)) = (ring.last(), ring.first()) {
        ctx.draw(&CanvasLine::new(
            f64::from(last[0]),
            f64::from(last[1]),
            f64::from(first[0]),
            f64::from(first[1]),
            color,
        ));
    }
}

fn draw_polygon(
    ctx: &mut Context<'_>,
    rings: &[Vec<Coordinate>],
    is_highlighted: bool,
    fallback_color: Color,
) {
    for ring in rings {
        let color = if is_highlighted {
            fallback_color
        } else {
            ring_color(ring)
        };
        draw_ring(ctx, ring, color);
    }
}

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
        Geometry::Point(_) => Color::Magenta,
        Geometry::MultiPoint(_) => Color::LightMagenta,
        Geometry::LineString(_) => Color::Cyan,
        Geometry::MultiLineString(_) => Color::LightCyan,
        Geometry::Polygon(_) => Color::Green,
        Geometry::MultiPolygon(_) => Color::LightGreen,
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
            format!(" ({total}v)")
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
            format!(" ({total}v)")
        }),
        _ => String::new(),
    }
}

/// Determine sub-part highlight color.
/// Selected → Yellow, hovered → White, sibling of selected/hovered → `DarkGray`.
fn part_color(selected: Option<usize>, hovered: Option<usize>, idx: usize, base: Color) -> Color {
    if selected == Some(idx) {
        Color::Yellow
    } else if hovered == Some(idx) {
        Color::White
    } else if selected.is_some() || hovered.is_some() {
        Color::DarkGray
    } else {
        base
    }
}

/// Determine the display color for a polygon ring based on its winding order.
/// Counter-clockwise (outer ring) → Blue; clockwise (hole) → Red.
fn ring_color(ring: &[Coordinate]) -> Color {
    let mut area = 0.0_f64;
    for w in ring.windows(2) {
        let (x1, y1) = (f64::from(w[0][0]), f64::from(w[0][1]));
        let (x2, y2) = (f64::from(w[1][0]), f64::from(w[1][1]));
        area += (x2 - x1) * (y2 + y1);
    }
    if let (Some(last), Some(first)) = (ring.last(), ring.first()) {
        area +=
            (f64::from(first[0]) - f64::from(last[0])) * (f64::from(first[1]) + f64::from(last[1]));
    }
    if area < 0.0 { Color::Blue } else { Color::Red }
}

/// Iterate all coordinates in a geometry.
fn for_each_coord(geom: &Geometry, f: &mut impl FnMut(f64, f64)) {
    match geom {
        Geometry::Point(c) => f(f64::from(c[0]), f64::from(c[1])),
        Geometry::LineString(v) | Geometry::MultiPoint(v) => {
            for c in v {
                f(f64::from(c[0]), f64::from(c[1]));
            }
        }
        Geometry::Polygon(rings) => {
            for ring in rings {
                for c in ring {
                    f(f64::from(c[0]), f64::from(c[1]));
                }
            }
        }
        Geometry::MultiLineString(lines) => {
            for line in lines {
                for c in line {
                    f(f64::from(c[0]), f64::from(c[1]));
                }
            }
        }
        Geometry::MultiPolygon(polys) => {
            for p in polys {
                for r in p {
                    for c in r {
                        f(f64::from(c[0]), f64::from(c[1]));
                    }
                }
            }
        }
    }
}

/// Iterate coordinates of a specific sub-part of a multi-geometry.
fn for_each_sub_part_coord(geom: &Geometry, part: usize, f: &mut impl FnMut(f64, f64)) {
    match geom {
        Geometry::MultiPoint(v) => {
            if let Some(c) = v.get(part) {
                f(f64::from(c[0]), f64::from(c[1]));
            }
        }
        Geometry::MultiLineString(v) => {
            if let Some(line) = v.get(part) {
                for c in line {
                    f(f64::from(c[0]), f64::from(c[1]));
                }
            }
        }
        Geometry::MultiPolygon(v) => {
            if let Some(poly) = v.get(part) {
                for r in poly {
                    for c in r {
                        f(f64::from(c[0]), f64::from(c[1]));
                    }
                }
            }
        }
        _ => {}
    }
}

/// Find the minimum squared distance from a geometry's coordinates to a point.
fn nearest_dist(geom: &Geometry, cx: f64, cy: f64, threshold: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for_each_coord(geom, &mut |x, y| {
        let dx = (x - cx).abs();
        let dy = (y - cy).abs();
        if dx < threshold && dy < threshold {
            let d = dx * dx + dy * dy;
            if best.is_none_or(|b| d < b) {
                best = Some(d);
            }
        }
    });
    best
}

/// Find the minimum squared distance from a sub-part's coordinates to a point.
fn nearest_dist_sub(geom: &Geometry, part: usize, cx: f64, cy: f64, threshold: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for_each_sub_part_coord(geom, part, &mut |x, y| {
        let dx = (x - cx).abs();
        let dy = (y - cy).abs();
        if dx < threshold && dy < threshold {
            let d = dx * dx + dy * dy;
            if best.is_none_or(|b| d < b) {
                best = Some(d);
            }
        }
    });
    best
}
