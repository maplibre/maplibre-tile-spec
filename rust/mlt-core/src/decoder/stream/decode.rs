use std::borrow::Cow;
use std::mem;

use bitvec::prelude::{BitSlice, BitVec, Lsb0};
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::codecs::bytes::{PhysicalWord, debug_assert_length, decode_bytes_to_words};
use crate::codecs::rle::decode_byte_rle;
use crate::codecs::varint::{parse_varint_vec, parse_varint_vec_all};
#[cfg(feature = "unstable-v2")]
use crate::decoder::IntLogical;
#[cfg(feature = "unstable-v2")]
use crate::decoder::RleMeta;
use crate::decoder::{
    BoolLogical, FloatLogical, LogicalEncoding, LogicalValue, PhysicalEncoding, RawStream,
};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::{Decoder, MltError, MltResult};

impl<'a> RawStream<'a> {
    /// Decode a boolean stream (presence or bool data) into a packed bitvector.
    ///
    /// Both wire formats store one bit per value in an LSB-first packed bitmap;
    /// they differ only in how that bitmap is framed, which the logical encoding
    /// distinguishes:
    /// - tag `0x01` (`logical = Rle`): byte-RLE compressed bitmap, decompressed
    ///   into an owned `BitVec`.
    /// - tag `0x02` (`logical = None`): raw bitmap, borrowed straight from the tile
    ///   bytes - the same representation as a v2 presence bitfield.
    ///
    /// The result is always exactly `num_values` bits.
    pub(crate) fn decode_bitvec(self, dec: &mut Decoder) -> MltResult<Cow<'a, BitSlice<u8, Lsb0>>> {
        let num_values = self.meta.num_values.into_usize();
        let num_bytes = num_values.div_ceil(8);
        match self.meta.encoding.logical {
            LogicalEncoding::Bool(BoolLogical::ByteRle(_)) => {
                let bytes = decode_byte_rle(self.data, num_bytes, dec)?;
                fail_if_invalid_stream_size(bytes.len(), num_bytes)?;
                let mut bits = BitVec::<u8, Lsb0>::from_vec(bytes);
                bits.truncate(num_values);
                Ok(Cow::Owned(bits))
            }
            LogicalEncoding::Bool(BoolLogical::None)
                if self.meta.encoding.physical == PhysicalEncoding::None =>
            {
                fail_if_invalid_stream_size(self.data.len(), num_bytes)?;
                Ok(Cow::Borrowed(&self.data.view_bits::<Lsb0>()[..num_values]))
            }
            LogicalEncoding::Bool(BoolLogical::None)
            | LogicalEncoding::Int(_)
            | LogicalEncoding::Float(_)
            | LogicalEncoding::Vertex(_) => {
                Err(MltError::NotImplemented("unsupported bool stream encoding"))
            }
        }
    }

    /// Decode a boolean data stream into one `bool` per value, charging `dec`.
    ///
    /// Prefer [`RawStream::decode_bitvec`], which keeps the wire's packed layout.
    /// This 8x expansion exists for the dense `Vec<bool>` of a boolean column.
    pub fn decode_bools(self, dec: &mut Decoder) -> MltResult<Vec<bool>> {
        let bits = self.decode_bitvec(dec)?;
        let mut bools = dec.alloc(bits.len())?;
        bools.extend(bits.iter().by_vals());
        debug_assert_length(&bools, bits.len());
        Ok(bools)
    }

    /// Decode via physical type `W`, then narrow to `N`, erroring if a value is out of range.
    pub fn decode_narrow<N, W>(self, dec: &mut Decoder) -> MltResult<Vec<N>>
    where
        W: DecodeInt,
        N: TryFrom<W>,
        MltError: From<<N as TryFrom<W>>::Error>,
    {
        self.decode_ints::<W>(dec)?
            .into_iter()
            .map(N::try_from)
            .collect::<Result<Vec<N>, _>>()
            .map_err(Into::into)
    }

    /// Decode an integer stream into `Vec<T>`, applying the logical transform.
    ///
    /// Unsigned `T` with no logical transform decodes straight into a fresh `Vec`.
    /// Everything else goes through the decoder's scratch buffer, since signed
    /// types always need at least a zigzag transform.
    pub fn decode_ints<T: DecodeInt>(self, dec: &mut Decoder) -> MltResult<Vec<T>> {
        let meta = self.meta;
        if meta.encoding.logical.is_identity()
            && let Some(out) = T::decode_none_passthrough(&self, dec)?
        {
            return Ok(out);
        }
        let mut buf = mem::take(T::scratch(dec));
        self.decode_bits::<T::Physical>(&mut buf, dec)?;
        let result = T::logical_decode(LogicalValue::new(meta), &buf, dec);
        *T::scratch(dec) = buf;
        T::scratch(dec).clear();
        result
    }

    /// Decode a stream of `f32`/`f64` from raw little-endian bytes, charging `dec`.
    ///
    /// Raw is the only float representation either wire format can express today.
    /// Both fields are matched explicitly rather than defaulted to raw, so a
    /// stream tagged with an encoding floats do not have is rejected instead of
    /// being reinterpreted as little-endian bytes.
    pub fn decode_floats<T>(self, dec: &mut Decoder) -> MltResult<Vec<T>>
    where
        T: num_traits::FromBytes,
        for<'b> <T as num_traits::FromBytes>::Bytes: TryFrom<&'b [u8]>,
    {
        match self.meta.encoding.logical {
            LogicalEncoding::Float(FloatLogical::None) => {}
            LogicalEncoding::Float(FloatLogical::Dict | FloatLogical::Alp(_))
            | LogicalEncoding::Int(_)
            | LogicalEncoding::Bool(_)
            | LogicalEncoding::Vertex(_) => {
                return Err(MltError::UnsupportedLogicalEncoding(
                    self.meta.encoding.logical,
                    "float streams, which are stored raw",
                ));
            }
        }
        match self.meta.encoding.physical {
            PhysicalEncoding::None => {}
            PhysicalEncoding::VarInt => {
                return Err(MltError::UnsupportedPhysicalEncoding("varint floats"));
            }
            PhysicalEncoding::FastPFor(_) => {
                return Err(MltError::UnsupportedPhysicalEncoding("FastPFOR floats"));
            }
        }
        let num = self.meta.num_values.into_usize();
        let width = size_of::<T>();
        fail_if_invalid_stream_size(self.data.len(), num.checked_mul(width).or_overflow()?)?;
        dec.consume_items::<T>(num)?;

        Ok(self
            .data
            .chunks_exact(width)
            .map(|chunk| {
                T::from_le_bytes(
                    &chunk
                        .try_into()
                        .ok()
                        .expect("infallible: chunks_exact(width)"),
                )
            })
            .collect())
    }

    /// Physically decode the stream into `buf` as `T` (`u32` or `u64`) values.
    ///
    /// `buf` is cleared first. The caller decides whether the result is charged to `dec`.
    /// `FastPFOR` is `u32`-only; decoding a `u64` `FastPFOR` stream returns an error.
    pub fn decode_bits<T: PhysicalWord>(
        &self,
        buf: &mut Vec<T>,
        dec: &mut Decoder,
    ) -> MltResult<()> {
        buf.clear();
        match self.meta.encoding.physical {
            PhysicalEncoding::None => {
                let (_, values) = decode_bytes_to_words::<T>(self.data, self.meta.num_values, dec)?;
                *buf = values;
            }
            PhysicalEncoding::FastPFor(kind) => {
                *buf = T::decode_fastpfor(self.data, self.meta.num_values, kind, dec)?;
            }
            PhysicalEncoding::VarInt => {
                // v2 interleaved-RLE stores no run count on the wire: `num_values`
                // is the decoded count, so the varint pairs are scanned to the end.
                *buf = if self.meta.encoding.logical.scans_to_end() {
                    parse_varint_vec_all::<T>(self.data, dec)?
                } else {
                    let (_, values) = parse_varint_vec::<T>(self.data, self.meta.num_values, dec)?;
                    values
                };
            }
        }
        Ok(())
    }
}

