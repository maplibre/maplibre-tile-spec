//! A [`LayerWriter`] holding the m-values of a [`TileLayer`] encodes to the same bytes.

use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion, wound_layer};
use mlt_core::fast_mvt::MvtTileBuilder;
use mlt_core::mvt::tile_layers_to_mvt;
use mlt_core::{
    Decoder, FeatureWriter, GeometryType, Layer, LayerWriter, MValue, MValueKey, MltResult, Parser,
    PropKind, PropValueRef, TileLayer,
};
use rstest::rstest;

#[derive(Debug, Clone, Copy)]
enum Rings {
    Closed,
    Open,
}

fn v2() -> EncoderConfig {
    EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_wire_version(WireVersion::V02)
}

fn unsorted() -> EncoderConfig {
    v2().with_tessellation(Tessellation::Off)
}

fn sorted() -> EncoderConfig {
    unsorted()
        .with_spatial_morton_sort(true)
        .with_spatial_hilbert_sort(true)
        .with_id_sort(true)
}

fn tessellated() -> EncoderConfig {
    sorted().with_tessellation(Tessellation::Always)
}

fn triangles_only() -> EncoderConfig {
    tessellated().with_triangles_only(true)
}

fn c(x: i32, y: i32) -> Coord<i32> {
    Coord { x, y }
}

fn line(pts: &[(i32, i32)]) -> LineString<i32> {
    LineString::new(pts.iter().map(|&(x, y)| c(x, y)).collect())
}

fn polygon(exterior: &[(i32, i32)], holes: &[&[(i32, i32)]]) -> Polygon<i32> {
    Polygon::new(line(exterior), holes.iter().map(|h| line(h)).collect())
}

fn u32s(values: &[u32]) -> MValue {
    MValue::U32(Some(values.to_vec()))
}

fn f64s(values: &[f64]) -> MValue {
    MValue::F64(Some(values.to_vec()))
}

fn strs(values: &[&str]) -> MValue {
    MValue::Str(Some(values.iter().map(|&v| v.into()).collect()))
}

fn layer(features: Vec<(u64, Geometry<i32>, [MValue; 3])>) -> TileLayer {
    let mut builder = TileLayer::builder("roads", 4096).unwrap();
    let keys = [
        builder.add_m_value("speed", PropKind::U32).unwrap(),
        builder.add_m_value("lane", PropKind::Str).unwrap(),
        builder.add_m_value("grade", PropKind::F64).unwrap(),
    ];
    for (id, geometry, m_values) in features {
        let mut feature = builder.feature(geometry);
        feature.id(Some(id));
        for (key, value) in keys.into_iter().zip(m_values) {
            feature.m_value(key, value).unwrap();
        }
        feature.finish().unwrap();
    }
    builder.finish()
}

fn lines() -> TileLayer {
    layer(vec![
        (
            5,
            line(&[(100, 100), (200, 150), (300, 100)]).into(),
            [
                u32s(&[30, 50, 50]),
                strs(&["a", "b", "b"]),
                f64s(&[0.5, 1.5, -2.0]),
            ],
        ),
        (
            2,
            line(&[(4000, 4000), (3900, 3900)]).into(),
            [u32s(&[10, 20]), MValue::Str(None), f64s(&[1.0, 2.0])],
        ),
        (
            3,
            line(&[(2000, 100), (2100, 200), (2200, 300), (2300, 400)]).into(),
            [MValue::U32(None), MValue::Str(None), MValue::F64(None)],
        ),
        (
            1,
            line(&[(50, 3000), (60, 3100)]).into(),
            [u32s(&[70, 80]), strs(&["x", "y"]), MValue::F64(None)],
        ),
    ])
}

fn multi_lines() -> TileLayer {
    layer(vec![
        (
            8,
            MultiLineString::new(vec![
                line(&[(4000, 4000), (3900, 3900)]),
                line(&[(3800, 4000), (3700, 3950), (3600, 4000)]),
            ])
            .into(),
            [
                u32s(&[10, 20, 30, 40, 50]),
                MValue::Str(None),
                f64s(&[0.1, 0.2, 0.3, 0.4, 0.5]),
            ],
        ),
        (
            4,
            MultiLineString::new(vec![line(&[(10, 10), (20, 20)])]).into(),
            [MValue::U32(None), strs(&["p", "q"]), f64s(&[9.0, 8.0])],
        ),
        (
            6,
            MultiLineString::new(vec![]).into(),
            [u32s(&[]), MValue::Str(None), MValue::F64(None)],
        ),
    ])
}

