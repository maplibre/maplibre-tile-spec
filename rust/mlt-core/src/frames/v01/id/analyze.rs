use std::fmt::{Debug, Formatter};

use crate::analyse::{Analyze, StatType};
use crate::utils::OptSeqOpt;
use crate::v01::{EncodedId, EncodedIdValue, IdValues, RawId, RawIdValue, RawPresence, StreamMeta};

impl Analyze for EncodedId {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl Analyze for EncodedIdValue {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            EncodedIdValue::Id32(s) | EncodedIdValue::Id64(s) => {
                s.for_each_stream(cb);
            }
        }
    }
}

impl Analyze for RawId<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.value.for_each_stream(cb);
    }
}

impl Analyze for RawIdValue<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Id32(v) | Self::Id64(v) => v.for_each_stream(cb),
        }
    }
}

impl Analyze for RawPresence<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.0.for_each_stream(cb);
    }
}

impl Analyze for IdValues {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.0.collect_statistic(stat)
    }
}

impl Debug for IdValues {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IdValues({:?})", &OptSeqOpt(Some(&self.0)))
    }
}
