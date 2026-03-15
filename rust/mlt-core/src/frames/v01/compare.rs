/// Cross-type [`PartialEq`] between decode-side (`Parsed*` / `Layer01<'a>`) and
/// encode-side (`Staged*`) types.
///
/// These implementations allow round-trip tests to compare a decoded tile
/// directly against a hand-crafted `Staged*` value without having to convert
/// one side first.
use crate::frames::model::{EncodedUnknown, Layer, StagedLayer, Unknown};
use crate::v01::{
    Geometry, GeometryValues, Id, IdValues, Layer01, Property, StagedLayer01, StagedProperty,
};

// ── Id ────────────────────────────────────────────────────────────────────────

impl PartialEq<IdValues> for Id<'_> {
    fn eq(&self, other: &IdValues) -> bool {
        match self {
            Self::Parsed(parsed) => parsed == other,
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Id<'_>> for IdValues {
    fn eq(&self, other: &Id<'_>) -> bool {
        other == self
    }
}

// ── Geometry ──────────────────────────────────────────────────────────────────

impl PartialEq<GeometryValues> for Geometry<'_> {
    fn eq(&self, other: &GeometryValues) -> bool {
        match self {
            Self::Parsed(parsed) => parsed == other,
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Geometry<'_>> for GeometryValues {
    fn eq(&self, other: &Geometry<'_>) -> bool {
        other == self
    }
}

// ── Property ──────────────────────────────────────────────────────────────────

impl PartialEq<StagedProperty> for Property<'_> {
    fn eq(&self, other: &StagedProperty) -> bool {
        match self {
            Self::Parsed(parsed) => parsed == other,
            Self::Raw(_) | Self::ParsingFailed => false,
        }
    }
}

impl PartialEq<Property<'_>> for StagedProperty {
    fn eq(&self, other: &Property<'_>) -> bool {
        other == self
    }
}

// ── Layer01 ───────────────────────────────────────────────────────────────────

impl PartialEq<StagedLayer01> for Layer01<'_> {
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

impl PartialEq<Layer01<'_>> for StagedLayer01 {
    fn eq(&self, other: &Layer01<'_>) -> bool {
        other == self
    }
}

// ── Unknown ───────────────────────────────────────────────────────────────────

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

// ── Layer ─────────────────────────────────────────────────────────────────────

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
