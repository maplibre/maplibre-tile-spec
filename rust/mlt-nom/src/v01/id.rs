use std::fmt::{Debug, Formatter};
use std::io::Write;

use borrowme::borrowme;

use crate::MltError;
use crate::analyse::{Analyze, StatType};
use crate::decodable::{FromRaw, impl_decodable};
use crate::utils::{BinarySerializer as _, OptSeqOpt};
use crate::v01::{ColumnType, Stream};

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
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::None => Ok(()),
            Self::Raw(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v01::{DataRaw, LogicalDecoder, PhysicalDecoder, PhysicalStreamType, StreamMeta};

    #[test]
    fn test_decode_id32_simple() {
        // Create a simple stream with 3 ID values:
        let ids: Vec<u32> = vec![1, 2, 3];
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
        let ids: Vec<u32> = vec![10, 30];
        let data = ids.iter().flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
        let present_data: Vec<u8> = vec![1, 0x1 << 0 | 0x1 << 2]; // RLE: one time the bit pattern

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
