use std::borrow::Cow;
use std::num::NonZeroU32;

use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};

pub type MvtExtent = NonZeroU32;
pub type MvtCoord = Coord<i32>;
pub type MvtPoint = Point<i32>;
pub type MvtLineString = LineString<i32>;
pub type MvtPolygon = Polygon<i32>;
pub type MvtMultiPoint = MultiPoint<i32>;
pub type MvtMultiLineString = MultiLineString<i32>;
pub type MvtMultiPolygon = MultiPolygon<i32>;
pub type MvtGeometry = Geometry<i32>;

pub const DEFAULT_EXTENT: MvtExtent = MvtExtent::new(4096).unwrap();

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MvtTile {
    pub layers: Vec<MvtLayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MvtLayer {
    pub name: String,
    pub extent: NonZeroU32,
    pub features: Vec<MvtFeature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MvtFeature {
    pub id: Option<u64>,
    pub geometry: MvtGeometry,
    pub properties: Vec<(String, MvtValue)>,
}

impl MvtTile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_layer(&mut self, layer: MvtLayer) {
        self.layers.push(layer);
    }

    #[cfg(feature = "writer")]
    pub fn encode(self) -> crate::MvtResult<Vec<u8>> {
        crate::writer::encode_tile(self)
    }
}

impl MvtLayer {
    #[must_use]
    pub fn new(name: impl Into<String>, extent: MvtExtent) -> Self {
        Self {
            name: name.into(),
            extent,
            features: Vec::new(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn num_features(&self) -> usize {
        self.features.len()
    }

    pub fn add_feature(&mut self, feature: MvtFeature) {
        self.features.push(feature);
    }
}

impl MvtFeature {
    #[must_use]
    pub fn new(geometry: MvtGeometry) -> Self {
        Self {
            id: None,
            geometry,
            properties: Vec::new(),
        }
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = Some(id);
    }

    #[must_use]
    pub fn num_tags(&self) -> usize {
        self.properties.len()
    }

    pub fn add_tag(&mut self, key: impl Into<String>, value: MvtValue) {
        self.properties.push((key.into(), value));
    }

    pub fn add_tag_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.add_tag(key, MvtValue::String(value.into()));
    }

    pub fn add_tag_float(&mut self, key: impl Into<String>, value: f32) {
        self.add_tag(key, MvtValue::Float(value));
    }

    pub fn add_tag_double(&mut self, key: impl Into<String>, value: f64) {
        self.add_tag(key, MvtValue::Double(value));
    }

    pub fn add_tag_int(&mut self, key: impl Into<String>, value: i64) {
        self.add_tag(key, MvtValue::Int(value));
    }

    pub fn add_tag_uint(&mut self, key: impl Into<String>, value: u64) {
        self.add_tag(key, MvtValue::UInt(value));
    }

    pub fn add_tag_sint(&mut self, key: impl Into<String>, value: i64) {
        self.add_tag(key, MvtValue::SInt(value));
    }

    pub fn add_tag_bool(&mut self, key: impl Into<String>, value: bool) {
        self.add_tag(key, MvtValue::Bool(value));
    }
}

#[derive(Debug, Clone)]
pub enum MvtValue {
    String(String),
    Float(f32),
    Double(f64),
    Int(i64),
    UInt(u64),
    SInt(i64),
    Bool(bool),
    Null,
}

impl From<String> for MvtValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for MvtValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<Cow<'_, str>> for MvtValue {
    fn from(value: Cow<'_, str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<f32> for MvtValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<f64> for MvtValue {
    fn from(value: f64) -> Self {
        Self::Double(value)
    }
}

impl From<i64> for MvtValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for MvtValue {
    fn from(value: i32) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<i16> for MvtValue {
    fn from(value: i16) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<i8> for MvtValue {
    fn from(value: i8) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<u64> for MvtValue {
    fn from(value: u64) -> Self {
        Self::UInt(value)
    }
}

impl From<u32> for MvtValue {
    fn from(value: u32) -> Self {
        Self::UInt(u64::from(value))
    }
}

impl From<u16> for MvtValue {
    fn from(value: u16) -> Self {
        Self::UInt(u64::from(value))
    }
}

impl From<u8> for MvtValue {
    fn from(value: u8) -> Self {
        Self::UInt(u64::from(value))
    }
}

impl From<bool> for MvtValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl PartialEq for MvtValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Double(a), Self::Double(b)) => a.to_bits() == b.to_bits(),
            (Self::Int(a), Self::Int(b)) | (Self::SInt(a), Self::SInt(b)) => a == b,
            (Self::UInt(a), Self::UInt(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

impl Eq for MvtValue {}

impl std::hash::Hash for MvtValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::String(v) => v.hash(state),
            Self::Float(v) => v.to_bits().hash(state),
            Self::Double(v) => v.to_bits().hash(state),
            Self::Int(v) | Self::SInt(v) => v.hash(state),
            Self::UInt(v) => v.hash(state),
            Self::Bool(v) => v.hash(state),
            Self::Null => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as _, Hasher as _};

    use super::MvtValue;

    #[test]
    fn null_values_compare_and_hash() {
        assert_eq!(MvtValue::Null, MvtValue::Null);

        let mut hasher = DefaultHasher::new();
        MvtValue::Null.hash(&mut hasher);
        assert_ne!(hasher.finish(), 0);
    }
}
