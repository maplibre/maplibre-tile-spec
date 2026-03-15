#![expect(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use geo::{Convert as _, TriangulateEarcut as _};
use geo_types::{LineString, Polygon};
use mlt_core::geojson::{FeatureCollection, Geom32};
use mlt_core::v01::{
    EncodeProperties as _, EncodedLayer01, GeometryEncoder, GeometryValues, IdEncoder, IdValues,
    IntEncoder, PresenceStream, PropertyEncoder, ScalarEncoder, SharedDictEncoder,
    SharedDictItemEncoder, StagedProperty, StagedStrings, StrEncoder, VertexBufferType,
    build_staged_shared_dict,
};
use mlt_core::{Decoder, EncodedLayer, parse_layers};

/// Tessellate a polygon using the geo crate's earcut algorithm.
///
/// Geo's earcut includes the closing vertex in each ring; MLT (and Java's `earcut4j`) omit it.
/// We remap triangle indices so that any index referring to a ring's closing vertex is replaced
/// by that ring's start index, producing identical index buffers to Java.
fn tessellate_polygon(polygon: &Polygon<i32>) -> (Vec<u32>, u32) {
    // Convert i32 polygon to f64 for tessellation (geo's TriangulateEarcut requires CoordFloat)
    let polygon_f64: Polygon<f64> = polygon.convert();
    let raw = polygon_f64.earcut_triangles_raw();
    let num_triangles = u32::try_from(raw.triangle_indices.len() / 3).expect("too many triangles");

    // Build remap: geo index -> MLT index (closing vertex of each ring -> ring start).
    let mut geo_to_mlt = Vec::with_capacity(raw.vertices.len() / 2);
    let mut mlt_offset = 0;

    let mut push_ring = |ring: &LineString<i32>| {
        let len = ring.0.len();
        let mlt_len = if len > 1 && ring.0.first() == ring.0.last() {
            len - 1
        } else {
            len
        };
        for i in 0..len {
            geo_to_mlt.push(if i == len - 1 && mlt_len < len {
                mlt_offset
            } else {
                mlt_offset + i
            });
        }
        mlt_offset += mlt_len;
    };

    push_ring(polygon.exterior());
    for interior in polygon.interiors() {
        push_ring(interior);
    }

    let indices_u32: Vec<u32> = raw
        .triangle_indices
        .into_iter()
        .map(|i| {
            let mlt_idx = geo_to_mlt.get(i).copied().unwrap_or(i);
            u32::try_from(mlt_idx).expect("index overflow")
        })
        .collect();

    (indices_u32, num_triangles)
}

pub struct SynthWriter {
    dir: PathBuf,
}

impl SynthWriter {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    #[must_use]
    pub fn geo(&self, encoder: IntEncoder) -> Layer {
        Layer::new(self.dir.clone(), encoder)
    }

    /// Create a layer with all geometry encoders set to `VarInt`.
    #[must_use]
    pub fn geo_varint(&self) -> Layer {
        Layer::new(self.dir.clone(), IntEncoder::varint())
    }

    /// Create a layer with all geometry encoders set to `FastPFOR`.
    #[must_use]
    pub fn geo_fastpfor(&self) -> Layer {
        Layer::new(self.dir.clone(), IntEncoder::fastpfor())
    }
}

/// Layer builder: holds geometry encoder, geometry list, properties, extent, and IDs.
pub struct Layer {
    pub path: PathBuf,
    pub geometry_encoder: GeometryEncoder,
    pub geometry_items: Vec<Geom32>,
    /// Polygons that are also tessellated; triangle data is merged when building decoded geometry.
    pub tessellated_polygons: Vec<Option<Polygon<i32>>>,
    pub properties: Vec<StagedProperty>,
    pub prop_encoders: Vec<PropertyEncoder>,
    pub extent: Option<u32>,
    pub ids: Option<(Vec<Option<u64>>, IdEncoder)>,
}

impl Layer {
    #[must_use]
    pub fn new(path: PathBuf, default_geom_enc: IntEncoder) -> Layer {
        Layer {
            path,
            geometry_encoder: GeometryEncoder::all(default_geom_enc),
            geometry_items: vec![],
            tessellated_polygons: vec![],
            properties: vec![],
            prop_encoders: vec![],
            extent: None,
            ids: None,
        }
    }

