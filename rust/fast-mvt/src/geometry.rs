#[cfg(feature = "reader")]
use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use usize_cast::IntoUsize as _;

#[cfg(feature = "writer")]
use crate::MvtPolygon;
use crate::generated::vector_tile::tile::GeomType;
use crate::{MvtCoord, MvtError, MvtGeometry, MvtLineString, MvtResult};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
enum Command {
    MoveTo = 1,
    LineTo = 2,
    ClosePath = 7,
}

impl Command {
    #[cfg(feature = "reader")]
    fn decode(value: u32) -> MvtResult<(Self, usize)> {
        Ok((
            match value & 0x7 {
                1 => Self::MoveTo,
                2 => Self::LineTo,
                7 => Self::ClosePath,
                _ => return Err(MvtError::InvalidGeometry),
            },
            (value >> 3).into_usize(),
        ))
    }

    #[cfg(feature = "writer")]
    fn encode(self, count: u32) -> MvtResult<u32> {
        if count > 0x1fff_ffff {
            return Err(MvtError::CommandCount(count));
        }
        Ok(((self as u32) & 0x7) | (count << 3))
    }
}

#[cfg(any(feature = "writer", test))]
fn encode_parameter(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)).cast_unsigned()
}

fn signed_area(coords: &[MvtCoord]) -> i64 {
    #[inline]
    fn cross_product(a: MvtCoord, b: MvtCoord) -> i64 {
        i64::from(a.x) * i64::from(b.y) - i64::from(a.y) * i64::from(b.x)
    }

    let Some((&first, rest)) = coords.split_first() else {
        return 0;
    };
    let mut prev = first;
    let mut area = 0_i64;
    for &coord in rest {
        area += cross_product(prev, coord);
        prev = coord;
    }
    area + cross_product(prev, first)
}

#[cfg(feature = "reader")]
pub(crate) fn decode_geometry(
    geom_type: Option<GeomType>,
    commands: &[u32],
) -> MvtResult<MvtGeometry> {
    if commands.is_empty() {
        return match geom_type {
            Some(GeomType::Point) => Ok(MultiPoint(Vec::new()).into()),
            Some(GeomType::Linestring) => Ok(MultiLineString(Vec::new()).into()),
            Some(GeomType::Polygon) => Ok(MultiPolygon(Vec::new()).into()),
            _ => Err(MvtError::InvalidGeometry),
        };
    }
    match geom_type {
        Some(GeomType::Point) => parse_points(commands),
        Some(GeomType::Linestring) => parse_linestrings(commands),
        Some(GeomType::Polygon) => parse_polygons(commands),
        _ => Err(MvtError::InvalidGeometry),
    }
}

#[cfg(feature = "reader")]
fn decode_coord(cursor: &mut MvtCoord, values: &[u32]) -> MvtResult<MvtCoord> {
    let [dx, dy] = values else {
        return Err(MvtError::InvalidGeometry);
    };
    cursor.x = saturating_add_delta(cursor.x, *dx);
    cursor.y = saturating_add_delta(cursor.y, *dy);
    Ok(*cursor)
}

#[cfg(feature = "reader")]
fn saturating_add_delta(value: i32, delta: u32) -> i32 {
    let v = (delta >> 1).cast_signed() ^ -(delta & 1).cast_signed();
    value.saturating_add(v)
}

#[cfg(feature = "reader")]
fn parse_points(data: &[u32]) -> MvtResult<MvtGeometry> {
    let mut cursor = Coord { x: 0, y: 0 };
    let mut points = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (command, count) = Command::decode(data[offset])?;
        if command != Command::MoveTo {
            return Err(MvtError::InvalidGeometry);
        }
        offset += 1;
        let value_len = count.saturating_mul(2);
        if data.len() < offset + value_len {
            return Err(MvtError::InvalidGeometry);
        }
        points.reserve(count);
        for _ in 0..count {
            points.push(Point(decode_coord(&mut cursor, &data[offset..offset + 2])?));
            offset += 2;
        }
    }
    if points.is_empty() {
        return Err(MvtError::InvalidGeometry);
    }
    if points.len() == 1 {
        Ok(points.pop().ok_or(MvtError::InvalidGeometry)?.into())
    } else {
        Ok(MultiPoint(points).into())
    }
}

#[cfg(feature = "reader")]
#[derive(Debug, Copy, Clone)]
struct LineInfo {
    len: usize,
    coord_count: usize,
    line_count: usize,
}

