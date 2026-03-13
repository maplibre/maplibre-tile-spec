use crate::analyse::Analyze;
use crate::v01::{EncodedStream, RawStream, StreamMeta};

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
