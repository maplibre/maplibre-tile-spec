use std::fmt;
use std::io::Write;

use super::{OwnedStream, OwnedStreamData, Stream, StreamData};
use crate::analyse::{Analyze, StatType};
use crate::utils::formatter::fmt_byte_array;

impl Stream<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedStream {
        OwnedStream {
            meta: self.meta,
            data: self.data.to_owned(),
        }
    }
}

impl OwnedStream {
    #[must_use]
    pub fn as_borrowed(&self) -> Stream<'_> {
        Stream {
            meta: self.meta,
            data: self.data.as_borrowed(),
        }
    }
}

impl StreamData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedStreamData {
        match self {
            Self::VarInt(data) => OwnedStreamData::VarInt(data.to_vec()),
            Self::Encoded(data) => OwnedStreamData::Encoded(data.to_vec()),
        }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            StreamData::VarInt(d) | StreamData::Encoded(d) => d,
        }
    }
}

impl OwnedStreamData {
    #[must_use]
    pub fn as_borrowed(&self) -> StreamData<'_> {
        match self {
            Self::VarInt(data) => StreamData::VarInt(data),
            Self::Encoded(data) => StreamData::Encoded(data),
        }
    }

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
