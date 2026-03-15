mod analyze;
pub(super) mod decode;
pub(super) mod encode;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod geotype;
mod model;
mod optimizer;
mod serialize;

pub use encode::GeometryEncoder;
pub use model::*;
pub use optimizer::GeometryProfile;
