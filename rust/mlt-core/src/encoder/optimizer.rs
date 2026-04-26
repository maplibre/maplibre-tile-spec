use crate::decoder::TileLayer;
use crate::encoder::model::StagedLayer;
use crate::encoder::property::apply_string_groups;
use crate::encoder::property::encode::write_properties;
use crate::encoder::{
    Encoder, EncoderConfig, SortStrategy, StringGroup, spatial_sort_likely_to_help,
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

        let stats = self.analyze(cfg.allow_shared_dict)?;

        let (last, init) = sort_by.split_last().expect("at least one strategy");
        if init.is_empty() {
            StagedLayer::from_tile(self, *last, &stats, cfg.tessellate)
                .encode_into(Encoder::new(cfg))?
        } else {
            let mut enc: Encoder = {
                let first = init[0];
                StagedLayer::from_tile(self.clone(), first, &stats, cfg.tessellate)
                    .encode_into(Encoder::new(cfg))?
            };
            let mut best = enc.preserve_results();
            // Clone for all-but-last strategies
            for &sort in &init[1..] {
                let layer = StagedLayer::from_tile(self.clone(), sort, &stats, cfg.tessellate);
                enc = layer.encode_into(enc)?;
                if enc.total_len() < best.total_len() {
                    best = enc.preserve_results();
                }
            }
            // Last strategy: consume self, no clone
            let layer = StagedLayer::from_tile(self, *last, &stats, cfg.tessellate);
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

    pub(crate) fn push(&mut self, prop: &PropValue) -> MltResult<()> {
        match prop {
            PropValue::Bool(Some(_)) => self.merge_bool()?,
            PropValue::I8(Some(v)) => self.merge_signed(i64::from(*v))?,
            PropValue::U8(Some(v)) => self.merge_unsigned(u64::from(*v))?,
            PropValue::I32(Some(v)) => self.merge_signed(i64::from(*v))?,
            PropValue::U32(Some(v)) => self.merge_unsigned(u64::from(*v))?,
            PropValue::I64(Some(v)) => self.merge_signed(*v)?,
            PropValue::U64(Some(v)) => self.merge_unsigned(*v)?,
            PropValue::F32(Some(_)) => self.merge_same_kind(Self::F32)?,
            PropValue::F64(Some(_)) => self.merge_same_kind(Self::F64)?,
            PropValue::Str(Some(_)) => self.merge_string()?,
            _ => {}
        }
        Ok(())
    }

    fn merge_bool(&mut self) -> MltResult<()> {
        self.merge_same_kind(Self::Bool)
    }

    fn merge_signed(&mut self, value: i64) -> MltResult<()> {
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
            _ => return Err(MltError::MixedPropertyTypes),
        }
        Ok(())
    }

    fn merge_unsigned(&mut self, value: u64) -> MltResult<()> {
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
            _ => return Err(MltError::MixedPropertyTypes),
        }
        Ok(())
    }

    fn merge_string(&mut self) -> MltResult<()> {
        match self {
            Self::None => {
                *self = Self::String {
                    shared_dict: SharedDictRole::None,
                };
            }
            Self::String { .. } => {}
            _ => return Err(MltError::MixedPropertyTypes),
        }
        Ok(())
    }

    fn merge_same_kind(&mut self, kind: Self) -> MltResult<()> {
        match self {
            Self::None => *self = kind,
            Self::Bool if matches!(kind, Self::Bool) => {}
            Self::F32 if matches!(kind, Self::F32) => {}
            Self::F64 if matches!(kind, Self::F64) => {}
            _ => return Err(MltError::MixedPropertyTypes),
        }
        Ok(())
    }
}

impl TileLayer {
    /// Analyze a [`TileLayer`] and return reusable ID/property facts for the optimizer.
    #[hotpath::measure]
    pub(crate) fn analyze(&self, allow_shared_dict: bool) -> MltResult<LayerStats> {
        let id = self.analyze_ids();
        let mut properties = self.profile_properties()?;
        let string_groups = if allow_shared_dict {
            self.apply_string_groups(&mut properties)
        } else {
            Vec::new()
        };

        Ok(LayerStats {
            id,
            properties,
            string_groups,
        })
    }

    fn analyze_ids(&self) -> Option<PropertyStats> {
        let mut present = 0usize;
        let mut min = u64::MAX;
        let mut max = 0u64;
        for feature in &self.features {
            if let Some(id) = feature.id {
                present += 1;
                min = min.min(id);
                max = max.max(id);
            }
        }
        if present == 0 {
            return None;
        }
        let presence = if present == self.features.len() {
            Presence::AllPresent
        } else {
            Presence::Mixed
        };
        Some(PropertyStats {
            presence,
            stats: PropertyTypedStats::Unsigned { min, max },
        })
    }

    fn profile_properties(&self) -> MltResult<Vec<PropertyStats>> {
        self.property_names
            .iter()
            .enumerate()
            .map(|(col_idx, _name)| -> MltResult<PropertyStats> {
                let mut present = 0usize;
                let mut stats = PropertyTypedStats::default();
                for feature in &self.features {
                    let prop = feature.properties.get(col_idx);
                    if prop_is_present(prop) {
                        present += 1;
                        let prop = prop.expect("present property exists");
                        stats.push(prop)?;
                    }
                }

                let presence = if present == 0 {
                    Presence::AllNull
                } else if present == self.features.len() {
                    Presence::AllPresent
                } else {
                    Presence::Mixed
                };

                Ok(PropertyStats { presence, stats })
            })
            .collect()
    }
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
