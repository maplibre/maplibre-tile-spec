use crate::decoder::{ParsedId, RawId, RawIdValue};
use crate::utils::decode_presence;
use crate::{Decode, Decoder, MltResult};

impl<'a> Decode<ParsedId<'a>> for RawId<'a> {
    /// Decode into a [`ParsedId`], charging `dec` before each allocation.
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedId<'a>> {
        let RawId { presence, value } = self;

        let values: Vec<u64> = match value {
            RawIdValue::Id32(stream) => {
                // FIXME: ParsedId should be an enum of u32 or u64 to avoid extra allocation
                let ids = stream.decode_u32s(dec)?;
                dec.consume_items::<u64>(ids.len())?;
                ids.into_iter().map(u64::from).collect()
            }
            RawIdValue::Id64(stream) => stream.decode_u64s(dec)?,
        };

        Ok(ParsedId(decode_presence(presence, values, dec)?))
    }
}
