//! The city model that the `CityGML` reader produces and the tiler consumes.

use std::collections::HashSet;

use anyhow::Result as AnyResult;

use super::crs::Reprojector;
use super::lods;
use super::triangulate::{Triangulator, dot};

/// The up component of a unit normal above which a surface without semantics is a roof.
const ROOF_MIN_UP: f64 = 0.34;

/// The part of a building a triangle belongs to, its `part` m-value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Wall,
    Roof,
    Ground,
    Window,
    Door,
}

impl Part {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wall => "wall",
            Self::Roof => "roof",
            Self::Ground => "ground",
            Self::Window => "window",
            Self::Door => "door",
        }
    }
}

/// What a semantic surface type stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Semantic {
    Seen(Part),
    /// A surface not seen from outside, such as a closure surface or an interior wall.
    Hidden,
}

impl Semantic {
    /// The semantic of a `CityGML` element, `None` if it names no part.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        Some(match name {
            "WallSurface" | "OuterCeilingSurface" => Self::Seen(Part::Wall),
            "RoofSurface" | "OuterFloorSurface" => Self::Seen(Part::Roof),
            "GroundSurface" => Self::Seen(Part::Ground),
            "Window" | "WindowSurface" => Self::Seen(Part::Window),
            "Door" | "DoorSurface" => Self::Seen(Part::Door),
            "ClosureSurface" | "InteriorWallSurface" | "CeilingSurface" | "FloorSurface" => {
                Self::Hidden
            }
            _ => return None,
        })
    }
}

/// What a city object is to the building it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    /// A building, bridge or tunnel, each of which becomes one feature.
    Root,
    /// A part that picks its own highest LOD, such as a building part.
    Part,
    /// An installation, which picks its own highest LOD and is left out of LOD2.
    Installation,
}

impl ObjectKind {
    /// The kind of a `CityGML` element, `None` if it is no part of a building, bridge or tunnel.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        Some(match name {
            "Building" | "Bridge" | "Tunnel" => Self::Root,
            "BuildingPart"
            | "BuildingConstructiveElement"
            | "BridgePart"
            | "BridgeConstructionElement"
            | "BridgeConstructiveElement"
            | "TunnelPart"
            | "TunnelConstructiveElement" => Self::Part,
            "BuildingInstallation" | "BridgeInstallation" | "TunnelInstallation" => {
                Self::Installation
            }
            _ => return None,
        })
    }
}

/// The meshes of a building a polygon is in: that of the highest LOD of each part, that of LOD2, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lods {
    Both,
    Highest,
    Lod2,
}

impl Lods {
    /// Whether a polygon in these meshes is in the mesh of `lod`.
    #[must_use]
    pub fn includes(self, lod: Self) -> bool {
        self == Self::Both || self == lod
    }
}

/// A polygon in source coordinates: its rings, the exterior first, none repeating its first point at its end.
#[derive(Debug, PartialEq)]
pub struct SourcePolygon {
    /// The semantic surface the polygon lies in, if any.
    pub part: Option<Part>,
    pub lods: Lods,
    pub points: Vec<[f64; 3]>,
    /// Where each ring ends in `points`.
    pub ends: Vec<usize>,
}

/// The rings of a polygon, gathered one at a time.
#[derive(Debug, Default)]
pub struct Rings {
    points: Vec<[f64; 3]>,
    ends: Vec<usize>,
    /// Whether the exterior ring has fewer than three distinct points.
    broken: bool,
}

impl Rings {
    /// Adds `ring` without its repeated points, reversed with `reverse`.
    /// Only the first exterior ring and the holes after it count.
    /// An exterior of fewer than three points breaks the polygon, and such a hole is left out.
    pub fn push(
        &mut self,
        ring: impl IntoIterator<Item = [f64; 3]>,
        interior: bool,
        reverse: bool,
    ) {
        let start = self.points.len();
        for p in ring {
            if self.points.len() == start || self.points.last() != Some(&p) {
                self.points.push(p);
            }
        }
        if self.points.len() > start + 1 && self.points.last() == self.points.get(start) {
            self.points.pop();
        }
        if self.points.len() < start + 3 {
            self.points.truncate(start);
            self.broken |= !interior;
            return;
        }
        if interior == self.ends.is_empty() {
            self.points.truncate(start);
            return;
        }
        if reverse {
            self.points[start..].reverse();
        }
        self.ends.push(self.points.len());
    }

    /// The polygon of these rings in `part`, or `None` if its exterior is missing or broken.
    #[must_use]
    pub fn finish(self, part: Option<Part>) -> Option<SourcePolygon> {
        (!self.broken && !self.ends.is_empty()).then_some(SourcePolygon {
            part,
            lods: Lods::Both,
            points: self.points,
            ends: self.ends,
        })
    }
}

/// A polygon of a city object, with the part of the object that owns it and its LOD.
#[derive(Debug, PartialEq)]
pub struct OwnedPolygon {
    /// The object itself is owner 0, and each of its building parts, installations and the like has an owner of its own.
    pub owner: u32,
    pub lod: u8,
    pub polygon: SourcePolygon,
}