fn polygons_with_holes() -> TileLayer {
    layer(vec![
        (
            9,
            polygon(
                &[(1000, 1000), (1000, 2000), (2000, 2000), (2000, 1000)],
                &[&[(1200, 1200), (1400, 1200), (1400, 1400), (1200, 1400)]],
            )
            .into(),
            [
                u32s(&[1, 2, 3, 4, 5, 6, 7, 8]),
                strs(&["e0", "e1", "e2", "e3", "h0", "h1", "h2", "h3"]),
                MValue::F64(None),
            ],
        ),
        (
            1,
            polygon(
                &[(3000, 0), (4000, 0), (4000, 1000), (3000, 1000)],
                &[&[(3200, 200), (3200, 400), (3400, 400), (3400, 200)]],
            )
            .into(),
            [
                MValue::U32(None),
                strs(&["a", "b", "c", "d", "e", "f", "g", "h"]),
                f64s(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
            ],
        ),
        (
            7,
            MultiPolygon::new(vec![
                polygon(&[(0, 0), (0, 500), (500, 500), (500, 0)], &[]),
                polygon(
                    &[(600, 600), (900, 600), (900, 900), (600, 900)],
                    &[&[(700, 700), (800, 700), (750, 800)]],
                ),
            ])
            .into(),
            [
                u32s(&[11, 12, 13, 14, 21, 22, 23, 24, 31, 32, 33]),
                MValue::Str(None),
                f64s(&[-1.0, -2.0, -3.0, -4.0, 1.0, 2.0, 3.0, 4.0, 0.5, 0.25, 0.125]),
            ],
        ),
    ])
}

fn mixed() -> TileLayer {
    layer(vec![
        (
            3,
            Point::new(5, 5).into(),
            [u32s(&[1]), strs(&["pt"]), MValue::F64(None)],
        ),
        (
            2,
            MultiPoint::new(vec![Point::new(10, 10), Point::new(20, 30)]).into(),
            [u32s(&[2, 3]), MValue::Str(None), f64s(&[0.0, 1.0])],
        ),
        (
            1,
            line(&[(100, 100), (200, 150), (300, 100)]).into(),
            [
                u32s(&[30, 50, 50]),
                strs(&["a", "b", "b"]),
                f64s(&[0.5, 1.5, -2.0]),
            ],
        ),
        (
            4,
            polygon(
                &[(1000, 1000), (1000, 2000), (2000, 2000), (2000, 1000)],
                &[&[(1200, 1200), (1400, 1200), (1400, 1400), (1200, 1400)]],
            )
            .into(),
            [
                u32s(&[1, 2, 3, 4, 5, 6, 7, 8]),
                MValue::Str(None),
                f64s(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
            ],
        ),
    ])
}

fn refs(m_value: &MValue) -> Option<Vec<PropValueRef<'_>>> {
    Some(match m_value {
        MValue::Bool(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::I8(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::U8(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::I32(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::U32(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::I64(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::U64(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::F32(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::F64(v) => v.as_ref()?.iter().map(|&x| x.into()).collect(),
        MValue::Str(v) => v.as_ref()?.iter().map(|x| x.as_str().into()).collect(),
    })
}

fn write(layer: &TileLayer, rings: Rings) -> MltResult<LayerWriter<'_>> {
    let mut writer = LayerWriter::new(layer.name(), layer.extent().get())?;
    write_features(&mut writer, layer, rings)?;
    Ok(writer)
}

fn write_features<'a>(
    writer: &mut LayerWriter<'a>,
    layer: &'a TileLayer,
    rings: Rings,
) -> MltResult<()> {
    let keys = (layer.m_value_names().iter().zip(layer.m_value_kinds()))
        .map(|(name, &kind)| writer.add_m_value(name, kind))
        .collect::<MltResult<Vec<MValueKey>>>()?;
    for feature in layer.features() {
        let geometry = feature.geometry();
        let mut out = writer.feature(GeometryType::try_from(geometry).expect("one kind"));
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
            Geometry::Polygon(p) => write_rings(&mut out, p, rings)?,
            Geometry::MultiPolygon(mp) => {
                for p in &mp.0 {
                    write_rings(&mut out, p, rings)?;
                }
            }
            Geometry::Line(_)
            | Geometry::GeometryCollection(_)
            | Geometry::Rect(_)
            | Geometry::Triangle(_) => {}
        }
        for (&key, value) in keys.iter().zip(feature.m_values()) {
            if let Some(values) = refs(value) {
                out.m_values(key, values)?;
            }
        }
        out.finish()?;
    }
    Ok(())
}

fn write_rings(
    out: &mut FeatureWriter<'_, '_>,
    polygon: &Polygon<i32>,
    rings: Rings,
) -> MltResult<()> {
    let coords = |ring: &LineString<i32>| -> Vec<Coord<i32>> {
        match rings {
            Rings::Closed => ring.0.clone(),
            Rings::Open => ring.0[..ring.0.len() - 1].to_vec(),
        }
    };
    out.exterior_ring(coords(polygon.exterior()))?;
    for hole in polygon.interiors() {
        out.hole(coords(hole))?;
    }
    Ok(())
}

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).unwrap();
    let Some(Layer::Tag02(layer)) = layers.into_iter().next() else {
        panic!("expected one v2 layer")
    };
    layer.into_tile(&mut Decoder::default()).unwrap()
}

#[rstest]
fn writer_m_values_encode_like_tile_layer(
    #[values(lines(), multi_lines(), polygons_with_holes(), mixed())] layer: TileLayer,
    #[values(Rings::Closed, Rings::Open)] rings: Rings,
    #[values(v2(), unsorted(), sorted(), tessellated(), triangles_only())] cfg: EncoderConfig,
) {
    let writer = write(&layer, rings).unwrap();
    assert_eq!(writer.encode(cfg).unwrap(), layer.encode(cfg).unwrap());
}

#[rstest]
fn writer_m_values_decode_per_vertex_with_rings_wound(
    #[values(lines(), multi_lines(), polygons_with_holes(), mixed())] layer: TileLayer,
    #[values(Rings::Closed, Rings::Open)] rings: Rings,
) {
    let writer = write(&layer, rings).unwrap();
    assert_eq!(
        decode(&writer.encode(unsorted()).unwrap()),
        wound_layer(&layer)
    );
}

#[test]
fn line_m_values_decode_in_vertex_order() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::I32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5), c(9, 0)])
        .unwrap()
        .m_values(speed, [30, -10, 50])
        .unwrap();
    feature.finish().unwrap();

    let decoded = decode(&writer.encode(v2()).unwrap());
    assert_eq!(
        decoded.features()[0].m_values(),
        [MValue::I32(Some(vec![30, -10, 50]))]
    );
}

