use byteorder::{ByteOrder, LittleEndian};

use std::vec::Vec;
use crate::decoding::bitpacking_decoder::BitpackingDecoder;

pub const DEFAULT_PAGE_SIZE: usize = 65536;
pub const BLOCK_SIZE: usize = 256;


pub struct FastPFOR {
    pub(crate) page_size: usize,
    pub(crate) data_tobe_packed: Vec<Vec<u32>>,
    pub(crate) byte_container: Vec<u8>,
    pub(crate) data_pointers: Vec<usize>,
}

impl FastPFOR {
    pub fn new(page_size: usize) -> FastPFOR {
        let mut data_tobe_packed = vec![Vec::new(); 33]; // Init with 33 empty vectors
        for k in 1..data_tobe_packed.len() {
            data_tobe_packed[k] = vec![0; page_size / 32 * 4]; // statistic
        }

        let byte_container = vec![0; 3 * page_size / BLOCK_SIZE + page_size];

        FastPFOR {
            page_size,
            data_tobe_packed,
            byte_container,
            data_pointers: vec![0; 33],
        }
    }

    pub fn decode(&mut self, input: &[u32], output: &mut Vec<u32>) {
        self.decompress(input, &mut 0, output, &mut 0);
    }

    fn decompress(&mut self, input: &[u32], in_pos: &mut usize, out: &mut [u32], out_pos: &mut usize) {
        if input.is_empty() { return; }
        let out_length = input[*in_pos] as usize;
        *in_pos += 1;
        self.headless_decompress(input, in_pos, out, out_pos, out_length);
    }
    fn headless_decompress(&mut self, input: &[u32], in_pos: &mut usize, out: &mut [u32], out_pos: &mut usize, my_n_value: usize) {
        let my_n_value = greatest_multiple(my_n_value, BLOCK_SIZE);
        let final_out = *out_pos + my_n_value;
        while *out_pos != final_out {
            let this_size = core::cmp::min(self.page_size, final_out - *out_pos);
            self.decode_page(input, in_pos, out, out_pos, this_size);
        }
    }

    /// Decodes a compressed page of integers using the FastPFOR algorithm.
    ///
    /// # Arguments
    /// * `input` - The compressed input data as a slice of u32
    /// * `in_pos` - Mutable reference to the current position in the input slice
    /// * `out` - The output buffer where decoded integers will be written
    /// * `out_pos` - Mutable reference to the current position in the output buffer
    /// * `this_size` - The number of integers to decode in this page
    ///
    /// # Details
    /// This function implements the decoding step of the FastPFOR (Fast PatchedFrame-of-Reference) 
    /// compression algorithm. It handles both the regular packed values and exception values, 
    /// using a bitmap to track which bit lengths are used for exceptions.
    fn decode_page(&mut self, input: &[u32], in_pos: &mut usize, out: &mut [u32], out_pos: &mut usize, this_size: usize) {
        // Calculate required metadata
        let init_pos = *in_pos;
        let where_meta = input[*in_pos];
        *in_pos += 1;
        let mut in_except = init_pos + where_meta as usize;
        let byte_size = input[in_except] as usize;
        in_except += 1;

        // Setup aligned ByteContainer
        self.byte_container.clear();
        self.byte_container.resize((byte_size + 3) / 4 * 4, 0); // Ensure byte alignment
        LittleEndian::write_u32_into(
            &input[in_except..(in_except + (byte_size + 3) / 4)], 
            &mut self.byte_container[..]);
        in_except += (byte_size + 3) / 4;

        let bitmap = input[in_except];
        in_except += 1;

        // Perform numbers iteration
        for k in 2..=32 {
            if bitmap & (1 << (k - 1)) != 0 {
                let size = input[in_except] as usize;
                in_except += 1;

                let rounded_up = greatest_multiple(size + 31, 32);
                if self.data_tobe_packed[k].len() < rounded_up {
                    self.data_tobe_packed[k] = vec![0; rounded_up];
                }

                if in_except + rounded_up / 32 * k <= input.len() {
                    let mut j = 0;
                    while j < size {
                        BitpackingDecoder::fastunpack(
                            &input[in_except..], 
                            &mut self.data_tobe_packed[k][j..], 
                            k as u8);
                        in_except += k;
                        j += 32;
                    }
                    let overflow = j - size;
                    in_except -= overflow * k / 32;
                } else {
                    let mut data_buffer = vec![0; rounded_up / 32 * k];
                    let init_in_except = in_except;
                    let buff_size = data_buffer.len();
                    data_buffer.copy_from_slice(&input[in_except..in_except + buff_size]);
                    let mut j = 0;
                    while j < size {
                        BitpackingDecoder::fastunpack(
                            &data_buffer[in_except - init_in_except..], 
                            &mut self.data_tobe_packed[k][j..], 
                            k as u8);
                        in_except += k;
                        j += 32;
                    }
                    let overflow = j - size;
                    in_except -= overflow * k / 32;
                }
            }
        }

        self.data_pointers.fill(0);
        let mut tmp_out_pos = *out_pos;
        let mut tmp_in_pos = *in_pos;

        let mut container_index = 0;

        // Perform exception iteration
        for _run in 0..(this_size / BLOCK_SIZE) {
            let b = self.byte_container[container_index];
            container_index += 1;
            let cexcept = self.byte_container[container_index] as usize;
            container_index += 1;

            for k in (0..BLOCK_SIZE).step_by(32) {
                BitpackingDecoder::fastunpack(&input[tmp_in_pos..], &mut out[tmp_out_pos + k..], b);
                tmp_in_pos += b as usize;
            }

            // Decode exception on value
            if cexcept > 0 {
                let max_bits = self.byte_container[container_index] as usize;
                container_index += 1;
                let index = max_bits - b as usize;

                if index == 1 {
                    for _ in 0..cexcept {
                        let pos = self.byte_container[container_index] as usize;
                        container_index += 1;
                        out[tmp_out_pos + pos] |= 1 << b as u32;
                    }
                } else {
                    for _ in 0..cexcept {
                        let pos = self.byte_container[container_index] as usize;
                        container_index += 1;
                        let except_value = self.data_tobe_packed[index][self.data_pointers[index]];
                        self.data_pointers[index] += 1;
                        out[tmp_out_pos + pos] |= except_value << b as u32;
                    }
                }
            }

            tmp_out_pos += BLOCK_SIZE;
        }

        *out_pos = tmp_out_pos;
        *in_pos = in_except;
    }

}
impl Default for FastPFOR {
    fn default() -> FastPFOR {
        Self::new(DEFAULT_PAGE_SIZE)
    }
}


#[inline]
pub fn greatest_multiple(value: usize, factor: usize) -> usize {
    value - value % factor
}
