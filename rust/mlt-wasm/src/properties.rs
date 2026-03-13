use js_sys::{Array, Float32Array, Float64Array, Int8Array, Int32Array, Uint8Array, Uint32Array};
use mlt_core::v01::{ParsedProperty, StagedProperty};
use wasm_bindgen::prelude::*;

/// Cached bulk-property data for a single layer, built once on first access.
pub(crate) struct PropCache {
    /// One JS string per logical column, parallel to `columns`.
    /// `SharedDict` columns expand to one entry per sub-item
    /// (sub-item suffix appended to the parent column name).
    pub(crate) keys: Array,

    /// One typed array (or plain `Array`) per logical column, parallel to `keys`.
    /// Each array has length `feature_count`; index `i` is the value for feature `i`.
    pub(crate) columns: Array,
}

/// Build a [`PropCache`] from already-decoded property columns.
///
/// All entries in `props` must be `StagedProperty::Decoded` before calling this;
/// encoded entries are silently skipped.
pub(crate) fn build_prop_cache(props: &[StagedProperty], feature_count: u32) -> PropCache {
    let keys = Array::new();
    let columns = Array::new();

    for p in props {
        let StagedProperty::Decoded(prop) = p else {
            continue;
        };

        if let ParsedProperty::SharedDict(shared_dict) = prop {
            for item in &shared_dict.items {
                let key = format!("{}{}", prop.name(), item.suffix);
                keys.push(&JsValue::from_str(&key));

                let col = Array::new_with_length(feature_count);
                for i in 0_u32..feature_count {
                    if let Some(s) = item.get(shared_dict, i as usize) {
                        col.set(i, JsValue::from_str(s));
                    }
                }
                columns.push(&col);
            }
        } else {
            keys.push(&JsValue::from_str(prop.name()));
            columns.push(&prop_values_to_js_column(prop, feature_count));
        }
    }

    PropCache { keys, columns }
}

/// Convert an entire property column to a typed array (numeric) or plain `Array`
/// (bool / string / optional numeric).
///
/// Absent values:
/// - Float columns (`F32`, `F64`, `I64`, `U64`) → `NaN` (or `undefined` if optional)
/// - Integer columns (`I8`, `U8`, `I32`, `U32`) → `0` (or `undefined` if optional)
/// - Bool / string columns → `undefined` (Array slot left unset)
#[allow(clippy::cast_precision_loss)]
pub(crate) fn prop_values_to_js_column(prop: &ParsedProperty, n: u32) -> JsValue {
    match prop {
        ParsedProperty::Bool(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(b) = val {
                    arr.set(i, JsValue::from_bool(*b));
                }
            }
            arr.into()
        }
        ParsedProperty::I8(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(f64::from(*n)));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Int8Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::U8(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(f64::from(*n)));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Uint8Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::I32(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(f64::from(*n)));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Int32Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::U32(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(f64::from(*n)));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Uint32Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::I64(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(*n as f64));
                    }
                }
                arr.into()
            } else {
                let buf = v
                    .values
                    .iter()
                    .flatten()
                    .copied()
                    .map(|n| n as f64)
                    .collect::<Vec<_>>();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::U64(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(*n as f64));
                    }
                }
                arr.into()
            } else {
                let buf = v
                    .values
                    .iter()
                    .flatten()
                    .copied()
                    .map(|n| n as f64)
                    .collect::<Vec<_>>();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::F32(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(f64::from(*n)));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Float32Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::F64(v) => {
            if v.values.iter().any(Option::is_none) {
                let arr = Array::new_with_length(n);
                for (val, i) in v.values.iter().zip(0_u32..) {
                    if let Some(n) = val {
                        arr.set(i, JsValue::from_f64(*n));
                    }
                }
                arr.into()
            } else {
                let buf = v.values.iter().flatten().copied().collect::<Vec<_>>();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        ParsedProperty::Str(v) => {
            let arr = Array::new_with_length(n);
            for i in 0_u32..n {
                if let Some(s) = v.get(i) {
                    arr.set(i, JsValue::from_str(s));
                }
            }
            arr.into()
        }
        ParsedProperty::SharedDict(..) => {
            unreachable!("SharedDict is expanded by build_prop_cache before reaching here.")
        }
    }
}

/// Convert a single column value at row `i` to a JS primitive.
///
/// Used only by the compatibility [`crate::tile::MltTile::feature_properties`]
/// path.  Returns `None` for absent values so the caller can omit the key from
/// the output object entirely, matching `@mapbox/vector-tile` behaviour.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn prop_to_js(prop: &ParsedProperty, i: usize) -> Option<JsValue> {
    match prop {
        ParsedProperty::Bool(v) => v.values[i].map(JsValue::from_bool),
        ParsedProperty::I8(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        ParsedProperty::U8(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        ParsedProperty::I32(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        ParsedProperty::U32(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        // i64/u64 may lose precision beyond 2^53; matches the TS decoder and the
        // VectorTileFeatureLike contract (properties typed as `number | string | boolean`).
        ParsedProperty::I64(v) => v.values[i].map(|n| JsValue::from_f64(n as f64)),
        ParsedProperty::U64(v) => v.values[i].map(|n| JsValue::from_f64(n as f64)),
        ParsedProperty::F32(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        ParsedProperty::F64(v) => v.values[i].map(JsValue::from_f64),
        ParsedProperty::Str(v) => u32::try_from(i)
            .ok()
            .and_then(|i| v.get(i))
            .map(JsValue::from_str),
        ParsedProperty::SharedDict(..) => None,
    }
}
