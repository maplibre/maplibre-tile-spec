mod store;

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Diplomat requires `Box<T>` returns for opaque constructors"
)]
#[expect(
    clippy::use_self,
    reason = "Diplomat expands macros using the concrete type name rather than `Self`."
)]
#[diplomat::bridge]
mod ffi {
    use std::fmt::Write as _;

    use mlt_core::encoder::{EncoderConfig, WireVersion};
    use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
    use mlt_core::{Decoder, Parser};

    use crate::store::{LayerBuffer, MvtGeometryType, StoreError, StoredValue};

    /// Which stage of a conversion failed.
    #[derive(Debug, PartialEq, Eq)]
    pub enum ConvertErrorKind {
        /// Input bytes could not be parsed or decoded.
        InvalidInput,
        /// Encoding failed.
        EncodingFailed,
        /// A feature handed to a layer builder is malformed.
        InvalidFeature,
    }

    /// Error returned by FFI conversion functions.
    #[diplomat::opaque]
    #[diplomat::attr(auto, error)]
    pub struct ConvertError {
        kind: ConvertErrorKind,
        message: String,
    }

    impl ConvertError {
        /// The stage that failed.
        #[diplomat::attr(auto, getter = "kind")]
        pub fn kind(&self) -> ConvertErrorKind {
            self.kind
        }

        /// Human-readable cause.
        #[diplomat::attr(auto, getter = "message")]
        pub fn message(&self, write: &mut DiplomatWrite) {
            let _ = write.write_str(&self.message);
        }

        #[cfg(test)]
        pub(crate) fn message_text(&self) -> &str {
            &self.message
        }
    }

    impl ConvertError {
        fn new(kind: ConvertErrorKind, source: &impl std::fmt::Display) -> Box<Self> {
            Box::new(Self {
                kind,
                message: source.to_string(),
            })
        }
    }

    fn feature_error(source: &StoreError) -> Box<ConvertError> {
        ConvertError::new(ConvertErrorKind::InvalidFeature, source)
    }

    fn invalid_input(source: impl std::fmt::Display) -> Box<ConvertError> {
        ConvertError::new(ConvertErrorKind::InvalidInput, &source)
    }

    fn encoding_failed(source: impl std::fmt::Display) -> Box<ConvertError> {
        ConvertError::new(ConvertErrorKind::EncodingFailed, &source)
    }

    fn set_value(
        buffer: &mut LayerBuffer,
        key: u32,
        value: StoredValue,
    ) -> Result<(), Box<ConvertError>> {
        buffer.set(key, value).map_err(|e| feature_error(&e))
    }

    /// Owned byte buffer returned from conversion functions.
    ///
    /// The caller borrows the contents via [`as_bytes`](MltBuffer::as_bytes)
    /// and the buffer is freed when the handle is dropped.
    #[diplomat::opaque_mut]
    pub struct MltBuffer(Vec<u8>);

    impl MltBuffer {
        /// An empty buffer, to collect the layers of one tile.
        #[diplomat::attr(auto, constructor)]
        pub fn new() -> Box<MltBuffer> {
            Box::new(MltBuffer(Vec::new()))
        }

        /// Empty the buffer, keeping its allocation for the next tile.
        pub fn clear(&mut self) {
            self.0.clear();
        }

