use crate::analyse::{Analyze, StatType};
use crate::v01::{
    DecodedScalar, DecodedSharedDict, EncodedProperty, OwnedEncodedProperty, StreamMeta,
};

impl Analyze for OwnedEncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            OwnedEncodedProperty::Bool(s)
            | OwnedEncodedProperty::I8(s)
            | OwnedEncodedProperty::U8(s)
            | OwnedEncodedProperty::I32(s)
            | OwnedEncodedProperty::U32(s)
            | OwnedEncodedProperty::I64(s)
            | OwnedEncodedProperty::U64(s)
            | OwnedEncodedProperty::F32(s)
            | OwnedEncodedProperty::F64(s) => {
                s.presence.0.for_each_stream(cb);
                s.data.for_each_stream(cb);
            }
            OwnedEncodedProperty::Str(s) => {
                s.presence.0.for_each_stream(cb);
                for stream in s.encoding.streams() {
                    stream.for_each_stream(cb);
                }
            }
            OwnedEncodedProperty::SharedDict(s) => {
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

impl Analyze for EncodedProperty<'_> {
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
