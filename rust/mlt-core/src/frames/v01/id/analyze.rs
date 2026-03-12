use std::fmt::{Debug, Formatter};

use crate::analyse::{Analyze, StatType};
use crate::utils::OptSeqOpt;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, StreamMeta};

impl Analyze for crate::v01::OwnedEncodedId {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl Analyze for crate::v01::OwnedEncodedIdValue {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        use crate::v01::OwnedEncodedIdValue;
        match self {
            OwnedEncodedIdValue::Id32(s) | OwnedEncodedIdValue::Id64(s) => {
                s.for_each_stream(cb);
            }
        }
    }
}

impl Analyze for EncodedId<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl Analyze for EncodedIdValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
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
