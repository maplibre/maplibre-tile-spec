//! One pass over a `CityGML` document by local names, so that `CityGML` 2.0 and 3.0 and any namespace prefixes read alike.

use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use anyhow::{Result as AnyResult, bail};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

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

/// A building, bridge or tunnel, with the polygons of the highest LOD of each of its parts and of its LOD2.
/// A part whose polygons all lie in semantic surfaces picks these LODs for its walls, roofs and grounds each.
/// The LOD2 of a part above LOD2 is its highest LOD of 2 or less, or else its highest LOD without windows, doors, installations and the holes they leave.
#[derive(Debug, PartialEq)]
pub struct CityObject {
    pub gml_id: Option<String>,
    pub polygons: Vec<SourcePolygon>,
    /// The points of its `lodNTerrainIntersection` curves, where it meets the ground.
    pub terrain_points: Vec<[f64; 3]>,
}

/// What a document holds.
#[derive(Debug, Default, PartialEq)]
pub struct Document {
    /// The first `srsName` of the document.
    pub srs_name: Option<String>,
    pub city_objects: Vec<CityObject>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    CityObject,
    /// A part of a city object that picks its own highest LOD, such as a building part or installation.
    Owner {
        installation: bool,
    },
    Semantic(Part),
    /// Content not seen from outside: closure surfaces, interiors, LOD0, LOD4, curves, appearances and implicit geometries.
    Skip,
    /// An `OrientableSurface` whose `orientation="-"` flips the winding of polygons inside it.
    OrientableSurface(bool),
    TerrainIntersection,
    /// An `lodN` geometry property.
    Lod(u8),
    Polygon,
    Interior,
    Ring,
    LineString,
    Coordinates,
    Other,
}

fn tag(name: &[u8]) -> Tag {
    match name {
        b"Building" | b"Bridge" | b"Tunnel" => Tag::CityObject,
        b"BuildingPart"
        | b"BuildingConstructiveElement"
        | b"BridgePart"
        | b"BridgeConstructionElement"
        | b"BridgeConstructiveElement"
        | b"TunnelPart"
        | b"TunnelConstructiveElement" => Tag::Owner {
            installation: false,
        },
        b"BuildingInstallation" | b"BridgeInstallation" | b"TunnelInstallation" => {
            Tag::Owner { installation: true }
        }
        b"WallSurface" | b"OuterCeilingSurface" => Tag::Semantic(Part::Wall),
        b"RoofSurface" | b"OuterFloorSurface" => Tag::Semantic(Part::Roof),
        b"GroundSurface" => Tag::Semantic(Part::Ground),
        b"Window" | b"WindowSurface" => Tag::Semantic(Part::Window),
        b"Door" | b"DoorSurface" => Tag::Semantic(Part::Door),
        b"ClosureSurface"
        | b"InteriorWallSurface"
        | b"CeilingSurface"
        | b"FloorSurface"
        | b"interiorRoom"
        | b"interiorBuildingInstallation"
        | b"interiorFurniture"
        | b"interiorBridgeRoom"
        | b"interiorBridgeInstallation"
        | b"interiorTunnelInstallation"
        | b"interiorHollowSpace"
        | b"buildingRoom"
        | b"buildingFurniture"
        | b"bridgeRoom"
        | b"bridgeFurniture"
        | b"hollowSpace"
        | b"tunnelFurniture"
        | b"appearance"
        | b"appearanceMember"
        | b"ImplicitGeometry" => Tag::Skip,
        b"OrientableSurface" => Tag::OrientableSurface(false),
        b"Polygon" | b"Triangle" | b"Rectangle" | b"PolygonPatch" => Tag::Polygon,
        b"interior" | b"innerBoundaryIs" => Tag::Interior,
        b"LinearRing" => Tag::Ring,
        b"LineString" => Tag::LineString,
        b"posList" | b"pos" | b"coordinates" => Tag::Coordinates,
        _ => lod_tag(name).unwrap_or(Tag::Other),
    }
}

/// The tag of an `lodN` property, or `None` if `name` names none.
fn lod_tag(name: &[u8]) -> Option<Tag> {
    let rest = name.strip_prefix(b"lod")?;
    let (&digit, kind) = rest.split_first()?;
    if !digit.is_ascii_digit() || !kind.first()?.is_ascii_uppercase() {
        return None;
    }
    if kind.starts_with(b"TerrainIntersection") && digit != b'0' && digit != b'4' {
        return Some(Tag::TerrainIntersection);
    }
    if matches!(digit, b'0' | b'4') || kind.ends_with(b"MultiCurve") {
        return Some(Tag::Skip);
    }
    Some(Tag::Lod(digit - b'0'))
}

