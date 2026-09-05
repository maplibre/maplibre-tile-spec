//! Tile filtering for `--bbox`.

use anyhow::{Result as AnyResult, bail};
use martin_tile_utils::{MAX_ZOOM, TileRect, bbox_to_xyz};
use mbtiles::invert_y_value;
use tilejson::{Bounds, Center};

/// The tiles a `--bbox` conversion keeps, as one tile rectangle per requested box per zoom level.
pub struct BboxFilter {
    /// Rectangles of kept tiles, indexed by zoom level, one per requested box.
    rects: Vec<Vec<TileRect>>,
    /// Smallest box enclosing every requested box.
    bounds: Bounds,
}

impl BboxFilter {
    /// Builds the filter for the requested boxes, or `None` when none were requested.
    pub fn new(bboxes: &[Bounds]) -> AnyResult<Option<Self>> {
        let Some(bounds) = bboxes.iter().copied().reduce(|a, b| a + b) else {
            return Ok(None);
        };
        for bbox in bboxes {
            validate(*bbox)?;
        }
        let rects = (0..=MAX_ZOOM)
            .map(|zoom| bboxes.iter().map(|b| rect_at_zoom(*b, zoom)).collect())
            .collect();
        Ok(Some(Self { rects, bounds }))
    }

    /// Smallest box enclosing every requested box.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Whether the tile at these XYZ coordinates overlaps a requested box.
    #[must_use]
    pub fn keeps(&self, z: u8, x: u32, y: u32) -> bool {
        // PMTiles reaches one zoom level deeper than the tile grid helpers, so a
        // tile past MAX_ZOOM is tested through its deepest addressable ancestor.
        let shift = z.saturating_sub(MAX_ZOOM);
        let zoom = z - shift;
        let (x, y) = (x >> shift, y >> shift);
        let probe = TileRect::new(zoom, x, y, x, y);
        self.rects[usize::from(zoom)]
            .iter()
            .any(|rect| rect.is_overlapping(&probe))
    }

    /// `WHERE` clause matching the kept tiles of an `MBTiles` `tiles` table.
    ///
    /// `MBTiles` rows count from the south, so every rectangle is flipped.
    #[must_use]
    pub fn mbtiles_where(&self) -> String {
        fn format_between(x: u32, y: u32) -> String {
            if x == y {
                format!("= {x}")
            } else {
                format!("BETWEEN {x} AND {y}")
            }
        }

        self.rects
            .iter()
            .flatten()
            .map(|rect| {
                format!(
                    "(zoom_level = {z} AND tile_column {between_x} AND tile_row {between_y})",
                    z = rect.zoom,
                    between_x = format_between(rect.min_x, rect.max_x),
                    between_y = format_between(
                        invert_y_value(rect.zoom, rect.max_y),
                        invert_y_value(rect.zoom, rect.min_y)
                    ),
                )
            })
            .collect::<Vec<_>>()
            .join("\n OR ")
    }
}

/// Every tile at `zoom` that overlaps `bbox`.
fn rect_at_zoom(bbox: Bounds, zoom: u8) -> TileRect {
    let (min_x, min_y, max_x, max_y) =
        bbox_to_xyz(bbox.left, bbox.bottom, bbox.right, bbox.top, zoom);
    TileRect::new(zoom, min_x, min_y, max_x, max_y)
}

/// Rejects a box that leaves WGS84 or has its corners swapped.
fn validate(bbox: Bounds) -> AnyResult<()> {
    if !(-180.0..=180.0).contains(&bbox.left) || !(-180.0..=180.0).contains(&bbox.right) {
        bail!("--bbox longitudes must be between -180 and 180, got {bbox}");
    }
    if !(-90.0..=90.0).contains(&bbox.bottom) || !(-90.0..=90.0).contains(&bbox.top) {
        bail!("--bbox latitudes must be between -90 and 90, got {bbox}");
    }
    if bbox.left > bbox.right || bbox.bottom > bbox.top {
        bail!("--bbox must be min_lon,min_lat,max_lon,max_lat, got {bbox}");
    }
    Ok(())
}

