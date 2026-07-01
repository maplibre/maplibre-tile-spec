use bitvec::vec::BitVec;

use crate::decoder::{Morton, PropKind, TileLayer};
use crate::encoder::model::{CurveParams, StagedLayer};
use crate::encoder::property::encode::write_properties;
use crate::encoder::{
    Codecs, Encoder, EncoderConfig, SortStrategy, StagedId, spatial_sort_likely_to_help,
};
use crate::{MltError, MltResult, PropValue};

impl StagedLayer {
    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside `TileLayer::encode`: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    #[hotpath::measure]
    pub fn encode_into(self, mut enc: Encoder, codecs: &mut Codecs) -> MltResult<Encoder> {
        if self.name.is_empty() {
            return Err(MltError::MissingLayerName);
        }
        let column_count = usize::from(!matches!(self.id, StagedId::None))
            + 1 // geometry
            + self.properties.len();

        let StagedLayer {
            name,
            extent,
            id,
            geometry,
            properties,
        } = self;

        id.write_to(&mut enc, codecs)?;
        geometry.write_to(&mut enc, codecs)?;
        write_properties(&properties, &mut enc, codecs)?;
        enc.write_header(&name, extent.get(), column_count)?;

        Ok(enc)
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

/// Feature-count threshold above which the spatial trial is subject to the
/// bounding-box pruning heuristic.
const SORT_TRIAL_THRESHOLD: usize = 512;

impl TileLayer {
    /// Encode a [`TileLayer`] to bytes, automatically optimizing all encoding choices.
    ///
    /// This is the primary encoding entry point. It:
    /// 1. Determines which sort strategies to try based on `cfg`
    /// 2. Tries each sort strategy, encoding and measuring the output size
    /// 3. Returns the smallest encoding as a complete layer record (including tag and length prefix)
    ///
    /// All encoding choices — sort order, per-stream integer encodings, string compression,
    /// vertex buffer layout — are selected automatically to minimize output size.
    #[hotpath::measure]
    pub fn encode(self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        if self.name().is_empty() {
            return Err(MltError::MissingLayerName);
        }
        if self.features().is_empty() {
            return Ok(Vec::new());
        }

        let mut sort_by = vec![SortStrategy::Unsorted];
        let try_spatial_sort =
            cfg.attempt_spatial_morton_sort() || cfg.attempt_spatial_hilbert_sort();
        if try_spatial_sort
            && (self.feature_count() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(&self))
        {
            if cfg.attempt_spatial_morton_sort() {
                sort_by.push(SortStrategy::SpatialMorton);
            }
            if cfg.attempt_spatial_hilbert_sort() {
                sort_by.push(SortStrategy::SpatialHilbert);
            }
        }
        if cfg.attempt_id_sort() {
            sort_by.push(SortStrategy::Id);
        }

        let stats = self.analyze(cfg.allow_shared_dict())?;
        // Bounds are order-invariant, so this scan is shared across every
        // sort trial and the encoder's Hilbert/Morton dictionary builders.
        let curve_params = self.curve_params();

        // `Encoder::preserve_results` clears caches only on the moved-out
        // archive, so a single seeding here serves every trial that reuses
        // `enc`.
        let mut enc = Encoder::new(cfg);
        seed_curve_caches(&mut enc, curve_params);

        let (last, init) = sort_by.split_last().expect("at least one strategy");
        if init.is_empty() {
            let mut codecs = Codecs::default();
            StagedLayer::from_tile(self, *last, &stats, cfg.tessellate(), curve_params)
                .encode_into(enc, &mut codecs)?
        } else {
            let mut codecs = Codecs::default();
            enc = {
                let first = init[0];
                StagedLayer::from_tile(self.clone(), first, &stats, cfg.tessellate(), curve_params)
                    .encode_into(enc, &mut codecs)?
            };
            let mut best = enc.preserve_results();
            // Clone for all-but-last strategies
            for &sort in &init[1..] {
                let layer = StagedLayer::from_tile(
                    self.clone(),
                    sort,
                    &stats,
                    cfg.tessellate(),
                    curve_params,
                );
                enc = layer.encode_into(enc, &mut codecs)?;
                if enc.total_len() < best.total_len() {
                    best = enc.preserve_results();
                } else {
                    // The losing trial's bytes must be dropped so the next trial
                    // encodes into a clean buffer. `preserve_results` only empties
                    // `enc` when a trial wins; without this, the next `encode_into`
                    // would append to the loser's bytes and overcount its
                    // `total_len`, so later trials could never win.
                    enc.clear_results();
                }
            }
            // Last strategy: consume self, no clone
            let layer = StagedLayer::from_tile(self, *last, &stats, cfg.tessellate(), curve_params);
            enc = layer.encode_into(enc, &mut codecs)?;
            if enc.total_len() < best.total_len() {
                best = enc.preserve_results();
            }
            best
        }
        .into_layer_bytes()
    }
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
    ///
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
            _ => SharedDictRole::None,
        }
    }

