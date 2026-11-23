//! One pass over a `CityGML` document by local names, so that `CityGML` 2.0 and 3.0 and any namespace prefixes read alike.

use std::collections::HashSet;
use std::io::BufRead;

use anyhow::{Result as AnyResult, bail};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::crs::epsg_of_srs_name;
use super::model::{ObjectKind, OwnedPolygon, Part, Rings, Semantic, SourceDocument, SourceObject};

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
    if let Ok(name) = std::str::from_utf8(name) {
        match ObjectKind::of(name) {
            Some(ObjectKind::Root) => return Tag::CityObject,
            Some(ObjectKind::Part) => {
                return Tag::Owner {
                    installation: false,
                };
            }
            Some(ObjectKind::Installation) => return Tag::Owner { installation: true },
            None => {}
        }
        match Semantic::of(name) {
            Some(Semantic::Seen(part)) => return Tag::Semantic(part),
            Some(Semantic::Hidden) => return Tag::Skip,
            None => {}
        }
    }
    match name {
        b"interiorRoom"
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
pub fn parse<R: BufRead>(reader: R) -> AnyResult<SourceDocument> {
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
            Ok(Event::CData(t)) if parser.in_coordinates => {
                if !parse_numbers(t.as_bytes(), &mut parser.numbers) {
                    parser.unreadable = true;
                }
            }
            Ok(Event::Eof) if !parser.stack.is_empty() => {
                bail!(
                    "the file ends inside an element at byte {}",
                    xml.buffer_position()
                );
            }
            Ok(Event::Eof) => {
                if let Some(other) = parser.other_crs {
                    let crs = parser.document.crs.unwrap_or_default();
                    bail!("the file names the CRS {crs:?}, but a geometry names {other:?}");
                }
                return Ok(parser.document);
            }
            Ok(_) => {}
            Err(e) => bail!("invalid XML at byte {}: {e}", xml.buffer_position()),
        }
    }
}

#[derive(Default)]
struct DocumentParser {
    stack: Vec<Tag>,
    /// Whether the parser is inside a skipped element.
    skip: bool,
    /// The enclosing `OrientableSurface` elements of `orientation="-"`, an odd count reversing rings.
    reverse_flips: u32,
    /// The enclosing `lodNTerrainIntersection` properties.
    terrain_depth: u32,
    /// The city object being read.
    city_object: Option<SourceObject>,
    owners: Vec<u32>,
    next_owner: u32,
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
    rings: Rings,
    /// The first `srsName` that names another EPSG code than the file's.
    other_crs: Option<String>,
    document: SourceDocument,
}

fn attribute(e: &BytesStart<'_>, local: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref().as_bytes() == local)
        .map(|a| a.value.into_owned())
}

impl DocumentParser {
    fn start(&mut self, e: &BytesStart<'_>) {
        if !self.skip
            && let Some(name) = attribute(e, b"srsName")
        {
            match &self.document.crs {
                None => self.document.crs = Some(name),
                Some(crs) => {
                    let epsg = epsg_of_srs_name(&name);
                    if self.other_crs.is_none() && epsg.is_some() && epsg != epsg_of_srs_name(crs) {
                        self.other_crs = Some(name);
                    }
                }
            }
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
                self.city_object = Some(SourceObject {
                    id: attribute(e, b"id"),
                    polygons: Vec::new(),
                    installations: HashSet::new(),
                    terrain_points: Vec::new(),
                });
            }
            Tag::Skip => self.skip = true,
            Tag::OrientableSurface(true) => self.reverse_flips += 1,
            Tag::TerrainIntersection => self.terrain_depth += 1,
            Tag::Owner { installation } => {
                self.next_owner += 1;
                self.owners.push(self.next_owner);
                self.semantics.push(None);
                if installation && let Some(city_object) = &mut self.city_object {
                    city_object.installations.insert(self.next_owner);
                }
            }
            Tag::Semantic(part) => self.semantics.push(Some(part)),
            Tag::Lod(lod) => self.lods.push(lod),
            Tag::Polygon => {
                self.rings = Rings::default();
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
        let positions = positions(&self.numbers, self.dimension.as_deref(), self.unreadable);
        if let Some(city_object) = &mut self.city_object {
            city_object.terrain_points.extend_from_slice(positions);
        }
    }

    fn end_ring(&mut self) {
        let interior = self.stack.last() == Some(&Tag::Interior);
        let reverse = !self.reverse_flips.is_multiple_of(2);
        let positions = positions(&self.numbers, self.dimension.as_deref(), self.unreadable);
        self.rings
            .push(positions.iter().copied(), interior, reverse);
    }

    fn end_polygon(&mut self) {
        let rings = std::mem::take(&mut self.rings);
        let (Some(&lod), Some(city_object)) = (self.lods.last(), &mut self.city_object) else {
            return;
        };
        let Some(polygon) = rings.finish(self.semantics.last().copied().flatten()) else {
            return;
        };
        city_object.polygons.push(OwnedPolygon {
            owner: self.owners.last().copied().unwrap_or(0),
            lod,
            polygon,
        });
    }

    fn end_city_object(&mut self) {
        if let Some(city_object) = self.city_object.take()
            && !city_object.polygons.is_empty()
        {
            self.document.city_objects.push(city_object);
        }
    }
}

/// The positions of `numbers`, or none if they are not 3D or one is `unreadable`.
fn positions<'a>(numbers: &'a [f64], dimension: Option<&str>, unreadable: bool) -> &'a [[f64; 3]] {
    let three_d = dimension.is_none_or(|d| d.trim() == "3");
    match numbers.as_chunks::<3>() {
        (positions, []) if three_d && !unreadable => positions,
        _ => &[],
    }
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
