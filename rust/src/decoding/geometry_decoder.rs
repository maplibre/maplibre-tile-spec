use crate::decoding::decoding_utils::DecodingUtils;
use crate::decoding::integer_decoder::IntegerDecoder;
use crate::decoding::stream_metadata_decoder::StreamMetadataDecoder;
use crate::types::compression_types::PhysicalLevelCompressionTechnique;
use crate::types::geometries::{CoordinateType, Geometry, GeometryType};
use crate::types::geometries::coordinate::Coordinate;
use crate::types::geometries::factory::GeometryFactory;
use crate::types::geometries::linearring::LinearRing;
use crate::types::geometry_column::GeometryColumn;
use crate::types::stream_types::{PhysicalStreamType, StreamType_Dictionary, StreamType_VariableSizedItems};

pub struct GeometryDecoder {}
impl GeometryDecoder {
    pub fn decode_geometry_column(data: &[u8], num_streams: usize, offset: &mut usize) -> GeometryColumn {
        let geometry_type_metadata = StreamMetadataDecoder::decode(data, offset);
        let geometry_types = IntegerDecoder::decode_int_stream(data, offset, &geometry_type_metadata, false);


        let mut num_geometries = Vec::new();
        let mut num_parts = Vec::new();
        let mut num_rings = Vec::new();
        let mut vertex_offsets = Some(Vec::new());
        let mut vertex_list = Vec::new();

        for i in 0..num_streams - 1 {
            let geometry_stream_metadata = StreamMetadataDecoder::decode(data, offset);
            match geometry_stream_metadata.stream_metadata.physical_stream_type {
                PhysicalStreamType::LENGTH => {
                    match geometry_stream_metadata.stream_metadata.logical_stream_type.LengthType.unwrap() {
                        StreamType_VariableSizedItems::GEOMETRIES => num_geometries = IntegerDecoder::decode_int_stream(data, offset, &geometry_stream_metadata, false),
                        StreamType_VariableSizedItems::PARTS => num_parts = IntegerDecoder::decode_int_stream(data, offset, &geometry_stream_metadata, false),
                        StreamType_VariableSizedItems::RINGS => num_rings = IntegerDecoder::decode_int_stream(data, offset, &geometry_stream_metadata, false),
                        StreamType_VariableSizedItems::TRIANGLES => panic!("Not implemented yet."),
                        _ => {}
                    }
                },
                PhysicalStreamType::OFFSET => {
                    vertex_offsets = Some(IntegerDecoder::decode_int_stream(data, offset, &geometry_stream_metadata, false));
                },
                PhysicalStreamType::DATA => {
                    if geometry_stream_metadata.stream_metadata.logical_stream_type.DictionaryType.unwrap()
                        == StreamType_Dictionary::VERTEX {
                        if geometry_stream_metadata.stream_metadata.physical_level_technique
                            == PhysicalLevelCompressionTechnique::FAST_PFOR {
                            let vertex_buffer = DecodingUtils::decode_fastpfor_delta_coordinates(
                                data,
                                offset,
                                geometry_stream_metadata.stream_metadata.num_values,
                                geometry_stream_metadata.stream_metadata.byte_length,
                            );
                            vertex_list = vertex_buffer
                                .iter()
                                .map(|i| *i as i32)
                                .collect();
                        } else {
                            vertex_list = IntegerDecoder::decode_int_stream(data, offset, &geometry_stream_metadata, true);
                        }
                    } else {
                        vertex_list = IntegerDecoder::decode_morton_stream(data, offset, &geometry_stream_metadata.morton_stream_metadata.unwrap());
                    }
                }
                PhysicalStreamType::PRESENT => { panic!("Present stream not allowed here"); },
            }
        }

        GeometryColumn {
            num_geometries,
            geometry_types,
            num_parts,
            num_rings,
            vertex_offsets,
            vertex_list,
        }
    }

