use fastpfor::{AnyLenCodec as _, FastPFor128};

use crate::utils::AsUsize as _;
use crate::{Decoder, MltError, MltResult};

/// Encode a `u32` sequence using `FastPFOR256` (composite codec).
///
/// This is the inverse of `decode_fastpfor_composite`
pub fn encode_fastpfor(values: &[u32]) -> MltResult<Vec<u8>> {
    if values.is_empty() {
        // FIXME: eventually there should not be a header anywhere at all
        return Ok(Vec::new());
    }

    let mut compressed = Vec::new();
    FastPFor128::default().encode(values, &mut compressed)?;

    // Convert u32 words to big-endian bytes to match the wire format.
    let mut data = Vec::with_capacity(compressed.len() * 4);
    for word in &compressed {
        data.extend_from_slice(&word.to_be_bytes());
    }
    Ok(data)
}

/// Decode `FastPFOR`-compressed data using the composite codec protocol.
///
/// The Java MLT encoder uses `Composition(FastPFOR(), VariableByte())`, matching
/// the C++ `CompositeCodec<FastPFor<8>, VariableByte>`. The wire format is:
///
/// 1. First u32 = number of compressed u32 words from the primary codec (`FastPFor`)
/// 2. Next N u32 words = primary codec (`FastPFor`) compressed data
/// 3. Remaining u32 words = secondary codec (`VByte`) compressed data
///
/// The compressed bytes are stored as big-endian u32 values by the Java encoder.
pub fn decode_fastpfor(data: &[u8], num_values: u32, dec: &mut Decoder) -> MltResult<Vec<u32>> {
    if num_values == 0 {
        // FIXME: eventually there should not be a header anywhere at all
        return if data.is_empty() {
            Ok(vec![])
        } else {
            Err(MltError::InvalidFastPforByteLength(0))
        };
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

    let mut result = Vec::new();
    FastPFor128::default().decode(&input, &mut result, Some(num_values))?;

    let Some(adjustment) = result
        .len()
        .checked_sub(num_values.as_usize())
        .and_then(|v| u32::try_from(v).ok())
    else {
        return Err(MltError::FastPforDecode(num_values, result.len()));
    };

    dec.adjust(adjustment);
    result.truncate(num_values.as_usize());

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
            let decoded = decode_fastpfor(&encoded, data.len().try_into().unwrap(), &mut dec()).unwrap();
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
        let decoded = decode_fastpfor(&[], 0, &mut dec()).unwrap();
        assert!(decoded.is_empty());
    }
}
