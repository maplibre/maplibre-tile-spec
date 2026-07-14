//! Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes,
//! delegating wire-format details to the [`fast_mvt`] crate.

use fast_mvt::{MvtTileBuilder, MvtValue};

use crate::decoder::{PropValue, TileLayer};
use crate::{MltError, MltResult};

/// Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes.
pub fn tile_layers_to_mvt(layers: Vec<TileLayer>) -> MltResult<Vec<u8>> {
    let mut tile = MvtTileBuilder::with_capacity(layers.len());
    for layer in layers {
        if layer.name.is_empty() {
            return Err(MltError::MissingLayerName);
        }
        let mut mvt_layer = tile.layer_with_capacity(layer.name, layer.features.len())?;
        mvt_layer.extent(layer.extent.into());
        for feat in layer.features {
            let mut feature = mvt_layer.feature(&feat.geometry)?;
            feature.id(feat.id);
            for (col_idx, prop) in feat.properties.into_iter().enumerate() {
                if let Some(name) = layer.property_names.get(col_idx)
                    && let Ok(value) = MvtValue::try_from(prop)
                {
                    feature.tag(name, value)?;
                }
            }
            mvt_layer = feature.end();
        }
        tile = mvt_layer.end();
    }
    Ok(tile.encode())
}

impl TryFrom<PropValue> for MvtValue {
    type Error = ();

    fn try_from(prop: PropValue) -> Result<Self, Self::Error> {
        Ok(match prop {
            PropValue::Bool(Some(b)) => Self::Bool(b),
            PropValue::I8(Some(i)) => Self::SInt(i.into()),
            PropValue::U8(Some(u)) => Self::UInt(u.into()),
            PropValue::I32(Some(i)) => Self::SInt(i.into()),
            PropValue::U32(Some(u)) => Self::UInt(u.into()),
            PropValue::I64(Some(i)) => Self::SInt(i),
            PropValue::U64(Some(u)) => Self::UInt(u),
            PropValue::F32(Some(f)) => Self::Float(f),
            PropValue::F64(Some(f)) => Self::Double(f),
            PropValue::Str(Some(s)) => Self::String(s),
            _ => Err(())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{Extent, TileFeature};
    use crate::mvt::mvt_to_tile_layers;

    #[test]
    fn empty_input_yields_empty_output() {
        let bytes = tile_layers_to_mvt(Vec::new()).unwrap();
        let decoded = mvt_to_tile_layers(bytes).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn rejects_empty_layer_name() {
        let layer = TileLayer {
            name: String::new(),
            extent: Extent::new(4096).unwrap(),
            property_names: vec![],
            property_kinds: vec![],
            features: vec![],
        };

        assert!(matches!(
            tile_layers_to_mvt(vec![layer]),
            Err(MltError::MissingLayerName)
        ));
    }

    /// `ClosePath` repeats the first vertex; an input with the closing
    /// duplicate must therefore round-trip without growing extra vertices.
    #[test]
    fn ring_is_implicitly_closed() {
        use geo_types::{Geometry, LineString, Polygon};
        let ring = vec![
            (0_i32, 0_i32).into(),
            (10, 0).into(),
            (10, 10).into(),
            (0, 10).into(),
            (0, 0).into(),
        ];
        let layer = TileLayer::from_parts(
            "L",
            4096,
            vec![],
            vec![TileFeature {
                id: Some(1),
                geometry: Geometry::Polygon(Polygon::new(LineString(ring), vec![])),
                properties: vec![],
            }],
        )
        .unwrap();
        let bytes = tile_layers_to_mvt(vec![layer]).unwrap();
        let back = mvt_to_tile_layers(bytes).unwrap();
        let Geometry::Polygon(p) = back[0].features()[0].geometry() else {
            panic!(
                "expected polygon, got {:?}",
                back[0].features()[0].geometry()
            );
        };
        assert_eq!(p.exterior().0.len(), 5);
        assert_eq!(p.exterior().0.first(), p.exterior().0.last());
    }
}
