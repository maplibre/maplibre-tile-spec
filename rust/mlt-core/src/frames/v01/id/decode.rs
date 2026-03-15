use std::mem::size_of;

use crate::enc_dec::Decode;
use crate::errors::AsMltError as _;
use crate::utils::apply_present;
use crate::v01::{Id, IdValues, RawId, RawIdValue, RawStream};
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

        // apply_present expands the dense values into a sparse Vec<Option<u64>>.
        // The presence-expanded output may be larger; charge the extra slots now.
        let presence_count = presence
            .as_ref()
            .map_or(ids_u64.len(), |p| p.meta.num_values as usize);
        let extra = presence_count.saturating_sub(ids_u64.len());
        dec.consume(u32::try_from(extra * size_of::<Option<u64>>()).or_overflow()?)?;

        Ok(IdValues(apply_present(presence, ids_u64, dec)?))
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn new_raw(presence: Option<RawStream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { presence, value })
    }
}

impl IdValues {
    #[must_use]
    pub fn values(&self) -> &[Option<u64>] {
        &self.0
    }
}
