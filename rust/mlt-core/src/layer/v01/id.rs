use std::fmt::{Debug, Formatter};
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromRaw, impl_decodable};
use crate::encode::{ToRaw, impl_encodable};
use crate::utils::{BinarySerializer as _, OptSeqOpt};
use crate::v01::{
    ColumnType, DictionaryType, LogicalDecoder, OwnedDataRaw, OwnedDataVarInt, OwnedStream,
    OwnedStreamData, PhysicalDecoder, PhysicalStreamType, Stream, StreamMeta,
};

/// ID column representation, either raw or decoded, or none if there are no IDs
#[borrowme]
#[derive(Debug, Default, PartialEq)]
pub enum Id<'a> {
    #[default]
    None,
    Raw(RawId<'a>),
    Decoded(DecodedId),
}

impl OwnedId {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::None => Ok(()),
            Self::Raw(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::None => Ok(()),
            Self::Raw(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

impl Analyze for Id<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::None => 0,
            Self::Raw(d) => d.collect_statistic(stat),
            Self::Decoded(d) => d.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::None => {}
            Self::Raw(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawId<'a> {
    optional: Option<Stream<'a>>,
    value: RawIdValue<'a>,
}

impl Default for OwnedRawId {
    fn default() -> Self {
        Self {
            optional: None,
            value: OwnedRawIdValue::Id32(OwnedStream {
                meta: StreamMeta {
                    physical_type: PhysicalStreamType::Data(DictionaryType::None),
                    num_values: 0,
                    logical_decoder: LogicalDecoder::None,
                    physical_decoder: PhysicalDecoder::None,
                },
                data: OwnedStreamData::Raw(OwnedDataRaw { data: Vec::new() }),
            }),
        }
    }
}
impl OwnedRawId {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match (&self.optional, &self.value) {
            (None, OwnedRawIdValue::Id32(_)) => ColumnType::Id.write_to(writer)?,
            (None, OwnedRawIdValue::Id64(_)) => ColumnType::LongId.write_to(writer)?,
            (Some(_), OwnedRawIdValue::Id32(_)) => ColumnType::OptId.write_to(writer)?,
            (Some(_), OwnedRawIdValue::Id64(_)) => ColumnType::OptLongId.write_to(writer)?,
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        if let Some(opt) = &self.optional {
            writer.write_boolean_stream(opt)?;
        }
        match &self.value {
            OwnedRawIdValue::Id32(s) | OwnedRawIdValue::Id64(s) => writer.write_stream(s)?,
        }
        Ok(())
    }
}

impl Analyze for RawId<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.optional.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

/// A sequence of encoded (raw) ID values, either 32-bit or 64-bit unsigned integers
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

impl Analyze for RawIdValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Id32(v) | Self::Id64(v) => v.for_each_stream(cb),
        }
    }
}

/// Decoded ID values as a vector of optional 64-bit unsigned integers
#[derive(Clone, Default, PartialEq)]
pub struct DecodedId(pub Option<Vec<Option<u64>>>);

impl Analyze for DecodedId {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.0.collect_statistic(stat)
    }
}

impl_decodable!(Id<'a>, RawId<'a>, DecodedId);
impl_encodable!(OwnedId, DecodedId, OwnedRawId);

impl Debug for DecodedId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            None => write!(f, "DecodedId(None)"),
            Some(ids) => {
                write!(f, "DecodedId({:?})", &OptSeqOpt(Some(ids)))
            }
        }
    }
}

impl<'a> From<RawId<'a>> for Id<'a> {
    fn from(value: RawId<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn raw(optional: Option<Stream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { optional, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedId::from_raw(v)?,
            Self::Decoded(v) => v,
            Self::None => DecodedId(None),
        })
    }
}

impl<'a> FromRaw<'a> for DecodedId {
    type Input = RawId<'a>;