/// Logical output integer type of a decoded stream (`i32` / `u32` / `i64` / `u64`).
///
/// Decoder-side mirror of the encoder's `LogicalIntStreamKind`.
pub trait DecodeInt: Sized {
    /// Physical word width the stream is decoded into before the logical transform.
    type Physical: PhysicalWord;

    /// The reusable scratch buffer the decoder holds for this physical width.
    fn scratch(dec: &mut Decoder) -> &mut Vec<Self::Physical>;

    /// Apply the logical transform (zigzag / delta / RLE / Morton / …).
    fn logical_decode(
        lv: LogicalValue,
        data: &[Self::Physical],
        dec: &mut Decoder,
    ) -> MltResult<Vec<Self>>;

    /// Fast path for [`IntLogical::None`]:
    /// for unsigned types the physical words are already the output, so decode straight into a fresh `Vec`.
    /// Signed types return `None` (zigzag transform always required), so they fall through to the general path.
    fn decode_none_passthrough(
        _stream: &RawStream<'_>,
        _dec: &mut Decoder,
    ) -> MltResult<Option<Vec<Self>>> {
        Ok(None)
    }
}

impl DecodeInt for i32 {
    type Physical = u32;

    fn scratch(dec: &mut Decoder) -> &mut Vec<u32> {
        &mut dec.buffer_u32
    }

