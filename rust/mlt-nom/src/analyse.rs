/// What to calculate with [`Analyze::decoded`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatType {
    /// Payload data size in bytes (excludes metadata overhead).
    PayloadDataSizeBytes,
    /// Metadata overhead in bytes (stream headers, names, extent, geometry types).
    MetadataOverheadBytes,
    /// Number of features (geometry entries).
    FeatureCount,
}

/// Trait for estimating various size/count metrics.
pub trait Analyze {
    fn decoded(&self, _stat: StatType) -> usize {
        0
    }

    /// Call `cb` for every [`Stream`](crate::v01::Stream) contained in `self`.
    /// Default implementation is a no-op (types that hold no streams).
    fn for_each_stream(&self, _cb: &mut dyn FnMut(&crate::v01::Stream<'_>)) {}
}

macro_rules! impl_statistics_fixed {
    ($($ty:ty),+) => {
        $(impl Analyze for $ty {
            fn decoded(&self, _stat: StatType) -> usize {
                size_of::<$ty>()
            }
        })+
    };
}

impl_statistics_fixed!(bool, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);

impl Analyze for String {
    fn decoded(&self, _stat: StatType) -> usize {
        self.len()
    }
}

impl<T: Analyze> Analyze for Option<T> {
    fn decoded(&self, stat: StatType) -> usize {
        self.as_ref().map_or(0, |v| v.decoded(stat))
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&crate::v01::Stream<'_>)) {
        if let Some(v) = self {
            v.for_each_stream(cb);
        }
    }
}

impl<T: Analyze> Analyze for [T] {
    fn decoded(&self, stat: StatType) -> usize {
        self.iter().map(|v| v.decoded(stat)).sum()
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&crate::v01::Stream<'_>)) {
        for v in self {
            v.for_each_stream(cb);
        }
    }
}

impl<T: Analyze> Analyze for Vec<T> {
    fn decoded(&self, stat: StatType) -> usize {
        self.as_slice().decoded(stat)
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&crate::v01::Stream<'_>)) {
        self.as_slice().for_each_stream(cb);
    }
}
