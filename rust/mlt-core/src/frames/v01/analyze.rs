use crate::v01::{Geometry, Id, Layer01, Property, StreamMeta};
use crate::{Analyze, DecodeState, StatType};

impl<'a, S: DecodeState> Analyze for Layer01<'a, S>
where
    Option<Id<'a, S>>: Analyze,
    Geometry<'a, S>: Analyze,
    Vec<Property<'a, S>>: Analyze,
{
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedMetaSize => self.name.len() + size_of::<u32>(),
            StatType::DecodedDataSize => {
                self.id.collect_statistic(stat)
                    + self.geometry.collect_statistic(stat)
                    + self.properties.collect_statistic(stat)
            }
            StatType::FeatureCount => self.geometry.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.id.for_each_stream(cb);
        self.geometry.for_each_stream(cb);
        self.properties.for_each_stream(cb);
    }
}
