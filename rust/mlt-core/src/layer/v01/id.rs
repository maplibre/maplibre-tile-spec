use std::fmt::{Debug, Formatter};
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decode::{FromEncoded, impl_decodable};
use crate::encode::{FromDecoded, impl_encodable};
use crate::utils::{
    BinarySerializer as _, OptSeqOpt, apply_present, encode_bools_to_bytes, encode_byte_rle,
};
use crate::v01::{
    ColumnType, Encoder, LogicalEncoder, LogicalEncoding, OwnedEncodedData, OwnedStream,
    OwnedStreamData, PhysicalEncoder, PhysicalEncoding, Stream, StreamMeta, StreamType,
};

/// ID column representation, either encoded or decoded, or none if there are no IDs
#[borrowme]
#[derive(Debug, Default, PartialEq)]
pub enum Id<'a> {
    #[default]
    None,
    Encoded(EncodedId<'a>),
    Decoded(DecodedId),
}

impl OwnedId {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::None => Ok(()),
            Self::Encoded(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::None => Ok(()),
            Self::Encoded(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

impl Analyze for Id<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::None => 0,
            Self::Encoded(d) => d.collect_statistic(stat),
            Self::Decoded(d) => d.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::None => {}
            Self::Encoded(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct EncodedId<'a> {
    optional: Option<Stream<'a>>,
    value: EncodedIdValue<'a>,
}

impl Default for OwnedEncodedId {
    fn default() -> Self {
        Self {
            optional: None,
            value: OwnedEncodedIdValue::Id32(OwnedStream::empty_without_encoding()),
        }
    }
}
impl OwnedEncodedId {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match (&self.optional, &self.value) {
            (None, OwnedEncodedIdValue::Id32(_)) => ColumnType::Id.write_to(writer)?,
            (None, OwnedEncodedIdValue::Id64(_)) => ColumnType::LongId.write_to(writer)?,
            (Some(_), OwnedEncodedIdValue::Id32(_)) => ColumnType::OptId.write_to(writer)?,
            (Some(_), OwnedEncodedIdValue::Id64(_)) => ColumnType::OptLongId.write_to(writer)?,
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        if let Some(opt) = &self.optional {
            writer.write_boolean_stream(opt)?;
        }
        match &self.value {
            OwnedEncodedIdValue::Id32(s) | OwnedEncodedIdValue::Id64(s) => {
                writer.write_stream(s)?;
            }
        }
        Ok(())
    }
}

impl Analyze for EncodedId<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.optional.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

/// A sequence of encoded ID values, either 32-bit or 64-bit unsigned integers
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum EncodedIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

impl Analyze for EncodedIdValue<'_> {
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

impl_decodable!(Id<'a>, EncodedId<'a>, DecodedId);
impl_encodable!(OwnedId, DecodedId, OwnedEncodedId);

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

impl<'a> From<EncodedId<'a>> for Id<'a> {
    fn from(value: EncodedId<'a>) -> Self {
        Self::Encoded(value)
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_encoded(optional: Option<Stream<'a>>, value: EncodedIdValue<'a>) -> Self {
        Self::Encoded(EncodedId { optional, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedId::from_encoded(v)?,
            Self::Decoded(v) => v,
            Self::None => DecodedId(None),
        })
    }
}

impl<'a> FromEncoded<'a> for DecodedId {
    type Input = EncodedId<'a>;

    fn from_encoded(EncodedId { optional, value }: EncodedId<'_>) -> Result<Self, MltError> {
        // Decode the ID values first
        let ids_u64: Vec<u64> = match value {
            EncodedIdValue::Id32(stream) => {
                // Decode 32-bit IDs as u32, then convert to u64
                let ids: Vec<u32> = stream.decode_bits_u32()?.decode_u32()?;
                ids.into_iter().map(u64::from).collect()
            }
            EncodedIdValue::Id64(stream) => {
                // Decode 64-bit IDs directly as u64
                stream.decode_u64()?
            }
        };

        let presence = if let Some(c) = optional {
            Some(c.decode_bools()?)
        } else {
            None
        };
        let ids_optional = apply_present(presence, ids_u64)?;

        Ok(DecodedId(Some(ids_optional)))
    }
}

/// How to encode IDs
#[derive(Debug, Clone, Copy)]
pub struct IdEncoder {
    pub logical: LogicalEncoder,
    pub id_width: IdWidth,
}
impl IdEncoder {
    #[must_use]
    pub fn new(logical: LogicalEncoder, id_width: IdWidth) -> Self {
        Self { logical, id_width }
    }
}

/// How wide are the IDs
#[derive(Debug, Clone, Copy)]
pub enum IdWidth {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}

impl FromDecoded<'_> for OwnedEncodedId {
    type Input = DecodedId;
    type Encoder = IdEncoder;

    fn from_decoded(decoded: &Self::Input, config: IdEncoder) -> Result<Self, MltError> {
        use IdWidth as CFG;

        let empty_vec =Vec::new();
        let ids = match decoded {
            DecodedId(Some(ids))=> ids,
            DecodedId(None) => &empty_vec,
        };

        let optional = if matches!(config.id_width, CFG::OptId32 | CFG::OptId64) {
            let present: Vec<bool> = ids.iter().map(Option::is_some).collect();
            let num_values = u32::try_from(present.len())?;
            let data = encode_byte_rle(&encode_bools_to_bytes(&present));

            let meta = StreamMeta::new(
                StreamType::Present,
                LogicalEncoding::None,
                PhysicalEncoding::None,
                num_values,
            );

            Some(OwnedStream {
                meta,
                data: OwnedStreamData::Encoded(OwnedEncodedData { data }),
            })
        } else {
            None
        };

        let value = if matches!(config.id_width, CFG::Id32 | CFG::OptId32) {
            #[expect(clippy::cast_possible_truncation, reason = "truncation was requested")]
            let vals: Vec<u32> = ids.iter().filter_map(|&id| id).map(|v| v as u32).collect();
            OwnedEncodedIdValue::Id32(OwnedStream::encode_u32s(
                &vals,
                Encoder::new(config.logical, PhysicalEncoder::None),
            )?)
        } else {
            let vals: Vec<u64> = ids.iter().filter_map(|&id| id).collect();
            OwnedEncodedIdValue::Id64(OwnedStream::encode_u64s(
                &vals,
                Encoder::new(config.logical, PhysicalEncoder::VarInt),
            )?)
        };

        Ok(Self { optional, value })
    }
}

#[cfg(test)]
mod tests {
    use IdWidth::*;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::{Decodable as _, Encodable as _};

    // Helper function to encode and decode for roundtrip testing
    fn roundtrip(decoded: &DecodedId, config: IdEncoder) -> DecodedId {
        let encoded = OwnedEncodedId::from_decoded(decoded, config).expect("Failed to encode");
        let borrowed_encoded = borrowme::borrow(&encoded);
        DecodedId::from_encoded(borrowed_encoded).expect("Failed to decode")
    }

    // Test that each config produces the correct variant and optional stream presence
    #[rstest]
    #[case::id32(Id32, vec![Some(1), Some(2), Some(3)])]
    #[case::opt_id32(OptId32, vec![Some(1), None, Some(3)])]
    #[case::id64(Id64, vec![Some(1), Some(2), Some(3)])]
    #[case::opt_id64(OptId64, vec![Some(1), None, Some(3)])]
    fn test_config_produces_correct_variant(
        #[case] id_width: IdWidth,
        #[case] ids: Vec<Option<u64>>,
    ) {
        let input = DecodedId(Some(ids));
        let config = IdEncoder {
            logical: LogicalEncoder::None,
            id_width,
        };
        let encoded = OwnedEncodedId::from_decoded(&input, config).unwrap();

        match id_width {
            OptId32 | Id32 => assert!(matches!(encoded.value, OwnedEncodedIdValue::Id32(_))),
            Id64 | OptId64 => assert!(matches!(encoded.value, OwnedEncodedIdValue::Id64(_))),
        }

        match id_width {
            OptId32 | OptId64 => assert!(encoded.optional.is_some()),
            Id32 | Id64 => assert!(encoded.optional.is_none()),
        }
    }

    #[rstest]
    #[case::id32_basic(Id32, &[Some(1), Some(2), Some(100), Some(1000)])]
    #[case::id32_single(Id32, &[Some(42)])]
    #[case::id32_boundaries(Id32, &[Some(0), Some(u64::from(u32::MAX))])]
    #[case::id64_basic(Id64, &[Some(1), Some(2), Some(100), Some(1000)])]
    #[case::id64_single(Id64, &[Some(u64::MAX)])]
    #[case::id64_boundaries(Id64, &[Some(0), Some(u64::MAX)])]
    #[case::id64_large_values(Id64, &[Some(0), Some(u64::from(u32::MAX)), Some(u64::from(u32::MAX) + 1), Some(u64::MAX)])]
    #[case::opt_id32_with_nulls(OptId32, &[Some(1), None, Some(100), None, Some(1000)])]
    #[case::opt_id32_no_nulls(OptId32, &[Some(1), Some(2), Some(3)])]
    #[case::opt_id32_single_null(OptId32, &[None])]
    #[case::opt_id64_with_nulls(OptId64, &[Some(1), None, Some(u64::from(u32::MAX) + 1), None, Some(u64::MAX)])]
    #[case::opt_id64_all_nulls(OptId64, &[None, None, None])]
    #[case::none(Id32, &[])]
    fn test_roundtrip(#[case] id_width: IdWidth, #[case] ids: &[Option<u64>]) {
        let input = DecodedId(Some(ids.to_vec()));
        let config = IdEncoder {
            logical: LogicalEncoder::None,
            id_width,
        };
        let output = roundtrip(&input, config);
        assert_eq!(output, input);
    }

    #[rstest]
    fn test_sequential_ids(
        #[values(LogicalEncoder::None)] logical: LogicalEncoder,
        #[values(Id32, OptId32, Id64, OptId64)] id_width: IdWidth,
    ) {
        let input = DecodedId(Some((1..=100).map(Some).collect()));
        let config = IdEncoder { logical, id_width };
        let output = roundtrip(&input, config);
        assert_eq!(output, input);
    }

    proptest! {
        #[test]
        fn test_roundtrip_id32(
            ids in prop::collection::vec(any::<u32>(), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
            assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: Id32,logical})?;
        }

        #[test]
        fn test_roundtrip_opt_id32(
            ids in prop::collection::vec(prop::option::of(any::<u32>()), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| id.map(u64::from)).collect();
            assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: OptId32,logical})?;
        }

