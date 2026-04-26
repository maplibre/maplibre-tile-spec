use crate::TileLayer;
use crate::encoder::model::ColumnKind;
use crate::encoder::{ExplicitEncoder, IntEncoder, SortStrategy, StagedLayer, VertexBufferType};

impl ExplicitEncoder {
    /// Use `enc` for all integer streams, plain string encoding, and `Vec2` vertex layout.
    #[must_use]
    pub fn all(enc: IntEncoder) -> Self {
        Self {
            vertex_buffer_type: VertexBufferType::Vec2,
            force_stream: Box::new(|_| false),
            get_int_encoder: Box::new(move |_| enc),
            get_str_encoding: Box::new(|_| crate::encoder::StrEncoding::Plain),
        }
    }

    /// Like [`Self::all`] but use `str_enc` for string property columns.
    #[must_use]
    pub fn all_with_str(enc: IntEncoder, str_enc: crate::encoder::StrEncoding) -> Self {
        Self {
            get_str_encoding: Box::new(move |_| str_enc),
            ..Self::all(enc)
        }
    }

    /// Use `id_enc` for the ID stream; `varint` for all other streams.
    ///
    /// Useful for tests that need to pin the exact ID encoding without caring about
    /// geometry or property streams.
    #[must_use]
    pub fn for_id(id_enc: IntEncoder) -> Self {
        Self {
            get_int_encoder: Box::new(move |ctx| {
                if ctx.kind == ColumnKind::Id {
                    id_enc
                } else {
                    IntEncoder::varint()
                }
            }),
            ..Self::all(IntEncoder::varint())
        }
    }
}

#[must_use]
pub fn stage_tile(
    tile: TileLayer,
    sort: SortStrategy,
    allow_shared_dict: bool,
    tessellate: bool,
) -> StagedLayer {
    let analysis = tile.analyze(allow_shared_dict).expect("analyze tile");
    StagedLayer::from_tile(tile, sort, &analysis, tessellate)
}
