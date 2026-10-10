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
    use mlt_core::{Decoder, GeometryType, Parser};

    use crate::store::{LayerBuffer, PartRole, StoreError, StoredValue};

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

    fn add_part(
        buffer: &mut LayerBuffer,
        role: PartRole,
        xy: &[i32],
    ) -> Result<(), Box<ConvertError>> {
        buffer.add_part(role, xy).map_err(|e| feature_error(&e))
    }

    fn set_value(
        buffer: &mut LayerBuffer,
        key: u32,
        value: StoredValue,
    ) -> Result<(), Box<ConvertError>> {
        buffer.set(key, value).map_err(|e| feature_error(&e))
    }

    impl From<MltGeometryType> for GeometryType {
        fn from(geometry: MltGeometryType) -> Self {
            match geometry {
                MltGeometryType::Point => Self::Point,
                MltGeometryType::LineString => Self::LineString,
                MltGeometryType::Polygon => Self::Polygon,
                MltGeometryType::MultiPoint => Self::MultiPoint,
                MltGeometryType::MultiLineString => Self::MultiLineString,
                MltGeometryType::MultiPolygon => Self::MultiPolygon,
            }
        }
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

    /// The geometry type of one feature.
    pub enum MltGeometryType {
        Point,
        LineString,
        Polygon,
        MultiPoint,
        MultiLineString,
        MultiPolygon,
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
        pub fn add_property(&mut self, name: &str) -> u32 {
            self.0.add_property(name)
        }

        /// Start a feature, ending the previous one.
        pub fn begin_feature(&mut self, geometry: MltGeometryType, id: Option<u64>) {
            self.0.begin_feature(geometry.into(), id);
        }

        /// Add the points of a point or multi-point feature.
        pub fn add_points(&mut self, xy: &[i32]) -> Result<(), Box<ConvertError>> {
            add_part(&mut self.0, PartRole::Points, xy)
        }

        /// Add a line of a line or multi-line feature.
        pub fn add_line(&mut self, xy: &[i32]) -> Result<(), Box<ConvertError>> {
            add_part(&mut self.0, PartRole::Line, xy)
        }

        /// Start a polygon of a polygon or multi-polygon feature with its exterior ring.
        pub fn add_exterior_ring(&mut self, xy: &[i32]) -> Result<(), Box<ConvertError>> {
            add_part(&mut self.0, PartRole::Exterior, xy)
        }

        /// Add a hole to the polygon the last exterior ring started.
        pub fn add_hole(&mut self, xy: &[i32]) -> Result<(), Box<ConvertError>> {
            add_part(&mut self.0, PartRole::Hole, xy)
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
        ConvertErrorKind, MltBuffer, MltConverter, MltEncoderOptions, MltGeometryType,
        MltLayerBuilder, MltWireVersion,
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
        builder.begin_feature(MltGeometryType::Point, Some(7));
        builder.add_points(&[8, 12]).ok().unwrap();
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
    fn builder_multipolygon_with_a_hole_round_trips_in_v2() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::MultiPolygon, None);
        builder
            .add_exterior_ring(&[0, 0, 100, 0, 100, 100, 0, 100, 0, 0])
            .ok()
            .unwrap();
        builder
            .add_hole(&[20, 20, 20, 40, 40, 40, 40, 20, 20, 20])
            .ok()
            .unwrap();
        builder
            .add_exterior_ring(&[200, 200, 300, 200, 300, 300, 200, 300, 200, 200])
            .ok()
            .unwrap();
        let mut options = MltEncoderOptions::new();
        options.set_wire_version(MltWireVersion::V02);

        let mut out = MltBuffer::new();
        builder.encode_into(&options, &mut out).ok().unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let layers = mvt_to_tile_layers(mvt.as_bytes()).unwrap();
        assert_eq!(layers.len(), 1);

        assert_debug_snapshot!(layers[0].features(), @"
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
    fn builder_multiline_layer_round_trips_two_features() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::LineString, Some(1));
        builder.add_line(&[0, 0, 10, 10, 20, 5]).ok().unwrap();
        builder.begin_feature(MltGeometryType::MultiLineString, Some(2));
        builder.add_line(&[1, 1, 2, 2]).ok().unwrap();
        builder.add_line(&[5, 5, 6, 7, 8, 9]).ok().unwrap();

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
                id: Some(
                    1,
                ),
                geometry: LINESTRING(0 0,10 10,20 5),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
            TileFeature {
                id: Some(
                    2,
                ),
                geometry: MULTILINESTRING((1 1,2 2),(5 5,6 7,8 9)),
                properties: [],
                m_values: [],
                nested: [],
                z: [],
            },
        ]
        ");
    }

    #[test]
    fn builder_multipoint_layer_round_trips() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::MultiPoint, None);
        builder.add_points(&[1, 2, 3, 4, 5, 6]).ok().unwrap();

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
    fn builder_layers_appended_to_one_buffer_decode_as_one_tile() {
        let mut out = MltBuffer::new();
        let options = MltEncoderOptions::new();
        let mut builder = MltLayerBuilder::new("first", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[1, 1]).ok().unwrap();
        builder.encode_into(&options, &mut out).ok().unwrap();
        builder.reset("second", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[2, 2]).ok().unwrap();
        builder.encode_into(&options, &mut out).ok().unwrap();

        let mvt = MltConverter::mlt_to_mvt(out.as_bytes()).ok().unwrap();
        let names: Vec<_> = mvt_to_tile_layers(mvt.as_bytes())
            .unwrap()
            .into_iter()
            .map(|layer| layer.name().to_owned())
            .collect();

        assert_eq!(names, ["first", "second"]);
    }

    #[test]
    fn builder_matches_mvt_conversion_of_the_same_layer() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        builder.begin_feature(MltGeometryType::Point, Some(1));
        builder.add_points(&[25, 17]).ok().unwrap();
        builder.set_bool(key, true).ok().unwrap();
        let options = MltEncoderOptions::new();

        let mut from_builder = MltBuffer::new();
        builder
            .encode_into(&options, &mut from_builder)
            .ok()
            .unwrap();
        let from_mvt = MltConverter::mvt_to_mlt(POINT, &options).ok().unwrap();

        let builder_mvt = MltConverter::mlt_to_mvt(from_builder.as_bytes())
            .ok()
            .unwrap();
        let mvt_mvt = MltConverter::mlt_to_mvt(from_mvt.as_bytes()).ok().unwrap();
        let builder_layers = mvt_to_tile_layers(builder_mvt.as_bytes()).unwrap();
        let mvt_layers = mvt_to_tile_layers(mvt_mvt.as_bytes()).unwrap();
        assert_eq!(builder_layers.len(), 1);
        assert_eq!(mvt_layers.len(), 1);
        assert_eq!(builder_layers[0].features(), mvt_layers[0].features());
    }

    fn encode_fifty_points(options: &MltEncoderOptions) -> Vec<u8> {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        for i in 0..50_i32 {
            builder.begin_feature(MltGeometryType::Point, Some(u64::from(i.unsigned_abs())));
            builder.add_points(&[i, 2 * i]).ok().unwrap();
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
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[1, 1]).ok().unwrap();
        let mut out = MltBuffer::new();
        builder
            .encode_into(&MltEncoderOptions::new(), &mut out)
            .ok()
            .unwrap();

        out.clear();

        assert_eq!(out.len(), 0);
    }

    #[test]
    fn adding_geometry_before_a_feature_begins_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();

        let error = builder.add_points(&[1, 1]).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""no feature begun""#);
    }

    #[test]
    fn odd_coordinate_count_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::LineString, None);

        let error = builder.add_line(&[1, 2, 3]).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""coordinates must come in x, y pairs""#);
    }

    #[test]
    fn setting_an_undeclared_property_key_is_invalid() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);

        let error = builder.set_i64(3, 1).err().unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::InvalidFeature);
        assert_debug_snapshot!(error.message_text(), @r#""unknown property key 3""#);
    }

    #[test]
    fn integer_and_float_values_of_one_key_widen_to_a_float_column() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        let key = builder.add_property("key");
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[1, 1]).ok().unwrap();
        builder.set_i64(key, 3).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[2, 2]).ok().unwrap();
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
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[1, 1]).ok().unwrap();
        builder.set_bool(key, true).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_points(&[2, 2]).ok().unwrap();
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

    #[test]
    fn line_geometry_on_a_point_feature_fails_at_encode() {
        let mut builder = MltLayerBuilder::new("layer", 4096).ok().unwrap();
        builder.begin_feature(MltGeometryType::Point, None);
        builder.add_line(&[1, 1, 2, 2]).ok().unwrap();

        let error = builder
            .encode_into(&MltEncoderOptions::new(), &mut MltBuffer::new())
            .err()
            .unwrap();

        assert_eq!(error.kind(), ConvertErrorKind::EncodingFailed);
        assert_debug_snapshot!(error.message_text(), @r#""a Point feature lines""#);
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