/// A building, bridge or tunnel as read, in source coordinates.
#[derive(Debug, PartialEq)]
pub struct SourceObject {
    pub id: Option<String>,
    pub polygons: Vec<OwnedPolygon>,
    /// The owners that are installations.
    pub installations: HashSet<u32>,
    /// The points of its `lodNTerrainIntersection` curves, where it meets the ground.
    pub terrain_points: Vec<[f64; 3]>,
}

/// What a file holds.
#[derive(Debug, Default, PartialEq)]
pub struct SourceDocument {
    /// The CRS the file names, such as `EPSG:25832` or `urn:ogc:def:crs:EPSG::25832`.
    pub crs: Option<String>,
    pub city_objects: Vec<SourceObject>,
}

impl SourceDocument {
    /// The first position of the file, if it holds one.
    #[must_use]
    pub fn first(&self) -> Option<[f64; 3]> {
        self.city_objects
            .iter()
            .flat_map(|o| &o.polygons)
            .find_map(|p| p.polygon.points.first().copied())
    }
}

/// A surface of one part, with geodetic positions: longitude and latitude in radians, and metres up.
pub struct Surface {
    pub part: Part,
    pub lods: Lods,
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// A building with triangulated geodetic surfaces.
pub struct Building {
    pub id: Option<String>,
    pub surfaces: Vec<Surface>,
    pub terrain: Vec<[f64; 3]>,
}

impl Building {
    /// Picks the polygons of `object` that the meshes hold, then triangulates and reprojects them with `reprojector`.
    /// A polygon without semantics is a roof, wall or ground by the way it faces.
    /// `None` if no polygon has an area.
    pub fn new(
        object: SourceObject,
        reprojector: &Reprojector,
        triangulator: &mut Triangulator,
    ) -> AnyResult<Option<Self>> {
        let polygons = lods::resolve(object.polygons, &object.installations);
        let mut surfaces = Vec::with_capacity(polygons.len());
        for polygon in polygons {
            let mut triangles = Vec::new();
            let Some(normal) = triangulator.polygon(
                &polygon.points,
                &polygon.ends,
                reprojector.geographic,
                &mut triangles,
            ) else {
                continue;
            };
            let part = polygon.part.unwrap_or_else(|| {
                let up = normal[2] / dot(normal, normal).sqrt();
                if up > ROOF_MIN_UP {
                    Part::Roof
                } else if up < -ROOF_MIN_UP {
                    Part::Ground
                } else {
                    Part::Wall
                }
            });
            let mut positions = polygon.points;
            reprojector.to_geodetic(&mut positions)?;
            surfaces.push(Surface {
                part,
                lods: polygon.lods,
                positions,
                triangles,
            });
        }
        if surfaces.is_empty() {
            return Ok(None);
        }
        let mut terrain = object.terrain_points;
        if !terrain.is_empty() {
            reprojector.to_geodetic(&mut terrain)?;
        }
        Ok(Some(Self {
            id: object.id,
            surfaces,
            terrain,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_drop_repeated_points_and_the_closing_point() {
        let mut rings = Rings::default();
        rings.push(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            false,
            false,
        );
        let polygon = rings.finish(Some(Part::Roof)).unwrap();
        assert_eq!(
            polygon.points,
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
        );
        assert_eq!(polygon.ends, [3]);
        assert_eq!(polygon.part, Some(Part::Roof));
    }

    #[test]
    fn rings_reverse_on_request_and_leave_out_holes_before_the_exterior() {
        let mut rings = Rings::default();
        rings.push(
            [[5.0, 5.0, 0.0], [6.0, 5.0, 0.0], [6.0, 6.0, 0.0]],
            true,
            false,
        );
        rings.push(
            [[0.0, 0.0, 0.0], [9.0, 0.0, 0.0], [9.0, 9.0, 0.0]],
            false,
            true,
        );
        rings.push(
            [[1.0, 1.0, 0.0], [2.0, 1.0, 0.0], [2.0, 2.0, 0.0]],
            true,
            false,
        );
        rings.push(
            [[7.0, 7.0, 0.0], [8.0, 7.0, 0.0], [8.0, 8.0, 0.0]],
            false,
            false,
        );
        let polygon = rings.finish(None).unwrap();
        assert_eq!(
            polygon.points,
            [
                [9.0, 9.0, 0.0],
                [9.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 1.0, 0.0],
                [2.0, 2.0, 0.0],
            ]
        );
        assert_eq!(polygon.ends, [3, 6]);
    }

    #[test]
    fn rings_without_an_exterior_of_three_points_make_no_polygon_but_a_short_hole_is_skipped() {
        let mut short_exterior = Rings::default();
        short_exterior.push(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            false,
            false,
        );
        assert_eq!(short_exterior.finish(None), None);
        assert_eq!(Rings::default().finish(None), None);

        let mut short_hole = Rings::default();
        short_hole.push(
            [[0.0, 0.0, 0.0], [9.0, 0.0, 0.0], [9.0, 9.0, 0.0]],
            false,
            false,
        );
        short_hole.push([[1.0, 1.0, 0.0], [2.0, 1.0, 0.0]], true, false);
        assert_eq!(short_hole.finish(None).unwrap().ends, [3]);
    }
}
