use crate::MltError;

pub trait AutomaticOptimisation {
    type UsedEncoder;
    /// Fully Automatic Layer Encoder
    ///
    /// Performs an extensive search over all available encoders and their parametrs.
    /// Selects the best encoder based on the data in the layer.
    /// The selected encoder is returned to be used to build a profile what options to use for encoding.
    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError>;
}
