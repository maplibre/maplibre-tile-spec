use std::ops::{Deref, DerefMut};

use bytes::{Buf, Bytes};

/// A wrapper around `Bytes` that tracks the read offset relative to the original size.
/// This is useful for debugging and decoding purposes, where knowing how many bytes
/// have been consumed is critical (e.g., during varint parsing or stream decoding).
pub struct TrackedBytes(usize, Bytes);

impl TrackedBytes {
    pub fn original_size(&self) -> usize {
        self.0
    }

    pub fn offset(&self) -> usize {
        self.0 - self.1.remaining()
    }
}

impl Deref for TrackedBytes {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl DerefMut for TrackedBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}

impl<T: Into<Bytes>> From<T> for TrackedBytes {
    fn from(data: T) -> Self {
        let bytes = data.into();
        Self(bytes.len(), bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracked_bytes_offset_and_reads() {
        let mut tile: TrackedBytes = [0x10, 0x20, 0x30, 0x40].as_slice().into();

        // Initial offset should be 0
        assert_eq!(tile.offset(), 0);

        // Read a single byte
        let byte = tile.get_u8();
        assert_eq!(byte, 0x10);
        assert_eq!(tile.offset(), 1);

        // Read next two bytes as u16 (0x20, 0x30)
        let u16_val = tile.get_u16();
        assert_eq!(u16_val, 0x2030);
        assert_eq!(tile.offset(), 3);

        // Read final byte
        let last = tile.get_u8();
        assert_eq!(last, 0x40);
        assert_eq!(tile.offset(), 4);

        // No bytes remaining
        assert_eq!(tile.remaining(), 0);
    }
}
