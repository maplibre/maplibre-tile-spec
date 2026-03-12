use crate::analyse::{Analyze, StatType};
use crate::v01::{
    DecodedScalar, DecodedSharedDict, EncodedProperty, OwnedEncodedProperty, StreamMeta,
};

impl Analyze for OwnedEncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            OwnedEncodedProperty::Bool(_, pres, data)
            | OwnedEncodedProperty::I8(_, pres, data)
            | OwnedEncodedProperty::U8(_, pres, data)
            | OwnedEncodedProperty::I32(_, pres, data)
            | OwnedEncodedProperty::U32(_, pres, data)
            | OwnedEncodedProperty::I64(_, pres, data)
            | OwnedEncodedProperty::U64(_, pres, data)
            | OwnedEncodedProperty::F32(_, pres, data)
            | OwnedEncodedProperty::F64(_, pres, data) => {
                pres.0.for_each_stream(cb);
                data.for_each_stream(cb);
            }
            OwnedEncodedProperty::Str(_, pres, enc) => {
                pres.0.for_each_stream(cb);
                for s in enc.streams() {
                    s.for_each_stream(cb);
                }
            }
            OwnedEncodedProperty::SharedDict(_, shared, children) => {
                for s in shared.dict_streams() {
                    s.for_each_stream(cb);
                }
                for child in children {
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
            Self::Bool(_, presence, data)
            | Self::I8(_, presence, data)
            | Self::U8(_, presence, data)
            | Self::I32(_, presence, data)
            | Self::U32(_, presence, data)
            | Self::I64(_, presence, data)
            | Self::U64(_, presence, data)
            | Self::F32(_, presence, data)
            | Self::F64(_, presence, data) => {
                presence.0.for_each_stream(cb);
                data.for_each_stream(cb);
            }
            Self::Str(_, presence, enc) => {
                presence.0.for_each_stream(cb);
                for s in enc.streams() {
                    s.for_each_stream(cb);
                }
            }
            Self::SharedDict(_, shared, children) => {
                for stream in shared.dict_streams() {
                    stream.for_each_stream(cb);
                }
                for child in children {
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
