#![expect(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::{fs, io};

use geo_types::{LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::geojson::{FeatureCollection, Geom32};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, Encoder, IdEncoder, IdWidth, LogicalEncoder,
    OwnedGeometry, OwnedId, OwnedLayer01, OwnedProperty, PropValue, PropertyEncoder,
};
use mlt_core::{Encodable as _, OwnedLayer, parse_layers};

use crate::geometry::ValidatingGeometryEncoder;

/// Tessellate a polygon using earcut algorithm.
/// Returns `(triangle_indices, num_triangles)`.
fn tessellate_polygon(polygon: &Polygon<i32>) -> (Vec<u32>, u32) {
    // Flatten coordinates without closing point (earcut expects open rings)
    let mut flat_coords: Vec<f64> = Vec::new();
    let mut hole_indices: Vec<usize> = Vec::new();

    // Add exterior ring (without closing point)
    let exterior = polygon.exterior();
    let exterior_coords: Vec<_> = exterior.coords().collect();
    let exterior_len =
        if exterior_coords.len() > 1 && exterior_coords.first() == exterior_coords.last() {
            exterior_coords.len() - 1
        } else {
            exterior_coords.len()
        };
    for coord in &exterior_coords[..exterior_len] {
        flat_coords.push(f64::from(coord.x));
        flat_coords.push(f64::from(coord.y));
    }

    // Add interior rings (holes)
    let mut vertex_count = exterior_len;
    for interior in polygon.interiors() {
        hole_indices.push(vertex_count);
        let interior_coords: Vec<_> = interior.coords().collect();
        let interior_len =
            if interior_coords.len() > 1 && interior_coords.first() == interior_coords.last() {
                interior_coords.len() - 1
            } else {
                interior_coords.len()
            };
        for coord in &interior_coords[..interior_len] {
            flat_coords.push(f64::from(coord.x));
            flat_coords.push(f64::from(coord.y));
        }
        vertex_count += interior_len;
    }

    // Run earcut
    let indices = earcutr::earcut(&flat_coords, &hole_indices, 2).expect("Tessellation failed");
    let num_triangles = u32::try_from(indices.len() / 3).expect("too many triangles");
    let indices_u32: Vec<u32> = indices
        .into_iter()
        .map(|i| u32::try_from(i).expect("index overflow"))
        .collect();

    (indices_u32, num_triangles)
}

/// A builder for creating synthetic MLT layers with multiple features.
/// This matches Java's approach where a layer contains multiple features,
/// each with a single geometry type.
#[derive(Debug, Clone)]
pub struct Layer {
    features: Vec<Feature>,
    extent: Option<u32>,
}

impl Layer {
    /// Create a layer with multiple features.
    pub fn new(features: Vec<Feature>) -> Self {
        Self {
            features,
            extent: None,
        }
    }

    /// Set the tile extent.
    #[must_use]
    pub fn extent(mut self, extent: u32) -> Self {
        self.extent = Some(extent);
        self
    }

    fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    /// Write the layer to an MLT file and a corresponding JSON file.
    pub fn write(&self, dir: &Path, name: &str) {
        let path = dir.join(format!("{name}.mlt"));
        self.write_mlt(&path);

        let buffer = fs::read(&path).unwrap();
        let mut data = parse_layers(&buffer).unwrap();
        for layer in &mut data {
            layer.decode_all().unwrap();
        }
        let fc = FeatureCollection::from_layers(&data).unwrap();
        let mut json = serde_json::to_string_pretty(&serde_json::to_value(&fc).unwrap()).unwrap();
        json.push('\n');
        let mut out_file = Self::open_new(&dir.join(format!("{name}.json"))).unwrap();
        out_file.write_all(json.as_bytes()).unwrap();
    }