#[cfg(feature = "reader")]
fn line_info(data: &[u32]) -> MvtResult<LineInfo> {
    if data.len() < 3 {
        return Err(MvtError::InvalidGeometry);
    }
    let (move_to, move_count) = Command::decode(data[0])?;
    if move_to != Command::MoveTo || move_count == 0 {
        return Err(MvtError::InvalidGeometry);
    }
    let move_len = 1 + move_count.saturating_mul(2);
    if data.len() < move_len {
        return Err(MvtError::InvalidGeometry);
    }
    let mut len = move_len;
    let mut line_count = 0;
    if data.len() > move_len {
        let (line_to, count) = Command::decode(data[move_len])?;
        if line_to == Command::LineTo {
            line_count = count;
            len = move_len + 1 + line_count.saturating_mul(2);
        }
    }
    if data.len() < len {
        return Err(MvtError::InvalidGeometry);
    }
    Ok(LineInfo {
        len,
        coord_count: move_count + line_count,
        line_count,
    })
}

#[cfg(feature = "reader")]
fn parse_line_slice(
    cursor: &mut MvtCoord,
    data: &[u32],
    info: LineInfo,
) -> MvtResult<MvtLineString> {
    let (_, move_count) = Command::decode(data[0])?;
    let mut coords = Vec::with_capacity(info.coord_count);
    for move_idx in 0..move_count {
        let offset = 1 + move_idx * 2;
        coords.push(decode_coord(cursor, &data[offset..offset + 2])?);
    }
    let line_start = 1 + move_count * 2;
    for coord_idx in 0..info.line_count {
        let offset = line_start + 1 + coord_idx * 2;
        coords.push(decode_coord(cursor, &data[offset..offset + 2])?);
    }
    if info.len != data.len() {
        return Err(MvtError::InvalidGeometry);
    }
    Ok(LineString(coords))
}

#[cfg(feature = "reader")]
fn parse_linestrings(data: &[u32]) -> MvtResult<MvtGeometry> {
    let mut cursor = Coord { x: 0, y: 0 };
    let mut offset = 0;
    let mut lines = Vec::new();
    while offset < data.len() {
        let info = line_info(&data[offset..])?;
        lines.push(parse_line_slice(
            &mut cursor,
            &data[offset..offset + info.len],
            info,
        )?);
        let len = info.len;
        offset += len;
    }
    if lines.is_empty() {
        return Err(MvtError::InvalidGeometry);
    }
    if lines.len() == 1 {
        Ok(lines.pop().ok_or(MvtError::InvalidGeometry)?.into())
    } else {
        Ok(MultiLineString(lines).into())
    }
}

#[cfg(feature = "reader")]
fn ring_info(data: &[u32]) -> MvtResult<(LineInfo, usize)> {
    let info = line_info(data)?;
    let len = info.len + 1;
    if data.len() < len {
        return Err(MvtError::InvalidGeometry);
    }
    let (close, close_count) = Command::decode(data[len - 1])?;
    if close != Command::ClosePath || close_count != 1 {
        return Err(MvtError::InvalidGeometry);
    }
    Ok((info, len))
}

#[cfg(feature = "reader")]
fn parse_ring(cursor: &mut MvtCoord, data: &[u32]) -> MvtResult<(MvtLineString, i64, usize)> {
    let (info, len) = ring_info(data)?;
    let mut ring = parse_line_slice(cursor, &data[..info.len], info)?;
    let area = signed_area(&ring.0);
    let first = *ring.0.first().ok_or(MvtError::InvalidGeometry)?;
    ring.0.push(first);
    Ok((ring, area, len))
}

#[cfg(feature = "reader")]
fn parse_polygons(data: &[u32]) -> MvtResult<MvtGeometry> {
    let mut cursor = MvtCoord::zero();
    let mut offset = 0;
    let mut current_exterior: Option<MvtLineString> = None;
    let mut current_interiors = Vec::new();
    let mut polygons = Vec::new();
    while offset < data.len() {
        let (ring, area, len) = parse_ring(&mut cursor, &data[offset..])?;
        if area > 0 || current_exterior.is_none() {
            if let Some(exterior) = current_exterior.replace(ring) {
                polygons.push(Polygon::new(
                    exterior,
                    std::mem::take(&mut current_interiors),
                ));
            }
        } else {
            current_interiors.push(ring);
        }
        offset += len;
    }
    if let Some(exterior) = current_exterior {
        polygons.push(Polygon::new(exterior, current_interiors));
    }
    if polygons.is_empty() {
        return Err(MvtError::InvalidGeometry);
    }
    if polygons.len() == 1 {
        Ok(polygons.pop().ok_or(MvtError::InvalidGeometry)?.into())
    } else {
        Ok(MultiPolygon(polygons).into())
    }
}

