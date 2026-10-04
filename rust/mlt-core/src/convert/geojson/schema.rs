//! Property schema inference and value conversion for `GeoJSON` properties.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::convert::infer::{ColumnInference, InferredKind};
use crate::tile::{PropKind, PropValue};
use crate::{MltError, MltResult};

/// The property columns of a set of `GeoJSON` features: names in first-seen
/// order, each with one [`PropKind`] shared by every feature.
///
/// `GeoJSON` properties are dynamically typed, so each column's kind is
/// inferred from every value it holds with the MVT importer's rules: signed
/// and unsigned integers widen to `I64`, integers mixed with floats widen to
/// `F64`, and any other conflict falls back to `Str`, as does a column that is
/// only ever `null`. Nested arrays and objects
/// are rejected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertySchema {
    names: Vec<String>,
    kinds: Vec<PropKind>,
    /// Each name's column, so a feature's values cost its own keys, not every column.
    index: HashMap<String, usize>,
}

impl PropertySchema {
    /// Infer the schema from each feature's properties.
    pub fn infer<'a, F, P>(features: F) -> MltResult<Self>
    where
        F: IntoIterator<Item = P>,
        P: IntoIterator<Item = (&'a String, &'a Value)>,
    {
        let mut columns = ColumnInference::default();
        for properties in features {
            for (name, value) in properties {
                columns.observe(name, kind_of(name, value)?);
            }
        }
        let (names, kinds) = columns.finish();
        let index = names.iter().cloned().zip(0..).collect();
        Ok(Self {
            names,
            kinds,
            index,
        })
    }

    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    #[must_use]
    pub fn kinds(&self) -> &[PropKind] {
        &self.kinds
    }

    /// One feature's non-null values as `(column index, value)` pairs, in
    /// schema order. Absent keys, JSON nulls, and keys outside the schema are
    /// left out; a layer builder reads a missing column as its typed null.
    pub fn values(&self, properties: &Map<String, Value>) -> MltResult<Vec<(usize, PropValue)>> {
        let mut out = Vec::with_capacity(properties.len());
        for (name, value) in properties {
            if let Some(&idx) = self.index.get(name)
                && !value.is_null()
            {
                out.push((idx, convert(self.kinds[idx], name, value)?));
            }
        }
        out.sort_unstable_by_key(|&(idx, _)| idx);
        Ok(out)
    }
}

/// The column kind a single JSON value asks for.
fn kind_of(name: &str, value: &Value) -> MltResult<InferredKind> {
    Ok(match value {
        Value::Null => InferredKind::Unknown,
        Value::Bool(_) => InferredKind::Bool,
        Value::Number(n) if n.is_u64() => InferredKind::U64,
        Value::Number(n) if n.is_i64() => InferredKind::I64,
        Value::Number(_) => InferredKind::F64,
        Value::String(_) => InferredKind::Str,
        Value::Array(_) | Value::Object(_) => {
            return Err(MltError::NestedPropertyValue { name: name.into() });
        }
    })
}

