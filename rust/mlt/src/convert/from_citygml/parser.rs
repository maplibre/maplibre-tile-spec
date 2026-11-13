//! One pass over a `CityGML` document by local names, so that `CityGML` 2.0 and 3.0 and any
//! namespace prefixes read alike.

use std::collections::{HashMap, HashSet};

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

/// The meshes of a building a polygon is in: that of the highest LOD of each part, and that of
/// LOD2 that lower zooms simplify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lods {
    Both,
    Highest,
    Lod2,
}

/// A polygon in source coordinates: its rings, the exterior first, none repeating its first
/// point at its end.
#[derive(Debug, PartialEq)]
pub struct Polygon {
    /// The semantic surface the polygon lies in, if any.
    pub part: Option<Part>,
    pub lods: Lods,
    pub points: Vec<[f64; 3]>,
    /// Where each ring ends in `points`.
    pub ends: Vec<usize>,
}

/// A building, bridge or tunnel, with the polygons of the highest LOD of each of its parts and
/// of its LOD2. A part whose polygons all lie in semantic surfaces has these of its walls, with
/// their windows and doors, of its roofs and of its grounds each.
///
/// The LOD2 of a part whose highest LOD is above 2 is its highest LOD of 2 or less or, without
/// one, its highest LOD without windows, doors and installations, and with the holes of the
/// other polygons closed.
#[derive(Debug, PartialEq)]
pub struct Building {
    pub gml_id: Option<String>,
    pub polygons: Vec<Polygon>,
}

/// What a document holds.
#[derive(Debug, Default, PartialEq)]
pub struct Document {
    /// The first `srsName` of the document.
    pub srs_name: Option<String>,
    pub buildings: Vec<Building>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    /// A city object that becomes one feature.
    Building,
    /// A part of a building that picks its own highest LOD, such as a building part or installation.
    Owner {
        installation: bool,
    },
    Semantic(Part),
    /// Content that is not drawn from outside: closure surfaces, interiors, LOD4, appearances and
    /// implicit geometries.
    Skip,
    /// An `lodN…` geometry property.
    Lod(u8),
    Polygon,
    Interior,
    Ring,
    Coordinates,
    Other,
}

fn tag(name: &[u8]) -> Tag {
    match name {
        b"Building" | b"Bridge" | b"Tunnel" => Tag::Building,
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
        b"Polygon" | b"Triangle" | b"Rectangle" | b"PolygonPatch" => Tag::Polygon,
        b"interior" | b"innerBoundaryIs" => Tag::Interior,
        b"LinearRing" => Tag::Ring,
        b"posList" | b"pos" | b"coordinates" => Tag::Coordinates,
        _ => lod_tag(name).unwrap_or(Tag::Other),
    }
}

/// `lodN…` geometry properties: LOD4 (interiors), curves and terrain intersections are skipped.
fn lod_tag(name: &[u8]) -> Option<Tag> {
    let rest = name.strip_prefix(b"lod")?;
    let (&digit, kind) = rest.split_first()?;
    if !digit.is_ascii_digit() || !kind.first()?.is_ascii_uppercase() {
        return None;
    }
    if digit == b'4' || kind.ends_with(b"MultiCurve") || kind.starts_with(b"TerrainIntersection") {
        return Some(Tag::Skip);
    }
    Some(Tag::Lod(digit - b'0'))
}

/// Reads the buildings of `bytes`.
pub fn parse(bytes: &[u8]) -> AnyResult<Document> {
    let mut parser = Parser::default();
    let mut reader = Reader::from_reader(bytes);
    loop {
        match reader.read_event() {
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
            Err(e) => bail!("invalid XML at byte {}: {e}", reader.buffer_position()),
        }
    }
}

