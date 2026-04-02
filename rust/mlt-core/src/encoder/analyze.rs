use crate::analyze::{Analyze, StatType};
use crate::decoder::{ParsedScalar, ParsedSharedDict, ParsedStrings, StreamMeta};
use crate::encoder::{
    EncodedPresence, EncodedProperty, EncodedScalar, EncodedSharedDict, EncodedSharedDictItem,
    EncodedStream, EncodedStrings,
};

impl Analyze for EncodedPresence {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.0.for_each_stream(cb);
    }
}

impl Analyze for EncodedStream {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
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

impl<T: Analyze + Copy + PartialEq> Analyze for ParsedScalar<'_, T> {
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
            self.prefix.len() + self.items.iter().map(|v| v.suffix.len()).sum::<usize>()
        } else {
            0
        };
        meta + self.data.len()
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
