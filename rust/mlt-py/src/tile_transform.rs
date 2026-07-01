use std::f64::consts::PI;

use pyo3::PyErr;
use pyo3::exceptions::PyValueError;

/// Affine transform from tile-local coords to EPSG:3857 meters.
#[derive(Clone, Copy)]
pub struct TileTransform {
    pub x_origin: f64,
    pub y_origin: f64,
    pub x_scale: f64,
    pub y_scale: f64,
}

impl TileTransform {
    /// Build a transform from tile z/x/y coordinates.
    ///
    /// `tms`: if true, y uses TMS convention (y=0 at south, used by OpenMapTiles
    /// and MBTiles). If false, y uses XYZ / slippy-map convention (y=0 at north,
    /// used by OSM tile servers).
    pub fn from_zxy(z: u32, x: u32, y: u32, extent: u32, tms: bool) -> Result<Self, PyErr> {
        if z > 30 {
            return Err(PyValueError::new_err(format!(
                "zoom level {z} exceeds maximum of 30"
            )));
        }

        let n = f64::from(1_u32 << z);
        let circumference = 2.0 * PI * 6_378_137.0;
        let tile_size = circumference / n;
        let half = circumference / 2.0;

        // Convert TMS y to XYZ y if needed (y_xyz = 2^z - 1 - y_tms)
        let y_xyz = if tms {
            (1_u32 << z).saturating_sub(1).saturating_sub(y)
        } else {
            y
        };

        // In XYZ convention: y=0 is the north edge of the map.
        // The tile's north (top) edge in EPSG:3857 meters:
        let x_origin = f64::from(x) * tile_size - half;
        let y_origin = half - f64::from(y_xyz) * tile_size;

        let scale = tile_size / f64::from(extent);

        Ok(TileTransform {
            x_origin,
            y_origin,
            x_scale: scale,
            y_scale: -scale, // tile pixel-y grows downward, EPSG:3857 y grows upward
        })
    }

    pub fn apply(self, coord: [i32; 2]) -> [f64; 2] {
        [
            self.x_origin + f64::from(coord[0]) * self.x_scale,
            self.y_origin + f64::from(coord[1]) * self.y_scale,
        ]
    }
}
