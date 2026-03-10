use super::model::{DecodedProperty, EncodedProperty, Property};
use crate::analyse::{Analyze, StatType};
use crate::v01::Stream;

impl Analyze for Property<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(d) => d.collect_statistic(stat),
            Self::Decoded(d) => d.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Encoded(d) => d.for_each_stream(cb),
            Self::Decoded(d) => d.for_each_stream(cb),
        }
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
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
                    cb(s);
                }
            }
            Self::SharedDict(_, shared, children) => {
                for stream in shared.dict_streams() {
                    cb(stream);
                }
                for child in children {
                    child.presence.0.for_each_stream(cb);
                    child.data.for_each_stream(cb);
                }
            }
        }
    }
}

impl Analyze for DecodedProperty<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name().len()
        } else {
            0
        };
        meta + self.collect_value_statistic(stat)
    }
}

impl DecodedProperty<'_> {
    pub(super) fn collect_value_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Bool(v) => v.values.collect_statistic(stat),
            Self::I8(v) => v.values.collect_statistic(stat),
            Self::U8(v) => v.values.collect_statistic(stat),
            Self::I32(v) => v.values.collect_statistic(stat),
            Self::U32(v) => v.values.collect_statistic(stat),
            Self::I64(v) => v.values.collect_statistic(stat),
            Self::U64(v) => v.values.collect_statistic(stat),
            Self::F32(v) => v.values.collect_statistic(stat),
            Self::F64(v) => v.values.collect_statistic(stat),
            Self::Str(v) => v.collect_statistic(stat),
            Self::SharedDict(shared_dict) => shared_dict
                .items
                .iter()
                .map(|item| item.materialize(shared_dict).collect_statistic(stat))
                .sum(),
        }
    }
}
