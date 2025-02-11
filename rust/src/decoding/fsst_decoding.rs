
pub struct Fsst {}
impl Fsst {

    /// Decodes FSST (Fast Static Symbol Table) compressed data back to its original form
    /// 
    /// # Arguments
    /// * `symbols` - The symbol dictionary containing all possible symbol values
    /// * `symbol_lengths` - Array of lengths for each symbol in the dictionary
    /// * `compressed_data` - The compressed input data to decode
    /// * `decompressed_length` - Expected length of decompressed output
    ///
    /// # Example
    /// ```
    /// let symbols = b"HelloWorld"; // Symbol dictionary
    /// let symbol_lengths = vec![5, 5]; // "Hello" and "World" are 5 chars each
    /// let compressed = vec![0, 1]; // Indexes into symbol dictionary
    /// let decompressed_len = 10;
    /// 
    /// let result = decode(symbols, &symbol_lengths, &compressed, decompressed_len);
    /// assert_eq!(result, b"HelloWorld");
    /// ```
    pub fn decode(
        symbols: &[u8],
        symbol_lengths: &[u32],
        compressed_data: &[u8],
        decompressed_length: usize,
    ) -> Vec<u8> {
        let mut output = Vec::with_capacity(decompressed_length);
        let mut symbol_offsets = vec![0; symbol_lengths.len()];

        for i in 1..symbol_lengths.len() {
            symbol_offsets[i] = symbol_offsets[i - 1] + symbol_lengths[i - 1];
        }

        let mut idx = 0;
        for &byte in compressed_data {
            let symbol_index = byte as usize;
            if symbol_index == 255 {
                if idx >= decompressed_length {
                    break;
                }
                output.push(compressed_data[idx + 1]);
                idx += 2;
            } else if symbol_index < symbol_lengths.len() {
                let len = symbol_lengths[symbol_index] as usize;
                let slice = &symbols[symbol_offsets[symbol_index] as usize..symbol_offsets[symbol_index] as usize + len];
                output.extend_from_slice(slice);
                idx += 1;
            }
        }

        output.resize(decompressed_length, 0);
        output
    }

    pub fn decode_unknown_length(symbols: &[u8], symbol_lengths: &[u32], compressed_data: &[u8]) -> Vec<u8> {
        let mut decoded_data = Vec::new();
        let mut symbol_offsets = vec![0; symbol_lengths.len()];

        for i in 1..symbol_lengths.len() {
            symbol_offsets[i] = symbol_offsets[i - 1] + symbol_lengths[i - 1];
        }

        let mut i = 0;
        while i < compressed_data.len() {
            let symbol_index = compressed_data[i] as usize;
            if symbol_index == 255 {
                if i + 1 < compressed_data.len() {
                    decoded_data.push(compressed_data[i + 1]);
                    i += 2;
                } else {
                    break;
                }
            } else if symbol_index < symbol_lengths.len() {
                let len = symbol_lengths[symbol_index] as usize;
                let slice = &symbols[symbol_offsets[symbol_index] as usize..symbol_offsets[symbol_index] as usize + len];
                decoded_data.extend_from_slice(slice);
                i += 1;
            }
        }

        decoded_data
    }
}
