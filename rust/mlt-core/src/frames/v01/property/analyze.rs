use crate::analyse::{Analyze, StatType};
use crate::v01::{DecodedScalar, DecodedSharedDict, EncodedProperty, Stream};

impl Analyze for crate::v01::OwnedEncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.as_borrowed().for_each_stream(cb);
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
