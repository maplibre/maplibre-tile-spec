#![no_std]
#![allow(non_snake_case)]

extern crate alloc;

use alloc::vec::Vec;

mod bool;
mod u8;
mod u16;
mod u32;
mod f32;
mod f64;


pub fn encode_bool(input: &[bool], output: &mut [u8]) {
    let encoded = bool::encode_bool(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_bool(input: &[u8], output: &mut [bool]) {
    let encoded = bool::decode_bool(input);
    output.copy_from_slice(encoded.as_slice());
}

pub fn encode_u8(input: &[u8], output: &mut [u8]) {
    let encoded = u8::encode_u8(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_u8(input: &[u8], output: &mut [u8]) {
    let encoded = u8::decode_u8(input);
    output.copy_from_slice(encoded.as_slice());
}


pub fn encode_u16(input: &[u16], output: &mut [u8]) {
    let encoded = u16::encode_u16(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_u16(input: &[u8], output: &mut [u16]) {
    let encoded = u16::decode_u16(input);
    output.copy_from_slice(encoded.as_slice());
}

pub fn encode_u32(input: &[u32], output: &mut [u8]) {
    let encoded = u32::encode_u32(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_u32(input: &[u8], output: &mut [u32]) {
    let encoded = u32::decode_u32(input);
    output.copy_from_slice(encoded.as_slice());
}

pub fn encode_f32(input: &[f32], output: &mut [u8]) {
    let encoded = f32::encode_f32(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_f32(input: &[u8], output: &mut [f32]) {
    let encoded = f32::decode_f32(input);
    output.copy_from_slice(encoded.as_slice());
}

pub fn encode_f64(input: &[f64], output: &mut [u8]) {
    let encoded = f64::encode_f64(input);
    output.copy_from_slice(encoded.as_slice());
}
pub fn decode_f64(input: &[u8], output: &mut [f64]) {
    let encoded = f64::decode_f64(input);
    output.copy_from_slice(encoded.as_slice());
}
