use geo_types::Geometry;
use geozero::mvt;

pub struct MapLibreTile {
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Float(f32),
    Double(f64),
    Int(i64),
    Uint(u64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub id: i64,
    pub geometry: Geometry,
    pub properties: Vec<(String, Value)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub features: Vec<Feature>,
    pub tile_extent: i32,
}

impl From<mvt::tile::Value> for Value {
    fn from(v: mvt::tile::Value) -> Self {
        if let Some(s) = v.string_value {
            Self::String(s)
        } else if let Some(f) = v.float_value {
            Self::Float(f)
        } else if let Some(d) = v.double_value {
            Self::Double(d)
        } else if let Some(i) = v.int_value {
            Self::Int(i)
        } else if let Some(u) = v.uint_value {
            Self::Uint(u)
        } else if let Some(b) = v.bool_value {
            Self::Bool(b)
        } else {
            panic!("Unknown or unsupported value type")
        }
    }
}