#[derive(Default)]
struct Parser {
    stack: Vec<Tag>,
    /// The depth of a skipped element: everything inside it is left unread.
    skip: Option<usize>,
    /// The building being read, its polygons collected in `polygons` until it ends.
    building: Option<Building>,
    /// The polygons of the building, each with its owner and LOD.
    polygons: Vec<(u32, u8, Polygon)>,
    owners: Vec<u32>,
    next_owner: u32,
    /// The owners of the building that are installations.
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

fn attribute(e: &BytesStart<'_>, local: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref().as_bytes() == local)
        .map(|a| a.value.into_owned())
}

impl Parser {
    fn start(&mut self, e: &BytesStart<'_>) {
        if self.document.srs_name.is_none() {
            self.document.srs_name = attribute(e, b"srsName");
        }
        let depth = self.stack.len();
        let tag = match (self.skip, tag(e.local_name().as_ref().as_bytes())) {
            (Some(_), _) => Tag::Other,
            (None, Tag::Building) if self.building.is_some() => Tag::Owner {
                installation: false,
            },
            (None, tag) => tag,
        };
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "only the tags that collect something start anything"
        )]
        match tag {
            Tag::Building => {
                self.building = Some(Building {
                    gml_id: attribute(e, b"id"),
                    polygons: Vec::new(),
                });
                self.polygons.clear();
                self.installations.clear();
            }
            Tag::Skip => self.skip = Some(depth),
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
            Tag::Building => self.end_building(),
            Tag::Skip => self.skip = None,
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
            Tag::Polygon => self.end_polygon(),
            Tag::Interior | Tag::Other => {}
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
        self.ends.push(self.points.len());
    }

    fn end_polygon(&mut self) {
        let Some(&lod) = self.lods.last() else {
            return;
        };
        if self.building.is_none() || !self.polygon_ok || self.ends.is_empty() {
            return;
        }
        let polygon = Polygon {
            part: self.semantics.last().copied().flatten(),
            lods: Lods::Both,
            points: std::mem::take(&mut self.points),
            ends: std::mem::take(&mut self.ends),
        };
        let owner = self.owners.last().copied().unwrap_or(0);
        self.polygons.push((owner, lod, polygon));
    }

    fn end_building(&mut self) {
        let Some(mut building) = self.building.take() else {
            return;
        };
        // An owner whose polygons all lie in semantic surfaces picks the LODs of its walls,
        // roofs and grounds each on its own; any other picks them for all its polygons at once.
        let mut unsplit = HashSet::new();
        for (owner, _, polygon) in &self.polygons {
            if polygon.part.is_none() {
                unsplit.insert(*owner);
            }
        }
        let key = |owner: u32, part: Option<Part>| {
            let kind = part.filter(|_| !unsplit.contains(&owner)).map(|p| match p {
                Part::Wall | Part::Window | Part::Door => Part::Wall,
                Part::Roof | Part::Ground => p,
            });
            (owner, kind)
        };
        // The highest LOD of each key, and its highest LOD of 2 or less.
        let mut best: HashMap<(u32, Option<Part>), (u8, Option<u8>)> = HashMap::new();
        for &(owner, lod, ref polygon) in &self.polygons {
            let (highest, low) = best.entry(key(owner, polygon.part)).or_insert((lod, None));
            *highest = (*highest).max(lod);
            if lod <= 2 {
                *low = (*low).max(Some(lod));
            }
        }
        for (owner, lod, polygon) in self.polygons.drain(..) {
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
            } else if let [exterior, _, ..] = polygon.ends[..] {
                building.polygons.push(Polygon {
                    part: polygon.part,
                    lods: Lods::Lod2,
                    points: polygon.points[..exterior].to_vec(),
                    ends: vec![exterior],
                });
                Lods::Highest
            } else {
                Lods::Both
            };
            building.polygons.push(Polygon { lods, ..polygon });
        }
        if !building.polygons.is_empty() {
            self.document.buildings.push(building);
        }
    }
}