    fn logical_decode(lv: LogicalValue, data: &[u32], dec: &mut Decoder) -> MltResult<Vec<Self>> {
        lv.decode_i32(data, dec)
    }
}

impl DecodeInt for u32 {
    type Physical = Self;

    fn scratch(dec: &mut Decoder) -> &mut Vec<Self> {
        &mut dec.buffer_u32
    }

    fn logical_decode(lv: LogicalValue, data: &[Self], dec: &mut Decoder) -> MltResult<Vec<Self>> {
        lv.decode_u32(data, dec)
    }

    fn decode_none_passthrough(
        stream: &RawStream<'_>,
        dec: &mut Decoder,
    ) -> MltResult<Option<Vec<Self>>> {
        let mut out = Vec::new();
        stream.decode_bits::<Self>(&mut out, dec)?;
        Ok(Some(out))
    }
}

impl DecodeInt for i64 {
    type Physical = u64;

    fn scratch(dec: &mut Decoder) -> &mut Vec<u64> {
        &mut dec.buffer_u64
    }

    fn logical_decode(lv: LogicalValue, data: &[u64], dec: &mut Decoder) -> MltResult<Vec<Self>> {
        lv.decode_i64(data, dec)
    }
}

impl DecodeInt for u64 {
    type Physical = Self;

    fn scratch(dec: &mut Decoder) -> &mut Vec<Self> {
        &mut dec.buffer_u64
    }

    fn logical_decode(lv: LogicalValue, data: &[Self], dec: &mut Decoder) -> MltResult<Vec<Self>> {
        lv.decode_u64(data, dec)
    }

    fn decode_none_passthrough(
        stream: &RawStream<'_>,
        dec: &mut Decoder,
    ) -> MltResult<Option<Vec<Self>>> {
        let mut out = Vec::new();
        stream.decode_bits::<Self>(&mut out, dec)?;
        Ok(Some(out))
    }
}

impl LogicalEncoding {
    /// Whether the physical word count is unknown up front and the payload is
    /// scanned to its end: v2 interleaved-RLE stores no run count on the wire, and
    /// `num_values` holds the *decoded* count instead of the encoded word count.
    #[cfg(feature = "unstable-v2")]
    pub(crate) fn scans_to_end(self) -> bool {
        matches!(
            self,
            Self::Int(
                IntLogical::Rle(RleMeta::Interleaved { .. })
                    | IntLogical::DeltaRle(RleMeta::Interleaved { .. })
            )
        )
    }

    /// Without `unstable-v2`, no logical encoding ever scans to end: v1's RLE
    /// always carries an explicit run count.
    #[cfg(not(feature = "unstable-v2"))]
    #[expect(clippy::unused_self, reason = "tmp because feature gate")]
    pub(crate) fn scans_to_end(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::codecs::bytes::encode_bools_to_bytes;
    use crate::codecs::rle::encode_byte_rle;
    use crate::decoder::{
        DictionaryType, FastPForKind, IntEncoding, RleMeta, StreamMeta, StreamType, ValueKind,
    };
    use crate::test_helpers::dec;

    fn packed(bools: &[bool]) -> Vec<u8> {
        let mut target = Vec::new();
        encode_bools_to_bytes(bools.iter().copied(), &mut target).to_vec()
    }

    fn raw_bitmap(data: &[u8], num_values: usize) -> RawStream<'_> {
        let meta = StreamMeta::new_none(StreamType::Present, ValueKind::Bool, num_values).unwrap();
        RawStream::new(meta, data)
    }

