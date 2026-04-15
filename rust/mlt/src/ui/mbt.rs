//! `MBTiles` map viewer state and tile loading.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use geo_types::Polygon;
use martin_tile_utils::{decode_gzip, decode_zstd};
use mbtiles::Mbtiles;
use mlt_core::geojson::FeatureCollection;
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::{Coord32, Geom32};
use rstar::{AABB, PointDistance, RTree, RTreeObject};

use super::group_by_layer;
use super::state::LayerGroup;

type DecodedTile = (FeatureCollection, u32, Vec<LayerGroup>, RTree<MbtGeoEntry>);
type BestHover = Option<(f64, (u8, u32, u32), usize, usize)>;

// ---------------------------------------------------------------------------
// Tile loading channel types
// ---------------------------------------------------------------------------

pub(crate) enum TileLoadRequest {
    Load { z: u8, x: u32, y: u32 },
}

pub(crate) struct TileLoadResult {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub data: Result<Option<Vec<u8>>, String>,
}

// ---------------------------------------------------------------------------
// Geometry index entry (world coordinates)
// ---------------------------------------------------------------------------

/// Feature entry stored in the per-tile R-tree, using world coordinates.
/// World coordinate space: x ∈ [0,1] west→east, y ∈ [0,1] north→south.
pub(crate) struct MbtGeoEntry {
    pub layer: usize,
    pub feat: usize,
    pub vertices: Vec<[f64; 2]>,
}

impl RTreeObject for MbtGeoEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        if self.vertices.is_empty() {
            return AABB::from_point([0.0, 0.0]);
        }
        let (ax, ay, bx, by) = self.vertices.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(ax, ay, bx, by), v| (ax.min(v[0]), ay.min(v[1]), bx.max(v[0]), by.max(v[1])),
        );
        AABB::from_corners([ax, ay], [bx, by])
    }
}

impl PointDistance for MbtGeoEntry {
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

// ---------------------------------------------------------------------------
// Tile coordinate transform (tile-local → world)
// ---------------------------------------------------------------------------

/// Transforms coordinates from tile-local space ([0, extent]) to world space ([0, 1]).
pub(crate) struct TileTransform {
    n: f64,   // 2^z
    tx: f64,  // tile x index
    ty: f64,  // tile y index
    ext: f64, // tile extent
}

impl TileTransform {
    pub(crate) fn new(z: u8, tile_x: u32, tile_y: u32, extent: u32) -> Self {
        Self {
            n: f64::from(1u32 << z),
            tx: f64::from(tile_x),
            ty: f64::from(tile_y),
            ext: f64::from(extent),
        }
    }

    /// Convert a tile-local coordinate to world coordinates.
    #[inline]
    pub(crate) fn to_world(&self, c: Coord32) -> [f64; 2] {
        [
            (self.tx + f64::from(c.x) / self.ext) / self.n,
            (self.ty + f64::from(c.y) / self.ext) / self.n,
        ]
    }

    /// Collect world-coordinate vertices from a polygon.
    pub(crate) fn poly_verts(&self, poly: &Polygon<i32>) -> Vec<[f64; 2]> {
        poly.exterior()
            .0
            .iter()
            .copied()
            .chain(poly.interiors().iter().flat_map(|r| r.0.iter().copied()))
            .map(|c| self.to_world(c))
            .collect()
    }

