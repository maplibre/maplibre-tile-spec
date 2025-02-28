use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::maplibre::feature::MapLibreFeature;

#[derive(Debug, Clone, PartialEq)]
pub struct MapLibreLayer {
    pub name: String,
    pub version: u8,
    pub features: Vec<MapLibreFeature>,
    pub tile_extent: u32,
}

impl Serialize for MapLibreLayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MapLibreLayer", 3)?;

        state.serialize_field("name", &self.name)?;
        state.serialize_field("features", &self.features)?;
        state.serialize_field("tileExtent", &self.tile_extent)?;

        state.end()
    }
}
