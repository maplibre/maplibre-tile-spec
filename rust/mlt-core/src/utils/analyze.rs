use std::ops::Deref;

use enum_dispatch::enum_dispatch;

use crate::LazyParsed;
use crate::decoder::{ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedStrings, StreamMeta};

/// What to calculate with [`Analyze::collect_statistic`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatType {
    /// Geometry/Feature/id data size in bytes (excludes metadata overhead).
    DecodedDataSize,
    /// Metadata overhead in bytes (stream headers, names, extent, geometry types).
    DecodedMetaSize,
    /// Number of features (geometry entries).
    FeatureCount,
}

/// Trait for estimating various size/count metrics.
#[enum_dispatch]
pub trait Analyze {
    fn collect_statistic(&self, _stat: StatType) -> usize {
        0
    }

    /// Call `cb` with the [`StreamMeta`] of every stream contained in `self`.
    /// Default implementation is a no-op (types that hold no streams).
    fn for_each_stream(&self, _cb: &mut dyn FnMut(StreamMeta)) {}
}

macro_rules! impl_statistics_fixed {
    ($($ty:ty),+) => {
        $(impl Analyze for $ty {
            fn collect_statistic(&self, _stat: StatType) -> usize {
                size_of::<$ty>()
            }
        }
        impl Analyze for &[$ty] {
            fn collect_statistic(&self, _stat: StatType) -> usize {
                size_of::<$ty>() * self.len()
            }
        })+
    };
}

impl_statistics_fixed!(bool, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);

impl Analyze for String {
    fn collect_statistic(&self, _stat: StatType) -> usize {
        self.len()
    }
}

impl<T: Analyze> Analyze for Option<T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.as_ref().map_or(0, |v| v.collect_statistic(stat))
    }
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        if let Some(v) = self {
            v.for_each_stream(cb);
        }
    }
}

impl<T: Analyze> Analyze for [T] {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.iter().map(|v| v.collect_statistic(stat)).sum()
    }
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        for v in self {
            v.for_each_stream(cb);
        }
    }
}

impl<T: Analyze> Analyze for Vec<T> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        self.as_slice().collect_statistic(stat)
    }
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.as_slice().for_each_stream(cb);
    }
}

/// Opt-in marker for blanket `Analyze` delegation via `Deref`.
///
/// A type that implements both `Deref<Target = T>` (with `T: Analyze`) and this
/// marker trait automatically receives an `Analyze` impl that delegates every call
/// to the dereferenced value.
pub(crate) trait AnalyzeViaDeref {}

impl<T: Analyze, R: Deref<Target = T> + AnalyzeViaDeref> Analyze for R {
    fn collect_statistic(&self, stat: StatType) -> usize {
        (**self).collect_statistic(stat)
    }
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        (**self).for_each_stream(cb);
    }
}

impl<Raw: Analyze, Parsed: Analyze> Analyze for LazyParsed<Raw, Parsed> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match self {
            Self::Raw(encoded) => encoded.collect_statistic(stat),
            Self::Parsed(decoded) => decoded.collect_statistic(stat),
            Self::ParsingFailed => 0,
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        match self {
            Self::Raw(encoded) => encoded.for_each_stream(cb),
            Self::Parsed(decoded) => decoded.for_each_stream(cb),
            Self::ParsingFailed => {}
        }
    }
}
