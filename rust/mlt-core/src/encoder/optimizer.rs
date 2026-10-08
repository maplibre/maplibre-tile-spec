use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

use bitvec::vec::BitVec;

use crate::decoder::Morton;
use crate::encoder::model::{CurveParams, StagedLayer};
use crate::encoder::property::group_string_properties;
use crate::encoder::sort::{curve_params, sort_order};
use crate::encoder::source::LayerSource;
use crate::encoder::tile::stage_layer;
use crate::encoder::{Codecs, Encoder, EncoderConfig, SortStrategy, encode01};
#[cfg(feature = "unstable-v2")]
use crate::encoder::{WireVersion, encode02};
use crate::tile::TileLayer;
use crate::{MltError, MltResult, PropValueRef};

impl StagedLayer {
    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside `TileLayer::encode`: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    #[hotpath::measure]
    pub fn encode_into(self, enc: Encoder, codecs: &mut Codecs) -> MltResult<Encoder> {
        if self.name.is_empty() {
            return Err(MltError::MissingLayerName);
        }
        #[cfg(feature = "unstable-v2")]
        match enc.config().wire_version() {
            WireVersion::V01 => encode01::encode_into01(self, enc, codecs),
            WireVersion::V02 => encode02::encode_into02(self, enc, codecs),
        }
        #[cfg(not(feature = "unstable-v2"))]
        encode01::encode_into01(self, enc, codecs)
    }
}

/// Scratch capacity above which a [`Codecs`] is dropped instead of kept for the next layer.
const POOLED_CODECS_MAX_BYTES: usize = 16 << 20;

thread_local! {
    static CODECS_POOL: RefCell<Option<Codecs>> = const { RefCell::new(None) };
}

/// A [`Codecs`] borrowed from this thread's pool, which keeps scratch buffers warm between layers.
struct PooledCodecs(Option<Codecs>);

impl PooledCodecs {
    fn take() -> Self {
        let pooled = CODECS_POOL.try_with(|pool| pool.borrow_mut().take());
        Self(Some(pooled.ok().flatten().unwrap_or_default()))
    }
}

impl Deref for PooledCodecs {
    type Target = Codecs;

    fn deref(&self) -> &Codecs {
        self.0.as_ref().expect("codecs are only taken on drop")
    }
}

impl DerefMut for PooledCodecs {
    fn deref_mut(&mut self) -> &mut Codecs {
        self.0.as_mut().expect("codecs are only taken on drop")
    }
}

impl Drop for PooledCodecs {
    fn drop(&mut self) {
        if let Some(codecs) = self.0.take()
            && codecs.scratch_bytes() <= POOLED_CODECS_MAX_BYTES
        {
            let _ = CODECS_POOL.try_with(|pool| *pool.borrow_mut() = Some(codecs));
        }
    }
}

/// Seed the encoder's curve-derived caches so the Hilbert/Morton dictionary
/// builders skip their min/max scan. `Morton::new` returns `Err` when bits > 16;
/// `dict_may_be_beneficial` reads `morton_cache.is_none()` and falls back to
/// a Vec2-only path in that case.
fn seed_curve_caches(enc: &mut Encoder, curve_params: CurveParams) {
    enc.hilbert_cache = Some(curve_params);
    enc.morton_cache = Morton::new(curve_params.bits, curve_params.shift).ok();
}

impl TileLayer {
    /// Encode a [`TileLayer`] to bytes, automatically optimizing all encoding choices.
    ///
    /// This is the primary encoding entry point. It:
    /// 1. Determines which sort strategies to try based on `cfg`
    /// 2. Tries each sort strategy, encoding and measuring the output size
    /// 3. Returns the smallest encoding as a complete layer record (including tag and length prefix)
    ///
    /// All encoding choices - sort order, per-stream integer encodings, string compression,
    /// vertex buffer layout - are selected automatically to minimize output size.
    pub fn encode(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        encode_layer(self, cfg)
    }
}

