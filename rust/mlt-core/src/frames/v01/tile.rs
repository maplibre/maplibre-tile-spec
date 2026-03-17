//! Row-oriented "source form" for the optimizer.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! The only conversion from [`TileLayer01`] to [`StagedLayer01`] is [`From`] at the
//! optimizer exit boundary; there is no encoded→decoded conversion from Staged back to Tile.

use crate::errors::AsMltError as _;
use crate::v01::{
    GeometryValues, IdValues, Layer01, ParsedProperty, PropValue, StagedLayer01, StagedProperty,
    StagedScalar, StagedSharedDict, StagedStrings, TileFeature, TileLayer01,
    build_staged_shared_dict,
};
use crate::{Decoder, MltError};

// ── Layer01 → TileLayer01 ────────────────────────────────────────────────────

impl Layer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer01`], charging every
    /// heap allocation against `dec`.
    ///
    /// Callers do not need to pre-call `decode_all` on the source layer.
    pub fn into_tile(self, dec: &mut Decoder) -> Result<TileLayer01, MltError> {
        let id = self.id.map(|id| id.into_parsed(dec)).transpose()?;
        let geometry = self.geometry.into_parsed(dec)?;
        let properties: Vec<ParsedProperty<'_>> = self
            .properties
            .into_iter()
            .map(|p| p.into_parsed(dec))
            .collect::<Result<Vec<_>, _>>()?;

        let n = geometry.vector_types.len();

        let mut property_names: Vec<String> = Vec::new();
        for prop in &properties {
            match prop {
                ParsedProperty::SharedDict(sd) => {
                    for item in &sd.items {
                        property_names.push(format!("{}{}", sd.prefix, item.suffix));
                    }
                }
                other => property_names.push(other.name().to_string()),
            }
        }

        let ids: Option<&[Option<u64>]> = id.as_ref().map(|d| d.0.as_slice());

        let mut features = dec.alloc::<TileFeature>(n)?;
        for i in 0..n {
            let feat_id = ids.and_then(|ids| ids.get(i).copied().flatten());
            let geom = geometry.to_geojson(i)?;
            let mut values = dec.alloc::<PropValue>(property_names.len())?;
            for prop in &properties {
                extract_parsed_values(prop, i, &mut values);
            }

            // Charge owned String bytes inside PropValue::Str.
            charge_str_props(dec, &values)?;

            features.push(TileFeature {
                id: feat_id,
                geometry: geom,
                properties: values,
            });
        }

        Ok(TileLayer01 {
            name: self.name.to_string(),
            extent: self.extent,
            property_names,
            features,
        })
    }
}

/// Extract the per-feature value at index `i` from a parsed property column
/// and push it (or them, for `SharedDict`) into `out`.
fn extract_parsed_values(prop: &ParsedProperty<'_>, i: usize, out: &mut Vec<PropValue>) {
    use crate::v01::ParsedProperty as P;
    match prop {
        P::Bool(s) => out.push(PropValue::Bool(s.values[i])),
        P::I8(s) => out.push(PropValue::I8(s.values[i])),
        P::U8(s) => out.push(PropValue::U8(s.values[i])),
        P::I32(s) => out.push(PropValue::I32(s.values[i])),
        P::U32(s) => out.push(PropValue::U32(s.values[i])),
        P::I64(s) => out.push(PropValue::I64(s.values[i])),
        P::U64(s) => out.push(PropValue::U64(s.values[i])),
        P::F32(s) => out.push(PropValue::F32(s.values[i])),
        P::F64(s) => out.push(PropValue::F64(s.values[i])),
        P::Str(s) => {
            let val = s
                .get(u32::try_from(i).unwrap_or(u32::MAX))
                .map(str::to_string);
            out.push(PropValue::Str(val));
        }
        P::SharedDict(sd) => {
            for item in &sd.items {
                let val = item.get(sd, i).map(str::to_string);
                out.push(PropValue::Str(val));
            }
        }
    }
}

// ── TileLayer01 → StagedLayer01 ─────────────────────────────────────────────

/// FIXME: this should be part of the [`crate::v01::optimizer::Tile01Encoder::encode`]
///   `rebuild_properties` would use proper shared dict grouping settings
impl From<TileLayer01> for StagedLayer01 {
    fn from(source: TileLayer01) -> Self {
        // Rebuild geometry column
        let mut geometry = GeometryValues::default();
        for f in &source.features {
            geometry.push_geom(&f.geometry);
        }

        let id = if source.features.iter().any(|f| f.id.is_some()) {
            Some(IdValues(source.features.iter().map(|f| f.id).collect()))
        } else {
            None
        };

        let num_cols = source.property_names.len();
        let properties = rebuild_properties(&source.property_names, &source.features, num_cols);

        StagedLayer01 {
            name: source.name,
            extent: source.extent,
            id,
            geometry,
            properties,
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
                result.push(StagedProperty::SharedDict(shared_dict));
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
            let mut ds = StagedStrings::from(values);
            ds.name = name.to_string();
            StagedProperty::Str(ds)
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
    // Build per-item (start,end) ranges from raw feature data, then
    // call build_staged_shared_dict to deduplicate into a shared corpus.
    let items_raw: Vec<(String, StagedStrings)> = (start_col..end_col)
        .map(|c| {
            let (_, suffix) = split_prefix(&names[c]);
            let values: Vec<Option<String>> = features
                .iter()
                .map(|f| {
                    if let Some(PropValue::Str(s)) = f.properties.get(c) {
                        s.clone()
                    } else {
                        None
                    }
                })
                .collect();
            (suffix.to_string(), StagedStrings::from(values))
        })
        .collect();

    // Set the suffix names on each StagedStrings (for the corpus-dedup step).
    // The names aren't stored in StagedStrings.name here; they're passed as the
    // tuple key to build_staged_shared_dict.
    build_staged_shared_dict(prefix.to_string(), items_raw)
        .expect("rebuild_shared_dict should always succeed for valid feature data")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Charge `dec` for the heap bytes of owned `String` values inside `PropValue::Str`.
fn charge_str_props(dec: &mut Decoder, props: &[PropValue]) -> Result<(), MltError> {
    let str_bytes = props
        .iter()
        .filter_map(|p| {
            if let PropValue::Str(Some(s)) = p {
                Some(s.len())
            } else {
                None
            }
        })
        .try_fold(0u32, |acc, n| {
            acc.checked_add(u32::try_from(n).or_overflow()?)
                .ok_or(MltError::IntegerOverflow)
        })?;
    if str_bytes > 0 {
        dec.consume(str_bytes)?;
    }
    Ok(())
}