/// The city objects of `reader`, read in one pass.
pub fn parse_reader<R: BufRead>(reader: R) -> AnyResult<Document> {
    let mut parser = DocumentParser::default();
    let mut xml = Reader::from_reader(reader);
    let mut buf = Vec::with_capacity(8192);
    loop {
        buf.clear();
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => parser.start(&e),
            Ok(Event::Empty(e)) => {
                parser.start(&e);
                parser.end();
            }
            Ok(Event::End(_)) => parser.end(),
            Ok(Event::Text(t)) if parser.in_coordinates => {
                if !parse_numbers(t.as_bytes(), &mut parser.numbers) {
                    parser.unreadable = true;
                }
            }
            Ok(Event::Eof) => return Ok(parser.document),
            Ok(_) => {}
            Err(e) => bail!("invalid XML at byte {}: {e}", xml.buffer_position()),
        }
    }
}

#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent flags of a single-pass parser"
)]
struct DocumentParser {
    stack: Vec<Tag>,
    /// Whether the parser is inside a skipped element.
    skip: bool,
    /// The enclosing `OrientableSurface` elements of `orientation="-"`, an odd count reversing rings.
    reverse_flips: u32,
    /// The enclosing `lodNTerrainIntersection` properties.
    terrain_depth: u32,
    /// The city object being read, its polygons collected in `polygons` until it ends.
    city_object: Option<CityObject>,
    polygons: Vec<OwnedPolygon>,
    owners: Vec<u32>,
    next_owner: u32,
    /// The owners of the city object that are installations.
    installations: HashSet<u32>,
    lods: Vec<u8>,
    /// The semantic surface of the innermost surface or owner, `None` for an owner.
    semantics: Vec<Option<Part>>,
    in_coordinates: bool,
    /// The `srsDimension` of the polygon being read, which its rings and positions may override.
    polygon_dimension: Option<String>,
    /// The `srsDimension` of the ring being read.
    dimension: Option<String>,
    /// Whether a coordinate of the ring is not a finite number.
    unreadable: bool,
    numbers: Vec<f64>,
    points: Vec<[f64; 3]>,
    ends: Vec<usize>,
    polygon_ok: bool,
    document: Document,
}

/// A polygon of the city object being read, with its owner and LOD.
struct OwnedPolygon {
    owner: u32,
    lod: u8,
    polygon: SourcePolygon,
}

fn attribute(e: &BytesStart<'_>, local: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref().as_bytes() == local)
        .map(|a| a.value.into_owned())
}

