#[cfg(feature = "reader")]
use crate::DEFAULT_EXTENT;
pub use crate::generated::vector_tile::Tile;
pub use crate::generated::vector_tile::tile::{Feature, GeomType, Layer, Value};

#[cfg(feature = "reader")]
impl Tile {
    #[must_use]
    pub fn from_reader(reader: &crate::MvtReaderRef<'_>) -> Self {
        let mut tile = reader.to_proto();
        for layer in &mut tile.layers {
            layer.extent.get_or_insert(DEFAULT_EXTENT.get());
        }
        tile
    }
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use super::Tile;

    #[test]
    fn tile_deserializes_from_object_but_not_layers_array() {
        let object: Tile = serde_json::from_str(
            r#"{
                "layers": [{
                    "version": 2,
                    "name": "places",
                    "extent": 4096
                }]
            }"#,
        )
        .unwrap();

        let array = serde_json::from_str::<Tile>(
            r#"[{
                "version": 2,
                "name": "places",
                "extent": 4096
            }]"#,
        );

        assert!(array.is_err());
        assert_eq!(object.layers[0].name, "places");
    }
}