#[cfg(feature = "writer")]
pub(crate) fn encode_geometry(geometry: &MvtGeometry) -> MvtResult<(GeomType, Vec<u32>)> {
    match geometry {
        MvtGeometry::Point(point) => {
            let coords = [point.0];
            let mut encoder = GeometryEncoder::with_capacity(coords.len());
            encoder.points(coords)?;
            Ok((GeomType::Point, encoder.into_data()))
        }
        MvtGeometry::MultiPoint(points) => {
            if points.0.is_empty() {
                return Ok((GeomType::Point, Vec::new()));
            }
            let mut encoder = GeometryEncoder::with_capacity(points.0.len());
            encoder.points(points.0.iter().map(|point| point.0))?;
            Ok((GeomType::Point, encoder.into_data()))
        }
        MvtGeometry::LineString(line) => {
            let mut encoder = GeometryEncoder::with_capacity(line.0.len());
            encoder.line(line)?;
            Ok((GeomType::Linestring, encoder.into_data()))
        }
        MvtGeometry::MultiLineString(lines) => {
            if lines.0.is_empty() {
                return Ok((GeomType::Linestring, Vec::new()));
            }
            let capacity = lines.0.iter().map(|line| line.0.len()).sum();
            let mut encoder = GeometryEncoder::with_capacity(capacity);
            for line in &lines.0 {
                encoder.line(line)?;
            }
            Ok((GeomType::Linestring, encoder.into_data()))
        }
        MvtGeometry::Polygon(polygon) => {
            let mut encoder = GeometryEncoder::with_capacity(polygon_vertex_count(polygon));
            encoder.polygon(polygon)?;
            Ok((GeomType::Polygon, encoder.into_data()))
        }
        MvtGeometry::MultiPolygon(polygons) => {
            if polygons.0.is_empty() {
                return Ok((GeomType::Polygon, Vec::new()));
            }
            let capacity = polygons.0.iter().map(polygon_vertex_count).sum();
            let mut encoder = GeometryEncoder::with_capacity(capacity);
            for polygon in &polygons.0 {
                encoder.polygon(polygon)?;
            }
            Ok((GeomType::Polygon, encoder.into_data()))
        }
        MvtGeometry::GeometryCollection(collection) if collection.0.len() == 1 => {
            encode_geometry(&collection.0[0])
        }
        MvtGeometry::GeometryCollection(_) => Err(MvtError::UnsupportedGeometry(
            "GeometryCollection with multiple items",
        )),
        MvtGeometry::Line(_) => Err(MvtError::UnsupportedGeometry("Line")),
        MvtGeometry::Rect(_) => Err(MvtError::UnsupportedGeometry("Rect")),
        MvtGeometry::Triangle(_) => Err(MvtError::UnsupportedGeometry("Triangle")),
    }
}

#[cfg(feature = "writer")]
fn polygon_vertex_count(polygon: &MvtPolygon) -> usize {
    polygon.exterior().0.len()
        + polygon
            .interiors()
            .iter()
            .map(|line| line.0.len())
            .sum::<usize>()
}

#[cfg(feature = "writer")]
struct GeometryEncoder {
    data: Vec<u32>,
    cursor: MvtCoord,
}

#[cfg(feature = "writer")]
impl GeometryEncoder {
    fn with_capacity(coords: usize) -> Self {
        Self {
            data: Vec::with_capacity(1 + coords.saturating_mul(2)),
            cursor: MvtCoord { x: 0, y: 0 },
        }
    }

    fn into_data(self) -> Vec<u32> {
        self.data
    }

    fn points(&mut self, coords: impl IntoIterator<Item = MvtCoord>) -> MvtResult<()> {
        let start = self.data.len();
        self.data.push(0);
        let mut count = 0_u32;
        for coord in coords {
            self.push_delta(coord);
            count += 1;
        }
        if count == 0 {
            return Err(MvtError::InvalidGeometry);
        }
        self.data[start] = Command::MoveTo.encode(count)?;
        Ok(())
    }

    fn line(&mut self, line: &MvtLineString) -> MvtResult<()> {
        let coords = &line.0;
        if coords.is_empty() {
            return Err(MvtError::InvalidGeometry);
        }
        self.data.push(Command::MoveTo.encode(1)?);
        self.push_delta(coords[0]);
        if coords.len() > 1 {
            self.data
                .push(Command::LineTo.encode(u32_index(coords.len() - 1)?)?);
            for &coord in &coords[1..] {
                self.push_delta(coord);
            }
        }
        Ok(())
    }

