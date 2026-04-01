use std::io;
use std::io::Write;

use crate::frames::{EncodedLayer, EncodedUnknown, Unknown};

impl Unknown<'_> {
    /// Write Unknown's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.value)
    }
}

impl<'a> From<Unknown<'a>> for EncodedLayer {
    fn from(u: Unknown<'a>) -> Self {
        Self::Unknown(EncodedUnknown {
            tag: u.tag,
            value: u.value.to_vec(),
        })
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedUnknown {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut tag: u8 = u.arbitrary()?;
        // Tag 1 is the known Tag01 format; producing it as Unknown would break round-trip-ability
        if tag == 1 {
            tag = 0;
        }
        Ok(Self {
            tag,
            value: u.arbitrary()?,
        })
    }
}
