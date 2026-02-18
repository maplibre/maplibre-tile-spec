mod decode;

use std::fmt::Debug;
use std::io::Write;
use std::ops::Range;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;
use num_enum::TryFromPrimitive;

use crate::MltError;
use crate::MltError::{
    GeometryIndexOutOfBounds, GeometryOutOfBounds, GeometryVertexOutOfBounds, IntegerOverflow,
    NoGeometryOffsets, NoPartOffsets, NoRingOffsets, NotImplemented, UnexpectedOffsetCombination,
};
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::geojson::Geometry as GeoGeom;
use crate::utils::{BinarySerializer as _, OptSeq, SetOptionOnce as _};
use crate::v01::column::ColumnType;
use crate::v01::geometry::decode::{
    decode_geometry_types, decode_level1_length_stream,
    decode_level1_without_ring_buffer_length_stream, decode_level2_length_stream,
    decode_root_length_stream,
};
use crate::v01::{DictionaryType, LengthType, OffsetType, PhysicalStreamType, Stream};

/// Geometry column representation, either raw or decoded
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Geometry<'a> {
    Raw(RawGeometry<'a>),
    Decoded(DecodedGeometry),
}

impl Analyze for Geometry<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(g) => g.collect_statistic(stat),
            Self::Decoded(g) => g.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Raw(g) => g.for_each_stream(cb),
            Self::Decoded(g) => g.for_each_stream(cb),
        }
    }
}

