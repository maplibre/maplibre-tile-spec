//! Owned feature columns that replay into a borrowing `LayerWriter` on encode.
use std::fmt;
use std::ops::Range;

use mlt_core::encoder::EncoderConfig;
use mlt_core::geo_types::Coord;
use mlt_core::{GeometryType, InferredKind, LayerWriter, MltResult, PropKind};

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    NoFeatureBegun,
    OddCoordinateCount,
    UnknownPropertyKey(u32),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFeatureBegun => f.write_str("no feature begun"),
            Self::OddCoordinateCount => f.write_str("coordinates must come in x, y pairs"),
            Self::UnknownPropertyKey(key) => write!(f, "unknown property key {key}"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PartRole {
    Points,
    Line,
    Exterior,
    Hole,
}

#[derive(Clone)]
pub enum StoredValue {
    Bool(bool),
    I64(i64),
    F32(f32),
    F64(f64),
    Str(Range<usize>),
}

impl StoredValue {
    fn inferred_kind(&self) -> InferredKind {
        match self {
            Self::Bool(_) => InferredKind::Bool,
            Self::I64(_) => InferredKind::I64,
            Self::F32(_) => InferredKind::F32,
            Self::F64(_) => InferredKind::F64,
            Self::Str(_) => InferredKind::Str,
        }
    }
}

struct Part {
    role: PartRole,
    coords: Range<usize>,
}

struct Feature {
    geometry: GeometryType,
    id: Option<u64>,
    parts: Range<usize>,
    props: Range<usize>,
}

struct LayerHeader {
    name: String,
    extent: u32,
}

pub struct LayerBuffer {
    header: LayerHeader,
    prop_names: Vec<String>,
    prop_kinds: Vec<InferredKind>,
    features: Vec<Feature>,
    parts: Vec<Part>,
    coords: Vec<Coord<i32>>,
    props: Vec<(u32, StoredValue)>,
    strings: String,
}

impl LayerBuffer {
    pub fn new(name: &str, extent: u32) -> MltResult<Self> {
        mlt_core::TileLayer::new(name, extent)?;
        Ok(Self {
            header: LayerHeader {
                name: name.to_owned(),
                extent,
            },
            prop_names: Vec::new(),
            prop_kinds: Vec::new(),
            features: Vec::new(),
            parts: Vec::new(),
            coords: Vec::new(),
            props: Vec::new(),
            strings: String::new(),
        })
    }

    pub fn reset(&mut self, name: &str, extent: u32) -> MltResult<()> {
        mlt_core::TileLayer::new(name, extent)?;
        self.header.name.clear();
        self.header.name.push_str(name);
        self.header.extent = extent;
        self.prop_names.clear();
        self.prop_kinds.clear();
        self.features.clear();
        self.parts.clear();
        self.coords.clear();
        self.props.clear();
        self.strings.clear();
        Ok(())
    }

    pub fn add_property(&mut self, name: &str) -> u32 {
        self.prop_names.push(name.to_owned());
        self.prop_kinds.push(InferredKind::Unknown);
        u32::try_from(self.prop_names.len() - 1).expect("fewer than 2^32 properties")
    }

    pub fn begin_feature(&mut self, geometry: GeometryType, id: Option<u64>) {
        let (parts_start, props_start) = (self.parts.len(), self.props.len());
        self.features.push(Feature {
            geometry,
            id,
            parts: parts_start..parts_start,
            props: props_start..props_start,
        });
    }

    pub fn add_part(&mut self, role: PartRole, xy: &[i32]) -> Result<(), StoreError> {
        let Some(feature) = self.features.last_mut() else {
            return Err(StoreError::NoFeatureBegun);
        };
        if !xy.len().is_multiple_of(2) {
            return Err(StoreError::OddCoordinateCount);
        }
        let start = self.coords.len();
        self.coords
            .extend(xy.as_chunks::<2>().0.iter().map(|&[x, y]| Coord { x, y }));
        self.parts.push(Part {
            role,
            coords: start..self.coords.len(),
        });
        feature.parts.end = self.parts.len();
        Ok(())
    }

    pub fn set(&mut self, key: u32, value: StoredValue) -> Result<(), StoreError> {
        let Some(feature) = self.features.last_mut() else {
            return Err(StoreError::NoFeatureBegun);
        };
        if key as usize >= self.prop_names.len() {
            return Err(StoreError::UnknownPropertyKey(key));
        }
        let kind = &mut self.prop_kinds[key as usize];
        *kind = kind.merge(value.inferred_kind());
        self.props.push((key, value));
        feature.props.end = self.props.len();
        Ok(())
    }

    pub fn set_str(&mut self, key: u32, value: &str) -> Result<(), StoreError> {
        let start = self.strings.len();
        self.strings.push_str(value);
        self.set(key, StoredValue::Str(start..self.strings.len()))
    }

    fn column_kinds(&self) -> Vec<PropKind> {
        self.prop_kinds.iter().map(|k| k.prop_kind()).collect()
    }

    fn text_fallbacks(&self, kinds: &[PropKind]) -> Vec<Option<String>> {
        self.props
            .iter()
            .map(|(key, value)| {
                if kinds[*key as usize] != PropKind::Str {
                    return None;
                }
                match value {
                    StoredValue::Str(_) => None,
                    StoredValue::Bool(v) => Some(v.to_string()),
                    StoredValue::I64(v) => Some(v.to_string()),
                    StoredValue::F32(v) => Some(v.to_string()),
                    StoredValue::F64(v) => Some(v.to_string()),
                }
            })
            .collect()
    }

    pub fn encode(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        let kinds = self.column_kinds();
        let texts = self.text_fallbacks(&kinds);
        let mut layer = LayerWriter::new(&self.header.name, self.header.extent)?;
        let mut keys = Vec::with_capacity(self.prop_names.len());
        for (name, kind) in self.prop_names.iter().zip(&kinds) {
            keys.push(layer.add_property(name, *kind)?);
        }
        for feature in &self.features {
            let mut writer = layer.feature(feature.geometry);
            writer.id(feature.id);
            for part in &self.parts[feature.parts.clone()] {
                let coords = self.coords[part.coords.clone()].iter().copied();
                match part.role {
                    PartRole::Points => writer.points(coords)?,
                    PartRole::Line => writer.line(coords)?,
                    PartRole::Exterior => writer.exterior_ring(coords)?,
                    PartRole::Hole => writer.hole(coords)?,
                };
            }
            for index in feature.props.clone() {
                let (key, value) = &self.props[index];
                let column = *key as usize;
                let key = keys[column];
                match (kinds[column], value) {
                    (PropKind::Bool, StoredValue::Bool(v)) => writer.property(key, *v)?,
                    (PropKind::I64, StoredValue::I64(v)) => writer.property(key, *v)?,
                    (PropKind::F32, StoredValue::F32(v)) => writer.property(key, *v)?,
                    (PropKind::F64, StoredValue::F64(v)) => writer.property(key, *v)?,
                    (PropKind::F64, StoredValue::F32(v)) => writer.property(key, f64::from(*v))?,
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "integers in a float column are taken as floats"
                    )]
                    (PropKind::F64, StoredValue::I64(v)) => writer.property(key, *v as f64)?,
                    (_, StoredValue::Str(r)) => writer.property(key, &self.strings[r.clone()])?,
                    (_, _) => writer.property(key, texts[index].as_deref().unwrap_or_default())?,
                };
            }
            writer.finish()?;
        }
        layer.encode(cfg)
    }
}
