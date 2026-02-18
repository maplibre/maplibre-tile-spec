//! TUI visualizer for MLT files using ratatui

mod rendering;

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

use crate::cli::ls::{FileSortColumn, LsRow, MltFileInfo, analyze_mlt_files, row_cells};
use crate::cli::ui::rendering::files::{
    render_file_browser, render_file_filter_panel, render_file_info_panel,
};
use crate::cli::ui::rendering::layers::{render_properties_panel, render_tree_panel};
use crate::cli::ui::rendering::map::render_map_panel;

#[derive(Args)]
pub struct UiArgs {
    /// Path to the MLT file or directory
    path: PathBuf,
}

pub fn ui(args: &UiArgs) -> anyhow::Result<()> {
    let app = if args.path.is_dir() {
        let paths = find_mlt_files(&args.path)?;
        if paths.is_empty() {
            bail!(
                "No .mlt files found in {}",
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
                        path: crate::cli::ls::relative_path(p, &base),
                    },
                )
            })
            .collect();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(analyze_mlt_files(&paths, &base, true));
        });
        App::new_file_browser(files, Some(rx))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeHandle {
    LeftRight,
    FeaturesProperties,
    FileBrowserLeftRight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeItem {
    All,
    Layer(usize),
    Feature {
        layer: usize,
        feat: usize,
    },
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

struct LayerGroup {
    name: String,
    extent: f64,
    feature_indices: Vec<usize>,
}

struct App {
    mode: ViewMode,
    mlt_files: Vec<(PathBuf, LsRow)>,
    selected_file_index: usize,
    file_list_state: TableState,
    analysis_rx: Option<mpsc::Receiver<Vec<LsRow>>>,
    file_sort: Option<(FileSortColumn, bool)>,
    file_table_area: Option<Rect>,
    file_table_widths: Option<[Constraint; 5]>,
    current_file: Option<PathBuf>,
    fc: FeatureCollection,
    layer_groups: Vec<LayerGroup>,
    tree_items: Vec<TreeItem>,
    selected_index: usize,
    hovered: Option<HoveredInfo>,
    expanded_layers: Vec<bool>,
    expanded_features: HashSet<(usize, usize)>,
    last_scroll_time: Instant,
    scroll_speed: usize,
    needs_redraw: bool,
    cached_bounds: Option<(f64, f64, f64, f64)>,
    cached_bounds_key: usize,
    left_pct: u16,
    features_pct: u16,
    resizing: Option<ResizeHandle>,
    properties_scroll: u16,
    tree_scroll: u16,
    tree_inner_height: usize,
    last_properties_key: Option<(usize, usize)>,
    geometry_index: Option<RTree<GeometryIndexEntry>>,
    file_left_pct: u16,
    geom_filters: HashSet<String>,
    algo_filters: HashSet<String>,
    filter_scroll: u16,
    filtered_file_indices: Vec<usize>,
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

    fn new_single_file(fc: FeatureCollection, path: Option<PathBuf>) -> Self {
        let layer_groups = group_by_layer(&fc);
        let expanded_layers = auto_expand(&layer_groups);
        let mut app = Self {
            mode: ViewMode::LayerOverview,
            current_file: path,
            expanded_layers,
            layer_groups,
            fc,
            ..Self::default()
        };
        app.build_geometry_index();
        app.build_tree_items();
        app
    }

    fn data_loaded(&self) -> bool {
        self.analysis_rx.is_none()
            && !self
                .mlt_files
                .iter()
                .any(|(_, r)| matches!(r, LsRow::Loading { .. }))
    }

    fn handle_file_header_click(&mut self, col: FileSortColumn) {
        if !self.data_loaded() {
            return;
        }
        let prev_path = self
            .selected_file_real_index()
            .and_then(|i| self.mlt_files.get(i))
            .map(|(p, _)| p.clone());
        let asc = !matches!(self.file_sort, Some((c, a)) if c == col && a);
        self.file_sort = Some((col, asc));
        self.mlt_files.sort_by(|a, b| file_cmp(a, b, col, asc));
        self.rebuild_filtered_files();
        if let Some(path) = prev_path {
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

    fn global_idx(&self, layer: usize, feat: usize) -> usize {
        self.layer_groups[layer].feature_indices[feat]
    }

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
                    for part in 0..multi_part_count(&self.fc.features[gi].geometry) {
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

    fn build_geometry_index(&mut self) {
        let mut entries: Vec<GeometryIndexEntry> = Vec::new();
        for (li, group) in self.layer_groups.iter().enumerate() {
            for (fi, &gi) in group.feature_indices.iter().enumerate() {
                let geom = &self.fc.features[gi].geometry;
                let n = multi_part_count(geom);
                let parts: Vec<Option<usize>> = if n == 0 {
                    vec![None]
                } else {
                    (0..n).map(Some).collect()
                };
                for part in parts {
                    let vertices = geometry_vertices(geom, part);
                    if !vertices.is_empty() {
                        entries.push(GeometryIndexEntry {
                            layer: li,
                            feat: fi,
                            part,
                            vertices,
                        });
                    }
                }
            }
        }
        self.geometry_index = (!entries.is_empty()).then(|| RTree::bulk_load(entries));
    }

    fn scroll_step(&mut self) -> usize {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_scroll_time).as_millis();
        self.last_scroll_time = now;
        self.scroll_speed = match elapsed {
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
                let max = self.filtered_file_indices.len().saturating_sub(1);
                self.selected_file_index = self.selected_file_index.saturating_sub(n).min(max);
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
                if let Some(path) = self
                    .selected_file_real_index()
                    .and_then(|i| self.mlt_files.get(i))
                    .map(|(p, _)| p.clone())
                {
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
            TreeItem::Feature { layer, .. } => self
                .tree_items
                .iter()
                .position(|t| matches!(t, TreeItem::Layer(l) if *l == layer)),
            TreeItem::Layer(_) => Some(0),
            TreeItem::All => {
                if !self.mlt_files.is_empty() {
                    self.mode = ViewMode::FileBrowser;
                }
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
        let pos = prev_real
            .and_then(|ri| self.filtered_file_indices.iter().position(|&i| i == ri))
            .unwrap_or(0);
        self.selected_file_index = pos;
        self.file_list_state.select(Some(pos));
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

    fn find_hovered_feature(&mut self, cx: f64, cy: f64, bounds: (f64, f64, f64, f64)) {
        let selected = self.get_selected_item().clone();
        let threshold = (bounds.2 - bounds.0).max(bounds.3 - bounds.1) * 0.02;
        let thresh_sq = threshold * threshold;
        let early_exit = thresh_sq * 0.01;
        let pt = [cx, cy];

        let best = if let Some(ref tree) = self.geometry_index {
            let mut best: Option<(f64, usize, usize, Option<usize>)> = None;
            for entry in tree.nearest_neighbor_iter(&pt) {
                let d = entry.distance_2(&pt);
                if d > thresh_sq {
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

    fn find_tree_idx_for_feature(
        &self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
    ) -> Option<usize> {
        for (idx, item) in self.tree_items.iter().enumerate() {
            match item {
                TreeItem::Layer(li) if *li == layer => {
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

    fn select_layer_expand_and_scroll(&mut self, layer: usize, tree_height: u16) {
        let inner = tree_height.saturating_sub(2) as usize;
        self.ensure_layer_expanded(layer);
        if let Some(idx) = self
            .tree_items
            .iter()
            .position(|it| matches!(it, TreeItem::Layer(li) if *li == layer))
        {
            self.selected_index = idx;
            self.scroll_selected_into_view(inner);
        }
        self.invalidate_bounds();
    }

    fn select_feature_expand_and_scroll(
        &mut self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
        tree_height: u16,
    ) {
        let inner = tree_height.saturating_sub(2) as usize;
        self.ensure_layer_expanded(layer);
        if multi_part_count(&self.feature(layer, feat).geometry) > 0 {
            self.expanded_features.insert((layer, feat));
            self.build_tree_items();
        }
        if let Some(idx) = self.find_tree_idx_for_feature(layer, feat, part) {
            self.selected_index = idx;
            self.scroll_selected_into_view(inner);
        }
        self.invalidate_bounds();
    }

    fn scroll_selected_into_view(&mut self, inner_height: usize) {
        let idx = self.selected_index;
        if idx < self.tree_scroll as usize {
            self.tree_scroll = u16::try_from(idx).unwrap_or(0);
        } else if inner_height > 0 && idx >= self.tree_scroll as usize + inner_height {
            self.tree_scroll =
                u16::try_from(idx.saturating_sub(inner_height.saturating_sub(1))).unwrap_or(0);
        }
    }

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

// --- Data loading ---

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

fn auto_expand(groups: &[LayerGroup]) -> Vec<bool> {
    if groups.len() == 1 {
        vec![true]
    } else {
        vec![false; groups.len()]
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

// --- Hit testing ---

fn point_in_rect(col: u16, row: u16, area: Rect) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

fn click_row_in_area(col: u16, row: u16, area: Rect, scroll_offset: usize) -> Option<usize> {
    let top = area.y + 1;
    let bot = area.y + area.height.saturating_sub(1);
    (col >= area.x && col < area.x + area.width && row >= top && row < bot)
        .then(|| (row - top) as usize + scroll_offset)
}

const HIGHLIGHT_SYMBOL_WIDTH: u16 = 3;
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
        let end = if i == resolved.len() - 1 {
            inner.x + inner.width
        } else {
            x + w
        };
        if mouse_col >= x && mouse_col < end {
            return Some(cols[i]);
        }
        x = end + COLUMN_SPACING;
    }
    None
}

fn block_with_title(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title)
}

const DIVIDER_GRAB: u16 = 2;

fn divider_hit(col: u16, row: u16, left: Rect, tree: Rect) -> Option<ResizeHandle> {
    let div_x = left.x + left.width;
    if col >= div_x.saturating_sub(DIVIDER_GRAB)
        && col < div_x.saturating_add(DIVIDER_GRAB)
        && row >= left.y
        && row < left.y + left.height
    {
        return Some(ResizeHandle::LeftRight);
    }
    let div_y = tree.y + tree.height;
    if row >= div_y.saturating_sub(DIVIDER_GRAB)
        && row < div_y.saturating_add(DIVIDER_GRAB)
        && col >= left.x
        && col < left.x + left.width
    {
        return Some(ResizeHandle::FeaturesProperties);
    }
    None
}

// --- App loop ---

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
    let mut properties_area: Option<Rect> = None;
    let mut left_panel_area: Option<Rect> = None;
    let mut file_filter_area: Option<Rect> = None;
    let mut file_info_area: Option<Rect> = None;
    let mut file_left_area: Option<Rect> = None;
    let mut last_tree_click: Option<(Instant, usize)> = None;
    let mut last_file_click: Option<(Instant, usize)> = None;

    loop {
        if let Some(rows) = app.analysis_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
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
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(app.file_left_pct),
                            Constraint::Percentage(100u16.saturating_sub(app.file_left_pct)),
                        ])
                        .split(f.area());
                    let right = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(chunks[1]);
                    render_file_browser(f, chunks[0], app);
                    render_file_filter_panel(f, right[0], app);
                    render_file_info_panel(f, right[1], app);
                    file_left_area = Some(chunks[0]);
                    file_filter_area = Some(right[0]);
                    file_info_area = Some(right[1]);
                }
                ViewMode::LayerOverview => {
                    let chunks = Layout::default()
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
                        .split(chunks[0]);
                    render_tree_panel(f, left[0], app);
                    render_properties_panel(f, left[1], app);
                    render_map_panel(f, chunks[1], app);
                    tree_area = Some(left[0]);
                    properties_area = Some(left[1]);
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
                            let pg = app.page_size().saturating_sub(1).max(1);
                            app.move_up_by(pg);
                        }
                        KeyCode::PageDown => {
                            let pg = app.page_size().saturating_sub(1).max(1);
                            app.move_down_by(pg);
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
                            match handle {
                                ResizeHandle::LeftRight => {
                                    app.left_pct = pct_at(mouse.column, area.x, area.width);
                                }
                                ResizeHandle::FeaturesProperties => {
                                    app.features_pct = pct_at(mouse.row, left.y, left.height);
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
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
                        let s = app.scroll_step();
                        let step = u16::try_from(s)?;
                        if app.mode == ViewMode::FileBrowser {
                            if file_filter_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                if up {
                                    app.filter_scroll = app.filter_scroll.saturating_sub(step);
                                } else {
                                    app.filter_scroll = app.filter_scroll.saturating_add(step);
                                }
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
                            if properties_area
                                .is_some_and(|a| point_in_rect(mouse.column, mouse.row, a))
                            {
                                if up {
                                    app.properties_scroll =
                                        app.properties_scroll.saturating_sub(step);
                                } else {
                                    app.properties_scroll =
                                        app.properties_scroll.saturating_add(step);
                                }
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
                            if let Some(left) = file_left_area {
                                let div_x = left.x + left.width;
                                if mouse.column >= div_x.saturating_sub(DIVIDER_GRAB)
                                    && mouse.column < div_x.saturating_add(DIVIDER_GRAB)
                                    && mouse.row >= left.y
                                    && mouse.row < left.y + left.height
                                {
                                    app.resizing = Some(ResizeHandle::FileBrowserLeftRight);
                                    app.invalidate();
                                    continue;
                                }
                            }
                            if let Some(fa) = file_filter_area {
                                if point_in_rect(mouse.column, mouse.row, fa) {
                                    let row = (mouse.row.saturating_sub(fa.y + 1)) as usize
                                        + app.filter_scroll as usize;
                                    handle_filter_click(app, row);
                                    continue;
                                }
                            }
                            if let Some(ia) = file_info_area {
                                if point_in_rect(mouse.column, mouse.row, ia)
                                    && app.filtered_file_indices.is_empty()
                                    && !app.mlt_files.is_empty()
                                {
                                    let row = (mouse.row.saturating_sub(ia.y + 1)) as usize;
                                    if row == 2 {
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
                                        let r = row - 1;
                                        let dbl = last_file_click.is_some_and(|(t, prev)| {
                                            prev == r && t.elapsed().as_millis() < 400
                                        });
                                        last_file_click = Some((Instant::now(), r));
                                        app.selected_file_index = r;
                                        app.file_list_state.select(Some(r));
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
                                        let dbl = last_tree_click.is_some_and(|(t, prev)| {
                                            prev == row && t.elapsed().as_millis() < 400
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
                                    if map_area
                                        .is_some_and(|m| point_in_rect(mouse.column, mouse.row, m))
                                    {
                                        app.handle_feature_click(
                                            info.layer,
                                            info.feat,
                                            info.part,
                                            tree.height,
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

// --- Sorting ---

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

// --- Filtering ---

fn handle_filter_click(app: &mut App, row: usize) {
    let geoms = collect_file_values(&app.mlt_files, MltFileInfo::geometries);
    let algos = collect_file_values(&app.mlt_files, MltFileInfo::algorithms);

    let geom_start = 3;
    let geom_end = geom_start + geoms.len();
    let algo_start = geom_end + 2;
    let algo_end = algo_start + algos.len();

    if row == 0 {
        app.geom_filters.clear();
        app.algo_filters.clear();
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

// --- Drawing primitives ---

fn coord_f64(c: Coordinate) -> [f64; 2] {
    [f64::from(c[0]), f64::from(c[1])]
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

fn geometry_vertices(geom: &Geometry, part: Option<usize>) -> Vec<[f64; 2]> {
    let coords = |cs: &[Coordinate]| cs.iter().copied().map(coord_f64).collect();
    let rings = |rs: &[Vec<Coordinate>]| {
        rs.iter()
            .flat_map(|r| r.iter().copied().map(coord_f64))
            .collect()
    };
    match (geom, part) {
        (Geometry::Point(c), None) => vec![coord_f64(*c)],
        (Geometry::LineString(v) | Geometry::MultiPoint(v), None) => coords(v),
        (Geometry::Polygon(r), None) => rings(r),
        (Geometry::MultiLineString(ls), None) => ls
            .iter()
            .flat_map(|l| l.iter().copied().map(coord_f64))
            .collect(),
        (Geometry::MultiPolygon(ps), None) => ps
            .iter()
            .flat_map(|p| p.iter().flat_map(|r| r.iter().copied().map(coord_f64)))
            .collect(),
        (Geometry::MultiPoint(v), Some(p)) => {
            v.get(p).map(|&c| vec![coord_f64(c)]).unwrap_or_default()
        }
        (Geometry::MultiLineString(v), Some(p)) => v.get(p).map_or_else(Vec::new, |l| coords(l)),
        (Geometry::MultiPolygon(v), Some(p)) => v.get(p).map_or_else(Vec::new, |poly| rings(poly)),
        _ => Vec::new(),
    }
}
