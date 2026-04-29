use std::collections::HashMap;

use fastpfor::FastPFor256;
use num_traits::WrappingSub;
use zigzag::ZigZag;

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
        let meta = LogicalEncoding::Rle(RleMeta {
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
        encoder::write_stream_payload(&mut enc.data, meta, true, vals)
    }

    pub(crate) fn write_presence_stream(
        &mut self,
        values: impl ExactSizeIterator<Item = bool>,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        self.write_bool_stream(values, StreamType::Present, enc)
    }

    pub(crate) fn write_int_stream<T>(
        &mut self,
        values: &[T],
        ctx: &StreamCtx<'_>,
        enc: &mut Encoder,
    ) -> MltResult<()>
    where
        [T]: LogicalIntStreamKind<Input = T>,
        <<[T] as LogicalIntStreamKind>::Profile as ZigZag>::UInt: WrappingSub,
        LogicalCodecs: LogicalIntCodec<[T]>,
    {
        type Output<T> = <[T] as LogicalIntStreamKind>::Output;

        use LogicalEncoding as LE;
        use PhysicalEncoding as PE;

        // FIXME: does StreamMeta encode values.len() or vals1.len()?
        if let Some(int_enc) = enc.override_int_enc(ctx) {
            let (le, vals) = match int_enc.logical {
                LogicalEncoder::None => (LE::None, self.logical.none(values)),
                LogicalEncoder::Delta => (LE::Delta, self.logical.delta(values)),
                LogicalEncoder::Rle => self.logical.rle(values)?,
                LogicalEncoder::DeltaRle => self.logical.delta_rle(values)?,
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
            return encoder::write_stream_payload(&mut enc.data, meta, false, vals2);
        }

        let Self { logical, physical } = self;
        let mut alt = enc.try_alternatives();

        let sample = logical.none(DataProfile::take_sample(values));
        let profile = DataProfile::profile::<<[T] as LogicalIntStreamKind>::Profile>(sample);

        if profile.delta_is_beneficial()
            && (profile.rle_is_viable() || profile.delta_rle_is_viable())
        {
            let (logical_enc, values) = logical.delta_rle(values)?;
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                logical_enc,
                ctx.stream_type,
            )?;
        }
        if profile.delta_is_beneficial() {
            let values = logical.delta(values);
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                LE::Delta,
                ctx.stream_type,
            )?;
        }
        if profile.rle_is_viable() {
            let (logical_enc, values) = logical.rle(values)?;
            physical.write_alternatives::<Output<T>>(
                &mut alt,
                values,
                logical_enc,
                ctx.stream_type,
            )?;
        }
        let values = logical.none(values);
        physical.write_alternatives::<Output<T>>(&mut alt, values, LE::None, ctx.stream_type)
    }
}
