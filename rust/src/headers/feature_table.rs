use crate::proto::TileSetMetadata;

#[derive(Debug)]
#[allow(dead_code)]
// Size = 17
pub struct FeatureTableMetadata {
    version: u8,
    id: u32,
    layer_extent: u32,
    max_layer_extent: u32,
    num_features: u32,
}
impl FeatureTableMetadata {
    pub fn load(data: &[u8], offset: &mut usize, schemas: &TileSetMetadata) -> Self {
        todo!()























        // let fields = Vec::new();
        // 
        // let mut infos = [0; 4];
        // *offset += VarInt::decode_varint(&data[1..], 4, &mut infos);
        // 
        // let current_schema = schemas.featureTables.get(infos[0] as usize).unwrap();
        // 
        // for column in current_schema.columns.iter() {
        //     let field = FieldMetadata::load(data, offset, column);
        //     // TODO: continue
        // }
        // 
        // Self {
        //     version:            data[0],
        //     id:                 infos[0],
        //     layer_extent:       infos[1],
        //     max_layer_extent:   infos[2],
        //     num_features:       infos[3],
        //     fields,
        // }
    }
}
