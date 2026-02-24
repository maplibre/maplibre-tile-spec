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
    pub fn prop(self, name: &impl ToString, values: PropValue, encoder: PropertyEncoder) -> Self {
        let name = name.to_string();
        Self {
            props: vec![(DecodedProperty { name, values }, encoder)],
            ..self
        }
    }

    /// Add multiple properties to this feature.
    #[must_use]
    pub fn props(self, props: Vec<DecodedProperty>, encoder: PropertyEncoder) -> Self {
        Self {
            props: props.into_iter().map(|p| (p, encoder)).collect(),
            ..self
        }
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
        let feat = self.clone();

        let mut geometry = OwnedGeometry::Decoded(feat.geom);
        geometry
            .encode_with(Box::new(self.geometry_encoder))
            .unwrap();

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
            .unwrap_or_else(|_| panic!("cannot create feature {}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|_| panic!("cannot encode feature {}", path.display()));
    }
}
