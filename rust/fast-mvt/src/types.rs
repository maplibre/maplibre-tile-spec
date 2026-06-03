use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};

pub type MvtCoord = Coord<i32>;
pub type MvtPoint = Point<i32>;
pub type MvtLineString = LineString<i32>;
pub type MvtPolygon = Polygon<i32>;
pub type MvtMultiPoint = MultiPoint<i32>;
pub type MvtMultiLineString = MultiLineString<i32>;
pub type MvtMultiPolygon = MultiPolygon<i32>;
pub type MvtGeometry = Geometry<i32>;

pub const DEFAULT_EXTENT: u32 = 4096;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MvtTile {
    pub layers: Vec<MvtLayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MvtLayer {
    pub name: String,
    pub extent: u32,
    pub features: Vec<MvtFeature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MvtFeature {
    pub id: Option<u64>,
    pub geometry: MvtGeometry,
    pub properties: Vec<(String, MvtValue)>,
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
