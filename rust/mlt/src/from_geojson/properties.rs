//! Property schema inference and value conversion for `GeoJSON` features.
//!
//! `GeoJSON` properties are dynamically typed JSON, while an MLT layer needs one
//! fixed [`PropKind`] per column shared across every feature (and here across
//! every tile). All features are scanned once to infer a column kind: signed
//! and unsigned integers widen to `I64`, integers and floats widen to `F64`,
//! and any other conflict falls back to `Str`. (`mlt-py` has its own copy of
//! this inference that widens integers and floats to `Str` instead.) Nested
//! arrays and objects are rejected, as `mlt-py` does; MLT v2's nested columns
//! are not produced yet.

use std::collections::HashMap;

use anyhow::{Result as AnyResult, bail};
use geojson::feature::Id;
use geojson::{Feature, JsonValue};
use mlt_core::{PropKind, PropValue};

/// Column type inferred so far. `Unknown` is a column seen only as absent or
/// `null`, which resolves to `Str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredKind {
    Unknown,
    Bool,
    I64,
    U64,
    F64,
    Str,
}

impl InferredKind {
    /// Classify one scalar value; a nested array or object is an error.
    fn from_json(name: &str, val: &JsonValue) -> AnyResult<Self> {
        Ok(match val {
            JsonValue::Null => Self::Unknown,
            JsonValue::Bool(_) => Self::Bool,
            JsonValue::Number(n) => {
                if n.is_u64() {
                    Self::U64
                } else if n.is_i64() {
                    Self::I64
                } else {
                    Self::F64
                }
            }
            JsonValue::String(_) => Self::Str,
            JsonValue::Array(_) | JsonValue::Object(_) => bail!(
                "property {name:?} holds a nested array or object, which is not supported; \
                 flatten it or remove it"
            ),
        })
    }

    fn merge(self, other: Self) -> Self {
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown || self == other {
            return self;
        }
        match (self, other) {
            (Self::I64, Self::U64) | (Self::U64, Self::I64) => Self::I64,
            (Self::F64, Self::I64 | Self::U64) | (Self::I64 | Self::U64, Self::F64) => Self::F64,
            _ => Self::Str,
        }
    }

    fn kind(self) -> PropKind {
        match self {
            Self::Unknown | Self::Str => PropKind::Str,
            Self::Bool => PropKind::Bool,
            Self::I64 => PropKind::I64,
            Self::U64 => PropKind::U64,
            Self::F64 => PropKind::F64,
        }
    }
}

/// The ordered property schema shared by every tile: column names in first-seen
/// order with their inferred kinds.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Schema {
    pub columns: Vec<(String, PropKind)>,
}

impl Schema {
    /// Infer the schema from every feature's properties.
    pub fn infer(features: &[Feature]) -> AnyResult<Self> {
        let mut names: Vec<String> = Vec::new();
        let mut kinds: Vec<InferredKind> = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for feature in features {
            let Some(props) = feature.properties.as_ref() else {
                continue;
            };
            for (name, value) in props {
                let idx = *index.entry(name).or_insert_with(|| {
                    names.push(name.clone());
                    kinds.push(InferredKind::Unknown);
                    names.len() - 1
                });
                kinds[idx] = kinds[idx].merge(InferredKind::from_json(name, value)?);
            }
        }
        Ok(Self {
            columns: names
                .into_iter()
                .zip(kinds)
                .map(|(name, kind)| (name, kind.kind()))
                .collect(),
        })
    }

    /// The feature's non-null property values as `(column index, value)` pairs,
    /// in schema order. Absent keys and JSON nulls are left out, which the layer
    /// builder reads as the column's typed null.
    pub fn values(&self, feature: &Feature) -> AnyResult<Vec<(usize, PropValue)>> {
        let Some(props) = feature.properties.as_ref() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(props.len());
        for (idx, (name, kind)) in self.columns.iter().enumerate() {
            let Some(value) = props.get(name) else {
                continue;
            };
            if let Some(value) = convert(*kind, value, name)? {
                out.push((idx, value));
            }
        }
        Ok(out)
    }
}

