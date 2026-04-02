use crate::Analyze;
use crate::decoder::StreamMeta;
use crate::encoder::EncodedStream;

/// Wire-ready encoded geometry data (owns its byte buffers)
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedGeometry {
    pub meta: EncodedStream,
    pub items: Vec<EncodedStream>,
}

impl Analyze for EncodedGeometry {
    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        self.meta.for_each_stream(cb);
        self.items.for_each_stream(cb);
    }
}

/// Describes how polygon tessellation should be performed during geometry value construction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum TessellationMode {
    /// No tessellation; polygons are stored as outline rings only.
    #[default]
    None,
    /// Tessellate polygons using the Earcut algorithm, producing triangle index buffers.
    Earcut,
}

/// Describes how the vertex buffer should be encoded.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum VertexBufferType {
    /// Standard 2D `(x, y)` pairs encoded with componentwise delta.
    #[default]
    Vec2,
    /// Morton (Z-order) dictionary encoding:
    /// Unique vertices are sorted by their Morton code and stored once.
    /// Each vertex position in the stream is replaced by its index into that dictionary.
    Morton,
}
