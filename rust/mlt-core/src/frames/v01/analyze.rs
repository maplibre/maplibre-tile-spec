use std::fmt::{Debug, Formatter};

use crate::DecodeState;
use crate::analyze::{Analyze, StatType};
use crate::encoder::{
    EncodedPresence, EncodedProperty, EncodedScalar, EncodedSharedDict, EncodedSharedDictItem,
    EncodedStrings, RawSharedDict, RawSharedDictEncoding, RawStrings, RawStringsEncoding,
};
use crate::utils::OptSeqOpt;
use crate::v01::{
    Geometry, GeometryType, GeometryValues, Id, IdValues, Layer01, ParsedScalar, ParsedSharedDict,
    ParsedStrings, Property, RawFsstData, RawGeometry, RawId, RawIdValue, RawPlainData,
    RawPresence, RawProperty, RawScalar, RawSharedDictItem, StreamMeta,
};

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

impl Analyze for RawPlainData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.lengths.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawFsstData<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.symbol_lengths.for_each_stream(cb);
        self.symbol_table.for_each_stream(cb);
        self.lengths.for_each_stream(cb);
        self.corpus.for_each_stream(cb);
    }
}

impl Analyze for RawStringsEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Plain(plain_data) => plain_data.for_each_stream(cb),
            Self::Dictionary {
                plain_data,
                offsets,
            } => {
                plain_data.for_each_stream(cb);
                offsets.for_each_stream(cb);
            }
            Self::FsstPlain(fsst_data) => fsst_data.for_each_stream(cb),
            Self::FsstDictionary { fsst_data, offsets } => {
                fsst_data.for_each_stream(cb);
                offsets.for_each_stream(cb);
            }
        }
    }
}

impl Analyze for RawScalar<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawStrings<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.encoding.for_each_stream(cb);
    }
}

impl Analyze for RawSharedDictItem<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for RawSharedDictEncoding<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Plain(plain_data) => plain_data.for_each_stream(cb),
            Self::FsstPlain(fsst_data) => fsst_data.for_each_stream(cb),
        }
    }
}

impl Analyze for RawSharedDict<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.encoding.for_each_stream(cb);
        self.children.for_each_stream(cb);
    }
}

impl Analyze for RawProperty<'_> {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s) => s.for_each_stream(cb),
            Self::Str(s) => s.for_each_stream(cb),
            Self::SharedDict(s) => s.for_each_stream(cb),
        }
    }
}

impl Analyze for EncodedPresence {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.0.for_each_stream(cb);
    }
}

impl Analyze for EncodedScalar {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for EncodedStrings {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        for stream in self.encoding.streams() {
            stream.for_each_stream(cb);
        }
    }
}

impl Analyze for EncodedSharedDictItem {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.presence.for_each_stream(cb);
        self.data.for_each_stream(cb);
    }
}

impl Analyze for EncodedSharedDict {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        for stream in self.encoding.dict_streams() {
            stream.for_each_stream(cb);
        }
        self.children.for_each_stream(cb);
    }
}

impl Analyze for EncodedProperty {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Bool(s)
            | Self::I8(s)
            | Self::U8(s)
            | Self::I32(s)
            | Self::U32(s)
            | Self::I64(s)
            | Self::U64(s)
            | Self::F32(s)
            | Self::F64(s) => s.for_each_stream(cb),
            Self::Str(s) => s.for_each_stream(cb),
            Self::SharedDict(s) => s.for_each_stream(cb),
        }
    }
}

impl<T: Analyze + Copy + PartialEq> Analyze for ParsedScalar<'_, T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.values.collect_statistic(stat)
    }
}

impl Analyze for ParsedSharedDict<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.prefix.len()
        } else {
            0
        };
        meta + self
            .items
            .iter()
            .map(|item| item.materialize(self).collect_statistic(stat))
            .sum::<usize>()
    }
}

impl Analyze for ParsedStrings<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        let meta = if stat == StatType::DecodedMetaSize {
            self.name.len()
        } else {
            0
        };
        meta + self.dense_values().collect_statistic(stat)
    }
}
