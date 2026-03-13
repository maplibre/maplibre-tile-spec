//! Row-oriented "source form" for the encoder.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! ## Pipeline
//!
//! **Decoding** (one-way, no `to_owned`):
//! `raw bytes` → `Layer01<'a>` → `TileLayer01` (via `TileLayer01::from_layer01`)
//!
//! **Encoding**:
//! `TileLayer01` → `StagedLayer01` → (encode) → `EncodedLayer01` → `write_to` bytes

use geo_types::Geometry;

use crate::v01::{
    Layer01, ParsedGeometry, ParsedId, ParsedProperty, StagedLayer01, StagedProperty, StagedScalar,
    StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
use crate::{EncDec, MltError};

/// Row-oriented working form for the encoder.
///
/// All features are stored as a flat [`Vec<TileFeature>`] so that sorting is
/// a single `sort_by_cached_key` call.  The `property_names` vec is parallel
/// to every `TileFeature::properties` slice in this layer.
#[derive(Debug, Clone)]
pub struct TileLayer01 {
    pub name: String,
    pub extent: u32,
    /// Column names, parallel to `TileFeature::properties`.
    pub property_names: Vec<String>,
    pub features: Vec<TileFeature>,
}

/// A single map feature in row form.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFeature {
    pub id: Option<u64>,
    /// Geometry in `geo_types` / `Geom32` form.
    pub geometry: Geometry<i32>,
    /// One value per property column, in the same order as
    /// [`TileLayer01::property_names`].
    pub properties: Vec<PropValue>,
}

/// A single typed value for one property of one feature.
///
/// Mirrors the scalar variants of [`ParsedProperty`] at the per-feature
/// level. `SharedDict` items are flattened: each sub-field becomes its own
/// `PropValue::Str` entry in `TileFeature::properties`, with the
/// corresponding entry in `TileLayer01::property_names` set to
/// `"prefix:suffix"`.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Bool(Option<bool>),
    I8(Option<i8>),
    U8(Option<u8>),
    I32(Option<i32>),
    U32(Option<u32>),
    I64(Option<i64>),
    U64(Option<u64>),
    F32(Option<f32>),
    F64(Option<f64>),
    Str(Option<String>),
}

// ── Layer01 → TileLayer01 ────────────────────────────────────────────────────

impl TileLayer01 {
    /// Decode a fully-decoded `Layer01` into row-oriented `TileLayer01`.
    ///
    /// The caller must ensure the layer has been fully decoded (via
    /// `layer.decode_all()`) before calling this — geometry and properties
    /// must be in the `Decoded` variant.
    pub fn from_layer01(layer: Layer01<'_>) -> Result<Self, MltError> {
        let geom = match layer.geometry {
            EncDec::Decoded(g) => g,
            EncDec::Encoded(_) => return Err(MltError::NotDecoded("geometry")),
        };

        let n = geom.vector_types.len();

        let mut property_names: Vec<String> = Vec::new();
        let mut decoded_props: Vec<ParsedProperty<'_>> = Vec::new();

        for prop in layer.properties {
            match prop {
                EncDec::Decoded(dp) => {
                    match &dp {
                        ParsedProperty::SharedDict(sd) => {
                            for item in &sd.items {
                                property_names.push(format!(
                                    "{}:{}",
                                    sd.prefix.as_ref(),
                                    item.suffix.as_ref()
                                ));
                            }
                        }
                        other => property_names.push(other.name().to_string()),
                    }
                    decoded_props.push(dp);
                }
                EncDec::Encoded(_) => return Err(MltError::NotDecoded("property")),
            }
        }

        let ids: Option<Vec<Option<u64>>> = match layer.id {
            Some(EncDec::Decoded(d)) => Some(d.0),
            None => None,
            Some(EncDec::Encoded(_)) => return Err(MltError::NotDecoded("id")),
        };

        let mut features = Vec::with_capacity(n);
        for i in 0..n {
            let id = ids.as_ref().and_then(|ids| ids.get(i).copied().flatten());
            let geometry = geom.to_geojson(i)?;
            let mut properties = Vec::with_capacity(property_names.len());

            for prop in &decoded_props {
                extract_values(prop, i, &mut properties);
            }

            features.push(TileFeature {
                id,
                geometry,
                properties,
            });
        }

        Ok(TileLayer01 {
            name: layer.name.to_string(),
            extent: layer.extent,
            property_names,
            features,
        })
    }
}

