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
    let curve_params = tile.curve_params();
    StagedLayer::from_tile(tile, sort, &analysis, tessellate, curve_params)
}

#[cfg(test)]
mod invariant_tests {
    use geo_types::{Geometry, Point};

    use crate::decoder::{GeometryValues, TileFeature};
    use crate::encoder::{Codecs, Encoder, EncoderConfig, StagedId, StagedLayer};
    use crate::{MltError, TileLayer};

    fn empty_named_tile(features: Vec<TileFeature>) -> TileLayer {
        TileLayer {
            name: String::new(),
            extent: 4096,
            property_names: vec![],
            features,
        }
    }

    fn point_feature() -> TileFeature {
        TileFeature {
            id: None,
            geometry: Geometry::Point(Point::new(0, 0)),
            properties: vec![],
        }
    }

    #[test]
    fn tile_layer_encode_rejects_empty_name() {
        for tile in [
            empty_named_tile(vec![]),
            empty_named_tile(vec![point_feature()]),
        ] {
            assert!(matches!(
                tile.encode(EncoderConfig::default()),
                Err(MltError::MissingLayerName)
            ));
        }
    }

    #[test]
    fn staged_layer_encode_rejects_empty_name() {
        let staged = StagedLayer {
            name: String::new(),
            extent: 4096,
            id: StagedId::None,
            geometry: GeometryValues::default(),
            properties: vec![],
        };

        assert!(matches!(
            staged.encode_into(Encoder::default(), &mut Codecs::default()),
            Err(MltError::MissingLayerName)
        ));
    }
}