#[test]
fn feature_without_m_values_is_null() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U8).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u8, 2])
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature.line([c(9, 9), c(7, 7)]).unwrap();
    feature.finish().unwrap();

    let decoded = decode(&writer.encode(v2()).unwrap());
    assert_eq!(decoded.features()[1].m_values(), [MValue::U8(None)]);
}

#[test]
fn rejects_fewer_m_values_than_line_vertices() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5), c(9, 0)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    insta::assert_snapshot!(feature.finish().unwrap_err(), @"m-value 0 holds 2 values, but the feature has 3 vertices");
    assert_eq!(writer.feature_count(), 0);
}

#[test]
fn rejects_an_m_value_for_a_closing_ring_coordinate() {
    let mut writer = LayerWriter::new("areas", 4096).unwrap();
    let height = writer.add_m_value("height", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::Polygon);
    feature
        .exterior_ring([c(0, 0), c(9, 0), c(9, 9), c(0, 9), c(0, 0)])
        .unwrap()
        .m_values(height, [1_u32, 2, 3, 4, 5])
        .unwrap();
    insta::assert_snapshot!(feature.finish().unwrap_err(), @"m-value 0 holds 5 values, but the feature has 4 vertices");
}

#[test]
fn rejected_vertex_count_leaves_the_layer_as_it_was() {
    let mut expected = LayerWriter::new("roads", 4096).unwrap();
    let speed = expected.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = expected.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature.finish().unwrap();

    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(1, 1), c(2, 2), c(3, 3)])
        .unwrap()
        .m_values(speed, [7_u32])
        .unwrap();
    feature.finish().unwrap_err();

    assert_eq!(writer.encode(v2()).unwrap(), expected.encode(v2()).unwrap());
}