/// Extract the per-feature value at index `i` from a decoded property column
/// and push it (or them, for `SharedDict`) into `out`.
fn extract_values(prop: &ParsedProperty<'_>, i: usize, out: &mut Vec<PropValue>) {
    match prop {
        ParsedProperty::Bool(s) => out.push(PropValue::Bool(s.values[i])),
        ParsedProperty::I8(s) => out.push(PropValue::I8(s.values[i])),
        ParsedProperty::U8(s) => out.push(PropValue::U8(s.values[i])),
        ParsedProperty::I32(s) => out.push(PropValue::I32(s.values[i])),
        ParsedProperty::U32(s) => out.push(PropValue::U32(s.values[i])),
        ParsedProperty::I64(s) => out.push(PropValue::I64(s.values[i])),
        ParsedProperty::U64(s) => out.push(PropValue::U64(s.values[i])),
        ParsedProperty::F32(s) => out.push(PropValue::F32(s.values[i])),
        ParsedProperty::F64(s) => out.push(PropValue::F64(s.values[i])),
        ParsedProperty::Str(s) => {
            let val = s
                .get(u32::try_from(i).unwrap_or(u32::MAX))
                .map(str::to_string);
            out.push(PropValue::Str(val));
        }
        ParsedProperty::SharedDict(sd) => {
            for item in &sd.items {
                let val = item.get(sd, i).map(str::to_string);
                out.push(PropValue::Str(val));
            }
        }
    }
}

// ── StagedLayer01 → TileLayer01 ─────────────────────────────────────────────

impl From<StagedLayer01> for TileLayer01 {
    /// Convert a staged (columnar) layer into row-oriented form for sorting.
    ///
    /// # Panics
    ///
    /// Panics if any geometry index is out of bounds. This is only used during
    /// the encoding pipeline where geometry is always valid.
    fn from(layer: StagedLayer01) -> Self {
        let geom = &layer.geometry;
        let n = geom.vector_types.len();

        let mut property_names: Vec<String> = Vec::new();
        for prop in &layer.properties {
            match prop {
                StagedProperty::SharedDict(sd) => {
                    for item in &sd.items {
                        property_names.push(format!("{}:{}", sd.prefix, item.suffix));
                    }
                }
                other => property_names.push(other.name().to_string()),
            }
        }

        let ids: Option<&[Option<u64>]> = layer.id.as_ref().map(|id| id.0.as_slice());

        let mut features = Vec::with_capacity(n);
        for i in 0..n {
            let id = ids.and_then(|ids| ids.get(i).copied().flatten());
            let geometry = geom
                .to_geojson(i)
                .expect("geometry index in bounds during encode");
            let mut properties = Vec::with_capacity(property_names.len());
            for prop in &layer.properties {
                staged_extract_values(prop, i, &mut properties);
            }
            features.push(TileFeature {
                id,
                geometry,
                properties,
            });
        }

        TileLayer01 {
            name: layer.name,
            extent: layer.extent,
            property_names,
            features,
        }
    }
}

