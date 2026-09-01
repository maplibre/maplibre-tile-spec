use arbitrary::Error::IncorrectFormat;
use arbitrary::{Arbitrary, Result, Unstructured};
use num_traits::Float;
use strum::EnumCount as _;

use crate::encoder::model::StagedLayer;
use crate::encoder::optimizer::Presence;
use crate::encoder::{EncoderConfig, StagedId, StagedProperty, StagedSharedDict};

impl Arbitrary<'_> for EncoderConfig {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let config = Self::default()
            .with_wire_version(u.arbitrary()?)
            .with_tessellation(u.arbitrary()?)
            .with_spatial_morton_sort(u.arbitrary()?)
            .with_spatial_hilbert_sort(u.arbitrary()?)
            .with_id_sort(u.arbitrary()?)
            .with_fsst(u.arbitrary()?)
            .with_fastpfor(u.arbitrary()?)
            .with_shared_dict(u.arbitrary()?);
        #[cfg(feature = "unstable-v2")]
        let config = config.with_float_dict(u.arbitrary()?);
        Ok(config)
    }
}

impl Arbitrary<'_> for StagedId {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound ID count to prevent OOM from unbounded vector generation
        let count = usize::from(u.int_in_range(0..=64u8)?);
        let values: Vec<Option<u64>> = (0..count).map(|_| u.arbitrary()).collect::<Result<_>>()?;
        Ok(Self::from_optional(values))
    }
}

