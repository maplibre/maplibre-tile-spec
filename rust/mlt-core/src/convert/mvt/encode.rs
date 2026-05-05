//! Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes,
//! delegating wire-format details to the [`mvt`] crate.

use geo_types::{Coord, Geometry, Polygon};
use mvt::{GeomEncoder, GeomType, Tile};

use crate::decoder::{PropValue, TileLayer};
use crate::{MltError, MltResult};

const DEFAULT_EXTENT: u32 = 4096;

/// Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes.
///
/// All input layers must share the same `extent` — MVT writes a single
/// tile-level extent, and [`mvt::Tile::add_layer`] rejects mismatches.
pub fn tile_layers_to_mvt(layers: Vec<TileLayer>) -> MltResult<Vec<u8>> {
    let extent = layers.first().map_or(DEFAULT_EXTENT, |l| l.extent);
    let mut tile = Tile::new(extent);
    for layer in layers {
        let built = build_layer(&tile, layer)?;
        tile.add_layer(built).map_err(|e| mvt_err(&e))?;
    }
    tile.to_bytes().map_err(|e| mvt_err(&e))
}

fn mvt_err(e: &mvt::Error) -> MltError {
    MltError::MvtParse(e.to_string())
}

fn build_layer(tile: &Tile, layer: TileLayer) -> MltResult<mvt::Layer> {
    let mut mvt_layer = tile.create_layer(&layer.name);
    let names = layer.property_names;
    for feat in layer.features {
        let geom_data = encode_geometry(&feat.geometry)?;
        let mut feature = mvt_layer.into_feature(geom_data);
        if let Some(id) = feat.id {
            feature.set_id(id);
        }
        for (col_idx, prop) in feat.properties.into_iter().enumerate() {
            let Some(name) = names.get(col_idx) else {
                continue;
            };
            add_tag(&mut feature, name, prop);
        }
        mvt_layer = feature.into_layer();
    }
    Ok(mvt_layer)
}

fn add_tag(feature: &mut mvt::Feature, key: &str, prop: PropValue) {
    match prop {
        PropValue::Bool(Some(b)) => feature.add_tag_bool(key, b),
        PropValue::I8(Some(i)) => feature.add_tag_sint(key, i.into()),
        PropValue::U8(Some(u)) => feature.add_tag_uint(key, u.into()),
        PropValue::I32(Some(i)) => feature.add_tag_sint(key, i.into()),
        PropValue::U32(Some(u)) => feature.add_tag_uint(key, u.into()),
        PropValue::I64(Some(i)) => feature.add_tag_sint(key, i),
        PropValue::U64(Some(u)) => feature.add_tag_uint(key, u),
        PropValue::F32(Some(f)) => feature.add_tag_float(key, f),
        PropValue::F64(Some(f)) => feature.add_tag_double(key, f),
        PropValue::Str(Some(s)) => feature.add_tag_string(key, &s),
        _ => {}
    }
}

fn encode_geometry(g: &Geometry<i32>) -> MltResult<mvt::GeomData> {
    let mut enc = match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => GeomEncoder::<f64>::new(GeomType::Point),
        Geometry::LineString(_) | Geometry::MultiLineString(_) => {
            GeomEncoder::<f64>::new(GeomType::Linestring)
        }
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {
            GeomEncoder::<f64>::new(GeomType::Polygon)
        }
        _ => {
            return Err(MltError::BadMvtGeometry(
                "unsupported geometry variant for MVT encoding",
            ));
        }
    };
    match g {
        Geometry::Point(p) => add_point(&mut enc, p.0)?,
        Geometry::MultiPoint(mp) => {
            for pt in &mp.0 {
                add_point(&mut enc, pt.0)?;
            }
        }
        Geometry::LineString(ls) => add_ring(&mut enc, &ls.0, false)?,
        Geometry::MultiLineString(mls) => {
            for (i, ls) in mls.0.iter().enumerate() {
                add_ring(&mut enc, &ls.0, false)?;
                if i + 1 < mls.0.len() {
                    enc.complete_geom().map_err(|e| mvt_err(&e))?;
                }
            }
        }
        Geometry::Polygon(poly) => add_polygon(&mut enc, poly)?,
        Geometry::MultiPolygon(mp) => {
            for (i, poly) in mp.0.iter().enumerate() {
                add_polygon(&mut enc, poly)?;
                if i + 1 < mp.0.len() {
                    enc.complete_geom().map_err(|e| mvt_err(&e))?;
                }
            }
        }
        _ => unreachable!("validated above"),
    }
    enc.encode().map_err(|e| mvt_err(&e))
}

