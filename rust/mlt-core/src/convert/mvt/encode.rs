//! Encode layers as MVT (Mapbox Vector Tile) bytes

use std::borrow::Borrow;

use fast_mvt::{MvtTileBuilder, MvtValueRef};

use crate::tile::TileLayer;
use crate::{MltError, MltResult, PropValueRef};

/// Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes.
pub fn tile_layers_to_mvt(
    layers: impl IntoIterator<Item = impl Borrow<TileLayer>>,
) -> MltResult<Vec<u8>> {
    let mut tile = MvtTileBuilder::new();
    for layer in layers {
        tile = write_mvt_layer(layer.borrow(), tile)?;
    }
    Ok(tile.encode())
}

/// Adds `source` to `tile`, writing each feature straight from the layer.
fn write_mvt_layer(source: &TileLayer, tile: MvtTileBuilder) -> MltResult<MvtTileBuilder> {
    if source.name().is_empty() {
        return Err(MltError::MissingLayerName);
    }
    let mut layer = tile.layer(source.name())?;
    layer.extent(source.extent().into());
    for feature in source.features() {
        let mut out = layer.feature(feature.geometry())?;
        out.id(feature.id());
        for (name, prop) in source.property_names().iter().zip(feature.properties()) {
            if let Some(value) = prop.value_ref() {
                out.tag_ref(name, value.into())?;
            }
        }
        layer = out.end();
    }
    Ok(layer.end())
}

impl<'a> From<PropValueRef<'a>> for MvtValueRef<'a> {
    fn from(value: PropValueRef<'a>) -> Self {
        match value {
            PropValueRef::Bool(v) => Self::Bool(v),
            PropValueRef::I8(v) => Self::SInt(v.into()),
            PropValueRef::U8(v) => Self::UInt(v.into()),
            PropValueRef::I32(v) => Self::SInt(v.into()),
            PropValueRef::U32(v) => Self::UInt(v.into()),
            PropValueRef::I64(v) => Self::SInt(v),
            PropValueRef::U64(v) => Self::UInt(v),
            PropValueRef::F32(v) => Self::Float(v),
            PropValueRef::F64(v) => Self::Double(v),
            PropValueRef::Str(v) => Self::String(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvt::mvt_to_tile_layers;
    use crate::tile::{Extent, TileFeature};

    #[test]
    fn empty_input_yields_empty_output() {
        let bytes = tile_layers_to_mvt(Vec::<TileLayer>::new()).unwrap();
        let decoded = mvt_to_tile_layers(bytes).unwrap();
        assert_eq!(decoded, [] as [TileLayer; 0]);
    }

    #[test]
    fn rejects_empty_layer_name() {
        let layer = TileLayer {
            name: String::new(),
            extent: Extent::new(4096).unwrap(),
            property_names: vec![],
            property_kinds: vec![],
            #[cfg(feature = "unstable-v2")]
            m_value_names: vec![],
            #[cfg(feature = "unstable-v2")]
            m_value_kinds: vec![],
            #[cfg(feature = "unstable-v2")]
            nested_names: vec![],
            #[cfg(feature = "unstable-v2")]
            nested_kinds: vec![],
            #[cfg(feature = "unstable-v2")]
            z_step: None,
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
                #[cfg(feature = "unstable-v2")]
                m_values: vec![],
                #[cfg(feature = "unstable-v2")]
                nested: vec![],
                #[cfg(feature = "unstable-v2")]
                z: Vec::new(),
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
