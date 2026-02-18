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
use mlt_nom::geojson::{Coordinate, Feature, FeatureCollection, Geometry};
use mlt_nom::parse_layers;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine, Rectangle};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};
use rstar::{AABB, PointDistance, RTree, RTreeObject};
use serde_json::Value as JsonValue;

use crate::cli::ls::{FileSortColumn, LsRow, MltFileInfo, analyze_mlt_files, row_cells};

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
            let rows = analyze_mlt_files(&paths_for_analysis, &base_path, true);
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
    /// Vertical divider in file browser between table and info panel.
    FileBrowserLeftRight,
}

/// Represents a selectable item in the tree view.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeItem {
    All,
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

impl TreeItem {
    fn layer_feat_part(&self) -> Option<(usize, usize, Option<usize>)> {
        match self {
            Self::Feature { layer, feat } => Some((*layer, *feat, None)),
            Self::SubFeature { layer, feat, part } => Some((*layer, *feat, Some(*part))),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct HoveredInfo {
    tree_idx: usize,
    layer: usize,
    feat: usize,
    part: Option<usize>,
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
    file_table_widths: Option<[Constraint; 5]>,
    // Current file
    current_file: Option<PathBuf>,
    fc: FeatureCollection,
    layer_groups: Vec<LayerGroup>,
    // Tree
    tree_items: Vec<TreeItem>,
    selected_index: usize,
    hovered: Option<HoveredInfo>,
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
    /// Scroll offset for tree/feature list (viewport only, selection unchanged).
    tree_scroll: u16,
    /// Last known tree panel inner height; used for `scroll_selected_into_view` on keyboard nav.
    tree_inner_height: usize,
    /// (layer, feat) of last feature we showed properties for; used to reset scroll on selection change.
    last_properties_key: Option<(usize, usize)>,
    /// R-tree index of geometry bounding boxes for efficient spatial search.
    geometry_index: Option<RTree<GeometryIndexEntry>>,
    /// Percentage of width for file table (left side) in file browser. 10..90.
    file_left_pct: u16,
    /// Active geometry type filters (empty = show all).
    geom_filters: HashSet<String>,
    /// Active algorithm combination filters (empty = show all).
    algo_filters: HashSet<String>,
    /// Scroll offset for the file browser filter panel.
    filter_scroll: u16,
    /// Indices into `mlt_files` that pass current filters (empty filters = all).
    filtered_file_indices: Vec<usize>,
    /// Last known file table inner height (rows visible).
    file_table_inner_height: usize,
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
            hovered: None,
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
            tree_scroll: 0,
            tree_inner_height: 0,
            geometry_index: None,
            file_left_pct: 70,
            geom_filters: HashSet::new(),
            algo_filters: HashSet::new(),
            filter_scroll: 0,
            filtered_file_indices: Vec::new(),
            file_table_inner_height: 10,
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
        let filtered_file_indices = (0..mlt_files.len()).collect();
        Self {
            mlt_files,
            file_list_state,
            analysis_rx,
            filtered_file_indices,
            ..Self::default()
        }
    }

    fn new_single_file(fc: FeatureCollection, file_path: Option<PathBuf>) -> Self {
        let layer_groups = group_by_layer(&fc);
        let expanded_layers = auto_expand(&layer_groups);
        let mut app = Self {
            mode: ViewMode::LayerOverview,
            current_file: file_path,
            expanded_layers,
            layer_groups,
            fc,
            ..Self::default()
        };
        app.build_geometry_index();
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
            .selected_file_real_index()
            .and_then(|i| self.mlt_files.get(i))
            .map(|(p, _)| p.clone());
        let (new_col, asc) = match self.file_sort {
            Some((c, a)) if c == col => (col, !a),
            _ => (col, true),
        };
        self.file_sort = Some((new_col, asc));
        self.mlt_files.sort_by(|a, b| file_cmp(a, b, new_col, asc));
        self.rebuild_filtered_files();
        if let Some(path) = selected_path {
            if let Some(pos) = self
                .filtered_file_indices
                .iter()
                .position(|&i| self.mlt_files[i].0 == path)
            {
                self.selected_file_index = pos;
                self.file_list_state.select(Some(pos));
            }
        }
    }

    fn load_file(&mut self, path: &Path) -> anyhow::Result<()> {
        self.fc = load_fc(path)?;
        self.layer_groups = group_by_layer(&self.fc);
        self.current_file = Some(path.to_path_buf());
        self.mode = ViewMode::LayerOverview;
        self.expanded_layers = auto_expand(&self.layer_groups);
        self.expanded_features.clear();
        self.build_geometry_index();
        self.build_tree_items();
        self.selected_index = 0;
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
        self.tree_items.push(TreeItem::All);

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

    /// Build rtree index once on tile load. For Multi* geometries, each sub-part gets its own entry.
    /// For non-Multi geometries, the whole geometry is one entry (with `part: None`).
    fn build_geometry_index(&mut self) {
        let mut entries: Vec<GeometryIndexEntry> = Vec::new();
        for (li, group) in self.layer_groups.iter().enumerate() {
            for (fi, &gi) in group.feature_indices.iter().enumerate() {
                let geom = &self.fc.features[gi].geometry;
                let n_parts = multi_part_count(geom);
                if n_parts == 0 {
                    let vertices = geometry_vertices(geom, None);
                    if !vertices.is_empty() {
                        entries.push(GeometryIndexEntry {
                            layer: li,
                            feat: fi,
                            part: None,
                            vertices,
                        });
                    }
                } else {
                    for part in 0..n_parts {
                        let vertices = geometry_vertices(geom, Some(part));
                        if !vertices.is_empty() {
                            entries.push(GeometryIndexEntry {
                                layer: li,
                                feat: fi,
                                part: Some(part),
                                vertices,
                            });
                        }
                    }
                }
            }
        }
        self.geometry_index = if entries.is_empty() {
            None
        } else {
            Some(RTree::bulk_load(entries))
        };
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

    fn move_up_by(&mut self, n: usize) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                self.selected_file_index = self.selected_file_index.saturating_sub(n);
                let max = self.filtered_file_indices.len().saturating_sub(1);
                self.selected_file_index = self.selected_file_index.min(max);
                self.file_list_state.select(Some(self.selected_file_index));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                self.selected_index = self.selected_index.saturating_sub(n);
                if self.selected_index != prev {
                    self.scroll_selected_into_view(self.tree_inner_height);
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn move_down_by(&mut self, n: usize) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                let max = self.filtered_file_indices.len().saturating_sub(1);
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
                if self.selected_index != prev {
                    self.scroll_selected_into_view(self.tree_inner_height);
                    self.invalidate_bounds();
                }
            }
        }
    }

    fn move_to_start(&mut self) {
        self.move_up_by(usize::MAX);
    }

    fn move_to_end(&mut self) {
        self.move_down_by(usize::MAX);
    }

    fn page_size(&self) -> usize {
        match self.mode {
            ViewMode::FileBrowser => self.file_table_inner_height,
            ViewMode::LayerOverview => self.tree_inner_height,
        }
    }

    fn handle_enter(&mut self) -> anyhow::Result<()> {
        match self.mode {
            ViewMode::FileBrowser => {
                let path = self
                    .selected_file_real_index()
                    .and_then(|i| self.mlt_files.get(i))
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
            TreeItem::All => {
                if !self.mlt_files.is_empty() { self.mode = ViewMode::FileBrowser; }
                return;
            }
        };
        if let Some(idx) = target {
            if idx != self.selected_index {
                self.selected_index = idx;
                self.invalidate_bounds();
            }
        }
    }

    fn rebuild_and_clamp(&mut self) {
        self.build_tree_items();
        self.selected_index = self
            .selected_index
            .min(self.tree_items.len().saturating_sub(1));
        self.invalidate_bounds();
    }

    fn rebuild_and_select(&mut self, pred: impl Fn(&TreeItem) -> bool) {
        self.build_tree_items();
        if let Some(idx) = self.tree_items.iter().position(pred) {
            self.selected_index = idx;
        }
        self.invalidate_bounds();
    }

    fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    fn invalidate_bounds(&mut self) {
        self.cached_bounds = None;
        self.invalidate();
    }

    fn selected_file_real_index(&self) -> Option<usize> {
        self.filtered_file_indices
            .get(self.selected_file_index)
            .copied()
    }

    fn rebuild_filtered_files(&mut self) {
        let prev_real = self.selected_file_real_index();
        let has_filters = !self.geom_filters.is_empty() || !self.algo_filters.is_empty();
        self.filtered_file_indices = (0..self.mlt_files.len())
            .filter(|&i| {
                !has_filters
                    || match &self.mlt_files[i].1 {
                        LsRow::Info(info) => {
                            file_matches_filters(info, &self.geom_filters, &self.algo_filters)
                        }
                        _ => true,
                    }
            })
            .collect();
        let new_pos = prev_real
            .and_then(|ri| self.filtered_file_indices.iter().position(|&i| i == ri))
            .unwrap_or(0);
        self.selected_file_index = new_pos;
        self.file_list_state.select(Some(new_pos));
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

        let mut update = |v: &[f64; 2]| {
            min_x = min_x.min(v[0]);
            min_y = min_y.min(v[1]);
            max_x = max_x.max(v[0]);
            max_y = max_y.max(v[1]);
        };

        let geoms: Vec<&Geometry> = match selected {
            TreeItem::All => self.fc.features.iter().map(|f| &f.geometry).collect(),
            TreeItem::Layer(l) => self.layer_groups[*l]
                .feature_indices
                .iter()
                .map(|&gi| &self.fc.features[gi].geometry)
                .collect(),
            TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. } => {
                vec![&self.feature(*layer, *feat).geometry]
            }
        };
        for geom in geoms {
            for v in geometry_vertices(geom, None) {
                update(&v);
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
        let threshold_sq = threshold * threshold;
        let early_exit = threshold_sq * 0.01;
        let query_point = [canvas_x, canvas_y];

        let best = if let Some(ref tree) = self.geometry_index {
            let mut best: Option<(f64, usize, usize, Option<usize>)> = None;
            for entry in tree.nearest_neighbor_iter(&query_point) {
                let d = entry.distance_2(&query_point);
                if d > threshold_sq {
                    break;
                }
                if !is_entry_visible(entry.layer, entry.feat, &selected) {
                    continue;
                }
                if best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, entry.layer, entry.feat, entry.part));
                    if d < early_exit {
                        break;
                    }
                }
            }
            best
        } else {
            None
        };

        self.hovered = best.and_then(|(_, layer, feat, part)| {
            self.find_tree_idx_for_feature(layer, feat, part)
                .map(|tree_idx| HoveredInfo {
                    tree_idx,
                    layer,
                    feat,
                    part,
                })
        });
    }

    /// Map (layer, feat, part) to `tree_idx` for hover highlighting. Returns None if layer is collapsed.
    fn find_tree_idx_for_feature(
        &self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
    ) -> Option<usize> {
        for (idx, item) in self.tree_items.iter().enumerate() {
            match item {
                TreeItem::Layer(li) if *li == layer => {
                    // Layer row: use when feature is from collapsed layer (no Feature items below)
                    if !self.expanded_layers.get(layer).copied().unwrap_or(false) {
                        return Some(idx);
                    }
                }
                TreeItem::Feature {
                    layer: li,
                    feat: fi,
                } if *li == layer && *fi == feat => {
                    if part.is_none() || !self.expanded_features.contains(&(layer, feat)) {
                        return Some(idx);
                    }
                }
                TreeItem::SubFeature {
                    layer: li,
                    feat: fi,
                    part: p,
                } if *li == layer && *fi == feat && part == Some(*p) => {
                    return Some(idx);
                }
                _ => {}
            }
        }
        None
    }

    fn ensure_layer_expanded(&mut self, layer: usize) {
        if layer < self.expanded_layers.len() && !self.expanded_layers[layer] {
            self.expanded_layers[layer] = true;
            self.build_tree_items();
        }
    }

    /// Ensure layer is expanded, select the layer row, and scroll it into view.
    fn select_layer_expand_and_scroll(&mut self, layer: usize, tree_height: u16) {
        let inner_height = tree_height.saturating_sub(2) as usize;
        self.ensure_layer_expanded(layer);
        if let Some(idx) = self
            .tree_items
            .iter()
            .position(|it| matches!(it, TreeItem::Layer(li) if *li == layer))
        {
            self.selected_index = idx;
            self.scroll_selected_into_view(inner_height);
        }
        self.invalidate_bounds();
    }

    /// Ensure layer is expanded, select the feature (or sub-item), rebuild tree, and scroll it into view.
    /// When the feature has sub-parts, auto-expands it so individual sub-items can be highlighted/selected.
    fn select_feature_expand_and_scroll(
        &mut self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
        tree_height: u16,
    ) {
        let inner_height = tree_height.saturating_sub(2) as usize;
        self.ensure_layer_expanded(layer);
        let geom = &self.feature(layer, feat).geometry;
        if multi_part_count(geom) > 0 {
            self.expanded_features.insert((layer, feat));
            self.build_tree_items();
        }
        if let Some(idx) = self.find_tree_idx_for_feature(layer, feat, part) {
            self.selected_index = idx;
            self.scroll_selected_into_view(inner_height);
        }
        self.invalidate_bounds();
    }

    /// Adjust `tree_scroll` so `selected_index` is visible in the viewport.
    fn scroll_selected_into_view(&mut self, inner_height: usize) {
        let idx = self.selected_index;
        let scroll_max = self.tree_scroll as usize + inner_height;
        if idx < self.tree_scroll as usize {
            self.tree_scroll = u16::try_from(idx).unwrap_or(0);
        } else if inner_height > 0 && idx >= scroll_max {
            self.tree_scroll =
                u16::try_from(idx.saturating_sub(inner_height.saturating_sub(1))).unwrap_or(0);
        }
    }

    /// Handle clicking on a feature (from either tree or map), dispatching based on current selection.
    fn handle_feature_click(
        &mut self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
        tree_height: u16,
    ) {
        match self.get_selected_item().clone() {
            TreeItem::All => self.select_layer_expand_and_scroll(layer, tree_height),
            TreeItem::Layer(_) => {
                self.select_feature_expand_and_scroll(layer, feat, None, tree_height);
            }
            _ => self.select_feature_expand_and_scroll(layer, feat, part, tree_height),
        }
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

fn point_in_rect(col: u16, row: u16, area: Rect) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

fn click_row_in_area(col: u16, row: u16, area: Rect, scroll_offset: usize) -> Option<usize> {
    let top = area.y + 1;
    let bot = area.y + area.height.saturating_sub(1);
    (col >= area.x && col < area.x + area.width && row >= top && row < bot)
        .then(|| (row - top) as usize + scroll_offset)
}

const HIGHLIGHT_SYMBOL_WIDTH: u16 = 3; // ">> "
const COLUMN_SPACING: u16 = 1;

fn file_header_click_column(
    area: Rect,
    widths: &[Constraint; 5],
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
        let col_end = if i == resolved.len() - 1 {
            inner.x + inner.width
        } else {
            x + w
        };
        if mouse_col >= x && mouse_col < col_end {
            return Some(cols[i]);
        }
        x = col_end + COLUMN_SPACING;
    }
    None
}

fn block_with_title(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title)
}

/// Hit zone width/height for divider grab (pixels each side of boundary).
const DIVIDER_GRAB: u16 = 2;

fn divider_hit(col: u16, row: u16, left_panel: Rect, tree_area: Rect) -> Option<ResizeHandle> {
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
    let mut file_filter_area: Option<Rect> = None;
    let mut file_info_area: Option<Rect> = None;
    let mut file_left_area: Option<Rect> = None;
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
            app.rebuild_filtered_files();
        }

