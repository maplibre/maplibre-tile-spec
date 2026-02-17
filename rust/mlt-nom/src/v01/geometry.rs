use std::fmt::Debug;

use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::geojson::Geometry as GeoGeom;
use crate::utils::{OptSeq, SetOptionOnce as _};
use crate::v01::{DictionaryType, LengthType, OffsetType, PhysicalStreamType, Stream};

/// Geometry column representation, either raw or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Geometry<'a> {
    Raw(RawGeometry<'a>),
    Decoded(DecodedGeometry),
}

impl Analyze for Geometry<'_> {
    fn decoded(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(g) => g.decoded(stat),
            Self::Decoded(g) => g.decoded(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Raw(g) => g.for_each_stream(cb),
            Self::Decoded(g) => g.for_each_stream(cb),
        }
    }
}

/// Unparsed geometry data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawGeometry<'a> {
    pub meta: Stream<'a>,
    pub items: Vec<Stream<'a>>,
}

impl Analyze for RawGeometry<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.meta.for_each_stream(cb);
        self.items.for_each_stream(cb);
    }
}

/// Decoded geometry data
#[derive(Clone, Default, PartialEq)]
pub struct DecodedGeometry {
    // pub vector_type: VectorType,
    // pub vertex_buffer_type: VertexBufferType,
    pub vector_types: Vec<GeometryType>,
    pub geometry_offsets: Option<Vec<u32>>,
    pub part_offsets: Option<Vec<u32>>,
    pub ring_offsets: Option<Vec<u32>>,
    pub vertex_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

impl Analyze for DecodedGeometry {
    fn decoded(&self, stat: StatType) -> usize {
        match stat {
            StatType::PayloadDataSizeBytes => {
                self.vector_types.decoded(stat)
                    + self.geometry_offsets.decoded(stat)
                    + self.part_offsets.decoded(stat)
                    + self.ring_offsets.decoded(stat)
                    + self.vertex_offsets.decoded(stat)
                    + self.index_buffer.decoded(stat)
                    + self.triangles.decoded(stat)
                    + self.vertices.decoded(stat)
            }
            StatType::MetadataOverheadBytes => 0,
            StatType::FeatureCount => self.vector_types.len(),
        }
    }
}

impl DecodedGeometry {
    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> Result<GeoGeom, MltError> {
        let err = |msg: &str| MltError::DecodeError(format!("geometry[{index}]: {msg}"));
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();

        let v = |idx: usize| [verts[idx * 2], verts[idx * 2 + 1]];
        let line = |start: usize, end: usize| (start..end).map(&v).collect();
        let closed_ring = |start: usize, end: usize| {
            let mut coords: Vec<[i32; 2]> = (start..end).map(&v).collect();
            coords.push(v(start));
            coords
        };

        let geom_type = *self
            .vector_types
            .get(index)
            .ok_or_else(|| err("index out of bounds"))?;

        match geom_type {
            GeometryType::Point => {
                let pt = match (geoms, parts, rings) {
                    (Some(g), Some(p), Some(r)) => v(r[p[g[index] as usize] as usize] as usize),
                    (Some(g), Some(p), None) => v(p[g[index] as usize] as usize),
                    (None, Some(p), Some(r)) => v(r[p[index] as usize] as usize),
                    (None, Some(p), None) => v(p[index] as usize),
                    (None, None, None) => v(index),
                    _ => return Err(err("unexpected offset combination for Point")),
                };
                Ok(GeoGeom::point(pt))
            }
            GeometryType::LineString => {
                let coords = match (parts, rings) {
                    (Some(p), Some(r)) => {
                        let i = p[index] as usize;
                        line(r[i] as usize, r[i + 1] as usize)
                    }
                    (Some(p), None) => line(p[index] as usize, p[index + 1] as usize),
                    _ => return Err(err("missing part_offsets for LineString")),
                };
                Ok(GeoGeom::line_string(coords))
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or_else(|| err("missing part_offsets for Polygon"))?;
                let rings = rings.ok_or_else(|| err("missing ring_offsets for Polygon"))?;
                let (ring_start, ring_end) = if let Some(g) = geoms {
                    let i = g[index] as usize;
                    (parts[i] as usize, parts[i + 1] as usize)
                } else {
                    (parts[index] as usize, parts[index + 1] as usize)
                };
                Ok(GeoGeom::polygon(
                    (ring_start..ring_end)
                        .map(|r| closed_ring(rings[r] as usize, rings[r + 1] as usize))
                        .collect(),
                ))
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or_else(|| err("missing geometry_offsets for MultiPoint"))?;
                Ok(GeoGeom::multi_point(
                    (geoms[index] as usize..geoms[index + 1] as usize)
                        .map(&v)
                        .collect(),
                ))
            }
            GeometryType::MultiLineString => {
                let geoms =
                    geoms.ok_or_else(|| err("missing geometry_offsets for MultiLineString"))?;
                let parts = parts.ok_or_else(|| err("missing part_offsets for MultiLineString"))?;
                Ok(GeoGeom::multi_line_string(
                    (geoms[index] as usize..geoms[index + 1] as usize)
                        .map(|p| line(parts[p] as usize, parts[p + 1] as usize))
                        .collect(),
                ))
            }
            GeometryType::MultiPolygon => {
                let geoms =
                    geoms.ok_or_else(|| err("missing geometry_offsets for MultiPolygon"))?;
                let parts = parts.ok_or_else(|| err("missing part_offsets for MultiPolygon"))?;
                let rings = rings.ok_or_else(|| err("missing ring_offsets for MultiPolygon"))?;
                Ok(GeoGeom::multi_polygon(
                    (geoms[index] as usize..geoms[index + 1] as usize)
                        .map(|p| {
                            let (rs, re) = (parts[p] as usize, parts[p + 1] as usize);
                            (rs..re)
                                .map(|r| closed_ring(rings[r] as usize, rings[r + 1] as usize))
                                .collect()
                        })
                        .collect(),
                ))
            }
        }
    }
}

/// Types of geometries supported in MLT
#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord, TryFromPrimitive, strum::Display,
)]
#[repr(u8)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}

