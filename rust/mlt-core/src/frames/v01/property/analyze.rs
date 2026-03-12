use borrowme::Borrow as _;

use crate::analyse::{Analyze, StatType};
use crate::v01::{
    DecodedScalar, DecodedSharedDict, EncodedProperty, EncodedScalar, EncodedSharedDict,
    EncodedSharedDictChild, EncodedStrings, FsstData, OwnedEncodedProperty, OwnedEncodedScalar,
    OwnedEncodedSharedDict, OwnedEncodedStrings, OwnedFsstData, OwnedPlainData,
    OwnedSharedDictEncoding, OwnedStringsEncoding, PlainData, SharedDictEncoding, Stream,
    StringsEncoding,
};

impl Analyze for PlainData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.lengths.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for OwnedPlainData {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for FsstData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.symbol_lengths.for_each_stream(cb);
        self.symbol_table.for_each_stream(cb);
        self.lengths.for_each_stream(cb);
        self.corpus.for_each_stream(cb);
    }
}

impl Analyze for OwnedFsstData {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for StringsEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Plain(plain_data) => plain_data.for_each_stream(cb),
            Self::Dictionary {
                plain_data,
                offsets,
            } => {
                plain_data.for_each_stream(cb);
                offsets.for_each_stream(cb);
            }
            Self::FsstPlain(fsst_data) => fsst_data.for_each_stream(cb),
            Self::FsstDictionary { fsst_data, offsets } => {
                fsst_data.for_each_stream(cb);
                offsets.for_each_stream(cb);
            }
        }
    }
}

impl Analyze for OwnedStringsEncoding {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for SharedDictEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Plain(plain_data) => plain_data.for_each_stream(cb),
            Self::FsstPlain(fsst_data) => fsst_data.for_each_stream(cb),
        }
    }
}

impl Analyze for OwnedSharedDictEncoding {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedScalar<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.presence.0.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for OwnedEncodedScalar {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedStrings<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.presence.0.for_each_stream(cb);
        self.encoding.for_each_stream(cb);
    }
}

impl Analyze for OwnedEncodedStrings {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedSharedDict<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.encoding.for_each_stream(cb);
        self.children.iter().for_each(|v| v.for_each_stream(cb));
    }
}

impl Analyze for EncodedSharedDictChild<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.presence.0.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for OwnedEncodedSharedDict {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for OwnedEncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s) => s.for_each_stream(cb),
            Self::Str(s) => s.for_each_stream(cb),
            Self::SharedDict(s) => s.for_each_stream(cb),
        }
    }
}

impl<T: Analyze + Copy + PartialEq> Analyze for DecodedScalar<'_, T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.values.collect_statistic(stat)
    }
}

impl Analyze for DecodedSharedDict<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.prefix.len()
        } else {
            0
        };
        meta + self
            .items
            .iter()
            .map(|item| item.materialize(self).collect_statistic(stat))
            .sum::<usize>()
    }
}
