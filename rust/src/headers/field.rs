use crate::proto::Column;
use crate::types::vector_types::VectorType;

#[derive(Debug)]
#[allow(dead_code)]
// Size = 5
pub struct FieldMetadata {
    num_streams: u32,
    vector_type: VectorType,
}
impl FieldMetadata {
    pub fn load(data: &[u8], offset: &mut usize, column: &Column) -> Self {
        todo!()
    }
}