impl Analyze for GeometryType {
    fn decoded(&self, _stat: StatType) -> usize {
        size_of::<Self>()
    }
}

// /// Vertex buffer type used for geometry columns
// #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
// pub enum VertexBufferType {
//     Morton,
//     Vec2,
//     Vec3,
// }

// #[derive(Debug, Clone, Copy, PartialEq)]
// pub enum VectorType {
//     Flat,
//     Const,
//     Sequence,
//     // Dictionary,
//     // FsstDictionary,
// }

impl_decodable!(Geometry<'a>, RawGeometry<'a>, DecodedGeometry);

impl<'a> From<RawGeometry<'a>> for Geometry<'a> {
    fn from(value: RawGeometry<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Geometry<'a> {
    #[must_use]
    pub fn raw(meta: Stream<'a>, items: Vec<Stream<'a>>) -> Self {
        Self::Raw(RawGeometry { meta, items })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedGeometry, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedGeometry::from_raw(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl Debug for DecodedGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedGeometry")
            .field("vector_types", &OptSeq(Some(&self.vector_types)))
            .field(
                "geometry_offsets",
                &OptSeq(self.geometry_offsets.as_deref()),
            )
            .field("part_offsets", &OptSeq(self.part_offsets.as_deref()))
            .field("ring_offsets", &OptSeq(self.ring_offsets.as_deref()))
            .field("vertex_offsets", &OptSeq(self.vertex_offsets.as_deref()))
            .field("index_buffer", &OptSeq(self.index_buffer.as_deref()))
            .field("triangles", &OptSeq(self.triangles.as_deref()))
            .field("vertices", &OptSeq(self.vertices.as_deref()))
            .finish()
    }
}

impl<'a> FromRaw<'a> for DecodedGeometry {
    type Input = RawGeometry<'a>;

    fn from_raw(RawGeometry { meta, items }: RawGeometry<'a>) -> Result<Self, MltError> {
        let vector_types = decode_geometry_types(meta)?;
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        for stream in items {
            match stream.meta.physical_type {
                PhysicalStreamType::Present => {}
                PhysicalStreamType::Data(v) => match v {
                    DictionaryType::Vertex => {
                        let v = stream.decode_bits_u32()?.decode_i32()?;
                        vertices.set_once(v)?;
                    }
                    v => Err(MltError::DecodeError(format!(
                        "Geometry stream cannot have Data physical type {v:?}"
                    )))?,
                },
                PhysicalStreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        v => Err(MltError::DecodeError(format!(
                            "Geometry stream cannot have Offset physical type {v:?}"
                        )))?,
                    };
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
                PhysicalStreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        v => Err(MltError::DecodeError(format!(
                            "Geometry stream cannot have Length physical type {v:?}"
                        )))?,
                    };
                    // LogicalStream2<U> -> LogicalStream -> trait LogicalStreamDecoder<T>
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
            }
        }

        if index_buffer.is_some() && part_offsets.is_none() {
            // Case when the indices of a Polygon outline are not encoded in the data so no
            // topology data are present in the tile
            //
            // return FlatGpuVector::new(vector_types, triangles, index_buffer, vertices);
            return Err(MltError::NotImplemented(
                "index_buffer.is_some() && part_offsets.is_none() case",
            ));
        }

        // Use decode_root_length_stream if geometry_offsets is present
        if let Some(offsets) = geometry_offsets.take() {
            geometry_offsets = Some(decode_root_length_stream(
                &vector_types,
                &offsets,
                GeometryType::Polygon,
            ));
            if let Some(part_offsets_copy) = part_offsets.take() {
                if let Some(ring_offsets_copy) = ring_offsets.take() {
                    part_offsets = Some(decode_level1_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                        false, // isLineStringPresent
                    ));
                    ring_offsets = Some(decode_level2_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        part_offsets.as_ref().unwrap(),
                        &ring_offsets_copy,
                    ));
                } else {
                    part_offsets = Some(decode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                    ));
                }
            }
        } else if let Some(offsets) = part_offsets.take() {
            if let Some(ring_offsets_copy) = ring_offsets.take() {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::LineString,
                ));
                ring_offsets = Some(decode_level1_length_stream(
                    &vector_types,
                    part_offsets.as_ref().unwrap(),
                    &ring_offsets_copy,
                    true, // isLineStringPresent
                ));
            } else {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::Point,
                ));
            }
        }

        // Case when the indices of a Polygon outline are encoded in the tile
        // This is handled by including index_buffer in the DecodedGeometry

        Ok(DecodedGeometry {
            // vertex_buffer_type: VertexBufferType::Vec2, // Morton not supported yet
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertex_offsets,
            index_buffer,
            triangles,
            vertices,
        })
    }
}

