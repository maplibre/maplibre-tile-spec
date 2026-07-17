use std::borrow::Cow;
use std::mem;

use bitvec::prelude::{BitSlice, BitVec, Lsb0};
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::codecs::bytes::{PhysicalWord, decode_bytes_to_bools, decode_bytes_to_words};
use crate::codecs::rle::decode_byte_rle;
use crate::codecs::varint::{parse_varint_vec, parse_varint_vec_all};
use crate::decoder::{LogicalEncoding, LogicalValue, PhysicalEncoding, RawStream, RleMeta};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::{Decoder, MltError, MltResult};

impl<'a> RawStream<'a> {
    /// Decode a presence/nullability stream into a packed bitvector.
    ///
    /// Borrows directly from tile bytes (zero-copy) when both logical and physical
    /// encodings are `None`; otherwise decompresses byte-RLE into an owned `BitVec`.
    /// The result is always truncated to exactly `num_values` bits.
    pub(crate) fn decode_bitvec(self, dec: &mut Decoder) -> MltResult<Cow<'a, BitSlice<u8, Lsb0>>> {
        let num_values = self.meta.num_values.into_usize();
        if self.meta.encoding.physical == PhysicalEncoding::VarInt {
            return Err(MltError::NotImplemented("varint presence decoding"));
        }
        if self.meta.encoding.logical == LogicalEncoding::None
            && self.meta.encoding.physical == PhysicalEncoding::None
        {
            // Zero-copy: raw tile bytes are the packed bitvector.
            let num_bytes = num_values.div_ceil(8);
            fail_if_invalid_stream_size(self.data.len(), num_bytes)?;
            Ok(Cow::Borrowed(&self.data.view_bits::<Lsb0>()[..num_values]))
        } else {
            let num_bytes = num_values.div_ceil(8);
            let bytes = decode_byte_rle(self.data, num_bytes, dec)?;
            let mut bvec = BitVec::<u8, Lsb0>::from_vec(bytes);
            bvec.truncate(num_values);
            Ok(Cow::Owned(bvec))
        }
    }

    /// Decode a boolean data stream into `Vec<bool>`, charging `dec`.
    ///
    /// Both wire formats store one bit per value in an LSB-first packed bitmap;
    /// they differ only in how that bitmap is framed, which the logical encoding
    /// distinguishes:
    /// - tag `0x01` (`logical = Rle`): byte-RLE compressed bitmap.
    /// - tag `0x02` (`logical = None`): raw bitmap, no compression — the same
    ///   representation as a v2 presence bitfield.
    pub fn decode_bools(self, dec: &mut Decoder) -> MltResult<Vec<bool>> {
        let num_values = self.meta.num_values.into_usize();
        match self.meta.encoding.logical {
            LogicalEncoding::Rle(_) => {
                let bytes = decode_byte_rle(self.data, num_values.div_ceil(8), dec)?;
                decode_bytes_to_bools(&bytes, num_values, dec)
            }
            LogicalEncoding::None if self.meta.encoding.physical == PhysicalEncoding::None => {
                decode_bytes_to_bools(self.data, num_values, dec)
            }
            _ => Err(MltError::NotImplemented("unsupported bool stream encoding")),
        }
    }

    /// Decode an integer stream via its 32-bit physical type `W`, then narrow each
    /// value to the 8-bit output `N`, erroring if any value is out of range.
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
    /// Fast path: when there is no logical transform and `T` is unsigned, the
    /// physical words *are* the output and are decoded straight into a fresh
    /// `Vec`, skipping the scratch-buffer round-trip. Otherwise the physical
    /// decode uses the decoder's reusable scratch buffer and the logical
    /// transform (zigzag / delta / RLE / Morton / …) produces the output. Signed
    /// types always take the transform path (zigzag is required even for `None`).
    pub fn decode_ints<T: DecodeInt>(self, dec: &mut Decoder) -> MltResult<Vec<T>> {
        let meta = self.meta;
        if meta.encoding.logical == LogicalEncoding::None
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

    /// Decode a stream of floating-point values (`f32` / `f64`) from raw little-endian
    /// bytes, charging `dec`. Varint physical encoding is not supported for floats.
    pub fn decode_floats<T>(self, dec: &mut Decoder) -> MltResult<Vec<T>>
    where
        T: num_traits::FromBytes,
        for<'b> <T as num_traits::FromBytes>::Bytes: TryFrom<&'b [u8]>,
    {
        if self.meta.encoding.physical == PhysicalEncoding::VarInt {
            return Err(MltError::NotImplemented("varint float decoding"));
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
    /// `buf` is cleared and filled with the decoded words. The caller owns the
    /// buffer and is responsible for deciding whether it constitutes a final
    /// persistent allocation (and therefore should be charged to a [`Decoder`]).
    ///
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
            PhysicalEncoding::FastPFor256 => {
                *buf = T::decode_fastpfor(self.data, self.meta.num_values, dec)?;
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
/// Maps the output type to the physical word width it decodes from, the decoder
/// scratch buffer to reuse for that width, and the logical-decode entry point.
/// Decoder-side mirror of the encoder's `LogicalIntStreamKind`.
pub trait DecodeInt: Sized {
    /// Physical word width the stream is decoded into before the logical transform.
    type Physical: PhysicalWord;

    /// The reusable scratch buffer the decoder holds for this physical width.
    fn scratch(dec: &mut Decoder) -> &mut Vec<Self::Physical>;

    /// Apply the logical transform (zigzag / delta / RLE / Morton / …) to the
    /// physically decoded words, producing the output values.
    fn logical_decode(
        lv: LogicalValue,
        data: &[Self::Physical],
        dec: &mut Decoder,
    ) -> MltResult<Vec<Self>>;

    /// Fast path for [`LogicalEncoding::None`]: for unsigned types the physical
    /// words are already the output, so decode straight into a fresh `Vec` and
    /// skip the scratch round-trip. Signed types return `None` — a zigzag
    /// transform is always required, so they fall through to the general path.
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
    fn scans_to_end(self) -> bool {
        matches!(
            self,
            Self::Rle(RleMeta::Interleaved { .. }) | Self::DeltaRle(RleMeta::Interleaved { .. })
        )
    }
}
