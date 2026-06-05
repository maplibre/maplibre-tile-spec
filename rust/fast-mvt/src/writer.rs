use std::collections::HashSet;

use buffa::Message as _;
use dup_indexer::{DupIndexer, DupIndexerRefs, PtrRead};
use usize_cast::IntoUsize;

use crate::generated::vector_tile::Tile;
use crate::generated::vector_tile::tile::{Feature, Layer, Value};
use crate::geometry::encode_geometry;
use crate::{MvtError, MvtFeature, MvtLayer, MvtResult, MvtTile, MvtValue};

pub fn encode_to_vec(tile: &MvtTile) -> MvtResult<Vec<u8>> {
    Ok(tile_to_proto(tile)?.encode_to_vec())
}

pub fn encode(tile: &MvtTile, out: &mut Vec<u8>) -> MvtResult<()> {
    tile_to_proto(tile)?.encode(out);
    Ok(())
}

pub(crate) fn encoded_len(tile: &MvtTile) -> MvtResult<usize> {
    Ok(tile_to_proto(tile)?.encoded_len().into_usize())
}

fn tile_to_proto(tile: &MvtTile) -> MvtResult<Tile> {
    let mut names = HashSet::with_capacity(tile.layers.len());
    let mut layers = Vec::with_capacity(tile.layers.len());
    for layer in &tile.layers {
        if !names.insert(layer.name.as_str()) {
            return Err(MvtError::DuplicateLayer(layer.name.clone()));
        }
        layers.push(layer_to_proto(layer)?);
    }
    Ok(Tile { layers })
}

fn layer_to_proto(layer: &MvtLayer) -> MvtResult<Layer> {
    let mut tags = TagsBuilder::new();
    let mut features = Vec::with_capacity(layer.features.len());
    for feature in &layer.features {
        features.push(feature_to_proto(feature, &mut tags)?);
    }
    let (keys, values) = tags.into_tags();
    Ok(Layer {
        version: 2,
        name: layer.name.clone(),
        features,
        keys,
        values,
        extent: Some(layer.extent.get()),
    })
}

fn feature_to_proto(feature: &MvtFeature, tags: &mut TagsBuilder) -> MvtResult<Feature> {
    let (geom_type, geometry) = encode_geometry(&feature.geometry)?;
    let mut proto = Feature {
        id: feature.id,
        tags: Vec::with_capacity(feature.properties.len().saturating_mul(2)),
        r#type: Some(geom_type),
        geometry,
    };
    for (key, value) in &feature.properties {
        if matches!(value, MvtValue::Null) {
            continue;
        }
        let (key_idx, value_idx) = tags.insert(key, value.clone())?;
        proto.tags.push(key_idx);
        proto.tags.push(value_idx);
    }
    Ok(proto)
}

struct TagsBuilder {
    keys: DupIndexerRefs<String>,
    values: DupIndexer<MvtValue>,
}

// This is safe because all `MvtValue` variants contain only `PtrRead` values
// (`String`, floats, integers, bools, or no payload).
unsafe impl PtrRead for MvtValue {}

impl TagsBuilder {
    fn new() -> Self {
        Self {
            keys: DupIndexerRefs::new(),
            values: DupIndexer::new(),
        }
    }

    /// Inserts the KV pair into the key and value dictionary and returns the index where it was inserted.
    fn insert(&mut self, key: &str, value: MvtValue) -> MvtResult<(u32, u32)> {
        let key_idx = u32_index(self.keys.insert_ref(key))?;
        let value_idx = self.values.insert(value);
        Ok((key_idx, u32_index(value_idx)?))
    }

    fn into_tags(self) -> (Vec<String>, Vec<Value>) {
        (
            self.keys.into_vec(),
            self.values.into_iter().map(value_to_proto).collect(),
        )
    }
}

fn value_to_proto(value: MvtValue) -> Value {
    match value {
        MvtValue::String(v) => Value::default().with_string_value(v),
        MvtValue::Float(v) => Value::default().with_float_value(v),
        MvtValue::Double(v) => Value::default().with_double_value(v),
        MvtValue::Int(v) => Value::default().with_int_value(v),
        MvtValue::UInt(v) => Value::default().with_uint_value(v),
        MvtValue::SInt(v) => Value::default().with_sint_value(v),
        MvtValue::Bool(v) => Value::default().with_bool_value(v),
        MvtValue::Null => Value::default(),
    }
}

fn u32_index(value: usize) -> MvtResult<u32> {
    u32::try_from(value).map_err(|_| MvtError::IndexOverflow(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_EXTENT, MvtGeometry};

    #[test]
    fn tag_builder_deduplicates_keys_and_values() {
        let mut tags = TagsBuilder::new();
        assert_eq!(
            tags.insert("foo", MvtValue::String("bar".into())).unwrap(),
            (0, 0)
        );
        assert_eq!(
            tags.insert("foo", MvtValue::String("baz".into())).unwrap(),
            (0, 1)
        );
        assert_eq!(
            tags.insert("bar", MvtValue::String("bar".into())).unwrap(),
            (1, 0)
        );
        assert_eq!(tags.insert("n", MvtValue::Int(1)).unwrap(), (2, 2));
        assert_eq!(tags.insert("n", MvtValue::SInt(1)).unwrap(), (2, 3));
        assert_eq!(tags.insert("f", MvtValue::Float(f32::NAN)).unwrap(), (3, 4));
        assert_eq!(tags.insert("f", MvtValue::Float(f32::NAN)).unwrap(), (3, 4));
    }

    #[test]
    fn encode_appends_and_validates_tile_metadata() {
        let tile = MvtTile {
            layers: vec![MvtLayer {
                name: "layer".into(),
                extent: DEFAULT_EXTENT,
                features: vec![MvtFeature {
                    id: Some(1),
                    geometry: MvtGeometry::Point((1, 2).into()),
                    properties: vec![("skip".into(), MvtValue::Null)],
                }],
            }],
        };
        let mut out = vec![0xaa];
        encode(&tile, &mut out).unwrap();
        assert_eq!(out[0], 0xaa);

        let proto = tile_to_proto(&tile).unwrap();
        assert!(proto.layers[0].keys.is_empty());
        assert!(proto.layers[0].features[0].tags.is_empty());

        let duplicate = MvtTile {
            layers: vec![
                MvtLayer {
                    name: "same".into(),
                    extent: DEFAULT_EXTENT,
                    features: vec![],
                },
                MvtLayer {
                    name: "same".into(),
                    extent: DEFAULT_EXTENT,
                    features: vec![],
                },
            ],
        };
        assert!(matches!(
            tile_to_proto(&duplicate),
            Err(MvtError::DuplicateLayer(name)) if name == "same"
        ));
    }

    #[test]
    fn value_to_proto_handles_all_variants() {
        assert_eq!(
            value_to_proto(MvtValue::String("x".into()))
                .string_value
                .as_deref(),
            Some("x")
        );
        assert_eq!(value_to_proto(MvtValue::Float(1.0)).float_value, Some(1.0));
        assert_eq!(
            value_to_proto(MvtValue::Double(2.0)).double_value,
            Some(2.0)
        );
        assert_eq!(value_to_proto(MvtValue::Int(-3)).int_value, Some(-3));
        assert_eq!(value_to_proto(MvtValue::UInt(4)).uint_value, Some(4));
        assert_eq!(value_to_proto(MvtValue::SInt(-5)).sint_value, Some(-5));
        assert_eq!(value_to_proto(MvtValue::Bool(true)).bool_value, Some(true));
        assert_eq!(value_to_proto(MvtValue::Null), Value::default());
    }
}
