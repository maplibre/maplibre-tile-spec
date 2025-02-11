
#![allow(non_snake_case)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LogicalStreamType {
    pub DictionaryType: Option<StreamType_Dictionary>,
    pub OffsetType: Option<StreamType_OffsetIntoDictionary>,
    pub LengthType: Option<StreamType_VariableSizedItems>,
}
impl LogicalStreamType {
    pub fn new_none() -> Self {
        Self { DictionaryType: None, OffsetType: None, LengthType: None }
    }
    pub fn new_dictionary(dict: StreamType_Dictionary) -> Self {
        Self { DictionaryType: Some(dict), OffsetType: None, LengthType: None }
    }
    pub fn new_offset(offset: StreamType_OffsetIntoDictionary) -> Self {
        Self { DictionaryType: None, OffsetType: Some(offset), LengthType: None }
    }
    pub fn new_length(length: StreamType_VariableSizedItems) -> Self {
        Self { DictionaryType: None, OffsetType: None, LengthType: Some(length) }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum StreamType_Dictionary {
    NONE,
    SINGLE,
    SHARED,
    VERTEX,
    MORTON,
    FSST,
}
impl From<u8> for StreamType_Dictionary {
    fn from(value: u8) -> Self {
        match value {
            0 => StreamType_Dictionary::NONE,
            1 => StreamType_Dictionary::SINGLE,
            2 => StreamType_Dictionary::SHARED,
            3 => StreamType_Dictionary::VERTEX,
            4 => StreamType_Dictionary::MORTON,
            5 => StreamType_Dictionary::FSST,
            _ => panic!("Invalid StreamType_Dictionary ({})", value),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(C)]
pub enum StreamType_OffsetIntoDictionary {
    VERTEX,
    INDEX,
    STRING,
    KEY,
}
impl From<u8> for StreamType_OffsetIntoDictionary {
    fn from(value: u8) -> Self {
        match value {
            0 => StreamType_OffsetIntoDictionary::VERTEX,
            1 => StreamType_OffsetIntoDictionary::INDEX,
            2 => StreamType_OffsetIntoDictionary::STRING,
            3 => StreamType_OffsetIntoDictionary::KEY,
            _ => panic!("Invalid StreamType_OffsetIntoDictionary ({})", value),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(C)]
pub enum StreamType_VariableSizedItems {
    VAR_BINARY,
    GEOMETRIES,
    PARTS,
    RINGS,
    TRIANGLES,
    SYMBOL,
    DICTIONARY,
}
impl From<u8> for StreamType_VariableSizedItems {
    fn from(value: u8) -> Self {
        match value {
            0 => StreamType_VariableSizedItems::VAR_BINARY,
            1 => StreamType_VariableSizedItems::GEOMETRIES,
            2 => StreamType_VariableSizedItems::PARTS,
            3 => StreamType_VariableSizedItems::RINGS,
            4 => StreamType_VariableSizedItems::TRIANGLES,
            5 => StreamType_VariableSizedItems::SYMBOL,
            6 => StreamType_VariableSizedItems::DICTIONARY,
            _ => panic!("Invalid StreamType_VariableSizedItems ({})", value),
        }
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum PhysicalStreamType {
    PRESENT,
    DATA,
    OFFSET,
    LENGTH,
}
impl From<u8> for PhysicalStreamType {
    fn from(value: u8) -> Self {
        match value {
            0 => PhysicalStreamType::PRESENT,
            1 => PhysicalStreamType::DATA,
            2 => PhysicalStreamType::OFFSET,
            3 => PhysicalStreamType::LENGTH,
            _ => panic!("Invalid PhysicalStreamType ({})", value),
        }
    }
}