/// Convert one non-null JSON value to the column's kind.
fn convert(kind: PropKind, name: &str, value: &Value) -> MltResult<PropValue> {
    let converted = match (kind, value) {
        (PropKind::Bool, Value::Bool(b)) => Some(PropValue::Bool(Some(*b))),
        (PropKind::I64, Value::Number(n)) => n.as_i64().map(|i| PropValue::I64(Some(i))),
        (PropKind::U64, Value::Number(n)) => n.as_u64().map(|u| PropValue::U64(Some(u))),
        (PropKind::F64, Value::Number(n)) => n.as_f64().map(|f| PropValue::F64(Some(f))),
        (PropKind::Str, Value::String(s)) => Some(PropValue::Str(Some(s.clone()))),
        // A scalar in a column that widened to `Str` keeps its JSON text.
        (PropKind::Str, other @ (Value::Bool(_) | Value::Number(_))) => {
            Some(PropValue::Str(Some(other.to_string())))
        }
        // Inference merges every value a column holds, so only a number out of
        // its widened column's range gets here.
        _ => None,
    };
    converted.ok_or_else(|| MltError::PropertyValueMismatch {
        name: name.into(),
        value: value.to_string(),
        kind,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::assert_snapshot;
    use serde_json::json;

    use super::*;

    fn props(value: &Value) -> Map<String, Value> {
        value.as_object().expect("a JSON object").clone()
    }

    fn infer(features: &[Map<String, Value>]) -> MltResult<PropertySchema> {
        PropertySchema::infer(features)
    }

    fn kinds(features: &[Map<String, Value>]) -> BTreeMap<String, PropKind> {
        let schema = infer(features).unwrap();
        schema
            .names()
            .iter()
            .cloned()
            .zip(schema.kinds().iter().copied())
            .collect()
    }

    #[test]
    fn scalar_kinds_are_inferred() {
        let k = kinds(&[props(
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
        let schema =
            infer(&[props(&json!({ "b": 1 })), props(&json!({ "a": 1, "b": 2 }))]).unwrap();
        assert_eq!(schema.names(), ["b", "a"]);
    }

    #[test]
    fn signed_and_unsigned_integers_widen_to_i64() {
        assert_eq!(
            kinds(&[props(&json!({ "n": -1 })), props(&json!({ "n": 5 }))])["n"],
            PropKind::I64
        );
    }

    #[test]
    fn conflicting_kinds_fall_back_to_str() {
        let k = kinds(&[
            props(&json!({ "a": true, "b": 1, "c": 3.5 })),
            props(&json!({ "a": "x", "b": "y", "c": 1 })),
        ]);
        assert_eq!(k["a"], PropKind::Str);
        assert_eq!(k["b"], PropKind::Str);
        assert_eq!(k["c"], PropKind::Str);
    }

    #[test]
    fn an_always_null_column_is_str_and_yields_no_value() {
        let features = [props(&json!({ "x": null })), props(&json!({ "x": null }))];
        let schema = infer(&features).unwrap();
        assert_eq!(schema.names(), ["x"]);
        assert_eq!(schema.kinds(), [PropKind::Str]);
        assert_eq!(schema.values(&features[0]).unwrap(), []);
    }

    #[test]
    fn missing_keys_and_nulls_yield_no_value() {
        let schema = infer(&[props(&json!({ "a": 1, "b": true, "c": "s" }))]).unwrap();
        assert_eq!(
            schema
                .values(&props(&json!({ "a": null, "c": "t" })))
                .unwrap(),
            [(2, PropValue::Str(Some("t".into())))]
        );
    }

    #[test]
    fn nested_values_are_rejected() {
        let errors: Vec<String> = [json!({ "obj": { "x": 1 } }), json!({ "arr": [1, 2] })]
            .iter()
            .map(|p| infer(&[props(p)]).unwrap_err().to_string())
            .collect();
        assert_snapshot!(errors.join("\n"), @r#"
        property "obj" holds a nested array or object, which is not supported; flatten it or remove it
        property "arr" holds a nested array or object, which is not supported; flatten it or remove it
        "#);
    }

    #[test]
    fn a_scalar_in_a_widened_str_column_keeps_its_text() {
        let features = [props(&json!({ "a": true })), props(&json!({ "a": "x" }))];
        let schema = infer(&features).unwrap();
        assert_eq!(
            schema.values(&features[0]).unwrap(),
            [(0, PropValue::Str(Some("true".into())))]
        );
    }

    #[test]
    fn a_huge_unsigned_value_in_a_signed_column_is_an_error() {
        let features = [props(&json!({ "n": -1 })), props(&json!({ "n": u64::MAX }))];
        let schema = infer(&features).unwrap();
        assert_eq!(schema.kinds(), [PropKind::I64]);
        let err = schema.values(&features[1]).unwrap_err();
        assert_snapshot!(err, @r#"property "n" value 18446744073709551615 does not fit its I64 column"#);
    }
}
