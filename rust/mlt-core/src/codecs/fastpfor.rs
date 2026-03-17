use fastpfor::rust::{Composition, FastPFOR, Integer as _, VariableByte};

use crate::{Decoder, MltError};

/// Encode a `u32` sequence using `FastPFOR256` (composite codec).
///
/// This is the inverse of `decode_fastpfor_composite`
pub fn encode_fastpfor(values: &[u32]) -> Result<Vec<u8>, MltError> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(feature = "fastpfor-cpp")]
    {
        use fastpfor::cpp::{Codec32 as _, FastPFor256Codec};
        let codec = FastPFor256Codec::new();
        // Over-allocate: FastPFOR may write a header and padding beyond the input length.
        let mut compressed = vec![0u32; values.len() + 1024];
        let out = codec.encode32(values, &mut compressed)?;

        // Convert u32 words to big-endian bytes to match the wire format.
        let mut data = Vec::with_capacity(out.len() * 4);
        for word in out.iter() {
            data.extend_from_slice(&word.to_be_bytes());
        }
        Ok(data)
    }
    #[cfg(all(feature = "fastpfor-rust", not(feature = "fastpfor-cpp")))]
    {
        use fastpfor::rust::{Composition, FastPFOR, Integer as _, VariableByte};

        // Over-allocate: FastPFOR may write a header and padding beyond the input length.
        let mut compressed = vec![0u32; values.len() + 1024];
        let mut comp = Composition::new(FastPFOR::default(), VariableByte::new());
        let mut output_offset = std::io::Cursor::new(0u32);

        comp.compress(
            values,
            u32::try_from(values.len())?,
            &mut std::io::Cursor::new(0u32),
            &mut compressed,
            &mut output_offset,
        )?;

        // FIXME: handle usize casting to be within u32?
        let written = usize::try_from(output_offset.position())?;

        // Convert u32 words to big-endian bytes to match the wire format.
        let mut data = Vec::with_capacity(written * 4);
        for word in &compressed[..written] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        Ok(data)
    }
}

/// Decode `FastPFOR`-compressed data using the composite codec protocol.
///
/// The Java MLT encoder uses `Composition(FastPFOR(), VariableByte())`. The wire format is:
///
/// 1. First u32 = number of compressed u32 words from the primary codec (`FastPFor`)
/// 2. Next N u32 words = primary codec (`FastPFor`) compressed data
/// 3. Remaining u32 words = secondary codec (`VByte`) compressed data
///
/// The compressed bytes are stored as big-endian u32 values by the Java encoder.
pub fn decode_fastpfor_composite(
    data: &[u8],
    num_values: usize,
    dec: &mut Decoder,
) -> Result<Vec<u32>, MltError> {
    if num_values == 0 {
        return Ok(vec![]);
    }

    // Convert big-endian bytes to u32 values
    if !data.len().is_multiple_of(4) {
        return Err(MltError::InvalidFastPforByteLength(data.len()));
    }
    // The Java MLT encoder writes compressed int[] → byte[] in big-endian order.
    // We must convert BE bytes → u32 to reconstruct the original integer values
    // that the Composition(FastPFOR, VariableByte) codec produced.
    let num_words = data.len() / 4;
    dec.consume_items::<u32>(num_words)?;
    let input: Vec<u32> = (0..num_words)
        .map(|i| {
            let o = i * 4;
            u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        })
        .collect();

    if input.is_empty() {
        return Err(MltError::FastPforDecode(num_values, 0));
    }

    // Over-allocate output buffer — the codec may decode padding beyond num_values.
    let buf_size = num_values + 1024;
    let mut result = vec![0u32; buf_size];

    let mut comp = Composition::new(FastPFOR::default(), VariableByte::new());
    let mut output_offset = std::io::Cursor::new(0u32);

    comp.uncompress(
        &input,
        u32::try_from(input.len())?,
        &mut std::io::Cursor::new(0u32),
        &mut result,
        &mut output_offset,
    )?;

    let decoded_len = usize::try_from(output_offset.position())?;

    let Some(adjustment) = decoded_len
        .checked_sub(num_values)
        .and_then(|v| u32::try_from(v).ok())
    else {
        return Err(MltError::FastPforDecode(num_values, decoded_len));
    };

    dec.adjust(adjustment);
    result.truncate(num_values);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::test_helpers::dec;

    proptest! {
        #[test]
        fn test_fastpfor_roundtrip(data: Vec<u32>) {
            let encoded = encode_fastpfor(&data).unwrap();
            let decoded = decode_fastpfor_composite(&encoded, data.len(), &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }

    #[test]
    fn test_encode_fastpfor_empty() {
        let encoded = encode_fastpfor(&[]).unwrap();
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_decode_fastpfor_empty() {
        let decoded = decode_fastpfor_composite(&[], 0, &mut dec()).unwrap();
        assert!(decoded.is_empty());
    }
}
