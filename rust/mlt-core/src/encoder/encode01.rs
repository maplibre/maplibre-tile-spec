//! Layer envelope and column ordering for tag `0x01` (v1) layers.

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

    let StagedLayer {
        name,
        extent,
        id,
        geometry,
        properties,
    } = layer;

    id.write_to(&mut enc, codecs)?;
    geometry.write_to(&mut enc, codecs)?;
    for prop in properties {
        write_prop(&prop, &mut enc, codecs)?;
    }
    enc.write_header01(&name, extent.get(), column_count)?;

    Ok(enc)
}