#[test]
fn dropped_feature_leaves_the_layer_as_it_was() {
    let mut expected = LayerWriter::new("roads", 4096).unwrap();
    let speed = expected.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = expected.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature.finish().unwrap();

    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature.finish().unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(1, 1), c(2, 2)])
        .unwrap()
        .m_values(speed, [7_u32, 8])
        .unwrap();
    drop(feature);

    assert_eq!(writer.encode(v2()).unwrap(), expected.encode(v2()).unwrap());
}

#[test]
fn rejects_m_value_of_another_kind() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    insta::assert_snapshot!(
        feature.m_values(speed, [1_u32, 2, 3]).and_then(|f| f.m_values(speed, [1_i32])).err().unwrap(),
        @"m-value 0 kind mismatch: expected U32, got I32"
    );
}

#[test]
fn rejected_m_values_leave_the_feature_null() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature
        .m_values(speed, [PropValueRef::U32(3), PropValueRef::Str("no")])
        .err()
        .unwrap();
    feature.finish().unwrap();

    let decoded = decode(&writer.encode(v2()).unwrap());
    assert_eq!(decoded.features()[0].m_values(), [MValue::U32(None)]);
}

#[test]
fn rejects_unknown_m_value_key() {
    let mut other = LayerWriter::new("other", 4096).unwrap();
    other.add_m_value("a", PropKind::U32).unwrap();
    let key = other.add_m_value("b", PropKind::U32).unwrap();
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    writer.add_m_value("a", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    insta::assert_snapshot!(
        feature.m_values(key, [1_u32]).err().unwrap(),
        @"m-value key 1 belongs to another layer: this feature has 1 columns"
    );
}

#[test]
fn rejects_m_value_named_like_a_property() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    writer.add_property("speed", PropKind::U32).unwrap();
    insta::assert_snapshot!(
        writer.add_m_value("speed", PropKind::U32).unwrap_err(),
        @"duplicate column name speed: the m-value column repeats the property column"
    );
}

#[test]
fn rejects_property_named_like_an_m_value() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    writer.add_m_value("speed", PropKind::U32).unwrap();
    insta::assert_snapshot!(
        writer.add_property("speed", PropKind::U32).unwrap_err(),
        @"duplicate column name speed: the property column repeats the m-value column"
    );
}

#[test]
fn point_layer_m_values_are_rejected_like_tile_layer() {
    let mut writer = LayerWriter::new("pois", 4096).unwrap();
    let rank = writer.add_m_value("rank", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::Point);
    feature
        .points([c(1, 2)])
        .unwrap()
        .m_values(rank, [5_u32])
        .unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("pois", 4096).unwrap();
    let rank = builder.add_m_value("rank", PropKind::U32).unwrap();
    let mut feature = builder.feature(Point::new(1, 2).into());
    feature.m_value(rank, MValue::U32(Some(vec![5]))).unwrap();
    feature.finish().unwrap();
    let layer = builder.finish();

    insta::assert_snapshot!(
        writer.encode(v2()).unwrap_err(),
        @"v2 geometry layout Points gives no per-feature vertex count, so it cannot carry m-values"
    );
    assert_eq!(
        writer.encode(v2()).unwrap_err().to_string(),
        layer.encode(v2()).unwrap_err().to_string()
    );
}

#[test]
fn v1_rejects_writer_m_values_like_tile_layer() {
    let layer = lines();
    let writer = write(&layer, Rings::Closed).unwrap();
    let v1 = v2().with_wire_version(WireVersion::V01);
    insta::assert_snapshot!(
        writer.encode(v1).unwrap_err(),
        @"m-values are a v2 feature, so layer roads cannot be written as v1"
    );
    assert_eq!(
        writer.encode(v1).unwrap_err().to_string(),
        layer.encode(v1).unwrap_err().to_string()
    );
}

#[test]
fn eight_m_value_columns_encode_like_tile_layer() {
    let names = ["m0", "m1", "m2", "m3", "m4", "m5", "m6", "m7"];
    let mut builder = TileLayer::builder("roads", 4096).unwrap();
    let keys: Vec<_> = names
        .iter()
        .map(|name| builder.add_m_value(*name, PropKind::I32).unwrap())
        .collect();
    let mut feature = builder.feature(line(&[(0, 0), (5, 5)]).into());
    for (i, key) in (0..).zip(&keys) {
        feature
            .m_value(*key, MValue::I32(Some(vec![i, -i])))
            .unwrap();
    }
    feature.finish().unwrap();
    let layer = builder.finish();

    let writer = write(&layer, Rings::Closed).unwrap();
    assert_eq!(writer.encode(v2()).unwrap(), layer.encode(v2()).unwrap());
}