/// Encode the layer `source` provides, as [`TileLayer::encode`] describes.
#[hotpath::measure]
pub(crate) fn encode_layer(source: &impl LayerSource, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
    if source.name().is_empty() {
        return Err(MltError::MissingLayerName);
    }
    let len = source.feature_count();
    if len == 0 {
        return Ok(Vec::new());
    }

    let mut sort_by = vec![SortStrategy::Unsorted];
    if cfg.attempt_spatial_morton_sort() {
        sort_by.push(SortStrategy::SpatialMorton);
    }
    if cfg.attempt_spatial_hilbert_sort() {
        sort_by.push(SortStrategy::SpatialHilbert);
    }
    // A stable sort of ascending (or absent) IDs is the source order.
    if cfg.attempt_id_sort() && !(0..len).map(|f| source.id(f)).is_sorted() {
        sort_by.push(SortStrategy::Id);
    }

    let stats = analyze(source, cfg.allow_shared_dict())?;
    // Bounds are order-invariant, so this scan is shared across every
    // sort trial and the encoder's Hilbert/Morton dictionary builders.
    let curve_params = curve_params(source);
    let stage = |sort| {
        let order = sort_order(source, sort, curve_params);
        stage_layer(source, &order, &stats, cfg.into())
    };

    // `Encoder::preserve_results` clears caches only on the moved-out
    // archive, so a single seeding here serves every trial that reuses
    // `enc`.
    let mut enc = Encoder::new(cfg);
    seed_curve_caches(&mut enc, curve_params);
    let mut codecs = PooledCodecs::take();
    if let [sort] = sort_by[..] {
        return stage(sort)
            .encode_into(enc, &mut codecs)?
            .into_layer_bytes();
    }
    let mut best: Option<Encoder> = None;
    for &sort in &sort_by {
        enc = stage(sort).encode_into(enc, &mut codecs)?;
        if best
            .as_ref()
            .is_none_or(|best| enc.total_len() < best.total_len())
        {
            best = Some(enc.preserve_results());
        } else {
            // Drop the losing trial's bytes, or the next trial would append to them and overcount its total_len.
            enc.clear_results();
        }
    }
    best.expect("at least two trials ran").into_layer_bytes()
}

/// Row-order-independent presence classification for IDs and properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// No feature has a value for this logical column.
    AllNull,
    /// Every feature has a value for this logical column.
    AllPresent,
    /// Some, but not all, features have a value for this logical column.
    Mixed,
    /// Mixed presence with the same per-feature mask as an earlier property column.
    ///
    /// Only tells the stager the column is nullable, same as [`Self::Mixed`]. Which
    /// columns actually end up sharing one presence bitfield is decided per wire
    /// format at write time, from the staged masks - see `SharedPresence` in
    /// `encoder::encode02`.
    SameAsProp(usize),
}
impl Presence {
    /// Create presence value
    #[must_use]
    pub fn from_bits(bits: &BitVec<u8>, existing: &[(BitVec<u8>, usize)]) -> Self {
        if bits.not_any() {
            Self::AllNull
        } else if bits.all() {
            Self::AllPresent
        } else if let Some((_, idx)) = existing.iter().find(|(v, _)| v == bits) {
            Self::SameAsProp(*idx)
        } else {
            Self::Mixed
        }
    }
}

/// How a property participates in a shared dictionary group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedDictRole {
    /// The property is encoded as a standalone column.
    None,
    /// The property is the first column in this group and emits the shared dictionary with this prefix.
    Owner(String),
    /// The property is emitted by the group owner at this property index.
    Member(usize),
}

/// Row-order-independent facts for a single property column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyStats {
    pub presence: Presence,
    pub stats: PropertyTypedStats,
}

/// Row-order-independent layer facts computed once before sort trials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerStats {
    pub id: Option<PropertyStats>,
    pub properties: Vec<PropertyStats>,
}

/// Row-order-independent value statistics for a property column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PropertyTypedStats {
    /// No present values.
    #[default]
    None,
    Bool,
    Signed {
        min: i64,
        max: i64,
    },
    Unsigned {
        min: u64,
        max: u64,
    },
    F32,
    F64,
    String {
        shared_dict: SharedDictRole,
    },
}

