use crate::decoder::{ParsedScalar, ParsedSharedDict, ParsedStrings, StreamMeta};
use crate::encoder::EncodedStream;
use crate::{Analyze, StatType};

impl Analyze for EncodedStream {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
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
