/// Describes how the vertex buffer should be encoded.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, strum::IntoStaticStr)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum VertexBufferType {
    /// Standard 2D `(x, y)` pairs encoded with componentwise delta.
    #[default]
    Vec2,
    /// Morton (Z-order) dictionary encoding:
    /// Unique vertices are sorted by their Morton code and stored once.
    /// Each vertex position in the stream is replaced by its index into that dictionary.
    /// Only v1 can write it.
    Morton,
    /// Hilbert curve dictionary encoding:
    /// Unique vertices are sorted by their Hilbert curve index and stored once,
    /// componentwise-delta encoded. Each vertex position in the stream is replaced
    /// by its index into that dictionary.
    /// Only v1 can write it.
    Hilbert,
    /// Vertices as residuals against a guess from the decoded vertices, over `(x, y)` pairs or `(x, y, z)` triples.
    /// Only v2 can write it.
    Predicted,
}
