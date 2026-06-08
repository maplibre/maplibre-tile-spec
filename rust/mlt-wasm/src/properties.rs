use js_sys::{Array, Float64Array, Int8Array, Int32Array, Uint8Array, Uint32Array};
use mlt_core::{PropValue, TileLayer};
use wasm_bindgen::prelude::*;

/// Cached bulk-property data for a single layer.
pub(crate) struct PropCache {
    /// One JS string per logical column, parallel to `columns`.
    pub(crate) keys: Array,

    /// One typed array (or plain `Array`) per logical column, parallel to `keys`.
    /// Each array has length `feature_count`; index `i` is the value for feature `i`.
    pub(crate) columns: Array,
}

/// Build a [`PropCache`] from a fully decoded [`TileLayer`].
pub(crate) fn build_prop_cache(tile: &TileLayer) -> PropCache {
    let n = tile.features.len();
    let keys = Array::new();
    let columns = Array::new();

    for (col_idx, name) in tile.property_names.iter().enumerate() {
        keys.push(&JsValue::from_str(name));
        columns.push(&build_column(tile, col_idx, n));
    }

    PropCache { keys, columns }
}

fn feature_count_u32(n: usize) -> u32 {
    u32::try_from(n).expect("feature count fits in u32")
}

fn idx_u32(i: usize) -> u32 {
    u32::try_from(i).expect("index fits in u32")
}

#[allow(clippy::cast_precision_loss)]
fn build_column(tile: &TileLayer, col_idx: usize, n: usize) -> JsValue {
    // Peek at the first feature to determine the column variant.
    let first = tile
        .features
        .first()
        .and_then(|f| f.properties.get(col_idx));

    match first {
        Some(PropValue::Bool(_)) => {
            let arr = Array::new_with_length(feature_count_u32(n));
            for (i, f) in tile.features.iter().enumerate() {
                if let Some(PropValue::Bool(Some(b))) = f.properties.get(col_idx) {
                    arr.set(idx_u32(i), JsValue::from_bool(*b));
                }
            }
            arr.into()
        }
        Some(PropValue::I8(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::I8(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::I8(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(f64::from(*v)));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<i8> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::I8(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                Int8Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::U8(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::U8(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::U8(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(f64::from(*v)));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<u8> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::U8(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                Uint8Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::I32(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::I32(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::I32(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(f64::from(*v)));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<i32> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::I32(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                Int32Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::U32(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::U32(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::U32(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(f64::from(*v)));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<u32> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::U32(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                Uint32Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::I64(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::I64(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::I64(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(*v as f64));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<f64> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::I64(v)) = f.properties.get(col_idx) {
                            v.map(|n| n as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::U64(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::U64(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::U64(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(*v as f64));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<f64> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::U64(v)) = f.properties.get(col_idx) {
                            v.map(|n| n as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::F32(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::F32(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::F32(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(f64::from(*v)));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<f32> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::F32(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                js_sys::Float32Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::F64(_)) => {
            let any_none = tile
                .features
                .iter()
                .any(|f| matches!(f.properties.get(col_idx), Some(PropValue::F64(None))));
            if any_none {
                let arr = Array::new_with_length(feature_count_u32(n));
                for (i, f) in tile.features.iter().enumerate() {
                    if let Some(PropValue::F64(Some(v))) = f.properties.get(col_idx) {
                        arr.set(idx_u32(i), JsValue::from_f64(*v));
                    }
                }
                arr.into()
            } else {
                let buf: Vec<f64> = tile
                    .features
                    .iter()
                    .filter_map(|f| {
                        if let Some(PropValue::F64(v)) = f.properties.get(col_idx) {
                            *v
                        } else {
                            None
                        }
                    })
                    .collect();
                Float64Array::from(buf.as_slice()).into()
            }
        }
        Some(PropValue::Str(_)) | None => {
            let arr = Array::new_with_length(feature_count_u32(n));
            for (i, f) in tile.features.iter().enumerate() {
                if let Some(PropValue::Str(Some(s))) = f.properties.get(col_idx) {
                    arr.set(idx_u32(i), JsValue::from_str(s));
                }
            }
            arr.into()
        }
    }
}

/// Convert a single [`PropValue`] to a JS primitive for the per-feature
/// compatibility API.  Returns `None` for absent values.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn prop_value_to_js(val: &PropValue) -> Option<JsValue> {
    match val {
        PropValue::Bool(v) => v.map(JsValue::from_bool),
        PropValue::I8(v) => v.map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U8(v) => v.map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::I32(v) => v.map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U32(v) => v.map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::I64(v) => v.map(|n| JsValue::from_f64(n as f64)),
        PropValue::U64(v) => v.map(|n| JsValue::from_f64(n as f64)),
        PropValue::F32(v) => v.map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::F64(v) => v.map(JsValue::from_f64),
        PropValue::Str(v) => v.as_deref().map(JsValue::from_str),
    }
}
