//! Layer envelope and column ordering for tag `0x01` (v1) layers.

#[cfg(feature = "unstable-v2")]
use crate::MltError;
use crate::MltResult;
use crate::encoder::model::StagedLayer;
use crate::encoder::property::encode::write_prop;
use crate::encoder::{Codecs, Encoder, StagedId};

/// Encode and serialize a staged layer as a v1 (tag `0x01`) body into `enc`.
pub(crate) fn encode_into01(
    layer: StagedLayer,
    mut enc: Encoder,
    codecs: &mut Codecs,
) -> MltResult<Encoder> {
    let column_count = usize::from(!matches!(layer.id, StagedId::None))
        + 1 // geometry
        + layer.properties.len();

    // v1 has nowhere to put a vertex-scoped column, so a layer with one is not
    // writable as v1 - dropping them silently would lose data the caller staged.
    #[cfg(feature = "unstable-v2")]
    if !layer.m_values.is_empty() {
        return Err(MltError::MValuesNeedV2(layer.name));
    }
    // A nested column has nowhere to go in v1 either, and flattening it would be
    // a different tile rather than the same one in another format.
    #[cfg(feature = "unstable-v2")]
    if !layer.nested.is_empty() {
        return Err(MltError::NestedNeedsV2(layer.name));
    }

    let StagedLayer {
        name,
        extent,
        id,
        geometry,
        properties,
        #[cfg(feature = "unstable-v2")]
            m_values: _,
        #[cfg(feature = "unstable-v2")]
            nested: _,
    } = layer;

    id.write_to(&mut enc, codecs)?;
    geometry.write_to(&mut enc, codecs)?;
    for prop in properties {
        write_prop(&prop, &mut enc, codecs)?;
    }
    enc.write_header01(&name, extent.get(), column_count)?;

    Ok(enc)
}
