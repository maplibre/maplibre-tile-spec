use crate::analyse::Analyze;
use crate::v01::{OwnedStream, Stream, StreamMeta};

impl Analyze for Stream<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
    }
}

impl Analyze for OwnedStream {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        cb(self.meta);
    }
}