    /// Collect world-coordinate vertices from any geometry.
    pub(crate) fn geom_verts(&self, geom: &Geom32) -> Vec<[f64; 2]> {
        match geom {
            Geom32::Point(p) => vec![self.to_world(p.0)],
            Geom32::LineString(ls) => ls.0.iter().copied().map(|c| self.to_world(c)).collect(),
            Geom32::MultiPoint(mp) => mp.iter().map(|p| self.to_world(p.0)).collect(),
            Geom32::Polygon(poly) => self.poly_verts(poly),
            Geom32::MultiLineString(mls) => mls
                .iter()
                .flat_map(|ls| ls.0.iter().copied().map(|c| self.to_world(c)))
                .collect(),
            Geom32::MultiPolygon(mpoly) => mpoly.iter().flat_map(|p| self.poly_verts(p)).collect(),
            _ => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Tile cache entry
// ---------------------------------------------------------------------------

pub(crate) enum MbtTileData {
    Loading,
    Empty,
    #[allow(dead_code)]
    Error(String),
    Loaded {
        fc: FeatureCollection,
        extent: u32,
        layer_groups: Vec<LayerGroup>,
        geo_index: RTree<MbtGeoEntry>,
    },
}

// ---------------------------------------------------------------------------
// Hover state
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct MbtHoveredInfo {
    pub tile: (u8, u32, u32),
    pub layer_idx: usize,
    pub feat_idx: usize,
}

// ---------------------------------------------------------------------------
// MbtilesState
// ---------------------------------------------------------------------------

pub(crate) struct MbtilesState {
    #[allow(dead_code)]
    pub path: PathBuf,
    /// Viewport bounds in world coords: x ∈ [0,1] west→east, y ∈ [0,1] north→south.
    pub vp_x0: f64,
    pub vp_x1: f64,
    pub vp_y0: f64,
    pub vp_y1: f64,
    /// Cached tile data keyed by (z, x, y) in XYZ scheme.
    pub tiles: HashMap<(u8, u32, u32), MbtTileData>,
    /// Currently hovered feature.
    pub hovered: Option<MbtHoveredInfo>,
    /// Nominal zoom level (0 = full world width); changes by ±0.5 per scroll step.
    /// Kept in sync with viewport width after pan (`sync_zoom_f_from_vp`).
    pub zoom_f: f64,
    /// Last mouse cell during left-button map drag (`Drag` deltas are applied from here).
    pub map_drag_last: Option<(u16, u16)>,
    loading: HashSet<(u8, u32, u32)>,
    request_tx: mpsc::SyncSender<TileLoadRequest>,
    pub result_rx: mpsc::Receiver<TileLoadResult>,
    /// Until the loader thread reports success, holds the one-shot init handshake receiver.
    loader_init_rx: Option<mpsc::Receiver<Result<(), String>>>,
    /// Fatal error from `MBTiles` open / connection (surfaced to the UI once via `take_loader_fatal`).
    loader_fatal: Option<String>,
}

impl MbtilesState {
    pub(crate) fn new(path: PathBuf) -> Self {
        let (req_tx, req_rx) = mpsc::sync_channel::<TileLoadRequest>(200);
        let (res_tx, res_rx) = mpsc::sync_channel::<TileLoadResult>(200);
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<(), String>>(1);

        let path_clone = path.clone();
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("tokio runtime");
            rt.block_on(async move {
                let mbt = match Mbtiles::new(&path_clone) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Failed to open mbtiles: {e}")));
                        return;
                    }
                };
                let mut conn = match mbt.open_readonly().await {
                    Ok(c) => c,
                    Err(e) => {
                        let _ =
                            init_tx.send(Err(format!("MBTiles read-only connection failed: {e}")));
                        return;
                    }
                };
                if init_tx.send(Ok(())).is_err() {
                    return;
                }
                drop(init_tx);
                while let Ok(TileLoadRequest::Load { z, x, y }) = req_rx.recv() {
                    let result = mbt
                        .get_tile(&mut conn, z, x, y)
                        .await
                        .map_err(|e| e.to_string());
                    let _ = res_tx.send(TileLoadResult {
                        z,
                        x,
                        y,
                        data: result,
                    });
                }
            });
        });

        Self {
            path,
            vp_x0: 0.0,
            vp_x1: 1.0,
            vp_y0: 0.0,
            vp_y1: 1.0,
            tiles: HashMap::new(),
            hovered: None,
            zoom_f: 0.0,
            map_drag_last: None,
            loading: HashSet::new(),
            request_tx: req_tx,
            result_rx: res_rx,
            loader_init_rx: Some(init_rx),
            loader_fatal: None,
        }
    }

