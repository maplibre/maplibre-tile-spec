use std::fmt;
use std::io::Write;

use crate::decoder::StreamMeta;
use crate::utils::formatter::fmt_byte_array;

/// Owned variant of [`RawStream`](crate::decoder::RawStream).
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: EncodedStreamData,
}

/// Wire-ready encoded plain-text string data (lengths stream + raw bytes stream).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPlainData {
    pub lengths: EncodedStream,
    pub data: EncodedStream,
}

impl EncodedPlainData {
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        vec![&self.lengths, &self.data]
    }
}

/// Wire-ready encoded FSST-compressed string data (4 streams).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedFsstData {
    pub symbol_lengths: EncodedStream,
    pub symbol_table: EncodedStream,
    pub lengths: EncodedStream,
    pub corpus: EncodedStream,
}

impl EncodedFsstData {
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        vec![
            &self.symbol_lengths,
            &self.symbol_table,
            &self.lengths,
            &self.corpus,
        ]
    }
}

/// Wire-ready encoded string column encoding (owns byte buffers for all content streams).
#[derive(Debug, Clone, PartialEq)]
pub enum EncodedStringsEncoding {
    Plain(EncodedPlainData),
    Dictionary {
        plain_data: EncodedPlainData,
        offsets: EncodedStream,
    },
    FsstPlain(EncodedFsstData),
    FsstDictionary {
        fsst_data: EncodedFsstData,
        offsets: EncodedStream,
    },
}

impl EncodedStringsEncoding {
    /// Content streams in wire order.
    #[must_use]
    pub fn streams(&self) -> Vec<&EncodedStream> {
        match self {
            Self::Plain(plain_data) => plain_data.streams(),
            Self::Dictionary {
                plain_data,
                offsets,
            } => {
                let mut streams = plain_data.streams();
                streams.insert(1, offsets);
                streams
            }
            Self::FsstPlain(fsst_data) => fsst_data.streams(),
            Self::FsstDictionary { fsst_data, offsets } => {
                let mut streams = fsst_data.streams();
                streams.push(offsets);
                streams
            }
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum EncodedStreamData {
    VarInt(Vec<u8>),
    Encoded(Vec<u8>),
}
impl EncodedStreamData {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Self::VarInt(d) | Self::Encoded(d) => writer.write_all(d),
        }
    }
}
impl fmt::Debug for EncodedStreamData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarInt(d) | Self::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}
