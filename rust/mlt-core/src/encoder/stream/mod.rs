mod encode_stream;
pub(crate) use encode_stream::dedup_strings;
pub mod encoder;
mod model;
mod optimizer;
mod physical;
#[cfg(test)]
mod tests;

pub use encoder::{FsstStrEncoder, IntEncoder};
pub use model::*;
pub use optimizer::DataProfile;
pub use physical::*;