        /// Borrow the contents as a byte slice.
        #[diplomat::attr(auto, getter = "bytes")]
        #[expect(
            clippy::needless_lifetimes,
            reason = "diplomat requires explicit lifetimes"
        )]
        pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
            &self.0
        }

        /// Number of bytes in the buffer.
        #[diplomat::attr(auto, getter = "len")]
        pub fn len(&self) -> usize {
            self.0.len()
        }
    }

    /// The wire format an encoded layer uses.
    #[expect(
        dead_code,
        reason = "Diplomat constructs the variants on the foreign side"
    )]
    pub enum MltWireVersion {
        /// Tag `0x01`, the stable v1 format.
        V01,
        /// Tag `0x02`, the experimental v2 format.
        V02,
    }

    /// Encoder options controlling which optimisations are attempted for
    /// MVT -> MLT conversion.
    ///
    /// Construct with [`new`](MltEncoderOptions::new) (FSST, `FastPFOR` and shared
    /// dictionaries enabled, no sorting and no tessellation) and toggle individual flags with the
    /// setter methods.
    #[diplomat::opaque_mut]
    pub struct MltEncoderOptions(EncoderConfig);

    impl MltEncoderOptions {
        /// Create encoder options with the default configuration.
        #[diplomat::attr(auto, constructor)]
        pub fn new() -> Box<MltEncoderOptions> {
            Box::new(MltEncoderOptions(EncoderConfig::default()))
        }

        /// Generate tessellation data for polygons and multi-polygons.
        pub fn set_tessellate(&mut self, enabled: bool) {
            self.0 = self.0.with_tessellation(enabled);
        }

        /// Try sorting features by the Z-order (Morton) curve index.
        pub fn set_attempt_spatial_morton_sort(&mut self, enabled: bool) {
            self.0 = self.0.with_spatial_morton_sort(enabled);
        }

        /// Try sorting features by the Hilbert curve index.
        pub fn set_attempt_spatial_hilbert_sort(&mut self, enabled: bool) {
            self.0 = self.0.with_spatial_hilbert_sort(enabled);
        }

        /// Try sorting features by their feature ID in ascending order.
        pub fn set_attempt_id_sort(&mut self, enabled: bool) {
            self.0 = self.0.with_id_sort(enabled);
        }

        /// Allow FSST string compression.
        pub fn set_allow_fsst(&mut self, enabled: bool) {
            self.0 = self.0.with_fsst(enabled);
        }

        /// Allow `FastPFOR` integer compression.
        pub fn set_allow_fastpfor(&mut self, enabled: bool) {
            self.0 = self.0.with_fastpfor(enabled);
        }

        /// Allow string grouping into shared dictionaries.
        pub fn set_allow_shared_dict(&mut self, enabled: bool) {
            self.0 = self.0.with_shared_dict(enabled);
        }

        /// Select the wire format to encode to.
        /// Every setter marked v2 only has no effect on a v1 layer.
        pub fn set_wire_version(&mut self, version: MltWireVersion) {
            let version = match version {
                MltWireVersion::V01 => WireVersion::V01,
                MltWireVersion::V02 => WireVersion::V02,
            };
            self.0 = self.0.with_wire_version(version);
        }

        /// v2 only: let a tessellated all-polygon layer store its triangles without the outlines.
        /// Each polygon then decodes as the triangles it was cut into.
        /// Requires tessellation.
        pub fn set_allow_triangles_only(&mut self, enabled: bool) {
            self.0 = self.0.with_triangles_only(enabled);
        }

        /// v2 only: allow integer and vertex streams to store the deltas of their deltas.
        pub fn set_allow_delta2(&mut self, enabled: bool) {
            self.0 = self.0.with_delta2(enabled);
        }

        /// v2 only: allow float columns to store one code per value into a dictionary.
        pub fn set_allow_float_dict(&mut self, enabled: bool) {
            self.0 = self.0.with_float_dict(enabled);
        }

        /// v2 only: allow float columns to store each value as a decimal-scaled integer (ALP).
        pub fn set_allow_float_alp(&mut self, enabled: bool) {
            self.0 = self.0.with_float_alp(enabled);
        }

        /// v2 only: allow dictionary code streams to be bit-packed.
        pub fn set_allow_packed_dict_codes(&mut self, enabled: bool) {
            self.0 = self.0.with_packed_dict_codes(enabled);
        }

        /// v2 only: allow plain vertex streams to be rANS-coded.
        pub fn set_allow_rans_vertices(&mut self, enabled: bool) {
            self.0 = self.0.with_rans_vertices(enabled);
        }

        /// v2 only: let a nested struct or map code its row shapes instead of per-field presence.
        pub fn set_allow_row_shapes(&mut self, enabled: bool) {
            self.0 = self.0.with_row_shapes(enabled);
        }
    }

    impl From<MltMvtGeometryType> for MvtGeometryType {
        fn from(geometry: MltMvtGeometryType) -> Self {
            match geometry {
                MltMvtGeometryType::Point => Self::Point,
                MltMvtGeometryType::LineString => Self::LineString,
                MltMvtGeometryType::Polygon => Self::Polygon,
            }
        }
    }

    /// The geometry type of an MVT feature, which also covers its multi variant.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Diplomat constructs the variants on the foreign side"
        )
    )]
    pub enum MltMvtGeometryType {
        Point,
        LineString,
        Polygon,
    }

    /// A layer written one feature at a time, then encoded without going through MVT.
    #[diplomat::opaque_mut]
    pub struct MltLayerBuilder(LayerBuffer);

    impl MltLayerBuilder {
        /// Start a layer.
        pub fn new(name: &str, extent: u32) -> Result<Box<MltLayerBuilder>, Box<ConvertError>> {
            LayerBuffer::new(name, extent)
                .map(|buffer| Box::new(MltLayerBuilder(buffer)))
                .map_err(|e| ConvertError::new(ConvertErrorKind::InvalidFeature, &e))
        }

        /// Start another layer, keeping the allocations of the last one.
        pub fn reset(&mut self, name: &str, extent: u32) -> Result<(), Box<ConvertError>> {
            self.0
                .reset(name, extent)
                .map_err(|e| ConvertError::new(ConvertErrorKind::InvalidFeature, &e))
        }

        /// Declare a property column, returning the key to set it with.
        /// Declaring a name again returns its existing key.
        pub fn add_property(&mut self, name: &str) -> u32 {
            self.0.add_property(name)
        }

        /// Start a feature with its geometry given as MVT commands, ending the previous one.
        /// A ring with positive area starts a polygon and any other ring is a hole of the last one.
        pub fn begin_mvt_feature(
            &mut self,
            geometry: MltMvtGeometryType,
            commands: &[u32],
            id: Option<u64>,
        ) -> Result<(), Box<ConvertError>> {
            self.0
                .begin_mvt_feature(geometry.into(), commands, id)
                .map_err(|e| feature_error(&e))
        }

        /// Set a boolean property of the current feature.
        pub fn set_bool(&mut self, key: u32, value: bool) -> Result<(), Box<ConvertError>> {
            set_value(&mut self.0, key, StoredValue::Bool(value))
        }

        /// Set an integer property of the current feature.
        pub fn set_i64(&mut self, key: u32, value: i64) -> Result<(), Box<ConvertError>> {
            set_value(&mut self.0, key, StoredValue::I64(value))
        }

        /// Set a 32-bit float property of the current feature.
        pub fn set_f32(&mut self, key: u32, value: f32) -> Result<(), Box<ConvertError>> {
            set_value(&mut self.0, key, StoredValue::F32(value))
        }

        /// Set a 64-bit float property of the current feature.
        pub fn set_f64(&mut self, key: u32, value: f64) -> Result<(), Box<ConvertError>> {
            set_value(&mut self.0, key, StoredValue::F64(value))
        }

        /// Set a string property of the current feature.
        pub fn set_str(&mut self, key: u32, value: &str) -> Result<(), Box<ConvertError>> {
            self.0.set_str(key, value).map_err(|e| feature_error(&e))
        }

        /// Encode the layer and append it to `out`.
        pub fn encode_into(
            &self,
            options: &MltEncoderOptions,
            out: &mut MltBuffer,
        ) -> Result<(), Box<ConvertError>> {
            let layer = self.0.encode(options.0).map_err(encoding_failed)?;
            out.0.extend_from_slice(&layer);
            Ok(())
        }
    }

    fn decode_to_mvt(
        mlt: &[u8],
        mut parser: Parser,
        mut dec: Decoder,
    ) -> Result<Box<MltBuffer>, Box<ConvertError>> {
        let layers = parser.parse_layers(mlt).map_err(invalid_input)?;
        let mut tiles = Vec::new();
        for layer in layers {
            let tile = layer.into_tile(&mut dec).map_err(invalid_input)?;
            tiles.extend(tile);
        }
        let out = tile_layers_to_mvt(tiles).map_err(invalid_input)?;
        Ok(Box::new(MltBuffer(out)))
    }

    /// Stateless FFI entry-points for MLT <-> MVT conversion.
    #[diplomat::opaque]
    pub struct MltConverter;

    impl MltConverter {
        /// Decode MLT bytes into MVT bytes.
        pub fn mlt_to_mvt(mlt: &[u8]) -> Result<Box<MltBuffer>, Box<ConvertError>> {
            decode_to_mvt(mlt, Parser::default(), Decoder::default())
        }

        /// Decode MLT bytes into MVT bytes within a memory budget.
        pub fn mlt_to_mvt_with_limit(
            mlt: &[u8],
            max_bytes: u32,
        ) -> Result<Box<MltBuffer>, Box<ConvertError>> {
            decode_to_mvt(
                mlt,
                Parser::with_max_size(max_bytes),
                Decoder::with_max_size(max_bytes),
            )
        }

        /// Encode MVT bytes into MLT bytes using the given encoder options.
        pub fn mvt_to_mlt(
            mvt: &[u8],
            options: &MltEncoderOptions,
        ) -> Result<Box<MltBuffer>, Box<ConvertError>> {
            let mut out = Vec::new();
            let layers = mvt_to_tile_layers(mvt).map_err(encoding_failed)?;
            for tile in layers {
                let encoded_tile = tile.encode(options.0).map_err(encoding_failed)?;
                out.extend_from_slice(&encoded_tile);
            }
            Ok(Box::new(MltBuffer(out)))
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use mlt_core::mvt::mvt_to_tile_layers;

    use super::ffi::{
        ConvertErrorKind, MltBuffer, MltConverter, MltEncoderOptions, MltLayerBuilder,
        MltMvtGeometryType, MltWireVersion,
    };

    const POINT: &[u8] = include_bytes!("../../../test/fixtures/simple/point-boolean.mvt");
    const POLYGON: &[u8] = include_bytes!("../../../test/fixtures/simple/polygon-boolean.mvt");
    const MULTIPOLYGON: &[u8] =
        include_bytes!("../../../test/fixtures/simple/multipolygon-boolean.mvt");

    #[test]
    fn v1_encoding_of_a_point_layer_decodes_to_the_same_layers() {
        let options = MltEncoderOptions::new();

        let mlt = MltConverter::mvt_to_mlt(POINT, &options).ok().unwrap();
        let decoded = MltConverter::mlt_to_mvt(mlt.as_bytes()).ok().unwrap();

        assert_eq!(
            mvt_to_tile_layers(decoded.as_bytes()).unwrap(),
            mvt_to_tile_layers(POINT).unwrap()
        );
    }

    #[test]
    fn v1_encoding_of_a_multipolygon_layer_decodes_to_the_same_layers() {
        let options = MltEncoderOptions::new();

        let mlt = MltConverter::mvt_to_mlt(MULTIPOLYGON, &options)
            .ok()
            .unwrap();
        let decoded = MltConverter::mlt_to_mvt(mlt.as_bytes()).ok().unwrap();

        assert_eq!(
            mvt_to_tile_layers(decoded.as_bytes()).unwrap(),
            mvt_to_tile_layers(MULTIPOLYGON).unwrap()
        );
    }

    #[test]
    fn v2_encoding_of_a_multipolygon_layer_decodes_to_the_same_layers() {
        let mut options = MltEncoderOptions::new();
        options.set_wire_version(MltWireVersion::V02);

        let mlt = MltConverter::mvt_to_mlt(MULTIPOLYGON, &options)
            .ok()
            .unwrap();
        let decoded = MltConverter::mlt_to_mvt(mlt.as_bytes()).ok().unwrap();

        assert_eq!(
            mvt_to_tile_layers(decoded.as_bytes()).unwrap(),
            mvt_to_tile_layers(MULTIPOLYGON).unwrap()
        );
    }

    #[test]
    fn v2_tessellated_encoding_without_outlines_decodes_a_polygon_as_triangles() {
        let mut options = MltEncoderOptions::new();
        options.set_wire_version(MltWireVersion::V02);
        options.set_tessellate(true);
        options.set_allow_triangles_only(true);

        let mlt = MltConverter::mvt_to_mlt(POLYGON, &options).ok().unwrap();
        let decoded = MltConverter::mlt_to_mvt(mlt.as_bytes()).ok().unwrap();

        assert_debug_snapshot!(mvt_to_tile_layers(decoded.as_bytes()).unwrap(), @r#"
        [
            TileLayer {
                name: "layer",
                extent: Extent(
                    4096,
                ),
                property_names: [
                    "key",
                ],
                property_kinds: [
                    Bool,
                ],
                m_value_names: [],
                m_value_kinds: [],
                nested_names: [],
                nested_kinds: [],
                z_step: None,
                features: [
                    TileFeature {
                        id: Some(
                            1,
                        ),
                        geometry: POLYGON((8 12,20 34,3 6,8 12)),
                        properties: [
                            Bool(
                                Some(
                                    true,
                                ),
                            ),
                        ],
                        m_values: [],
                        nested: [],
                        z: [],
                    },
                ],
            },
        ]
        "#);
    }

    #[test]
    fn builder_point_layer_with_every_property_kind_round_trips() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let flag = builder.add_property("flag");
        let count = builder.add_property("count");
        let ratio = builder.add_property("ratio");
        let precise = builder.add_property("precise");
        let name = builder.add_property("name");
        add_point(&mut builder, [8, 12], Some(7));
        builder.set_bool(flag, true).ok().unwrap();
        builder.set_i64(count, -3).ok().unwrap();
        builder.set_f32(ratio, 1.5).ok().unwrap();
        builder.set_f64(precise, 2.25).ok().unwrap();
        builder.set_str(name, "alpha").ok().unwrap();

        let mut out = MltBuffer::new();
        builder
            .encode_into(&MltEncoderOptions::new(), &mut out)
            .ok()
            .unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(layers.len(), 1);

        assert_debug_snapshot!(layers[0].features(), @r#"
        [
            TileFeature {
                id: Some(
                    7,
                ),
                geometry: POINT(8 12),
                properties: [
                    Bool(
                        Some(
                            true,
                        ),
                    ),
                    I64(
                        Some(
                            -3,
                        ),
                    ),
                    F32(
                        Some(
                            1.5,
                        ),
                    ),
                    F64(
                        Some(
                            2.25,
                        ),
                    ),
                    Str(
                        Some(
                            "alpha",
                        ),
                    ),
                ],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        "#);
    }

    #[test]
    fn builder_reset_rejects_an_empty_layer_name() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let error = builder.reset("", 4096).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
    }

    #[test]
    fn builder_reset_rejects_a_zero_extent() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let error = builder.reset("layer", 0).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
    }

    #[test]
    fn builder_layers_appended_to_one_buffer_decode_as_one_tile() {
        let mut out = MltBuffer::new();
        let options = MltEncoderOptions::new();
        let mut builder = MltLayerBuilder::new("first", 4096).ok().unwrap();
        add_point(&mut builder, [1, 1], None);
        builder.encode_into(&options, &mut out).ok().unwrap();
        builder.reset("second", 4096).ok().unwrap();
        add_point(&mut builder, [2, 2], None);
        builder.encode_into(&options, &mut out).ok().unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let names: Vec<_> = mvt_to_tile_layers(mvt.as_bytes())
            .unwrap()
            .into_iter()
            .map(|layer| layer.name().to_owned())
            .collect();

        assert_eq!(names, ["first", "second"]);
    }

    fn encode_fifty_points(options: &MltEncoderOptions) -> Vec<u8> {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        for i in 0..50_i32 {
            add_point(&mut builder, [i, 2 * i], Some(u64::from(i.unsigned_abs())));
            builder.set_i64(key, i64::from(i)).ok().unwrap();
        }
        let mut out = MltBuffer::new();
        builder.encode_into(options, &mut out).ok().unwrap();
        out.as_bytes().to_vec()
    }

    #[test]
    fn encoding_on_two_threads_with_separate_options_matches_encoding_on_one() {
        let v1_options = MltEncoderOptions::new();
        let mut v2_options = MltEncoderOptions::new();
        v2_options.set_wire_version(MltWireVersion::V02);
        let expected = [
            encode_fifty_points(&v1_options),
            encode_fifty_points(&v2_options),
        ];

        let actual = std::thread::scope(|scope| {
            let v1 = scope.spawn(|| encode_fifty_points(&v1_options));
            let v2 = scope.spawn(|| encode_fifty_points(&v2_options));
            [v1.join().unwrap(), v2.join().unwrap()]
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn clearing_a_buffer_empties_it() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        add_point(&mut builder, [1, 1], None);
        let mut out = MltBuffer::new();
        builder
            .encode_into(&MltEncoderOptions::new(), &mut out)
            .ok()
            .unwrap();

        out.clear();

        assert_eq!(out.len(), 0);
    }

    #[test]
    fn setting_a_property_before_a_feature_begins_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let key = builder.add_property("key");

        let error = builder.set_i64(key, 1).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""no feature begun""#);
    }

    #[test]
    fn setting_an_undeclared_property_key_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        add_point(&mut builder, [1, 1], None);

        let error = builder.set_i64(3, 1).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""unknown property key 3""#);
    }

    #[test]
    fn integer_and_float_values_of_one_key_widen_to_a_float_column() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        add_point(&mut builder, [1, 1], None);
        builder.set_i64(key, 3).ok().unwrap();
        add_point(&mut builder, [2, 2], None);
        builder.set_f32(key, 1.5).ok().unwrap();

        let mut out = MltBuffer::new();
        builder
            .encode_into(&MltEncoderOptions::new(), &mut out)
            .ok()
            .unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(layers.len(), 1);

        assert_debug_snapshot!(layers[0].features(), @"
        [
            TileFeature {
                id: None,
                geometry: POINT(1 1),
                properties: [
                    F64(
                        Some(
                            3.0,
                        ),
                    ),
                ],
                m_values: [],
                nested: [],
                z: [],
            },
            TileFeature {
                id: None,
                geometry: POINT(2 2),
                properties: [
                    F64(
                        Some(
                            1.5,
                        ),
                    ),
                ],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        ");
    }

    #[test]
    fn boolean_and_integer_values_of_one_key_become_text() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        add_point(&mut builder, [1, 1], None);
        builder.set_bool(key, true).ok().unwrap();
        add_point(&mut builder, [2, 2], None);
        builder.set_i64(key, 5).ok().unwrap();

        let mut out = MltBuffer::new();
        builder
            .encode_into(&MltEncoderOptions::new(), &mut out)
            .ok()
            .unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(layers.len(), 1);

        assert_debug_snapshot!(layers[0].features(), @r#"
        [
            TileFeature {
                id: None,
                geometry: POINT(1 1),
                properties: [
                    Str(
                        Some(
                            "true",
                        ),
                    ),
                ],
                m_values: [],
                nested: [],
                z: [],
            },
            TileFeature {
                id: None,
                geometry: POINT(2 2),
                properties: [
                    Str(
                        Some(
                            "5",
                        ),
                    ),
                ],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        "#);
    }

    fn add_point(builder: &mut MltLayerBuilder, [x, y]: [i32; 2], id: Option<u64>) {
        builder
            .begin_mvt_feature(MltMvtGeometryType::Point, &[9, zigzag(x), zigzag(y)], id)
            .ok()
            .unwrap();
    }

    fn zigzag(value: i32) -> u32 {
        ((value << 1) ^ (value >> 31)).cast_unsigned()
    }

    fn mvt_commands(parts: &[&[i32]], close: bool) -> Vec<u32> {
        let mut commands = Vec::new();
        let (mut x, mut y) = (0, 0);
        for part in parts {
            for (i, xy) in part.chunks(2).enumerate() {
                match i {
                    0 => commands.push(9),
                    1 => commands.push(2 | (u32::try_from(part.len() / 2 - 1).unwrap() << 3)),
                    _ => {}
                }
                commands.extend([zigzag(xy[0] - x), zigzag(xy[1] - y)]);
                (x, y) = (xy[0], xy[1]);
            }
            if close {
                commands.push(15);
            }
        }
        commands
    }

    fn decoded_features(builder: &MltLayerBuilder) -> Vec<mlt_core::TileFeature> {
        decoded_features_with(builder, &MltEncoderOptions::new())
    }

    fn decoded_features_with(
        builder: &MltLayerBuilder,
        options: &MltEncoderOptions,
    ) -> Vec<mlt_core::TileFeature> {
        let mut out = MltBuffer::new();
        builder.encode_into(options, &mut out).ok().unwrap();
        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let mut layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(layers.len(), 1);
        layers.remove(0).features().to_vec()
    }

    #[test]
    fn mvt_point_feature_matches_mvt_conversion_of_the_same_layer() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        builder
            .begin_mvt_feature(MltMvtGeometryType::Point, &[9, 50, 34], Some(1))
            .ok()
            .unwrap();
        builder.set_bool(key, true).ok().unwrap();

        let from_mvt = MltConverter::mvt_to_mlt(POINT, &MltEncoderOptions::new())
            .ok()
            .unwrap();
        let mvt = MltConverter::mlt_to_mvt(from_mvt.as_bytes()).ok().unwrap();
        let mvt_layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(decoded_features(&builder), mvt_layers[0].features());
    }

    #[test]
    fn v2_mvt_multipolygon_with_a_hole_splits_rings_by_winding() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let commands = mvt_commands(
            &[
                &[0, 0, 100, 0, 100, 100, 0, 100],
                &[20, 20, 20, 40, 40, 40, 40, 20],
                &[200, 200, 300, 200, 300, 300, 200, 300],
            ],
            true,
        );
        builder
            .begin_mvt_feature(MltMvtGeometryType::Polygon, &commands, None)
            .ok()
            .unwrap();
        let mut options = MltEncoderOptions::new();
        options.set_wire_version(MltWireVersion::V02);

        assert_debug_snapshot!(decoded_features_with(&builder, &options), @"
        [
            TileFeature {
                id: None,
                geometry: MULTIPOLYGON(((0 0,100 0,100 100,0 100,0 0),(20 20,20 40,40 40,40 20,20 20)),((200 200,300 200,300 300,200 300,200 200))),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        ");
    }

    #[test]
    fn mvt_features_take_the_single_or_multi_type_from_their_part_count() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let mvt = MltMvtGeometryType::LineString;
        builder
            .begin_mvt_feature(mvt, &mvt_commands(&[&[0, 0, 10, 10, 20, 5]], false), None)
            .ok()
            .unwrap();
        let lines = mvt_commands(&[&[1, 1, 2, 2], &[5, 5, 6, 7]], false);
        builder.begin_mvt_feature(mvt, &lines, None).ok().unwrap();
        builder
            .begin_mvt_feature(
                MltMvtGeometryType::Polygon,
                &mvt_commands(&[&[0, 0, 9, 0, 9, 9]], true),
                None,
            )
            .ok()
            .unwrap();

        assert_debug_snapshot!(decoded_features(&builder), @"
        [
            TileFeature {
                id: None,
                geometry: LINESTRING(0 0,10 10,20 5),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
            TileFeature {
                id: None,
                geometry: MULTILINESTRING((1 1,2 2),(5 5,6 7)),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
            TileFeature {
                id: None,
                geometry: POLYGON((0 0,9 0,9 9,0 0)),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        ");
    }

    #[test]
    fn mvt_points_of_one_move_to_become_a_multipoint() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder
            .begin_mvt_feature(MltMvtGeometryType::Point, &[25, 2, 4, 4, 4, 4, 4], None)
            .ok()
            .unwrap();

        assert_debug_snapshot!(decoded_features(&builder), @"
        [
            TileFeature {
                id: None,
                geometry: MULTIPOINT(1 2,3 4,5 6),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        ");
    }

    #[test]
    fn truncated_mvt_commands_are_invalid_and_leave_no_feature_behind() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder
            .begin_mvt_feature(MltMvtGeometryType::Point, &[9, 2, 2], None)
            .ok()
            .unwrap();

        let error = builder
            .begin_mvt_feature(MltMvtGeometryType::LineString, &[9, 2, 2, 18, 4], None)
            .err()
            .unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""invalid MVT geometry: ends inside a command""#);
        assert_eq!(decoded_features(&builder).len(), 1);
    }

    #[test]
    fn line_to_in_a_point_geometry_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let error = builder
            .begin_mvt_feature(MltMvtGeometryType::Point, &[9, 2, 2, 10, 4, 4], None)
            .err()
            .unwrap();

        assert_debug_snapshot!(error.message_text(), @r#""invalid MVT geometry: a LineTo outside a line or ring""#);
    }

    #[test]
    fn empty_mvt_commands_are_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let error = builder
            .begin_mvt_feature(MltMvtGeometryType::Polygon, &[], None)
            .err()
            .unwrap();

        assert_debug_snapshot!(error.message_text(), @r#""invalid MVT geometry: no rings""#);
    }

    #[test]
    fn declaring_a_property_name_again_returns_its_key() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let first = builder.add_property("name");
        builder.add_property("other");

        assert_eq!(builder.add_property("name"), first);
    }

    #[test]
    fn reset_forgets_the_declared_property_names() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.add_property("first");
        builder.add_property("second");
        builder.reset("layer", 4096).ok().unwrap();

        assert_eq!(builder.add_property("second"), 0);
    }

    #[test]
    fn decoding_with_a_tiny_memory_limit_reports_the_budget() {
        let mlt = MltConverter::mvt_to_mlt(POINT, &MltEncoderOptions::new())
            .ok()
            .unwrap();

        let error = MltConverter::mlt_to_mvt_with_limit(mlt.as_bytes(), 1)
            .err()
            .unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidInput);
        assert_debug_snapshot!(error.message_text(), @r#""memory limit exceeded: limit=1, used=0, requested=8""#);
    }

    #[test]
    fn decoding_with_a_generous_memory_limit_matches_the_default() {
        let mlt = MltConverter::mvt_to_mlt(POINT, &MltEncoderOptions::new())
            .ok()
            .unwrap();

        let limited = MltConverter::mlt_to_mvt_with_limit(mlt.as_bytes(), 1 << 30)
            .ok()
            .unwrap();

        assert_eq!(
            limited.as_bytes(),
            MltConverter::mlt_to_mvt(mlt.as_bytes())
                .ok()
                .unwrap()
                .as_bytes()
        );
    }

    #[test]
    fn decoding_garbage_as_mlt_reports_invalid_input_with_the_parser_message() {
        let error = MltConverter::mlt_to_mvt(&[0xff]).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidInput);
        assert_debug_snapshot!(error.message_text(), @r#""buffer underflow: needed 2 bytes, but only 1 remain""#);
    }

    #[test]
    fn encoding_garbage_as_mvt_reports_encoding_failed_with_the_parser_message() {
        let error = MltConverter::mvt_to_mlt(&[0xff], &MltEncoderOptions::new())
            .err()
            .unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::EncodingFailed);
        assert_debug_snapshot!(error.message_text(), @r#""MVT error: protobuf decode error: unexpected end of buffer""#);
    }
}
