use super::encode::{compute_geometry_payloads, select_vertex_strategy, write_geometry_auto};
use crate::MltResult;
use crate::decoder::GeometryValues;
use crate::encoder::Encoder;
use crate::encoder::optimizer::EncoderConfig;

impl GeometryValues {
    /// Automatically select the best encoder and write the geometry column to `enc`.
    ///
    /// Writes the `Geometry` column-type byte to [`enc.meta`](Encoder::meta) and
    /// all geometry streams to [`enc.data`](Encoder::data).
    pub fn write_to(self, enc: &mut Encoder, _cfg: EncoderConfig) -> MltResult<()> {
        let vertex_buffer_type = self
            .vertices
            .as_deref()
            .map_or(super::model::VertexBufferType::Vec2, select_vertex_strategy);
        let payloads = compute_geometry_payloads(&self, vertex_buffer_type)?;
        write_geometry_auto(&payloads, enc)
    }
}
