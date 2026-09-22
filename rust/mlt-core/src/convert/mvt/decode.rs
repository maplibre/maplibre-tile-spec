//! Decode MVT bytes into [`FeatureCollection`] or row-oriented [`TileLayer`]s.

use std::collections::{BTreeMap, HashMap};

use fast_mvt::{MvtLayer, MvtLayerRef, MvtReaderRef, MvtValue, MvtValueRef};
use serde_json::Value;

use crate::geojson::{Feature, FeatureCollection};
use crate::tile::{PropValue, TileFeature, TileLayer};
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
            #[cfg(feature = "unstable-v2")]
            m_values: Vec::new(),
            #[cfg(feature = "unstable-v2")]
            nested: Vec::new(),
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
                #[cfg(feature = "unstable-v2")]
                m_values: Vec::new(),
                #[cfg(feature = "unstable-v2")]
                nested: Vec::new(),
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
    use std::num::NonZeroU32;

    use rstest::rstest;

    use super::*;

    const POINT_AT_1_1: &[u64] = &[9, 2, 2];

    #[rstest]
    #[case::key_index_past_the_key_list(vec![5, 0], "MVT error: invalid key index 5")]
    #[case::value_index_past_the_value_list(vec![0, 9], "MVT error: invalid value index 9")]
    #[case::odd_tag_count(vec![0], "MVT error: invalid feature tags length: 1")]
    fn malformed_tags_are_reported(#[case] tags: Vec<u32>, #[case] expected: &str) {
        assert_eq!(
            mvt_to_tile_layers(mvt_with_tags(&tags))
                .expect_err("malformed tags must error")
                .to_string(),
            expected
        );
    }

    #[rstest]
    #[case::signed_then_unsigned(
        vec![MvtValue::SInt(-1), MvtValue::UInt(7)],
        vec![PropValue::I64(Some(-1)), PropValue::I64(Some(7))],
    )]
    #[case::unsigned_then_signed(
        vec![MvtValue::UInt(7), MvtValue::SInt(-1)],
        vec![PropValue::I64(Some(7)), PropValue::I64(Some(-1))],
    )]
    #[case::int_then_sint(
        vec![MvtValue::Int(i64::MIN), MvtValue::SInt(3)],
        vec![PropValue::I64(Some(i64::MIN)), PropValue::I64(Some(3))],
    )]
    #[case::unsigned_only(
        vec![MvtValue::UInt(7), MvtValue::UInt(u64::MAX)],
        vec![PropValue::U64(Some(7)), PropValue::U64(Some(u64::MAX))],
    )]
    #[case::float_only(
        vec![MvtValue::Float(1.5)],
        vec![PropValue::F32(Some(1.5))],
    )]
    #[case::float_then_double(
        vec![MvtValue::Float(1.5), MvtValue::Double(2.5)],
        vec![PropValue::F64(Some(1.5)), PropValue::F64(Some(2.5))],
    )]
    #[case::double_then_float(
        vec![MvtValue::Double(2.5), MvtValue::Float(1.5)],
        vec![PropValue::F64(Some(2.5)), PropValue::F64(Some(1.5))],
    )]
    #[case::bool_only(
        vec![MvtValue::Bool(true), MvtValue::Bool(false)],
        vec![PropValue::Bool(Some(true)), PropValue::Bool(Some(false))],
    )]
    #[case::string_only(
        vec![MvtValue::String("x".into())],
        vec![PropValue::Str(Some("x".into()))],
    )]
    #[case::bool_then_int(
        vec![MvtValue::Bool(true), MvtValue::Int(5)],
        vec![
            PropValue::Str(Some("Bool(true)".into())),
            PropValue::Str(Some("Int(5)".into())),
        ],
    )]
    #[case::double_then_string(
        vec![MvtValue::Double(1.5), MvtValue::String("x".into())],
        vec![
            PropValue::Str(Some("Double(1.5)".into())),
            PropValue::Str(Some("x".into())),
        ],
    )]
    #[case::null_between_bools(
        vec![MvtValue::Bool(true), MvtValue::Null, MvtValue::Bool(false)],
        vec![
            PropValue::Bool(Some(true)),
            PropValue::Bool(None),
            PropValue::Bool(Some(false)),
        ],
    )]
    #[case::null_only(
        vec![MvtValue::Null],
        vec![PropValue::Str(None)],
    )]
    fn a_column_takes_the_type_that_holds_every_value(
        #[case] values: Vec<MvtValue>,
        #[case] expected: Vec<PropValue>,
    ) {
        assert_eq!(single_column(&values), expected);
    }

    #[test]
    fn an_unsigned_value_past_i64_max_clashes_with_its_signed_column() {
        let data = mvt_with_values(&[MvtValue::SInt(-1), MvtValue::UInt(u64::MAX)]);
        assert_eq!(
            mvt_to_tile_layers(&data)
                .expect_err("the debug-string fallback leaves the column mixed")
                .to_string(),
            "property 0 kind mismatch: expected I64, got Str"
        );
    }

    #[test]
    fn missing_properties_become_typed_nulls() {
        let messages: Vec<Vec<u8>> = [
            MvtValue::Int(1),
            MvtValue::UInt(2),
            MvtValue::Float(1.5),
            MvtValue::Double(2.5),
            MvtValue::Bool(true),
            MvtValue::String("x".into()),
        ]
        .iter()
        .map(value_message)
        .collect();
        let every_key: Vec<u32> = (0..6).flat_map(|i| [i, i]).collect();
        let data = mvt_tile(&[layer_body(
            b"l",
            4096,
            &[
                b"i".as_slice(),
                b"u".as_slice(),
                b"f".as_slice(),
                b"d".as_slice(),
                b"b".as_slice(),
                b"s".as_slice(),
            ],
            &messages,
            &[(None, &every_key, POINT_AT_1_1), (None, &[], POINT_AT_1_1)],
        )]);

        let layers = both_paths(&data);
        let [layer] = &layers[..] else {
            panic!("expected one layer, got {}", layers.len())
        };
        assert_eq!(layer.property_names(), ["i", "u", "f", "d", "b", "s"]);
        assert_eq!(
            layer.features()[0].properties(),
            [
                PropValue::I64(Some(1)),
                PropValue::U64(Some(2)),
                PropValue::F32(Some(1.5)),
                PropValue::F64(Some(2.5)),
                PropValue::Bool(Some(true)),
                PropValue::Str(Some("x".into())),
            ]
        );
        assert_eq!(
            layer.features()[1].properties(),
            [
                PropValue::I64(None),
                PropValue::U64(None),
                PropValue::F32(None),
                PropValue::F64(None),
                PropValue::Bool(None),
                PropValue::Str(None),
            ]
        );
    }

    #[test]
    fn column_order_follows_first_use_not_the_key_list() {
        let messages: Vec<Vec<u8>> = [
            MvtValue::String("1".into()),
            MvtValue::String("2".into()),
            MvtValue::String("3".into()),
        ]
        .iter()
        .map(value_message)
        .collect();
        let data = mvt_tile(&[layer_body(
            b"l",
            4096,
            &[b"b".as_slice(), b"a".as_slice(), b"c".as_slice()],
            &messages,
            &[
                (None, &[2, 2, 0, 0], POINT_AT_1_1),
                (None, &[1, 1], POINT_AT_1_1),
            ],
        )]);

        let layers = both_paths(&data);
        let [layer] = &layers[..] else {
            panic!("expected one layer, got {}", layers.len())
        };
        assert_eq!(layer.property_names(), ["c", "b", "a"]);
        assert_eq!(
            layer.features()[0].properties(),
            [
                PropValue::Str(Some("3".into())),
                PropValue::Str(Some("1".into())),
                PropValue::Str(None),
            ]
        );
        assert_eq!(
            layer.features()[1].properties(),
            [
                PropValue::Str(None),
                PropValue::Str(None),
                PropValue::Str(Some("2".into())),
            ]
        );
    }

    #[test]
    fn each_layer_keeps_its_own_name_extent_and_columns() {
        let layers = both_paths(&two_layer_tile());
        assert_eq!(
            layers
                .iter()
                .map(|layer| (
                    layer.name(),
                    layer.extent().get(),
                    layer.property_names().to_vec()
                ))
                .collect::<Vec<_>>(),
            [
                ("roads", 4096, vec!["n".to_string()]),
                ("water", 512, vec!["z".to_string()]),
            ]
        );
        assert_eq!(
            layers[0].features()[0].properties(),
            [PropValue::Str(Some("a".into()))]
        );
        assert_eq!(
            layers[1].features()[0].properties(),
            [PropValue::I64(Some(3))]
        );
    }

    #[test]
    fn a_layer_without_features_has_no_columns() {
        let data = mvt_tile(&[layer_body(b"l", 4096, &[], &[], &[])]);
        let layers = both_paths(&data);
        let [layer] = &layers[..] else {
            panic!("expected one layer, got {}", layers.len())
        };
        let no_names: [&str; 0] = [];
        assert_eq!(layer.property_names(), no_names);
        assert_eq!(layer.feature_count(), 0);
    }

    #[test]
    fn feature_ids_survive_both_conversions() {
        let data = mvt_tile(&[layer_body(
            b"l",
            4096,
            &[],
            &[],
            &[(Some(7), &[], POINT_AT_1_1), (None, &[], POINT_AT_1_1)],
        )]);
        let layers = both_paths(&data);
        assert_eq!(
            layers[0]
                .features()
                .iter()
                .map(TileFeature::id)
                .collect::<Vec<_>>(),
            [Some(7), None]
        );
    }

    #[test]
    fn a_zero_extent_is_rejected() {
        let data = mvt_tile(&[layer_body(b"l", 0, &[], &[], &[])]);
        assert_eq!(
            mvt_to_tile_layers(&data)
                .expect_err("zero extent must error")
                .to_string(),
            "invalid extent: 0"
        );
    }

    #[test]
    fn a_truncated_geometry_is_reported() {
        let data = mvt_tile(&[layer_body(b"l", 4096, &[], &[], &[(None, &[], &[9])])]);
        assert_eq!(
            mvt_to_tile_layers(&data)
                .expect_err("truncated geometry must error")
                .to_string(),
            "MVT error: invalid geometry command stream"
        );
    }

    #[test]
    fn a_layer_without_a_name_is_rejected() {
        let unnamed = mvt_layer(b"", &[vec![]], b"k", &[]);
        assert_eq!(
            mvt_to_tile_layers(&unnamed)
                .expect_err("borrowed conversion must error")
                .to_string(),
            "MVT error: missing required layer name"
        );
        assert_eq!(
            mvt_to_feature_collection(&unnamed)
                .expect_err("feature collection must error")
                .to_string(),
            "MVT error: missing required layer name"
        );

        let hand_built = MvtLayer {
            name: String::new(),
            extent: NonZeroU32::new(4096).expect("non-zero"),
            features: Vec::new(),
        };
        assert_eq!(
            TileLayer::try_from(hand_built)
                .expect_err("owned conversion must error")
                .to_string(),
            "missing layer name"
        );
    }

    #[test]
    fn unreadable_tile_bytes_are_reported() {
        assert!(matches!(
            mvt_to_tile_layers([0xff_u8]),
            Err(MltError::Mvt(_))
        ));
        assert!(matches!(
            mvt_to_feature_collection([0xff_u8]),
            Err(MltError::Mvt(_))
        ));
    }

    #[test]
    fn an_empty_tile_holds_no_layers_and_no_features() {
        assert_eq!(
            mvt_to_tile_layers([0_u8; 0]).expect("convert"),
            Vec::<TileLayer>::new()
        );
        insta::assert_snapshot!(
            serde_json::to_string(&mvt_to_feature_collection([0_u8; 0]).expect("convert"))
                .expect("serialize"),
            @r#"{"type":"FeatureCollection","features":[]}"#
        );
    }

    #[test]
    fn every_feature_carries_its_own_layer_name_and_extent() {
        insta::assert_snapshot!(
            serde_json::to_string(&mvt_to_feature_collection(two_layer_tile()).expect("convert"))
                .expect("serialize"),
            @r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"_extent":4096,"_layer":"roads","n":"a"},"geometry":{"type":"Point","coordinates":[1,1]}},{"type":"Feature","properties":{"_extent":512,"_layer":"water","z":3},"geometry":{"type":"Point","coordinates":[1,1]}}]}"#
        );
    }

    #[test]
    fn a_non_finite_double_cannot_become_geojson() {
        assert_eq!(
            mvt_to_feature_collection(mvt_with_values(&[MvtValue::Double(f64::NAN)]))
                .expect_err("NaN must error")
                .to_string(),
            "MVT JSON value error: non-finite float cannot be represented as JSON"
        );
    }

    /// Convert a one-column tile through both paths and return that column's value per feature.
    fn single_column(values: &[MvtValue]) -> Vec<PropValue> {
        let data = mvt_with_values(values);
        let layers = both_paths(&data);
        let [layer] = &layers[..] else {
            panic!("expected one layer, got {}", layers.len())
        };
        assert_eq!(layer.property_names(), ["k"]);
        layer
            .features()
            .iter()
            .map(|feat| feat.properties()[0].clone())
            .collect()
    }

    /// Convert through the borrowed and the owned path, asserting they agree.
    fn both_paths(data: &[u8]) -> Vec<TileLayer> {
        let from_ref = mvt_to_tile_layers(data).expect("borrowed conversion");
        let owned: Vec<TileLayer> = MvtReaderRef::new(data)
            .expect("read")
            .to_tile()
            .expect("to_tile")
            .layers
            .into_iter()
            .map(TileLayer::try_from)
            .collect::<MltResult<_>>()
            .expect("owned conversion");
        assert_eq!(from_ref, owned);
        from_ref
    }

    /// Two layers of one point feature each, differing in name, extent, key and value type.
    fn two_layer_tile() -> Vec<u8> {
        mvt_tile(&[
            layer_body(
                b"roads",
                4096,
                &[b"n".as_slice()],
                &[value_message(&MvtValue::String("a".into()))],
                &[(None, &[0, 0], POINT_AT_1_1)],
            ),
            layer_body(
                b"water",
                512,
                &[b"z".as_slice()],
                &[value_message(&MvtValue::Int(3))],
                &[(None, &[0, 0], POINT_AT_1_1)],
            ),
        ])
    }

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

    /// One `Value` message, its field number picking the type the reader infers.
    fn value_message(value: &MvtValue) -> Vec<u8> {
        let mut out = Vec::new();
        match value {
            MvtValue::String(s) => bytes(1, s.as_bytes(), &mut out),
            MvtValue::Float(f) => {
                out.push(field(2, 5));
                out.extend(f.to_le_bytes());
            }
            MvtValue::Double(f) => {
                out.push(field(3, 1));
                out.extend(f.to_le_bytes());
            }
            MvtValue::Int(i) => {
                out.push(field(4, 0));
                varint(i.cast_unsigned(), &mut out);
            }
            MvtValue::UInt(u) => {
                out.push(field(5, 0));
                varint(*u, &mut out);
            }
            MvtValue::SInt(i) => {
                out.push(field(6, 0));
                varint(zigzag::ZigZag::encode(*i), &mut out);
            }
            MvtValue::Bool(b) => {
                out.push(field(7, 0));
                varint(u64::from(*b), &mut out);
            }
            // A `Value` with no field set is what the reader reports as `Null`.
            MvtValue::Null => {}
        }
        out
    }

    /// Tile of one layer holding one point feature per `values` entry, all under key `k`.
    fn mvt_with_values(values: &[MvtValue]) -> Vec<u8> {
        let tags: Vec<Vec<u32>> = (0..values.len())
            .map(|i| vec![0, u32::try_from(i).expect("small")])
            .collect();
        let messages: Vec<Vec<u8>> = values.iter().map(value_message).collect();
        mvt_layer(b"l", &tags, b"k", &messages)
    }

    /// Minimal hand-written MVT tile with one point feature carrying `tags`.
    fn mvt_with_tags(tags: &[u32]) -> Vec<u8> {
        mvt_layer(b"l", &[tags.to_vec()], b"k", &[])
    }

    /// Tile of one default-extent layer with a single key and one point feature per tag list.
    fn mvt_layer(
        name: &[u8],
        feature_tags: &[Vec<u32>],
        key: &[u8],
        values: &[Vec<u8>],
    ) -> Vec<u8> {
        let features: Vec<(Option<u64>, &[u32], &[u64])> = feature_tags
            .iter()
            .map(|tags| (None, tags.as_slice(), POINT_AT_1_1))
            .collect();
        mvt_tile(&[layer_body(name, 4096, &[key], values, &features)])
    }

    fn mvt_tile(layers: &[Vec<u8>]) -> Vec<u8> {
        let mut tile = Vec::new();
        for layer in layers {
            bytes(3, layer, &mut tile);
        }
        tile
    }

    /// One `Layer` message, each feature given as its id, its tags and its geometry commands.
    fn layer_body(
        name: &[u8],
        extent: u32,
        keys: &[&[u8]],
        values: &[Vec<u8>],
        features: &[(Option<u64>, &[u32], &[u64])],
    ) -> Vec<u8> {
        let mut layer = Vec::new();
        layer.push(field(15, 0));
        varint(2, &mut layer); // version
        bytes(1, name, &mut layer);
        for (id, tags, geometry) in features {
            let mut feat = Vec::new();
            if let Some(id) = id {
                feat.push(field(1, 0));
                varint(*id, &mut feat);
            }
            packed(
                2,
                &tags.iter().copied().map(u64::from).collect::<Vec<_>>(),
                &mut feat,
            );
            feat.push(field(3, 0));
            varint(1, &mut feat); // POINT
            packed(4, geometry, &mut feat);
            bytes(2, &feat, &mut layer);
        }
        for key in keys {
            bytes(3, key, &mut layer);
        }
        for value in values {
            bytes(4, value, &mut layer);
        }
        layer.push(field(5, 0));
        varint(u64::from(extent), &mut layer);
        layer
    }
}