        #[test]
        fn test_roundtrip_id64(
            ids in prop::collection::vec(any::<u64>(), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
            assert_roundtrip_succeeds(ids_u64, IdEncoder{ id_width: Id64, logical})?;
        }

        #[test]
        fn test_roundtrip_opt_id64(
            ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            assert_roundtrip_succeeds(ids, IdEncoder{ id_width: OptId64, logical})?;
        }

        #[test]
        fn test_encodable_trait_api_id32(
            ids in prop::collection::vec(any::<u32>(), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
            assert_encodable_api_works(ids_u64, IdEncoder{ id_width: Id32,logical})?;
        }

        #[test]
        fn test_encodable_trait_api_opt_id64(
            ids in prop::collection::vec(prop::option::of(any::<u64>()), 1..100),
            logical in any::<LogicalEncoder>()
        ) {
            assert_encodable_api_works(ids, IdEncoder{ id_width: OptId64,logical})?;
        }

        #[test]
        fn test_correct_variant_produced_id32(
            ids in prop::collection::vec(1u32..1000u32, 1..50),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(u64::from(id))).collect();
            assert_produces_correct_variant(ids_u64, IdEncoder{ id_width: Id32,logical})?;
        }

        #[test]
        fn test_correct_variant_produced_id64(
            ids in prop::collection::vec(any::<u64>(), 1..50),
            logical in any::<LogicalEncoder>()
        ) {
            let ids_u64: Vec<Option<u64>> = ids.iter().map(|&id| Some(id)).collect();
            assert_produces_correct_variant(ids_u64, IdEncoder{ id_width: Id64,logical})?;
        }
    }