/// Intersects `bounds` with `clip`, keeping `clip` when the two are disjoint.
#[must_use]
pub fn clip_bounds(bounds: Bounds, clip: Bounds) -> Bounds {
    let clipped = Bounds::new(
        bounds.left.max(clip.left),
        bounds.bottom.max(clip.bottom),
        bounds.right.min(clip.right),
        bounds.top.min(clip.top),
    );
    if clipped.left > clipped.right || clipped.bottom > clipped.top {
        clip
    } else {
        clipped
    }
}

/// Pulls `center` into `bounds`, which a `--bbox` conversion may have moved away from it.
#[must_use]
pub fn clip_center(center: Center, bounds: Bounds) -> Center {
    Center::new(
        center.longitude.clamp(bounds.left, bounds.right),
        center.latitude.clamp(bounds.bottom, bounds.top),
        center.zoom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(bboxes: &[Bounds]) -> BboxFilter {
        BboxFilter::new(bboxes)
            .expect("boxes are valid")
            .expect("boxes are not empty")
    }

    #[test]
    fn no_box_means_no_filter() {
        assert!(BboxFilter::new(&[]).expect("no box is valid").is_none());
    }

    #[test]
    fn rejects_swapped_corners() {
        assert!(BboxFilter::new(&[Bounds::new(10.0, 1.0, 1.0, 10.0)]).is_err());
        assert!(BboxFilter::new(&[Bounds::new(1.0, 10.0, 10.0, 1.0)]).is_err());
    }

    #[test]
    fn rejects_boxes_outside_wgs84() {
        assert!(BboxFilter::new(&[Bounds::new(-181.0, 1.0, 1.0, 10.0)]).is_err());
        assert!(BboxFilter::new(&[Bounds::new(-1.0, 1.0, 1.0, 91.0)]).is_err());
    }

    #[test]
    fn keeps_tiles_overlapping_the_box() {
        let filter = filter(&[Bounds::new(1.0, 1.0, 10.0, 10.0)]);
        assert!(filter.keeps(0, 0, 0));
        assert!(filter.keeps(1, 1, 0));
        assert!(!filter.keeps(1, 0, 0));
        assert!(!filter.keeps(1, 1, 1));
    }

    #[test]
    fn keeps_tiles_overlapping_any_box() {
        let filter = filter(&[
            Bounds::new(1.0, 1.0, 10.0, 10.0),
            Bounds::new(-10.0, -10.0, -1.0, -1.0),
        ]);
        assert!(filter.keeps(1, 1, 0));
        assert!(filter.keeps(1, 0, 1));
        assert!(!filter.keeps(1, 0, 0));
        assert!(!filter.keeps(1, 1, 1));
    }

    #[test]
    fn keeps_the_deepest_pmtiles_zoom_through_its_ancestor() {
        let filter = filter(&[Bounds::new(0.0, 0.0, 10.0, 10.0)]);
        assert!(filter.keeps(31, 1 << 30, (1 << 30) - 1));
        assert!(!filter.keeps(31, 0, 0));
    }

    #[test]
    fn union_bounds_enclose_every_box() {
        let filter = filter(&[
            Bounds::new(1.0, 1.0, 10.0, 10.0),
            Bounds::new(-10.0, -10.0, -1.0, -1.0),
        ]);
        assert_eq!(filter.bounds(), Bounds::new(-10.0, -10.0, 10.0, 10.0));
    }

    #[test]
    fn mbtiles_where_flips_rows() {
        insta::assert_snapshot!(filter(&[Bounds::new(1.0, 1.0, 10.0, 10.0)]).mbtiles_where(), @"
        (zoom_level = 0 AND tile_column = 0 AND tile_row = 0)
         OR (zoom_level = 1 AND tile_column = 1 AND tile_row = 1)
         OR (zoom_level = 2 AND tile_column = 2 AND tile_row = 2)
         OR (zoom_level = 3 AND tile_column = 4 AND tile_row = 4)
         OR (zoom_level = 4 AND tile_column = 8 AND tile_row = 8)
         OR (zoom_level = 5 AND tile_column = 16 AND tile_row = 16)
         OR (zoom_level = 6 AND tile_column BETWEEN 32 AND 33 AND tile_row BETWEEN 32 AND 33)
         OR (zoom_level = 7 AND tile_column BETWEEN 64 AND 67 AND tile_row BETWEEN 64 AND 67)
         OR (zoom_level = 8 AND tile_column BETWEEN 128 AND 135 AND tile_row BETWEEN 128 AND 135)
         OR (zoom_level = 9 AND tile_column BETWEEN 257 AND 270 AND tile_row BETWEEN 257 AND 270)
         OR (zoom_level = 10 AND tile_column BETWEEN 514 AND 540 AND tile_row BETWEEN 514 AND 540)
         OR (zoom_level = 11 AND tile_column BETWEEN 1029 AND 1080 AND tile_row BETWEEN 1029 AND 1081)
         OR (zoom_level = 12 AND tile_column BETWEEN 2059 AND 2161 AND tile_row BETWEEN 2059 AND 2162)
         OR (zoom_level = 13 AND tile_column BETWEEN 4118 AND 4323 AND tile_row BETWEEN 4118 AND 4324)
         OR (zoom_level = 14 AND tile_column BETWEEN 8237 AND 8647 AND tile_row BETWEEN 8237 AND 8649)
         OR (zoom_level = 15 AND tile_column BETWEEN 16475 AND 17294 AND tile_row BETWEEN 16475 AND 17298)
         OR (zoom_level = 16 AND tile_column BETWEEN 32950 AND 34588 AND tile_row BETWEEN 32950 AND 34597)
         OR (zoom_level = 17 AND tile_column BETWEEN 65900 AND 69176 AND tile_row BETWEEN 65900 AND 69195)
         OR (zoom_level = 18 AND tile_column BETWEEN 131800 AND 138353 AND tile_row BETWEEN 131800 AND 138391)
         OR (zoom_level = 19 AND tile_column BETWEEN 263600 AND 276707 AND tile_row BETWEEN 263600 AND 276782)
         OR (zoom_level = 20 AND tile_column BETWEEN 527200 AND 553415 AND tile_row BETWEEN 527200 AND 553564)
         OR (zoom_level = 21 AND tile_column BETWEEN 1054401 AND 1106830 AND tile_row BETWEEN 1054401 AND 1107128)
         OR (zoom_level = 22 AND tile_column BETWEEN 2108802 AND 2213660 AND tile_row BETWEEN 2108803 AND 2214256)
         OR (zoom_level = 23 AND tile_column BETWEEN 4217605 AND 4427320 AND tile_row BETWEEN 4217606 AND 4428512)
         OR (zoom_level = 24 AND tile_column BETWEEN 8435211 AND 8854641 AND tile_row BETWEEN 8435213 AND 8857025)
         OR (zoom_level = 25 AND tile_column BETWEEN 16870422 AND 17709283 AND tile_row BETWEEN 16870427 AND 17714051)
         OR (zoom_level = 26 AND tile_column BETWEEN 33740845 AND 35418567 AND tile_row BETWEEN 33740854 AND 35428103)
         OR (zoom_level = 27 AND tile_column BETWEEN 67481691 AND 70837134 AND tile_row BETWEEN 67481709 AND 70856207)
         OR (zoom_level = 28 AND tile_column BETWEEN 134963382 AND 141674268 AND tile_row BETWEEN 134963419 AND 141712415)
         OR (zoom_level = 29 AND tile_column BETWEEN 269926764 AND 283348536 AND tile_row BETWEEN 269926839 AND 283424831)
         OR (zoom_level = 30 AND tile_column BETWEEN 539853528 AND 566697073 AND tile_row BETWEEN 539853679 AND 566849663)
        ");
    }

    #[test]
    fn clips_bounds_to_the_box() {
        assert_eq!(
            clip_bounds(Bounds::MAX, Bounds::new(1.0, 2.0, 3.0, 4.0)),
            Bounds::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            clip_bounds(
                Bounds::new(0.0, 0.0, 2.0, 2.0),
                Bounds::new(1.0, 1.0, 3.0, 3.0)
            ),
            Bounds::new(1.0, 1.0, 2.0, 2.0)
        );
    }

    #[test]
    fn clips_to_the_box_when_the_two_are_disjoint() {
        let clip = Bounds::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(clip_bounds(Bounds::new(0.0, 0.0, 1.0, 1.0), clip), clip);
    }

    #[test]
    fn pulls_the_center_into_the_bounds() {
        let bounds = Bounds::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(
            clip_center(Center::new(0.0, 0.0, 3), bounds),
            Center::new(10.0, 10.0, 3)
        );
        assert_eq!(
            clip_center(Center::new(15.0, 15.0, 3), bounds),
            Center::new(15.0, 15.0, 3)
        );
    }
}