impl Arbitrary<'_> for StagedLayer {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound name length to prevent OOM from unbounded string generation
        let len = u.int_in_range(1..=32)?;
        let name: String = (0..len)
            .map(|_| u.arbitrary::<char>())
            .collect::<Result<_>>()?;
        let extent: u32 = u.arbitrary()?;
        // Generate geometry first -- its feature count drives ID and property columns.
        let geometry: crate::decoder::GeometryValues = u.arbitrary()?;
        let fc = geometry.vector_types().len();
        let id = if u.arbitrary::<bool>()? {
            let ids: Vec<Option<u64>> = (0..fc)
                .map(|_| -> Result<_> {
                    if u.arbitrary::<bool>()? {
                        Ok(Some(u.arbitrary::<u64>()?))
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<_>>()?;
            StagedId::from_optional(ids)
        } else {
            StagedId::None
        };
        // Bound property count to prevent OOM from unbounded vector generation.
        // Each column must have exactly `fc` values to match the feature count.
        let prop_count = usize::from(u.int_in_range(0..=4u8)?);
        let properties: Vec<StagedProperty> = (0..prop_count)
            .map(|i| generate_property(u, format!("prop{i}"), fc))
            .collect::<Result<_>>()?;

        Self::new(name, extent, id, geometry, properties).map_err(|_| IncorrectFormat)
    }
}

/// Generate a property column of an arbitrary kind holding exactly `count` values.
fn generate_property(
    u: &mut Unstructured<'_>,
    name: String,
    count: usize,
) -> Result<StagedProperty> {
    const _: () = assert!(
        StagedProperty::COUNT == 21,
        "needs new variant in match below"
    );
    Ok(match u.int_in_range(0..=StagedProperty::COUNT - 1)? {
        0 => StagedProperty::bool(name, generate_scalars(u, count)?),
        1 => StagedProperty::i8(name, generate_scalars(u, count)?),
        2 => StagedProperty::u8(name, generate_scalars(u, count)?),
        3 => StagedProperty::i32(name, generate_scalars(u, count)?),
        4 => StagedProperty::u32(name, generate_scalars(u, count)?),
        5 => StagedProperty::i64(name, generate_scalars(u, count)?),
        6 => StagedProperty::u64(name, generate_scalars(u, count)?),
        7 => StagedProperty::f32(name, generate_floats(u, count)?),
        8 => StagedProperty::f64(name, generate_floats(u, count)?),
        9 => StagedProperty::str(name, generate_strings(u, count)?),
        10 => StagedProperty::opt_bool(name, generate_scalars(u, count)?),
        11 => StagedProperty::opt_i8(name, generate_scalars(u, count)?),
        12 => StagedProperty::opt_u8(name, generate_scalars(u, count)?),
        13 => StagedProperty::opt_i32(name, generate_scalars(u, count)?),
        14 => StagedProperty::opt_u32(name, generate_scalars(u, count)?),
        15 => StagedProperty::opt_i64(name, generate_scalars(u, count)?),
        16 => StagedProperty::opt_u64(name, generate_scalars(u, count)?),
        17 => StagedProperty::opt_f32(name, generate_optional_floats(u, count)?),
        18 => StagedProperty::opt_f64(name, generate_optional_floats(u, count)?),
        19 => StagedProperty::opt_str(name, generate_optional_strings(u, count)?),
        _ => StagedProperty::SharedDict(generate_shared_dict(u, name, count)?),
    })
}

/// Generate a shared-dictionary column whose every item holds exactly `count` values.
fn generate_shared_dict(
    u: &mut Unstructured<'_>,
    prefix: String,
    count: usize,
) -> Result<StagedSharedDict> {
    // Bound item count to prevent OOM from unbounded vector generation
    let item_count = usize::from(u.int_in_range(1..=4u8)?);
    let items: Vec<(String, Vec<Option<String>>, Presence)> = (0..item_count)
        .map(|i| -> Result<_> {
            let values = generate_optional_strings(u, count)?;
            let presence = if values.iter().all(Option::is_some) {
                Presence::AllPresent
            } else {
                Presence::Mixed
            };
            Ok((format!("_{i}"), values, presence))
        })
        .collect::<Result<_>>()?;
    StagedSharedDict::new(prefix, items).map_err(|_| IncorrectFormat)
}

/// Generate exactly `count` scalars.
fn generate_scalars<'a, T: Arbitrary<'a>>(
    u: &mut Unstructured<'a>,
    count: usize,
) -> Result<Vec<T>> {
    (0..count).map(|_| u.arbitrary()).collect()
}

/// Generate exactly `count` floats, never NaN, since the fixpoint assertions compare with `==`.
fn generate_floats<'a, T: Arbitrary<'a> + Float>(
    u: &mut Unstructured<'a>,
    count: usize,
) -> Result<Vec<T>> {
    (0..count)
        .map(|_| Ok(zero_if_nan(u.arbitrary()?)))
        .collect()
}

/// Generate exactly `count` optional floats, never NaN.
fn generate_optional_floats<'a, T: Arbitrary<'a> + Float>(
    u: &mut Unstructured<'a>,
    count: usize,
) -> Result<Vec<Option<T>>> {
    (0..count)
        .map(|_| Ok(u.arbitrary::<Option<T>>()?.map(zero_if_nan)))
        .collect()
}

fn zero_if_nan<T: Float>(value: T) -> T {
    if value.is_nan() { T::zero() } else { value }
}

/// Generate exactly `count` strings with bounded lengths to prevent OOM.
fn generate_strings(u: &mut Unstructured<'_>, count: usize) -> Result<Vec<String>> {
    (0..count).map(|_| bounded_string(u, 64)).collect()
}

/// Generate exactly `count` optional strings with bounded lengths to prevent OOM.
fn generate_optional_strings(
    u: &mut Unstructured<'_>,
    count: usize,
) -> Result<Vec<Option<String>>> {
    (0..count)
        .map(|_| -> Result<_> {
            if u.arbitrary()? {
                Ok(Some(bounded_string(u, 64)?))
            } else {
                Ok(None)
            }
        })
        .collect()
}

/// Generate a string with bounded length to prevent OOM from unbounded string generation.
fn bounded_string(u: &mut Unstructured<'_>, max_len: u8) -> Result<String> {
    let len = usize::from(u.int_in_range(0..=max_len)?);
    (0..len)
        .map(|_| u.arbitrary::<char>())
        .collect::<Result<_>>()
}
