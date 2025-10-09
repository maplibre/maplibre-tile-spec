use crate::structures::enums::{DictionaryType, LengthType, OffsetType};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PhysicalStreamType {
    Present,
    Data(DictionaryType),
    Offset(OffsetType),
    Length(LengthType),
}

impl PhysicalStreamType {
    pub fn from_u8(value: u8) -> Option<Self> {
        let high4 = value >> 4;
        let low4 = value & 0x0F;
        Some(match high4 {
            0 => PhysicalStreamType::Present,
            1 => PhysicalStreamType::Data(DictionaryType::try_from(low4).ok()?),
            2 => PhysicalStreamType::Offset(OffsetType::try_from(low4).ok()?),
            3 => PhysicalStreamType::Length(LengthType::try_from(low4).ok()?),
            _ => return None,
        })
    }
}

pub enum Decoder {
    None,
    Delta,
    DeltaRle {
        runs: u32,
        num_rle_values: u32,
    },
    ComponentwiseDelta,
    Rle {
        runs: u32,
        num_rle_values: u32,
    },
    Morton {
        num_bits: u32,
        coordinate_shift: u32,
    },
    PseudoDecimal,
}
