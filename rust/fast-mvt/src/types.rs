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

    #[must_use]
    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    pub fn add_layer(&mut self, layer: MvtLayer) -> crate::MvtResult<()> {
        if self
            .layers
            .iter()
            .any(|existing| existing.name == layer.name)
        {
            return Err(crate::MvtError::DuplicateLayer(layer.name));
        }
        self.layers.push(layer);
        Ok(())
    }

    #[cfg(feature = "writer")]
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> crate::MvtResult<()> {
        out.write_all(&crate::encode_to_vec(self)?)?;
        Ok(())
    }

    #[cfg(feature = "writer")]
    pub fn to_bytes(&self) -> crate::MvtResult<Vec<u8>> {
        crate::encode_to_vec(self)
    }

    #[cfg(feature = "writer")]
    pub fn compute_size(&self) -> crate::MvtResult<usize> {
        crate::writer::encoded_len(self)
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
