//! A [`LayerWriter`] holding the features of a [`TileLayer`] encodes to the same MLT and MVT
//! bytes.

use std::fs;
use std::path::Path;

use geo_types::{Geometry, Polygon};
use mlt_core::encoder::EncoderConfig;
use mlt_core::fast_mvt::MvtTileBuilder;
use mlt_core::mvt::{mvt_to_tile_layers, tile_layers_to_mvt};
use mlt_core::{FeatureWriter, GeometryType, LayerWriter, MltResult, TileLayer};
use test_each_file::test_each_path;

test_each_path! { for ["mvt"] in "../test/fixtures" as fixtures => writer_encodes_like_tile_layer }

fn writer_encodes_like_tile_layer([path]: [&Path; 1]) {
    let layers = mvt_to_tile_layers(fs::read(path).expect("read fixture")).expect("decode fixture");
    let base = EncoderConfig::default();
    let sorted = base
        .with_spatial_morton_sort(true)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true);
    for layer in layers {
        let writer = write(&layer).expect("write layer");
        for cfg in [base, sorted, base.with_tessellation(true)] {
            assert_eq!(
                writer.encode(cfg).expect("encode writer"),
                layer.encode(cfg).expect("encode layer"),
                "{}: layer {}",
                path.display(),
                layer.name()
            );
        }
        let mvt = |r: MltResult<Vec<u8>>| r.map_err(|e| e.to_string());
        assert_eq!(
            mvt(writer
                .write_mvt(MvtTileBuilder::new())
                .map(MvtTileBuilder::encode)),
            mvt(tile_layers_to_mvt([&layer])),
            "{}: MVT layer {}",
            path.display(),
            layer.name()
        );
    }
}

/// The features of `layer`, written one at a time.
fn write(layer: &TileLayer) -> MltResult<LayerWriter<'_>> {
    let mut writer = LayerWriter::new(layer.name(), layer.extent().get())?;
    let keys = (layer.property_names().iter().zip(layer.property_kinds()))
        .map(|(name, &kind)| writer.add_property(name, kind))
        .collect::<MltResult<Vec<_>>>()?;
    for feature in layer.features() {
        let geometry = feature.geometry();
        let geometry_type = GeometryType::try_from(geometry).expect("a single geometry kind");
        let mut out = writer.feature(geometry_type);
        out.id(feature.id());
        match geometry {
            Geometry::Point(p) => {
                out.points([p.0])?;
            }
            Geometry::MultiPoint(mp) => {
                out.points(mp.0.iter().map(|p| p.0))?;
            }
            Geometry::LineString(ls) => {
                out.line(ls.0.iter().copied())?;
            }
            Geometry::MultiLineString(mls) => {
                for ls in &mls.0 {
                    out.line(ls.0.iter().copied())?;
                }
            }
            Geometry::Polygon(p) => rings(&mut out, p)?,
            Geometry::MultiPolygon(mp) => {
                for p in &mp.0 {
                    rings(&mut out, p)?;
                }
            }
            Geometry::Line(_)
            | Geometry::GeometryCollection(_)
            | Geometry::Rect(_)
            | Geometry::Triangle(_) => {}
        }
        for (&key, value) in keys.iter().zip(feature.properties()) {
            if let Some(value) = value.value_ref() {
                out.property(key, value)?;
            }
        }
        out.finish()?;
    }
    Ok(writer)
}

fn rings(out: &mut FeatureWriter<'_, '_>, polygon: &Polygon<i32>) -> MltResult<()> {
    out.exterior_ring(polygon.exterior().0.iter().copied())?;
    for hole in polygon.interiors() {
        out.hole(hole.0.iter().copied())?;
    }
    Ok(())
}
