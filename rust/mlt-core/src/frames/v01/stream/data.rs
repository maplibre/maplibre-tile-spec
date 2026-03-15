use std::fmt;
use std::io::Write;

use super::{EncodedStream, EncodedStreamData, RawStream, RawStreamData};
use crate::analyse::{Analyze, StatType};
use crate::utils::formatter::fmt_byte_array;

impl EncodedStream {
    #[must_use]
    pub fn as_borrowed(&self) -> RawStream<'_> {
        RawStream {
            meta: self.meta,
            data: self.data.as_borrowed(),
        }
    }
}

impl RawStreamData<'_> {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RawStreamData::VarInt(d) | RawStreamData::Encoded(d) => d,
        }
    }
}

impl EncodedStreamData {
    #[must_use]
    pub fn as_borrowed(&self) -> RawStreamData<'_> {
        match self {
            Self::VarInt(data) => RawStreamData::VarInt(data),
            Self::Encoded(data) => RawStreamData::Encoded(data),
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            EncodedStreamData::VarInt(d) | EncodedStreamData::Encoded(d) => writer.write_all(d),
        }
    }
}

impl Analyze for RawStreamData<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.as_bytes().collect_statistic(stat)
    }
}

impl fmt::Debug for RawStreamData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawStreamData::VarInt(d) | RawStreamData::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}

impl fmt::Debug for EncodedStreamData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncodedStreamData::VarInt(d) | EncodedStreamData::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}
