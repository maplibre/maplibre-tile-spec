use js_sys::{Array, Float32Array, Float64Array, Int8Array, Int32Array, Uint8Array, Uint32Array};
use mlt_core::v01::{DecodedProperty, OwnedProperty};
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
/// All entries in `props` must be `OwnedProperty::Decoded` before calling this;
/// encoded entries are silently skipped.
pub(crate) fn build_prop_cache(props: &[OwnedProperty], feature_count: u32) -> PropCache {
    let keys = Array::new();
    let columns = Array::new();

    for p in props {
        let OwnedProperty::Decoded(prop) = p else {
            continue;
        };

        if let DecodedProperty::SharedDict(shared_dict) = prop {
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
pub(crate) fn prop_values_to_js_column(prop: &DecodedProperty, n: u32) -> JsValue {
    match prop {
        DecodedProperty::Bool(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                arr.set(i, JsValue::from_bool(*val));
            }
            arr.into()
        }
        DecodedProperty::BoolOpt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(b) = val {
                    arr.set(i, JsValue::from_bool(*b));
                }
            }
            arr.into()
        }
        DecodedProperty::I8(v) => Int8Array::from(v.values.as_slice()).into(),
        DecodedProperty::I8Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(f64::from(*n)));
                }
            }
            arr.into()
        }
        DecodedProperty::U8(v) => Uint8Array::from(v.values.as_slice()).into(),
        DecodedProperty::U8Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(f64::from(*n)));
                }
            }
            arr.into()
        }
        DecodedProperty::I32(v) => Int32Array::from(v.values.as_slice()).into(),
        DecodedProperty::I32Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(f64::from(*n)));
                }
            }
            arr.into()
        }
        DecodedProperty::U32(v) => Uint32Array::from(v.values.as_slice()).into(),
        DecodedProperty::U32Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(f64::from(*n)));
                }
            }
            arr.into()
        }
        DecodedProperty::I64(v) => {
            let buf = v
                .values
                .iter()
                .copied()
                .map(|n| n as f64)
                .collect::<Vec<_>>();
            Float64Array::from(buf.as_slice()).into()
        }
        DecodedProperty::I64Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(*n as f64));
                }
            }
            arr.into()
        }
        DecodedProperty::U64(v) => {
            let buf = v
                .values
                .iter()
                .copied()
                .map(|n| n as f64)
                .collect::<Vec<_>>();
            Float64Array::from(buf.as_slice()).into()
        }
        DecodedProperty::U64Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(*n as f64));
                }
            }
            arr.into()
        }
        DecodedProperty::F32(v) => Float32Array::from(v.values.as_slice()).into(),
        DecodedProperty::F32Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(f64::from(*n)));
                }
            }
            arr.into()
        }
        DecodedProperty::F64(v) => Float64Array::from(v.values.as_slice()).into(),
        DecodedProperty::F64Opt(v) => {
            let arr = Array::new_with_length(n);
            for (val, i) in v.values.iter().zip(0_u32..) {
                if let Some(n) = val {
                    arr.set(i, JsValue::from_f64(*n));
                }
            }
            arr.into()
        }
        DecodedProperty::Str(v) => {
            let arr = Array::new_with_length(n);
            for i in 0_u32..n {
                if let Some(s) = v.get(i) {
                    arr.set(i, JsValue::from_str(s));
                }
            }
            arr.into()
        }
        DecodedProperty::SharedDict(..) => {
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
pub(crate) fn prop_to_js(prop: &DecodedProperty, i: usize) -> Option<JsValue> {
    match prop {
        DecodedProperty::Bool(v) => Some(JsValue::from_bool(v.values[i])),
        DecodedProperty::BoolOpt(v) => v.values[i].map(JsValue::from_bool),
        DecodedProperty::I8(v) => Some(JsValue::from_f64(f64::from(v.values[i]))),
        DecodedProperty::I8Opt(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        DecodedProperty::U8(v) => Some(JsValue::from_f64(f64::from(v.values[i]))),
        DecodedProperty::U8Opt(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        DecodedProperty::I32(v) => Some(JsValue::from_f64(f64::from(v.values[i]))),
        DecodedProperty::I32Opt(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        DecodedProperty::U32(v) => Some(JsValue::from_f64(f64::from(v.values[i]))),
        DecodedProperty::U32Opt(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        // i64/u64 may lose precision beyond 2^53; matches the TS decoder and the
        // VectorTileFeatureLike contract (properties typed as `number | string | boolean`).
        DecodedProperty::I64(v) => Some(JsValue::from_f64(v.values[i] as f64)),
        DecodedProperty::I64Opt(v) => v.values[i].map(|n| JsValue::from_f64(n as f64)),
        DecodedProperty::U64(v) => Some(JsValue::from_f64(v.values[i] as f64)),
        DecodedProperty::U64Opt(v) => v.values[i].map(|n| JsValue::from_f64(n as f64)),
        DecodedProperty::F32(v) => Some(JsValue::from_f64(f64::from(v.values[i]))),
        DecodedProperty::F32Opt(v) => v.values[i].map(|n| JsValue::from_f64(f64::from(n))),
        DecodedProperty::F64(v) => Some(JsValue::from_f64(v.values[i])),
        DecodedProperty::F64Opt(v) => v.values[i].map(JsValue::from_f64),
        DecodedProperty::Str(v) => u32::try_from(i)
            .ok()
            .and_then(|i| v.get(i))
            .map(JsValue::from_str),
        DecodedProperty::SharedDict(..) => None,
    }
}