impl DocumentParser {
    fn start(&mut self, e: &BytesStart<'_>) {
        if !self.skip && self.document.srs_name.is_none() {
            self.document.srs_name = attribute(e, b"srsName");
        }
        let tag = match (self.skip, tag(e.local_name().as_ref().as_bytes())) {
            (true, _) => Tag::Other,
            (false, Tag::CityObject) if self.city_object.is_some() => Tag::Owner {
                installation: false,
            },
            (false, Tag::OrientableSurface(_)) => {
                let flipped = attribute(e, b"orientation").is_some_and(|o| o.trim() == "-");
                Tag::OrientableSurface(flipped)
            }
            (false, tag) => tag,
        };
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "only the tags that collect something start anything"
        )]
        match tag {
            Tag::CityObject => {
                self.city_object = Some(CityObject {
                    gml_id: attribute(e, b"id"),
                    polygons: Vec::new(),
                    terrain_points: Vec::new(),
                });
                self.polygons.clear();
                self.installations.clear();
            }
            Tag::Skip => self.skip = true,
            Tag::OrientableSurface(true) => self.reverse_flips += 1,
            Tag::TerrainIntersection => self.terrain_depth += 1,
            Tag::Owner { installation } => {
                self.next_owner += 1;
                self.owners.push(self.next_owner);
                self.semantics.push(None);
                if installation {
                    self.installations.insert(self.next_owner);
                }
            }
            Tag::Semantic(part) => self.semantics.push(Some(part)),
            Tag::Lod(lod) => self.lods.push(lod),
            Tag::Polygon => {
                self.points = Vec::new();
                self.ends = Vec::new();
                self.polygon_ok = true;
                self.polygon_dimension = attribute(e, b"srsDimension");
            }
            Tag::Ring => {
                self.numbers.clear();
                self.unreadable = false;
                self.dimension =
                    attribute(e, b"srsDimension").or_else(|| self.polygon_dimension.clone());
            }
            Tag::LineString if self.terrain_depth > 0 => {
                self.numbers.clear();
                self.unreadable = false;
                self.dimension = attribute(e, b"srsDimension");
            }
            Tag::Coordinates => {
                self.in_coordinates = true;
                if let Some(dimension) = attribute(e, b"srsDimension") {
                    self.dimension = Some(dimension);
                }
            }
            _ => {}
        }
        self.stack.push(tag);
    }

    fn end(&mut self) {
        let Some(tag) = self.stack.pop() else {
            return;
        };
        match tag {
            Tag::CityObject => self.end_city_object(),
            Tag::Skip => self.skip = false,
            Tag::OrientableSurface(flipped) => {
                if flipped {
                    self.reverse_flips -= 1;
                }
            }
            Tag::TerrainIntersection => self.terrain_depth -= 1,
            Tag::Owner { .. } => {
                self.owners.pop();
                self.semantics.pop();
            }
            Tag::Semantic(_) => {
                self.semantics.pop();
            }
            Tag::Lod(_) => {
                self.lods.pop();
            }
            Tag::Coordinates => self.in_coordinates = false,
            Tag::Ring => self.end_ring(),
            Tag::LineString => {
                if self.terrain_depth > 0 {
                    self.end_terrain_curve();
                }
            }
            Tag::Polygon => self.end_polygon(),
            Tag::Interior | Tag::Other => {}
        }
    }

    fn end_terrain_curve(&mut self) {
        let Some(city_object) = &mut self.city_object else {
            return;
        };
        let three_d = self.dimension.as_deref().is_none_or(|d| d.trim() == "3");
        if three_d && !self.unreadable && self.numbers.len().is_multiple_of(3) {
            for c in self.numbers.as_chunks::<3>().0 {
                city_object.terrain_points.push([c[0], c[1], c[2]]);
            }
        }
    }

    fn end_ring(&mut self) {
        let interior = self.stack.last() == Some(&Tag::Interior);
        let start = self.points.len();
        let three_d = self.dimension.as_deref().is_none_or(|d| d.trim() == "3");
        if three_d && !self.unreadable && self.numbers.len().is_multiple_of(3) {
            for c in self.numbers.as_chunks::<3>().0 {
                let p = [c[0], c[1], c[2]];
                if self.points.len() == start || self.points.last() != Some(&p) {
                    self.points.push(p);
                }
            }
            if self.points.len() > start + 1 && self.points.last() == self.points.get(start) {
                self.points.pop();
            }
        }
        if self.points.len() < start + 3 {
            self.points.truncate(start);
            if !interior {
                self.polygon_ok = false;
            }
            return;
        }
        // Only the first exterior ring and the holes after it count.
        if interior == self.ends.is_empty() {
            self.points.truncate(start);
            return;
        }
        if !self.reverse_flips.is_multiple_of(2) {
            self.points[start..].reverse();
        }
        self.ends.push(self.points.len());
    }

    fn end_polygon(&mut self) {
        let Some(&lod) = self.lods.last() else {
            return;
        };
        if self.city_object.is_none() || !self.polygon_ok || self.ends.is_empty() {
            return;
        }
        let polygon = SourcePolygon {
            part: self.semantics.last().copied().flatten(),
            lods: Lods::Both,
            points: std::mem::take(&mut self.points),
            ends: std::mem::take(&mut self.ends),
        };
        let owner = self.owners.last().copied().unwrap_or(0);
        self.polygons.push(OwnedPolygon {
            owner,
            lod,
            polygon,
        });
    }

    fn end_city_object(&mut self) {
        let Some(mut city_object) = self.city_object.take() else {
            return;
        };
        let mut owner_highest: HashMap<u32, u8> = HashMap::new();
        for p in &self.polygons {
            let h = owner_highest.entry(p.owner).or_insert(p.lod);
            *h = (*h).max(p.lod);
        }
        // The owners whose highest LOD has a polygon without semantics.
        let mut unsplit = HashSet::new();
        for p in &self.polygons {
            if p.polygon.part.is_none() && owner_highest.get(&p.owner) == Some(&p.lod) {
                unsplit.insert(p.owner);
            }
        }
        let key = |owner: u32, part: Option<Part>| {
            let kind = part.filter(|_| !unsplit.contains(&owner)).map(|p| match p {
                Part::Wall | Part::Window | Part::Door => Part::Wall,
                Part::Roof | Part::Ground => p,
            });
            (owner, kind)
        };
        self.polygons
            .retain(|p| p.polygon.part.is_some() || unsplit.contains(&p.owner));
        // The highest LOD of each key, and its highest LOD of 2 or less.
        let mut best: HashMap<(u32, Option<Part>), (u8, Option<u8>)> = HashMap::new();
        for p in &self.polygons {
            let (highest, low) = best
                .entry(key(p.owner, p.polygon.part))
                .or_insert((p.lod, None));
            *highest = (*highest).max(p.lod);
            if p.lod <= 2 {
                *low = (*low).max(Some(p.lod));
            }
        }
        let mut openings: HashMap<u32, Vec<Bounds3>> = HashMap::new();
        for p in &self.polygons {
            if matches!(p.polygon.part, Some(Part::Window | Part::Door))
                && let Some(bounds) = ring_bounds(&p.polygon.points)
            {
                openings.entry(p.owner).or_default().push(bounds);
            }
        }
        for OwnedPolygon {
            owner,
            lod,
            polygon,
        } in self.polygons.drain(..)
        {
            let Some(&(highest, low)) = best.get(&key(owner, polygon.part)) else {
                continue;
            };
            let lods = if lod != highest {
                if Some(lod) != low {
                    continue;
                }
                Lods::Lod2
            } else if highest <= 2 {
                Lods::Both
            } else if low.is_some()
                || self.installations.contains(&owner)
                || matches!(polygon.part, Some(Part::Window | Part::Door))
            {
                Lods::Highest
            } else if polygon.ends.len() > 1 {
                let closed =
                    close_opening_holes(&polygon, openings.get(&owner).map_or(&[], Vec::as_slice));
                if let Some(lod2_polygon) = closed {
                    city_object.polygons.push(lod2_polygon);
                    Lods::Highest
                } else {
                    Lods::Both
                }
            } else {
                Lods::Both
            };
            city_object.polygons.push(SourcePolygon { lods, ..polygon });
        }
        if !city_object.polygons.is_empty() {
            self.document.city_objects.push(city_object);
        }
    }
}

