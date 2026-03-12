use borrowme::Borrow as _;

use crate::analyse::{Analyze, StatType};
use crate::v01::{DecodedOptScalar, DecodedScalar, DecodedSharedDict, EncodedProperty, Stream};

impl Analyze for crate::v01::OwnedEncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Bool(_, data)
            | Self::I8(_, data)
            | Self::U8(_, data)
            | Self::I32(_, data)
            | Self::U32(_, data)
            | Self::I64(_, data)
            | Self::U64(_, data)
            | Self::F32(_, data)
            | Self::F64(_, data) => {
                data.for_each_stream(cb);
            }
            Self::BoolOpt(_, presence, data)
            | Self::I8Opt(_, presence, data)
            | Self::U8Opt(_, presence, data)
            | Self::I32Opt(_, presence, data)
            | Self::U32Opt(_, presence, data)
            | Self::I64Opt(_, presence, data)
            | Self::U64Opt(_, presence, data)
            | Self::F32Opt(_, presence, data)
            | Self::F64Opt(_, presence, data) => {
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

impl<T: Analyze + Copy + PartialEq> Analyze for DecodedOptScalar<'_, T> {
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
