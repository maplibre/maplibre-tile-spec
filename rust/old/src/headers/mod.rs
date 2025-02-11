use log::info;
use varint::VarInt;
use crate::headers::feature_table::FeatureTableMetadata;
use crate::mlt::TileSetMetadata;

mod field;
mod stream;
mod feature_table;

pub struct MLT {
    feature_tables: Vec<FeatureTableMetadata>,
}
impl MLT {
    pub fn load(data: &[u8], metadata: &TileSetMetadata) -> Self {
        let mut offset = 0;
        let mut feature_tables = Vec::new();

        while offset < data.len() {
            let version = data[offset];
            offset += 1;

            let mut infos = Vec::new();
            offset += VarInt::decode_varint(&data[offset..], 4, &mut infos);

            let feature_tableid = infos[0];
            let tile_extent = infos[1];
            let max_tile_extent = infos[2];
            let num_features = infos[3];

            let metadata = metadata.featureTables.get(feature_tableid as usize).unwrap();
        }

        Self { feature_tables }
    }
}

