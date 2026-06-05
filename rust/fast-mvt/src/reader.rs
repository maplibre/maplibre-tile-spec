use std::collections::HashSet;
use std::num::NonZeroU32;

use buffa::{Enumeration as _, MessageView as _};
use usize_cast::IntoUsize;

use crate::generated::vector_tile::{TileView, tile as proto_tile};
use crate::geometry::decode_geometry;
use crate::types::{DEFAULT_EXTENT, MvtFeature, MvtLayer, MvtTile, MvtValue};
use crate::{MvtError, MvtGeometry, MvtResult};

#[derive(Debug, Clone)]
pub struct MvtReaderRef<'a>(TileView<'a>);

impl<'a> MvtReaderRef<'a> {
    pub fn new(data: &'a [u8]) -> MvtResult<Self> {
        let tile = TileView::decode_view(data)?;
        validate_layers(&tile)?;
        Ok(Self(tile))
    }

    #[must_use]
    pub fn layers(&self) -> impl ExactSizeIterator<Item = MvtLayerRef<'_>> {
        self.0.layers.iter().map(MvtLayerRef)
    }

    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.0.layers.len()
    }

    pub fn to_tile(&self) -> MvtResult<MvtTile> {
        let mut names = HashSet::with_capacity(self.0.layers.len());
        let mut layers = Vec::with_capacity(self.0.layers.len());
        for layer in self.layers() {
            if !names.insert(layer.name()) {
                return Err(MvtError::DuplicateLayer(layer.name().to_string()));
            }
            layers.push(layer.to_layer()?);
        }
        Ok(MvtTile { layers })
    }

    #[must_use]
    pub fn to_proto(&self) -> crate::generated::vector_tile::Tile {
        self.0.to_owned_message()
    }
}

fn validate_layers(tile: &TileView<'_>) -> MvtResult<()> {
    for layer in &tile.layers {
        if layer.name.is_empty() {
            return Err(MvtError::MissingLayerName);
        }
        if layer.version < 1 || layer.version > 3 {
            return Err(MvtError::UnsupportedVersion {
                layer: layer.name.to_string(),
                version: layer.version,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Copy, Clone)]
pub struct MvtLayerRef<'a>(&'a proto_tile::LayerView<'a>);

impl<'a> MvtLayerRef<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        self.0.name
    }

    #[must_use]
    pub fn version(self) -> u32 {
        self.0.version
    }

    #[must_use]
    pub fn extent(self) -> u32 {
        self.0.extent.unwrap_or(DEFAULT_EXTENT.get())
    }

    #[must_use]
    pub fn feature_count(self) -> usize {
        self.0.features.len()
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0.features.is_empty()
    }

    #[must_use]
    pub fn keys(self) -> &'a [&'a str] {
        &self.0.keys
    }

    #[must_use]
    pub fn values(self) -> impl ExactSizeIterator<Item = MvtValueRef<'a>> {
        self.0.values.iter().map(value_ref)
    }

    #[must_use]
    pub fn features(self) -> impl ExactSizeIterator<Item = MvtFeatureRef<'a>> {
        self.0.features.iter().map(move |feature| MvtFeatureRef {
            layer: self.0,
            feature,
        })
    }

    pub fn to_layer(self) -> MvtResult<MvtLayer> {
        let extent = match self.0.extent {
            Some(extent) => NonZeroU32::new(extent).ok_or(MvtError::InvalidExtent)?,
            None => DEFAULT_EXTENT,
        };
        self.features()
            .map(MvtFeatureRef::to_feature)
            .collect::<MvtResult<Vec<_>>>()
            .map(|features| MvtLayer {
                name: self.0.name.to_string(),
                extent,
                features,
            })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MvtFeatureRef<'a> {
    layer: &'a proto_tile::LayerView<'a>,
    feature: &'a proto_tile::FeatureView<'a>,
}

impl<'a> MvtFeatureRef<'a> {
    #[must_use]
    pub fn id(self) -> Option<u64> {
        self.feature.id
    }

