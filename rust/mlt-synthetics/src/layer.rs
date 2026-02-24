#![expect(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::{fs, io};

use geo_types::{Coord, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon};
use mlt_core::geojson::{Coord32, FeatureCollection, Geom32};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, IdEncoder, IdWidth, LogicalEncoder, OwnedGeometry,
    OwnedId, OwnedLayer01, OwnedProperty, PropValue, PropertyEncoder,
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

    /// Create a feature from a `geo_types` geometry with encoding configuration.
    pub fn from_geom(geom: impl Into<Geom32>, encoder: ValidatingGeometryEncoder) -> Self {
        let mut feat = default_feature();
        feat.geom.push_geom(&geom.into());
        feat.geometry_encoder = encoder;
        feat
    }

    /// Add another geometry to this feature.
    #[must_use]
    pub fn and(mut self, geom: impl Into<Geom32>, encoder: ValidatingGeometryEncoder) -> Self {
        self.geom.push_geom(&geom.into());
        self.geometry_encoder = self.geometry_encoder.merge(encoder);
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

fn default_feature() -> Feature {
    Feature {
        geom: DecodedGeometry::default(),
        props: vec![],
        ids: DecodedId(None),
        extent: None,
        ids_encoder: IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
        geometry_encoder: ValidatingGeometryEncoder::default(),
    }
}

// Helper functions to create geo_types geometries from coordinate arrays
pub fn point(x: i32, y: i32) -> Point<i32> {
    Point(Coord { x, y })
}

pub fn coord(x: i32, y: i32) -> Coord32 {
    Coord { x, y }
}

pub fn line(coords: &[[i32; 2]]) -> LineString<i32> {
    LineString(coords.iter().map(|[x, y]| Coord { x: *x, y: *y }).collect())
}

pub fn polygon(exterior: &[[i32; 2]]) -> Polygon<i32> {
    Polygon::new(line(exterior), vec![])
}

pub fn polygon_with_holes(exterior: &[[i32; 2]], holes: &[&[[i32; 2]]]) -> Polygon<i32> {
    Polygon::new(line(exterior), holes.iter().map(|h| line(h)).collect())
}

pub fn multi_point(coords: &[[i32; 2]]) -> MultiPoint<i32> {
    MultiPoint(coords.iter().map(|[x, y]| point(*x, *y)).collect())
}

pub fn multi_line(lines: &[&[[i32; 2]]]) -> MultiLineString<i32> {
    MultiLineString(lines.iter().map(|l| line(l)).collect())
}

pub fn multi_polygon(polygons: &[&[[i32; 2]]]) -> MultiPolygon<i32> {
    MultiPolygon(polygons.iter().map(|p| polygon(p)).collect())
}