impl PropertyTypedStats {
    #[must_use]
    pub fn values_fit_u32(&self) -> bool {
        match self {
            Self::None | Self::Bool | Self::F32 | Self::F64 | Self::String { .. } => false,
            Self::Signed { min, max } => *min >= 0 && u32::try_from(*max).is_ok(),
            Self::Unsigned { max, .. } => u32::try_from(*max).is_ok(),
        }
    }

    /// Returns `true` if every value fits in an `i32`.
    /// Unlike [`Self::values_fit_u32`] this admits negative values.
    #[must_use]
    pub fn values_fit_i32(&self) -> bool {
        match self {
            Self::None | Self::Bool | Self::F32 | Self::F64 | Self::String { .. } => false,
            Self::Signed { min, max } => i32::try_from(*min).is_ok() && i32::try_from(*max).is_ok(),
            Self::Unsigned { max, .. } => i32::try_from(*max).is_ok(),
        }
    }

    #[must_use]
    pub fn shared_dict(&self) -> SharedDictRole {
        match self {
            Self::String { shared_dict, .. } => shared_dict.clone(),
            Self::None
            | Self::Bool
            | Self::Signed { .. }
            | Self::Unsigned { .. }
            | Self::F32
            | Self::F64 => SharedDictRole::None,
        }
    }

    pub(crate) fn set_shared_dict(&mut self, role: SharedDictRole) {
        match self {
            Self::String { shared_dict, .. } => *shared_dict = role,
            Self::None
            | Self::Bool
            | Self::Signed { .. }
            | Self::Unsigned { .. }
            | Self::F32
            | Self::F64 => debug_assert_eq!(role, SharedDictRole::None),
        }
    }

    pub(crate) fn push(
        &mut self,
        prop: PropValueRef<'_>,
        column_idx: usize,
        property_name: &str,
    ) -> MltResult<()> {
        match prop {
            PropValueRef::Bool(_) => self.merge_same_kind(Self::Bool, column_idx, property_name),
            PropValueRef::I8(v) => self.merge_signed(i64::from(v), column_idx, property_name),
            PropValueRef::U8(v) => self.merge_unsigned(u64::from(v), column_idx, property_name),
            PropValueRef::I32(v) => self.merge_signed(i64::from(v), column_idx, property_name),
            PropValueRef::U32(v) => self.merge_unsigned(u64::from(v), column_idx, property_name),
            PropValueRef::I64(v) => self.merge_signed(v, column_idx, property_name),
            PropValueRef::U64(v) => self.merge_unsigned(v, column_idx, property_name),
            PropValueRef::F32(_) => self.merge_same_kind(Self::F32, column_idx, property_name),
            PropValueRef::F64(_) => self.merge_same_kind(Self::F64, column_idx, property_name),
            PropValueRef::Str(_) => self.merge_string(column_idx, property_name),
        }
    }

    fn merge_signed(
        &mut self,
        value: i64,
        column_idx: usize,
        property_name: &str,
    ) -> MltResult<()> {
        match self {
            Self::None => {
                *self = Self::Signed {
                    min: value,
                    max: value,
                };
            }
            Self::Signed { min, max } => {
                *min = (*min).min(value);
                *max = (*max).max(value);
            }
            Self::Bool | Self::Unsigned { .. } | Self::F32 | Self::F64 | Self::String { .. } => {
                return mixed_prop_err(column_idx, property_name);
            }
        }
        Ok(())
    }

    fn merge_unsigned(
        &mut self,
        value: u64,
        column_idx: usize,
        property_name: &str,
    ) -> MltResult<()> {
        match self {
            Self::None => {
                *self = Self::Unsigned {
                    min: value,
                    max: value,
                };
            }
            Self::Unsigned { min, max } => {
                *min = (*min).min(value);
                *max = (*max).max(value);
            }
            Self::Bool | Self::Signed { .. } | Self::F32 | Self::F64 | Self::String { .. } => {
                return mixed_prop_err(column_idx, property_name);
            }
        }
        Ok(())
    }

