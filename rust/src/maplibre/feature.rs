use core::fmt::Debug;
use std::collections::HashMap;
use serde::{Serialize, Serializer};
use serde::ser::SerializeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::maplibre::properties::Property;
use crate::types::geometries::Geometry;

#[derive(Debug, Clone, PartialEq)]
pub struct MapLibreFeature {
    pub id: i64,
    pub geometry: Geometry,
    pub properties: HashMap<String, Option<Property>>,
}

impl Serialize for MapLibreFeature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;

        let mut props = HashMap::new();
        for (key, value) in &self.properties {
            let value = if let Some(value) = value {
                match value {
                    Property::bool(value) => { value.to_string() }
                    Property::i8(value) => { value.to_string() }
                    Property::i16(value) => { value.to_string() }
                    Property::i32(value) => { value.to_string() }
                    Property::i64(value) => { value.to_string() }
                    Property::u8(value) => { value.to_string() }
                    Property::u16(value) => { value.to_string() }
                    Property::u32(value) => { value.to_string() }
                    Property::u64(value) => { value.to_string() }
                    Property::f32(value) => { value.to_string() }
                    Property::f64(value) => { value.to_string() }
                    Property::String(value) => { value.to_string() }
                }
            } else { "null".to_string() };
            props.insert(key, value);
        }

        state.serialize_entry("id", &self.id)?;
        state.serialize_entry("geometry", &self.geometry)?;
        state.serialize_entry("properties", &props)?;

        state.end()
    }
}
