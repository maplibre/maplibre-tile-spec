#![no_std]

#[macro_use]
extern crate alloc;
extern crate byteorder;

mod utils;

mod fastpfor256;

pub use fastpfor256::FastPFOR;
