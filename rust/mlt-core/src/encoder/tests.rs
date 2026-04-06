use crate::encoder::model::ColumnKind;
use crate::encoder::{ExplicitEncoder, IdWidth, IntEncoder, VertexBufferType};

impl ExplicitEncoder {
    /// Use `enc` for all integer streams, plain string encoding, and `Vec2` vertex layout.
    #[must_use]
    pub fn all(enc: IntEncoder) -> Self {
        Self {
            override_id_width: Box::new(|w| w),
            vertex_buffer_type: VertexBufferType::Vec2,
            force_stream: Box::new(|_| false),
            get_int_encoder: Box::new(move |_| enc),
            get_str_encoding: Box::new(|_| crate::encoder::StrEncoding::Plain),
            override_presence: Box::new(|_| false),
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

    /// Use `id_enc` for the ID stream with a fixed `id_width`; `varint` for all other streams.
    ///
    /// Useful for tests that need to pin the exact ID encoding without caring about
    /// geometry or property streams.
    #[must_use]
    pub fn for_id(id_enc: IntEncoder, id_width: IdWidth) -> Self {
        Self {
            override_id_width: Box::new(move |_| id_width),
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
