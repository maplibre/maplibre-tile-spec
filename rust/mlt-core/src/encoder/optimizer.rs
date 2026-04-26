use crate::decoder::TileLayer;
use crate::encoder::model::StagedLayer;
use crate::encoder::property::encode::write_properties;
use crate::encoder::property::{StringStatsBuilder, apply_string_groups};
use crate::encoder::{
    Encoder, EncoderConfig, SortStrategy, StringGroup, spatial_sort_likely_to_help,
};
use crate::{MltResult, PropValue};

impl StagedLayer {
    /// Encode and serialize the layer directly into `enc`, without creating any
    /// intermediate representation.
    ///
    /// This is the hot path inside `TileLayer::encode`: each sort-strategy
    /// trial calls this method on its own fresh `Encoder`, and only the
    /// `Encoder` with the smallest `total_len()` is kept.
    #[hotpath::measure]
    pub fn encode_into(self, mut enc: Encoder) -> MltResult<Encoder> {
        let Self {
            name,
            extent,
            id,
            geometry,
            properties,
        } = self;

        id.write_to(&mut enc)?;
        geometry.write_to(&mut enc)?;
        write_properties(&properties, &mut enc)?;
        enc.write_header(&name, extent)?;

        Ok(enc)
    }
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
        if self.features.is_empty() {
            return Ok(Vec::new());
        }

        let mut sort_by = vec![SortStrategy::Unsorted];
        let try_spatial_sort = cfg.try_spatial_morton_sort || cfg.try_spatial_hilbert_sort;
        if try_spatial_sort
            && (self.features.len() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(&self))
        {
            if cfg.try_spatial_morton_sort {
                sort_by.push(SortStrategy::SpatialMorton);
            }
            if cfg.try_spatial_hilbert_sort {
                sort_by.push(SortStrategy::SpatialHilbert);
            }
        }
        if cfg.try_id_sort {
            sort_by.push(SortStrategy::Id);
        }

        let analysis = analyze_layer(&self, cfg.allow_shared_dict);

        let (last, init) = sort_by.split_last().expect("at least one strategy");
        if init.is_empty() {
            StagedLayer::from_tile(self, *last, &analysis, cfg.tessellate)
                .encode_into(Encoder::new(cfg))?
        } else {
            let mut enc: Encoder = {
                let first = init[0];
                StagedLayer::from_tile(self.clone(), first, &analysis, cfg.tessellate)
                    .encode_into(Encoder::new(cfg))?
            };
            let mut best = enc.preserve_results();
            // Clone for all-but-last strategies
            for &sort in &init[1..] {
                let layer = StagedLayer::from_tile(self.clone(), sort, &analysis, cfg.tessellate);
                enc = layer.encode_into(enc)?;
                if enc.total_len() < best.total_len() {
                    best = enc.preserve_results();
                }
            }
            // Last strategy: consume self, no clone
            let layer = StagedLayer::from_tile(self, *last, &analysis, cfg.tessellate);
            enc = layer.encode_into(enc)?;
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
}

/// How a property participates in a shared dictionary group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedDictRole {
    /// The property is encoded as a standalone column.
    None,
    /// The property is the first column in this group and emits the shared dictionary.
    Owner(usize),
    /// The property is emitted by the group's owner column.
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
    pub string_groups: Vec<StringGroup>,
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
    Float,
    String {
        total_bytes: usize,
        max_bytes: usize,
        exact_hashes: Vec<u64>,
        trigram_hashes: Vec<u64>,
        shared_dict: SharedDictRole,
    },
}

impl PropertyTypedStats {
    #[must_use]
    pub fn values_fit_u32(&self) -> bool {
        match self {
            Self::None | Self::Bool | Self::Float | Self::String { .. } => false,
            Self::Signed { min, max } => *min >= 0 && u32::try_from(*max).is_ok(),
            Self::Unsigned { max, .. } => u32::try_from(*max).is_ok(),
        }
    }

    #[must_use]
    pub fn shared_dict(&self) -> SharedDictRole {
        match self {
            Self::String { shared_dict, .. } => *shared_dict,
            _ => SharedDictRole::None,
        }
    }

    pub(crate) fn set_shared_dict(&mut self, role: SharedDictRole) {
        match self {
            Self::String { shared_dict, .. } => *shared_dict = role,
            _ => debug_assert_eq!(role, SharedDictRole::None),
        }
    }

    pub(crate) fn push(&mut self, prop: &PropValue) {
        match prop {
            PropValue::Bool(Some(_)) => self.merge_bool(),
            PropValue::I8(Some(v)) => self.merge_signed(i64::from(*v)),
            PropValue::U8(Some(v)) => self.merge_unsigned(u64::from(*v)),
            PropValue::I32(Some(v)) => self.merge_signed(i64::from(*v)),
            PropValue::U32(Some(v)) => self.merge_unsigned(u64::from(*v)),
            PropValue::I64(Some(v)) => self.merge_signed(*v),
            PropValue::U64(Some(v)) => self.merge_unsigned(*v),
            PropValue::F32(Some(_)) | PropValue::F64(Some(_)) => self.merge_float(),
            PropValue::Str(Some(s)) => self.merge_string(s.len()),
            _ => {}
        }
    }