    fn byte_rle(data: &[u8], num_values: usize) -> RawStream<'_> {
        let logical = LogicalEncoding::Bool(BoolLogical::ByteRle(RleMeta::Split {
            runs: u32::try_from(num_values.div_ceil(8)).unwrap(),
            num_rle_values: u32::try_from(data.len()).unwrap(),
        }));
        let meta = StreamMeta::new2(
            StreamType::Present,
            logical,
            PhysicalEncoding::None,
            num_values,
        )
        .unwrap();
        RawStream::new(meta, data)
    }

    fn compressed(bools: &[bool]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encode_byte_rle(&packed(bools), &mut encoded).to_vec()
    }

    proptest! {
        #[test]
        fn decode_bitvec_borrows_raw_bitmap_from_tile_bytes(bools: Vec<bool>) {
            let bytes = packed(&bools);
            let bits = raw_bitmap(&bytes, bools.len()).decode_bitvec(&mut dec()).unwrap();
            prop_assert!(matches!(bits, Cow::Borrowed(_)));
            prop_assert_eq!(bits.iter().by_vals().collect::<Vec<bool>>(), bools);
        }

        #[test]
        fn decode_bitvec_expands_byte_rle(bools: Vec<bool>) {
            let bytes = compressed(&bools);
            let bits = byte_rle(&bytes, bools.len()).decode_bitvec(&mut dec()).unwrap();
            prop_assert_eq!(bits.len(), bools.len());
            prop_assert_eq!(bits.iter().by_vals().collect::<Vec<bool>>(), bools);
        }

        #[test]
        fn decode_bools_agrees_with_decode_bitvec(bools: Vec<bool>) {
            let bytes = compressed(&bools);
            let decoded = byte_rle(&bytes, bools.len()).decode_bools(&mut dec()).unwrap();
            prop_assert_eq!(decoded, bools);
        }
    }

    #[test]
    fn decode_bitvec_rejects_short_raw_bitmap() {
        let err = raw_bitmap(&[0xFF], 9)
            .decode_bitvec(&mut dec())
            .unwrap_err();
        assert!(matches!(err, MltError::InvalidDecodingStreamSize(1, 2)));
    }

    #[test]
    fn decode_bitvec_rejects_truncated_byte_rle() {
        // One literal byte, where nine bits need two.
        let err = byte_rle(&[0xFF, 0b0101_0101], 9)
            .decode_bitvec(&mut dec())
            .unwrap_err();
        assert!(matches!(err, MltError::InvalidDecodingStreamSize(1, 2)));
    }

    #[test]
    fn decode_bools_rejects_truncated_byte_rle() {
        let err = byte_rle(&[0xFF, 0b0101_0101], 9)
            .decode_bools(&mut dec())
            .unwrap_err();
        assert!(matches!(err, MltError::InvalidDecodingStreamSize(1, 2)));
    }

    #[test]
    fn decode_bitvec_rejects_varint_physical_encoding() {
        let meta = StreamMeta::new2(
            StreamType::Present,
            LogicalEncoding::Bool(BoolLogical::None),
            PhysicalEncoding::VarInt,
            8,
        )
        .unwrap();
        let err = RawStream::new(meta, &[0x01])
            .decode_bitvec(&mut dec())
            .unwrap_err();
        assert!(matches!(
            err,
            MltError::NotImplemented("unsupported bool stream encoding")
        ));
    }

    const DATA: StreamType = StreamType::Data(DictionaryType::None);

    fn float_stream(logical: LogicalEncoding, physical: PhysicalEncoding) -> RawStream<'static> {
        const BYTES: [u8; 8] = [0; 8];
        let meta = StreamMeta::new(DATA, IntEncoding::new(logical, physical), 2);
        RawStream::new(meta, &BYTES)
    }

    #[rstest]
    #[case::varint(PhysicalEncoding::VarInt)]
    #[case::fastpfor(PhysicalEncoding::FastPFor(FastPForKind::Block256Be))]
    #[cfg_attr(
        feature = "unstable-v2",
        case::fastpfor128(PhysicalEncoding::FastPFor(FastPForKind::Block128Le))
    )]
    fn decode_floats_rejects_non_raw_physical(#[case] physical: PhysicalEncoding) {
        let stream = float_stream(LogicalEncoding::Float(FloatLogical::None), physical);
        let err = stream.decode_floats::<f32>(&mut dec()).unwrap_err();
        assert!(matches!(err, MltError::UnsupportedPhysicalEncoding(_)));
    }

    #[test]
    fn decode_floats_reads_raw_little_endian() {
        let bytes = 1.5_f32.to_le_bytes();
        let meta = StreamMeta::new(DATA, IntEncoding::none(ValueKind::Float), 1);
        let stream = RawStream::new(meta, &bytes);
        assert_eq!(stream.decode_floats::<f32>(&mut dec()).unwrap(), vec![1.5]);
    }
}
