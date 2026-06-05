use buffa::Message as _;
use dup_indexer::{DupIndexer, DupIndexerRefs, PtrRead};
use usize_cast::IntoUsize;

use crate::generated::vector_tile::Tile;
use crate::generated::vector_tile::tile::{Feature, Layer, Value};
use crate::geom_writer::encode_geometry;
use crate::{DEFAULT_EXTENT, MvtError, MvtExtent, MvtGeometry, MvtResult, MvtTile, MvtValue};

#[derive(Debug, Default)]
pub struct MvtTileBuilder(Tile);

impl MvtTileBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(layers: usize) -> Self {
        Self(Tile {
            layers: Vec::with_capacity(layers),
        })
    }

    pub fn layer(self, name: impl Into<String>) -> MvtLayerBuilder {
        MvtLayerBuilder::with_tile(self, name.into())
    }

    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.0.encode_to_vec()
    }

    #[must_use]
    pub fn encoded_len(&self) -> usize {
        self.0.encoded_len().into_usize()
    }

    fn push_layer(mut self, layer: Layer) -> Self {
        self.0.layers.push(layer);
        self
    }
}

pub(crate) fn encode_tile(tile: MvtTile) -> MvtResult<Vec<u8>> {
    let mut tile_bld = MvtTileBuilder::with_capacity(tile.layers.len());
    for layer in tile.layers {
        let mut layer_bld = tile_bld.layer(layer.name);
        layer_bld.extent(layer.extent);
        layer_bld.reserve_features(layer.features.len());
        for feature in layer.features {
            let mut feature_bld = layer_bld.feature(feature.geometry)?;
            if let Some(id) = feature.id {
                feature_bld.id(id);
            }
            for (key, value) in feature.properties {
                feature_bld.tag(key, value)?;
            }
            layer_bld = feature_bld.finish();
        }
        tile_bld = layer_bld.finish();
    }
    Ok(tile_bld.finish())
}

#[derive(Debug)]
pub struct MvtLayerBuilder {
    tile: MvtTileBuilder,
    layer: Layer,
    keys: DupIndexerRefs<String>,
    values: DupIndexer<MvtValue>,
}

impl MvtLayerBuilder {
    fn with_tile(tile: MvtTileBuilder, name: String) -> Self {
        Self {
            tile,
            layer: Layer {
                version: 2,
                name,
                features: Vec::new(),
                keys: Vec::new(),
                values: Vec::new(),
                extent: Some(DEFAULT_EXTENT.get()),
            },
            keys: DupIndexerRefs::new(),
            values: DupIndexer::new(),
        }
    }

    pub fn extent(&mut self, extent: MvtExtent) -> &mut Self {
        self.layer.extent = Some(extent.get());
        self
    }

    pub fn reserve_features(&mut self, additional: usize) -> &mut Self {
        self.layer.features.reserve(additional);
        self
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.layer.name
    }

    #[must_use]
    pub fn num_features(&self) -> usize {
        self.layer.features.len()
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn feature(self, geometry: MvtGeometry) -> MvtResult<MvtFeatureBuilder> {
        let (geom_type, geometry) = encode_geometry(&geometry)?;
        Ok(MvtFeatureBuilder {
            layer: self,
            feature: Feature {
                id: None,
                tags: Vec::new(),
                r#type: Some(geom_type),
                geometry,
            },
        })
    }

    #[must_use]
    pub fn finish(self) -> MvtTileBuilder {
        let Self {
            tile,
            mut layer,
            keys,
            values,
        } = self;
        layer.keys = keys.into_vec();
        layer.values = values.into_iter().map(value_to_proto).collect();
        tile.push_layer(layer)
    }
}

#[derive(Debug)]
#[must_use = "finish the feature to commit it to the layer"]
pub struct MvtFeatureBuilder {
    layer: MvtLayerBuilder,
    feature: Feature,
}

impl MvtFeatureBuilder {
    pub fn id(&mut self, id: u64) -> &mut Self {
        self.feature.id = Some(id);
        self
    }

