use borrowme::borrowme;

use crate::MltError;
use crate::decodable::{FromRaw, impl_decodable};
use crate::v01::{PackedBitset, Stream};

/// ID column representation, either raw or decoded, or none if there are no IDs
#[borrowme]
#[derive(Debug, Default, PartialEq)]
pub enum Id<'a> {
    #[default]
    None,
    Raw(RawId<'a>),
    Decoded(DecodedId),
}

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawId<'a> {
    optional: Option<Stream<'a>>,
    value: RawIdValue<'a>,
}

/// A sequence of encoded (raw) ID values, either 32-bit or 64-bit unsigned integers
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

/// Decoded ID values as a vector of optional 64-bit unsigned integers
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedId(Option<Vec<Option<u64>>>);

impl_decodable!(Id<'a>, RawId<'a>, DecodedId);

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
            RawIdValue::Id32(stream) => stream
                .decode_bits_u32()?
                .decode_u32()?
                .into_iter()
                .map(u64::from)
                .collect(),
            RawIdValue::Id64(stream) => todo!("64-bit ID decoding not yet implemented"),
        };

        // Apply the offsets of these IDs via an optional/present bitmask
        let ids_optional: Vec<Option<u64>> = if let Some(optional_stream) = optional {
            let num_features = optional_stream.meta.num_values;
            let present_bits = optional_stream.decode_packed_bitset()?;

            apply_present_bitset(num_features, present_bits, &ids_u64)?
        } else {
            // No optional stream, so all IDs are present
            ids_u64.into_iter().map(Some).collect()
        };

        Ok(DecodedId(Some(ids_optional)))
    }
}

/// The ids_u64 vector only contains values for features where the bit is set
/// We need to iterate through the bitset and pull from ids_u64 in order
fn apply_present_bitset(
    num_features: u32,
    present_bits: PackedBitset,
    ids_u64: &[u64],
) -> Result<Vec<Option<u64>>, MltError> {
    let present_bit_count = present_bits.count_set_bits();
    if usize::try_from(present_bit_count)
        .map(|pbc| pbc != ids_u64.len())
        .unwrap_or(false)
    {
        return Err(MltError::InvalidStreamData {
            expected: "Number of ID values in the presence bitset does not match the number of provided IDs",
            got: format!(
                "{present_bit_count} bits set in the present bitset, but {} values for IDs",
                ids_u64.len()
            ),
        });
    }
    debug_assert!(
        present_bit_count <= num_features,
        "num_features is derived from present_bit_count and should always be greater"
    );
    debug_assert!(
        usize::try_from(num_features)
            .map(|feats| ids_u64.len() <= feats)
            .unwrap_or(false),
        "Since num_features <= present_bit_count (upper bound: all bits set) and ids_u64.len() == present_bit_count, there cannot be more IDs than features"
    );

    let mut id_index = 0;
    let mut result = vec![None; num_features as usize];
    for (feature_index, id) in result.iter_mut().enumerate() {
        // todo: benchmark, if nextSetBit like in the present_bitset.cpp is faster for real world usage
        if present_bits.test_bit(feature_index) {
            // below cannot panic since we checked present_bit_count == ids_u64.len()
            id.replace(ids_u64[id_index]);
            id_index += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v01::{DataRaw, LogicalDecoder, PhysicalDecoder, PhysicalStreamType, StreamMeta};

    #[test]
    fn test_decode_id32_simple() {
        // Create a simple stream with 3 ID values:
        let ids = vec![1_u32, 2, 3];
        let data = ids.iter().flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();

        let meta = StreamMeta {
            physical_type: PhysicalStreamType::Data(crate::v01::DictionaryType::None),
            num_values: 3,
            logical_decoder: LogicalDecoder::None,
            physical_decoder: PhysicalDecoder::None,
        };

        let stream = Stream::new(meta, DataRaw::new(&data));
        let raw_id = RawId {
            optional: None,
            value: RawIdValue::Id32(stream),
        };

        let decoded = DecodedId::from_raw(raw_id).expect("Failed to decode IDs");

        assert_eq!(
            decoded,
            DecodedId(Some(ids.into_iter().map(u64::from).map(Some).collect())),
            "Decoded IDs should match expected values"
        );
    }

    #[test]
    fn test_decode_id32_with_nulls() {
        // Test IDs with a present bitmask:
        let expected = vec![Some(10), None, Some(30)];
        // Only 2 values in the ID stream (10 and 30), not 3!
        let ids = [10_u32, 30];
        let data = ids.iter().flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
        let present_data: Vec<u8> = vec![0x1 << 0 | 0x1 << 2];

        let id_meta = StreamMeta {
            physical_type: PhysicalStreamType::Data(crate::v01::DictionaryType::None),
            num_values: 2, // Only 2 actual values
            logical_decoder: LogicalDecoder::None,
            physical_decoder: PhysicalDecoder::None,
        };

        let present_meta = StreamMeta {
            physical_type: PhysicalStreamType::Present,
            num_values: 3, // 3 features total
            logical_decoder: LogicalDecoder::None,
            physical_decoder: PhysicalDecoder::None,
        };

        let id_stream = Stream::new(id_meta, DataRaw::new(&data));
        let present_stream = Stream::new(present_meta, DataRaw::new(&present_data));

        let raw_id = RawId {
            optional: Some(present_stream),
            value: RawIdValue::Id32(id_stream),
        };

        let decoded = DecodedId::from_raw(raw_id).expect("Failed to decode IDs with nulls");

        assert_eq!(
            decoded,
            DecodedId(Some(expected)),
            "Decoded IDs should respect the present bitmask"
        );
    }
}
