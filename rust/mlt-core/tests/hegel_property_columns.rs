use hegel::generators as gs;
use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
use mlt_core::geo_types::{Geometry, Point};
use mlt_core::{Decoder, Layer, Parser, PropKind, PropValue, TileLayer};

fn decode(bytes: &[u8]) -> TileLayer {
    let layers = Parser::default().parse_layers(bytes).expect("parse");
    assert_eq!(layers.len(), 1);
    let layer: Layer<'_> = layers.into_iter().next().unwrap();
    layer
        .into_tile(&mut Decoder::default())
        .expect("into_tile")
        .expect("a known layer tag")
}

fn rendered(value: Option<&PropValue>) -> Option<String> {
    match value? {
        PropValue::Bool(v) => v.map(|v| v.to_string()),
        PropValue::I8(v) => v.map(|v| v.to_string()),
        PropValue::U8(v) => v.map(|v| v.to_string()),
        PropValue::I32(v) => v.map(|v| v.to_string()),
        PropValue::U32(v) => v.map(|v| v.to_string()),
        PropValue::I64(v) => v.map(|v| v.to_string()),
        PropValue::U64(v) => v.map(|v| v.to_string()),
        PropValue::F32(v) => v.map(|v| v.to_string()),
        PropValue::F64(v) => v.map(|v| v.to_string()),
        PropValue::Str(v) => v.as_ref().map(ToString::to_string),
    }
}

fn values_by_id(layer: &TileLayer) -> Vec<(Option<u64>, Option<String>)> {
    let mut rows: Vec<_> = layer
        .features()
        .iter()
        .map(|f| (f.id(), rendered(f.properties().first())))
        .collect();
    rows.sort_by_key(|row| row.0);
    rows
}

fn layer_of(kind: PropKind, column: &[PropValue]) -> TileLayer {
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let key = builder.add_property("value", kind).unwrap();
    for (id, value) in (0_u64..).zip(column) {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(Some(id));
        feature.property(key, value.clone()).unwrap();
        feature.finish().unwrap();
    }
    builder.finish()
}

#[hegel::composite]
fn presence_mask(tc: &hegel::TestCase) -> Vec<bool> {
    let len = tc.draw(gs::integers::<usize>().min_value(1).max_value(600));
    match tc.draw(gs::integers::<u8>().max_value(3)) {
        0 => {
            let per_mille = tc.draw(gs::integers::<u32>().max_value(1000));
            (0..len)
                .map(|_| tc.draw(gs::integers::<u32>().max_value(999)) < per_mille)
                .collect()
        }
        1 => {
            let mut mask = Vec::with_capacity(len);
            let mut present = tc.draw(gs::booleans());
            while mask.len() < len {
                let run = tc.draw(gs::integers::<usize>().min_value(1).max_value(200));
                mask.extend(std::iter::repeat_n(present, run.min(len - mask.len())));
                present = !present;
            }
            mask
        }
        2 => vec![tc.draw(gs::booleans()); len],
        _ => {
            let hot =
                tc.draw(gs::vecs(gs::integers::<usize>().max_value(len.max(1) - 1)).max_size(4));
            (0..len).map(|at| hot.contains(&at)).collect()
        }
    }
}

#[hegel::composite]
fn string_pool(tc: &hegel::TestCase) -> Vec<String> {
    let stems = tc.draw(gs::vecs(gs::text().max_size(12)).min_size(1).max_size(4));
    let count = tc.draw(gs::integers::<usize>().min_value(1).max_value(80));
    (0..count)
        .map(|_| {
            let at = tc.draw(gs::integers::<usize>().max_value(stems.len() - 1));
            let tail = tc.draw(gs::text().max_size(8));
            format!("{}{tail}", stems[at])
        })
        .collect()
}

#[hegel::composite]
fn bit_width_u64s(tc: &hegel::TestCase) -> Vec<u64> {
    let width = tc.draw(gs::integers::<u32>().max_value(64));
    let max = if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    };
    tc.draw(
        gs::vecs(gs::integers::<u64>().max_value(max))
            .min_size(1)
            .max_size(300),
    )
}

#[hegel::test]
fn optional_i32_column_round_trips_under_any_presence_pattern(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let mask = tc.draw(presence_mask());
    let column: Vec<PropValue> = mask
        .iter()
        .zip(0_i32..)
        .map(|(&present, n)| PropValue::I32(present.then_some(n)))
        .collect();
    let layer = layer_of(PropKind::I32, &column);
    for version in [WireVersion::V01, WireVersion::V02] {
        let bytes = layer
            .clone()
            .encode(unsorted_4g.with_wire_version(version))
            .expect("encode");
        assert_eq!(values_by_id(&decode(&bytes)), values_by_id(&layer));
    }
}

#[hegel::test]
fn optional_string_column_round_trips_with_and_without_fsst_and_shared_dict(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let pool = tc.draw(string_pool());
    let mask = tc.draw(
        gs::vecs(gs::booleans())
            .min_size(pool.len())
            .max_size(pool.len()),
    );
    let column: Vec<PropValue> = pool
        .iter()
        .zip(mask)
        .map(|(s, present)| PropValue::Str(present.then(|| s.as_str().into())))
        .collect();
    let layer = layer_of(PropKind::Str, &column);
    for version in [WireVersion::V01, WireVersion::V02] {
        for fsst in [false, true] {
            for shared_dict in [false, true] {
                let config = unsorted_4g
                    .with_wire_version(version)
                    .with_fsst(fsst)
                    .with_shared_dict(shared_dict);
                let bytes = layer.clone().encode(config).expect("encode");
                assert_eq!(values_by_id(&decode(&bytes)), values_by_id(&layer));
            }
        }
    }
}

#[hegel::test]
fn u64_column_round_trips_at_any_bit_width_with_and_without_fastpfor(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let values = tc.draw(bit_width_u64s());
    let column: Vec<PropValue> = values.iter().map(|&v| PropValue::U64(Some(v))).collect();
    let layer = layer_of(PropKind::U64, &column);
    for version in [WireVersion::V01, WireVersion::V02] {
        for fastpfor in [false, true] {
            let config = unsorted_4g
                .with_wire_version(version)
                .with_fastpfor(fastpfor);
            let bytes = layer.clone().encode(config).expect("encode");
            assert_eq!(values_by_id(&decode(&bytes)), values_by_id(&layer));
        }
    }
}

#[hegel::test]
fn u32_column_round_trips_for_any_values(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let values = tc.draw(gs::vecs(gs::integers::<u32>()).min_size(1).max_size(300));
    let column: Vec<PropValue> = values.iter().map(|&v| PropValue::U32(Some(v))).collect();
    let layer = layer_of(PropKind::U32, &column);
    for version in [WireVersion::V01, WireVersion::V02] {
        let bytes = layer
            .clone()
            .encode(unsorted_4g.with_wire_version(version))
            .expect("encode");
        assert_eq!(values_by_id(&decode(&bytes)), values_by_id(&layer));
    }
}
