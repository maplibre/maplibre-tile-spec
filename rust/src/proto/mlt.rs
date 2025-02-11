// Automatically generated rust module for 'mlt_tileset_metadata.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ColumnScope {
    FEATURE = 0,
    VERTEX = 1,
}

impl Default for ColumnScope {
    fn default() -> Self {
        ColumnScope::FEATURE
    }
}

impl From<i32> for ColumnScope {
    fn from(i: i32) -> Self {
        match i {
            0 => ColumnScope::FEATURE,
            1 => ColumnScope::VERTEX,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ColumnScope {
    fn from(s: &'a str) -> Self {
        match s {
            "FEATURE" => ColumnScope::FEATURE,
            "VERTEX" => ColumnScope::VERTEX,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ScalarType {
    BOOLEAN = 0,
    INT_8 = 1,
    UINT_8 = 2,
    INT_32 = 3,
    UINT_32 = 4,
    INT_64 = 5,
    UINT_64 = 6,
    FLOAT = 7,
    DOUBLE = 8,
    STRING = 9,
}

impl Default for ScalarType {
    fn default() -> Self {
        ScalarType::BOOLEAN
    }
}

impl From<i32> for ScalarType {
    fn from(i: i32) -> Self {
        match i {
            0 => ScalarType::BOOLEAN,
            1 => ScalarType::INT_8,
            2 => ScalarType::UINT_8,
            3 => ScalarType::INT_32,
            4 => ScalarType::UINT_32,
            5 => ScalarType::INT_64,
            6 => ScalarType::UINT_64,
            7 => ScalarType::FLOAT,
            8 => ScalarType::DOUBLE,
            9 => ScalarType::STRING,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ScalarType {
    fn from(s: &'a str) -> Self {
        match s {
            "BOOLEAN" => ScalarType::BOOLEAN,
            "INT_8" => ScalarType::INT_8,
            "UINT_8" => ScalarType::UINT_8,
            "INT_32" => ScalarType::INT_32,
            "UINT_32" => ScalarType::UINT_32,
            "INT_64" => ScalarType::INT_64,
            "UINT_64" => ScalarType::UINT_64,
            "FLOAT" => ScalarType::FLOAT,
            "DOUBLE" => ScalarType::DOUBLE,
            "STRING" => ScalarType::STRING,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComplexType {
    VEC_2 = 0,
    VEC_3 = 1,
    GEOMETRY = 2,
    GEOMETRY_Z = 3,
    LIST = 4,
    MAP = 5,
    STRUCT = 6,
}

impl Default for ComplexType {
    fn default() -> Self {
        ComplexType::VEC_2
    }
}

impl From<i32> for ComplexType {
    fn from(i: i32) -> Self {
        match i {
            0 => ComplexType::VEC_2,
            1 => ComplexType::VEC_3,
            2 => ComplexType::GEOMETRY,
            3 => ComplexType::GEOMETRY_Z,
            4 => ComplexType::LIST,
            5 => ComplexType::MAP,
            6 => ComplexType::STRUCT,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ComplexType {
    fn from(s: &'a str) -> Self {
        match s {
            "VEC_2" => ComplexType::VEC_2,
            "VEC_3" => ComplexType::VEC_3,
            "GEOMETRY" => ComplexType::GEOMETRY,
            "GEOMETRY_Z" => ComplexType::GEOMETRY_Z,
            "LIST" => ComplexType::LIST,
            "MAP" => ComplexType::MAP,
            "STRUCT" => ComplexType::STRUCT,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LogicalScalarType {
    TIMESTAMP = 0,
    DATE = 1,
    JSON = 2,
}

impl Default for LogicalScalarType {
    fn default() -> Self {
        LogicalScalarType::TIMESTAMP
    }
}

impl From<i32> for LogicalScalarType {
    fn from(i: i32) -> Self {
        match i {
            0 => LogicalScalarType::TIMESTAMP,
            1 => LogicalScalarType::DATE,
            2 => LogicalScalarType::JSON,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for LogicalScalarType {
    fn from(s: &'a str) -> Self {
        match s {
            "TIMESTAMP" => LogicalScalarType::TIMESTAMP,
            "DATE" => LogicalScalarType::DATE,
            "JSON" => LogicalScalarType::JSON,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LogicalComplexType {
    BINARY = 0,
    RANGE_MAP = 1,
}

impl Default for LogicalComplexType {
    fn default() -> Self {
        LogicalComplexType::BINARY
    }
}

impl From<i32> for LogicalComplexType {
    fn from(i: i32) -> Self {
        match i {
            0 => LogicalComplexType::BINARY,
            1 => LogicalComplexType::RANGE_MAP,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for LogicalComplexType {
    fn from(s: &'a str) -> Self {
        match s {
            "BINARY" => LogicalComplexType::BINARY,
            "RANGE_MAP" => LogicalComplexType::RANGE_MAP,
            _ => Self::default(),
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TileSetMetadata<'a> {
    pub version: i32,
    pub featureTables: Vec<FeatureTableSchema<'a>>,
    pub name: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub attribution: Cow<'a, str>,
    pub minZoom: i32,
    pub maxZoom: i32,
    pub bounds: Cow<'a, [f64]>,
    pub center: Cow<'a, [f64]>,
}

impl<'a> MessageRead<'a> for TileSetMetadata<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.version = r.read_int32(bytes)?,
                Ok(18) => msg.featureTables.push(r.read_message::<FeatureTableSchema>(bytes)?),
                Ok(26) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.description = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(42) => msg.attribution = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(48) => msg.minZoom = r.read_int32(bytes)?,
                Ok(56) => msg.maxZoom = r.read_int32(bytes)?,
                Ok(66) => msg.bounds = r.read_packed_fixed(bytes)?.into(),
                Ok(74) => msg.center = r.read_packed_fixed(bytes)?.into(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for TileSetMetadata<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.version != 0i32 { w.write_with_tag(8, |w| w.write_int32(*&self.version))?; }
        for s in &self.featureTables { w.write_with_tag(18, |w| w.write_message(s))?; }
        if self.name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.name))?; }
        if self.description != "" { w.write_with_tag(34, |w| w.write_string(&**&self.description))?; }
        if self.attribution != "" { w.write_with_tag(42, |w| w.write_string(&**&self.attribution))?; }
        if self.minZoom != 0i32 { w.write_with_tag(48, |w| w.write_int32(*&self.minZoom))?; }
        if self.maxZoom != 0i32 { w.write_with_tag(56, |w| w.write_int32(*&self.maxZoom))?; }
        w.write_packed_fixed_with_tag(66, &self.bounds)?;
        w.write_packed_fixed_with_tag(74, &self.center)?;
        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + if self.version == 0i32 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
        + self.featureTables.iter().map(|s| 1 + sizeof_len(s.get_size())).sum::<usize>()
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.description == "" { 0 } else { 1 + sizeof_len((&self.description).len()) }
        + if self.attribution == "" { 0 } else { 1 + sizeof_len((&self.attribution).len()) }
        + if self.minZoom == 0i32 { 0 } else { 1 + sizeof_varint(*(&self.minZoom) as u64) }
        + if self.maxZoom == 0i32 { 0 } else { 1 + sizeof_varint(*(&self.maxZoom) as u64) }
        + if self.bounds.is_empty() { 0 } else { 1 + sizeof_len(self.bounds.len() * 8) }
        + if self.center.is_empty() { 0 } else { 1 + sizeof_len(self.center.len() * 8) }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct FeatureTableSchema<'a> {
    pub name: Cow<'a, str>,
    pub columns: Vec<Column<'a>>,
}

impl<'a> MessageRead<'a> for FeatureTableSchema<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.columns.push(r.read_message::<Column>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for FeatureTableSchema<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.name != "" { w.write_with_tag(10, |w| w.write_string(&**&self.name))?; }
        for s in &self.columns { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + self.columns.iter().map(|s| 1 + sizeof_len(s.get_size())).sum::<usize>()
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub nullable: bool,
    pub columnScope: ColumnScope,
    pub type_pb: mod_Column::OneOftype_pb<'a>,
}

impl<'a> MessageRead<'a> for Column<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(16) => msg.nullable = r.read_bool(bytes)?,
                Ok(24) => msg.columnScope = r.read_enum(bytes)?,
                Ok(34) => msg.type_pb = mod_Column::OneOftype_pb::scalarType(r.read_message::<ScalarColumn>(bytes)?),
                Ok(42) => msg.type_pb = mod_Column::OneOftype_pb::complexType(r.read_message::<ComplexColumn>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Column<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.name != "" { w.write_with_tag(10, |w| w.write_string(&**&self.name))?; }
        if self.nullable != false { w.write_with_tag(16, |w| w.write_bool(*&self.nullable))?; }
        if self.columnScope != ColumnScope::FEATURE { w.write_with_tag(24, |w| w.write_enum(*&self.columnScope as i32))?; }
        match self.type_pb {            mod_Column::OneOftype_pb::scalarType(ref m) => { w.write_with_tag(34, |w| w.write_message(m))? },
            mod_Column::OneOftype_pb::complexType(ref m) => { w.write_with_tag(42, |w| w.write_message(m))? },
            mod_Column::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.nullable == false { 0 } else { 1 + sizeof_varint(*(&self.nullable) as u64) }
        + if self.columnScope == ColumnScope::FEATURE { 0 } else { 1 + sizeof_varint(*(&self.columnScope) as u64) }
        + match self.type_pb {
            mod_Column::OneOftype_pb::scalarType(ref m) => 1 + sizeof_len(m.get_size()),
            mod_Column::OneOftype_pb::complexType(ref m) => 1 + sizeof_len(m.get_size()),
            mod_Column::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_Column {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb<'a> {
    scalarType(ScalarColumn),
    complexType(ComplexColumn<'a>),
    None,
}

impl<'a> Default for OneOftype_pb<'a> {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ScalarColumn {
    pub type_pb: mod_ScalarColumn::OneOftype_pb,
}

impl<'a> MessageRead<'a> for ScalarColumn {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(32) => msg.type_pb = mod_ScalarColumn::OneOftype_pb::physicalType(r.read_enum(bytes)?),
                Ok(40) => msg.type_pb = mod_ScalarColumn::OneOftype_pb::logicalType(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ScalarColumn {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.type_pb {            mod_ScalarColumn::OneOftype_pb::physicalType(ref m) => { w.write_with_tag(32, |w| w.write_enum(*m as i32))? },
            mod_ScalarColumn::OneOftype_pb::logicalType(ref m) => { w.write_with_tag(40, |w| w.write_enum(*m as i32))? },
            mod_ScalarColumn::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + match self.type_pb {
            mod_ScalarColumn::OneOftype_pb::physicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ScalarColumn::OneOftype_pb::logicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ScalarColumn::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_ScalarColumn {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb {
    physicalType(ScalarType),
    logicalType(LogicalScalarType),
    None,
}

impl Default for OneOftype_pb {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ComplexColumn<'a> {
    pub children: Vec<Field<'a>>,
    pub type_pb: mod_ComplexColumn::OneOftype_pb,
}

impl<'a> MessageRead<'a> for ComplexColumn<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(50) => msg.children.push(r.read_message::<Field>(bytes)?),
                Ok(32) => msg.type_pb = mod_ComplexColumn::OneOftype_pb::physicalType(r.read_enum(bytes)?),
                Ok(40) => msg.type_pb = mod_ComplexColumn::OneOftype_pb::logicalType(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ComplexColumn<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.children { w.write_with_tag(50, |w| w.write_message(s))?; }
        match self.type_pb {            mod_ComplexColumn::OneOftype_pb::physicalType(ref m) => { w.write_with_tag(32, |w| w.write_enum(*m as i32))? },
            mod_ComplexColumn::OneOftype_pb::logicalType(ref m) => { w.write_with_tag(40, |w| w.write_enum(*m as i32))? },
            mod_ComplexColumn::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + self.children.iter().map(|s| 1 + sizeof_len(s.get_size())).sum::<usize>()
        + match self.type_pb {
            mod_ComplexColumn::OneOftype_pb::physicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ComplexColumn::OneOftype_pb::logicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ComplexColumn::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_ComplexColumn {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb {
    physicalType(ComplexType),
    logicalType(LogicalComplexType),
    None,
}

impl Default for OneOftype_pb {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Field<'a> {
    pub name: Cow<'a, str>,
    pub nullable: bool,
    pub type_pb: mod_Field::OneOftype_pb<'a>,
}

impl<'a> MessageRead<'a> for Field<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(16) => msg.nullable = r.read_bool(bytes)?,
                Ok(26) => msg.type_pb = mod_Field::OneOftype_pb::scalarField(r.read_message::<ScalarField>(bytes)?),
                Ok(34) => msg.type_pb = mod_Field::OneOftype_pb::complexField(r.read_message::<ComplexField>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Field<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.name != "" { w.write_with_tag(10, |w| w.write_string(&**&self.name))?; }
        if self.nullable != false { w.write_with_tag(16, |w| w.write_bool(*&self.nullable))?; }
        match self.type_pb {            mod_Field::OneOftype_pb::scalarField(ref m) => { w.write_with_tag(26, |w| w.write_message(m))? },
            mod_Field::OneOftype_pb::complexField(ref m) => { w.write_with_tag(34, |w| w.write_message(m))? },
            mod_Field::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.nullable == false { 0 } else { 1 + sizeof_varint(*(&self.nullable) as u64) }
        + match self.type_pb {
            mod_Field::OneOftype_pb::scalarField(ref m) => 1 + sizeof_len(m.get_size()),
            mod_Field::OneOftype_pb::complexField(ref m) => 1 + sizeof_len(m.get_size()),
            mod_Field::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_Field {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb<'a> {
    scalarField(ScalarField),
    complexField(ComplexField<'a>),
    None,
}

impl<'a> Default for OneOftype_pb<'a> {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ScalarField {
    pub type_pb: mod_ScalarField::OneOftype_pb,
}

impl<'a> MessageRead<'a> for ScalarField {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.type_pb = mod_ScalarField::OneOftype_pb::physicalType(r.read_enum(bytes)?),
                Ok(16) => msg.type_pb = mod_ScalarField::OneOftype_pb::logicalType(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ScalarField {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        match self.type_pb {            mod_ScalarField::OneOftype_pb::physicalType(ref m) => { w.write_with_tag(8, |w| w.write_enum(*m as i32))? },
            mod_ScalarField::OneOftype_pb::logicalType(ref m) => { w.write_with_tag(16, |w| w.write_enum(*m as i32))? },
            mod_ScalarField::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + match self.type_pb {
            mod_ScalarField::OneOftype_pb::physicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ScalarField::OneOftype_pb::logicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ScalarField::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_ScalarField {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb {
    physicalType(ScalarType),
    logicalType(LogicalScalarType),
    None,
}

impl Default for OneOftype_pb {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ComplexField<'a> {
    pub children: Vec<Field<'a>>,
    pub type_pb: mod_ComplexField::OneOftype_pb,
}

impl<'a> MessageRead<'a> for ComplexField<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(26) => msg.children.push(r.read_message::<Field>(bytes)?),
                Ok(8) => msg.type_pb = mod_ComplexField::OneOftype_pb::physicalType(r.read_enum(bytes)?),
                Ok(16) => msg.type_pb = mod_ComplexField::OneOftype_pb::logicalType(r.read_enum(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ComplexField<'a> {
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.children { w.write_with_tag(26, |w| w.write_message(s))?; }
        match self.type_pb {            mod_ComplexField::OneOftype_pb::physicalType(ref m) => { w.write_with_tag(8, |w| w.write_enum(*m as i32))? },
            mod_ComplexField::OneOftype_pb::logicalType(ref m) => { w.write_with_tag(16, |w| w.write_enum(*m as i32))? },
            mod_ComplexField::OneOftype_pb::None => {},
    }        Ok(())
    }

    fn get_size(&self) -> usize {
        0
        + self.children.iter().map(|s| 1 + sizeof_len(s.get_size())).sum::<usize>()
        + match self.type_pb {
            mod_ComplexField::OneOftype_pb::physicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ComplexField::OneOftype_pb::logicalType(ref m) => 1 + sizeof_varint(*(m) as u64),
            mod_ComplexField::OneOftype_pb::None => 0,
    }    }
}

pub mod mod_ComplexField {

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum OneOftype_pb {
    physicalType(ComplexType),
    logicalType(LogicalComplexType),
    None,
}

impl Default for OneOftype_pb {
    fn default() -> Self {
        OneOftype_pb::None
    }
}

}

