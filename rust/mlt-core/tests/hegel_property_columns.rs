use hegel::generators as gs;
use mlt_core::encoder::{EncoderConfig, Network, Profile, WireVersion};
use mlt_core::geo_types::{Geometry, Point};
use mlt_core::{Decoder, ParsedLayer, ParsedProperty, Parser, PropKind, PropValue, TileLayer};

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
    let expected: Vec<(Option<u64>, Option<i32>)> = (0_u64..)
        .zip(0_i32..)
        .zip(&mask)
        .map(|((id, n), &present)| (Some(id), present.then_some(n)))
        .collect();
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let value = builder.add_property("value", PropKind::I32).unwrap();
    for (id, n) in &expected {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(*id);
        feature.property(value, PropValue::I32(*n)).unwrap();
        feature.finish().unwrap();
    }
    let layer = builder.finish();
    for version in [WireVersion::V01, WireVersion::V02] {
        let bytes = layer
            .clone()
            .encode(unsorted_4g.with_wire_version(version))
            .expect("encode");
        let mut layers = Parser::default().parse_layers(&bytes).expect("parse");
        assert_eq!(layers.len(), 1);
        let decoded = layers
            .remove(0)
            .into_tile(&mut Decoder::default())
            .expect("into_tile")
            .expect("a known layer tag");
        let mut actual: Vec<(Option<u64>, Option<i32>)> = decoded
            .features()
            .iter()
            .map(|f| match f.properties().first() {
                Some(PropValue::I32(n)) => (f.id(), *n),
                _ => (f.id(), None),
            })
            .collect();
        actual.sort_by_key(|row| row.0);
        assert_eq!(actual, expected);
    }
}

