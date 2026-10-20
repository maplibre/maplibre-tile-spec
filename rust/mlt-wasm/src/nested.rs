//! Nested columns cross as owned JS values and their physical schema. Each node is walked
//! once, without rebuilding rows through prefix scans of the presence and length streams.

use js_sys::{Array, Object};
use mlt_core::{MValues, ParsedInterior, ParsedNested, ParsedNode};
use wasm_bindgen::prelude::*;

use crate::columns::{js_index, set, type_name};

pub(crate) fn column(
    column: &ParsedNested<'_>,
    feature_count: usize,
    exact_int64: bool,
) -> Result<Object, JsError> {
    let (schema, dense) = interior(column.root(), exact_int64)?;
    let values = if dense.length() == js_index(feature_count) {
        dense
    } else {
        expand(&dense, (0..feature_count).map(|i| column.is_present(i)))
    };
    let object = Object::new();
    set(&object, "name", &column.name().into());
    set(&object, "type", &"nested".into());
    set(&object, "schema", &schema);
    set(&object, "values", &values);
    Ok(object)
}

fn schema(kind: &str) -> Object {
    let object = Object::new();
    set(&object, "kind", &kind.into());
    object
}

/// Keys are data, including `__proto__` and `constructor`.
fn record() -> Object {
    Object::create(JsValue::NULL.unchecked_ref())
}

fn node(node: &ParsedNode<'_>, exact_int64: bool) -> Result<(Object, Array), JsError> {
    match node {
        ParsedNode::Interior(value) => interior(value, exact_int64),
        ParsedNode::Leaf(leaf) => {
            let schema = schema("leaf");
            set(&schema, "type", &type_name(leaf.values().kind()).into());
            let values = leaves(leaf.values(), exact_int64);
            Ok((schema, nullable(values, leaf.presence())))
        }
    }
}

fn interior(tree: &ParsedInterior<'_>, exact_int64: bool) -> Result<(Object, Array), JsError> {
    match tree {
        ParsedInterior::Struct(value) => {
            let schema = schema("struct");
            let fields = Array::new();
            let count = value.presence().map_or_else(
                || tree.parent_count(),
                |bits| bits.iter().filter(|&&present| present).count(),
            );
            let rows = Array::new_with_length(js_index(count));
            for i in 0..rows.length() {
                rows.set(i, record().into());
            }
            for (name, child) in value.fields() {
                let (child_schema, values) = node(child, exact_int64)?;
                let field = Object::new();
                set(&field, "name", &(*name).into());
                set(&field, "schema", &child_schema);
                fields.push(&field);
                for i in 0..rows.length() {
                    set(rows.get(i).unchecked_ref(), name, &values.get(i));
                }
            }
            set(&schema, "fields", &fields);
            Ok((schema, nullable(rows, value.presence())))
        }
        ParsedInterior::List(value) => {
            let schema = schema("list");
            let (element_schema, values) = node(value.element(), exact_int64)?;
            set(&schema, "element", &element_schema);
            let rows = Array::new();
            let mut start = 0;
            for &length in value.lengths() {
                rows.push(&values.slice(start, start + length));
                start += length;
            }
            Ok((schema, nullable(rows, value.presence())))
        }
        ParsedInterior::Map(value) => {
            let schema = schema("map");
            let (value_schema, values) = node(value.value(), exact_int64)?;
            set(&schema, "value", &value_schema);
            let rows = Array::new();
            let mut start = 0;
            for &length in value.lengths() {
                let row = record();
                for i in start..start + length {
                    let key = value
                        .key(i as usize)
                        .ok_or_else(|| JsError::new("a nested map entry has no key"))?;
                    set(&row, key, &values.get(i));
                }
                rows.push(&row);
                start += length;
            }
            Ok((schema, nullable(rows, value.presence())))
        }
    }
}

fn nullable(values: Array, presence: Option<&[bool]>) -> Array {
    match presence {
        None => values,
        Some(bits) => expand(&values, bits.iter().copied()),
    }
}

/// Fill absent slots with null, without copying any object or array held by a present slot.
fn expand(values: &Array, presence: impl Iterator<Item = bool>) -> Array {
    let rows = Array::new();
    let mut dense = 0;
    for present in presence {
        if present {
            rows.push(&values.get(dense));
            dense += 1;
        } else {
            rows.push(&JsValue::NULL);
        }
    }
    rows
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the default numeric API retains its f64 representation"
)]
fn leaves(values: &MValues<'_>, exact_int64: bool) -> Array {
    let array = Array::new();
    macro_rules! push {
        ($values:expr) => {
            for &value in $values {
                array.push(&JsValue::from(value));
            }
        };
    }
    macro_rules! integer64 {
        ($values:expr) => {
            for &value in $values {
                array.push(&if exact_int64 {
                    JsValue::from(value)
                } else {
                    JsValue::from_f64(value as f64)
                });
            }
        };
    }
    match values {
        MValues::Bool(values) => push!(values),
        MValues::I8(values) => push!(values),
        MValues::U8(values) => push!(values),
        MValues::I32(values) => push!(values),
        MValues::U32(values) => push!(values),
        MValues::I64(values) => integer64!(values),
        MValues::U64(values) => integer64!(values),
        MValues::F32(values) => push!(values),
        MValues::F64(values) => push!(values),
        MValues::Str(values) => {
            for i in 0..values.feature_count() {
                array.push(&JsValue::from_str(
                    values.get(js_index(i)).expect("decoded string entry"),
                ));
            }
        }
    }
    array
}