    fn write_mlt(&self, path: &Path) {
        // Merge all feature geometries, properties, and IDs
        let mut merged_geom = DecodedGeometry::default();
        let merged_props: Vec<(DecodedProperty, PropertyEncoder)> = vec![];
        let mut merged_ids: Vec<Option<u64>> = vec![];
        let mut merged_geometry_encoder = ValidatingGeometryEncoder::default();
        let mut ids_encoder = IdEncoder::new(LogicalEncoder::None, IdWidth::Id32);
        let mut has_ids = false;

        for feat in &self.features {
            // Merge geometry by iterating through each feature's geometry
            let num_geoms = feat.geom.vector_types.len();
            for idx in 0..num_geoms {
                let geom = feat.geom.to_geojson(idx).unwrap();
                merged_geom.push_geom(&geom);
            }
            merged_geometry_encoder = merged_geometry_encoder.merge(feat.geometry_encoder);

            // Merge IDs
            if let Some(ids) = &feat.ids.0 {
                has_ids = true;
                merged_ids.extend(ids.iter().copied());
                ids_encoder = feat.ids_encoder;
            } else {
                merged_ids.push(None);
            }

            // TODO: Merge properties properly
        }

        let mut geometry = OwnedGeometry::Decoded(merged_geom);
        geometry
            .encode_with(Box::new(merged_geometry_encoder))
            .unwrap();

        // Sort properties alphabetically by name to match Java's output
        let mut props_sorted = merged_props;
        props_sorted.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));

        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(4096),
            id: if has_ids {
                let mut id = OwnedId::Decoded(DecodedId(Some(merged_ids)));
                id.encode_with(ids_encoder).unwrap();
                id
            } else {
                OwnedId::None
            },
            geometry,
            properties: props_sorted
                .into_iter()
                .map(|(p, e)| {
                    let mut p = OwnedProperty::Decoded(p);
                    p.encode_with(e).unwrap();
                    p
                })
                .collect::<Vec<_>>(),
        });

        let mut file = Self::open_new(path)
            .unwrap_or_else(|_| panic!("cannot create feature {}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|_| panic!("cannot encode feature {}", path.display()));
    }
}

/// A builder for creating synthetic MLT features with explicit control over encoding parameters.
#[derive(Debug, Clone)]
pub struct Feature {
    geom: DecodedGeometry,
    props: Vec<(DecodedProperty, PropertyEncoder)>,
    ids: DecodedId,
    extent: Option<u32>,

    ids_encoder: IdEncoder,
    geometry_encoder: ValidatingGeometryEncoder,
}

impl Default for Feature {
    fn default() -> Self {
        Self {
            geom: DecodedGeometry::default(),
            props: vec![],
            ids: DecodedId(None),
            extent: None,
            ids_encoder: IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
            geometry_encoder: ValidatingGeometryEncoder::default(),
        }
    }
}

impl Feature {
    fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    /// Write the feature to an MLT file and a corresponding JSON file.
    pub fn write(&self, dir: &Path, name: &str) {
        let path = dir.join(format!("{name}.mlt"));
        self.write_mlt(&path);

        let buffer = fs::read(&path).unwrap();
        let mut data = parse_layers(&buffer).unwrap();
        for layer in &mut data {
            layer.decode_all().unwrap();
        }
        let fc = FeatureCollection::from_layers(&data).unwrap();
        let mut json = serde_json::to_string_pretty(&serde_json::to_value(&fc).unwrap()).unwrap();
        json.push('\n');
        let mut out_file = Self::open_new(&dir.join(format!("{name}.json"))).unwrap();
        out_file.write_all(json.as_bytes()).unwrap();
    }

