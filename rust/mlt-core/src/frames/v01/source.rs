//! Row-oriented "source form" for the optimizer.
//!
//! [`SourceLayer01`] holds one [`SourceFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<SourceValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! The only conversions to/from [`OwnedLayer01`] happen at the optimizer entry
//! and exit boundaries.

use std::borrow::Cow;

use geo_types::Geometry;

use crate::optimizer::ManualOptimisation as _;
use crate::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, DecodedScalar, DecodedSharedDict,
    DecodedSharedDictItem, DecodedStrings, GeometryEncoder, IntEncoder, OwnedGeometry, OwnedId,
    OwnedLayer01, OwnedProperty,
};
use crate::MltError;

/// Row-oriented working form for the optimizer.
///
/// All features are stored as a flat [`Vec<SourceFeature>`] so that sorting is
/// a single `sort_by_cached_key` call.  The `property_names` vec is parallel
/// to every `SourceFeature::properties` slice in this layer.
#[derive(Debug, Clone)]
pub struct SourceLayer01 {
    pub name: String,
    pub extent: u32,
    /// Column names, parallel to `SourceFeature::properties`.
    pub property_names: Vec<String>,
    pub features: Vec<SourceFeature>,
}

/// A single map feature in row form.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFeature {
    pub id: Option<u64>,
    /// Geometry in `geo_types` / `Geom32` form.
    pub geometry: Geometry<i32>,
    /// One value per property column, in the same order as
    /// [`SourceLayer01::property_names`].
    pub properties: Vec<SourceValue>,
}

/// A single typed value for one property of one feature.
///
/// Mirrors the scalar variants of [`DecodedProperty`] at the per-feature
/// level. `SharedDict` items are flattened: each sub-field becomes its own
/// `SourceValue::Str` entry in `SourceFeature::properties`, with the
/// corresponding entry in `SourceLayer01::property_names` set to
/// `"prefix:suffix"`.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceValue {
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

// ── OwnedLayer01 → SourceLayer01 ─────────────────────────────────────────────

impl TryFrom<OwnedLayer01> for SourceLayer01 {
    type Error = MltError;

    fn try_from(mut layer: OwnedLayer01) -> Result<Self, Self::Error> {
        // Decode all columns that are still in encoded form.
        decode_layer(&mut layer)?;

        let geom = match &layer.geometry {
            OwnedGeometry::Decoded(g) => g,
            OwnedGeometry::Encoded(_) => return Err(MltError::NotDecoded("geometry")),
        };

        let n = geom.vector_types.len();

        // Collect property names and decoded property references.
        // SharedDict columns are expanded: one entry per sub-field.
        let mut property_names: Vec<String> = Vec::new();
        let decoded_props: Vec<&OwnedProperty> = layer.properties.iter().collect();

        for prop in &decoded_props {
            match prop {
                OwnedProperty::Decoded(dp) => match dp {
                    DecodedProperty::SharedDict(sd) => {
                        for item in &sd.items {
                            property_names
                                .push(format!("{}:{}", sd.prefix.as_ref(), item.suffix.as_ref()));
                        }
                    }
                    other => property_names.push(other.name().to_string()),
                },
                OwnedProperty::Encoded(_) => return Err(MltError::NotDecoded("property")),
            }
        }

        let ids: Option<&[Option<u64>]> = match &layer.id {
            Some(OwnedId::Decoded(d)) => Some(&d.0),
            None => None,
            Some(OwnedId::Encoded(_)) => return Err(MltError::NotDecoded("id")),
        };

        let mut features = Vec::with_capacity(n);
        for i in 0..n {
            let id = ids.and_then(|ids| ids.get(i).copied().flatten());
            let geometry = geom.to_geojson(i)?;
            let mut properties = Vec::with_capacity(property_names.len());

            for prop in &decoded_props {
                match prop {
                    OwnedProperty::Decoded(dp) => {
                        extract_values(dp, i, &mut properties);
                    }
                    OwnedProperty::Encoded(_) => return Err(MltError::NotDecoded("property")),
                }
            }

            features.push(SourceFeature {
                id,
                geometry,
                properties,
            });
        }

        Ok(SourceLayer01 {
            name: layer.name,
            extent: layer.extent,
            property_names,
            features,
        })
    }
}

/// Extract the per-feature value at index `i` from a decoded property column
/// and push it (or them, for `SharedDict`) into `out`.
fn extract_values(prop: &DecodedProperty<'_>, i: usize, out: &mut Vec<SourceValue>) {
    match prop {
        DecodedProperty::Bool(s) => out.push(SourceValue::Bool(s.values[i])),
        DecodedProperty::I8(s) => out.push(SourceValue::I8(s.values[i])),
        DecodedProperty::U8(s) => out.push(SourceValue::U8(s.values[i])),
        DecodedProperty::I32(s) => out.push(SourceValue::I32(s.values[i])),
        DecodedProperty::U32(s) => out.push(SourceValue::U32(s.values[i])),
        DecodedProperty::I64(s) => out.push(SourceValue::I64(s.values[i])),
        DecodedProperty::U64(s) => out.push(SourceValue::U64(s.values[i])),
        DecodedProperty::F32(s) => out.push(SourceValue::F32(s.values[i])),
        DecodedProperty::F64(s) => out.push(SourceValue::F64(s.values[i])),
        DecodedProperty::Str(s) => {
            let val = s.get(u32::try_from(i).unwrap_or(u32::MAX)).map(str::to_string);
            out.push(SourceValue::Str(val));
        }
        DecodedProperty::SharedDict(sd) => {
            for item in &sd.items {
                let val = item.get(sd, i).map(str::to_string);
                out.push(SourceValue::Str(val));
            }
        }
    }
}