    pub(crate) fn take_loader_fatal(&mut self) -> Option<String> {
        self.loader_fatal.take()
    }

    /// Tile zoom used for loading tiles: floor of `-log2(viewport width)`.
    pub(crate) fn zoom_level(&self) -> u8 {
        let vp_w = self.vp_x1 - self.vp_x0;
        if vp_w <= 0.0 {
            return 0;
        }
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let z = (-(vp_w.log2())).floor().clamp(0.0, 22.0) as u8;
        z
    }

    /// `(z, 2^z as f64, 2^z)` for the current view tile grid.
    fn view_tile_scale(&self) -> (u8, f64, u32) {
        let z = self.zoom_level();
        let n = 1u32 << z;
        (z, f64::from(n), n)
    }

    /// XYZ tile indices at zoom `z` containing world point `(wx, wy)` (XYZ, clamped).
    pub(crate) fn world_to_tile_xy(z: u8, wx: f64, wy: f64) -> (u32, u32) {
        let n = 1u32 << z;
        let nf = f64::from(n);
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let tx = (wx * nf).max(0.0).floor() as u32;
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let ty = (wy * nf).max(0.0).floor() as u32;
        let tx = tx.min(n.saturating_sub(1));
        let ty = ty.min(n.saturating_sub(1));
        (tx, ty)
    }

    /// Map-area fractions `rx, ry` (0 = left/top of map widget) to world coordinates.
    pub(crate) fn viewport_world_at_fracs(&self, rx: f64, ry: f64) -> (f64, f64) {
        let wx = self.vp_x0 + rx * (self.vp_x1 - self.vp_x0);
        let wy = self.vp_y0 + ry * (self.vp_y1 - self.vp_y0);
        (wx, wy)
    }

    fn sync_zoom_f_from_vp(&mut self) {
        let vp_w = self.vp_x1 - self.vp_x0;
        if vp_w > 0.0 {
            self.zoom_f = (-vp_w.log2()).clamp(0.0, 22.0);
        }
    }

    /// XYZ tile indices visible in the current viewport at the current zoom.
    pub(crate) fn visible_tiles(&self) -> Vec<(u8, u32, u32)> {
        let (z, nf, n) = self.view_tile_scale();
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let x_min = (self.vp_x0 * nf).max(0.0).floor() as u32;
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let x_max = ((self.vp_x1 * nf).ceil() as u32)
            .saturating_sub(1)
            .min(n.saturating_sub(1));
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let y_min = (self.vp_y0 * nf).max(0.0).floor() as u32;
        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let y_max = ((self.vp_y1 * nf).ceil() as u32)
            .saturating_sub(1)
            .min(n.saturating_sub(1));
        let mut tiles = Vec::new();
        for ty in y_min..=y_max {
            for tx in x_min..=x_max {
                tiles.push((z, tx, ty));
            }
        }
        tiles
    }

    /// XYZ tile (at current tile zoom) that contains the viewport center.
    pub(crate) fn center_tile_xyz(&self) -> (u8, u32, u32) {
        let z = self.zoom_level();
        let wx = f64::midpoint(self.vp_x0, self.vp_x1);
        let wy = f64::midpoint(self.vp_y0, self.vp_y1);
        let (tx, ty) = Self::world_to_tile_xy(z, wx, wy);
        (z, tx, ty)
    }

    /// Fit the viewport exactly to XYZ tile `(z, tx, ty)` and sync `zoom_f`.
    pub(crate) fn set_viewport_to_tile(&mut self, z: u8, tx: u32, ty: u32) -> Result<(), String> {
        let n = 1u32
            .checked_shl(u32::from(z))
            .ok_or_else(|| format!("zoom z={z} is too large"))?;
        if tx >= n || ty >= n {
            return Err(format!(
                "tile index out of range for z={z}: x={tx} y={ty} (max {})",
                n.saturating_sub(1)
            ));
        }
        let nf = f64::from(n);
        self.vp_x0 = f64::from(tx) / nf;
        self.vp_x1 = f64::from(tx + 1) / nf;
        self.vp_y0 = f64::from(ty) / nf;
        self.vp_y1 = f64::from(ty + 1) / nf;
        self.sync_zoom_f_from_vp();
        Ok(())
    }

