#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VectorType {
    Flat,
    Const,
    Sequence,
    Dictionary,
    FsstDictionary,
}