    pub fn tag(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<MvtValue>,
    ) -> MvtResult<&mut Self> {
        let value = value.into();
        if value != MvtValue::Null {
            let key_idx = u32_index(self.layer.keys.insert_ref(key.as_ref()))?;
            let value_idx = u32_index(self.layer.values.insert(value))?;
            self.feature.tags.push(key_idx);
            self.feature.tags.push(value_idx);
        }
        Ok(self)
    }

    pub fn tag_string(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<String>,
    ) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::String(value.into()))
    }

    pub fn tag_float(&mut self, key: impl AsRef<str>, value: f32) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::Float(value))
    }

    pub fn tag_double(&mut self, key: impl AsRef<str>, value: f64) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::Double(value))
    }

    pub fn tag_int(&mut self, key: impl AsRef<str>, value: i64) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::Int(value))
    }

    pub fn tag_uint(&mut self, key: impl AsRef<str>, value: u64) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::UInt(value))
    }

    pub fn tag_sint(&mut self, key: impl AsRef<str>, value: i64) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::SInt(value))
    }

    pub fn tag_bool(&mut self, key: impl AsRef<str>, value: bool) -> MvtResult<&mut Self> {
        self.tag(key, MvtValue::Bool(value))
    }

    #[must_use]
    pub fn num_tags(&self) -> usize {
        self.feature.tags.len() / 2
    }

    #[must_use]
    pub fn finish(mut self) -> MvtLayerBuilder {
        self.layer.layer.features.push(self.feature);
        self.layer
    }
}

// This is safe because all `MvtValue` variants contain only `PtrRead` values
// (`String`, floats, integers, bools, or no payload).
unsafe impl PtrRead for MvtValue {}

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
    use crate::MvtGeometry;

    #[test]
    fn layer_builder_deduplicates_keys_and_values() {
        let layer = MvtTileBuilder::new().layer("layer");
        let mut feature = layer.feature(MvtGeometry::Point((1, 2).into())).unwrap();
        feature.tag("foo", MvtValue::String("bar".into())).unwrap();
        feature.tag("foo", MvtValue::String("baz".into())).unwrap();
        feature.tag("bar", MvtValue::String("bar".into())).unwrap();
        feature.tag("n", MvtValue::Int(1)).unwrap();
        feature.tag("n", MvtValue::SInt(1)).unwrap();
        feature.tag("f", MvtValue::Float(f32::NAN)).unwrap();
        feature.tag("f", MvtValue::Float(f32::NAN)).unwrap();

        assert_eq!(
            feature.feature.tags,
            vec![0, 0, 0, 1, 1, 0, 2, 2, 2, 3, 3, 4, 3, 4]
        );
    }

    #[test]
    fn encode_appends_and_validates_tile_metadata() {
        let tile = MvtTileBuilder::new();
        let layer = tile.layer("layer");
        let mut feature = layer.feature(MvtGeometry::Point((1, 2).into())).unwrap();
        feature.id(1);
        feature.tag("skip", MvtValue::Null).unwrap();
        let layer = feature.finish();
        let bytes = layer.finish().finish();
        let proto = Tile::decode_from_slice(&bytes).unwrap();
        assert!(proto.layers[0].keys.is_empty());
        assert!(proto.layers[0].features[0].tags.is_empty());

        let tile = MvtTileBuilder::new();
        let layer = tile.layer("layer");
        let mut feature = layer.feature(MvtGeometry::Point((1, 2).into())).unwrap();
        feature.id(1);
        let layer = feature.finish();
        let tile = layer.finish();
        let mut out = vec![0xaa];
        out.extend_from_slice(&tile.finish());
        assert_eq!(out[0], 0xaa);

        let tile = MvtTileBuilder::new();
        let tile = tile.layer("same").finish();
        let tile = tile.layer("same").finish();
        assert!(!tile.finish().is_empty());
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