    pub(crate) fn set_shared_dict(&mut self, role: SharedDictRole) {
        match self {
            Self::String { shared_dict, .. } => *shared_dict = role,
            _ => debug_assert_eq!(role, SharedDictRole::None),
        }
    }

    pub(crate) fn push(
        &mut self,
        prop: &PropValue,
        column_idx: usize,
        property_name: &str,
    ) -> MltResult<bool> {
        match prop {
            PropValue::Bool(Some(_)) => {
                self.merge_same_kind(Self::Bool, column_idx, property_name)?;
            }
            PropValue::I8(Some(v)) => {
                self.merge_signed(i64::from(*v), column_idx, property_name)?;
            }
            PropValue::U8(Some(v)) => {
                self.merge_unsigned(u64::from(*v), column_idx, property_name)?;
            }
            PropValue::I32(Some(v)) => {
                self.merge_signed(i64::from(*v), column_idx, property_name)?;
            }
            PropValue::U32(Some(v)) => {
                self.merge_unsigned(u64::from(*v), column_idx, property_name)?;
            }
            PropValue::I64(Some(v)) => self.merge_signed(*v, column_idx, property_name)?,
            PropValue::U64(Some(v)) => self.merge_unsigned(*v, column_idx, property_name)?,
            PropValue::F32(Some(_)) => {
                self.merge_same_kind(Self::F32, column_idx, property_name)?;
            }
            PropValue::F64(Some(_)) => {
                self.merge_same_kind(Self::F64, column_idx, property_name)?;
            }
            PropValue::Str(Some(_)) => self.merge_string(column_idx, property_name)?,
            _ => return Ok(false),
        }
        Ok(true)
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
            _ => return mixed_prop_err(column_idx, property_name),
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
            _ => return mixed_prop_err(column_idx, property_name),
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
            _ => return mixed_prop_err(column_idx, property_name),
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
            _ => return mixed_prop_err(column_idx, property_name),
        }
        Ok(())
    }
}

impl TileLayer {
    /// Analyze a [`TileLayer`] and return reusable ID/property facts for the optimizer.
    #[hotpath::measure]
    pub(crate) fn analyze(&self, allow_shared_dict: bool) -> MltResult<LayerStats> {
        let mut property_bits = Vec::with_capacity(self.property_names().len());
        let mut properties = self.analyze_properties(&mut property_bits)?;
        let id = self.analyze_ids(&property_bits);
        if allow_shared_dict {
            self.group_string_properties(&mut properties);
        }
        Ok(LayerStats { id, properties })
    }

    fn analyze_ids(&self, property_bits: &[(BitVec<u8>, usize)]) -> Option<PropertyStats> {
        let mut min = u64::MAX;
        let mut max = 0u64;
        let mut bits = BitVec::<u8>::with_capacity(self.feature_count());
        for feature in self.features() {
            if let Some(id) = feature.id() {
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
        &self,
        property_bits: &mut Vec<(BitVec<u8>, usize)>,
    ) -> MltResult<Vec<PropertyStats>> {
        self.property_names()
            .iter()
            .enumerate()
            .map(|(col_idx, name)| -> MltResult<PropertyStats> {
                let mut kind = None;
                let mut stats = PropertyTypedStats::default();
                let mut bits = BitVec::<u8>::with_capacity(self.feature_count());
                for feature in self.features() {
                    let prop = feature.properties().get(col_idx);
                    if let Some(prop_kind) = prop.map(PropKind::from) {
                        match kind {
                            Some(kind) if kind != prop_kind => {
                                return mixed_prop_err(col_idx, name.as_str());
                            }
                            None => kind = Some(prop_kind),
                            _ => {}
                        }
                    }
                    if let Some(prop) = prop
                        && stats.push(prop, col_idx, name)?
                    {
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
}

#[inline]
fn mixed_prop_err<T>(column_idx: usize, property_name: &str) -> MltResult<T> {
    Err(MltError::MixedPropertyTypes(
        column_idx,
        property_name.to_owned(),
    ))
}
