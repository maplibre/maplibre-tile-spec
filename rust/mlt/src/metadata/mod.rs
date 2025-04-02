pub mod proto_tileset;

use crate::TileSetMetadata;
use prost::Message;
use std::fs;
use std::io::Read;
use std::path::Path;
use crate::MltResult;

#[derive(Debug, Clone, Copy)]
pub enum PhysicalLevelTechnique {
    None,
    FastPfor,
    Varint,
    Alp,
}

// Future: Impl for TileSetMetadata
fn read(path: &Path) -> MltResult<TileSetMetadata> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut buf = buffer.as_slice();
    Ok(TileSetMetadata::decode(&mut buf)?)
}

#[test]
fn test_read_metadata_invalid_file() {
    let invalid_path = Path::new("non_existent_file.pbf");
    let result = read(invalid_path);
    assert!(result.is_err(), "Expected read() to return an error for an invalid file");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn test_read_metadata() {
        let metadata_path = Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf");
        let metadata = read(metadata_path).unwrap();

        let expected: HashSet<String> = [
            "boundary",
            "water_name",
            "landcover",
            "place",
            "water",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let actual: HashSet<String> = metadata
            .feature_tables
            .iter()
            .map(|columns| columns.name.clone())
            .collect();

        assert_eq!(actual, expected, "Feature table schema names do not match");
    }
}