#[test]
fn m_value_column_added_after_features_encodes_like_tile_layer() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature.line([c(0, 0), c(5, 5)]).unwrap();
    feature.finish().unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(9, 9), c(7, 7)])
        .unwrap()
        .m_values(speed, [3_u32, 4])
        .unwrap();
    feature.finish().unwrap();

    let mut builder = TileLayer::builder("roads", 4096).unwrap();
    builder
        .feature(line(&[(0, 0), (5, 5)]).into())
        .finish()
        .unwrap();
    let speed = builder.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = builder.feature(line(&[(9, 9), (7, 7)]).into());
    feature
        .m_value(speed, MValue::U32(Some(vec![3, 4])))
        .unwrap();
    feature.finish().unwrap();
    let layer = builder.finish();

    assert_eq!(writer.encode(v2()).unwrap(), layer.encode(v2()).unwrap());
}

#[test]
fn repeated_m_values_keep_the_last_ones() {
    let mut writer = LayerWriter::new("roads", 4096).unwrap();
    let speed = writer.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = writer.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5)])
        .unwrap()
        .m_values(speed, [1_u32, 2, 3])
        .unwrap()
        .m_values(speed, [4_u32, 5])
        .unwrap();
    feature.finish().unwrap();

    let decoded = decode(&writer.encode(v2()).unwrap());
    assert_eq!(
        decoded.features()[0].m_values(),
        [MValue::U32(Some(vec![4, 5]))]
    );
}

#[test]
fn reset_writer_with_m_values_encodes_like_a_new_one() {
    let layer = polygons_with_holes();
    let fresh = write(&layer, Rings::Closed).unwrap();

    let mut reused = LayerWriter::new("other", 512).unwrap();
    let text = reused.add_m_value("speed", PropKind::Str).unwrap();
    let number = reused.add_m_value("lane", PropKind::F32).unwrap();
    reused.add_m_value("unused", PropKind::Bool).unwrap();
    let mut feature = reused.feature(GeometryType::LineString);
    feature
        .line([c(0, 0), c(5, 5), c(6, 6)])
        .unwrap()
        .m_values(text, ["longer text", "b", "c"])
        .unwrap()
        .m_values(number, [1.0_f32, 2.0, 3.0])
        .unwrap();
    feature.finish().unwrap();
    reused.reset(layer.name(), layer.extent().get()).unwrap();
    write_features(&mut reused, &layer, Rings::Closed).unwrap();

    assert_eq!(reused.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[test]
fn reset_writer_drops_its_m_value_columns() {
    let mut fresh = LayerWriter::new("roads", 4096).unwrap();
    let mut feature = fresh.feature(GeometryType::LineString);
    feature.line([c(0, 0), c(5, 5)]).unwrap();
    feature.finish().unwrap();

    let mut reused = LayerWriter::new("roads", 4096).unwrap();
    let speed = reused.add_m_value("speed", PropKind::U32).unwrap();
    let mut feature = reused.feature(GeometryType::LineString);
    feature
        .line([c(1, 1), c(2, 2)])
        .unwrap()
        .m_values(speed, [1_u32, 2])
        .unwrap();
    feature.finish().unwrap();
    reused.reset("roads", 4096).unwrap();
    let mut feature = reused.feature(GeometryType::LineString);
    feature.line([c(0, 0), c(5, 5)]).unwrap();
    feature.finish().unwrap();

    assert_eq!(reused.encode(v2()).unwrap(), fresh.encode(v2()).unwrap());
}

#[rstest]
fn writer_mvt_drops_m_values_like_tile_layer(
    #[values(lines(), multi_lines(), polygons_with_holes(), mixed())] layer: TileLayer,
) {
    let writer = write(&layer, Rings::Closed).unwrap();
    assert_eq!(
        writer.write_mvt(MvtTileBuilder::new()).unwrap().encode(),
        tile_layers_to_mvt([&layer]).unwrap()
    );
}