    /// Set encoding for parts length stream when rings are present.
    #[must_use]
    pub fn rings(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.rings(e);
        self
    }

    /// Set encoding for ring vertex-count stream.
    #[must_use]
    pub fn rings2(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.rings2(e);
        self
    }

    /// Set encoding for the vertex data stream.
    #[must_use]
    pub fn vertex(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.vertex(e);
        self
    }

    /// Set encoding for the geometry types (meta) stream.
    #[must_use]
    pub fn meta(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.meta(e);
        self
    }

    /// Set encoding for the geometry length stream.
    #[must_use]
    pub fn geometries(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.geometries(e);
        self
    }

    /// Set encoding for parts length stream when rings are not present.
    #[must_use]
    pub fn no_rings(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.no_rings(e);
        self
    }

    /// Set encoding for parts length stream (with rings) when `geometry_offsets` absent.
    #[must_use]
    pub fn parts(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.parts(e);
        self
    }

    /// Set encoding for ring lengths when `geometry_offsets` absent.
    #[must_use]
    pub fn parts_ring(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.parts_ring(e);
        self
    }

    /// Set encoding for parts-only stream.
    #[must_use]
    pub fn only_parts(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.only_parts(e);
        self
    }

    /// Set encoding for triangles and triangle index buffer.
    #[must_use]
    pub fn triangles(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.triangles(e);
        self.geometry_encoder.triangles_indexes(e);
        self
    }

    /// Set encoding for vertex offsets.
    #[must_use]
    pub fn vertex_offsets(mut self, e: IntEncoder) -> Self {
        self.geometry_encoder.vertex_offsets(e);
        self
    }

    /// Set encoding of the vertex buffer.
    #[must_use]
    pub fn vertex_buffer_type(mut self, v: VertexBufferType) -> Self {
        self.geometry_encoder.vertex_buffer_type(v);
        self
    }

    /// Add a geometry (uses [`geo_types::Geometry`] `From` impls: `Point`, `LineString`, etc.).
    #[must_use]
    pub fn geo(mut self, geometry: impl Into<Geom32>) -> Self {
        self.geometry_items.push(geometry.into());
        self.tessellated_polygons.push(None);
        self
    }

    /// Add multiple geometries
    #[must_use]
    pub fn geos<T: Into<Geom32>, I: IntoIterator<Item = T>>(mut self, geometries: I) -> Self {
        for g in geometries {
            self = self.geo(g.into());
        }
        self
    }

    /// Add a tessellated polygon (polygon + triangle mesh).
    #[must_use]
    pub fn tessellated(mut self, polygon: Polygon<i32>) -> Self {
        self.geometry_items.push(Geom32::Polygon(polygon.clone()));
        self.tessellated_polygons.push(Some(polygon));
        self
    }

    /// Add a scalar property.
    #[must_use]
    pub fn add_prop(mut self, encoder: ScalarEncoder, prop: StagedProperty) -> Self {
        self.properties.push(prop);
        self.prop_encoders.push(PropertyEncoder::Scalar(encoder));
        self
    }

    /// Add a shared dictionary with its child columns.
    ///
    /// Use [`SharedDict::new`] to create the builder, then add columns with
    /// [`SharedDict::column`], and pass it to this method.
    #[must_use]
    pub fn add_shared_dict(mut self, shared_dict: SharedDict) -> Self {
        let name = shared_dict.name;
        let encoder = shared_dict.encoder;
        let dict = build_staged_shared_dict(name, shared_dict.items)
            .expect("shared dict builder should be valid");
        self.properties.push(StagedProperty::SharedDict(dict));
        self.prop_encoders.push(encoder.into());
        self
    }

    /// Set the tile extent.
    #[must_use]
    pub fn extent(mut self, extent: u32) -> Self {
        self.extent = Some(extent);
        self
    }

    /// Set feature IDs.
    #[must_use]
    pub fn ids(mut self, ids: Vec<Option<u64>>, encoder: IdEncoder) -> Self {
        self.ids = Some((ids, encoder));
        self
    }