    fn polygon(&mut self, polygon: &MvtPolygon) -> MvtResult<()> {
        self.ring(polygon.exterior(), true)?;
        for ring in polygon.interiors() {
            self.ring(ring, false)?;
        }
        Ok(())
    }

    fn ring(&mut self, ring: &MvtLineString, exterior: bool) -> MvtResult<()> {
        let coords = without_trailing_duplicate(&ring.0);
        if coords.is_empty() {
            return Err(MvtError::InvalidGeometry);
        }
        let area = signed_area(coords);
        let reverse = area != 0 && (area > 0) != exterior;
        self.data.push(Command::MoveTo.encode(1)?);
        let first = ring_coord(coords, 0, reverse);
        self.push_delta(first);
        if coords.len() > 1 {
            self.data
                .push(Command::LineTo.encode(u32_index(coords.len() - 1)?)?);
            for idx in 1..coords.len() {
                self.push_delta(ring_coord(coords, idx, reverse));
            }
        }
        self.data.push(Command::ClosePath.encode(1)?);
        Ok(())
    }

    fn push_delta(&mut self, coord: MvtCoord) {
        self.data
            .push(encode_parameter(coord.x.saturating_sub(self.cursor.x)));
        self.data
            .push(encode_parameter(coord.y.saturating_sub(self.cursor.y)));
        self.cursor = coord;
    }
}

#[cfg(feature = "writer")]
fn without_trailing_duplicate(coords: &[MvtCoord]) -> &[MvtCoord] {
    if coords.len() >= 2 && coords.first() == coords.last() {
        &coords[..coords.len() - 1]
    } else {
        coords
    }
}

#[cfg(feature = "writer")]
fn ring_coord(coords: &[MvtCoord], idx: usize, reverse: bool) -> MvtCoord {
    if reverse {
        coords[coords.len() - 1 - idx]
    } else {
        coords[idx]
    }
}

