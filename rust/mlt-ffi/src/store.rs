//! Owned feature columns that replay into a borrowing `LayerWriter` on encode.
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

use mlt_core::encoder::EncoderConfig;
use mlt_core::geo_types::Coord;
use mlt_core::{GeometryType, InferredKind, LayerWriter, MltResult, PropKind};

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    NoFeatureBegun,
    UnknownPropertyKey(u32),
    InvalidMvtGeometry(&'static str),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFeatureBegun => f.write_str("no feature begun"),
            Self::UnknownPropertyKey(key) => write!(f, "unknown property key {key}"),
            Self::InvalidMvtGeometry(reason) => write!(f, "invalid MVT geometry: {reason}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MvtGeometryType {
    Point,
    LineString,
    Polygon,
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

fn invalid_mvt(reason: &'static str) -> StoreError {
    StoreError::InvalidMvtGeometry(reason)
}

fn unzigzag(value: u32) -> i32 {
    (value >> 1).cast_signed() ^ -(value & 1).cast_signed()
}

fn signed_area(ring: &[Coord<i32>]) -> i64 {
    let next = ring.iter().cycle().skip(1);
    ring.iter()
        .zip(next)
        .map(|(a, b)| i64::from(a.x) * i64::from(b.y) - i64::from(b.x) * i64::from(a.y))
        .sum()
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
    prop_keys: HashMap<String, u32>,
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
            prop_keys: HashMap::new(),
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
        self.prop_keys.clear();
        self.prop_kinds.clear();
        self.features.clear();
        self.parts.clear();
        self.coords.clear();
        self.props.clear();
        self.strings.clear();
        Ok(())
    }

    pub fn add_property(&mut self, name: &str) -> u32 {
        if let Some(&key) = self.prop_keys.get(name) {
            return key;
        }
        let key = u32::try_from(self.prop_names.len()).expect("fewer than 2^32 properties");
        self.prop_names.push(name.to_owned());
        self.prop_keys.insert(name.to_owned(), key);
        self.prop_kinds.push(InferredKind::Unknown);
        key
    }

    pub fn begin_mvt_feature(
        &mut self,
        mvt_type: MvtGeometryType,
        commands: &[u32],
        id: Option<u64>,
    ) -> Result<(), StoreError> {
        let (coords_start, parts_start) = (self.coords.len(), self.parts.len());
        let geometry = self.decode_mvt(mvt_type, commands).inspect_err(|_| {
            self.coords.truncate(coords_start);
            self.parts.truncate(parts_start);
        })?;
        let props_start = self.props.len();
        self.features.push(Feature {
            geometry,
            id,
            parts: parts_start..self.parts.len(),
            props: props_start..props_start,
        });
        Ok(())
    }

    fn decode_mvt(
        &mut self,
        mvt_type: MvtGeometryType,
        commands: &[u32],
    ) -> Result<GeometryType, StoreError> {
        let coords_start = self.coords.len();
        let mut cursor = Coord { x: 0, y: 0 };
        let mut part: Option<usize> = None;
        let mut parts = 0;
        let mut exteriors = 0;
        let mut values = commands.iter();
        while let Some(&command) = values.next() {
            let count = command >> 3;
            match command & 7 {
                1 => {
                    if mvt_type != MvtGeometryType::Point {
                        if count != 1 {
                            return Err(invalid_mvt(
                                "a line or ring starts with more than one MoveTo point",
                            ));
                        }
                        self.end_mvt_part(mvt_type, part.take(), &mut parts, &mut exteriors);
                    }
                    part.get_or_insert(self.coords.len());
                    self.read_mvt_coords(&mut values, &mut cursor, count)?;
                }
                2 => {
                    if mvt_type == MvtGeometryType::Point || part.is_none() {
                        return Err(invalid_mvt("a LineTo outside a line or ring"));
                    }
                    self.read_mvt_coords(&mut values, &mut cursor, count)?;
                }
                7 => {
                    if mvt_type != MvtGeometryType::Polygon || part.is_none() || count != 1 {
                        return Err(invalid_mvt("a ClosePath outside a ring"));
                    }
                }
                _ => return Err(invalid_mvt("an unknown command")),
            }
        }
        self.end_mvt_part(mvt_type, part, &mut parts, &mut exteriors);
        Ok(match mvt_type {
            MvtGeometryType::Point => match self.coords.len() - coords_start {
                0 => return Err(invalid_mvt("no points")),
                1 => GeometryType::Point,
                _ => GeometryType::MultiPoint,
            },
            MvtGeometryType::LineString => match parts {
                0 => return Err(invalid_mvt("no lines")),
                1 => GeometryType::LineString,
                _ => GeometryType::MultiLineString,
            },
            MvtGeometryType::Polygon => match exteriors {
                0 => return Err(invalid_mvt("no rings")),
                1 => GeometryType::Polygon,
                _ => GeometryType::MultiPolygon,
            },
        })
    }

    fn read_mvt_coords(
        &mut self,
        values: &mut std::slice::Iter<'_, u32>,
        cursor: &mut Coord<i32>,
        count: u32,
    ) -> Result<(), StoreError> {
        for _ in 0..count {
            let (Some(&dx), Some(&dy)) = (values.next(), values.next()) else {
                return Err(invalid_mvt("ends inside a command"));
            };
            cursor.x = cursor.x.saturating_add(unzigzag(dx));
            cursor.y = cursor.y.saturating_add(unzigzag(dy));
            self.coords.push(*cursor);
        }
        Ok(())
    }

    /// The first ring of a feature and every ring with positive area start a polygon.
    fn end_mvt_part(
        &mut self,
        mvt_type: MvtGeometryType,
        start: Option<usize>,
        parts: &mut usize,
        exteriors: &mut usize,
    ) {
        let Some(start) = start else {
            return;
        };
        let role = match mvt_type {
            MvtGeometryType::Point => PartRole::Points,
            MvtGeometryType::LineString => PartRole::Line,
            MvtGeometryType::Polygon => {
                let ring = &self.coords[start..];
                let (first, exterior) = (ring[0], *exteriors == 0 || signed_area(ring) > 0);
                self.coords.push(first);
                if exterior {
                    *exteriors += 1;
                    PartRole::Exterior
                } else {
                    PartRole::Hole
                }
            }
        };
        *parts += 1;
        self.parts.push(Part {
            role,
            coords: start..self.coords.len(),
        });
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
