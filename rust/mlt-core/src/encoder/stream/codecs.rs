use std::collections::HashMap;

use fastpfor::FastPFor256;

use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{LogicalEncoding, RleMeta};
use crate::errors::MltResult;

#[derive(Default)]
pub struct Codecs {
    pub(crate) logical: LogicalCodecs,
    pub(crate) physical: PhysicalCodecs,
}

#[derive(Default)]
pub struct PhysicalCodecs {
    pub(crate) u32_tmp: Vec<u32>,
    pub(crate) u8_tmp: Vec<u8>,
    pub(crate) fastpfor: FastPFor256,
}

#[derive(Default)]
pub struct LogicalCodecs {
    pub(crate) u32_tmp: Vec<u32>,
    pub(crate) u32_tmp2: Vec<u32>,
    pub(crate) u64_tmp: Vec<u64>,
    pub(crate) u64_tmp2: Vec<u64>,
    u8_tmp: Vec<u8>,
    u8_tmp2: Vec<u8>,

    /// Reusable scratch for the Hilbert vertex-dictionary builder. The four
    /// slots are taken out via `mem::take` for the duration of a build so the
    /// caller can hold a `&[..]` view into one slot while passing `&mut Codecs`
    /// to a stream writer; capacity is preserved across geometry columns.
    pub(crate) hilbert_offsets: Vec<u32>,
    pub(crate) hilbert_indexed: Vec<u64>,
    pub(crate) hilbert_dict_xy: Vec<i32>,
    pub(crate) hilbert_remap: HashMap<u32, u32>,
}

impl LogicalCodecs {
    #[hotpath::measure]
    pub(crate) fn encode_bools(
        &mut self,
        values: impl ExactSizeIterator<Item = bool>,
    ) -> MltResult<(LogicalEncoding, &[u8])> {
        let num_values = u32::try_from(values.len())?;
        let data = encode_bools_to_bytes(values, &mut self.u8_tmp);
        let encoded = encode_byte_rle(data, &mut self.u8_tmp2);
        let meta = LogicalEncoding::Rle(RleMeta {
            runs: num_values.div_ceil(8),
            num_rle_values: u32::try_from(encoded.len())?,
        });
        Ok((meta, encoded))
    }
}