    fn merge_bool(&mut self) {
        self.merge_same_kind(Self::Bool);
    }

    fn merge_signed(&mut self, value: i64) {
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
            _ => panic!("mixed property types are not allowed"),
        }
    }

    fn merge_unsigned(&mut self, value: u64) {
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
            _ => panic!("mixed property types are not allowed"),
        }
    }

    fn merge_float(&mut self) {
        self.merge_same_kind(Self::Float);
    }

    fn merge_string(&mut self, len: usize) {
        match self {
            Self::None => {
                *self = Self::String {
                    total_bytes: len,
                    max_bytes: len,
                    exact_hashes: Vec::new(),
                    trigram_hashes: Vec::new(),
                    shared_dict: SharedDictRole::None,
                };
            }
            Self::String {
                total_bytes,
                max_bytes,
                exact_hashes: _,
                trigram_hashes: _,
                shared_dict: _,
            } => {
                *total_bytes += len;
                *max_bytes = (*max_bytes).max(len);
            }
            _ => panic!("mixed property types are not allowed"),
        }
    }

    fn merge_same_kind(&mut self, kind: Self) {
        match self {
            Self::None => *self = kind,
            Self::Bool if matches!(kind, Self::Bool) => {}
            Self::Float if matches!(kind, Self::Float) => {}
            _ => panic!("mixed property types are not allowed"),
        }
    }
}

/// Analyze a [`TileLayer`] and return reusable ID/property facts for the optimizer.
#[must_use]
#[hotpath::measure]
pub fn analyze_layer(source: &TileLayer, allow_shared_dict: bool) -> LayerStats {
    let id = analyze_ids(source);
    let mut properties = profile_properties(source, allow_shared_dict);
    let string_groups = if allow_shared_dict {
        apply_string_groups(&source.property_names, &mut properties)
    } else {
        Vec::new()
    };

    LayerStats {
        id,
        properties,
        string_groups,
    }
}

fn analyze_ids(source: &TileLayer) -> Option<PropertyStats> {
    let mut present = 0usize;
    let mut min = u64::MAX;
    let mut max = 0u64;
    for feature in &source.features {
        if let Some(id) = feature.id {
            present += 1;
            min = min.min(id);
            max = max.max(id);
        }
    }
    if present == 0 {
        return None;
    }
    let presence = if present == source.features.len() {
        Presence::AllPresent
    } else {
        Presence::Mixed
    };
    Some(PropertyStats {
        presence,
        stats: PropertyTypedStats::Unsigned { min, max },
    })
}

fn profile_properties(source: &TileLayer, allow_shared_dict: bool) -> Vec<PropertyStats> {
    let string_stats = allow_shared_dict.then(StringStatsBuilder::new);
    source
        .property_names
        .iter()
        .enumerate()
        .map(|(col_idx, _name)| {
            let mut present = 0usize;
            let mut stats = PropertyTypedStats::default();
            let mut string_values = Vec::new();
            for feature in &source.features {
                let prop = feature.properties.get(col_idx);
                if prop_is_present(prop) {
                    present += 1;
                    let prop = prop.expect("present property exists");
                    stats.push(prop);
                    if let (Some(_), PropValue::Str(Some(s))) = (&string_stats, prop) {
                        string_values.push(s.as_str());
                    }
                }
            }

            let presence = if present == 0 {
                Presence::AllNull
            } else if present == source.features.len() {
                Presence::AllPresent
            } else {
                Presence::Mixed
            };

            if let (Some(string_stats), false) = (string_stats.as_ref(), string_values.is_empty()) {
                let (exact, trigram) = string_stats.hashes(&string_values);
                if let PropertyTypedStats::String {
                    exact_hashes,
                    trigram_hashes,
                    ..
                } = &mut stats
                {
                    *exact_hashes = exact;
                    *trigram_hashes = trigram;
                }
            }

            PropertyStats { presence, stats }
        })
        .collect()
}

fn prop_is_present(prop: Option<&PropValue>) -> bool {
    matches!(
        prop,
        Some(
            PropValue::Bool(Some(_))
                | PropValue::I8(Some(_))
                | PropValue::U8(Some(_))
                | PropValue::I32(Some(_))
                | PropValue::U32(Some(_))
                | PropValue::I64(Some(_))
                | PropValue::U64(Some(_))
                | PropValue::F32(Some(_))
                | PropValue::F64(Some(_))
                | PropValue::Str(Some(_)),
        )
    )
}
