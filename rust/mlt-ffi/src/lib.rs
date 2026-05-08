use mlt_core::encoder::EncoderConfig;
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{Decoder, Layer, MltError, Parser};

fn mlt_to_mvt_inner(mlt: &[u8]) -> Result<Vec<u8>, MltError> {
    let layers = Parser::default().parse_layers(mlt)?;
    let mut dec = Decoder::default();
    let mut tiles = Vec::new();
    for layer in layers {
        if let Layer::Tag01(l) = layer {
            tiles.push(l.into_tile(&mut dec)?);
        }
    }
    tile_layers_to_mvt(tiles)
}

fn mvt_to_mlt_inner(mvt: &[u8], cfg: EncoderConfig) -> Result<Vec<u8>, MltError> {
    let mut out = Vec::new();
    for tile in mvt_to_tile_layers(mvt.to_vec())? {
        out.extend_from_slice(&tile.encode(cfg)?);
    }
    Ok(out)
}

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
    use super::{mlt_to_mvt_inner, mvt_to_mlt_inner};
    use mlt_core::encoder::EncoderConfig;

    /// Error type returned by FFI conversion functions.
    #[diplomat::attr(auto, error)]
    pub enum ConvertError {
        /// Input bytes could not be parsed or decoded.
        InvalidInput,
        /// Encoding failed.
        EncodingFailed,
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

    /// Encoder options controlling which optimisations are attempted for
    /// MVT → MLT conversion.
    ///
    /// Construct with [`new`](MltEncoderOptions::new) (all optimisations
    /// enabled except tessellation) and toggle individual flags with the
    /// setter methods.
    #[diplomat::opaque_mut]
    pub struct MltEncoderOptions(EncoderConfig);

    impl MltEncoderOptions {
        /// Create encoder options with the default configuration (all
        /// optimisations enabled except tessellation).
        #[diplomat::attr(auto, constructor)]
        pub fn new() -> Box<MltEncoderOptions> {
            Box::new(MltEncoderOptions(EncoderConfig::default()))
        }

        /// Generate tessellation data for polygons and multi-polygons.
        pub fn set_tessellate(&mut self, enabled: bool) {
            self.0.tessellate = enabled;
        }

        /// Try sorting features by the Z-order (Morton) curve index.
        pub fn set_try_spatial_morton_sort(&mut self, enabled: bool) {
            self.0.try_spatial_morton_sort = enabled;
        }

        /// Try sorting features by the Hilbert curve index.
        pub fn set_try_spatial_hilbert_sort(&mut self, enabled: bool) {
            self.0.try_spatial_hilbert_sort = enabled;
        }

        /// Try sorting features by their feature ID in ascending order.
        pub fn set_try_id_sort(&mut self, enabled: bool) {
            self.0.try_id_sort = enabled;
        }

        /// Allow FSST string compression.
        pub fn set_allow_fsst(&mut self, enabled: bool) {
            self.0.allow_fsst = enabled;
        }

        /// Allow `FastPFOR` integer compression.
        pub fn set_allow_fpf(&mut self, enabled: bool) {
            self.0.allow_fpf = enabled;
        }

        /// Allow string grouping into shared dictionaries.
        pub fn set_allow_shared_dict(&mut self, enabled: bool) {
            self.0.allow_shared_dict = enabled;
        }
    }

    /// Stateless FFI entry-points for MLT ↔ MVT conversion.
    #[diplomat::opaque]
    pub struct MltConverter;

    impl MltConverter {
        /// Decode MLT bytes into MVT bytes.
        pub fn mlt_to_mvt(mlt: &[u8]) -> Result<Box<MltBuffer>, ConvertError> {
            mlt_to_mvt_inner(mlt)
                .map(|v| Box::new(MltBuffer(v)))
                .map_err(|_| ConvertError::InvalidInput)
        }

        /// Encode MVT bytes into MLT bytes using the given encoder options.
        pub fn mvt_to_mlt(
            mvt: &[u8],
            options: &MltEncoderOptions,
        ) -> Result<Box<MltBuffer>, ConvertError> {
            mvt_to_mlt_inner(mvt, options.0)
                .map(|v| Box::new(MltBuffer(v)))
                .map_err(|_| ConvertError::EncodingFailed)
        }
    }
}
