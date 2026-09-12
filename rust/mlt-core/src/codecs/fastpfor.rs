#[cfg(feature = "unstable-v2")]
use fastpfor::FastPFor128;
use fastpfor::{AnyLenCodec as _, FastPFor256};
use usize_cast::IntoUsize as _;

use crate::decoder::FastPForKind;
use crate::{Decoder, MltError, MltResult};

/// Decode `FastPFOR`-compressed data using the composite codec protocol.
///
/// The Java MLT encoder uses `Composition(FastPFOR(), VariableByte())`, matching
/// the C++ `CompositeCodec<FastPFor<8>, VariableByte>`. The wire format is:
///
/// 1. First u32 = number of compressed u32 words from the primary codec (`FastPFor`)
/// 2. Next N u32 words = primary codec (`FastPFor`) compressed data
/// 3. Remaining u32 words = secondary codec (`VByte`) compressed data
///
/// [`FastPForKind`] fixes both the block size and the order the words are stored in.
pub fn decode_fastpfor(
    data: &[u8],
    num_values: u32,
    kind: FastPForKind,
    dec: &mut Decoder,
) -> MltResult<Vec<u32>> {
    if num_values == 0 {
        // FIXME: eventually there should not be a header anywhere at all
        return if data.is_empty() {
            Ok(vec![])
        } else {
            Err(MltError::InvalidFastPforByteLength(0))
        };
    }

    let (words, rest) = data.as_chunks::<4>();
    if !rest.is_empty() {
        return Err(MltError::InvalidFastPforByteLength(data.len()));
    }
    dec.consume_items::<u32>(words.len())?;

    // Both branches are per stream, so neither word order nor codec is re-decided per word.
    let mut result = Vec::new();
    match kind {
        FastPForKind::Block256Be => {
            let input: Vec<u32> = words.iter().copied().map(u32::from_be_bytes).collect();
            FastPFor256::default().decode(&input, &mut result, Some(num_values))?;
        }
        #[cfg(feature = "unstable-v2")]
        FastPForKind::Block128Le => {
            let input: Vec<u32> = words.iter().copied().map(u32::from_le_bytes).collect();
            FastPFor128::default().decode(&input, &mut result, Some(num_values))?;
        }
    }

    let Some(adjustment) = result
        .len()
        .checked_sub(num_values.into_usize())
        .and_then(|v| u32::try_from(v).ok())
    else {
        return Err(MltError::FastPforDecode(num_values, result.len()));
    };

    dec.adjust(adjustment);
    result.truncate(num_values.into_usize());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::test_helpers::dec;

    fn encode(kind: FastPForKind, data: &[u32]) -> Vec<u8> {
        let mut words = Vec::new();
        match kind {
            FastPForKind::Block256Be => {
                FastPFor256::default().encode(data, &mut words).unwrap();
                words.iter().flat_map(|w| w.to_be_bytes()).collect()
            }
            #[cfg(feature = "unstable-v2")]
            FastPForKind::Block128Le => {
                FastPFor128::default().encode(data, &mut words).unwrap();
                words.iter().flat_map(|w| w.to_le_bytes()).collect()
            }
        }
    }

    proptest! {
        #[test]
        fn test_fastpfor_roundtrip(data: Vec<u32>, block128: bool) {
            // FastPFor produces a non-empty output (VByte header) even for empty input,
            // but decode_fastpfor requires zero bytes when num_values == 0 - consistent
            // with how PhysicalEncoder guards `if !values.is_empty()`.
            prop_assume!(!data.is_empty());
            #[cfg(feature = "unstable-v2")]
            let kind = if block128 { FastPForKind::Block128Le } else { FastPForKind::Block256Be };
            #[cfg(not(feature = "unstable-v2"))]
            let kind = { let _ = block128; FastPForKind::Block256Be };
            let encoded = encode(kind, &data);
            let decoded = decode_fastpfor(&encoded, data.len().try_into().unwrap(), kind, &mut dec()).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }

    #[rstest]
    #[case(FastPForKind::Block256Be)]
    #[cfg_attr(feature = "unstable-v2", case(FastPForKind::Block128Le))]
    fn test_decode_fastpfor_empty(#[case] kind: FastPForKind) {
        let decoded = decode_fastpfor(&[], 0, kind, &mut dec()).unwrap();
        assert_eq!(decoded, [] as [u32; 0]);
    }

    #[cfg(feature = "unstable-v2")]
    #[rstest]
    fn test_word_order_is_not_interchangeable(
        #[values(FastPForKind::Block256Be, FastPForKind::Block128Le)] kind: FastPForKind,
    ) {
        let other = match kind {
            FastPForKind::Block256Be => FastPForKind::Block128Le,
            FastPForKind::Block128Le => FastPForKind::Block256Be,
        };
        let data: Vec<u32> = (0..500).map(|i| i * 7 + 3).collect();
        let encoded = encode(kind, &data);
        let num_values = u32::try_from(data.len()).unwrap();
        assert_ne!(
            decode_fastpfor(&encoded, num_values, other, &mut dec()).ok(),
            Some(data)
        );
    }
}