impl OwnedGeometry {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Raw(_) => OwnedRawGeometry::write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Raw(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
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

impl OwnedRawGeometry {
    pub(crate) fn write_columns_meta_to<W: Write>(writer: &mut W) -> Result<(), MltError> {
        ColumnType::Geometry.write_to(writer)?;
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let items_len = u64::try_from(self.items.len()).map_err(|_| IntegerOverflow)?;
        let items_len = items_len.checked_add(1).ok_or(IntegerOverflow)?;
        writer.write_varint(items_len)?;
        writer.write_stream(&self.meta)?;
        for item in &self.items {
            writer.write_stream(item)?;
        }
        Ok(())
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
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedDataSize => {
                self.vector_types.collect_statistic(stat)
                    + self.geometry_offsets.collect_statistic(stat)
                    + self.part_offsets.collect_statistic(stat)
                    + self.ring_offsets.collect_statistic(stat)
                    + self.vertex_offsets.collect_statistic(stat)
                    + self.index_buffer.collect_statistic(stat)
                    + self.triangles.collect_statistic(stat)
                    + self.vertices.collect_statistic(stat)
            }
            StatType::DecodedMetaSize => 0,
            StatType::FeatureCount => self.vector_types.len(),
        }
    }
}

impl DecodedGeometry {
    /// Build a `GeoJSON` geometry for a single feature at index `i`.
    /// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec
    /// (MLT omits the closing vertex).
    pub fn to_geojson(&self, index: usize) -> Result<GeoGeom, MltError> {
        let verts = self.vertices.as_deref().unwrap_or(&[]);
        let geoms = self.geometry_offsets.as_deref();
        let parts = self.part_offsets.as_deref();
        let rings = self.ring_offsets.as_deref();
        let vo = self.vertex_offsets.as_deref();
        let num_verts = verts.len() / 2;

        let off = |s: &[u32], idx: usize, field: &'static str| -> Result<usize, MltError> {
            match s.get(idx) {
                Some(&v) => Ok(v as usize),
                None => Err(GeometryOutOfBounds {
                    index,
                    field,
                    idx,
                    len: s.len(),
                }),
            }
        };
        let geom_off = |s: &[u32], idx: usize| off(s, idx, "geometry_offsets");
        let part_off = |s: &[u32], idx: usize| off(s, idx, "part_offsets");
        let ring_off = |s: &[u32], idx: usize| off(s, idx, "ring_offsets");
        let geom_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(geom_off(s, i)?..geom_off(s, i + 1)?)
        };
        let part_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(part_off(s, i)?..part_off(s, i + 1)?)
        };
        let ring_off_pair = |s: &[u32], i: usize| -> Result<Range<usize>, MltError> {
            Ok(ring_off(s, i)?..ring_off(s, i + 1)?)
        };

        let v = |idx: usize| -> Result<[i32; 2], MltError> {
            let vertex = match vo {
                Some(vo) => *vo.get(idx).ok_or(GeometryOutOfBounds {
                    index,
                    field: "vertex_offsets",
                    idx,
                    len: vo.len(),
                })? as usize,
                None => idx,
            };
            let i = vertex * 2;
            let s = verts.get(i..i + 2).ok_or(GeometryVertexOutOfBounds {
                index,
                vertex,
                count: num_verts,
            })?;
            Ok([s[0], s[1]])
        };
        let line = |r: Range<usize>| -> Result<Vec<[i32; 2]>, MltError> { r.map(&v).collect() };
        let closed_ring = |r: Range<usize>| -> Result<Vec<[i32; 2]>, MltError> {
            let start = r.start;
            let mut coords: Vec<[i32; 2]> = r.map(&v).collect::<Result<_, _>>()?;
            coords.push(v(start)?);
            Ok(coords)
        };
        let rings_in = |part_range: Range<usize>, rings: &[u32]| {
            part_range
                .map(|r| closed_ring(ring_off_pair(rings, r)?))
                .collect::<Result<_, _>>()
        };

        let geom_type = *self
            .vector_types
            .get(index)
            .ok_or(GeometryIndexOutOfBounds { index })?;

        match geom_type {
            GeometryType::Point => {
                let pt = match (geoms, parts, rings) {
                    (Some(g), Some(p), Some(r)) => {
                        v(ring_off(r, part_off(p, geom_off(g, index)?)?)?)?
                    }
                    (Some(g), Some(p), None) => v(part_off(p, geom_off(g, index)?)?)?,
                    (None, Some(p), Some(r)) => v(ring_off(r, part_off(p, index)?)?)?,
                    (None, Some(p), None) => v(part_off(p, index)?)?,
                    (None, None, None) => v(index)?,
                    _ => {
                        return Err(UnexpectedOffsetCombination { index, geom_type });
                    }
                };
                Ok(GeoGeom::point(pt))
            }
            GeometryType::LineString => {
                let r = match (parts, rings) {
                    (Some(p), Some(r)) => ring_off_pair(r, part_off(p, index)?)?,
                    (Some(p), None) => part_off_pair(p, index)?,
                    _ => return Err(NoPartOffsets { index, geom_type }),
                };
                line(r).map(GeoGeom::line_string)
            }
            GeometryType::Polygon => {
                let parts = parts.ok_or(NoPartOffsets { index, geom_type })?;
                let rings = rings.ok_or(NoRingOffsets { index, geom_type })?;
                let i = geoms
                    .map(|g| geom_off(g, index))
                    .transpose()?
                    .unwrap_or(index);
                rings_in(part_off_pair(parts, i)?, rings).map(GeoGeom::polygon)
            }
            GeometryType::MultiPoint => {
                let geoms = geoms.ok_or(NoGeometryOffsets { index, geom_type })?;
                geom_off_pair(geoms, index)?
                    .map(&v)
                    .collect::<Result<_, _>>()
                    .map(GeoGeom::multi_point)
            }
            GeometryType::MultiLineString => {
                let geoms = geoms.ok_or(NoGeometryOffsets { index, geom_type })?;
                let parts = parts.ok_or(NoPartOffsets { index, geom_type })?;
                geom_off_pair(geoms, index)?
                    .map(|p| line(part_off_pair(parts, p)?))
                    .collect::<Result<_, _>>()
                    .map(GeoGeom::multi_line_string)
            }
            GeometryType::MultiPolygon => {
                let geoms = geoms.ok_or(NoGeometryOffsets { index, geom_type })?;
                let parts = parts.ok_or(NoPartOffsets { index, geom_type })?;
                let rings = rings.ok_or(NoRingOffsets { index, geom_type })?;
                geom_off_pair(geoms, index)?
                    .map(|p| rings_in(part_off_pair(parts, p)?, rings))
                    .collect::<Result<_, _>>()
                    .map(GeoGeom::multi_polygon)
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
    fn collect_statistic(&self, _stat: StatType) -> usize {
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
                    _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
                },
                PhysicalStreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
                    };
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
                PhysicalStreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.physical_type))?,
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
            return Err(NotImplemented(
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