    #[must_use]
    pub fn tags(self) -> &'a [u32] {
        &self.feature.tags
    }

    #[must_use]
    pub fn geometry_commands(self) -> &'a [u32] {
        &self.feature.geometry
    }

    #[must_use]
    pub fn geom_type(self) -> Option<proto_tile::GeomType> {
        self.feature.r#type
    }

    #[must_use]
    pub fn geom_type_value(self) -> Option<i32> {
        self.feature.r#type.map(|v| v.to_i32())
    }

    #[must_use]
    pub fn properties(self) -> MvtPropertyIter<'a> {
        MvtPropertyIter {
            keys: &self.layer.keys,
            values: &self.layer.values,
            tags: self.feature.tags.chunks(2),
        }
    }

    pub fn properties_vec(self) -> MvtResult<Vec<(&'a str, MvtValueRef<'a>)>> {
        self.properties().collect()
    }

    pub fn geometry(self) -> MvtResult<MvtGeometry> {
        decode_geometry(self.geom_type(), &self.feature.geometry)
    }

    pub fn to_feature(self) -> MvtResult<MvtFeature> {
        let properties = self
            .properties()
            .map(|property| property.map(|(key, value)| (key.to_string(), value.into_owned())))
            .collect::<MvtResult<Vec<_>>>()?;
        Ok(MvtFeature {
            id: self.id(),
            geometry: self.geometry()?,
            properties,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MvtValueRef<'a> {
    String(&'a str),
    Float(f32),
    Double(f64),
    Int(i64),
    UInt(u64),
    SInt(i64),
    Bool(bool),
    Null,
}

impl MvtValueRef<'_> {
    #[must_use]
    pub fn into_owned(self) -> MvtValue {
        match self {
            Self::String(value) => MvtValue::String(value.to_string()),
            Self::Float(value) => MvtValue::Float(value),
            Self::Double(value) => MvtValue::Double(value),
            Self::Int(value) => MvtValue::Int(value),
            Self::UInt(value) => MvtValue::UInt(value),
            Self::SInt(value) => MvtValue::SInt(value),
            Self::Bool(value) => MvtValue::Bool(value),
            Self::Null => MvtValue::Null,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MvtPropertyIter<'a> {
    keys: &'a [&'a str],
    values: &'a [proto_tile::ValueView<'a>],
    tags: std::slice::Chunks<'a, u32>,
}

impl<'a> Iterator for MvtPropertyIter<'a> {
    type Item = MvtResult<(&'a str, MvtValueRef<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let pair = self.tags.next()?;
        let [key_idx, value_idx] = pair else {
            return Some(Err(MvtError::InvalidTagsLength(pair.len())));
        };
        let key = match self.keys.get((*key_idx).into_usize()) {
            Some(key) => *key,
            None => return Some(Err(MvtError::InvalidKeyIndex(*key_idx))),
        };
        let value = match self.values.get((*value_idx).into_usize()) {
            Some(value) => value_ref(value),
            None => return Some(Err(MvtError::InvalidValueIndex(*value_idx))),
        };
        Some(Ok((key, value)))
    }
}

fn value_ref<'a>(value: &'a proto_tile::ValueView<'a>) -> MvtValueRef<'a> {
    if let Some(value) = value.string_value {
        MvtValueRef::String(value)
    } else if let Some(value) = value.float_value {
        MvtValueRef::Float(value)
    } else if let Some(value) = value.double_value {
        MvtValueRef::Double(value)
    } else if let Some(value) = value.int_value {
        MvtValueRef::Int(value)
    } else if let Some(value) = value.uint_value {
        MvtValueRef::UInt(value)
    } else if let Some(value) = value.sint_value {
        MvtValueRef::SInt(value)
    } else if let Some(value) = value.bool_value {
        MvtValueRef::Bool(value)
    } else {
        MvtValueRef::Null
    }
}

#[cfg(test)]
mod tests {
    use buffa::Message as _;
    use geo_types::{Geometry, MultiLineString, MultiPoint, MultiPolygon};

    use super::*;

    #[allow(clippy::disallowed_methods)]
    fn reader_from_layer(layer: proto_tile::Layer) -> MvtReaderRef<'static> {
        let bytes = crate::generated::vector_tile::Tile {
            layers: vec![layer],
        }
        .encode_to_vec();
        let bytes = Box::leak(bytes.into_boxed_slice());
        MvtReaderRef::new(bytes).unwrap()
    }

    #[test]
    fn borrowed_api_reads_accessors_properties_and_repeated_points() {
        let layer = proto_tile::Layer {
            version: 3,
            name: "places".to_string(),
            keys: vec![
                "string".into(),
                "float".into(),
                "double".into(),
                "int".into(),
                "uint".into(),
                "sint".into(),
                "bool".into(),
                "null".into(),
            ],
            values: vec![
                proto_tile::Value {
                    string_value: Some("name".into()),
                    ..Default::default()
                },
                proto_tile::Value {
                    float_value: Some(1.25),
                    ..Default::default()
                },
                proto_tile::Value {
                    double_value: Some(2.5),
                    ..Default::default()
                },
                proto_tile::Value {
                    int_value: Some(-3),
                    ..Default::default()
                },
                proto_tile::Value {
                    uint_value: Some(4),
                    ..Default::default()
                },
                proto_tile::Value {
                    sint_value: Some(-5),
                    ..Default::default()
                },
                proto_tile::Value {
                    bool_value: Some(true),
                    ..Default::default()
                },
                proto_tile::Value::default(),
            ],
            features: vec![proto_tile::Feature {
                id: Some(7),
                tags: (0_u32..8).flat_map(|idx| [idx, idx]).collect(),
                r#type: Some(proto_tile::GeomType::Point),
                geometry: vec![9, 2, 4, 9, 6, 8],
            }],
            ..Default::default()
        };
        let reader = reader_from_layer(layer);
        let layer = reader.layers().next().unwrap();

        assert_eq!(reader.layer_count(), 1);
        assert_eq!(layer.name(), "places");
        assert_eq!(layer.version(), 3);
        assert_eq!(layer.extent(), DEFAULT_EXTENT.get());
        assert_eq!(layer.feature_count(), 1);
        assert!(!layer.is_empty());
        assert_eq!(
            layer.keys(),
            [
                "string", "float", "double", "int", "uint", "sint", "bool", "null"
            ]
        );
        assert_eq!(layer.values().len(), 8);

        let feature = layer.features().next().unwrap();
        assert_eq!(feature.id(), Some(7));
        assert_eq!(
            feature.tags(),
            [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7]
        );
        assert_eq!(feature.geometry_commands(), [9, 2, 4, 9, 6, 8]);
        assert_eq!(feature.geom_type(), Some(proto_tile::GeomType::Point));
        assert_eq!(feature.geom_type_value(), Some(1));

        let properties = feature.properties_vec().unwrap();
        assert_eq!(properties[0].1, MvtValueRef::String("name"));
        assert_eq!(properties[1].1, MvtValueRef::Float(1.25));
        assert_eq!(properties[2].1, MvtValueRef::Double(2.5));
        assert_eq!(properties[3].1, MvtValueRef::Int(-3));
        assert_eq!(properties[4].1, MvtValueRef::UInt(4));
        assert_eq!(properties[5].1, MvtValueRef::SInt(-5));
        assert_eq!(properties[6].1, MvtValueRef::Bool(true));
        assert_eq!(properties[7].1, MvtValueRef::Null);
        assert_eq!(
            properties[0].1.into_owned(),
            MvtValue::String("name".into())
        );
        assert_eq!(properties[7].1.into_owned(), MvtValue::Null);

        let Geometry::MultiPoint(points) = feature.geometry().unwrap() else {
            panic!("expected multipoint");
        };
        assert_eq!(points.0.len(), 2);
        assert!(matches!(
            feature.geometry().unwrap(),
            Geometry::MultiPoint(_)
        ));
    }

    #[test]
    fn property_iterator_reports_malformed_tags() {
        let layer = proto_tile::Layer {
            version: 2,
            name: "tags".into(),
            keys: vec!["k".into()],
            values: vec![proto_tile::Value::default()],
            ..Default::default()
        };

        for (tags, expected) in [
            (vec![0], "invalid feature tags length: 1"),
            (vec![1, 0], "invalid key index 1"),
            (vec![0, 1], "invalid value index 1"),
        ] {
            let feature = proto_tile::Feature {
                tags,
                ..Default::default()
            };
            let mut layer = layer.clone();
            layer.features = vec![feature];
            let reader = reader_from_layer(layer);
            let feature = reader.layers().next().unwrap().features().next().unwrap();
            let err = feature.properties().next().unwrap().unwrap_err();
            assert_eq!(err.to_string(), expected);
        }
    }

    #[test]
    fn empty_geometries_keep_declared_type() {
        let layer = proto_tile::Layer {
            version: 2,
            name: "empty".into(),
            extent: Some(DEFAULT_EXTENT.get()),
            features: vec![],
            ..Default::default()
        };
        for (geom_type, expected) in [
            (
                proto_tile::GeomType::Point,
                Geometry::MultiPoint(MultiPoint(vec![])),
            ),
            (
                proto_tile::GeomType::Linestring,
                Geometry::MultiLineString(MultiLineString(vec![])),
            ),
            (
                proto_tile::GeomType::Polygon,
                Geometry::MultiPolygon(MultiPolygon(vec![])),
            ),
        ] {
            let feature = proto_tile::Feature {
                r#type: Some(geom_type),
                geometry: vec![],
                ..Default::default()
            };
            let mut layer = layer.clone();
            layer.features = vec![feature];
            let reader = reader_from_layer(layer);
            let feature = reader.layers().next().unwrap().features().next().unwrap();
            assert_eq!(feature.geometry().unwrap(), expected);
        }

        let feature = proto_tile::Feature {
            r#type: Some(proto_tile::GeomType::Unknown),
            geometry: vec![],
            ..Default::default()
        };
        let mut layer = layer.clone();
        layer.features = vec![feature];
        let reader = reader_from_layer(layer);
        let feature = reader.layers().next().unwrap().features().next().unwrap();
        assert!(matches!(feature.geometry(), Err(MvtError::InvalidGeometry)));
    }

    #[test]
    fn invalid_versions_and_geometry_types_are_errors() {
        let layer = proto_tile::Layer {
            version: 4,
            name: "bad".into(),
            ..Default::default()
        };
        let bytes = crate::generated::vector_tile::Tile {
            layers: vec![layer.clone()],
        }
        .encode_to_vec();
        assert!(matches!(
            MvtReaderRef::new(&bytes),
            Err(MvtError::UnsupportedVersion { version: 4, .. })
        ));

        let feature = proto_tile::Feature {
            r#type: Some(proto_tile::GeomType::Unknown),
            geometry: vec![9, 0, 0],
            ..Default::default()
        };
        let mut layer = proto_tile::Layer {
            version: 2,
            name: "geometry".into(),
            ..Default::default()
        };
        layer.features = vec![feature];
        let reader = reader_from_layer(layer);
        let feature = reader.layers().next().unwrap().features().next().unwrap();
        assert!(matches!(feature.geometry(), Err(MvtError::InvalidGeometry)));
    }
}
