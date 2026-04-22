use crate::decoder::{IdValues, RawId, RawIdValue};
use crate::utils::decode_presence;
use crate::utils::presence::Presence;
use crate::{Decode, Decoder, MltResult};

impl Decode<IdValues> for RawId<'_> {
    /// Decode into [`IdValues`], charging `dec` before each `Vec` allocation.
    fn decode(self, dec: &mut Decoder) -> MltResult<IdValues> {
        let RawId { presence, value } = self;

        // Decode the raw integer stream, charging for it before allocation.
        let ids_u64: Vec<u64> = match value {
            RawIdValue::Id32(stream) => {
                // FIXME: IdValues should be an enum of i32 or i64 values to avoid extra allocations
                let ids = stream.decode_u32s(dec)?;
                dec.consume_items::<u64>(ids.len())?;
                ids.into_iter().map(u64::from).collect()
            }
            RawIdValue::Id64(stream) => stream.decode_u64s(dec)?,
        };

        let decoded_presence = decode_presence(presence, ids_u64.len(), dec)?;
        let values = match decoded_presence {
            Presence::AllPresent => {
                dec.consume_items::<Option<u64>>(ids_u64.len())?;
                ids_u64.into_iter().map(Some).collect()
            }
            Presence::Bits(bits) => {
                dec.consume_items::<Option<u64>>(bits.len())?;
                let mut result = Vec::with_capacity(bits.len());
                let mut dense = ids_u64.into_iter();
                for present in bits.iter().by_vals() {
                    result.push(if present { dense.next() } else { None });
                }
                result
            }
        };
        Ok(IdValues(values))
    }
}
