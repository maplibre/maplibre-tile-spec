use crate::analyse::Analyze;
use crate::v01::Stream;

impl Analyze for Stream<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        cb(self);
    }
}
