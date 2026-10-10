mod encode01;
#[cfg(feature = "unstable-v2")]
pub(crate) mod encode02;
mod geotype;
mod model;
mod streams;
#[cfg(test)]
mod tests;

pub(crate) use geotype::coord_count;
pub use geotype::wound_geometry;
#[cfg(feature = "unstable-v2")]
pub(crate) use geotype::wound_vertex_order;
pub use model::*;