fn decode_geometry_types(meta: Stream) -> Result<Vec<GeometryType>, MltError> {
    // TODO: simplify this, e.g. use u8 or even GeometryType directly rather than going via Vec<u32>
    let vector_types: Vec<u32> = meta.decode_bits_u32()?.decode_u32()?;
    let vector_types: Vec<GeometryType> = vector_types
        .into_iter()
        .map::<Result<GeometryType, MltError>, _>(|v| Ok(u8::try_from(v)?.try_into()?))
        .collect::<Result<_, _>>()?;
    Ok(vector_types)
}
// pub fn get_vector_type_int_stream(metadata: &Stream) -> VectorType {
//     match metadata.stream {
//         StreamType::Rle => {
//             if metadata.data.len() == 1 {
//                 VectorType::Const
//             } else {
//                 VectorType::Flat
//             }
//         }
//         StreamType::DeltaRle if (1..=2).contains(&metadata.data.len()) => VectorType::Sequence,
//         _ => {
//             if metadata.num_values == 1 {
//                 VectorType::Const
//             } else {
//                 VectorType::Flat
//             }
//         }
//     }
// }

/// Handle the parsing of the different topology length buffers separate not generic to reduce the
/// branching and improve the performance
fn decode_root_length_stream(
    geometry_types: &[GeometryType],
    root_length_stream: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    let mut root_buffer_offsets = Vec::with_capacity(geometry_types.len() + 1);
    root_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut root_length_counter = 0_usize;
    for &geom_type in geometry_types {
        let offset = previous_offset
            + if geom_type > buffer_id {
                let val = root_length_stream[root_length_counter];
                root_length_counter += 1;
                val
            } else {
                1
            };
        root_buffer_offsets.push(offset);
        previous_offset = offset;
    }
    root_buffer_offsets
}

