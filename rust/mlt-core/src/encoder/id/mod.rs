mod encode;
mod model;
mod optimizer;
#[cfg(test)]
mod tests;

/// Write the ID column with an explicit `id_width` and `int_enc` directly to `enc`.
///
/// This is the low-level inner function used by synthetics.  For automatic encoding,
/// prefer the [`IdValues::write_to`] method which tries multiple candidate encodings.
#[cfg(feature = "__private")]
pub use encode::write_id_to;
pub use model::*;