    /// Create a feature with a Point geometry.
    ///
    /// Parameters:
    /// - `geom`: The point geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    pub fn point(geom: Point<i32>, meta: Encoder, vertex: Encoder) -> Self {
        Self::default().and_point(geom, meta, vertex)
    }

    /// Add another Point geometry to this feature.
    #[must_use]
    pub fn and_point(mut self, geom: Point<i32>, meta: Encoder, vertex: Encoder) -> Self {
        self.geom.push_geom(&Geom32::Point(geom));
        self.geometry_encoder = self
            .geometry_encoder
            .merge(ValidatingGeometryEncoder::default().point(meta, vertex));
        self
    }

    /// Create a feature with a `LineString` geometry.
    ///
    /// Parameters:
    /// - `geom`: The linestring geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `parts`: Encoder for the parts length stream
    pub fn linestring(
        geom: LineString<i32>,
        meta: Encoder,
        vertex: Encoder,
        parts: Encoder,
    ) -> Self {
        Self::default().and_linestring(geom, meta, vertex, parts)
    }

    /// Add another `LineString` geometry to this feature.
    #[must_use]
    pub fn and_linestring(
        mut self,
        geom: LineString<i32>,
        meta: Encoder,
        vertex: Encoder,
        parts: Encoder,
    ) -> Self {
        self.geom.push_geom(&Geom32::LineString(geom));
        self.geometry_encoder = self
            .geometry_encoder
            .merge(ValidatingGeometryEncoder::default().linestring(meta, vertex, parts));
        self
    }

    /// Create a feature with a Polygon geometry.
    ///
    /// Parameters:
    /// - `geom`: The polygon geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `parts`: Encoder for the parts length stream
    /// - `rings`: Encoder for the rings length stream
    pub fn polygon(
        geom: Polygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        parts: Encoder,
        rings: Encoder,
    ) -> Self {
        Self::default().and_polygon(geom, meta, vertex, parts, rings)
    }

    /// Add another Polygon geometry to this feature.
    #[must_use]
    pub fn and_polygon(
        mut self,
        geom: Polygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        parts: Encoder,
        rings: Encoder,
    ) -> Self {
        self.geom.push_geom(&Geom32::Polygon(geom));
        self.geometry_encoder = self
            .geometry_encoder
            .merge(ValidatingGeometryEncoder::default().polygon(meta, vertex, parts, rings));
        self
    }

    /// Create a feature with a tessellated Polygon geometry.
    ///
    /// Parameters:
    /// - `geom`: The polygon geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `num_geometries`: Encoder for the geometry count stream (empty for single polygon)
    /// - `parts`: Encoder for the parts length stream
    /// - `rings`: Encoder for the rings length stream
    /// - `triangles`: Encoder for the triangles count stream
    /// - `triangles_indexes`: Encoder for the triangle index buffer
    #[expect(clippy::too_many_arguments)]
    pub fn polygon_tessellated(
        geom: Polygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
        rings: Encoder,
        triangles: Encoder,
        triangles_indexes: Encoder,
    ) -> Self {
        Self::default().and_polygon_tessellated(
            geom,
            meta,
            vertex,
            num_geometries,
            parts,
            rings,
            triangles,
            triangles_indexes,
        )
    }

    /// Add another tessellated Polygon geometry to this feature.
    #[must_use]
    #[expect(clippy::too_many_arguments)]
    pub fn and_polygon_tessellated(
        mut self,
        geom: Polygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
        rings: Encoder,
        triangles: Encoder,
        triangles_indexes: Encoder,
    ) -> Self {
        // Tessellate the polygon
        let (indices, num_triangles) = tessellate_polygon(&geom);

        // Push the polygon geometry
        self.geom.push_geom(&Geom32::Polygon(geom));

        // Store tessellation data
        let tris = self.geom.triangles.get_or_insert_with(Vec::new);
        tris.push(num_triangles);

        let idx_buf = self.geom.index_buffer.get_or_insert_with(Vec::new);
        idx_buf.extend(indices);

        self.geometry_encoder =
            self.geometry_encoder
                .merge(ValidatingGeometryEncoder::default().polygon_tessellated(
                    meta,
                    vertex,
                    num_geometries,
                    parts,
                    rings,
                    triangles,
                    triangles_indexes,
                ));
        self
    }

    /// Create a feature with a `MultiPoint` geometry.
    ///
    /// Parameters:
    /// - `geom`: The multipoint geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `num_geometries`: Encoder for the geometry count stream
    pub fn multi_point(
        geom: MultiPoint<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
    ) -> Self {
        Self::default().and_multi_point(geom, meta, vertex, num_geometries)
    }

    /// Add another `MultiPoint` geometry to this feature.
    #[must_use]
    pub fn and_multi_point(
        mut self,
        geom: MultiPoint<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
    ) -> Self {
        self.geom.push_geom(&Geom32::MultiPoint(geom));
        self.geometry_encoder = self
            .geometry_encoder
            .merge(ValidatingGeometryEncoder::default().multi_point(meta, vertex, num_geometries));
        self
    }

    /// Create a feature with a `MultiLineString` geometry.
    ///
    /// Parameters:
    /// - `geom`: The multi-linestring geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `num_geometries`: Encoder for the geometry count stream
    /// - `parts`: Encoder for the parts length stream (no rings)
    pub fn multi_linestring(
        geom: MultiLineString<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
    ) -> Self {
        Self::default().and_multi_linestring(geom, meta, vertex, num_geometries, parts)
    }

    /// Add another `MultiLineString` geometry to this feature.
    #[must_use]
    pub fn and_multi_linestring(
        mut self,
        geom: MultiLineString<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
    ) -> Self {
        self.geom.push_geom(&Geom32::MultiLineString(geom));
        self.geometry_encoder =
            self.geometry_encoder
                .merge(ValidatingGeometryEncoder::default().multi_linestring(
                    meta,
                    vertex,
                    num_geometries,
                    parts,
                ));
        self
    }

    /// Create a feature with a `MultiPolygon` geometry.
    ///
    /// Parameters:
    /// - `geom`: The multi-polygon geometry
    /// - `meta`: Encoder for the geometry type stream
    /// - `vertex`: Encoder for the vertex data stream
    /// - `num_geometries`: Encoder for the geometry count stream
    /// - `parts`: Encoder for the parts length stream
    /// - `rings`: Encoder for the rings length stream
    pub fn multi_polygon(
        geom: MultiPolygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
        rings: Encoder,
    ) -> Self {
        Self::default().and_multi_polygon(geom, meta, vertex, num_geometries, parts, rings)
    }

    /// Add another `MultiPolygon` geometry to this feature.
    #[must_use]
    pub fn and_multi_polygon(
        mut self,
        geom: MultiPolygon<i32>,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
        rings: Encoder,
    ) -> Self {
        self.geom.push_geom(&Geom32::MultiPolygon(geom));
        self.geometry_encoder =
            self.geometry_encoder
                .merge(ValidatingGeometryEncoder::default().multi_polygon(
                    meta,
                    vertex,
                    num_geometries,
                    parts,
                    rings,
                ));
        self
    }

    /// Set feature ID with encoding parameters.
    #[must_use]
    pub fn id(self, id: u64, logical: LogicalEncoder, id_width: IdWidth) -> Self {
        let ids_encoder = IdEncoder::new(logical, id_width);
        Self {
            ids: DecodedId(Some(vec![Some(id)])),
            ids_encoder,
            ..self
        }
    }

    /// Set multiple feature IDs with encoding parameters.
    #[must_use]
    pub fn ids(self, ids: Vec<Option<u64>>, ids_encoder: IdEncoder) -> Self {
        let ids = DecodedId(Some(ids));
        Self {
            ids,
            ids_encoder,
            ..self
        }
    }

    /// Add a property to this feature.
    #[must_use]
    pub fn prop(
        mut self,
        name: &impl ToString,
        values: PropValue,
        encoder: PropertyEncoder,
    ) -> Self {
        let name = name.to_string();
        self.props.push((DecodedProperty { name, values }, encoder));
        self
    }

    /// Add another property to this feature.
    #[must_use]
    pub fn and_prop(
        self,
        name: &impl ToString,
        values: PropValue,
        encoder: PropertyEncoder,
    ) -> Self {
        self.prop(name, values, encoder)
    }

    /// Add multiple properties to this feature.
    #[must_use]
    pub fn props(mut self, props: Vec<DecodedProperty>, encoder: PropertyEncoder) -> Self {
        self.props.extend(props.into_iter().map(|p| (p, encoder)));
        self
    }

    /// Set the tile extent.
    #[must_use]
    pub fn extent(self, extent: u32) -> Self {
        Self {
            extent: Some(extent),
            ..self
        }
    }

    fn write_mlt(&self, path: &Path) {
        let mut feat = self.clone();

        let mut geometry = OwnedGeometry::Decoded(feat.geom);
        geometry
            .encode_with(Box::new(self.geometry_encoder))
            .unwrap();

        // Sort properties alphabetically by name to match Java's output
        feat.props.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));

        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(4096),
            id: if self.ids.0.is_some() {
                let mut id = OwnedId::Decoded(feat.ids);
                id.encode_with(self.ids_encoder).unwrap();
                id
            } else {
                OwnedId::None
            },
            geometry,
            properties: feat
                .props
                .into_iter()
                .map(|(p, e)| {
                    let mut p = OwnedProperty::Decoded(p);
                    p.encode_with(e).unwrap();
                    p
                })
                .collect::<Vec<_>>(),
        });

        let mut file = Self::open_new(path)
            .unwrap_or_else(|e| panic!("cannot create feature {} because {e}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|e| panic!("cannot encode feature {} because {e}", path.display()));
    }
}