#[hegel::test]
fn optional_string_columns_round_trip_with_and_without_fsst_and_shared_dict(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let pool = tc.draw(string_pool());
    let names = ["label_a", "label_b", "label_c"];
    let column_count = tc.draw(gs::integers::<usize>().min_value(2).max_value(3));
    let mut columns: Vec<Vec<Option<String>>> = Vec::new();
    for _ in 0..column_count {
        let shift = tc.draw(gs::integers::<usize>().max_value(pool.len() - 1));
        let mask = tc.draw(
            gs::vecs(gs::booleans())
                .min_size(pool.len())
                .max_size(pool.len()),
        );
        columns.push(
            mask.iter()
                .enumerate()
                .map(|(at, &present)| present.then(|| pool[(at + shift) % pool.len()].clone()))
                .collect(),
        );
    }
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let keys: Vec<_> = names[..column_count]
        .iter()
        .map(|name| builder.add_property(*name, PropKind::Str).unwrap())
        .collect();
    for id in 0..pool.len() {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(Some(id as u64));
        for (key, column) in keys.iter().zip(&columns) {
            let value = column[id].as_deref().map(Into::into);
            feature.property(*key, PropValue::Str(value)).unwrap();
        }
        feature.finish().unwrap();
    }
    let layer = builder.finish();
    let expected: Vec<(Option<u64>, Vec<Option<String>>)> = (0..pool.len())
        .map(|id| {
            let row = columns.iter().map(|column| column[id].clone());
            (Some(id as u64), row.collect())
        })
        .collect();
    for version in [WireVersion::V01, WireVersion::V02] {
        for fsst in [false, true] {
            for shared_dict in [false, true] {
                let config = unsorted_4g
                    .with_wire_version(version)
                    .with_fsst(fsst)
                    .with_shared_dict(shared_dict);
                let bytes = layer.clone().encode(config).expect("encode");
                let mut layers = Parser::default().parse_layers(&bytes).expect("parse");
                assert_eq!(layers.len(), 1);
                let decoded = layers
                    .remove(0)
                    .into_tile(&mut Decoder::default())
                    .expect("into_tile")
                    .expect("a known layer tag");
                let positions: Vec<Option<usize>> = names[..column_count]
                    .iter()
                    .map(|name| decoded.property_names().iter().position(|n| n == name))
                    .collect();
                let mut actual: Vec<(Option<u64>, Vec<Option<String>>)> = decoded
                    .features()
                    .iter()
                    .map(|f| {
                        let row = positions.iter().map(|position| {
                            match position.and_then(|at| f.properties().get(at)) {
                                Some(PropValue::Str(Some(s))) => Some(s.to_string()),
                                _ => None,
                            }
                        });
                        (f.id(), row.collect())
                    })
                    .collect();
                actual.sort_by_key(|row| row.0);
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn two_heavily_overlapping_string_columns_become_one_shared_dictionary_only_when_allowed() {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let first = builder.add_property("label_a", PropKind::Str).unwrap();
    let second = builder.add_property("label_b", PropKind::Str).unwrap();
    for id in 0_u64..40 {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(Some(id));
        let street = |n: u64| PropValue::Str(Some(format!("street {}", n % 8).into()));
        feature.property(first, street(id)).unwrap();
        feature.property(second, street(id + 3)).unwrap();
        feature.finish().unwrap();
    }
    let layer = builder.finish();
    for version in [WireVersion::V01, WireVersion::V02] {
        let mut shared_dict_counts = Vec::new();
        for shared_dict in [false, true] {
            let config = unsorted_4g
                .with_wire_version(version)
                .with_shared_dict(shared_dict);
            let bytes = layer.clone().encode(config).expect("encode");
            let mut layers = Parser::default().parse_layers(&bytes).expect("parse");
            assert_eq!(layers.len(), 1);
            let parsed = layers
                .remove(0)
                .decode_all(&mut Decoder::default())
                .expect("decode_all");
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "ParsedLayer is non_exhaustive"
            )]
            let properties = match &parsed {
                ParsedLayer::Tag01(layer) => layer.properties(),
                ParsedLayer::Tag02(layer) => layer.layer().properties(),
                _ => panic!("expected a v1 or v2 layer"),
            };
            shared_dict_counts.push(
                properties
                    .iter()
                    .filter(|p| matches!(p, ParsedProperty::SharedDict(_)))
                    .count(),
            );
        }
        assert_eq!(shared_dict_counts, [0, 1], "{version:?}");
    }
}

#[hegel::test]
fn u64_column_round_trips_at_any_bit_width_with_and_without_fastpfor(tc: hegel::TestCase) {
    let unsorted_4g = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G))
        .with_spatial_morton_sort(false)
        .with_spatial_hilbert_sort(false)
        .with_id_sort(false);
    let values = tc.draw(bit_width_u64s());
    let expected: Vec<(Option<u64>, Option<u64>)> = (0_u64..)
        .zip(&values)
        .map(|(id, &n)| (Some(id), Some(n)))
        .collect();
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let value = builder.add_property("value", PropKind::U64).unwrap();
    for (id, n) in &expected {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(*id);
        feature.property(value, PropValue::U64(*n)).unwrap();
        feature.finish().unwrap();
    }
    let layer = builder.finish();
    for version in [WireVersion::V01, WireVersion::V02] {
        for fastpfor in [false, true] {
            let config = unsorted_4g
                .with_wire_version(version)
                .with_fastpfor(fastpfor);
            let bytes = layer.clone().encode(config).expect("encode");
            let mut layers = Parser::default().parse_layers(&bytes).expect("parse");
            assert_eq!(layers.len(), 1);
            let decoded = layers
                .remove(0)
                .into_tile(&mut Decoder::default())
                .expect("into_tile")
                .expect("a known layer tag");
            let mut actual: Vec<(Option<u64>, Option<u64>)> = decoded
                .features()
                .iter()
                .map(|f| match f.properties().first() {
                    Some(PropValue::U64(n)) => (f.id(), *n),
                    Some(PropValue::U32(n)) => (f.id(), n.map(u64::from)),
                    _ => (f.id(), None),
                })
                .collect();
            actual.sort_by_key(|row| row.0);
            assert_eq!(actual, expected);
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
    let expected: Vec<(Option<u64>, Option<u32>)> = (0_u64..)
        .zip(&values)
        .map(|(id, &n)| (Some(id), Some(n)))
        .collect();
    let mut builder = TileLayer::builder("columns", 4096).unwrap();
    let value = builder.add_property("value", PropKind::U32).unwrap();
    for (id, n) in &expected {
        let mut feature = builder.feature(Geometry::Point(Point::new(0, 0)));
        feature.id(*id);
        feature.property(value, PropValue::U32(*n)).unwrap();
        feature.finish().unwrap();
    }
    let layer = builder.finish();
    for version in [WireVersion::V01, WireVersion::V02] {
        let bytes = layer
            .clone()
            .encode(unsorted_4g.with_wire_version(version))
            .expect("encode");
        let mut layers = Parser::default().parse_layers(&bytes).expect("parse");
        assert_eq!(layers.len(), 1);
        let decoded = layers
            .remove(0)
            .into_tile(&mut Decoder::default())
            .expect("into_tile")
            .expect("a known layer tag");
        let mut actual: Vec<(Option<u64>, Option<u32>)> = decoded
            .features()
            .iter()
            .map(|f| match f.properties().first() {
                Some(PropValue::U32(n)) => (f.id(), *n),
                _ => (f.id(), None),
            })
            .collect();
        actual.sort_by_key(|row| row.0);
        assert_eq!(actual, expected);
    }
}