    /// Helper: Asserts that encoding and decoding with the given config produces the original data
    fn assert_roundtrip_succeeds(
        ids: Vec<Option<u64>>,
        config: IdEncoder,
    ) -> Result<(), TestCaseError> {
        let input = DecodedId(Some(ids.clone()));
        let output = roundtrip(&input, config);
        prop_assert_eq!(output, DecodedId(Some(ids)));
        Ok(())
    }

    /// Helper: Asserts that the Encodable trait API works correctly (encode -> materialize -> decode)
    fn assert_encodable_api_works(
        ids: Vec<Option<u64>>,
        config: IdEncoder,
    ) -> Result<(), TestCaseError> {
        let decoded = DecodedId(Some(ids.clone()));

        let mut id_enum = OwnedId::Decoded(decoded);
        id_enum.encode_with(config).expect("Failed to encode");

        prop_assert!(!id_enum.is_decoded(), "Should be Encoded after encoding");
        prop_assert!(
            id_enum.borrow_encoded().is_some(),
            "Encoded variant should be Some"
        );

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
        config: IdEncoder,
    ) -> Result<(), TestCaseError> {
        let input = DecodedId(Some(ids));
        let encoded = OwnedEncodedId::from_decoded(&input, config).expect("Failed to encode");

        if matches!(config.id_width, Id32 | OptId32) {
            prop_assert!(
                matches!(encoded.value, OwnedEncodedIdValue::Id32(_)),
                "Expected Id32 variant"
            );
        } else {
            prop_assert!(
                matches!(encoded.value, OwnedEncodedIdValue::Id64(_)),
                "Expected Id64 variant"
            );
        }

        if matches!(config.id_width, OptId32 | OptId64) {
            prop_assert!(
                encoded.optional.is_some(),
                "Expected optional stream to be present"
            );
        } else {
            prop_assert!(encoded.optional.is_none(), "Expected no optional stream");
        }
        Ok(())
    }
}