    fn from_raw(RawId { optional, value }: RawId<'_>) -> Result<Self, MltError> {
        // Decode the ID values first
        let ids_u64: Vec<u64> = match value {
            RawIdValue::Id32(stream) => {
                // Decode 32-bit IDs as u32, then convert to u64
                let ids: Vec<u32> = stream.decode_bits_u32()?.decode_u32()?;
                ids.into_iter().map(u64::from).collect()
            }
            RawIdValue::Id64(stream) => {
                // Decode 64-bit IDs directly as u64
                stream.decode_u64()?
            }
        };

        // Apply the offsets of these IDs via an optional/present bitmask
        let ids_optional: Vec<Option<u64>> = if let Some(optional_stream) = optional {
            let present_bits = optional_stream.decode_bools();

            apply_present_bitset(&present_bits, &ids_u64)?
        } else {
            // No optional stream, so all IDs are present
            ids_u64.into_iter().map(Some).collect()
        };

        Ok(DecodedId(Some(ids_optional)))
    }
}
/// The `ids_u64` vector only contains values for features where the bit is set
///
/// We need to iterate through the bitset and pull from `ids_u64` when the bit is set.
fn apply_present_bitset(
    present_bits: &Vec<bool>,
    ids_u64: &[u64],
) -> Result<Vec<Option<u64>>, MltError> {
    let present_bit_count = present_bits.iter().filter(|b| **b).count();
    if present_bit_count != ids_u64.len() {
        return Err(MltError::InvalidStreamData {
            expected: "Number of ID values in the presence stream does not match the number of provided IDs",
            got: format!(
                "{present_bit_count} bits set in the present stream, but {} values for IDs",
                ids_u64.len()
            ),
        });
    }
    debug_assert!(
        ids_u64.len() <= present_bits.len(),
        "Since present_bits.len() <= present_bit_count (upper bound: all bits set) and ids_u64.len() == present_bit_count, there cannot be more IDs than features"
    );

    let mut result = vec![None; present_bits.len()];
    // todo: Currently, optional ids are not well supported by encoders
    // Once this is the case, benchmark for real world usage if using a packed_bitset and possibly nextSetBit
    // See present_bitset.cpp is faster
    let present_ids = result
        .iter_mut()
        .zip(present_bits)
        .filter_map(|(id, is_present)| if *is_present { Some(id) } else { None });
    for (target_id, id_in_stream) in present_ids.zip(ids_u64) {
        target_id.replace(*id_in_stream);
    }
    Ok(result)
}

/// ID encoding configuration
#[derive(Debug, Clone, Copy)]
pub enum IdEncodingConfig {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}

impl ToRaw<'_> for OwnedRawId {
    type Output = DecodedId;
    type Config = IdEncodingConfig;

    fn to_raw(decoded: &DecodedId, config: IdEncodingConfig) -> Result<Self, MltError> {
        use IdEncodingConfig as CFG;

        // skipped one level higher
        let DecodedId(Some(ids)) = decoded else {
            return Err(MltError::InvalidStreamData {
                expected: "Some IDs to encode",
                got: "None".to_string(),
            });
        };

        let optional = if matches!(config, CFG::OptId32 | CFG::OptId64) {
            let present: Vec<bool> = ids.iter().map(Option::is_some).collect();
            let num_values = u32::try_from(present.len()).map_err(|_| MltError::IntegerOverflow)?;

            let num_bytes = present.len().div_ceil(8);
            let mut bytes = vec![0u8; num_bytes];
            for (i, _) in present.into_iter().enumerate().filter(|(_, bit)| *bit) {
                bytes[i / 8] |= 1 << (i % 8);
            }
            let data = crate::utils::encode_byte_rle(&bytes);

            let meta = StreamMeta {
                physical_type: PhysicalStreamType::Present,
                num_values,
                logical_decoder: LogicalDecoder::None,
                physical_decoder: PhysicalDecoder::None,
            };

            Some(OwnedStream {
                meta,
                data: OwnedStreamData::Raw(OwnedDataRaw { data }),
            })
        } else {
            None
        };

        let value = if matches!(config, CFG::Id32 | CFG::OptId32) {
            #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
            let vals: Vec<u32> = ids.iter().filter_map(|&id| id).map(|v| v as u32).collect();
            let num_values = u32::try_from(vals.len()).map_err(|_| MltError::IntegerOverflow)?;
            let data: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();

            let meta = StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values,
                logical_decoder: LogicalDecoder::None,
                physical_decoder: PhysicalDecoder::None,
            };

            OwnedRawIdValue::Id32(OwnedStream {
                meta,
                data: OwnedStreamData::Raw(OwnedDataRaw { data }),
            })
        } else {
            let vals: Vec<u64> = ids.iter().filter_map(|&id| id).collect();

            #[expect(
                clippy::cast_possible_wrap,
                reason = "Values > i64::MAX will wrap, but zigzag+delta handles this correctly"
            )]
            let vals_i64: Vec<i64> = vals.iter().map(|&v| v as i64).collect();
            let encoded = crate::utils::encode_zigzag_delta(&vals_i64);

            let mut data = Vec::new();
            for &val in &encoded {
                crate::utils::encode_varint(&mut data, val);
            }

            let meta = StreamMeta {
                physical_type: PhysicalStreamType::Data(DictionaryType::None),
                num_values: u32::try_from(vals.len()).map_err(|_| MltError::IntegerOverflow)?,
                logical_decoder: LogicalDecoder::Delta,
                physical_decoder: PhysicalDecoder::VarInt,
            };

            OwnedRawIdValue::Id64(OwnedStream {
                meta,
                data: OwnedStreamData::VarInt(OwnedDataVarInt { data }),
            })
        };

        Ok(Self { optional, value })
    }
}

