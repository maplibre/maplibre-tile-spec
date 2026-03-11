use crate::MltError;

pub trait ManualOptimisation {
    type UsedEncoder;

    /// Applies a given encoder
    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError>;
}

pub trait ProfileOptimisation {
    type UsedEncoder;
    type Profile;
    /// Automatic Profile Optimisation
    ///
    /// Only searches over the given profile and its parameters.
    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError>;
}

pub trait AutomaticOptimisation {
    type UsedEncoder;
    /// Fully Automatic Layer Encoder
    ///
    /// Performs an extensive search over all available encoders and their parameters.
    /// Selects the best encoder based on the data in the layer.
    /// The selected encoder is returned to be used to build a profile what options to use for encoding.
    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError>;
}
