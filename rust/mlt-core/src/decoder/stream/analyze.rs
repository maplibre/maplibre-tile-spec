use crate::decoder::{RawStream, StreamMeta};
use crate::{Analyze, StatType};

impl Analyze for RawStream<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
    }

    fn collect_statistic(&self, stat: StatType) -> usize {
        self.data.collect_statistic(stat)
    }
}

impl Analyze for StreamMeta {
    fn collect_statistic(&self, stat: StatType) -> usize {
        if stat == StatType::DecodedMetaSize {
            size_of::<Self>()
        } else {
            0
        }
    }
}