    fn get_line_string(vertex_buffer: &[i32], start_index: usize, num_vertices: u32, close_line_string: bool) -> Vec<Coordinate> {
        let mut vertices = Vec::with_capacity(if close_line_string { num_vertices + 1 } else { num_vertices } as usize);

        for i in (0..num_vertices as usize * 2).step_by(2) {
            let x = vertex_buffer[start_index + i];
            let y = vertex_buffer[start_index + i + 1];
            vertices.push(Coordinate { x: x as CoordinateType, y: y as CoordinateType });
        }

        if close_line_string {
            vertices.push(vertices[0].clone());
        }

        vertices
    }


    fn decode_dictionary_encoded_line_string(
        vertex_buffer: &[i32],
        vertex_offsets: &[i32],
        vertex_offset: usize,
        num_vertices: u32,
        close_line_string: bool,
    ) -> Vec<Coordinate> {
        let mut vertices = Vec::with_capacity(if close_line_string { num_vertices + 1 } else { num_vertices } as usize);

        for i in 0..num_vertices as usize {
            let offset = vertex_offsets[vertex_offset + i] as usize * 2;
            let x = vertex_buffer[offset];
            let y = vertex_buffer[offset + 1];
            vertices.push(Coordinate { x: x as CoordinateType, y: y as CoordinateType });
        }

        if close_line_string {
            vertices.push(vertices[0].clone());
        }

        vertices
    }

    fn get_linear_ring(
        vertex_buffer: &[i32],
        start_index: usize,
        num_vertices: u32,
    ) -> LinearRing {
        let linear_ring = Self::get_line_string(vertex_buffer, start_index, num_vertices, true);
        GeometryFactory::create_linear_ring(linear_ring)
    }

    fn decode_dictionary_encoded_linear_ring(
        vertex_buffer: &[i32],
        vertex_offsets: &[i32],
        vertex_offset: usize,
        num_vertices: u32,
    ) -> LinearRing {
        let linear_ring = Self::decode_dictionary_encoded_line_string(vertex_buffer, vertex_offsets, vertex_offset, num_vertices, true);
        GeometryFactory::create_linear_ring(linear_ring)
    }