/// Extract the per-feature value at index `i` from a `StagedProperty` column.
fn staged_extract_values(prop: &StagedProperty, i: usize, out: &mut Vec<PropValue>) {
    match prop {
        StagedProperty::Bool(s) => out.push(PropValue::Bool(s.values[i])),
        StagedProperty::I8(s) => out.push(PropValue::I8(s.values[i])),
        StagedProperty::U8(s) => out.push(PropValue::U8(s.values[i])),
        StagedProperty::I32(s) => out.push(PropValue::I32(s.values[i])),
        StagedProperty::U32(s) => out.push(PropValue::U32(s.values[i])),
        StagedProperty::I64(s) => out.push(PropValue::I64(s.values[i])),
        StagedProperty::U64(s) => out.push(PropValue::U64(s.values[i])),
        StagedProperty::F32(s) => out.push(PropValue::F32(s.values[i])),
        StagedProperty::F64(s) => out.push(PropValue::F64(s.values[i])),
        StagedProperty::Str(s) => out.push(PropValue::Str(staged_str_get(s, i))),
        StagedProperty::SharedDict(sd) => {
            for item in &sd.items {
                let val = staged_shared_dict_get(sd, item, i);
                out.push(PropValue::Str(val));
            }
        }
    }
}

/// Get the string value at feature index `i` from a `StagedStrings` column.
///
/// Uses the same lengths encoding as `ParsedStrings::get()`: non-negative values are
/// end offsets, negative values encode the current offset as `!end` (two's complement).
fn staged_str_get(s: &StagedStrings, i: usize) -> Option<String> {
    let end_raw = *s.lengths.get(i)?;
    if end_raw < 0 {
        return None;
    }
    let end = end_raw as usize;
    let start = i
        .checked_sub(1)
        .and_then(|prev| s.lengths.get(prev).copied())
        .map_or(0, |v| if v < 0 { (!v) as usize } else { v as usize });
    s.data.get(start..end).map(str::to_string)
}

/// Get the string value at feature index `i` from a `StagedSharedDict` item.
fn staged_shared_dict_get(
    sd: &StagedSharedDict,
    item: &StagedSharedDictItem,
    i: usize,
) -> Option<String> {
    let (start, end) = item.ranges.get(i).copied().unwrap_or((-1, -1));
    if start < 0 {
        None
    } else {
        let s = usize::try_from(start).ok()?;
        let e = usize::try_from(end).ok()?;
        sd.data.get(s..e).map(str::to_string)
    }
}

// ── TileLayer01 → StagedLayer01 ─────────────────────────────────────────────

impl From<TileLayer01> for StagedLayer01 {
    fn from(source: TileLayer01) -> Self {
        // Rebuild geometry column
        let mut geom = ParsedGeometry::default();
        for f in &source.features {
            geom.push_geom(&f.geometry);
        }

        // Rebuild ID column
        let has_ids = source.features.iter().any(|f| f.id.is_some());
        let id = if has_ids || !source.features.is_empty() {
            Some(ParsedId(source.features.iter().map(|f| f.id).collect()))
        } else {
            None
        };

        let num_cols = source.property_names.len();
        let properties = rebuild_properties(&source.property_names, &source.features, num_cols);

        StagedLayer01 {
            name: source.name,
            extent: source.extent,
            id,
            geometry: geom,
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        }
    }
}

/// Rebuild the property columns from per-feature `PropValue` rows.
fn rebuild_properties(
    names: &[String],
    features: &[TileFeature],
    num_cols: usize,
) -> Vec<StagedProperty> {
    if num_cols == 0 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut col = 0;

    while col < num_cols {
        let (prefix, _suffix) = split_prefix(&names[col]);

        if let Some(dict_prefix) = prefix {
            let start_col = col;
            let mut group_end = col + 1;
            while group_end < num_cols {
                let (p, _) = split_prefix(&names[group_end]);
                if p == Some(dict_prefix) {
                    group_end += 1;
                } else {
                    break;
                }
            }

            if group_end > start_col + 1 {
                let shared_dict =
                    rebuild_shared_dict(dict_prefix, names, features, start_col, group_end);
                result.push(StagedProperty::SharedDict(shared_dict));
                col = group_end;
                continue;
            }
        }

        let prop = rebuild_scalar_column(&names[col], col, features);
        result.push(prop);
        col += 1;
    }

    result
}

