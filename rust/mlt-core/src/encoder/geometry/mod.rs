mod encode01;
#[cfg(feature = "unstable-v2")]
pub(crate) mod encode02;
mod geotype;
mod model;
mod streams;
#[cfg(test)]
mod tests;

pub(crate) use geotype::coord_count;
pub use model::*;