    pub fn decode_geometry(geometry_column: GeometryColumn) -> Vec<Geometry> {
        let mut geometries = Vec::with_capacity(geometry_column.geometry_types.len());

        let mut part_offset_counter = 0;
        let mut ring_offsets_counter = 0;
        let mut geometry_offsets_counter = 0;
        let mut vertex_buffer_offset = 0;
        let mut vertex_offsets_offset = 0;

        let geometry_types = &geometry_column.geometry_types;
        let geometry_offsets = geometry_column.num_geometries;
        let part_offsets = geometry_column.num_parts;
        let ring_offsets = geometry_column.num_rings;
        let vertex_offsets = geometry_column.vertex_offsets.as_ref().map(|v| v.clone()).unwrap_or_default();
        let vertex_buffer = geometry_column.vertex_list.clone();

        let contains_polygon = geometry_column.geometry_types.iter().any(|&g| {
            g == GeometryType::Polygon as i32 || g == GeometryType::MultiPolygon as i32
        });

        for geometry_type in geometry_types {
            match *geometry_type {
                g if g == GeometryType::Point as i32 => {
                    if vertex_offsets.is_empty() {
                        let x = vertex_buffer[vertex_buffer_offset];
                        vertex_buffer_offset += 1;
                        let y = vertex_buffer[vertex_buffer_offset];
                        vertex_buffer_offset += 1;
                        let coordinate = Coordinate { x: x as CoordinateType, y: y as CoordinateType };
                        geometries.push(Geometry::from(GeometryFactory::create_point(coordinate)));
                    } else {
                        let offset = vertex_offsets[vertex_offsets_offset] * 2;
                        vertex_offsets_offset += 1;
                        let x = vertex_buffer[offset as usize];
                        let y = vertex_buffer[offset as usize + 1];
                        let coordinate = Coordinate { x: x as CoordinateType, y: y as CoordinateType };
                        geometries.push(Geometry::from(GeometryFactory::create_point(coordinate)));
                    }
                },
                g if g == GeometryType::MultiPoint as i32 => {
                    let num_points = geometry_offsets[geometry_offsets_counter];
                    geometry_offsets_counter += 1;
                    let mut points = Vec::with_capacity(num_points as usize);
                    if vertex_offsets.is_empty() {
                        for _ in 0..num_points as usize {
                            let x = vertex_buffer[vertex_buffer_offset];
                            vertex_buffer_offset += 1;
                            let y = vertex_buffer[vertex_buffer_offset];
                            vertex_buffer_offset += 1;
                            let coordinate = Coordinate { x: x as CoordinateType, y: y as CoordinateType };
                            points.push(GeometryFactory::create_point(coordinate));
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_point(points)));
                    } else {
                        for _ in 0..num_points {
                            let offset = vertex_offsets[vertex_offsets_offset] * 2;
                            vertex_offsets_offset += 1;
                            let x = vertex_buffer[offset as usize];
                            let y = vertex_buffer[offset as usize + 1];
                            let coordinate = Coordinate { x: x as CoordinateType, y: y as CoordinateType };
                            points.push(GeometryFactory::create_point(coordinate));
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_point(points)));
                    }
                }
                g if g == GeometryType::Linestring as i32 => {
                    let num_vertices = if contains_polygon {
                        ring_offsets_counter += 1;
                        ring_offsets[ring_offsets_counter - 1]
                    } else {
                        part_offset_counter += 1;
                        part_offsets[part_offset_counter - 1]
                    };

                    if vertex_offsets.is_empty() {
                        let vertices = Self::get_line_string(&vertex_buffer, vertex_buffer_offset, num_vertices as u32, false);
                        vertex_buffer_offset += num_vertices as usize * 2;
                        geometries.push(Geometry::from(GeometryFactory::create_line_string(vertices)));
                    } else {
                        let vertices = Self::decode_dictionary_encoded_line_string(
                            &vertex_buffer, &vertex_offsets, vertex_offsets_offset, num_vertices as u32, false);
                        vertex_offsets_offset += num_vertices as usize;

                        geometries.push(Geometry::from(GeometryFactory::create_line_string(vertices)));
                    }
                }
                g if g == GeometryType::Polygon as i32 => {
                    let num_rings = part_offsets[part_offset_counter];
                    part_offset_counter += 1;
                    let mut rings = Vec::with_capacity(num_rings as usize - 1);
                    let mut num_vertices = ring_offsets[ring_offsets_counter];
                    ring_offsets_counter += 1;
                    if vertex_offsets.is_empty() {
                        let shell = Self::get_linear_ring(&vertex_buffer, vertex_buffer_offset, num_vertices as u32);
                        vertex_buffer_offset += num_vertices as usize * 2;
                        for _ in 0..num_rings as usize - 1 {
                            num_vertices = ring_offsets[ring_offsets_counter];
                            ring_offsets_counter += 1;
                            rings.push(
                                Self::get_linear_ring(&vertex_buffer, vertex_buffer_offset, num_vertices as u32));
                            vertex_buffer_offset += num_vertices as usize * 2;
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_polygon(shell, rings)));
                    } else {
                        let shell =
                            Self::decode_dictionary_encoded_linear_ring(&vertex_buffer, &*vertex_offsets, vertex_offsets_offset, num_vertices as u32);
                        vertex_offsets_offset += num_vertices as usize;
                        for _ in 0..num_rings as usize - 1 {
                            num_vertices = ring_offsets[ring_offsets_counter];
                            ring_offsets_counter += 1;
                            rings.push(
                                Self::decode_dictionary_encoded_linear_ring(&vertex_buffer, &vertex_offsets, vertex_offsets_offset, num_vertices as u32));
                            vertex_offsets_offset += num_vertices as usize;
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_polygon(shell, rings)));
                    }
                }
                g if g == GeometryType::MultiLineString as i32 => {
                    let num_line_strings = geometry_offsets[geometry_offsets_counter];
                    geometry_offsets_counter += 1;
                    let mut line_strings = Vec::with_capacity(num_line_strings as usize);
                    if vertex_offsets.is_empty() {
                        for _ in 0..num_line_strings {
                            let num_vertices = if contains_polygon {
                                ring_offsets_counter += 1;
                                ring_offsets[ring_offsets_counter - 1]
                            } else {
                                part_offset_counter += 1;
                                part_offsets[part_offset_counter - 1]
                            };
                            
                            let vertices = Self::get_line_string(&vertex_buffer, vertex_buffer_offset, num_vertices as u32, false);
                            line_strings.push(GeometryFactory::create_line_string(vertices));
                            vertex_buffer_offset += num_vertices as usize * 2;
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_line_string(line_strings)));
                    } else {
                        for _ in 0..num_line_strings {
                            let num_vertices =
                                if contains_polygon {
                                    ring_offsets_counter += 1;
                                    ring_offsets[ring_offsets_counter - 1]
                                } else {
                                    part_offset_counter += 1;
                                    part_offsets[part_offset_counter - 1]
                                };
                            let vertices = 
                                Self::decode_dictionary_encoded_line_string(
                                    &vertex_buffer, &vertex_offsets, vertex_offsets_offset, num_vertices as u32, false);
                            line_strings.push(GeometryFactory::create_line_string(vertices));
                            vertex_offsets_offset += num_vertices as usize;
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_line_string(line_strings)));
                    }
                }
                g if g == GeometryType::MultiPolygon as i32 => {
                    let num_polygons = geometry_offsets[geometry_offsets_counter];
                    geometry_offsets_counter += 1;
                    let mut polygons = Vec::with_capacity(num_polygons as usize);
                    let mut num_vertices;
                    if vertex_offsets.is_empty() {
                        for _ in 0..num_polygons {
                            let num_rings = part_offsets[part_offset_counter];
                            part_offset_counter += 1;
                            let mut rings = Vec::with_capacity(num_rings as usize - 1);
                            num_vertices = ring_offsets[ring_offsets_counter];
                            ring_offsets_counter += 1;
                            let shell =
                                Self::get_linear_ring(
                                    &vertex_buffer, vertex_buffer_offset, num_vertices as u32);
                            vertex_buffer_offset += num_vertices as usize * 2;
                            for j in 0..num_rings as usize - 1 {
                                let num_ring_vertices = ring_offsets[ring_offsets_counter];
                                ring_offsets_counter += 1;
                                rings.push(
                                    Self::get_linear_ring(
                                        &vertex_buffer, vertex_buffer_offset, num_ring_vertices as u32));
                                vertex_buffer_offset += num_ring_vertices as usize * 2;
                            }

                            polygons.push(GeometryFactory::create_polygon(shell, rings));
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_polygon(polygons)));
                    } else {
                        for _ in 0..num_polygons {
                            let num_rings = part_offsets[part_offset_counter];
                            part_offset_counter += 1;
                            let mut rings = Vec::with_capacity(num_rings as usize - 1);
                            num_vertices = ring_offsets[ring_offsets_counter];
                            ring_offsets_counter += 1;
                            let shell =
                                Self::decode_dictionary_encoded_linear_ring(
                                    &vertex_buffer, &vertex_offsets, vertex_offsets_offset, num_vertices as u32);
                            vertex_offsets_offset += num_vertices as usize;
                            for j in 0..num_rings as usize - 1 {
                                num_vertices = ring_offsets[ring_offsets_counter];
                                ring_offsets_counter += 1;
                                rings.push(
                                    Self::decode_dictionary_encoded_linear_ring(
                                        &vertex_buffer, &vertex_offsets, vertex_offsets_offset, num_vertices as u32));
                                vertex_offsets_offset += num_vertices as usize;
                            }
                            polygons.push(GeometryFactory::create_polygon(shell, rings));
                        }
                        geometries.push(Geometry::from(GeometryFactory::create_multi_polygon(polygons)));
                    }
                }
                _ => { panic!("specified geometry type is currently not supported: {}", geometry_type) }
            }
        }
        geometries
    }
}
