mod encode01;
#[cfg(feature = "unstable-v2")]
pub(crate) mod encode02;
mod geotype;
mod model;
mod streams;
#[cfg(test)]
mod tests;

#[cfg(feature = "unstable-v2")]
pub(crate) use geotype::stored_vertex_count;
pub use model::*;
