use fastpfor::cpp::{Codec32 as _, FastPFor256Codec};

use crate::MltError;

/// Decode FastPFOR-compressed data using the composite codec protocol.
///
/// The Java MLT encoder uses `Composition(FastPFOR(), VariableByte())`, matching
/// the C++ `CompositeCodec<FastPFor<8>, VariableByte>`. The wire format is:
///
/// 1. First u32 = number of compressed u32 words from the primary codec (`FastPFor`)
/// 2. Next N u32 words = primary codec (`FastPFor`) compressed data
/// 3. Remaining u32 words = secondary codec (`VByte`) compressed data
///
/// The compressed bytes are stored as big-endian u32 values by the Java encoder.
pub fn decode_fastpfor_composite(data: &[u8], num_values: usize) -> Result<Vec<u32>, MltError> {
    if num_values == 0 {
        return Ok(vec![]);
    }

    // Convert big-endian bytes to u32 values
    if !data.len().is_multiple_of(4) {
        return Err(MltError::InvalidByteMultiple {
            ctx: "FastPFOR data",
            multiple_of: 4,
            got: data.len(),
        });
    }
    // The Java MLT encoder writes compressed int[] → byte[] in big-endian order.
    // We must convert BE bytes → u32 to reconstruct the original integer values
    // that the Composition(FastPFOR, VariableByte) codec produced.
    let num_words = data.len() / 4;
    let input: Vec<u32> = (0..num_words)
        .map(|i| {
            let o = i * 4;
            u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        })
        .collect();

    if input.is_empty() {
        return Err(MltError::FastPforDecode {
            expected: num_values,
            got: 0,
        });
    }

    // The fastpfor crate's FastPFor256Codec is already a CompositeCodec<FastPFor<8>, VariableByte>.
    // It handles the full Composition protocol internally (FastPFor header + VByte remainder).

    // Over-allocate output buffer — the codec may decode padding beyond num_values.
    let buf_size = num_values + 1024;
    let mut result = vec![0u32; buf_size];

    let decoded = FastPFor256Codec::new().decode32(&input, &mut result)?;

    if decoded.len() < num_values {
        return Err(MltError::FastPforDecode {
            expected: num_values,
            got: decoded.len(),
        });
    }

    result.truncate(num_values);
    Ok(result)
}
