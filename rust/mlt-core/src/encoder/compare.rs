/// Cross-type [`PartialEq`] between decode-side (`Parsed*` / `Layer01<'a>`) and
/// encode-side (`Staged*`) types.
///
/// These implementations allow round-trip tests to compare a decoded tile
/// directly against a hand-crafted `Staged*` value without having to convert
/// one side first.
use crate::Layer;
use crate::decoder::{Geometry, GeometryValues, Id, Layer01, Property, Unknown};
use crate::encoder::model::{StagedLayer, StagedLayer01};
use crate::encoder::{EncodedUnknown, StagedId, StagedProperty};
use crate::{DecodeState, Lazy};

/// Compare a decoded `Id<'_, Lazy>` against an encoder-side `StagedId`.
/// Uses `materialize()` on both sides to sidestep the lifetime difference.
impl PartialEq<StagedId> for Id<'_, Lazy> {
    fn eq(&self, other: &StagedId) -> bool {
        match self {
            Self::Parsed(parsed) => parsed.materialize() == other.materialize(),
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Id<'_, Lazy>> for StagedId {
    fn eq(&self, other: &Id<'_, Lazy>) -> bool {
        other == self
    }
}

impl PartialEq<GeometryValues> for Geometry<'_, Lazy> {
    fn eq(&self, other: &GeometryValues) -> bool {
        match self {
            Self::Parsed(parsed) => parsed == other,
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Geometry<'_, Lazy>> for GeometryValues {
    fn eq(&self, other: &Geometry<'_, Lazy>) -> bool {
        other == self
    }
}

impl PartialEq<StagedProperty> for Property<'_, Lazy> {
    fn eq(&self, other: &StagedProperty) -> bool {
        match self {
            Self::Parsed(parsed) => parsed == other,
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Property<'_, Lazy>> for StagedProperty {
    fn eq(&self, other: &Property<'_, Lazy>) -> bool {
        other == self
    }
}

/// TODO: not certain this is needed
impl<'a, S> PartialEq for Layer01<'a, S>
where
    S: DecodeState,
    Option<Id<'a, S>>: PartialEq,
    Geometry<'a, S>: PartialEq,
    Vec<Property<'a, S>>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.extent == other.extent
            && self.id == other.id
            && self.geometry == other.geometry
            && self.properties == other.properties
    }
}

impl PartialEq<StagedLayer01> for Layer01<'_, Lazy> {
    fn eq(&self, other: &StagedLayer01) -> bool {
        let Self {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
                layer_order: _,
        } = self;
        let StagedLayer01 {
            name: other_name,
            extent: other_extent,
            id: other_id,
            geometry: other_geometry,
            properties: other_properties,
        } = other;
        name == other_name
            && extent == other_extent
            && id.as_ref().map_or(other_id.is_none(), |id| {
                other_id.as_ref().is_some_and(|oid| id == oid)
            })
            && geometry == other_geometry
            && properties.len() == other_properties.len()
            && properties.iter().zip(other_properties).all(|(a, b)| a == b)
    }
}

impl PartialEq<Layer01<'_, Lazy>> for StagedLayer01 {
    fn eq(&self, other: &Layer01<'_, Lazy>) -> bool {
        other == self
    }
}

impl PartialEq<EncodedUnknown> for Unknown<'_> {
    fn eq(&self, other: &EncodedUnknown) -> bool {
        let Self { tag, value } = self;
        let EncodedUnknown {
            tag: other_tag,
            value: other_value,
        } = other;
        tag == other_tag && *value == other_value.as_slice()
    }
}

impl PartialEq<Unknown<'_>> for EncodedUnknown {
    fn eq(&self, other: &Unknown<'_>) -> bool {
        other == self
    }
}

impl PartialEq<StagedLayer> for Layer<'_> {
    fn eq(&self, other: &StagedLayer) -> bool {
        match (self, other) {
            (Self::Tag01(a), StagedLayer::Tag01(b)) => a == b,
            (Self::Unknown(a), StagedLayer::Unknown(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialEq<Layer<'_>> for StagedLayer {
    fn eq(&self, other: &Layer<'_>) -> bool {
        other == self
    }
}