    fn merge_string(&mut self, column_idx: usize, property_name: &str) -> MltResult<()> {
        match self {
            Self::None => {
                *self = Self::String {
                    shared_dict: SharedDictRole::None,
                };
            }
            Self::String { .. } => {}
            Self::Bool | Self::Signed { .. } | Self::Unsigned { .. } | Self::F32 | Self::F64 => {
                return mixed_prop_err(column_idx, property_name);
            }
        }
        Ok(())
    }

    fn merge_same_kind(
        &mut self,
        kind: Self,
        column_idx: usize,
        property_name: &str,
    ) -> MltResult<()> {
        match self {
            Self::None => *self = kind,
            Self::Bool if matches!(kind, Self::Bool) => {}
            Self::F32 if matches!(kind, Self::F32) => {}
            Self::F64 if matches!(kind, Self::F64) => {}
            Self::Bool
            | Self::Signed { .. }
            | Self::Unsigned { .. }
            | Self::F32
            | Self::F64
            | Self::String { .. } => return mixed_prop_err(column_idx, property_name),
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "__private"))]
impl TileLayer {
    /// Analyze a [`TileLayer`] and return reusable ID/property facts for the optimizer.
    pub(crate) fn analyze(&self, allow_shared_dict: bool) -> MltResult<LayerStats> {
        analyze(self, allow_shared_dict)
    }
}

/// Analyze the layer `source` provides and return reusable ID/property facts for the optimizer.
#[hotpath::measure]
pub(crate) fn analyze(source: &impl LayerSource, allow_shared_dict: bool) -> MltResult<LayerStats> {
    let mut property_bits = Vec::with_capacity(source.property_count());
    let mut properties = analyze_properties(source, &mut property_bits)?;
    let id = analyze_ids(source, &property_bits);
    if allow_shared_dict {
        group_string_properties(source, &mut properties);
    }
    Ok(LayerStats { id, properties })
}

fn analyze_ids(
    source: &impl LayerSource,
    property_bits: &[(BitVec<u8>, usize)],
) -> Option<PropertyStats> {
    let mut min = u64::MAX;
    let mut max = 0u64;
    let mut bits = BitVec::<u8>::with_capacity(source.feature_count());
    for feature in 0..source.feature_count() {
        if let Some(id) = source.id(feature) {
            min = min.min(id);
            max = max.max(id);
            bits.push(true);
        } else {
            bits.push(false);
        }
    }
    let presence = Presence::from_bits(&bits, property_bits);
    if presence == Presence::AllNull {
        None
    } else {
        Some(PropertyStats {
            presence,
            stats: PropertyTypedStats::Unsigned { min, max },
        })
    }
}

fn analyze_properties(
    source: &impl LayerSource,
    property_bits: &mut Vec<(BitVec<u8>, usize)>,
) -> MltResult<Vec<PropertyStats>> {
    let kinds = source.property_kinds();
    (0..source.property_count())
        .map(|col_idx| -> MltResult<PropertyStats> {
            let mut stats = PropertyTypedStats::default();
            let mut bits = BitVec::<u8>::with_capacity(source.feature_count());
            for feature in 0..source.feature_count() {
                if let Some(prop) = source.property(feature, col_idx) {
                    if Some(&prop.kind()) != kinds.get(col_idx) {
                        return mixed_prop_err(col_idx, source.property_name(col_idx));
                    }
                    stats.push(prop, col_idx, source.property_name(col_idx))?;
                    bits.push(true);
                } else {
                    bits.push(false);
                }
            }

            let presence = Presence::from_bits(&bits, property_bits);
            if presence == Presence::Mixed {
                property_bits.push((bits, col_idx));
            }
            Ok(PropertyStats { presence, stats })
        })
        .collect()
}

#[inline]
fn mixed_prop_err<T>(column_idx: usize, property_name: &str) -> MltResult<T> {
    Err(MltError::MixedPropertyTypes(
        column_idx,
        property_name.to_owned(),
    ))
}
