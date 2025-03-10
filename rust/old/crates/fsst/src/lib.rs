#![no_std]
extern crate alloc;

use alloc::vec::Vec;

pub fn encode() {
    unimplemented!("FSST encoding ist not part of the projects scope")
}

pub fn decode(symbols: &[u8], symbol_lengths: &[usize], input: &[u8], output: &mut Vec<u8>) {
    // TODO: add output preallocation
    let mut symbol_offsets: Vec<usize> = Vec::with_capacity(symbol_lengths.len());

    for i in 1..symbol_lengths.len() {
        symbol_offsets[i] = symbol_offsets[i - 1] + symbol_lengths[i - 1];
    }

    for (i, &byte) in input.iter().enumerate() {
        match byte {
            0xFF => output.push(input[i + 1]),
            _ => {
                let symbol_length = symbol_lengths.get(input[i] as usize).unwrap();
                let symbol_offset = symbol_offsets.get(input[i] as usize).unwrap();

                for j in 0..*symbol_length {
                    output.push(symbols[*symbol_offset + j]);
                }
            }
        }
    }
}
