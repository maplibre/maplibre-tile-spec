use crate::decoder::{IdValues, RawId, RawIdValue};
use crate::utils::apply_present;
use crate::{Decode, Decoder, MltResult};

impl Decode<IdValues> for RawId<'_> {
    fn decode(self, decoder: &mut Decoder) -> MltResult<IdValues> {
        RawId::decode(self, decoder)
    }
}

impl RawId<'_> {
    /// Decode into [`IdValues`], charging `dec` before each `Vec` allocation.
    pub fn decode(self, dec: &mut Decoder) -> MltResult<IdValues> {
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

        Ok(IdValues(apply_present(presence, ids_u64, dec)?))
    }
}

impl IdValues {
    #[must_use]
    pub fn values(&self) -> &[Option<u64>] {
        &self.0
    }
}