#[cfg(test)]
mod tests {
    use IdEncodingConfig::*;
    use rstest::rstest;

    use super::*;

    // Helper function to encode and decode for roundtrip testing
    fn roundtrip(decoded: &DecodedId, config: IdEncodingConfig) -> DecodedId {
        let raw = OwnedRawId::to_raw(decoded, config).expect("Failed to encode");
        let borrowed_raw = borrowme::borrow(&raw);
        DecodedId::from_raw(borrowed_raw).expect("Failed to decode")
    }

    // Test that each config produces the correct variant and optional stream presence
    #[rstest]
    #[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
    #[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
    #[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
    #[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
    fn test_config_produces_correct_variant(
        #[case] config: IdEncodingConfig,
        #[case] ids: Vec<Option<u64>>,
    ) {
        let input = DecodedId(Some(ids));
        let raw = OwnedRawId::to_raw(&input, config).unwrap();

        match config {
            OptId32 | Id32 => assert!(matches!(raw.value, OwnedRawIdValue::Id32(_))),
            Id64 | OptId64 => assert!(matches!(raw.value, OwnedRawIdValue::Id64(_))),
        }

        match config {
            OptId32 | OptId64 => assert!(raw.optional.is_some()),
            Id32 | Id64 => assert!(raw.optional.is_none()),
        }
    }

    #[rstest]
    #[case::id32_basic(Id32, &[Some(1), Some(2), Some(100), Some(1000)])]
    #[case::id32_single(Id32, &[Some(42)])]
    #[case::id32_boundaries(Id32, &[Some(0), Some(u64::from(u32::MAX))])]
    #[case::id64_basic(Id64, &[Some(1), Some(2), Some(100), Some(1000)])]
    #[case::id64_single(Id64, &[Some(u64::MAX)])]
    #[case::id64_boundaries(Id64, &[Some(0), Some(u64::MAX)])]
    #[case::id64_large_values(        Id64,        &[Some(0), Some(u64::from(u32::MAX)), Some(u64::from(u32::MAX) + 1), Some(u64::MAX)])]
    #[case::opt_id32_with_nulls(OptId32, &[Some(1), None, Some(100), None, Some(1000)])]
    #[case::opt_id32_no_nulls(OptId32, &[Some(1), Some(2), Some(3)])]
    #[case::opt_id32_single_null(OptId32, &[None])]
    #[case::opt_id64_with_nulls(        OptId64,        &[Some(1), None, Some(u64::from(u32::MAX) + 1), None, Some(u64::MAX)]    )]
    #[case::opt_id64_all_nulls(OptId64, &[None, None, None])]
    #[case::none(Id32, &[])]
    fn test_roundtrip(#[case] config: IdEncodingConfig, #[case] ids: &[Option<u64>]) {
        let input = DecodedId(Some(ids.to_vec()));
        let output = roundtrip(&input, config);
        assert_eq!(output, input);
    }

    #[test]
    fn test_sequential_ids_for_delta_encoding() {
        // Sequential IDs should compress well with delta encoding
        let input = DecodedId(Some((1..=100).map(Some).collect()));
        let output = roundtrip(&input, Id64);
        assert_eq!(output, input);
    }

    #[cfg(test)]
    mod proptests {
        use proptest::prelude::*;

        use super::*;
        use crate::{Decodable as _, Encodable as _};

        proptest! {
            #[test]
            fn test_roundtrip_id32(ids in prop::collection::vec(any::<u32>(), 1..100)) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
                assert_roundtrip_succeeds(ids_u64, Id32)?;
            }

