//! Decode MVT bytes into [`FeatureCollection`] or row-oriented [`TileLayer`]s.

use std::collections::{BTreeMap, HashMap};

use fast_mvt::{MvtLayer, MvtReaderRef, MvtValue};
use serde_json::Value;

use crate::decoder::{PropValue, TileFeature, TileLayer};
use crate::geojson::{Feature, FeatureCollection};
use crate::MltResult;

/// Parse MVT bytes into a list of layers, each holding its raw features.
///
/// This is the single place where the `fast-mvt` API is called; both
/// [`mvt_to_feature_collection`] and [`mvt_to_tile_layers`] build on top of it.
fn read_mvt_layers(data: Vec<u8>) -> MltResult<Vec<MvtLayer>> {
    Ok(MvtReaderRef::new(&data)?.to_tile()?.layers)
}

/// Parse MVT binary data and convert to a [`FeatureCollection`].
pub fn mvt_to_feature_collection(data: Vec<u8>) -> MltResult<FeatureCollection> {
    let mut features = Vec::new();

    for layer in read_mvt_layers(data)? {
        for feat in layer.features {
            let mut properties = feat
                .properties
                .into_iter()
                .map(|(k, v)| Ok((k, Value::try_from(v)?)))
                .collect::<MltResult<BTreeMap<_, _>>>()?;
            properties.insert("_layer".into(), Value::String(layer.name.clone()));
            properties.insert("_extent".into(), Value::Number(layer.extent.get().into()));
            features.push(Feature {
                geometry: feat.geometry,
                id: feat.id,
                properties,
                ty: "Feature".into(),
            });
        }
    }

    Ok(FeatureCollection {
        features,
        ty: "FeatureCollection".into(),
    })
}

/// Parse MVT binary data and convert each layer to a row-oriented [`TileLayer`].
///
/// Each MVT layer becomes one [`TileLayer`].  Property column types are inferred
/// from all features in the layer: the first non-null value seen for each column
/// determines its type, with `I64`+`U64` widened to `I64` and `F32`+`F64` widened
/// to `F64`; all other type conflicts fall back to `Str`.
pub fn mvt_to_tile_layers(data: Vec<u8>) -> MltResult<Vec<TileLayer>> {
    read_mvt_layers(data)?
        .into_iter()
        .map(mvt_layer_to_tile)
        .collect()
}

fn mvt_layer_to_tile(layer: MvtLayer) -> MltResult<TileLayer> {
    // First pass: collect property names (insertion-ordered) and infer column types.
    let mut col_names: Vec<String> = Vec::new();
    let mut col_index: HashMap<String, usize> = HashMap::new();
    let mut col_types: Vec<InferredType> = Vec::new();

    for feat in &layer.features {
        for (key, val) in &feat.properties {
            let idx = *col_index.entry(key.clone()).or_insert_with(|| {
                let i = col_names.len();
                col_names.push(key.clone());
                col_types.push(InferredType::Unknown);
                i
            });
            col_types[idx] = col_types[idx].merge(InferredType::from_mvt(val));
        }
    }

    // Columns that were only ever null fall back to Str.
    for t in &mut col_types {
        if *t == InferredType::Unknown {
            *t = InferredType::Str;
        }
    }

    // Second pass: build TileFeature objects.
    let mut tile_features = Vec::with_capacity(layer.features.len());
    for feat in layer.features {
        // Start every slot with a typed null; fill in present values below.
        let mut properties: Vec<PropValue> = col_types.iter().map(|t| t.typed_null()).collect();
        for (key, val) in feat.properties {
            if let Some(&idx) = col_index.get(&key)
                && !matches!(val, MvtValue::Null)
            {
                properties[idx] = col_types[idx].convert(val);
            }
        }
        tile_features.push(TileFeature {
            id: feat.id,
            geometry: feat.geometry,
            properties,
        });
    }

    Ok(TileLayer {
        name: layer.name,
        extent: layer.extent.get(),
        property_names: col_names,
        features: tile_features,
    })
}

/// Column type inferred from MVT property values across all features in a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredType {
    Unknown,
    Bool,
    I64,
    U64,
    F32,
    F64,
    Str,
}

impl InferredType {
    fn from_mvt(val: &MvtValue) -> Self {
        match val {
            MvtValue::Bool(_) => Self::Bool,
            MvtValue::Int(_) | MvtValue::SInt(_) => Self::I64,
            MvtValue::UInt(_) => Self::U64,
            MvtValue::Float(_) => Self::F32,
            MvtValue::Double(_) => Self::F64,
            MvtValue::String(_) => Self::Str,
            MvtValue::Null => Self::Unknown,
        }
    }

    /// Merge with another type, widening when necessary.
    fn merge(self, other: Self) -> Self {
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown || self == other {
            return self;
        }
        if matches!(
            (self, other),
            (Self::I64, Self::U64) | (Self::U64, Self::I64)
        ) {
            return Self::I64;
        }
        if matches!(
            (self, other),
            (Self::F32, Self::F64) | (Self::F64, Self::F32)
        ) {
            return Self::F64;
        }
        Self::Str
    }

    fn typed_null(self) -> PropValue {
        match self {
            Self::Unknown | Self::Str => PropValue::Str(None),
            Self::Bool => PropValue::Bool(None),
            Self::I64 => PropValue::I64(None),
            Self::U64 => PropValue::U64(None),
            Self::F32 => PropValue::F32(None),
            Self::F64 => PropValue::F64(None),
        }
    }

    /// Convert an owned [`MvtValue`] into a [`PropValue`] matching this column type.
    fn convert(self, val: MvtValue) -> PropValue {
        match (self, val) {
            (_, MvtValue::Null) => self.typed_null(),
            (Self::Bool, MvtValue::Bool(b)) => PropValue::Bool(Some(b)),
            (Self::I64, MvtValue::Int(i) | MvtValue::SInt(i)) => PropValue::I64(Some(i)),
            (Self::I64, MvtValue::UInt(u)) if i64::try_from(u).is_ok() => {
                // Value must be within 0..i64::MAX
                #[expect(clippy::cast_possible_wrap, reason = "checked above")]
                PropValue::I64(Some(u as i64))
            }
            (Self::U64, MvtValue::UInt(u)) => PropValue::U64(Some(u)),
            (Self::F32, MvtValue::Float(f)) => PropValue::F32(Some(f)),
            (Self::F64, MvtValue::Double(f)) => PropValue::F64(Some(f)),
            (Self::F64, MvtValue::Float(f)) => PropValue::F64(Some(f64::from(f))),
            (_, MvtValue::String(s)) => PropValue::Str(Some(s)),
            // Type conflict at runtime: fall back to a debug string.
            (_, v) => PropValue::Str(Some(format!("{v:?}"))),
        }
    }
}
