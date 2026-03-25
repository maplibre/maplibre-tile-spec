use crate::StatType;
use crate::analyze::Analyze;
use crate::v01::{EncodedStream, RawStream, RawStreamData, StreamMeta};

impl Analyze for RawStream<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
    }
}

impl Analyze for EncodedStream {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
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

impl Analyze for RawStreamData<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.as_bytes().collect_statistic(stat)
    }
}
