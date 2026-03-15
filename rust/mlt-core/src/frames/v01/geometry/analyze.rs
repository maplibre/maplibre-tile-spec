use crate::analyse::{Analyze, StatType};
use crate::v01::{EncodedGeometry, GeometryType, GeometryValues, RawGeometry, StreamMeta};

impl Analyze for EncodedGeometry {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.meta.for_each_stream(cb);
        self.items.for_each_stream(cb);
    }
}

impl Analyze for RawGeometry<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.meta.for_each_stream(cb);
        self.items.for_each_stream(cb);
    }
}

impl Analyze for GeometryValues {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedDataSize => {
                self.vector_types.collect_statistic(stat)
                    + self.geometry_offsets.collect_statistic(stat)
                    + self.part_offsets.collect_statistic(stat)
                    + self.ring_offsets.collect_statistic(stat)
                    + self.index_buffer.collect_statistic(stat)
                    + self.triangles.collect_statistic(stat)
                    + self.vertices.collect_statistic(stat)
            }
            StatType::DecodedMetaSize => 0,
            StatType::FeatureCount => self.vector_types.len(),
        }
    }
}

impl Analyze for GeometryType {
    fn collect_statistic(&self, _stat: StatType) -> usize {
        size_of::<Self>()
    }
}