            #[test]
            fn test_roundtrip_opt_id32(
                ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100)
            ) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
                assert_roundtrip_succeeds(ids_u64, OptId32)?;
            }

            #[test]
            fn test_roundtrip_id64(ids in prop::collection::vec(any::<u64>(), 1..100)) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
                assert_roundtrip_succeeds(ids_u64, Id64)?;
            }

            #[test]
            fn test_roundtrip_opt_id64(
                ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100)
            ) {
                assert_roundtrip_succeeds(ids, OptId64)?;
            }

            #[test]
            fn test_encodable_trait_api_id32(ids in prop::collection::vec(any::<u32>(), 1..100)) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
                assert_encodable_api_works(ids_u64, Id32)?;
            }

            #[test]
            fn test_encodable_trait_api_opt_id64(
                ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100)
            ) {
                assert_encodable_api_works(ids, OptId64)?;
            }

            #[test]
            fn test_correct_variant_produced_id32(
                ids in prop::collection::vec(1u32..1000u32, 1..50)
            ) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
                assert_produces_correct_variant(ids_u64, Id32)?;
            }

            #[test]
            fn test_correct_variant_produced_id64(
                ids in prop::collection::vec(any::<u64>(), 1..50)
            ) {
                let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
                assert_produces_correct_variant(ids_u64, Id64)?;
            }
        }

        enum ExpectedVariant {
            Id32,
            OptionalId32,
            Id64,
            OptionalId64,
        }

        impl ExpectedVariant {
            fn from_config(config: IdEncodingConfig) -> Self {
                match config {
                    Id32 => ExpectedVariant::Id32,
                    OptId32 => ExpectedVariant::OptionalId32,
                    Id64 => ExpectedVariant::Id64,
                    OptId64 => ExpectedVariant::OptionalId64,
                }
            }

            fn is_id32(&self) -> bool {
                matches!(self, ExpectedVariant::Id32 | ExpectedVariant::OptionalId32)
            }

            fn has_optional(&self) -> bool {
                matches!(
                    self,
                    ExpectedVariant::OptionalId32 | ExpectedVariant::OptionalId64
                )
            }
        }

        /// Helper: Asserts that encoding and decoding with the given config produces the original data
        fn assert_roundtrip_succeeds(
            ids: Vec<Option<u64>>,
            config: IdEncodingConfig,
        ) -> Result<(), TestCaseError> {
            let input = DecodedId(Some(ids.clone()));
            let output = roundtrip(&input, config);
            prop_assert_eq!(output, DecodedId(Some(ids)));
            Ok(())
        }

        /// Helper: Asserts that the Encodable trait API works correctly (encode -> materialize -> decode)
        fn assert_encodable_api_works(
            ids: Vec<Option<u64>>,
            config: IdEncodingConfig,
        ) -> Result<(), TestCaseError> {
            let decoded = DecodedId(Some(ids.clone()));

            let mut id_enum = OwnedId::Decoded(decoded);
            id_enum.encode_with(config).expect("Failed to encode");

            prop_assert!(!id_enum.is_decoded(), "Should be Raw after encoding");
            prop_assert!(id_enum.borrow_raw().is_some(), "Raw variant should be Some");

            let mut borrowed_id = borrowme::borrow(&id_enum);
            borrowed_id.materialize().expect("Failed to materialize");

            if let Id::Decoded(decoded_back) = borrowed_id {
                prop_assert_eq!(decoded_back, DecodedId(Some(ids)));
            } else {
                TestCaseError::fail("Expected Decoded variant after materialization");
            }
            Ok(())
        }

        /// Helper: Asserts that encoding produces the expected variant type for the given config
        fn assert_produces_correct_variant(
            ids: Vec<Option<u64>>,
            config: IdEncodingConfig,
        ) -> Result<(), TestCaseError> {
            let input = DecodedId(Some(ids));
            let raw = OwnedRawId::to_raw(&input, config).expect("Failed to encode");

            let expected = ExpectedVariant::from_config(config);

            if expected.is_id32() {
                prop_assert!(
                    matches!(raw.value, OwnedRawIdValue::Id32(_)),
                    "Expected Id32 variant"
                );
            } else {
                prop_assert!(
                    matches!(raw.value, OwnedRawIdValue::Id64(_)),
                    "Expected Id64 variant"
                );
            }

            if expected.has_optional() {
                prop_assert!(
                    raw.optional.is_some(),
                    "Expected optional stream to be present"
                );
            } else {
                prop_assert!(raw.optional.is_none(), "Expected no optional stream");
            }
            Ok(())
        }
    }
}
