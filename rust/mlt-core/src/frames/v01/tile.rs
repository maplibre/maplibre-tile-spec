//! Row-oriented "source form" for the optimizer.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! The only conversions to/from [`StagedLayer01`] happen at the optimizer entry
//! and exit boundaries.

use std::borrow::Cow;

use geo_types::Geometry;

use crate::MltError;
use crate::optimizer::ManualOptimisation as _;
use crate::v01::{
    GeometryEncoder, IntEncoder, ParsedGeometry, ParsedId, ParsedProperty, ParsedScalar,
    ParsedSharedDict, ParsedSharedDictItem, ParsedStrings, StagedGeometry, StagedId, StagedLayer01,
    StagedProperty,
};

/// Row-oriented working form for the optimizer.
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

// ── StagedLayer01 → TileLayer01 ─────────────────────────────────────────────

impl TryFrom<StagedLayer01> for TileLayer01 {
    type Error = MltError;

    fn try_from(mut layer: StagedLayer01) -> Result<Self, Self::Error> {
        // Decode all columns that are still in encoded form.
        decode_layer(&mut layer)?;

        let geom = match &layer.geometry {
            StagedGeometry::Decoded(g) => g,
            StagedGeometry::Encoded(_) => return Err(MltError::NotDecoded("geometry")),
        };

        let n = geom.vector_types.len();

        // Collect property names and decoded property references.
        // SharedDict columns are expanded: one entry per sub-field.
        let mut property_names: Vec<String> = Vec::new();
        let decoded_props: Vec<&StagedProperty> = layer.properties.iter().collect();

        for prop in &decoded_props {
            match prop {
                StagedProperty::Decoded(dp) => match dp {
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
                },
                StagedProperty::Encoded(_) => return Err(MltError::NotDecoded("property")),
            }
        }

        let ids: Option<&[Option<u64>]> = match &layer.id {
            Some(StagedId::Decoded(d)) => Some(&d.0),
            None => None,
            Some(StagedId::Encoded(_)) => return Err(MltError::NotDecoded("id")),
        };

        let mut features = Vec::with_capacity(n);
        for i in 0..n {
            let id = ids.and_then(|ids| ids.get(i).copied().flatten());
            let geometry = geom.to_geojson(i)?;
            let mut properties = Vec::with_capacity(property_names.len());

            for prop in &decoded_props {
                match prop {
                    StagedProperty::Decoded(dp) => {
                        extract_values(dp, i, &mut properties);
                    }
                    StagedProperty::Encoded(_) => return Err(MltError::NotDecoded("property")),
                }
            }

            features.push(TileFeature {
                id,
                geometry,
                properties,
            });
        }

        Ok(TileLayer01 {
            name: layer.name,
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

// ── TileLayer01 → StagedLayer01 ─────────────────────────────────────────────

impl From<TileLayer01> for StagedLayer01 {
    fn from(source: TileLayer01) -> Self {
        // Rebuild geometry column
        let mut geom = ParsedGeometry::default();
        for f in &source.features {
            geom.push_geom(&f.geometry);
        }

        // Rebuild ID column (only if at least one feature has a non-None id)
        let has_ids = source.features.iter().any(|f| f.id.is_some());
        let id = if has_ids || !source.features.is_empty() {
            Some(StagedId::Decoded(ParsedId(
                source.features.iter().map(|f| f.id).collect(),
            )))
        } else {
            None
        };

        // Rebuild property columns from the flattened PropValue rows.
        // We need to reconstruct each column as a Vec of per-feature values.
        let num_cols = source.property_names.len();
        let properties = rebuild_properties(&source.property_names, &source.features, num_cols);

        StagedLayer01 {
            name: source.name,
            extent: source.extent,
            id,
            geometry: StagedGeometry::Decoded(geom),
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        }
    }
}

/// Rebuild the property columns from per-feature `PropValue` rows.
///
/// Each column index `c` maps to a column name in `property_names[c]`.
/// A `SharedDict` column is detected by two or more consecutive names sharing
/// the same `"prefix:"` portion.  All other columns become scalar columns.
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
        // Check if the next column(s) form a SharedDict group (share the same prefix).
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
                // Multiple columns with the same prefix → SharedDict
                let shared_dict =
                    rebuild_shared_dict(dict_prefix, names, features, start_col, group_end);
                result.push(StagedProperty::Decoded(ParsedProperty::SharedDict(
                    shared_dict,
                )));
                col = group_end;
                continue;
            }
        }

        // Single scalar column
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
    // Determine the variant by looking at the first non-None feature value.
    // Fall back to Str if all values are None or the column is empty.
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
            StagedProperty::Decoded(ParsedProperty::$variant(ParsedScalar {
                name: Cow::Owned(name.to_string()),
                values,
            }))
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
            let values: Vec<Option<String>> = features
                .iter()
                .map(|f| {
                    if let Some(PropValue::Str(v)) = f.properties.get(col) {
                        v.clone()
                    } else {
                        None
                    }
                })
                .collect();
            let mut ds: ParsedStrings<'static> = values.into();
            ds.name = Cow::Owned(name.to_string());
            StagedProperty::Decoded(ParsedProperty::Str(ds))
        }
    }
}

