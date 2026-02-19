use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use mlt_core::geojson::{Coordinate, Feature, FeatureCollection, Geometry};
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::TableState;
use rstar::{PointDistance as _, RTree};

use crate::ls::{FileSortColumn, LsRow, MltFileInfo};
use crate::ui::{
    GeometryIndexEntry, auto_expand, coord_f64, group_by_layer, is_entry_visible, load_fc,
    multi_part_count,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    FileBrowser,
    LayerOverview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    LeftRight,
    FeaturesProperties,
    FileBrowserLeftRight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeItem {
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
    pub(crate) fn layer_feat_part(&self) -> Option<(usize, usize, Option<usize>)> {
        match self {
            Self::Feature { layer, feat } => Some((*layer, *feat, None)),
            Self::SubFeature { layer, feat, part } => Some((*layer, *feat, Some(*part))),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HoveredInfo {
    pub(crate) tree_idx: usize,
    pub(crate) layer: usize,
    pub(crate) feat: usize,
    pub(crate) part: Option<usize>,
}

impl HoveredInfo {
    pub fn new(tree_idx: usize, layer: usize, feat: usize, part: Option<usize>) -> Self {
        Self {
            tree_idx,
            layer,
            feat,
            part,
        }
    }
}

pub struct LayerGroup {
    pub(crate) name: String,
    pub(crate) extent: f64,
    pub(crate) feature_indices: Vec<usize>,
}

impl LayerGroup {
    pub fn new(name: String, extent: f64, feature_indices: Vec<usize>) -> Self {
        Self {
            name,
            extent,
            feature_indices,
        }
    }
}

pub struct App {
    pub(crate) mode: ViewMode,
    pub(crate) mlt_files: Vec<(PathBuf, LsRow)>,
    pub(crate) selected_file_index: usize,
    pub(crate) file_list_state: TableState,
    pub(crate) analysis_rx: Option<mpsc::Receiver<Vec<LsRow>>>,
    file_sort: Option<(FileSortColumn, bool)>,
    pub(crate) file_table_area: Option<Rect>,
    pub(crate) file_table_widths: Option<[Constraint; 5]>,
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) fc: FeatureCollection,
    pub(crate) layer_groups: Vec<LayerGroup>,
    pub(crate) tree_items: Vec<TreeItem>,
    pub(crate) selected_index: usize,
    pub(crate) hovered: Option<HoveredInfo>,
    expanded_layers: Vec<bool>,
    pub(crate) expanded_features: HashSet<(usize, usize)>,
    last_scroll_time: Instant,
    scroll_speed: usize,
    pub(crate) needs_redraw: bool,
    cached_bounds: Option<(f64, f64, f64, f64)>,
    cached_bounds_key: usize,
    pub(crate) left_pct: u16,
    pub(crate) features_pct: u16,
    pub(crate) resizing: Option<ResizeHandle>,
    pub(crate) properties_scroll: u16,
    pub(crate) tree_scroll: u16,
    pub(crate) tree_inner_height: usize,
    pub(crate) last_properties_key: Option<(usize, usize)>,
    geometry_index: Option<RTree<GeometryIndexEntry>>,
    pub(crate) file_left_pct: u16,
    pub(crate) ext_filters: HashSet<String>,
    pub(crate) geom_filters: HashSet<String>,
    pub(crate) algo_filters: HashSet<String>,
    pub(crate) filter_scroll: u16,
    pub(crate) filtered_file_indices: Vec<usize>,
    pub(crate) file_table_inner_height: usize,
    pub(crate) show_help: bool,
    pub(crate) help_scroll: u16,
    pub(crate) error_popup: Option<(String, String)>,
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
            ext_filters: HashSet::new(),
            geom_filters: HashSet::new(),
            algo_filters: HashSet::new(),
            filter_scroll: 0,
            filtered_file_indices: Vec::new(),
            file_table_inner_height: 10,
            show_help: false,
            help_scroll: 0,
            error_popup: None,
        }
    }
}

impl App {
    pub(crate) fn new_file_browser(
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

    pub(crate) fn new_single_file(fc: FeatureCollection, path: Option<PathBuf>) -> Self {
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

    pub(crate) fn data_loaded(&self) -> bool {
        self.analysis_rx.is_none()
            && !self
                .mlt_files
                .iter()
                .any(|(_, r)| matches!(r, LsRow::Loading { .. }))
    }

    pub(crate) fn open_help(&mut self) {
        self.show_help = true;
        self.help_scroll = 0;
        self.invalidate();
    }

    pub(crate) fn handle_file_header_click(&mut self, col: FileSortColumn) {
        if !self.data_loaded() {
            return;
        }
        let prev = self
            .selected_file_real_index()
            .and_then(|i| self.mlt_files.get(i))
            .map(|(p, _)| p.clone());
        let asc = !matches!(self.file_sort, Some((c, a)) if c == col && a);
        self.file_sort = Some((col, asc));
        self.mlt_files.sort_by(|a, b| file_cmp(a, b, col, asc));
        self.rebuild_filtered_files();
        if let Some(path) = prev {
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

    pub(crate) fn global_idx(&self, layer: usize, feat: usize) -> usize {
        self.layer_groups[layer].feature_indices[feat]
    }

    pub(crate) fn feature(&self, layer: usize, feat: usize) -> &Feature {
        &self.fc.features[self.global_idx(layer, feat)]
    }

    pub(crate) fn extent(&self) -> f64 {
        self.layer_groups.first().map_or(4096.0, |g| g.extent)
    }

    pub(crate) fn selected_item(&self) -> &TreeItem {
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

    pub(crate) fn scroll_step(&mut self) -> usize {
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

    pub(crate) fn move_up_by(&mut self, n: usize) {
        self.move_by(n, false);
    }

    pub(crate) fn move_down_by(&mut self, n: usize) {
        self.move_by(n, true);
    }

    fn move_by(&mut self, n: usize, down: bool) {
        match self.mode {
            ViewMode::FileBrowser => {
                let prev = self.selected_file_index;
                let max = self.filtered_file_indices.len().saturating_sub(1);
                self.selected_file_index = if down {
                    self.selected_file_index.saturating_add(n).min(max)
                } else {
                    self.selected_file_index.saturating_sub(n).min(max)
                };
                self.file_list_state.select(Some(self.selected_file_index));
                if self.selected_file_index != prev {
                    self.invalidate_bounds();
                }
            }
            ViewMode::LayerOverview => {
                let prev = self.selected_index;
                let max = self.tree_items.len().saturating_sub(1);
                self.selected_index = if down {
                    self.selected_index.saturating_add(n).min(max)
                } else {
                    self.selected_index.saturating_sub(n)
                };
                if self.selected_index != prev {
                    self.scroll_selected_into_view(self.tree_inner_height);
                    self.invalidate_bounds();
                }
            }
        }
    }

    pub(crate) fn move_to_start(&mut self) {
        self.move_up_by(usize::MAX);
    }

    pub(crate) fn move_to_end(&mut self) {
        self.move_down_by(usize::MAX);
    }

    pub(crate) fn page_size(&self) -> usize {
        match self.mode {
            ViewMode::FileBrowser => self.file_table_inner_height,
            ViewMode::LayerOverview => self.tree_inner_height,
        }
    }

    pub(crate) fn handle_enter(&mut self) {
        match self.mode {
            ViewMode::FileBrowser => {
                if let Some((path, row)) = self
                    .selected_file_real_index()
                    .and_then(|i| self.mlt_files.get(i))
                    .map(|(p, r)| (p.clone(), r))
                {
                    let path_str = match row {
                        LsRow::Info(info) => info.path().to_string(),
                        LsRow::Error { path: p, .. } | LsRow::Loading { path: p } => p.clone(),
                    };
                    if let LsRow::Error { error, .. } = row {
                        self.error_popup = Some((path_str, error.clone()));
                        self.invalidate();
                    } else if let Err(e) = self.load_file(&path) {
                        self.error_popup = Some((path_str, e.to_string()));
                        self.invalidate();
                    }
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
    }

    pub(crate) fn handle_plus(&mut self) {
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

    pub(crate) fn handle_minus(&mut self) {
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

    pub(crate) fn handle_star(&mut self) {
        if self.mode != ViewMode::LayerOverview {
            return;
        }
        let new = !self.expanded_layers.iter().all(|&e| e);
        self.expanded_layers.fill(new);
        self.rebuild_and_clamp();
    }

    pub(crate) fn handle_escape(&mut self) -> bool {
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

    pub(crate) fn handle_left_arrow(&mut self) {
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

    pub(crate) fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    pub(crate) fn invalidate_bounds(&mut self) {
        self.cached_bounds = None;
        self.invalidate();
    }

    pub(crate) fn selected_file_real_index(&self) -> Option<usize> {
        self.filtered_file_indices
            .get(self.selected_file_index)
            .copied()
    }

    pub(crate) fn rebuild_filtered_files(&mut self) {
        let prev = self.selected_file_real_index();
        let has_filters = !self.ext_filters.is_empty()
            || !self.geom_filters.is_empty()
            || !self.algo_filters.is_empty();
        self.filtered_file_indices = (0..self.mlt_files.len())
            .filter(|&i| {
                if !has_filters {
                    return true;
                }
                let path = &self.mlt_files[i].0;
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_lowercase)
                    .unwrap_or_default();
                let ext_match = self.ext_filters.is_empty()
                    || self.ext_filters.iter().any(|f| f.as_str() == ext);
                if !ext_match {
                    return false;
                }
                match &self.mlt_files[i].1 {
                    LsRow::Info(info) => {
                        file_matches_filters(info, &self.geom_filters, &self.algo_filters)
                    }
                    _ => true,
                }
            })
            .collect();
        let pos = prev
            .and_then(|ri| self.filtered_file_indices.iter().position(|&i| i == ri))
            .unwrap_or(0);
        self.selected_file_index = pos;
        self.file_list_state.select(Some(pos));
        self.invalidate();
    }

    pub(crate) fn get_bounds(&mut self) -> (f64, f64, f64, f64) {
        if self.cached_bounds_key != self.selected_index || self.cached_bounds.is_none() {
            self.cached_bounds = Some(self.calculate_bounds());
            self.cached_bounds_key = self.selected_index;
        }
        self.cached_bounds.unwrap()
    }

    pub fn calculate_bounds(&self) -> (f64, f64, f64, f64) {
        let sel = self.selected_item();
        let ext = self.extent();
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);

        let mut update = |v: &[f64; 2]| {
            x0 = x0.min(v[0]);
            y0 = y0.min(v[1]);
            x1 = x1.max(v[0]);
            y1 = y1.max(v[1]);
        };

        let geoms: Vec<&Geometry> = match sel {
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

        x0 = x0.min(0.0);
        y0 = y0.min(0.0);
        x1 = x1.max(ext);
        y1 = y1.max(ext);
        let px = (x1 - x0) * 0.1;
        let py = (y1 - y0) * 0.1;
        (x0 - px, y0 - py, x1 + px, y1 + py)
    }

    pub(crate) fn find_hovered_feature(&mut self, cx: f64, cy: f64, bounds: (f64, f64, f64, f64)) {
        let sel = self.selected_item().clone();
        let threshold = (bounds.2 - bounds.0).max(bounds.3 - bounds.1) * 0.02;
        let thresh_sq = threshold * threshold;
        let early_exit = thresh_sq * 0.01;
        let pt = [cx, cy];

        let best = if let Some(ref tree) = self.geometry_index {
            let mut best: Option<(f64, usize, usize, Option<usize>)> = None;
            for e in tree.nearest_neighbor_iter(&pt) {
                let d = e.distance_2(&pt);
                if d > thresh_sq {
                    break;
                }
                if !is_entry_visible(e.layer, e.feat, &sel) {
                    continue;
                }
                if best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, e.layer, e.feat, e.part));
                    if d < early_exit {
                        break;
                    }
                }
            }
            best
        } else {
            None
        };

        self.hovered = best.and_then(|(_, l, f, p)| {
            self.find_tree_idx_for_feature(l, f, p)
                .map(|idx| HoveredInfo::new(idx, l, f, p))
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
                TreeItem::Feature { layer: l, feat: f } if *l == layer && *f == feat => {
                    if part.is_none() || !self.expanded_features.contains(&(layer, feat)) {
                        return Some(idx);
                    }
                }
                TreeItem::SubFeature {
                    layer: l,
                    feat: f,
                    part: p,
                } if *l == layer && *f == feat && part == Some(*p) => {
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

    fn select_and_scroll(
        &mut self,
        layer: usize,
        feat: Option<usize>,
        part: Option<usize>,
        tree_height: u16,
    ) {
        let inner = tree_height.saturating_sub(2) as usize;
        self.ensure_layer_expanded(layer);
        if let Some(f) = feat {
            if multi_part_count(&self.feature(layer, f).geometry) > 0 {
                self.expanded_features.insert((layer, f));
                self.build_tree_items();
            }
            if let Some(idx) = self.find_tree_idx_for_feature(layer, f, part) {
                self.selected_index = idx;
                self.scroll_selected_into_view(inner);
            }
        } else if let Some(idx) = self
            .tree_items
            .iter()
            .position(|it| matches!(it, TreeItem::Layer(l) if *l == layer))
        {
            self.selected_index = idx;
            self.scroll_selected_into_view(inner);
        }
        self.invalidate_bounds();
    }

    pub(crate) fn scroll_selected_into_view(&mut self, inner_height: usize) {
        let idx = self.selected_index;
        if idx < self.tree_scroll as usize {
            self.tree_scroll = u16::try_from(idx).unwrap_or(0);
        } else if inner_height > 0 && idx >= self.tree_scroll as usize + inner_height {
            self.tree_scroll =
                u16::try_from(idx.saturating_sub(inner_height.saturating_sub(1))).unwrap_or(0);
        }
    }

    pub(crate) fn handle_feature_click(
        &mut self,
        layer: usize,
        feat: usize,
        part: Option<usize>,
        tree_height: u16,
    ) {
        match self.selected_item().clone() {
            TreeItem::All => self.select_and_scroll(layer, None, None, tree_height),
            TreeItem::Layer(_) => {
                self.select_and_scroll(layer, Some(feat), None, tree_height);
            }
            _ => self.select_and_scroll(layer, Some(feat), part, tree_height),
        }
    }
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

fn file_matches_filters(
    info: &MltFileInfo,
    geom_filters: &HashSet<String>,
    algo_filters: &HashSet<String>,
) -> bool {
    let file_geoms: HashSet<&str> = info.geometries().split(',').map(str::trim).collect();
    let file_algos: HashSet<&str> = info.algorithms().split(',').map(str::trim).collect();
    (geom_filters.is_empty() || geom_filters.iter().all(|g| file_geoms.contains(g.as_str())))
        && (algo_filters.is_empty() || algo_filters.iter().all(|a| file_algos.contains(a.as_str())))
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
        (Geometry::MultiPolygon(v), Some(p)) => v.get(p).map_or_else(Vec::new, |r| rings(r)),
        _ => Vec::new(),
    }
}