/// Convert one JSON value to the column's kind. `None` for a JSON null.
fn convert(kind: PropKind, value: &JsonValue, name: &str) -> AnyResult<Option<PropValue>> {
    let value = match (kind, value) {
        (_, JsonValue::Null) => return Ok(None),
        (PropKind::Bool, JsonValue::Bool(b)) => PropValue::Bool(Some(*b)),
        (PropKind::I64, JsonValue::Number(n)) => match n.as_i64() {
            Some(i) => PropValue::I64(Some(i)),
            None => bail!("property {name:?} value {n} does not fit a signed 64-bit column"),
        },
        (PropKind::U64, JsonValue::Number(n)) => match n.as_u64() {
            Some(u) => PropValue::U64(Some(u)),
            None => bail!("property {name:?} value {n} does not fit an unsigned 64-bit column"),
        },
        (PropKind::F64, JsonValue::Number(n)) => match n.as_f64() {
            Some(f) => PropValue::F64(Some(f)),
            None => bail!("property {name:?} value {n} is not a finite number"),
        },
        (PropKind::Str, JsonValue::String(s)) => PropValue::Str(Some(s.clone())),
        // A scalar in a column that widened to `Str` keeps its JSON text.
        (PropKind::Str, other @ (JsonValue::Bool(_) | JsonValue::Number(_))) => {
            PropValue::Str(Some(other.to_string()))
        }
        // Inference merges every value a column holds, so no other pairing can occur.
        (kind, other) => bail!("property {name:?} value {other} does not fit its {kind:?} column"),
    };
    Ok(Some(value))
}