    /// Enqueue a tile for loading if not already cached or loading.
    pub(crate) fn request_tile(&mut self, z: u8, x: u32, y: u32) {
        let key = (z, x, y);
        if !self.tiles.contains_key(&key) && !self.loading.contains(&key) {
            self.loading.insert(key);
            self.tiles.insert(key, MbtTileData::Loading);
            if self
                .request_tx
                .try_send(TileLoadRequest::Load { z, x, y })
                .is_err()
            {
                self.loading.remove(&key);
                self.tiles.insert(
                    key,
                    MbtTileData::Error(
                        "Could not enqueue tile load (channel full or loader stopped)".into(),
                    ),
                );
            }
        }
    }

    /// Request this tile and every XYZ ancestor down to z=0 (for overzoom fallbacks).
    pub(crate) fn request_tile_with_ancestors(&mut self, z: u8, x: u32, y: u32) {
        let mut cz = z;
        let mut cx = x;
        let mut cy = y;
        loop {
            self.request_tile(cz, cx, cy);
            if cz == 0 {
                break;
            }
            cz -= 1;
            cx >>= 1;
            cy >>= 1;
        }
    }

    /// First loaded ancestor for overzoom (`k` levels up: tile `(x>>k, y>>k)` at `z-k`), if any.
    pub(crate) fn find_overzoom_source(&self, z: u8, tx: u32, ty: u32) -> Option<(u8, u32, u32)> {
        for k in 1..=z {
            let pz = z - k;
            let shift = u32::from(k);
            let px = tx >> shift;
            let py = ty >> shift;
            let key = (pz, px, py);
            if matches!(self.tiles.get(&key), Some(MbtTileData::Loaded { .. })) {
                return Some(key);
            }
        }
        None
    }

    /// Tile to use for data at `(z,tx,ty)`: native if loaded, otherwise first loaded ancestor.
    pub(crate) fn effective_tile_key(&self, z: u8, tx: u32, ty: u32) -> Option<(u8, u32, u32)> {
        let key = (z, tx, ty);
        match self.tiles.get(&key) {
            Some(MbtTileData::Loaded { .. }) => Some(key),
            _ => self.find_overzoom_source(z, tx, ty),
        }
    }

