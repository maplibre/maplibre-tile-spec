mod common;

use mlt::MltResult;
use mlt::{
    create_tileset_metadata,
    mvt::{decode_mvt, ColumnMapping},
    FeatureTableOptimizations,
};
use std::collections::HashMap;
use std::path::Path;

fn decode_bing_tiles() -> MltResult<()> {
    let tile_ids = vec![
        "4-8-5", "4-9-5", "4-12-6", "4-13-6", "5-16-11", "5-17-11", "5-17-10", "6-32-22",
        "6-33-22", "6-32-23", "6-32-21", "7-65-42", "7-66-42", "7-66-43", "7-66-44",
    ];

    for tile_id in tile_ids {
        let bing_mvt_path = Path::new("../../../../test/fixtures/bing");
        let result = test_tile(tile_id, bing_mvt_path, false);
    }

    Ok(())
}

fn test_tile(tile_id: &str, tile_directory: &Path, allow_sorting: bool) -> MltResult<()> {
    let mvt_file_path = tile_directory.join(format!("{}.mvt", tile_id));
    let mvt_bytes = std::fs::read(mvt_file_path)?;
    let mvt_tile = decode_mvt(&mvt_bytes);
    let column_mappings: Vec<ColumnMapping> = vec![];
    let tile_metadata = create_tileset_metadata(mvt_tile, true, Some(&column_mappings));
    let allow_id_regeneration = true;

    let optimizations = FeatureTableOptimizations {
        allow_sorting,
        allow_id_regeneration,
        column_mappings: Some(column_mappings),
    };

    let optimizations: HashMap<String, FeatureTableOptimizations> = common::OPTIMIZED_MVT_LAYERS
        .iter()
        .map(|layer| (layer.to_string(), optimizations.clone()))
        .collect();

    let include_ids = true;

    Ok(())
}
