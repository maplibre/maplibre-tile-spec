use crate::geom::{Command, signed_area};
use crate::proto::GeomType;
use crate::{MvtCoord, MvtError, MvtGeometry, MvtLineString, MvtPolygon, MvtResult};

pub(crate) fn encode_parameter(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)).cast_unsigned()
}

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

fn polygon_vertex_count(polygon: &MvtPolygon) -> usize {
    polygon.exterior().0.len()
        + polygon
            .interiors()
            .iter()
            .map(|line| line.0.len())
            .sum::<usize>()
}

struct GeometryEncoder {
    data: Vec<u32>,
    cursor: MvtCoord,
}

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

fn without_trailing_duplicate(coords: &[MvtCoord]) -> &[MvtCoord] {
    if coords.len() >= 2 && coords.first() == coords.last() {
        &coords[..coords.len() - 1]
    } else {
        coords
    }
}

fn ring_coord(coords: &[MvtCoord], idx: usize, reverse: bool) -> MvtCoord {
    if reverse {
        coords[coords.len() - 1 - idx]
    } else {
        coords[idx]
    }
}

fn u32_index(value: usize) -> MvtResult<u32> {
    u32::try_from(value).map_err(|_| MvtError::IndexOverflow(value))
}

#[cfg(test)]
mod tests {
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