    /// Drain incoming results and update the tile cache. Returns true if any tiles changed.
    pub(crate) fn process_results(&mut self) -> bool {
        let mut changed = false;

        if let Some(rx) = self.loader_init_rx.take() {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    // Init succeeded; drop the receiver so we do not treat later disconnect as init failure.
                }
                Ok(Err(msg)) => {
                    self.loader_fatal = Some(msg);
                    changed = true;
                }
                Err(TryRecvError::Empty) => {
                    self.loader_init_rx = Some(rx);
                }
                Err(TryRecvError::Disconnected) => {
                    if self.loader_fatal.is_none() {
                        self.loader_fatal =
                            Some("MBTiles loader thread exited during initialization".into());
                    }
                    changed = true;
                }
            }
        }

        while let Ok(res) = self.result_rx.try_recv() {
            let key = (res.z, res.x, res.y);
            self.loading.remove(&key);
            let entry = match res.data {
                Err(e) => MbtTileData::Error(e),
                Ok(None) => MbtTileData::Empty,
                Ok(Some(raw)) => match decode_and_parse(res.z, res.x, res.y, raw) {
                    Ok(Some((fc, extent, layer_groups, geo_index))) => MbtTileData::Loaded {
                        fc,
                        extent,
                        layer_groups,
                        geo_index,
                    },
                    Ok(None) => MbtTileData::Empty,
                    Err(e) => MbtTileData::Error(e.to_string()),
                },
            };
            self.tiles.insert(key, entry);
            changed = true;
        }
        changed
    }

    /// Zoom in/out by half a zoom level around `(wx, wy)` (scroll wheel).
    pub(crate) fn zoom_wheel_at(&mut self, wx: f64, wy: f64, zoom_in: bool) {
        let dz = if zoom_in { 0.5 } else { -0.5 };
        let new_z = (self.zoom_f + dz).clamp(0.0, 22.0);
        if (new_z - self.zoom_f).abs() < 1e-12 {
            return;
        }
        let scale = 2_f64.powf(-(new_z - self.zoom_f));
        if !self.zoom_viewport_at(wx, wy, scale) {
            return;
        }
        self.sync_zoom_f_from_vp();
    }

    /// Scale the viewport around `(wx, wy)` by `scale` on both axes (`scale` \< 1 zooms in).
    fn zoom_viewport_at(&mut self, wx: f64, wy: f64, scale: f64) -> bool {
        let x0 = wx + (self.vp_x0 - wx) * scale;
        let x1 = wx + (self.vp_x1 - wx) * scale;
        let y0 = wy + (self.vp_y0 - wy) * scale;
        let y1 = wy + (self.vp_y1 - wy) * scale;
        let min_size = 2_f64.powf(-22.0);
        if x1 - x0 < min_size || y1 - y0 < min_size {
            return false;
        }
        self.vp_x0 = x0;
        self.vp_x1 = x1;
        self.vp_y0 = y0;
        self.vp_y1 = y1;
        true
    }

    /// Pan the viewport by mouse delta in terminal cells (`d_col`/`d_row` = current − previous).
    pub(crate) fn pan_by_pixels(&mut self, area_w: u16, area_h: u16, d_col: i32, d_row: i32) {
        if area_w == 0 || area_h == 0 {
            return;
        }
        let wf = f64::from(area_w);
        let hf = f64::from(area_h);
        let vp_w = self.vp_x1 - self.vp_x0;
        let vp_h = self.vp_y1 - self.vp_y0;
        let dx = f64::from(d_col) / wf * vp_w;
        let dy = f64::from(d_row) / hf * vp_h;
        self.vp_x0 -= dx;
        self.vp_x1 -= dx;
        self.vp_y0 -= dy;
        self.vp_y1 -= dy;
        self.sync_zoom_f_from_vp();
    }

    /// Find the nearest feature to world point (wx, wy) among visible cells (native or overzoom).
    pub(crate) fn find_hovered(&mut self, wx: f64, wy: f64) {
        let z = self.zoom_level();
        let threshold = (self.vp_x1 - self.vp_x0) * 0.02;
        let thresh_sq = threshold * threshold;
        let pt = [wx, wy];

        let mut best: BestHover = None;
        let mut seen_src: HashSet<(u8, u32, u32)> = HashSet::new();
        let visible = self.visible_tiles();
        for (tz, tx, ty) in visible {
            if tz != z {
                continue;
            }
            let Some(src_key) = self.effective_tile_key(tz, tx, ty) else {
                continue;
            };
            if !seen_src.insert(src_key) {
                continue;
            }
            let Some(MbtTileData::Loaded { geo_index, .. }) = self.tiles.get(&src_key) else {
                continue;
            };
            for e in geo_index.nearest_neighbor_iter(&pt) {
                let d = e.distance_2(&pt);
                if d > thresh_sq {
                    break;
                }
                if best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, src_key, e.layer, e.feat));
                }
            }
        }
        let new_hov = best.map(|(_, tile, layer, feat)| MbtHoveredInfo {
            tile,
            layer_idx: layer,
            feat_idx: feat,
        });
        self.hovered = new_hov;
    }

    /// Keys we keep when pruning: visible tiles (with a 1-tile margin) plus every ancestor chain.
    fn keep_tile_keys(&self) -> HashSet<(u8, u32, u32)> {
        let z = self.zoom_level();
        let Some(n) = 1u32.checked_shl(u32::from(z)) else {
            return HashSet::new();
        };
        let mut out = HashSet::new();
        for (_zv, tx, ty) in self.visible_tiles() {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let nx = (i64::from(tx) + i64::from(dx))
                        .clamp(0, i64::from(n.saturating_sub(1)))
                        as u32;
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let ny = (i64::from(ty) + i64::from(dy))
                        .clamp(0, i64::from(n.saturating_sub(1)))
                        as u32;
                    let mut cz = z;
                    let mut cx = nx;
                    let mut cy = ny;
                    loop {
                        out.insert((cz, cx, cy));
                        if cz == 0 {
                            break;
                        }
                        cz -= 1;
                        cx >>= 1;
                        cy >>= 1;
                    }
                }
            }
        }
        out
    }

    /// Drop cached tiles far from the viewport so panning does not grow memory without bound.
    pub(crate) fn prune_tile_cache_if_needed(&mut self) {
        const MAX: usize = 256;
        if self.tiles.len() <= MAX {
            return;
        }
        let keep = self.keep_tile_keys();
        let stale: Vec<(u8, u32, u32)> = self
            .tiles
            .keys()
            .filter(|k| !keep.contains(k))
            .copied()
            .collect();
        let stale_set: HashSet<_> = stale.iter().copied().collect();
        if self
            .hovered
            .as_ref()
            .is_some_and(|h| stale_set.contains(&h.tile))
        {
            self.hovered = None;
        }
        for k in stale {
            self.loading.remove(&k);
            self.tiles.remove(&k);
        }
    }
}

