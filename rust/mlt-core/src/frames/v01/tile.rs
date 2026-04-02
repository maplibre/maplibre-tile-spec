//! Row-oriented "source form" for the optimizer.
//!
//! [`TileLayer01`] holds one [`TileFeature`] per map feature, each owning
//! its geometry as a [`geo_types::Geometry<i32>`] and its property values as a
//! plain `Vec<PropValue>`.  This is the working form used throughout the
//! optimizer and sorting pipeline: it is cheap to clone, trivially sortable,
//! and free from any encoded/decoded duality.
//!
//! Conversion from [`TileLayer01`] to [`StagedLayer01`] is done via
//! [`StagedLayer01::from_tile`] (pre-computed [`StringGroup`] pairings produced by
//! [`crate::v01::group_string_properties`]) or the blanket [`From`] impl (no grouping).

use crate::errors::AsMltError as _;
use crate::v01::{
    Layer01, ParsedLayer01, ParsedProperty, PropValue, PropValueRef, TileFeature, TileLayer01,
};
use crate::{Decoder, MltResult};

impl ParsedLayer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer01`], charging every
    /// heap allocation against `dec`.
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer01> {
        let names: Vec<String> = self.iterate_prop_names().map(|n| n.to_string()).collect();
        let col_nulls = typed_nulls(&self.properties);
        let mut features = dec.alloc::<TileFeature>(self.feature_count())?;
        for feat in self.iter_features() {
            let feat = feat?;
            let mut values = dec.alloc::<PropValue>(names.len())?;
            for (col_idx, value) in feat.iter_all_properties().enumerate() {
                values.push(match value {
                    Some(v) => prop_value_from_ref(v),
                    None => col_nulls[col_idx].clone(),
                });
            }

            charge_str_props(dec, &values)?;

            features.push(TileFeature {
                id: feat.id,
                geometry: feat.geometry,
                properties: values,
            });
        }

        Ok(TileLayer01 {
            name: self.name.to_string(),
            extent: self.extent,
            property_names: names,
            features,
        })
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.geometry.vector_types.len()
    }
}

impl Layer01<'_> {
    /// Decode and convert into a row-oriented [`TileLayer01`]
    pub fn into_tile(self, dec: &mut Decoder) -> MltResult<TileLayer01> {
        self.decode_all(dec)?.into_tile(dec)
    }
}

/// Convert a [`PropValueRef`] (as yielded by [`crate::v01::FeatureRef::iter_all_properties`])
/// into an owned [`PropValue`].
fn prop_value_from_ref(value: PropValueRef<'_>) -> PropValue {
    match value {
        PropValueRef::Bool(v) => PropValue::Bool(Some(v)),
        PropValueRef::I8(v) => PropValue::I8(Some(v)),
        PropValueRef::U8(v) => PropValue::U8(Some(v)),
        PropValueRef::I32(v) => PropValue::I32(Some(v)),
        PropValueRef::U32(v) => PropValue::U32(Some(v)),
        PropValueRef::I64(v) => PropValue::I64(Some(v)),
        PropValueRef::U64(v) => PropValue::U64(Some(v)),
        PropValueRef::F32(v) => PropValue::F32(Some(v)),
        PropValueRef::F64(v) => PropValue::F64(Some(v)),
        PropValueRef::Str(s) => PropValue::Str(Some(s.to_string())),
    }
}

/// Build a flat list of typed null [`PropValue`]s, one per logical column position
/// as yielded by [`crate::v01::FeatureRef::iter_all_properties`].
///
/// Each scalar column contributes one entry with its specific null variant (e.g.
/// `PropValue::Bool(None)`).  A `SharedDict` column expands to one `PropValue::Str(None)`
/// entry per sub-item.
fn typed_nulls(properties: &[ParsedProperty<'_>]) -> Vec<PropValue> {
    use ParsedProperty as PP;
    use PropValue as PV;
    let mut nulls = Vec::new();
    for prop in properties {
        match prop {
            PP::Bool(_) => nulls.push(PV::Bool(None)),
            PP::I8(_) => nulls.push(PV::I8(None)),
            PP::U8(_) => nulls.push(PV::U8(None)),
            PP::I32(_) => nulls.push(PV::I32(None)),
            PP::U32(_) => nulls.push(PV::U32(None)),
            PP::I64(_) => nulls.push(PV::I64(None)),
            PP::U64(_) => nulls.push(PV::U64(None)),
            PP::F32(_) => nulls.push(PV::F32(None)),
            PP::F64(_) => nulls.push(PV::F64(None)),
            PP::Str(_) => nulls.push(PV::Str(None)),
            PP::SharedDict(d) => {
                for _ in &d.items {
                    nulls.push(PV::Str(None));
                }
            }
        }
    }
    nulls
}

/// Charge `dec` for the heap bytes of owned `String` values inside `PropValue::Str`.
fn charge_str_props(dec: &mut Decoder, props: &[PropValue]) -> MltResult<()> {
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
                .or_overflow()
        })?;
    if str_bytes > 0 {
        dec.consume(str_bytes)?;
    }
    Ok(())
}
