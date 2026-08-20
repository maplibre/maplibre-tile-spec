//! Decode MVT bytes into [`FeatureCollection`] or row-oriented [`TileLayer`]s.

use std::collections::{BTreeMap, HashMap};

use fast_mvt::{MvtLayer, MvtLayerRef, MvtReaderRef, MvtValue, MvtValueRef};
use serde_json::Value;

use crate::decoder::{PropValue, TileFeature, TileLayer};
use crate::geojson::{Feature, FeatureCollection};
use crate::{MltError, MltResult};

/// Parse MVT bytes into a list of layers, each holding its raw features.
fn read_mvt_layers(data: &[u8]) -> MltResult<Vec<MvtLayer>> {
    let layers = MvtReaderRef::new(data)?.to_tile()?.layers;
    if layers.iter().any(|layer| layer.name.is_empty()) {
        return Err(MltError::MissingLayerName);
    }
    Ok(layers)
}

/// Parse MVT binary data and convert to a [`FeatureCollection`].
pub fn mvt_to_feature_collection(data: impl AsRef<[u8]>) -> MltResult<FeatureCollection> {
    let mut features = Vec::new();

    for layer in read_mvt_layers(data.as_ref())? {
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
pub fn mvt_to_tile_layers(data: impl AsRef<[u8]>) -> MltResult<Vec<TileLayer>> {
    MvtReaderRef::new(data.as_ref())?
        .layers()
        .map(tile_layer_from_ref)
        .collect()
}

/// Build a [`TileLayer`] straight from the borrowed reader.
fn tile_layer_from_ref(layer: MvtLayerRef<'_>) -> MltResult<TileLayer> {
    let name = layer.name();
    if name.is_empty() {
        return Err(MltError::MissingLayerName);
    }

    // First pass: collect property names (insertion-ordered) and infer column types.
    let mut col_names: Vec<String> = Vec::new();
    let mut col_index: HashMap<&str, usize> = HashMap::new();
    let mut col_types: Vec<InferredType> = Vec::new();
    // Each value with its column, so the second pass resolves no keys.
    let mut values: Vec<(usize, MvtValueRef<'_>)> = Vec::new();
    let mut feature_ends: Vec<usize> = Vec::with_capacity(layer.feature_count());

    for feat in layer.features() {
        for prop in feat.properties() {
            let (key, value) = prop?;
            let idx = if let Some(&idx) = col_index.get(key) {
                idx
            } else {
                let idx = col_names.len();
                col_names.push(key.to_string());
                col_index.insert(key, idx);
                col_types.push(InferredType::Unknown);
                idx
            };
            // One bounds check rather than one per index expression.
            let slot = &mut col_types[idx];
            *slot = slot.merge(InferredType::from_mvt(value));
            values.push((idx, value));
        }
        feature_ends.push(values.len());
    }

    // Columns that were only ever null fall back to Str.
    for t in &mut col_types {
        if *t == InferredType::Unknown {
            *t = InferredType::Str;
        }
    }

    // Second pass: build TileFeature objects.
    let mut tile_features = Vec::with_capacity(layer.feature_count());
    let mut start = 0;
    for (feat, &end) in layer.features().zip(&feature_ends) {
        // Start every slot with a typed null; fill in present values below.
        let mut properties: Vec<PropValue> = col_types.iter().map(|t| t.typed_null()).collect();
        for &(idx, value) in &values[start..end] {
            if !matches!(value, MvtValueRef::Null) {
                properties[idx] = col_types[idx].convert(value.into_owned());
            }
        }
        start = end;
        tile_features.push(TileFeature {
            id: feat.id(),
            geometry: feat.geometry()?,
            properties,
        });
    }

    TileLayer::from_parts(name, layer.extent(), col_names, tile_features)
}

impl TryFrom<MvtLayer> for TileLayer {
    type Error = MltError;

    fn try_from(layer: MvtLayer) -> Result<Self, Self::Error> {
        if layer.name.is_empty() {
            return Err(MltError::MissingLayerName);
        }

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
                let slot = &mut col_types[idx];
                *slot = slot.merge(InferredType::from_mvt(as_value_ref(val)));
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

        Self::from_parts(layer.name, layer.extent.get(), col_names, tile_features)
    }
}

/// Borrow an owned [`MvtValue`], so both conversion paths share one inference pass.
fn as_value_ref(value: &MvtValue) -> MvtValueRef<'_> {
    match value {
        MvtValue::String(s) => MvtValueRef::String(s),
        MvtValue::Float(f) => MvtValueRef::Float(*f),
        MvtValue::Double(f) => MvtValueRef::Double(*f),
        MvtValue::Int(i) => MvtValueRef::Int(*i),
        MvtValue::UInt(u) => MvtValueRef::UInt(*u),
        MvtValue::SInt(i) => MvtValueRef::SInt(*i),
        MvtValue::Bool(b) => MvtValueRef::Bool(*b),
        MvtValue::Null => MvtValueRef::Null,
    }
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
    fn from_mvt(val: MvtValueRef<'_>) -> Self {
        match val {
            MvtValueRef::Bool(_) => Self::Bool,
            MvtValueRef::Int(_) | MvtValueRef::SInt(_) => Self::I64,
            MvtValueRef::UInt(_) => Self::U64,
            MvtValueRef::Float(_) => Self::F32,
            MvtValueRef::Double(_) => Self::F64,
            MvtValueRef::String(_) => Self::Str,
            MvtValueRef::Null => Self::Unknown,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_tags_are_reported() {
        for (tags, expected) in [
            (&[5, 0][..], "invalid key index 5"),
            (&[0, 9][..], "invalid value index 9"),
            (&[0][..], "invalid feature tags length: 1"),
        ] {
            let err = mvt_to_tile_layers(mvt_with_tags(tags))
                .expect_err("malformed tags must error")
                .to_string();
            assert!(err.contains(expected), "got {err:?}, wanted {expected:?}");
        }
    }

    /// Minimal hand-written MVT tile with one point feature carrying `tags`.
    fn mvt_with_tags(tags: &[u32]) -> Vec<u8> {
        fn field(number: u32, wire: u32) -> u8 {
            u8::try_from((number << 3) | wire).expect("small field number")
        }
        fn varint(mut value: u64, out: &mut Vec<u8>) {
            loop {
                let byte = u8::try_from(value & 0x7f).expect("masked");
                value >>= 7;
                if value == 0 {
                    out.push(byte);
                    return;
                }
                out.push(byte | 0x80);
            }
        }
        fn packed(number: u32, values: &[u64], out: &mut Vec<u8>) {
            let mut body = Vec::new();
            for value in values {
                varint(*value, &mut body);
            }
            out.push(field(number, 2));
            varint(u64::try_from(body.len()).expect("small"), out);
            out.extend(&body);
        }
        fn bytes(number: u32, body: &[u8], out: &mut Vec<u8>) {
            out.push(field(number, 2));
            varint(u64::try_from(body.len()).expect("small"), out);
            out.extend(body);
        }

        let mut feat = Vec::new();
        packed(
            2,
            &tags.iter().copied().map(u64::from).collect::<Vec<_>>(),
            &mut feat,
        );
        feat.push(field(3, 0));
        varint(1, &mut feat); // POINT
        packed(4, &[9, 2, 2], &mut feat); // MoveTo(1, 1)

        let mut layer = Vec::new();
        layer.push(field(15, 0));
        varint(2, &mut layer); // version
        bytes(1, b"l", &mut layer); // name
        bytes(2, &feat, &mut layer); // features
        bytes(3, b"k", &mut layer); // keys
        layer.push(field(5, 0));
        varint(4096, &mut layer); // extent

        let mut tile = Vec::new();
        bytes(3, &layer, &mut tile);
        tile
    }
}