fn rebuild_shared_dict(
    prefix: &str,
    names: &[String],
    features: &[TileFeature],
    start_col: usize,
    end_col: usize,
) -> ParsedSharedDict<'static> {
    // Build a shared corpus of all non-null strings across all sub-columns,
    // and per-item (start,end) ranges into that corpus.
    let mut items: Vec<ParsedSharedDictItem<'static>> = (start_col..end_col)
        .map(|c| {
            let (_, suffix) = split_prefix(&names[c]);
            ParsedSharedDictItem {
                suffix: Cow::Owned(suffix.to_string()),
                ranges: Vec::with_capacity(features.len()),
            }
        })
        .collect();

    let mut corpus = String::new();

    for f in features {
        for (item_idx, item) in items.iter_mut().enumerate() {
            let col = start_col + item_idx;
            let val = f.properties.get(col).and_then(|v| {
                if let PropValue::Str(s) = v {
                    s.as_deref()
                } else {
                    None
                }
            });
            let range = if let Some(s) = val {
                let start = i32::try_from(corpus.len()).unwrap_or(i32::MAX);
                corpus.push_str(s);
                let end = i32::try_from(corpus.len()).unwrap_or(i32::MAX);
                (start, end)
            } else {
                (-1, -1)
            };
            item.ranges.push(range);
        }
    }

    ParsedSharedDict {
        prefix: Cow::Owned(prefix.to_string()),
        data: Cow::Owned(corpus),
        items,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Decode (and canonicalize) all columns of `layer` that are still in encoded
/// or sparse-decoded form.
///
/// Geometry that was built with `push_geom` is in a "sparse" offset-array
/// layout that differs from the "dense" layout produced by the wire
/// encode→decode round-trip.  [`ParsedGeometry::to_geojson`] requires the
/// dense form, so we canonicalize by encoding and decoding the geometry here.
fn decode_layer(layer: &mut StagedLayer01) -> Result<(), MltError> {
    // Always canonicalize geometry through encode→decode to get dense offsets.
    layer
        .geometry
        .manual_optimisation(GeometryEncoder::all(IntEncoder::varint()))?;
    let geom = std::mem::replace(
        &mut layer.geometry,
        StagedGeometry::Decoded(ParsedGeometry::default()),
    );
    layer.geometry = StagedGeometry::Decoded(ParsedGeometry::try_from(geom)?);

    if let Some(id) = layer.id.take() {
        layer.id = Some(StagedId::Decoded(ParsedId::try_from(id)?));
    }

    // Properties must already be decoded; encoded properties are not expected here.
    for prop in &layer.properties {
        if let StagedProperty::Encoded(_) = prop {
            return Err(MltError::NotDecoded("property"));
        }
    }

    Ok(())
}