/// The feature `id` as an MLT feature id, which must be a non-negative integer
/// when present.
pub(super) fn feature_id(feature: &Feature) -> AnyResult<Option<u64>> {
    let Some(id) = feature.id.as_ref() else {
        return Ok(None);
    };
    match id {
        Id::Number(n) => match n.as_u64() {
            Some(id) => Ok(Some(id)),
            None => bail!("id {n} is not a non-negative integer, which is all an MLT id can hold"),
        },
        Id::String(s) => bail!(
            "id {s:?} is a string, but an MLT id must be a non-negative integer; drop it or move it to a property"
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn feature(props: &serde_json::Value) -> Feature {
        serde_json::from_value(json!({ "type": "Feature", "geometry": null, "properties": props }))
            .unwrap()
    }

    fn kinds(features: &[Feature]) -> std::collections::BTreeMap<String, PropKind> {
        Schema::infer(features)
            .unwrap()
            .columns
            .into_iter()
            .collect()
    }

    #[test]
    fn scalar_kinds_are_inferred() {
        let k = kinds(&[feature(
            &json!({ "b": true, "u": 5, "i": -3, "f": 3.5, "s": "hi" }),
        )]);
        assert_eq!(k["b"], PropKind::Bool);
        assert_eq!(k["u"], PropKind::U64);
        assert_eq!(k["i"], PropKind::I64);
        assert_eq!(k["f"], PropKind::F64);
        assert_eq!(k["s"], PropKind::Str);
    }

    #[test]
    fn columns_keep_first_seen_order() {
        let schema = Schema::infer(&[
            feature(&json!({ "b": 1 })),
            feature(&json!({ "a": 1, "b": 2 })),
        ])
        .unwrap();
        let names: Vec<_> = schema.columns.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["b", "a"]);
    }

    #[test]
    fn integers_widen_across_features() {
        assert_eq!(
            kinds(&[feature(&json!({ "n": -1 })), feature(&json!({ "n": 5 }))])["n"],
            PropKind::I64
        );
        assert_eq!(
            kinds(&[feature(&json!({ "n": 3.5 })), feature(&json!({ "n": 1 }))])["n"],
            PropKind::F64
        );
    }

    #[test]
    fn conflicting_kinds_fall_back_to_str() {
        let k = kinds(&[
            feature(&json!({ "a": true, "b": 1 })),
            feature(&json!({ "a": "x", "b": "y" })),
        ]);
        assert_eq!(k["a"], PropKind::Str);
        assert_eq!(k["b"], PropKind::Str);
    }

    #[test]
    fn an_always_null_column_is_str_and_yields_no_value() {
        let features = [
            feature(&json!({ "x": null })),
            feature(&json!({ "x": null })),
        ];
        let schema = Schema::infer(&features).unwrap();
        assert_eq!(schema.columns, vec![("x".to_string(), PropKind::Str)]);
        assert_eq!(schema.values(&features[0]).unwrap(), vec![]);
    }

    #[test]
    fn missing_keys_and_nulls_yield_no_value() {
        let schema = Schema::infer(&[feature(&json!({ "a": 1, "b": 2, "c": "s" }))]).unwrap();
        let f = feature(&json!({ "a": null, "c": "t" }));
        assert_eq!(
            schema.values(&f).unwrap(),
            vec![(2, PropValue::Str(Some("t".into())))]
        );
    }

    #[test]
    fn nested_values_are_rejected() {
        let errors: Vec<String> = [json!({ "obj": { "x": 1 } }), json!({ "arr": [1, 2] })]
            .iter()
            .map(|props| Schema::infer(&[feature(props)]).unwrap_err().to_string())
            .collect();
        insta::assert_snapshot!(errors.join("\n"), @r#"
        property "obj" holds a nested array or object, which is not supported; flatten it or remove it
        property "arr" holds a nested array or object, which is not supported; flatten it or remove it
        "#);
    }

    #[test]
    fn a_scalar_in_a_widened_str_column_keeps_its_text() {
        let features = [
            feature(&json!({ "a": true })),
            feature(&json!({ "a": "x" })),
        ];
        let schema = Schema::infer(&features).unwrap();
        assert_eq!(
            schema.values(&features[0]).unwrap(),
            vec![(0, PropValue::Str(Some("true".into())))]
        );
    }

    #[test]
    fn a_huge_unsigned_value_in_a_signed_column_is_an_error() {
        let features = [
            feature(&json!({ "n": -1 })),
            feature(&json!({ "n": u64::MAX })),
        ];
        let schema = Schema::infer(&features).unwrap();
        assert_eq!(schema.columns[0].1, PropKind::I64);
        let err = schema.values(&features[1]).unwrap_err();
        insta::assert_snapshot!(err, @r#"property "n" value 18446744073709551615 does not fit a signed 64-bit column"#);
    }

    fn with_id(id: &serde_json::Value) -> Feature {
        serde_json::from_value(
            json!({ "type": "Feature", "id": id, "geometry": null, "properties": {} }),
        )
        .unwrap()
    }

    #[test]
    fn non_negative_integer_ids_are_kept_and_an_absent_id_is_none() {
        assert_eq!(feature_id(&with_id(&json!(42))).unwrap(), Some(42));
        assert_eq!(feature_id(&with_id(&json!(0))).unwrap(), Some(0));
        assert_eq!(feature_id(&feature(&json!({}))).unwrap(), None);
    }

    #[test]
    fn other_ids_are_rejected() {
        let errors: Vec<String> = [json!(-1), json!(3.5), json!("abc")]
            .iter()
            .map(|id| feature_id(&with_id(id)).unwrap_err().to_string())
            .collect();
        insta::assert_snapshot!(errors.join("\n"), @r#"
        id -1 is not a non-negative integer, which is all an MLT id can hold
        id 3.5 is not a non-negative integer, which is all an MLT id can hold
        id "abc" is a string, but an MLT id must be a non-negative integer; drop it or move it to a property
        "#);
    }
}
