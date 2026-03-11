use std::fmt::{Debug, Formatter};

use crate::analyse::{Analyze, StatType};
use crate::utils::OptSeqOpt;
use crate::v01::{DecodedId, EncodedId, EncodedIdValue, Id, Stream};

impl Analyze for Id<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Encoded(Some(d)) => d.collect_statistic(stat),
            Self::Decoded(Some(d)) => d.collect_statistic(stat),
            Self::Encoded(None) | Self::Decoded(None) => 0,
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        match self {
            Self::Encoded(Some(d)) => d.for_each_stream(cb),
            Self::Decoded(Some(d)) => d.for_each_stream(cb),
            Self::Encoded(None) | Self::Decoded(None) => {}
        }
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