/// Appends the numbers of `text`, separated by whitespace or commas, to `out`, or says that
/// one of them is not a finite number.
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

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    const HEAD: &str = r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
        xmlns:bldg="http://www.opengis.net/citygml/building/2.0" xmlns:gml="http://www.opengis.net/gml">"#;

    fn ring(points: &[[f64; 3]]) -> String {
        let list: Vec<String> = points
            .iter()
            .chain(&points[..1])
            .map(|p| format!("{} {} {}", p[0], p[1], p[2]))
            .collect();
        format!(
            "<gml:LinearRing><gml:posList>{}</gml:posList></gml:LinearRing>",
            list.join(" ")
        )
    }

    fn polygon(rings: &[&[[f64; 3]]]) -> String {
        let mut out = format!(
            "<gml:Polygon><gml:exterior>{}</gml:exterior>",
            ring(rings[0])
        );
        for hole in &rings[1..] {
            write!(out, "<gml:interior>{}</gml:interior>", ring(hole)).unwrap();
        }
        out + "</gml:Polygon>"
    }

    fn surface(semantic: &str, lod: u8, polygon: &str) -> String {
        format!(
            "<bldg:boundedBy><bldg:{semantic}><bldg:lod{lod}MultiSurface><gml:MultiSurface>\
             <gml:surfaceMember>{polygon}</gml:surfaceMember></gml:MultiSurface>\
             </bldg:lod{lod}MultiSurface></bldg:{semantic}></bldg:boundedBy>"
        )
    }

    const SQUARE: [[f64; 3]; 4] = [
        [0.0, 0.0, 10.0],
        [10.0, 0.0, 10.0],
        [10.0, 10.0, 10.0],
        [0.0, 10.0, 10.0],
    ];
    const HOLE: [[f64; 3]; 3] = [[2.0, 2.0, 10.0], [4.0, 2.0, 10.0], [2.0, 4.0, 10.0]];

    #[test]
    fn a_building_keeps_its_highest_lod_with_semantics_and_holes() {
        let xml = format!(
            r#"{HEAD}<gml:Envelope srsName="EPSG:25832"/><core:cityObjectMember><bldg:Building gml:id="B1">{}{}{}</bldg:Building></core:cityObjectMember></core:CityModel>"#,
            surface("RoofSurface", 1, &polygon(&[&SQUARE])),
            surface("RoofSurface", 2, &polygon(&[&SQUARE, &HOLE])),
            surface("ClosureSurface", 2, &polygon(&[&SQUARE])),
        );
        let document = parse(xml.as_bytes()).unwrap();
        assert_eq!(document.srs_name.as_deref(), Some("EPSG:25832"));
        let [building] = &document.buildings[..] else {
            panic!("{document:?}");
        };
        assert_eq!(building.gml_id.as_deref(), Some("B1"));
        let [roof] = &building.polygons[..] else {
            panic!("{building:?}");
        };
        assert_eq!(roof.part, Some(Part::Roof));
        assert_eq!(roof.ends, [4, 7]);
        assert_eq!(roof.points[4..], HOLE);
    }

    #[test]
    fn each_part_of_a_building_picks_its_own_lod() {
        let xml = format!(
            "{HEAD}<bldg:Building>{}<bldg:consistsOfBuildingPart><bldg:BuildingPart>{}\
             </bldg:BuildingPart></bldg:consistsOfBuildingPart></bldg:Building></core:CityModel>",
            surface("WallSurface", 3, &polygon(&[&SQUARE])),
            surface("GroundSurface", 2, &polygon(&[&SQUARE])),
        );
        let document = parse(xml.as_bytes()).unwrap();
        let parts: Vec<_> = document.buildings[0]
            .polygons
            .iter()
            .map(|p| p.part)
            .collect();
        assert_eq!(parts, [Some(Part::Wall), Some(Part::Ground)]);
        assert_eq!(document.buildings[0].gml_id, None);
    }

    #[test]
    fn lod2_closes_the_holes_of_lod3_and_leaves_out_windows_and_installations() {
        let window = format!(
            "<bldg:opening><bldg:Window><bldg:lod3MultiSurface><gml:MultiSurface><gml:surfaceMember>\
             {}</gml:surfaceMember></gml:MultiSurface></bldg:lod3MultiSurface></bldg:Window></bldg:opening>",
            polygon(&[&HOLE]),
        );
        let xml = format!(
            "{HEAD}<bldg:Building><bldg:boundedBy><bldg:WallSurface><bldg:lod3MultiSurface>\
             <gml:MultiSurface><gml:surfaceMember>{}</gml:surfaceMember></gml:MultiSurface>\
             </bldg:lod3MultiSurface>{window}</bldg:WallSurface></bldg:boundedBy>\
             <bldg:outerBuildingInstallation><bldg:BuildingInstallation>{}</bldg:BuildingInstallation>\
             </bldg:outerBuildingInstallation></bldg:Building></core:CityModel>",
            polygon(&[&SQUARE, &HOLE]),
            surface("RoofSurface", 3, &polygon(&[&SQUARE])),
        );
        let document = parse(xml.as_bytes()).unwrap();
        let polygons: Vec<_> = document.buildings[0]
            .polygons
            .iter()
            .map(|p| (p.part, p.lods, p.ends.clone()))
            .collect();
        assert_eq!(
            polygons,
            [
                (Some(Part::Wall), Lods::Lod2, vec![4]),
                (Some(Part::Wall), Lods::Highest, vec![4, 7]),
                (Some(Part::Window), Lods::Highest, vec![3]),
                (Some(Part::Roof), Lods::Highest, vec![4]),
            ]
        );
    }

    #[test]
    fn walls_roofs_and_grounds_each_pick_their_highest_lod_and_lod2() {
        let xml = format!(
            "{HEAD}<bldg:Building>{}{}{}{}</bldg:Building></core:CityModel>",
            surface("WallSurface", 1, &polygon(&[&SQUARE])),
            surface("WallSurface", 3, &polygon(&[&SQUARE, &HOLE])),
            surface("RoofSurface", 2, &polygon(&[&SQUARE, &HOLE])),
            surface("GroundSurface", 1, &polygon(&[&SQUARE])),
        );
        let document = parse(xml.as_bytes()).unwrap();
        let polygons: Vec<_> = document.buildings[0]
            .polygons
            .iter()
            .map(|p| (p.part, p.lods, p.ends.len()))
            .collect();
        assert_eq!(
            polygons,
            [
                (Some(Part::Wall), Lods::Lod2, 1),
                (Some(Part::Wall), Lods::Highest, 2),
                (Some(Part::Roof), Lods::Both, 2),
                (Some(Part::Ground), Lods::Both, 1),
            ]
        );
    }

    #[test]
    fn a_part_with_a_polygon_outside_semantic_surfaces_picks_its_lods_at_once() {
        let solid = format!(
            "<bldg:lod1MultiSurface><gml:MultiSurface><gml:surfaceMember>{}</gml:surfaceMember>\
             </gml:MultiSurface></bldg:lod1MultiSurface>",
            polygon(&[&SQUARE]),
        );
        let xml = format!(
            "{HEAD}<bldg:Building>{solid}{}</bldg:Building></core:CityModel>",
            surface("RoofSurface", 2, &polygon(&[&SQUARE])),
        );
        let document = parse(xml.as_bytes()).unwrap();
        let polygons: Vec<_> = document.buildings[0]
            .polygons
            .iter()
            .map(|p| (p.part, p.lods))
            .collect();
        assert_eq!(polygons, [(Some(Part::Roof), Lods::Both)]);
    }

    #[test]
    fn rings_without_area_and_2d_rings_are_left_out() {
        let line = [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
        let flat = "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList srsDimension=\"2\">\
                    0 0 1 0 1 1 0 0</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon>";
        let xml = format!(
            "{HEAD}<bldg:Building>{}{}</bldg:Building></core:CityModel>",
            surface("WallSurface", 2, &polygon(&[&line])),
            surface("WallSurface", 2, flat),
        );
        assert_eq!(parse(xml.as_bytes()).unwrap().buildings, []);
    }

    #[test]
    fn numbers_are_split_by_whitespace_and_commas() {
        let mut out = Vec::new();
        assert!(parse_numbers(b" 1.5,2\n-3e2  4", &mut out));
        assert_eq!(out, [1.5, 2.0, -300.0, 4.0]);
    }

    #[test]
    fn a_token_that_is_not_a_finite_number_spoils_the_text() {
        for text in [&b"1 2 x 3"[..], b"1 2 NaN", b"1 inf 3"] {
            assert!(!parse_numbers(text, &mut Vec::new()), "{text:?}");
        }
    }

    #[test]
    fn a_ring_with_an_unreadable_coordinate_or_a_2d_polygon_is_left_out() {
        let shifted = "<gml:Polygon><gml:exterior><gml:LinearRing><gml:posList>\
                       0 0 10 x 10 0 10 10 10 10 0 10 10 0 0 10</gml:posList></gml:LinearRing>\
                       </gml:exterior></gml:Polygon>";
        let flat = "<gml:Polygon srsDimension=\"2\"><gml:exterior><gml:LinearRing><gml:posList>\
                    0 0 1 0 1 1 0 1 0 0 1 0</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon>";
        let xml = format!(
            "{HEAD}<bldg:Building>{}{}</bldg:Building></core:CityModel>",
            surface("WallSurface", 2, shifted),
            surface("WallSurface", 2, flat),
        );
        assert_eq!(parse(xml.as_bytes()).unwrap().buildings, []);
    }

    #[test]
    fn broken_xml_fails_with_its_position() {
        let err = parse(b"<a><b></a").unwrap_err();
        assert!(err.to_string().starts_with("invalid XML at byte"), "{err}");
    }

    #[test]
    fn a_mismatched_end_tag_is_broken_xml() {
        let err = parse(b"<a><b></c></a>").unwrap_err();
        assert!(err.to_string().starts_with("invalid XML at byte"), "{err}");
    }
}
