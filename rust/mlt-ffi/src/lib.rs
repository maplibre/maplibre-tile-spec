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

    /// Which stage of a conversion failed.
    #[derive(Debug, PartialEq, Eq)]
    pub enum ConvertErrorKind {
        /// Input bytes could not be parsed or decoded.
        InvalidInput,
        /// Encoding failed.
        EncodingFailed,
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

    /// Owned byte buffer returned from conversion functions.
    ///
    /// The caller borrows the contents via [`as_bytes`](MltBuffer::as_bytes)
    /// and the buffer is freed when the handle is dropped.
    #[diplomat::opaque]
    pub struct MltBuffer(Vec<u8>);

    impl MltBuffer {
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

    /// Stateless FFI entry-points for MLT <-> MVT conversion.
    #[diplomat::opaque]
    pub struct MltConverter;

    impl MltConverter {
        /// Decode MLT bytes into MVT bytes.
        pub fn mlt_to_mvt(mlt: &[u8]) -> Result<Box<MltBuffer>, Box<ConvertError>> {
            let layers = Parser::default()
                .parse_layers(mlt)
                .map_err(|e| ConvertError::new(ConvertErrorKind::InvalidInput, &e))?;
            let mut dec = Decoder::default();
            let mut tiles = Vec::new();
            for layer in layers {
                let tile = layer
                    .into_tile(&mut dec)
                    .map_err(|e| ConvertError::new(ConvertErrorKind::InvalidInput, &e))?;
                tiles.extend(tile);
            }
            let out = tile_layers_to_mvt(tiles)
                .map_err(|e| ConvertError::new(ConvertErrorKind::InvalidInput, &e))?;
            Ok(Box::new(MltBuffer(out)))
        }

        /// Encode MVT bytes into MLT bytes using the given encoder options.
        pub fn mvt_to_mlt(
            mvt: &[u8],
            options: &MltEncoderOptions,
        ) -> Result<Box<MltBuffer>, Box<ConvertError>> {
            let mut out = Vec::new();
            let layers = mvt_to_tile_layers(mvt)
                .map_err(|e| ConvertError::new(ConvertErrorKind::EncodingFailed, &e))?;
            for tile in layers {
                let encoded_tile = tile
                    .encode(options.0)
                    .map_err(|e| ConvertError::new(ConvertErrorKind::EncodingFailed, &e))?;
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

    use super::ffi::{ConvertErrorKind, MltConverter, MltEncoderOptions, MltWireVersion};

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
