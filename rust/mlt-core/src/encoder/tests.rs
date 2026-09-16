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
            get_float_encoding: Box::new(|_| crate::encoder::FloatEncoding::None),
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
    use crate::decoder::GeometryValues;
    #[cfg(feature = "unstable-v2")]
    use crate::encoder::{
        Presence, StagedInterior, StagedLeaf, StagedList, StagedMValue, StagedNested, StagedNode,
        StagedSharedDict, StagedValues,
    };
    use crate::encoder::{StagedId, StagedLayer, StagedProperty};
    use crate::{MltError, TileLayer};

    #[cfg(feature = "unstable-v2")]
    fn empty_shared_dict(prefix: &str, suffix: &str) -> StagedProperty {
        let children = [(
            suffix.to_string(),
            Vec::<Option<String>>::new(),
            Presence::AllPresent,
        )];
        StagedProperty::SharedDict(
            StagedSharedDict::new(prefix, children).expect("build shared dict"),
        )
    }

    #[cfg(feature = "unstable-v2")]
    fn empty_m_value(name: &str) -> StagedMValue {
        StagedMValue::new(name, None, StagedValues::U32(Vec::new()))
    }

    #[cfg(feature = "unstable-v2")]
    fn empty_nested(name: &str) -> StagedNested {
        let leaf = StagedNode::Leaf(StagedLeaf::new(None, StagedValues::U32(Vec::new())));
        StagedNested::new(
            name,
            StagedInterior::List(StagedList::new(None, Vec::new(), leaf)),
        )
    }

    #[cfg(feature = "unstable-v2")]
    fn staged_m_values(
        properties: Vec<StagedProperty>,
        m_values: Vec<StagedMValue>,
    ) -> crate::MltResult<StagedLayer> {
        StagedLayer::with_m_values(
            "layer",
            4096,
            StagedId::None,
            GeometryValues::default(),
            properties,
            m_values,
        )
    }

    #[cfg(feature = "unstable-v2")]
    fn staged_nested(
        properties: Vec<StagedProperty>,
        m_values: Vec<StagedMValue>,
        nested: Vec<StagedNested>,
    ) -> crate::MltResult<StagedLayer> {
        StagedLayer::with_nested(
            "layer",
            4096,
            StagedId::None,
            GeometryValues::default(),
            properties,
            m_values,
            nested,
        )
    }

    #[test]
    fn tile_layer_constructor_rejects_empty_name() {
        assert!(matches!(
            TileLayer::new("", 4096),
            Err(MltError::MissingLayerName)
        ));
    }

    #[test]
    fn staged_layer_constructor_rejects_empty_name() {
        assert!(matches!(
            StagedLayer::new("", 4096, StagedId::None, GeometryValues::default(), vec![]),
            Err(MltError::MissingLayerName)
        ));
    }

    #[test]
    fn staged_layer_constructor_rejects_zero_extent() {
        assert!(matches!(
            StagedLayer::new(
                "layer",
                0,
                StagedId::None,
                GeometryValues::default(),
                vec![]
            ),
            Err(MltError::InvalidExtent(0))
        ));
    }

    #[test]
    fn staged_layer_constructor_rejects_duplicate_property_names() {
        let props = vec![
            StagedProperty::opt_u32("dup", Vec::<Option<u32>>::new()),
            StagedProperty::opt_u32("dup", Vec::<Option<u32>>::new()),
        ];
        assert_eq!(
            StagedLayer::new(
                "layer",
                4096,
                StagedId::None,
                GeometryValues::default(),
                props
            )
            .unwrap_err()
            .to_string(),
            "duplicate column name dup: the property column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_duplicate_m_value_names() {
        assert_eq!(
            staged_m_values(vec![], vec![empty_m_value("dup"), empty_m_value("dup")])
                .unwrap_err()
                .to_string(),
            "duplicate column name dup: the m-value column repeats the m-value column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_an_m_value_repeating_a_property_name() {
        let props = vec![StagedProperty::opt_u32("dup", Vec::<Option<u32>>::new())];
        assert_eq!(
            staged_m_values(props, vec![empty_m_value("dup")])
                .unwrap_err()
                .to_string(),
            "duplicate column name dup: the m-value column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_an_m_value_repeating_a_shared_dict_child_name() {
        assert_eq!(
            staged_m_values(
                vec![empty_shared_dict("a:", "b")],
                vec![empty_m_value("a:b")]
            )
            .unwrap_err()
            .to_string(),
            "duplicate column name a:b: the m-value column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_allows_an_m_value_named_after_a_shared_dict_prefix() {
        assert!(
            staged_m_values(
                vec![empty_shared_dict("a:", "b")],
                vec![empty_m_value("a:")]
            )
            .is_ok()
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_duplicate_nested_names() {
        assert_eq!(
            staged_nested(
                vec![],
                vec![],
                vec![empty_nested("dup"), empty_nested("dup")]
            )
            .unwrap_err()
            .to_string(),
            "duplicate column name dup: the nested column repeats the nested column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_a_nested_column_repeating_a_property_name() {
        let props = vec![StagedProperty::opt_u32("dup", Vec::<Option<u32>>::new())];
        assert_eq!(
            staged_nested(props, vec![], vec![empty_nested("dup")])
                .unwrap_err()
                .to_string(),
            "duplicate column name dup: the nested column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_a_nested_column_repeating_a_shared_dict_child_name() {
        assert_eq!(
            staged_nested(
                vec![empty_shared_dict("a:", "b")],
                vec![],
                vec![empty_nested("a:b")]
            )
            .unwrap_err()
            .to_string(),
            "duplicate column name a:b: the nested column repeats the property column"
        );
    }

    #[cfg(feature = "unstable-v2")]
    #[test]
    fn staged_layer_constructor_rejects_a_nested_column_repeating_an_m_value_name() {
        assert_eq!(
            staged_nested(
                vec![],
                vec![empty_m_value("dup")],
                vec![empty_nested("dup")]
            )
            .unwrap_err()
            .to_string(),
            "duplicate column name dup: the nested column repeats the m-value column"
        );
    }
}
