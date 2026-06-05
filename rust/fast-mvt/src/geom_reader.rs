use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};

use crate::geom::{Command, signed_area};
use crate::proto::GeomType;
use crate::{MvtCoord, MvtError, MvtGeometry, MvtLineString, MvtResult};

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

fn decode_coord(cursor: &mut MvtCoord, values: &[u32]) -> MvtResult<MvtCoord> {
    let [dx, dy] = values else {
        return Err(MvtError::InvalidGeometry);
    };
    cursor.x = saturating_add_delta(cursor.x, *dx);
    cursor.y = saturating_add_delta(cursor.y, *dy);
    Ok(*cursor)
}

pub(crate) fn saturating_add_delta(value: i32, delta: u32) -> i32 {
    let v = (delta >> 1).cast_signed() ^ -(delta & 1).cast_signed();
    value.saturating_add(v)
}

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

#[derive(Debug, Copy, Clone)]
struct LineInfo {
    len: usize,
    coord_count: usize,
    line_count: usize,
}

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

fn parse_ring(cursor: &mut MvtCoord, data: &[u32]) -> MvtResult<(MvtLineString, i64, usize)> {
    let (info, len) = ring_info(data)?;
    let mut ring = parse_line_slice(cursor, &data[..info.len], info)?;
    let area = signed_area(&ring.0);
    let first = *ring.0.first().ok_or(MvtError::InvalidGeometry)?;
    ring.0.push(first);
    Ok((ring, area, len))
}

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

#[cfg(all(test, feature = "writer", feature = "reader"))]
mod tests {
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