type Bounds3 = ([f64; 3], [f64; 3]);

fn ring_bounds(points: &[[f64; 3]]) -> Option<Bounds3> {
    let &first = points.first()?;
    Some(points[1..].iter().fold((first, first), |(lo, hi), &p| {
        (
            [0, 1, 2].map(|i| lo[i].min(p[i])),
            [0, 1, 2].map(|i| hi[i].max(p[i])),
        )
    }))
}

/// The LOD2 copy of `polygon`: a wall without its holes, or a roof or ground without those of window and door openings.
/// `None` if a roof or ground has no such hole.
fn close_opening_holes(
    polygon: &SourcePolygon,
    owner_openings: &[Bounds3],
) -> Option<SourcePolygon> {
    let exterior = polygon.ends[0];
    if polygon.part == Some(Part::Wall) {
        return Some(SourcePolygon {
            part: polygon.part,
            lods: Lods::Lod2,
            points: polygon.points[..exterior].to_vec(),
            ends: vec![exterior],
        });
    }
    if owner_openings.is_empty() {
        return None;
    }
    let mut points = polygon.points[..exterior].to_vec();
    let mut ends = vec![exterior];
    let mut dropped_any = false;
    let mut start = exterior;
    for &end in &polygon.ends[1..] {
        let hole = &polygon.points[start..end];
        let is_opening = ring_bounds(hole).is_some_and(|(r_lo, r_hi)| {
            let tol = [0, 1, 2].map(|i| 0.5 * (r_hi[i] - r_lo[i]).max(1e-6));
            owner_openings.iter().any(|&(o_lo, o_hi)| {
                (0..3).all(|i| {
                    (o_lo[i] - r_lo[i]).abs() <= tol[i] && (o_hi[i] - r_hi[i]).abs() <= tol[i]
                })
            })
        });
        if is_opening {
            dropped_any = true;
        } else {
            points.extend_from_slice(hole);
            ends.push(points.len());
        }
        start = end;
    }
    dropped_any.then_some(SourcePolygon {
        part: polygon.part,
        lods: Lods::Lod2,
        points,
        ends,
    })
}

/// Appends the numbers of `text`, separated by whitespace or commas, to `out`.
/// Returns `false` if one of them is not a finite number.
fn parse_numbers(text: &[u8], out: &mut Vec<f64>) -> bool {
    let Ok(text) = std::str::from_utf8(text) else {
        return false;
    };
    for token in text
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
    {
        match token.parse::<f64>() {
            Ok(v) if v.is_finite() => out.push(v),
            _ => return false,
        }
    }
    true
}