    fn build_decoded_geometry(&self) -> GeometryValues {
        let mut geom = GeometryValues::default();
        for g in &self.geometry_items {
            geom.push_geom(g);
        }
        for poly in &self.tessellated_polygons {
            let Some(poly) = poly else { continue };
            let (indices, num_triangles) = tessellate_polygon(poly);
            geom.triangles
                .get_or_insert_with(Vec::new)
                .push(num_triangles);
            geom.index_buffer
                .get_or_insert_with(Vec::new)
                .extend(indices);
        }
        geom
    }

    fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    /// Write the layer to an MLT file and a corresponding JSON file (consumes self).
    pub fn write(self, name: impl AsRef<str>) {
        let name = name.as_ref();
        let dir = self.path.clone();
        let path = dir.join(format!("{name}.mlt"));
        self.write_mlt(&path);

        let buffer = fs::read(&path).unwrap();
        let mut dec = Decoder::default();
        let mut data = parse_layers(&buffer, &mut dec).unwrap();
        let fc = FeatureCollection::from_layers(&mut data, &mut dec).unwrap();
        let mut json = serde_json::to_string_pretty(&fc).unwrap();
        json.push('\n');
        let mut out_file = Self::open_new(&dir.join(format!("{name}.json"))).unwrap();
        out_file.write_all(json.as_bytes()).unwrap();
    }

    fn write_mlt(self, path: &Path) {
        let geometry = self.build_decoded_geometry();
        let encoded_geometry = geometry
            .encode(self.geometry_encoder)
            .unwrap_or_else(|e| panic!("cannot encode geometry: {e}"));

        let id = if let Some((ids, ids_encoder)) = self.ids {
            IdValues(ids)
                .encode(ids_encoder)
                .unwrap_or_else(|e| panic!("cannot encode id: {e}"))
        } else {
            None
        };

        let encoded_properties = self
            .properties
            .encode(self.prop_encoders)
            .unwrap_or_else(|e| panic!("cannot encode properties: {e}"));

        let layer = EncodedLayer::Tag01(EncodedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(80),
            id,
            geometry: encoded_geometry,
            properties: encoded_properties,
        });

        let mut file = Self::open_new(path)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|e| panic!("cannot encode {}: {e}", path.display()));
    }
}

/// Builder for a shared dictionary struct column with multiple string sub-properties.
///
/// Use [`SharedDict::new`] to create the builder, add columns with [`SharedDict::column`],
/// then pass it to [`Layer::add_shared_dict`].
pub struct SharedDict {
    name: String,
    encoder: SharedDictEncoder,
    items: Vec<(String, StagedStrings)>,
}

impl SharedDict {
    /// Create a new shared dictionary builder.
    ///
    /// # Arguments
    /// * `name` - The name for the property (e.g., `"name:"` for `"name:de"`, `"name:en"`).
    /// * `dict_encoder` - The string encoder for the shared dictionary (plain or FSST).
    #[must_use]
    pub fn new(name: impl Into<String>, dict_encoder: StrEncoder) -> Self {
        Self {
            name: name.into(),
            encoder: SharedDictEncoder {
                dict_encoder,
                items: vec![],
            },
            items: vec![],
        }
    }

    /// Add a child column to the shared dictionary.
    ///
    /// # Arguments
    /// * `suffix` - The suffix name for this child (e.g., `"de"` for `"name:de"`).
    /// * `optional` - Whether to include a presence stream for null values.
    /// * `offset` - The integer encoder for the offset-index stream.
    /// * `values` - The string values for each feature.
    #[must_use]
    pub fn column(
        mut self,
        suffix: impl Into<String>,
        presence: PresenceStream,
        offsets: IntEncoder,
        values: impl IntoIterator<Item = Option<String>>,
    ) -> Self {
        let enc = SharedDictItemEncoder { presence, offsets };
        self.encoder.items.push(enc);
        let suffix = suffix.into();
        let values: Vec<Option<String>> = values.into_iter().collect();
        self.items.push((suffix, StagedStrings::from(values)));
        self
    }
}
