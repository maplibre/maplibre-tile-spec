use js_sys::{Array, Float32Array, Float64Array, Int32Array, Uint8Array, Uint32Array};
use mlt_core::v01::{OwnedProperty, PropValue};
use wasm_bindgen::prelude::*;

/// Cached bulk-property data for a single layer, built once on first access.
pub(crate) struct PropCache {
    /// One JS string per logical column, parallel to `columns`.
    /// `SharedDict` columns expand to one entry per sub-item
    /// (sub-item suffix appended to the parent column name).
    pub(crate) keys: Array,

    /// One typed array (or plain `Array`) per logical column, parallel to `keys`.
    /// Each array has length `feature_count`; index `i` is the value for feature `i`.
    ///
    /// Type mapping:
    /// - `Bool`       → plain `Array` of `true`/`false`  (absent = `undefined`)
    /// - `I8`/`I32`   → `Int32Array`                     (absent = `0`)
    /// - `U8`         → `Uint8Array`                     (absent = `0`)
    /// - `U32`        → `Uint32Array`                    (absent = `0`)
    /// - `I64`/`U64`  → `Float64Array`                   (absent = `NaN`, precision-loss accepted)
    /// - `F32`        → `Float32Array`                   (absent = `NaN`)
    /// - `F64`        → `Float64Array`                   (absent = `NaN`)
    /// - `Str`        → plain `Array` of strings         (absent = `undefined`)
    /// - `SharedDict` → plain `Array` of strings         (absent = `undefined`)
    pub(crate) columns: Array,
}

/// Build a [`PropCache`] from already-decoded property columns.
///
/// All entries in `props` must be `OwnedProperty::Decoded` before calling this;
/// encoded entries are silently skipped.
pub(crate) fn build_prop_cache(props: &[OwnedProperty], feature_count: usize) -> PropCache {
    let keys = Array::new();
    let columns = Array::new();

    for p in props {
        let OwnedProperty::Decoded(prop) = p else {
            continue;
        };

        match &prop.values {
            PropValue::SharedDict(items) => {
                for item in items {
                    let key = format!("{}{}", prop.name, item.suffix);
                    keys.push(&JsValue::from_str(&key));

                    let col = Array::new_with_length(feature_count as u32);
                    for (i, v) in item.values.iter().enumerate() {
                        if let Some(s) = v {
                            col.set(i as u32, JsValue::from_str(s));
                        }
                        // absent → undefined (Array slot default)
                    }
                    columns.push(&col);
                }
            }
            _ => {
                keys.push(&JsValue::from_str(&prop.name));
                columns.push(&prop_values_to_js_column(&prop.values, feature_count));
            }
        }
    }

    PropCache { keys, columns }
}

/// Convert an entire property column to a typed array (numeric) or plain `Array`
/// (bool / string).
///
/// Absent values:
/// - Float columns (`F32`, `F64`, `I64`, `U64`) → `NaN`
/// - Integer columns (`I8`, `U8`, `I32`, `U32`) → `0`
/// - Bool / string columns → `undefined` (Array slot left unset)
#[allow(clippy::cast_precision_loss)]
pub(crate) fn prop_values_to_js_column(pv: &PropValue, n: usize) -> JsValue {
    match pv {
        PropValue::Bool(v) => {
            let arr = Array::new_with_length(n as u32);
            for (i, val) in v.iter().enumerate() {
                if let Some(b) = val {
                    arr.set(i as u32, JsValue::from_bool(*b));
                }
            }
            arr.into()
        }
        PropValue::I8(v) => {
            let buf: Vec<i32> = v.iter().map(|x| x.map_or(0, |n| i32::from(n))).collect();
            Int32Array::from(buf.as_slice()).into()
        }
        PropValue::U8(v) => {
            let buf: Vec<u8> = v.iter().map(|x| x.unwrap_or(0)).collect();
            Uint8Array::from(buf.as_slice()).into()
        }
        PropValue::I32(v) => {
            let buf: Vec<i32> = v.iter().map(|x| x.unwrap_or(0)).collect();
            Int32Array::from(buf.as_slice()).into()
        }
        PropValue::U32(v) => {
            let buf: Vec<u32> = v.iter().map(|x| x.unwrap_or(0)).collect();
            Uint32Array::from(buf.as_slice()).into()
        }
        PropValue::I64(v) => {
            let buf: Vec<f64> = v.iter().map(|x| x.map_or(f64::NAN, |n| n as f64)).collect();
            Float64Array::from(buf.as_slice()).into()
        }
        PropValue::U64(v) => {
            let buf: Vec<f64> = v.iter().map(|x| x.map_or(f64::NAN, |n| n as f64)).collect();
            Float64Array::from(buf.as_slice()).into()
        }
        PropValue::F32(v) => {
            let buf: Vec<f32> = v.iter().map(|x| x.unwrap_or(f32::NAN)).collect();
            Float32Array::from(buf.as_slice()).into()
        }
        PropValue::F64(v) => {
            let buf: Vec<f64> = v.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
            Float64Array::from(buf.as_slice()).into()
        }
        PropValue::Str(v) => {
            let arr = Array::new_with_length(n as u32);
            for (i, val) in v.iter().enumerate() {
                if let Some(s) = val {
                    arr.set(i as u32, JsValue::from_str(s));
                }
            }
            arr.into()
        }
        PropValue::SharedDict(_) => {
            // SharedDict is expanded by build_prop_cache before reaching here.
            Array::new().into()
        }
    }
}

/// Convert a single column value at row `i` to a JS primitive.
///
/// Used only by the compatibility [`crate::tile::MltTile::feature_properties`]
/// path.  Returns `None` for absent values so the caller can omit the key from
/// the output object entirely, matching `@mapbox/vector-tile` behaviour.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn prop_to_js(pv: &PropValue, i: usize) -> Option<JsValue> {
    match pv {
        PropValue::Bool(v) => v[i].map(JsValue::from_bool),
        PropValue::I8(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U8(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::I32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        // i64/u64 may lose precision beyond 2^53; matches the TS decoder and the
        // VectorTileFeatureLike contract (properties typed as `number | string | boolean`).
        PropValue::I64(v) => v[i].map(|n| JsValue::from_f64(n as f64)),
        PropValue::U64(v) => v[i].map(|n| JsValue::from_f64(n as f64)),
        PropValue::F32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::F64(v) => v[i].map(JsValue::from_f64),
        PropValue::Str(v) => v[i].as_ref().map(|s| JsValue::from_str(s)),
        PropValue::SharedDict(_) => None,
    }
}