/// Case where no ring buffer exists so no `MultiPolygon` or `Polygon` geometry is part of the buffer
fn decode_level1_without_ring_buffer_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
) -> Vec<u32> {
    let final_size = root_offset_buffer[root_offset_buffer.len() - 1] as usize + 1;
    let mut level1_buffer_offsets = Vec::with_capacity(final_size);
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_offset_buffer_counter = 1_usize;
    let mut level1_length_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type == GeometryType::MultiLineString
            || geometry_type == GeometryType::LineString
        {
            // For MultiLineString and LineString a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_counter];
                level1_length_counter += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_offset_buffer_counter += 1;
            }
        } else {
            // For MultiPoint and Point no value in level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_offset_buffer_counter += 1;
            }
        }
    }

    level1_buffer_offsets
}

fn decode_level1_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_length_buffer: &[u32],
    is_line_string_present: bool,
) -> Vec<u32> {
    let final_size = root_offset_buffer[root_offset_buffer.len() - 1] as usize + 1;
    let mut level1_buffer_offsets = Vec::with_capacity(final_size);
    level1_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_buffer_counter = 1_usize;
    let mut level1_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type == GeometryType::MultiPolygon
            || geometry_type == GeometryType::Polygon
            || (is_line_string_present
                && (geometry_type == GeometryType::MultiLineString
                    || geometry_type == GeometryType::LineString))
        {
            // For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
            // a value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += level1_length_buffer[level1_length_buffer_counter];
                level1_length_buffer_counter += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_buffer_counter += 1;
            }
        } else {
            // For MultiPoint and Point and in some cases for MultiLineString and LineString
            // no value in the level1LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level1_buffer_offsets.push(previous_offset);
                level1_buffer_counter += 1;
            }
        }
    }

    level1_buffer_offsets
}

fn decode_level2_length_stream(
    geometry_types: &[GeometryType],
    root_offset_buffer: &[u32],
    level1_offset_buffer: &[u32],
    level2_length_buffer: &[u32],
) -> Vec<u32> {
    let final_size = level1_offset_buffer[level1_offset_buffer.len() - 1] as usize + 1;
    let mut level2_buffer_offsets = Vec::with_capacity(final_size);
    level2_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut level1_offset_buffer_counter = 1_usize;
    let mut level2_offset_buffer_counter = 1_usize;
    let mut level2_length_buffer_counter = 0_usize;

    for (i, &geometry_type) in geometry_types.iter().enumerate() {
        let num_geometries = (root_offset_buffer[i + 1] - root_offset_buffer[i]) as usize;

        if geometry_type != GeometryType::Point && geometry_type != GeometryType::MultiPoint {
            // For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
            // exists
            for _j in 0..num_geometries {
                let num_parts = (level1_offset_buffer[level1_offset_buffer_counter]
                    - level1_offset_buffer[level1_offset_buffer_counter - 1])
                    as usize;
                level1_offset_buffer_counter += 1;
                for _k in 0..num_parts {
                    previous_offset += level2_length_buffer[level2_length_buffer_counter];
                    level2_length_buffer_counter += 1;
                    level2_buffer_offsets.push(previous_offset);
                    level2_offset_buffer_counter += 1;
                }
            }
        } else {
            // For MultiPoint and Point no value in level2LengthBuffer exists
            for _j in 0..num_geometries {
                previous_offset += 1;
                level2_buffer_offsets.push(previous_offset);
                level2_offset_buffer_counter += 1;
                level1_offset_buffer_counter += 1;
            }
        }
    }

    level2_buffer_offsets
}
