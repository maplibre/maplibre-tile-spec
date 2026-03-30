use crate::analyze::{Analyze, StatType};
use crate::v01::property::scalars::scalar_match;
use crate::v01::{
    EncodedPresence, EncodedProperty, EncodedScalar, EncodedSharedDict, EncodedSharedDictItem,
    EncodedStrings, ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedStrings, RawFsstData,
    RawPlainData, RawProperty, RawScalar, RawSharedDict, RawSharedDictEncoding, RawSharedDictItem,
    RawStrings, RawStringsEncoding, Scalar, StreamMeta,
};

impl Analyze for RawPlainData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.lengths.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawFsstData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.symbol_lengths.for_each_stream(cb);
        self.symbol_table.for_each_stream(cb);
        self.lengths.for_each_stream(cb);
        self.corpus.for_each_stream(cb);
    }
}

impl Analyze for RawStringsEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
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

impl Analyze for RawScalar<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawStrings<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.encoding.for_each_stream(cb);
    }
}

impl Analyze for RawSharedDictItem<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawSharedDictEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Plain(plain_data) => plain_data.for_each_stream(cb),
            Self::FsstPlain(fsst_data) => fsst_data.for_each_stream(cb),
        }
    }
}

impl Analyze for RawSharedDict<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.encoding.for_each_stream(cb);
        self.children.for_each_stream(cb);
    }
}

impl Analyze for RawProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Scalar(s) => s.raw_scalar().for_each_stream(cb),
            Self::Str(s) => s.for_each_stream(cb),
            Self::SharedDict(s) => s.for_each_stream(cb),
        }
    }
}

impl Analyze for EncodedPresence {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.0.for_each_stream(cb);
    }
}

impl Analyze for EncodedScalar {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for EncodedStrings {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        for stream in self.encoding.streams() {
            stream.for_each_stream(cb);
        }
    }
}

impl Analyze for EncodedSharedDictItem {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for EncodedSharedDict {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        for stream in self.encoding.dict_streams() {
            stream.for_each_stream(cb);
        }
        self.children.for_each_stream(cb);
    }
}

impl Analyze for EncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Scalar(s) => s.encoded_scalar().for_each_stream(cb),
            Self::Str(s) => s.for_each_stream(cb),
            Self::SharedDict(s) => s.for_each_stream(cb),
        }
    }
}

impl<T: Analyze + Copy + PartialEq + std::fmt::Debug> Analyze for ParsedScalar<'_, T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.values.collect_statistic(stat)
    }
}

impl Analyze for ParsedSharedDict<'_> {
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

impl Analyze for ParsedStrings<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.dense_values().collect_statistic(stat)
    }
}

impl Analyze for ParsedProperty<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Scalar(s) => scalar_match!(s, v => v.collect_statistic(stat)),
            Self::Str(v) => v.collect_statistic(stat),
            Self::SharedDict(v) => v.collect_statistic(stat),
        }
    }
}
