use std::io;
use std::io::Write;

use borrowme::borrowme;

/// Unknown layer data, stored as encoded bytes
#[borrowme]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(
    all(not(test), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
pub struct Unknown<'a> {
    pub tag: u8,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub value: &'a [u8],
}

impl Unknown<'_> {
    /// Write Unknown's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.value)
    }
}
