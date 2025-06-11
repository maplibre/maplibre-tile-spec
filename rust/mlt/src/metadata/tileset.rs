use std::fs;
use std::io::Read;
use std::path::Path;

use prost::Message;

use crate::metadata::proto_tileset::TileSetMetadata;
use crate::{MltError, MltResult};

// Future: Impl for TileSetMetadata
pub fn read_metadata(path: &Path) -> MltResult<TileSetMetadata> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut buf = buffer.as_slice();
    TileSetMetadata::decode(&mut buf)
        .map_err(|e| MltError::MetadataDecodeError(format!("Invalid metadata structure: {e}")))
}

#[test]
fn test_read_metadata_invalid_file() {
    let invalid_path = Path::new("non_existent_file.pbf");
    let result = read_metadata(invalid_path);
    assert!(
        result.is_err(),
        "Expected read() to return an error for an invalid file"
    );
}

#[test]
fn test_read_mlt_file() {
    let mlt_path = Path::new("../../test/expected/omt/2_2_2.mlt");
    let result = read_metadata(mlt_path);
    assert!(
        result.is_err(),
        "Expected read() to return a valid TileSetMetadata"
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use super::*;

    #[test]
    fn test_read_metadata() {
        let metadata_path = Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf");
        let metadata = read_metadata(metadata_path).unwrap();

        let expected: HashSet<String> = ["boundary", "water_name", "landcover", "place", "water"]
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