// ── SourceLayer01 → OwnedLayer01 ─────────────────────────────────────────────

impl From<SourceLayer01> for OwnedLayer01 {
    fn from(source: SourceLayer01) -> Self {

        // Rebuild geometry column
        let mut geom = DecodedGeometry::default();
        for f in &source.features {
            geom.push_geom(&f.geometry);
        }

        // Rebuild ID column (only if at least one feature has a non-None id)
        let has_ids = source.features.iter().any(|f| f.id.is_some());
        let id = if has_ids || !source.features.is_empty() {
            Some(OwnedId::Decoded(DecodedId(
                source.features.iter().map(|f| f.id).collect(),
            )))
        } else {
            None
        };

        // Rebuild property columns from the flattened SourceValue rows.
        // We need to reconstruct each column as a Vec of per-feature values.
        let num_cols = source.property_names.len();
        let properties = rebuild_properties(&source.property_names, &source.features, num_cols);

        OwnedLayer01 {
            name: source.name,
            extent: source.extent,
            id,
            geometry: OwnedGeometry::Decoded(geom),
            properties,
            #[cfg(fuzzing)]
            layer_order: vec![],
        }
    }
}

/// Rebuild the property columns from per-feature `SourceValue` rows.
///
/// Each column index `c` maps to a column name in `property_names[c]`.
/// A `SharedDict` column is detected by two or more consecutive names sharing
/// the same `"prefix:"` portion.  All other columns become scalar columns.
fn rebuild_properties(
    names: &[String],
    features: &[SourceFeature],
    num_cols: usize,
) -> Vec<OwnedProperty> {
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
                result.push(OwnedProperty::Decoded(DecodedProperty::SharedDict(
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

fn rebuild_scalar_column(name: &str, col: usize, features: &[SourceFeature]) -> OwnedProperty {
    // Determine the variant by looking at the first non-None feature value.
    // Fall back to Str if all values are None or the column is empty.
    let first_val = features.iter().find_map(|f| f.properties.get(col));

    macro_rules! scalar_col {
        ($variant:ident, $ty:ty, $sv:ident) => {{
            let values: Vec<Option<$ty>> = features
                .iter()
                .map(|f| {
                    if let Some(SourceValue::$sv(v)) = f.properties.get(col) {
                        *v
                    } else {
                        None
                    }
                })
                .collect();
            OwnedProperty::Decoded(DecodedProperty::$variant(DecodedScalar {
                name: Cow::Owned(name.to_string()),
                values,
            }))
        }};
    }

    match first_val {
        Some(SourceValue::Bool(_)) => scalar_col!(Bool, bool, Bool),
        Some(SourceValue::I8(_)) => scalar_col!(I8, i8, I8),
        Some(SourceValue::U8(_)) => scalar_col!(U8, u8, U8),
        Some(SourceValue::I32(_)) => scalar_col!(I32, i32, I32),
        Some(SourceValue::U32(_)) => scalar_col!(U32, u32, U32),
        Some(SourceValue::I64(_)) => scalar_col!(I64, i64, I64),
        Some(SourceValue::U64(_)) => scalar_col!(U64, u64, U64),
        Some(SourceValue::F32(_)) => scalar_col!(F32, f32, F32),
        Some(SourceValue::F64(_)) => scalar_col!(F64, f64, F64),
        Some(SourceValue::Str(_)) | None => {
            let values: Vec<Option<String>> = features
                .iter()
                .map(|f| {
                    if let Some(SourceValue::Str(v)) = f.properties.get(col) {
                        v.clone()
                    } else {
                        None
                    }
                })
                .collect();
            let mut ds: DecodedStrings<'static> = values.into();
            ds.name = Cow::Owned(name.to_string());
            OwnedProperty::Decoded(DecodedProperty::Str(ds))
        }
    }
}

fn rebuild_shared_dict(
    prefix: &str,
    names: &[String],
    features: &[SourceFeature],
    start_col: usize,
    end_col: usize,
) -> DecodedSharedDict<'static> {
    // Build a shared corpus of all non-null strings across all sub-columns,
    // and per-item (start,end) ranges into that corpus.
    let mut items: Vec<DecodedSharedDictItem<'static>> = (start_col..end_col)
        .map(|c| {
            let (_, suffix) = split_prefix(&names[c]);
            DecodedSharedDictItem {
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
                if let SourceValue::Str(s) = v {
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

    DecodedSharedDict {
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
/// encode→decode round-trip.  [`DecodedGeometry::to_geojson`] requires the
/// dense form, so we canonicalize by encoding and decoding the geometry here.
fn decode_layer(layer: &mut OwnedLayer01) -> Result<(), MltError> {
    // Always canonicalize geometry through encode→decode to get dense offsets.
    layer
        .geometry
        .manual_optimisation(GeometryEncoder::all(IntEncoder::varint()))?;
    if let OwnedGeometry::Encoded(e) = &layer.geometry {
        let dec = e.decode()?;
        layer.geometry = OwnedGeometry::Decoded(dec);
    }

    if let Some(OwnedId::Encoded(e)) = &layer.id {
        let dec = DecodedId::try_from(borrowme::borrow(e))?;
        layer.id = Some(OwnedId::Decoded(dec));
    }

    let mut decoded_props = Vec::with_capacity(layer.properties.len());
    for prop in &layer.properties {
        if let OwnedProperty::Decoded(_) = prop {
            decoded_props.push(prop.clone());
        } else {
            let decoded_ref = borrowme::borrow(prop).decode()?;
            decoded_props.push(OwnedProperty::Decoded(
                borrowme::ToOwned::to_owned(&decoded_ref),
            ));
        }
    }
    layer.properties = decoded_props;

    Ok(())
}
