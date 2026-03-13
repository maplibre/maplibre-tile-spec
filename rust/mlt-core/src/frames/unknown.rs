use std::io;
use std::io::Write;

use crate::frames::{OwnedUnknown, Unknown};

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedUnknown {
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

impl Unknown<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedUnknown {
        OwnedUnknown {
            tag: self.tag,
            value: self.value.to_vec(),
        }
    }

    /// Write Unknown's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.value)
    }
}
