use crate::analyze::{Analyze, StatType};
use crate::v01::{
    EncodedProperty, ParsedScalar, ParsedSharedDict, ParsedStrings, RawProperty, StreamMeta,
};

impl Analyze for EncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            EncodedProperty::Bool(s)
            | EncodedProperty::I8(s)
            | EncodedProperty::U8(s)
            | EncodedProperty::I32(s)
            | EncodedProperty::U32(s)
            | EncodedProperty::I64(s)
            | EncodedProperty::U64(s)
            | EncodedProperty::F32(s)
            | EncodedProperty::F64(s) => {
                s.presence.0.for_each_stream(cb);
                s.data.for_each_stream(cb);
            }
            EncodedProperty::Str(s) => {
                s.presence.0.for_each_stream(cb);
                for stream in s.encoding.streams() {
                    stream.for_each_stream(cb);
                }
            }
            EncodedProperty::SharedDict(s) => {
                for stream in s.encoding.dict_streams() {
                    stream.for_each_stream(cb);
                }
                for child in &s.children {
                    child.presence.0.for_each_stream(cb);
                    child.data.for_each_stream(cb);
                }
            }
        }
    }
}

impl Analyze for RawProperty<'_> {
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
            | Self::F64(s) => {
                s.presence.0.for_each_stream(cb);
                s.data.for_each_stream(cb);
            }
            Self::Str(s) => {
                s.presence.0.for_each_stream(cb);
                for stream in s.encoding.streams() {
                    stream.for_each_stream(cb);
                }
            }
            Self::SharedDict(s) => {
                for stream in s.encoding.dict_streams() {
                    stream.for_each_stream(cb);
                }
                for child in &s.children {
                    child.presence.0.for_each_stream(cb);
                    child.data.for_each_stream(cb);
                }
            }
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
