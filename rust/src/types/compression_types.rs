#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LogicalLevelCompressionTechnique {
    NONE,
    DELTA,
    COMPONENTWISE_DELTA,
    RLE,
    MORTON,
    PDE,
}
impl From<u8> for LogicalLevelCompressionTechnique {
    fn from(value: u8) -> Self {
        match value {
            0 => LogicalLevelCompressionTechnique::NONE,
            1 => LogicalLevelCompressionTechnique::DELTA,
            2 => LogicalLevelCompressionTechnique::COMPONENTWISE_DELTA,
            3 => LogicalLevelCompressionTechnique::RLE,
            4 => LogicalLevelCompressionTechnique::MORTON,
            5 => LogicalLevelCompressionTechnique::PDE,
            _ => panic!("Invalid LogicalLevelCompressionTechnique ({})", value),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum PhysicalLevelCompressionTechnique {
    NONE,
    FAST_PFOR,
    VARINT,
    ALP,
}
impl From<u8> for PhysicalLevelCompressionTechnique {
    fn from(value: u8) -> Self {
        match value {
            0 => PhysicalLevelCompressionTechnique::NONE,
            1 => PhysicalLevelCompressionTechnique::FAST_PFOR,
            2 => PhysicalLevelCompressionTechnique::VARINT,
            3 => PhysicalLevelCompressionTechnique::ALP,
            _ => panic!("Invalid PhysicalLevelCompressionTechnique ({})", value),
        }
    }
}