use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{
    BatchSize, BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main,
};
use fast_mvt::{
    DEFAULT_EXTENT, MvtCoord, MvtGeometry, MvtLineString, MvtPolygon, MvtReaderRef, MvtTile,
    MvtValue,
};
use geo_types::Geometry;
use mvt::{GeomEncoder, GeomType};

mod common;

use common::load_repo_mvt_files;

fn bench_encode(c: &mut Criterion) {
    let fixtures = load_repo_mvt_files();
    let tiles = fixtures
        .iter()
        .map(|data| {
            let tile = MvtReaderRef::new(data)
                .and_then(|reader| reader.to_tile())
                .expect("decode fixture");
            BenchTile {
                bytes: data.len(),
                tile,
            }
        })
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("mvt encode");
    bench_owned(&mut group, "fast-mvt encode", &tiles, |tile| {
        fast_mvt::encode_to_vec(tile).expect("fast-mvt encode")
    });
    bench_owned(&mut group, "mvt encode", &tiles, |tile| {
        encode_with_mvt(tile).expect("mvt encode")
    });

    let total_bytes = total_owned_bytes(&tiles);
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function(
        format!("fast-mvt encode into reused vec ({} tiles)", tiles.len()),
        |bench| {
            bench.iter_batched(
                Vec::new,
                |mut out| {
                    for tile in &tiles {
                        out.clear();
                        fast_mvt::encode(black_box(&tile.tile), black_box(&mut out))
                            .expect("fast-mvt encode reused");
                        black_box(&out);
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
    group.finish();
}

criterion_group!(benches, bench_encode);
criterion_main!(benches);

#[derive(Clone)]
struct BenchTile {
    bytes: usize,
    tile: MvtTile,
}

fn bench_owned<R>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    tiles: &[BenchTile],
    mut bench_fn: impl FnMut(&MvtTile) -> R,
) {
    group.throughput(Throughput::Bytes(total_owned_bytes(tiles) as u64));
    group.bench_function(format!("{name} ({} tiles)", tiles.len()), |bench| {
        bench.iter(|| {
            for tile in tiles {
                black_box(bench_fn(black_box(&tile.tile)));
            }
        });
    });
}

fn total_owned_bytes(tiles: &[BenchTile]) -> usize {
    tiles.iter().map(|tile| tile.bytes).sum()
}

fn encode_with_mvt(tile: &MvtTile) -> Result<Vec<u8>, mvt::Error> {
    let extent = tile.layers.first().map_or(DEFAULT_EXTENT, |v| v.extent);
    let mut out = mvt::Tile::new(extent);
    for layer in &tile.layers {
        let mut mvt_layer = out.create_layer(&layer.name);
        for feature in &layer.features {
            let geom_data = mvt_geom_data(&feature.geometry)?;
            let mut mvt_feature = mvt_layer.into_feature(geom_data);
            if let Some(id) = feature.id {
                mvt_feature.set_id(id);
            }
            for (key, value) in &feature.properties {
                add_mvt_tag(&mut mvt_feature, key, value);
            }
            mvt_layer = mvt_feature.into_layer();
        }
        out.add_layer(mvt_layer)?;
    }
    out.to_bytes()
}

fn add_mvt_tag(feature: &mut mvt::Feature, key: &str, value: &MvtValue) {
    match value {
        MvtValue::String(value) => feature.add_tag_string(key, value),
        MvtValue::Float(value) => feature.add_tag_float(key, *value),
        MvtValue::Double(value) => feature.add_tag_double(key, *value),
        MvtValue::Int(value) => feature.add_tag_int(key, *value),
        MvtValue::UInt(value) => feature.add_tag_uint(key, *value),
        MvtValue::SInt(value) => feature.add_tag_sint(key, *value),
        MvtValue::Bool(value) => feature.add_tag_bool(key, *value),
        MvtValue::Null => {}
    }
}

fn mvt_geom_data(geometry: &MvtGeometry) -> Result<mvt::GeomData, mvt::Error> {
    let mut encoder = match geometry {
        Geometry::Point(_) | Geometry::MultiPoint(_) => GeomEncoder::<f64>::new(GeomType::Point),
        Geometry::LineString(_) | Geometry::MultiLineString(_) => {
            GeomEncoder::<f64>::new(GeomType::Linestring)
        }
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => {
            GeomEncoder::<f64>::new(GeomType::Polygon)
        }
        Geometry::GeometryCollection(collection) if collection.0.len() == 1 => {
            return mvt_geom_data(&collection.0[0]);
        }
        _ => return Err(mvt::Error::InvalidGeometry()),
    };

    match geometry {
        Geometry::Point(point) => add_mvt_point(&mut encoder, point.0)?,
        Geometry::MultiPoint(points) => {
            for point in &points.0 {
                add_mvt_point(&mut encoder, point.0)?;
            }
        }
        Geometry::LineString(line) => add_mvt_line(&mut encoder, line)?,
        Geometry::MultiLineString(lines) => {
            for line in &lines.0 {
                add_mvt_line(&mut encoder, line)?;
                encoder.complete_geom()?;
            }
        }
        Geometry::Polygon(polygon) => add_mvt_polygon(&mut encoder, polygon)?,
        Geometry::MultiPolygon(polygons) => {
            for polygon in &polygons.0 {
                add_mvt_polygon(&mut encoder, polygon)?;
                encoder.complete_geom()?;
            }
        }
        Geometry::GeometryCollection(_)
        | Geometry::Line(_)
        | Geometry::Rect(_)
        | Geometry::Triangle(_) => unreachable!("validated above"),
    }
    encoder.encode()
}

fn add_mvt_point(encoder: &mut GeomEncoder<f64>, coord: MvtCoord) -> Result<(), mvt::Error> {
    encoder.add_point(coord.x.into(), coord.y.into())
}

fn add_mvt_line(encoder: &mut GeomEncoder<f64>, line: &MvtLineString) -> Result<(), mvt::Error> {
    for &coord in without_trailing_duplicate(&line.0) {
        add_mvt_point(encoder, coord)?;
    }
    Ok(())
}

fn add_mvt_polygon(encoder: &mut GeomEncoder<f64>, polygon: &MvtPolygon) -> Result<(), mvt::Error> {
    add_mvt_line(encoder, polygon.exterior())?;
    encoder.complete_geom()?;
    for ring in polygon.interiors() {
        add_mvt_line(encoder, ring)?;
        encoder.complete_geom()?;
    }
    Ok(())
}

fn without_trailing_duplicate(coords: &[MvtCoord]) -> &[MvtCoord] {
    if coords.len() >= 2 && coords.first() == coords.last() {
        &coords[..coords.len() - 1]
    } else {
        coords
    }
}
