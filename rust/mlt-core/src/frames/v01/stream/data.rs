use std::fmt;
use std::io::Write;

use crate::analyse::Analyze;

/// Representation of the raw stream data, in various physical formats.
macro_rules! stream_data {
    ($($enm:ident : $ty:ident / $owned:ident),+ $(,)?) => {
        #[borrowme::borrowme]
        #[derive(Debug, PartialEq, Clone)]
        pub enum StreamData<'a> {
            $($enm($ty<'a>),)+
        }

        impl Analyze for StreamData<'_> {
            fn collect_statistic(&self, stat: crate::StatType) -> usize {
                match &self {
                    $(StreamData::$enm(d) => d.data.collect_statistic(stat),)+
                }
            }
        }

        $(
            #[borrowme::borrowme]
            #[derive(PartialEq, Clone)]
            pub struct $ty<'a> {
                #[borrowme(borrow_with = Vec::as_slice)]
                pub data: &'a [u8],
            }

            impl<'a> $ty<'a> {
                #[expect(clippy::new_ret_no_self)]
                pub fn new(data: &'a [u8]) -> StreamData<'a> {
                    StreamData::$enm(Self { data })
                }
            }

            impl<'a> fmt::Debug for $ty<'a> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    crate::utils::formatter::fmt_byte_array(self.data, f)
                }
            }

            impl fmt::Debug for $owned {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    crate::utils::formatter::fmt_byte_array(&self.data, f)
                }
            }
        )+
    };
}

stream_data![
    VarInt: DataVarInt / OwnedDataVarInt,
    Encoded: EncodedData / OwnedEncodedData,
];

impl OwnedStreamData {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            OwnedStreamData::VarInt(d) => writer.write_all(&d.data),
            OwnedStreamData::Encoded(d) => writer.write_all(&d.data),
        }
    }
}