/// Split `"prefix:suffix"` into `(Some("prefix"), "suffix")`, or
/// `(None, name)` if there is no colon.
fn split_prefix(name: &str) -> (Option<&str>, &str) {
    if let Some(pos) = name.find(':') {
        (Some(&name[..pos]), &name[pos + 1..])
    } else {
        (None, name)
    }
}

fn rebuild_scalar_column(name: &str, col: usize, features: &[TileFeature]) -> StagedProperty {
    let first_val = features.iter().find_map(|f| f.properties.get(col));

    macro_rules! scalar_col {
        ($variant:ident, $ty:ty, $sv:ident) => {{
            let values: Vec<Option<$ty>> = features
                .iter()
                .map(|f| {
                    if let Some(PropValue::$sv(v)) = f.properties.get(col) {
                        *v
                    } else {
                        None
                    }
                })
                .collect();
            StagedProperty::$variant(StagedScalar {
                name: name.to_string(),
                values,
            })
        }};
    }

    match first_val {
        Some(PropValue::Bool(_)) => scalar_col!(Bool, bool, Bool),
        Some(PropValue::I8(_)) => scalar_col!(I8, i8, I8),
        Some(PropValue::U8(_)) => scalar_col!(U8, u8, U8),
        Some(PropValue::I32(_)) => scalar_col!(I32, i32, I32),
        Some(PropValue::U32(_)) => scalar_col!(U32, u32, U32),
        Some(PropValue::I64(_)) => scalar_col!(I64, i64, I64),
        Some(PropValue::U64(_)) => scalar_col!(U64, u64, U64),
        Some(PropValue::F32(_)) => scalar_col!(F32, f32, F32),
        Some(PropValue::F64(_)) => scalar_col!(F64, f64, F64),
        Some(PropValue::Str(_)) | None => {
            let mut lengths = Vec::with_capacity(features.len());
            let mut data = String::new();
            for f in features {
                if let Some(PropValue::Str(Some(v))) = f.properties.get(col) {
                    lengths.push(i32::try_from(v.len()).unwrap_or(i32::MAX));
                    data.push_str(v);
                } else {
                    lengths.push(-1);
                }
            }
            StagedProperty::Str(StagedStrings {
                name: name.to_string(),
                lengths,
                data,
            })
        }
    }
}

fn rebuild_shared_dict(
    prefix: &str,
    names: &[String],
    features: &[TileFeature],
    start_col: usize,
    end_col: usize,
) -> StagedSharedDict {
    let mut items: Vec<StagedSharedDictItem> = (start_col..end_col)
        .map(|c| {
            let (_, suffix) = split_prefix(&names[c]);
            StagedSharedDictItem {
                suffix: suffix.to_string(),
                ranges: Vec::with_capacity(features.len()),
            }
        })
        .collect();

    // Build a shared corpus of unique string values.  Two values that are equal
    // must map to the same byte span so that `collect_shared_dict_spans` can
    // deduplicate them, which is required for correct encoding.
    let mut corpus = String::new();
    let mut unique: std::collections::HashMap<String, (i32, i32)> =
        std::collections::HashMap::new();

    for f in features {
        for (item_idx, item) in items.iter_mut().enumerate() {
            let col = start_col + item_idx;
            let val = f.properties.get(col).and_then(|v| {
                if let PropValue::Str(s) = v {
                    s.clone()
                } else {
                    None
                }
            });
            let range = if let Some(s) = val {
                *unique.entry(s.clone()).or_insert_with(|| {
                    let start = i32::try_from(corpus.len()).unwrap_or(i32::MAX);
                    corpus.push_str(&s);
                    let end = i32::try_from(corpus.len()).unwrap_or(i32::MAX);
                    (start, end)
                })
            } else {
                (-1, -1)
            };
            item.ranges.push(range);
        }
    }

    StagedSharedDict {
        prefix: prefix.to_string(),
        data: corpus,
        items,
    }
}
