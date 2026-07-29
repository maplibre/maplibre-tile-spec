use std::collections::HashMap;

use bytemuck::{NoUninit, cast_slice};
use fastpfor::FastPFor256;

use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{LogicalEncoding, PhysicalEncoding, RleMeta, StreamMeta, StreamType};
use crate::encoder;
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::logical::LogicalEncoder;
use crate::encoder::stream::optimizer::DataProfile;
use crate::encoder::write::{LogicalIntCodec, LogicalIntStreamKind, PhysicalIntStreamKind};
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
        let meta = LogicalEncoding::Rle(RleMeta::Split {
            runs: num_values.div_ceil(8),
            num_rle_values: u32::try_from(encoded.len())?,
        });
        Ok((meta, encoded))
    }
}

impl Codecs {
    pub(crate) fn write_bool_stream(
        &mut self,
        values: impl ExactSizeIterator<Item = bool>,
        stream_type: StreamType,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        let num_values = values.len();
        let (logical, vals) = self.logical.encode_bools(values)?;
        let meta = StreamMeta::new2(stream_type, logical, PhysicalEncoding::None, num_values)?;
        encoder::write_stream_payload(enc, meta, true, vals)
    }

    pub(crate) fn write_presence_stream(
        &mut self,
        values: impl ExactSizeIterator<Item = bool>,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        self.write_bool_stream(values, StreamType::Present, enc)
    }

    #[expect(
        clippy::unused_self,
        reason = "kept as a Codecs method to match the other stream writers"
    )]
    pub(crate) fn write_float_stream<T: NoUninit>(
        &mut self,
        values: &[T],
        stream_type: StreamType,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        #[cfg(not(target_endian = "little"))]
        compile_error!("not implemented for non-little-endian targets");

        let meta = StreamMeta::new_none(stream_type, values.len())?;
        encoder::write_stream_payload(enc, meta, false, cast_slice(values))
    }

    pub(crate) fn write_int_stream<T>(
        &mut self,
        values: &[T],
        ctx: &StreamCtx<'_>,
        enc: &mut Encoder,
    ) -> MltResult<()>
    where
        [T]: LogicalIntStreamKind<Input = T>,
        LogicalCodecs: LogicalIntCodec<[T]>,
    {
        type Output<T> = <[T] as LogicalIntStreamKind>::Output;

        use LogicalEncoding as LE;
        use PhysicalEncoding as PE;

        // FIXME: does StreamMeta encode values.len() or vals1.len()?
        if let Some(int_enc) = enc.override_int_enc(ctx) {
            let rle_layout = enc.config().wire_version().rle_layout();
            let (le, vals) = match int_enc.logical {
                LogicalEncoder::None => (LE::None, self.logical.none(values)),
                LogicalEncoder::Delta => (LE::Delta, self.logical.delta(values)),
                LogicalEncoder::Rle => self.logical.rle(values, rle_layout)?,
                LogicalEncoder::DeltaRle => self.logical.delta_rle(values, rle_layout)?,
            };
            let phys = int_enc.physical;
            return self
                .physical
                .write_encoded_as::<Output<T>>(ctx, enc, le, vals, phys);
        }

        if values.is_empty() {
            let vals1 = self.logical.none(values);
            let vals2 = Output::<T>::none(&mut self.physical, vals1);
            let meta = StreamMeta::new2(ctx.stream_type, LE::None, PE::None, vals1.len())?;
            return encoder::write_stream_payload(enc, meta, false, vals2);
        }

        // A single value has no deltas or runs, so Delta would only zigzag the
        // lone value into something equal or larger, and FastPFOR never pays off
        // for one value. Only the physical layout matters, so skip the
        // competition. Matching the encoded stream binds the lone value with no
        // indexing bounds check. VarInt is shorter until the value needs all its
        // bytes (>= 2^28 for u32, >= 2^56 for u64, post-zigzag for signed),
        // where fixed-width plain avoids VarInt's continuation byte.
        let Self { logical, physical } = self;
        if values.len() == 1
            && let [value] = logical.none(values)
        {
            let n: u64 = (*value).into();
            let varint_len = (u64::BITS - n.leading_zeros()).max(1).div_ceil(7) as usize;
            let encoded = std::slice::from_ref(value);
            let (pe, payload) = if varint_len <= size_of_val(encoded) {
                (PE::VarInt, physical.varint(encoded))
            } else {
                (PE::None, cast_slice(encoded))
            };
            let meta = StreamMeta::new2(ctx.stream_type, LE::None, pe, 1)?;
            return encoder::write_stream_payload(enc, meta, false, payload);
        }

        let allow_fastpfor = enc.config().race_fastpfor();
        let rle_layout = enc.config().wire_version().rle_layout();
        let mut alt = enc.try_alternatives();

        let sample = logical.none(DataProfile::take_sample(values));
        let profile = DataProfile::profile::<<[T] as LogicalIntStreamKind>::Profile>(sample);

        if profile.delta_is_beneficial()
            && (profile.rle_is_viable() || profile.delta_rle_is_viable())
        {
            let (logical_enc, values) = logical.delta_rle(values, rle_layout)?;
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                logical_enc,
                ctx.stream_type,
                allow_fastpfor,
            )?;
        }
        if profile.delta_is_beneficial() {
            let values = logical.delta(values);
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                LE::Delta,
                ctx.stream_type,
                allow_fastpfor,
            )?;
        }
        if profile.rle_is_viable() {
            let (logical_enc, values) = logical.rle(values, rle_layout)?;
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                logical_enc,
                ctx.stream_type,
                allow_fastpfor,
            )?;
        }
        let values = logical.none(values);
        physical.write_alternatives::<Output<T>>(
            &mut alt,
            values,
            LE::None,
            ctx.stream_type,
            allow_fastpfor,
        )
    }
}
