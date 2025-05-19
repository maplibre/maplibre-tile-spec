use bitvec::prelude::*;

pub struct IntFlatVector {
    pub name: String,
    pub data_buffer: Vec<i32>,
    pub nullability_buffer: Option<BitVec<u8, Lsb0>>,
}