        if app.needs_redraw {
            app.needs_redraw = false;
            terminal.draw(|f| match app.mode {
                ViewMode::FileBrowser => {
                    let right_pct = 100u16.saturating_sub(app.file_left_pct);
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(app.file_left_pct),
                            Constraint::Percentage(right_pct),
                        ])
                        .split(f.area());
                    let right_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(chunks[1]);
                    render_file_browser(f, chunks[0], app);
                    render_file_filter_panel(f, right_chunks[0], app);
                    render_file_info_panel(f, right_chunks[1], app);
                    file_left_area = Some(chunks[0]);
                    file_filter_area = Some(right_chunks[0]);
                    file_info_area = Some(right_chunks[1]);
                }
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
                    render_map_panel(f, chunks[1], app);
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
                        KeyCode::Up | KeyCode::Char('k') => app.move_up_by(1),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down_by(1),
                        KeyCode::Left => app.handle_left_arrow(),
                        KeyCode::PageUp => {
                            let page = app.page_size().saturating_sub(1).max(1);
                            app.move_up_by(page);
                        }
                        KeyCode::PageDown => {
                            let page = app.page_size().saturating_sub(1).max(1);
                            app.move_down_by(page);
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
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Up(_) => {
                        if app.resizing.take().is_some() {
                            app.invalidate();
                        }
                    }
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        if let Some(handle) = app.resizing {
                            let area = terminal.get_frame().area();
                            let left = left_panel_area.unwrap_or_default();
                            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            match handle {
                                ResizeHandle::LeftRight => {
                                    let pct = (f32::from(mouse.column.saturating_sub(area.x))
                                        / f32::from(area.width.max(1))
                                        * 100.0)
                                        .round()
                                        as u16;
                                    app.left_pct = pct.clamp(10, 90);
                                }
                                ResizeHandle::FeaturesProperties => {
                                    let pct = (f32::from(mouse.row.saturating_sub(left.y))
                                        / f32::from(left.height.max(1))
                                        * 100.0)
                                        .round()
                                        as u16;
                                    app.features_pct = pct.clamp(10, 90);
                                }
                                ResizeHandle::FileBrowserLeftRight => {
                                    let pct = (f32::from(mouse.column.saturating_sub(area.x))
                                        / f32::from(area.width.max(1))
                                        * 100.0)
                                        .round()
                                        as u16;
                                    app.file_left_pct = pct.clamp(10, 90);
                                }
                            }
                            app.invalidate();
                            continue;
                        }
                        let prev = app.hovered.clone();
                        app.hovered = None;

                        if app.mode == ViewMode::LayerOverview {
                            let tree_hover_enabled = !matches!(
                                app.tree_items.get(app.selected_index),
                                Some(TreeItem::Feature { layer, feat })
                                    if !app.expanded_features.contains(&(*layer, *feat))
                            );
                            if tree_hover_enabled {
                                if let Some(area) = tree_area {
                                    if let Some(row) = click_row_in_area(
                                        mouse.column,
                                        mouse.row,
                                        area,
                                        app.tree_scroll as usize,
                                    ) {
                                        if let Some((layer, feat, part)) = app
                                            .tree_items
                                            .get(row)
                                            .and_then(TreeItem::layer_feat_part)
                                        {
                                            app.hovered = Some(HoveredInfo {
                                                tree_idx: row,
                                                layer,
                                                feat,
                                                part,
                                            });
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
                    MouseEventKind::ScrollUp => {
                        let s = app.scroll_step();
                        if app.mode == ViewMode::FileBrowser {
                            if file_filter_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                app.filter_scroll =
                                    app.filter_scroll.saturating_sub(u16::try_from(s)?);
                                app.invalidate();
                                continue;
                            }
                            if file_info_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                continue;
                            }
                        }
                        if app.mode == ViewMode::LayerOverview {
                            if let Some(area) = properties_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    app.properties_scroll =
                                        app.properties_scroll.saturating_sub(u16::try_from(s)?);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(area) = tree_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    app.tree_scroll =
                                        app.tree_scroll.saturating_sub(u16::try_from(s)?);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if map_area.is_some_and(|a| point_in_rect(mouse.column, mouse.row, a)) {
                                continue;
                            }
                        }
                        app.move_up_by(s);
                    }
                    MouseEventKind::ScrollDown => {
                        let s = app.scroll_step();
                        if app.mode == ViewMode::FileBrowser {
                            if file_filter_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                app.filter_scroll =
                                    app.filter_scroll.saturating_add(u16::try_from(s)?);
                                app.invalidate();
                                continue;
                            }
                            if file_info_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                continue;
                            }
                        }
                        if app.mode == ViewMode::LayerOverview {
                            if let Some(area) = properties_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    app.properties_scroll =
                                        app.properties_scroll.saturating_add(u16::try_from(s)?);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(area) = tree_area {
                                if mouse.column >= area.x
                                    && mouse.column < area.x + area.width
                                    && mouse.row >= area.y
                                    && mouse.row < area.y + area.height
                                {
                                    let inner = area.height.saturating_sub(2) as usize;
                                    let max_off = u16::try_from(
                                        app.tree_items.len().saturating_sub(inner).max(0),
                                    )?;
                                    app.tree_scroll = app
                                        .tree_scroll
                                        .saturating_add(u16::try_from(s)?)
                                        .min(max_off);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if map_area.is_some_and(|a| point_in_rect(mouse.column, mouse.row, a)) {
                                continue;
                            }
                        }
                        app.move_down_by(s);
                    }
                    MouseEventKind::Down(_) => {
                        if app.mode == ViewMode::FileBrowser {
                            if let Some(left) = file_left_area {
                                let divider_x = left.x + left.width;
                                if mouse.column >= divider_x.saturating_sub(DIVIDER_GRAB)
                                    && mouse.column < divider_x.saturating_add(DIVIDER_GRAB)
                                    && mouse.row >= left.y
                                    && mouse.row < left.y + left.height
                                {
                                    app.resizing = Some(ResizeHandle::FileBrowserLeftRight);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(filter_area) = file_filter_area {
                                if point_in_rect(mouse.column, mouse.row, filter_area) {
                                    let row_in_panel = (mouse.row.saturating_sub(filter_area.y + 1))
                                        as usize
                                        + app.filter_scroll as usize;
                                    handle_filter_click(app, row_in_panel);
                                    continue;
                                }
                            }
                            if let Some(info_area) = file_info_area {
                                if point_in_rect(mouse.column, mouse.row, info_area)
                                    && app.filtered_file_indices.is_empty()
                                    && !app.mlt_files.is_empty()
                                {
                                    let row_in_panel =
                                        (mouse.row.saturating_sub(info_area.y + 1)) as usize;
                                    if row_in_panel == 2 {
                                        app.geom_filters.clear();
                                        app.algo_filters.clear();
                                        app.rebuild_filtered_files();
                                    }
                                    continue;
                                }
                            }
                            if let Some(area) = app.file_table_area {
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
                            }
                        } else if app.mode == ViewMode::LayerOverview {
                            if let (Some(left), Some(tree)) = (left_panel_area, tree_area) {
                                if let Some(handle) =
                                    divider_hit(mouse.column, mouse.row, left, tree)
                                {
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
                                    app.tree_scroll as usize,
                                ) {
                                    if row < app.tree_items.len() {
                                        let dbl = last_tree_click.is_some_and(|(t, r)| {
                                            r == row && t.elapsed().as_millis() < 400
                                        });
                                        last_tree_click = Some((Instant::now(), row));
                                        if let Some((layer, feat, part)) = app
                                            .tree_items
                                            .get(row)
                                            .and_then(TreeItem::layer_feat_part)
                                        {
                                            app.handle_feature_click(
                                                layer,
                                                feat,
                                                part,
                                                area.height,
                                            );
                                        } else {
                                            app.selected_index = row;
                                            app.scroll_selected_into_view(
                                                area.height.saturating_sub(2) as usize,
                                            );
                                        }
                                        app.invalidate_bounds();
                                        if dbl {
                                            app.handle_enter()?;
                                        }
                                    }
                                }
                            }
                            if let Some(ref info) = app.hovered {
                                if let Some(tree) = tree_area {
                                    if let Some(map) = map_area {
                                        if mouse.column >= map.x
                                            && mouse.column < map.x + map.width
                                            && mouse.row >= map.y
                                            && mouse.row < map.y + map.height
                                        {
                                            let (layer, feat, part) =
                                                (info.layer, info.feat, info.part);
                                            app.handle_feature_click(
                                                layer,
                                                feat,
                                                part,
                                                tree.height,
                                            );
                                            app.invalidate_bounds();
                                        }
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
    use std::cmp::Ordering;

    let ord = match (&a.1, &b.1) {
        (LsRow::Info(ai), LsRow::Info(bi)) => match col {
            FileSortColumn::File => ai.path().cmp(bi.path()),
            FileSortColumn::Size => ai.size().cmp(&bi.size()),
            FileSortColumn::EncPct => ai.encoding_pct().total_cmp(&bi.encoding_pct()),
            FileSortColumn::Layers => ai.layers().cmp(&bi.layers()),
            FileSortColumn::Features => ai.features().cmp(&bi.features()),
        },
        (LsRow::Info(_), _) => Ordering::Less,
        (_, LsRow::Info(_)) => Ordering::Greater,
        _ => a.0.cmp(&b.0),
    };
    if asc { ord } else { ord.reverse() }
}

fn render_file_browser(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.file_table_area = Some(area);
    app.file_table_inner_height = area.height.saturating_sub(3) as usize;

    let data_loaded = app.analysis_rx.is_none()
        && !app
            .mlt_files
            .iter()
            .any(|(_, r)| matches!(r, LsRow::Loading { .. }));

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from("File"),
        Cell::from(Line::from("Size").alignment(ratatui::layout::Alignment::Right)),
        Cell::from(Line::from("Enc %").alignment(ratatui::layout::Alignment::Right)),
        Cell::from(Line::from("Layers").alignment(ratatui::layout::Alignment::Right)),
        Cell::from(Line::from("Features").alignment(ratatui::layout::Alignment::Right)),
    ])
    .style(bold);

    let rows: Vec<Row> = app
        .filtered_file_indices
        .iter()
        .map(|&i| {
            let cells = row_cells(&app.mlt_files[i].1);
            Row::new(vec![
                Cell::from(cells[0].clone()),
                Cell::from(cells[1].clone()),
                Cell::from(cells[2].clone()),
                Cell::from(cells[3].clone()),
                Cell::from(cells[4].clone()),
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

    let widths = [
        Constraint::Length(u16::try_from(file_col_width).unwrap_or_default().min(200)),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(10),
    ];
    app.file_table_widths = Some(widths);

    let sort_hint = if data_loaded {
        " Click header to sort"
    } else {
        ""
    };
    let filtered = app.filtered_file_indices.len();
    let total = app.mlt_files.len();
    let count_str = if filtered < total {
        format!("{filtered}/{total}")
    } else {
        total.to_string()
    };
    let title =
        format!("MLT Files ({count_str} found) - ↑/↓ navigate, Enter open, q quit{sort_hint}");
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
    f.render_stateful_widget(table, area, &mut app.file_list_state);
}

fn handle_filter_click(app: &mut App, row: usize) {
    let geom_types = collect_all_geom_types(&app.mlt_files);
    let algo_types = collect_all_algorithms(&app.mlt_files);

    let geom_start = 3;
    let geom_end = geom_start + geom_types.len();
    let algo_start = geom_end + 2;
    let algo_end = algo_start + algo_types.len();

    if row == 0 {
        app.geom_filters.clear();
        app.algo_filters.clear();
    } else if row >= geom_start && row < geom_end {
        let g = &geom_types[row - geom_start];
        if !app.geom_filters.remove(g) {
            app.geom_filters.insert(g.clone());
        }
    } else if row >= algo_start && row < algo_end {
        let a = &algo_types[row - algo_start];
        if !app.algo_filters.remove(a) {
            app.algo_filters.insert(a.clone());
        }
    }
    app.rebuild_filtered_files();
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

fn collect_all_geom_types(files: &[(PathBuf, LsRow)]) -> Vec<String> {
    let mut set = HashSet::new();
    for (_, row) in files {
        if let LsRow::Info(info) = row {
            for g in info.geometries().split(',') {
                let g = g.trim();
                if !g.is_empty() {
                    set.insert(g.to_string());
                }
            }
        }
    }
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort();
    v
}

fn collect_all_algorithms(files: &[(PathBuf, LsRow)]) -> Vec<String> {
    let mut set = HashSet::new();
    for (_, row) in files {
        if let LsRow::Info(info) = row {
            for a in info.algorithms().split(',') {
                let a = a.trim();
                if !a.is_empty() {
                    set.insert(a.to_string());
                }
            }
        }
    }
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort();
    v
}

fn file_matches_filters(
    info: &MltFileInfo,
    geom_filters: &HashSet<String>,
    algo_filters: &HashSet<String>,
) -> bool {
    let file_geoms: HashSet<&str> = info.geometries().split(',').map(str::trim).collect();
    let file_algos: HashSet<&str> = info.algorithms().split(',').map(str::trim).collect();
    let geom_ok =
        geom_filters.is_empty() || geom_filters.iter().all(|g| file_geoms.contains(g.as_str()));
    let algo_ok =
        algo_filters.is_empty() || algo_filters.iter().all(|a| file_algos.contains(a.as_str()));
    geom_ok && algo_ok
}

fn render_file_filter_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let geom_types = collect_all_geom_types(&app.mlt_files);
    let algo_types = collect_all_algorithms(&app.mlt_files);
    let has_any = !app.geom_filters.is_empty() || !app.algo_filters.is_empty();

    let sel_info = app
        .selected_file_real_index()
        .and_then(|i| app.mlt_files.get(i))
        .and_then(|(_, r)| match r {
            LsRow::Info(i) => Some(i),
            _ => None,
        });
    let sel_geoms: HashSet<&str> = sel_info
        .map(|i| i.geometries().split(',').map(str::trim).collect())
        .unwrap_or_default();
    let sel_algos: HashSet<&str> = sel_info
        .map(|i| i.algorithms().split(',').map(str::trim).collect())
        .unwrap_or_default();

    let mut lines: Vec<Line<'static>> = Vec::new();

    let reset_style = if has_any {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled("[Reset filters]", reset_style)));
    lines.push(Line::from(""));

    if !geom_types.is_empty() {
        lines.push(Line::from(Span::styled(
            "Geometry Types:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for g in &geom_types {
            let checked = if app.geom_filters.contains(g) {
                "[x] "
            } else {
                "[ ] "
            };
            let style = if sel_geoms.contains(g.as_str()) {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            lines.push(Line::from(Span::styled(
                format!("  {checked}{}", geom_abbrev_to_full(g)),
                style,
            )));
        }
    }
    if !algo_types.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Algorithms:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for a in &algo_types {
            let checked = if app.algo_filters.contains(a) {
                "[x] "
            } else {
                "[ ] "
            };
            let style = if sel_algos.contains(a.as_str()) {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            lines.push(Line::from(Span::styled(format!("  {checked}{a}"), style)));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("(loading…)"));
    }

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len().saturating_sub(inner_height as usize)).unwrap_or(0);
    app.filter_scroll = app.filter_scroll.min(max_scroll);

    let para = Paragraph::new(lines)
        .block(block_with_title("Filter (click to toggle)"))
        .scroll((app.filter_scroll, 0));
    f.render_widget(para, area);
}

fn render_file_info_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let info = app
        .selected_file_real_index()
        .and_then(|i| app.mlt_files.get(i))
        .and_then(|(_, r)| match r {
            LsRow::Info(i) => Some(i),
            _ => None,
        });

    let lines: Vec<Line<'static>> = if let Some(info) = info {
        let fmt_size = |n: usize| format!("{:.1}B", size_format::SizeFormatterSI::new(n as u64));
        let label = Style::default().fg(Color::Cyan);
        let hint = Style::default().fg(Color::DarkGray);
        vec![
            Line::from(vec![
                Span::styled("File: ", label),
                Span::raw(info.path().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Size: ", label),
                Span::raw(fmt_size(info.size())),
                Span::styled("  raw MLT file size", hint),
            ]),
            Line::from(vec![
                Span::styled("Encoding: ", label),
                Span::raw(format!("{:.1}%", info.encoding_pct())),
                Span::styled("  MLT / (data + metadata)", hint),
            ]),
            Line::from(vec![
                Span::styled("Data: ", label),
                Span::raw(fmt_size(info.data_size())),
                Span::styled("  decoded payload size", hint),
            ]),
            Line::from(vec![
                Span::styled("Metadata: ", label),
                Span::raw(format!(
                    "{} ({:.1}% of data)",
                    fmt_size(info.meta_size()),
                    info.meta_pct()
                )),
                Span::styled("  encoding overhead", hint),
            ]),
            Line::from(vec![
                Span::styled("Layers: ", label),
                Span::raw(info.layers().to_string()),
                Span::styled("  tile layer count", hint),
            ]),
            Line::from(vec![
                Span::styled("Features: ", label),
                Span::raw(info.features().to_string()),
                Span::styled("  total across all layers", hint),
            ]),
            Line::from(vec![
                Span::styled("Streams: ", label),
                Span::raw(info.streams().to_string()),
                Span::styled("  encoded data streams", hint),
            ]),
            Line::from(vec![
                Span::styled("Geometries: ", label),
                Span::raw(info.geometries().to_string()),
                Span::styled("  geometry types present", hint),
            ]),
            Line::from(vec![
                Span::styled("Algorithms: ", label),
                Span::raw(info.algorithms().to_string()),
                Span::styled("  compression methods", hint),
            ]),
        ]
    } else if app.filtered_file_indices.is_empty() && !app.mlt_files.is_empty() {
        vec![
            Line::from("No files match the current filters."),
            Line::from(""),
            Line::from(Span::styled(
                "[Reset filters]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ]
    } else {
        vec![Line::from("Select a file to view details")]
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("File Info"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_tree_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
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
            } else if let Some(c) = base_color {
                Style::default().fg(c)
            } else {
                Style::default()
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

fn feature_property_lines(feat: &Feature) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = feat
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
    if lines.is_empty() {
        lines.push(Line::from(Span::raw("(no properties)")));
    }
    lines
}

fn render_properties_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_properties_top(f, chunks[0], app);
    render_geometry_stats(f, chunks[1], app);
}

fn render_properties_top(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let selected = app.selected_index;
    let item = app.tree_items.get(selected);
    let hovered = app.hovered.as_ref();
    let (title, lines): (String, Vec<Line<'static>>) = match item {
        None | Some(TreeItem::All | TreeItem::Layer(_)) => {
            if let Some(h) = hovered {
                let key = (h.layer, h.feat);
                if app.last_properties_key != Some(key) {
                    app.properties_scroll = 0;
                    app.last_properties_key = Some(key);
                }
                let feat_ref = app.feature(h.layer, h.feat);
                (
                    format!("Properties (feat {}, hover)", h.feat),
                    feature_property_lines(feat_ref),
                )
            } else {
                app.last_properties_key = None;
                (
                    "Properties".to_string(),
                    vec![Line::from(Span::raw(
                        "Select a feature or hover over map to view properties",
                    ))],
                )
            }
        }
        Some(TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. }) => {
            let key = (*layer, *feat);
            if app.last_properties_key != Some(key) {
                app.properties_scroll = 0;
                app.last_properties_key = Some(key);
            }
            let feat_ref = app.feature(*layer, *feat);
            (
                format!("Properties (feat {feat})"),
                feature_property_lines(feat_ref),
            )
        }
    };
    let block = block_with_title(title);
    let inner_height = area.height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len().saturating_sub(inner_height as usize)).unwrap_or(0);
    app.properties_scroll = app.properties_scroll.min(max_scroll);
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((app.properties_scroll, 0));
    f.render_widget(para, area);
}

fn geometry_stats_lines(geom: &Geometry) -> Vec<Line<'static>> {
    let cyan = Style::default().fg(Color::Cyan);
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Type: ", cyan),
        Span::raw(geometry_type_name(geom).to_string()),
    ]));

    match geom {
        Geometry::Point(c) => {
            lines.push(Line::from(vec![
                Span::styled("Coords: ", cyan),
                Span::raw(format!("[{}, {}]", c[0], c[1])),
            ]));
        }
        Geometry::MultiPoint(pts) => {
            lines.push(Line::from(vec![
                Span::styled("Points: ", cyan),
                Span::raw(pts.len().to_string()),
            ]));
        }
        Geometry::LineString(c) => {
            lines.push(Line::from(vec![
                Span::styled("Vertices: ", cyan),
                Span::raw(c.len().to_string()),
            ]));
        }
        Geometry::MultiLineString(v) => {
            lines.push(Line::from(vec![
                Span::styled("Parts: ", cyan),
                Span::raw(v.len().to_string()),
            ]));
            let total: usize = v.iter().map(Vec::len).sum();
            lines.push(Line::from(vec![
                Span::styled("Vertices: ", cyan),
                Span::raw(total.to_string()),
            ]));
        }
        Geometry::Polygon(rings) => {
            let total: usize = rings.iter().map(Vec::len).sum();
            lines.push(Line::from(vec![
                Span::styled("Vertices: ", cyan),
                Span::raw(total.to_string()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Rings: ", cyan),
                Span::raw(rings.len().to_string()),
            ]));
            for (i, ring) in rings.iter().enumerate() {
                let winding = if is_ring_ccw(ring) { "CCW" } else { "CW" };
                lines.push(Line::from(format!(
                    "  Ring {i}: {}v, {winding}",
                    ring.len()
                )));
            }
        }
        Geometry::MultiPolygon(polys) => {
            lines.push(Line::from(vec![
                Span::styled("Parts: ", cyan),
                Span::raw(polys.len().to_string()),
            ]));
            let total: usize = polys.iter().flat_map(|p| p.iter()).map(Vec::len).sum();
            lines.push(Line::from(vec![
                Span::styled("Vertices: ", cyan),
                Span::raw(total.to_string()),
            ]));
            for (pi, poly) in polys.iter().enumerate() {
                lines.push(Line::from(format!("  Poly {pi}: {} rings", poly.len())));
                for (ri, ring) in poly.iter().enumerate() {
                    let winding = if is_ring_ccw(ring) { "CCW" } else { "CW" };
                    lines.push(Line::from(format!(
                        "    Ring {ri}: {}v, {winding}",
                        ring.len()
                    )));
                }
            }
        }
    }

    lines
}

fn render_geometry_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
    let selected = app.selected_index;
    let item = app.tree_items.get(selected);
    let hovered = app.hovered.as_ref();

    let geom_opt: Option<&Geometry> = match item {
        Some(TreeItem::Feature { layer, feat } | TreeItem::SubFeature { layer, feat, .. }) => {
            Some(&app.feature(*layer, *feat).geometry)
        }
        _ => hovered.map(|h| &app.feature(h.layer, h.feat).geometry),
    };

    let lines = if let Some(geom) = geom_opt {
        geometry_stats_lines(geom)
    } else {
        vec![Line::from("Select a feature to view geometry details")]
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("Geometry"))
        .wrap(Wrap { trim: false });
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
                let hov_part = hovered.and_then(|h| {
                    if app.global_idx(h.layer, h.feat) == gi {
                        h.part
                    } else {
                        None
                    }
                });
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

fn ring_color(ring: &[Coordinate]) -> Color {
    if is_ring_ccw(ring) {
        Color::Blue
    } else {
        Color::Red
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
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) if has_nonstandard_winding(geom) => {
            Color::LightRed
        }
        Geometry::Polygon(_) => Color::Blue,
        Geometry::MultiPolygon(_) => Color::LightBlue,
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

/// Returns true if a geometry entry (layer, feat) matches the current selection for hover.
fn is_entry_visible(layer: usize, feat: usize, selected: &TreeItem) -> bool {
    match selected {
        TreeItem::All => true,
        TreeItem::Layer(l) => *l == layer,
        TreeItem::Feature {
            layer: sl,
            feat: sf,
        }
        | TreeItem::SubFeature {
            layer: sl,
            feat: sf,
            ..
        } => *sl == layer && *sf == feat,
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

/// Signed area of a ring (shoelace formula). Negative = CCW, positive = CW.
fn ring_signed_area(ring: &[Coordinate]) -> f64 {
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
    area
}

fn is_ring_ccw(ring: &[Coordinate]) -> bool {
    ring_signed_area(ring) < 0.0
}

/// Returns true if any ring has non-standard winding:
/// first ring CW (should be CCW) or additional rings CCW (should be CW).
fn has_nonstandard_winding(geom: &Geometry) -> bool {
    let check_rings = |rings: &[Vec<Coordinate>]| {
        rings.first().is_some_and(|r| !is_ring_ccw(r))
            || rings.iter().skip(1).any(|r| is_ring_ccw(r))
    };
    match geom {
        Geometry::Polygon(rings) => check_rings(rings),
        Geometry::MultiPolygon(polys) => polys.iter().any(|p| check_rings(p)),
        _ => false,
    }
}

/// Index entry for rstar spatial search. Stores layer/feat/part and vertices for distance computation.
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
            |(min_x, min_y, max_x, max_y), v| {
                (
                    min_x.min(v[0]),
                    min_y.min(v[1]),
                    max_x.max(v[0]),
                    max_y.max(v[1]),
                )
            },
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

/// Extract vertices from a geometry (full or sub-part) as f64 coordinates.
fn geometry_vertices(geom: &Geometry, part: Option<usize>) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    match (geom, part) {
        (Geometry::Point(c), None) => {
            out.push([f64::from(c[0]), f64::from(c[1])]);
        }
        (Geometry::LineString(v) | Geometry::MultiPoint(v), None) => {
            for c in v {
                out.push([f64::from(c[0]), f64::from(c[1])]);
            }
        }
        (Geometry::Polygon(rings), None) => {
            for ring in rings {
                for c in ring {
                    out.push([f64::from(c[0]), f64::from(c[1])]);
                }
            }
        }
        (Geometry::MultiLineString(lines), None) => {
            for line in lines {
                for c in line {
                    out.push([f64::from(c[0]), f64::from(c[1])]);
                }
            }
        }
        (Geometry::MultiPolygon(polys), None) => {
            for p in polys {
                for r in p {
                    for c in r {
                        out.push([f64::from(c[0]), f64::from(c[1])]);
                    }
                }
            }
        }
        (Geometry::MultiPoint(v), Some(p)) => {
            if let Some(c) = v.get(p) {
                out.push([f64::from(c[0]), f64::from(c[1])]);
            }
        }
        (Geometry::MultiLineString(v), Some(p)) => {
            if let Some(line) = v.get(p) {
                for c in line {
                    out.push([f64::from(c[0]), f64::from(c[1])]);
                }
            }
        }
        (Geometry::MultiPolygon(v), Some(p)) => {
            if let Some(poly) = v.get(p) {
                for r in poly {
                    for c in r {
                        out.push([f64::from(c[0]), f64::from(c[1])]);
                    }
                }
            }
        }
        _ => {}
    }
    out
}