// ---------------------------------------------------------------------------
// Tile decode & parse helpers
// ---------------------------------------------------------------------------

fn decode_and_parse(z: u8, tx: u32, ty: u32, raw: Vec<u8>) -> anyhow::Result<Option<DecodedTile>> {
    let buf = decompress(raw)?;
    if buf.is_empty() {
        return Ok(None);
    }
    let fc = mvt_to_feature_collection(buf)?;
    if fc.features.is_empty() {
        return Ok(None);
    }
    let extent = fc
        .features
        .first()
        .and_then(|f| f.properties.get("_extent"))
        .and_then(serde_json::Value::as_u64)
        .map_or(4096, |v| u32::try_from(v).unwrap_or(4096));
    let layer_groups = group_by_layer(&fc);
    let geo_index = build_world_geo_index(z, tx, ty, &fc, &layer_groups, extent);
    Ok(Some((fc, extent, layer_groups, geo_index)))
}

fn decompress(raw: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    if raw.len() >= 2 && raw[0] == 0x1f && raw[1] == 0x8b {
        Ok(decode_gzip(&raw)?)
    } else if raw.len() >= 4 && raw[0] == 0x28 && raw[1] == 0xb5 && raw[2] == 0x2f && raw[3] == 0xfd
    {
        Ok(decode_zstd(&raw)?)
    } else {
        Ok(raw)
    }
}

fn build_world_geo_index(
    z: u8,
    tx: u32,
    ty: u32,
    fc: &FeatureCollection,
    layer_groups: &[LayerGroup],
    extent: u32,
) -> RTree<MbtGeoEntry> {
    let transform = TileTransform::new(z, tx, ty, extent);
    let mut entries = Vec::new();
    for (li, group) in layer_groups.iter().enumerate() {
        for (fi, &gi) in group.feature_indices.iter().enumerate() {
            let geom = &fc.features[gi].geometry;
            let vertices = transform.geom_verts(geom);
            if !vertices.is_empty() {
                entries.push(MbtGeoEntry {
                    layer: li,
                    feat: fi,
                    vertices,
                });
            }
        }
    }
    RTree::bulk_load(entries)
}
