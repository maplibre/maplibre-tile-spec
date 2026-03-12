use std::fmt::{Debug, Formatter};

use borrowme::Borrow as _;

use crate::analyse::{Analyze, StatType};
use crate::utils::OptSeqOpt;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, Stream};

impl Analyze for crate::v01::OwnedEncodedId {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.borrow().for_each_stream(cb);
    }
}

impl Analyze for EncodedId<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.presence.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl Analyze for EncodedIdValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Id32(v) | Self::Id64(v) => v.for_each_stream(cb),
        }
    }
}

impl Analyze for DecodedId {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.0.collect_statistic(stat)
    }
}

impl Debug for DecodedId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodedId({:?})", &OptSeqOpt(Some(&self.0)))
    }
}
