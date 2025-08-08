use crate::decoder::tracked_bytes::TrackedBytes;
use crate::MltError;
use bytes_varint::*;

pub trait VarintDecodable: Sized {
    fn try_get_varint(input: &mut TrackedBytes) -> Result<Self, MltError>;
}

impl VarintDecodable for u32 {
    fn try_get_varint(input: &mut TrackedBytes) -> Result<Self, MltError> {
        input
            .try_get_u32_varint()
            .map_err(|e| MltError::DecodeError(e.to_string()))
    }
}

impl VarintDecodable for u64 {
    fn try_get_varint(input: &mut TrackedBytes) -> Result<Self, MltError> {
        input
            .try_get_u64_varint()
            .map_err(|e| MltError::DecodeError(e.to_string()))
    }
}

pub fn decode<T: VarintDecodable>(
    input: &mut TrackedBytes,
    num_values: usize,
) -> Result<Vec<T>, MltError> {
    let mut values = Vec::with_capacity(num_values);
    for _ in 0..num_values {
        let val = T::try_get_varint(input)?;
        values.push(val);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_decode_varint_single_u32() {
        let target = u32::MAX;
        let mut buf = Vec::new();
        buf.put_u32_varint(target);
        let mut tracked: TrackedBytes = Bytes::from(buf).into();
        let decoded = decode::<u32>(&mut tracked, 1).unwrap();
        assert_eq!(decoded, vec![target]);
    }

    #[test]
    fn test_decode_varint_multiple_u32() {
        let targets = [1u32, 127, 300];
        let mut buf = Vec::new();
        for &v in &targets {
            buf.put_u32_varint(v);
        }
        let mut tracked: TrackedBytes = Bytes::from(buf).into();
        let decoded = decode::<u32>(&mut tracked, targets.len()).unwrap();
        assert_eq!(decoded, targets);
    }

    #[test]
    fn test_decode_varint_single_u64() {
        let target = u64::MAX;
        let mut buf = Vec::new();
        buf.put_u64_varint(target);
        let mut tracked: TrackedBytes = Bytes::from(buf).into();
        let decoded = decode::<u64>(&mut tracked, 1).unwrap();

        assert_eq!(decoded, vec![target]);
    }

    #[test]
    fn test_decode_varint_multiple_u64() {
        let targets = [1u64, 127, 300];
        let mut buf = Vec::new();
        for &v in &targets {
            buf.put_u64_varint(v);
        }
        let mut tracked: TrackedBytes = Bytes::from(buf).into();
        let decoded = decode::<u64>(&mut tracked, targets.len()).unwrap();
        assert_eq!(decoded, targets);
    }
}
