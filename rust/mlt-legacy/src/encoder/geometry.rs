use geo_types::{Geometry, LineString, Polygon};

use crate::metadata::stream_encoding::PhysicalLevelTechnique;

pub struct GeometryScaling {
    pub extent: i32,
    pub min: f64,
    pub max: f64,
    pub scale: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}

pub struct Vertex {
    pub x: f64,
    pub y: f64,
}

pub struct EncodedGeometryColumn {
    pub num_streams: i32,
    pub encoded_values: Vec<u8>,
    pub max_vertex_value: i32,
    pub geometry_column_sorted: bool,
}

pub struct SortSettings {
    pub is_sortable: bool,
    pub feature_ids: Vec<i64>,
}

impl SortSettings {
    pub fn new(is_sortable: bool, feature_ids: Vec<i64>) -> Self {
        Self {
            is_sortable,
            feature_ids,
        }
    }
}

pub struct GeometryEncoder;

impl GeometryEncoder {
    pub fn encode_geometry_column(
        geometries: Vec<Geometry>,
        _physical_level_technique: PhysicalLevelTechnique,
        _sort_settings: Option<SortSettings>,
    ) -> Vec<u8> {
        let mut geometry_types: Vec<u8> = vec![];
        let mut num_geometries: Vec<i32> = vec![];
        let mut num_parts: Vec<i32> = vec![];
        let mut num_rings: Vec<i32> = vec![];
        let mut vertex_buffer: Vec<Vertex> = vec![];

        let contains_polygon = geometries
            .iter()
            .any(|g| matches!(g, Geometry::MultiPolygon(_) | Geometry::Polygon(_)));

        for geometry in geometries {
            match geometry {
                Geometry::Point(p) => {
                    geometry_types.push(GeometryType::Point as u8);
                    vertex_buffer.push(Vertex { x: p.x(), y: p.y() });
                }
                Geometry::MultiPoint(mp) => {
                    geometry_types.push(GeometryType::MultiPoint as u8);
                    num_geometries.push(mp.len() as i32);
                    mp.iter()
                        .for_each(|p| vertex_buffer.push(Vertex { x: p.x(), y: p.y() }));
                }
                Geometry::LineString(l) => {
                    geometry_types.push(GeometryType::LineString as u8);
                    add_linestring(
                        contains_polygon,
                        l.points().len() as i32,
                        &mut num_parts,
                        &mut num_rings,
                    );
                    let vertices = flat_linestring(&l);
                    vertex_buffer.extend(vertices);
                }
                Geometry::MultiLineString(ml) => {
                    geometry_types.push(GeometryType::MultiLineString as u8);
                    num_geometries.push(ml.iter().map(|l| l.points().len() as i32).sum());
                    ml.iter().for_each(|l| {
                        add_linestring(
                            contains_polygon,
                            l.points().len() as i32,
                            &mut num_parts,
                            &mut num_rings,
                        );
                        let vertices = flat_linestring(l);
                        vertex_buffer.extend(vertices);
                    });
                }
                Geometry::Polygon(p) => {
                    geometry_types.push(GeometryType::Polygon as u8);
                    let vertices = flat_polygon(&p, &mut num_parts, &mut num_rings);
                    vertex_buffer.extend(vertices);
                }
                Geometry::MultiPolygon(mp) => {
                    geometry_types.push(GeometryType::MultiPolygon as u8);
                    num_geometries
                        .push(mp.iter().map(|p| p.exterior().points().len() as i32).sum());
                    mp.iter().for_each(|p| {
                        let vertices = flat_polygon(p, &mut num_parts, &mut num_rings);
                        vertex_buffer.extend(vertices);
                    });
                }
                _ => (),
            }
        }

        let _min_vertex_value = vertex_buffer
            .iter()
            .map(|v| v.x.min(v.y))
            .fold(f64::INFINITY, f64::min);
        let _max_vertex_value = vertex_buffer
            .iter()
            .map(|v| v.x.max(v.y))
            .fold(f64::NEG_INFINITY, f64::max);

        Vec::new() // Dummy
    }
}

fn add_linestring(
    contains_polygon: bool,
    num_vertices: i32,
    num_parts: &mut Vec<i32>,
    num_rings: &mut Vec<i32>,
) {
    if contains_polygon {
        num_rings.push(num_vertices);
    } else {
        num_parts.push(num_vertices);
    }
}

fn flat_linestring(line_string: &LineString) -> Vec<Vertex> {
    line_string
        .points()
        .map(|v| Vertex { x: v.x(), y: v.y() })
        .collect()
}

fn flat_polygon(
    polygon: &Polygon,
    part_size: &mut Vec<i32>,
    ring_size: &mut Vec<i32>,
) -> Vec<Vertex> {
    let num_rings = polygon.interiors().len() + 1;
    part_size.push(num_rings as i32);

    let vertex_buffer = flat_linestring(polygon.exterior());
    ring_size.push(vertex_buffer.len() as i32);

    for interior in polygon.interiors() {
        let interior_vertices = flat_linestring(interior);
        ring_size.push(interior_vertices.len() as i32);
    }

    vertex_buffer
}