#[cfg(feature = "writer")]
fn u32_index(value: usize) -> MvtResult<u32> {
    u32::try_from(value).map_err(|_| MvtError::IndexOverflow(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_values_round_trip() {
        let move_to = Command::MoveTo.encode(1).unwrap();
        assert_eq!(move_to, 9);
        assert_eq!(Command::decode(move_to).unwrap(), (Command::MoveTo, 1));
        assert_eq!(Command::LineTo.encode(3).unwrap(), 26);
        assert_eq!(Command::ClosePath.encode(1).unwrap(), 15);
        assert!(matches!(
            Command::MoveTo.encode(0x2000_0000),
            Err(MvtError::CommandCount(0x2000_0000))
        ));
    }

    #[test]
    fn parameter_values_round_trip() {
        assert_eq!(encode_parameter(25), 50);
        assert_eq!(saturating_add_delta(0, 50), 25);
        assert_eq!(encode_parameter(-5), 9);
        assert_eq!(saturating_add_delta(0, 9), -5);
    }

    #[test]
    fn coordinate_overflow_saturates_at_i32_bounds() {
        assert_eq!(
            saturating_add_delta(i32::MAX, encode_parameter(1)),
            i32::MAX
        );
        assert_eq!(
            saturating_add_delta(i32::MIN, encode_parameter(-1)),
            i32::MIN
        );
    }
}

#[cfg(all(test, feature = "reader"))]
mod tests_reader {
    use geo_types::{Coord, Geometry};

    use super::*;

    #[test]
    fn parses_move_only_lines_and_polygons() {
        let Geometry::LineString(line) = parse_linestrings(&[9, 4, 6]).unwrap() else {
            panic!("expected line");
        };
        assert_eq!(line.0, vec![Coord { x: 2, y: 3 }]);

        let Geometry::MultiLineString(lines) = parse_linestrings(&[9, 4, 6, 9, 2, 2]).unwrap()
        else {
            panic!("expected multiline");
        };
        assert_eq!(lines.0.len(), 2);

        let Geometry::Polygon(poly) = parse_polygons(&[9, 0, 0, 15]).unwrap() else {
            panic!("expected polygon");
        };
        assert_eq!(poly.exterior().0, vec![(0, 0).into(), (0, 0).into()]);
    }

    #[test]
    fn direct_helper_edge_paths_are_covered() {
        let mut cursor = Coord { x: 0, y: 0 };
        let info = line_info(&[9, 0, 0, 15]).unwrap();
        assert!(matches!(
            parse_line_slice(&mut cursor, &[9, 0, 0, 15], info),
            Err(MvtError::InvalidGeometry)
        ));

        assert!(matches!(
            line_info(&[10, 0, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            line_info(&[17, 0, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            ring_info(&[9, 0, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            ring_info(&[9, 0, 0, 8]),
            Err(MvtError::InvalidGeometry)
        ));

        let Geometry::Polygon(poly) = parse_polygons(&[17, 0, 0, 10, 0, 15]).unwrap() else {
            panic!("expected polygon");
        };
        assert_eq!(poly.exterior().0.len(), 3);
    }

    #[test]
    fn invalid_geometry_streams_are_rejected() {
        assert!(matches!(
            decode_coord(&mut (0, 0).into(), &[1]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_points(&[Command::MoveTo.encode(0x1fff_ffff).unwrap(), 0, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(parse_points(&[]), Err(MvtError::InvalidGeometry)));
        assert!(matches!(
            parse_points(&[17, 1, 2]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_points(&[10, 0, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_linestrings(&[]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_linestrings(&[9, 0]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_linestrings(&[9, 0, 0, 18]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_polygons(&[]),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            parse_polygons(&[9, 0, 0, 8]),
            Err(MvtError::InvalidGeometry)
        ));
    }
}

#[cfg(all(test, feature = "writer"))]
mod tests_writer {
    use geo_types::{
        GeometryCollection, Line, LineString, MultiLineString, MultiPoint, MultiPolygon, Rect,
        Triangle,
    };

    use super::*;

    #[test]
    fn encodes_spec_point() {
        let geometry = MvtGeometry::Point((25, 17).into());
        let (_, data) = encode_geometry(&geometry).unwrap();
        assert_eq!(data, vec![9, 50, 34]);
    }

    #[test]
    fn encodes_spec_linestring() {
        let geometry = MvtGeometry::LineString(LineString(vec![
            (2, 2).into(),
            (2, 10).into(),
            (10, 10).into(),
        ]));
        let (_, data) = encode_geometry(&geometry).unwrap();
        assert_eq!(data, vec![9, 4, 4, 18, 0, 16, 16, 0]);
    }

    #[test]
    fn encodes_spec_polygon() {
        let geometry = MvtGeometry::Polygon(MvtPolygon::new(
            LineString(vec![(3, 6).into(), (8, 12).into(), (20, 34).into()]),
            vec![],
        ));
        let (_, data) = encode_geometry(&geometry).unwrap();
        assert_eq!(data, vec![9, 6, 12, 18, 10, 12, 24, 44, 15]);
    }

    #[test]
    fn encodes_empty_collections_and_geometry_collection_delegate() {
        assert_eq!(
            encode_geometry(&MvtGeometry::MultiPoint(MultiPoint(vec![]))).unwrap(),
            (GeomType::Point, Vec::new())
        );
        assert_eq!(
            encode_geometry(&MvtGeometry::MultiLineString(MultiLineString(vec![]))).unwrap(),
            (GeomType::Linestring, Vec::new())
        );
        assert_eq!(
            encode_geometry(&MvtGeometry::MultiPolygon(MultiPolygon(vec![]))).unwrap(),
            (GeomType::Polygon, Vec::new())
        );
        let collection =
            MvtGeometry::GeometryCollection(GeometryCollection(vec![MvtGeometry::Point(
                (1, 2).into(),
            )]));
        let direct = encode_geometry(&MvtGeometry::Point((1, 2).into())).unwrap();
        assert_eq!(encode_geometry(&collection).unwrap(), direct);
    }

    #[test]
    fn unsupported_and_invalid_geometries_are_errors() {
        let collection = MvtGeometry::GeometryCollection(GeometryCollection(vec![
            MvtGeometry::Point((0, 0).into()),
            MvtGeometry::Point((1, 1).into()),
        ]));
        assert!(matches!(
            encode_geometry(&collection),
            Err(MvtError::UnsupportedGeometry(_))
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::Line(Line::new((0, 0), (1, 1)))),
            Err(MvtError::UnsupportedGeometry("Line"))
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::Rect(Rect::new((0, 0), (1, 1)))),
            Err(MvtError::UnsupportedGeometry("Rect"))
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::Triangle(Triangle(
                (0, 0).into(),
                (1, 0).into(),
                (0, 1).into()
            ))),
            Err(MvtError::UnsupportedGeometry("Triangle"))
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::MultiPoint(MultiPoint(vec![(1, 1).into()]))),
            Ok((GeomType::Point, _))
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::LineString(LineString(vec![]))),
            Err(MvtError::InvalidGeometry)
        ));
        assert!(matches!(
            encode_geometry(&MvtGeometry::Polygon(MvtPolygon::new(
                LineString(vec![]),
                vec![]
            ))),
            Err(MvtError::InvalidGeometry)
        ));
        assert_eq!(signed_area(&[]), 0);
    }
}
