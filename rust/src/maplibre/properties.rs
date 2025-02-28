
// todo: make this better :/

use core::fmt::Debug;
use serde::{Serialize, Serializer};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    bool(bool),
    i8(i8),
    i16(i16),
    i32(i32),
    i64(i64),
    u8(u8),
    u16(u16),
    u32(u32),
    u64(u64),
    f32(f32),
    f64(f64),
    String(String),
}
impl Serialize for Property {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ok(match self {
            Property::bool(value) => { serializer.serialize_bool(*value)? }
            Property::i8(value) => { serializer.serialize_i8(*value)? }
            Property::i16(value) => { serializer.serialize_i16(*value)? }
            Property::i32(value) => { serializer.serialize_i32(*value)? }
            Property::i64(value) => { serializer.serialize_i64(*value)? }
            Property::u8(value) => { serializer.serialize_u8(*value)? }
            Property::u16(value) => { serializer.serialize_u16(*value)? }
            Property::u32(value) => { serializer.serialize_u32(*value)? }
            Property::u64(value) => { serializer.serialize_u64(*value)? }
            Property::f32(value) => { serializer.serialize_f32(*value)? }
            Property::f64(value) => { serializer.serialize_f64(*value)? }
            Property::String(value) => { serializer.serialize_str(value)? }
        })
    }
}