fn add_point(enc: &mut GeomEncoder<f64>, c: Coord<i32>) -> MltResult<()> {
    enc.add_point(c.x.into(), c.y.into()).map_err(|e| mvt_err(&e))
}

/// Add a single ring's vertices, dropping the trailing closing duplicate so the
/// `mvt` encoder's implicit `ClosePath` doesn't double-emit it.
fn add_ring(enc: &mut GeomEncoder<f64>, coords: &[Coord<i32>], close: bool) -> MltResult<()> {
    let pts = if close { unclose(coords) } else { coords };
    for &c in pts {
        add_point(enc, c)?;
    }
    Ok(())
}

fn add_polygon(enc: &mut GeomEncoder<f64>, poly: &Polygon<i32>) -> MltResult<()> {
    let rings: Vec<&[Coord<i32>]> = std::iter::once(poly.exterior().0.as_slice())
        .chain(poly.interiors().iter().map(|r| r.0.as_slice()))
        .collect();
    for (i, ring) in rings.iter().enumerate() {
        add_ring(enc, ring, true)?;
        if i + 1 < rings.len() {
            enc.complete_geom().map_err(|e| mvt_err(&e))?;
        }
    }
    Ok(())
}

/// Drop a ring's trailing closing vertex if it duplicates the first.
fn unclose(ring: &[Coord<i32>]) -> &[Coord<i32>] {
    if ring.len() >= 2 && ring[0] == ring[ring.len() - 1] {
        &ring[..ring.len() - 1]
    } else {
        ring
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::TileFeature;
    use crate::mvt::mvt_to_tile_layers;

    #[test]
    fn empty_input_yields_empty_output() {
        let bytes = tile_layers_to_mvt(Vec::new()).unwrap();
        let decoded = mvt_to_tile_layers(bytes).unwrap();
        assert!(decoded.is_empty());
    }

    /// `mvt-reader` surfaces every polygon as a `MultiPolygon`, and decodes
    /// `ClosePath` by repeating the first vertex; an input with the closing
    /// duplicate must therefore round-trip without growing extra vertices.
    #[test]
    fn ring_is_implicitly_closed() {
        use geo_types::{LineString, Polygon};
        let ring = vec![
            (0_i32, 0_i32).into(),
            (10, 0).into(),
            (10, 10).into(),
            (0, 10).into(),
            (0, 0).into(),
        ];
        let layer = TileLayer {
            name: "L".into(),
            extent: 4096,
            property_names: vec![],
            features: vec![TileFeature {
                id: Some(1),
                geometry: Geometry::Polygon(Polygon::new(LineString(ring), vec![])),
                properties: vec![],
            }],
        };
        let bytes = tile_layers_to_mvt(vec![layer]).unwrap();
        let back = mvt_to_tile_layers(bytes).unwrap();
        let Geometry::MultiPolygon(mp) = &back[0].features[0].geometry else {
            panic!(
                "expected multipolygon, got {:?}",
                back[0].features[0].geometry
            );
        };
        let p = &mp.0[0];
        assert_eq!(p.exterior().0.len(), 5);
        assert_eq!(p.exterior().0.first(), p.exterior().0.last());
    }
}
