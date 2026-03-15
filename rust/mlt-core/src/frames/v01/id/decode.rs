use crate::enc_dec::Decode;
use crate::utils::apply_present;
use crate::v01::{Id, IdValues, RawId, RawIdValue, RawPresence};
use crate::{Decoder, MltError};

impl Decode<IdValues> for RawId<'_> {
    fn decode(self, decoder: &mut Decoder) -> Result<IdValues, MltError> {
        RawId::decode(self, decoder)
    }
}

impl RawId<'_> {
    /// Decode into [`IdValues`], charging `dec` before each `Vec` allocation.
    pub fn decode(self, dec: &mut Decoder) -> Result<IdValues, MltError> {
        let RawId { presence, value } = self;

        // Decode the raw integer stream, charging for it before allocation.
        let ids_u64: Vec<u64> = match value {
            RawIdValue::Id32(stream) => {
                let ids = stream.decode_u32s(dec)?;
                ids.into_iter().map(u64::from).collect()
            }
            RawIdValue::Id64(stream) => stream.decode_u64s(dec)?,
        };

        Ok(IdValues(apply_present(presence, ids_u64, dec)?))
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_raw(presence: RawPresence<'a>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { presence, value })
    }
}

impl IdValues {
    #[must_use]
    pub fn values(&self) -> &[Option<u64>] {
        &self.0
    }
}
