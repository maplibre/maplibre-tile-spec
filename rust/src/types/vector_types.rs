#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum VectorType {
    FLAT,
    CONST,
    FREQUENCY,
    REE,
    DICTIONARY,
}
impl From<u8> for VectorType {
    fn from(value: u8) -> Self {
        match value {
            0 => VectorType::FLAT,
            1 => VectorType::CONST,
            2 => VectorType::FREQUENCY,
            3 => VectorType::REE,
            4 => VectorType::DICTIONARY,
            _ => panic!("Invalid VectoryType ({})", value),
        }
    }
}
