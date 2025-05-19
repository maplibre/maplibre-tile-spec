use bitvec::prelude::*;

pub struct Vector {
    pub name: String,
    pub data_buffer: Vec<i32>,
    pub nullability_buffer: Option<BitVec<u8, Lsb0>>,
    pub size: usize,
}
