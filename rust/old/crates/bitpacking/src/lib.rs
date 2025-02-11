#![no_std]
extern crate alloc;

mod fastpack;
mod fastunpack;
mod fastpackwithoutmask;

use fastpack::fastpack as _fastpack;
use fastunpack::fastunpack as _fastunpack;
use fastpackwithoutmask::fastpackwithoutmask as _fastpackwithoutmask;


pub struct BitPacking {}
impl BitPacking {
    pub fn encode(input: &[u32], output: &mut [u32], bit: u8) {
        _fastpack(input, 0, output, 0, bit);
    }
    pub fn decode(input: &[u32], output: &mut [u32], bit: u8) {
        _fastunpack(input, 0, output, 0, bit);
    }
    
    pub fn fastpack(input: &[u32], output: &mut [u32], bit: u8) {
        _fastpack(input, 0, output, 0, bit);
    }
    pub fn fastunpack(input: &[u32], output: &mut [u32], bit: u8) {
        _fastunpack(input, 0, output, 0, bit);
    }
    pub fn fastpackwithoutmask(input: &[u32], output: &mut [u32], bit: u8) {
        _fastpackwithoutmask(input, 0, output, 0, bit);
    }
}
