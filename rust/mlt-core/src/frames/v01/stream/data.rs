use std::fmt;
use std::io::Write;

use crate::analyse::{Analyze, StatType};
use crate::utils::formatter::fmt_byte_array;
use crate::v01::{OwnedStreamData, StreamData};

impl StreamData<'_> {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            StreamData::VarInt(d) | StreamData::Encoded(d) => d,
        }
    }
}

impl OwnedStreamData {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            OwnedStreamData::VarInt(d) | OwnedStreamData::Encoded(d) => writer.write_all(d),
        }
    }
}

impl Analyze for StreamData<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.as_bytes().collect_statistic(stat)
    }
}

impl fmt::Debug for StreamData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamData::VarInt(d) | StreamData::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}

impl fmt::Debug for OwnedStreamData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OwnedStreamData::VarInt(d) | OwnedStreamData::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}
