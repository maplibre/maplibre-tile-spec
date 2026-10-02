//! Pins the values and the JSON `dump::decode_blob` produces, which the wasm binding hands to JS.

use std::fs;

use mlt_core::Decoder;
use mlt_core::dump::{
    DecodeHint, DecodedBlob, DumpTree, Region, annotate_tile, decode_blob, decode_region,
};

const ID: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/id.mlt"
);

const ID64_MAX: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01-rust/id64_max_delta.mlt"
);

const PROPS_STR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/props_str.mlt"
);

const POLY_HOLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/poly_hole.mlt"
);

const MIX_PT_POLY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/mix_2_pt_poly.mlt"
);

#[cfg(feature = "unstable-v2")]
const Z_MIX: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x02/z_mix.mlt"
);

const FSST_V1: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/props_shared_dict_fsst.mlt"
);

#[cfg(feature = "unstable-v2")]
const FSST_V2: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x02/props_str_fsst.mlt"
);

#[test]
fn an_int_payload_decodes_to_numbers() {
    let (tree, buf) = annotate(ID);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::I32 | DecodeHint::U32));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 0)),
        @r#"{"kind":"numbers","values":[100.0],"truncatedFrom":null}"#
    );
}

#[test]
fn a_64_bit_payload_decodes_to_bigints() {
    let (tree, buf) = annotate(ID64_MAX);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::U64));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 0)),
        @r#"{"kind":"bigints","values":[18446744073709551615],"truncatedFrom":null}"#
    );
}

#[test]
fn a_string_payload_decodes_to_text() {
    let (tree, buf) = annotate(PROPS_STR);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::Bytes));

    let DecodedBlob::Text { value } = decode(region, &buf, 0) else {
        panic!("a text payload");
    };
    assert!(
        value.starts_with("residential_zone_north_sector_1"),
        "{value}"
    );
}

#[test]
fn max_values_caps_the_values_and_reports_the_full_count() {
    let (tree, buf) = annotate(POLY_HOLE);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::I32));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 3)),
        @r#"{"kind":"numbers","values":[11.0,52.0,71.0],"truncatedFrom":12}"#
    );
}

#[test]
fn a_v1_geometry_types_stream_decodes_to_names_beside_its_numbers() {
    let (tree, buf) = annotate(MIX_PT_POLY);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::GeometryType));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 0)),
        @r#"{"kind":"enum","values":[0,2],"names":["Point","Polygon"],"truncatedFrom":null}"#
    );
}

#[cfg(feature = "unstable-v2")]
#[test]
fn a_v2_geometry_types_stream_decodes_to_names_beside_its_numbers() {
    let (tree, buf) = annotate(Z_MIX);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::GeometryType));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 0)),
        @r#"{"kind":"enum","values":[0,1,2,2,3,4,5],"names":["Point","LineString","Polygon","Polygon","MultiPoint","MultiLineString","MultiPolygon"],"truncatedFrom":null}"#
    );
}

#[test]
fn max_values_caps_names_and_numbers_alike() {
    let (tree, buf) = annotate(MIX_PT_POLY);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::GeometryType));

    insta::assert_snapshot!(
        json(&decode(region, &buf, 1)),
        @r#"{"kind":"enum","values":[0],"names":["Point"],"truncatedFrom":2}"#
    );
}

#[test]
fn a_known_geometry_type_borrows_its_name_rather_than_allocating_one() {
    let (tree, buf) = annotate(MIX_PT_POLY);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::GeometryType));

    let DecodedBlob::Enum { names, .. } = decode(region, &buf, 0) else {
        panic!("an enum payload");
    };
    assert!(
        names
            .iter()
            .all(|n| matches!(n, std::borrow::Cow::Borrowed(_)))
    );
}

#[test]
fn a_geometry_type_no_geometry_has_is_named_unknown() {
    let (tree, _) = annotate(MIX_PT_POLY);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::GeometryType));
    let blob = region.blob.expect("stream metadata");

    // Two varints: a `Point`, then 7, past the last of the six types.
    insta::assert_snapshot!(
        json(&decode_blob(blob, &[0, 7], 0, &mut Decoder::default())),
        @r#"{"kind":"enum","values":[0,7],"names":["Point","unknown(7)"],"truncatedFrom":null}"#
    );
}

/// The corpus is the only payload there that is `Data(Single)` or `Data(Shared)`.
fn corpus_index(tree: &DumpTree) -> usize {
    tree.regions
        .iter()
        .position(|r| {
            r.blob
                .is_some_and(|b| b.meta.stream_type.to_string().starts_with("data[s"))
        })
        .expect("a corpus")
}

fn decode_at(tree: &DumpTree, buf: &[u8], index: usize, max_values: usize) -> DecodedBlob {
    decode_region(tree, buf, index, max_values, &mut Decoder::default())
}

#[cfg(feature = "unstable-v2")]
#[test]
fn a_v2_fsst_corpus_decodes_to_the_strings_it_stands_for() {
    let (tree, buf) = annotate(FSST_V2);
    insta::assert_snapshot!(
        json(&decode_at(&tree, &buf, corpus_index(&tree), 0)),
        @r#"{"kind":"strings","values":["residential_zone_north_sector_1","commercial_zone_south_sector_2","industrial_zone_east_sector_3","park_zone_west_sector_4","water_zone_north_sector_5","residential_zone_south_sector_6"],"truncatedFrom":null}"#
    );
}

#[test]
fn a_v1_fsst_corpus_decodes_to_the_strings_it_stands_for() {
    let (tree, buf) = annotate(FSST_V1);
    insta::assert_snapshot!(
        json(&decode_at(&tree, &buf, corpus_index(&tree), 0)),
        @r#"{"kind":"strings","values":["AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"],"truncatedFrom":null}"#
    );
}

#[cfg(feature = "unstable-v2")]
#[test]
fn max_values_caps_the_strings_and_reports_the_full_count() {
    let (tree, buf) = annotate(FSST_V2);
    insta::assert_snapshot!(
        json(&decode_at(&tree, &buf, corpus_index(&tree), 2)),
        @r#"{"kind":"strings","values":["residential_zone_north_sector_1","commercial_zone_south_sector_2"],"truncatedFrom":6}"#
    );
}

#[test]
fn a_payload_that_is_not_a_corpus_decodes_as_decode_blob_does() {
    let (tree, buf) = annotate(POLY_HOLE);
    let at = tree
        .regions
        .iter()
        .position(|r| r.blob.is_some_and(|b| matches!(b.hint, DecodeHint::I32)))
        .expect("a vertex stream");
    assert_eq!(
        json(&decode_at(&tree, &buf, at, 3)),
        json(&decode(&tree.regions[at], &buf, 3))
    );
}

#[test]
fn a_region_that_is_not_a_payload_is_an_error_not_a_panic() {
    let (tree, buf) = annotate(POLY_HOLE);
    let DecodedBlob::Error { message } = decode_at(&tree, &buf, 0, 0) else {
        panic!("an error");
    };
    assert!(message.contains("no stream metadata"), "{message}");
    let DecodedBlob::Error { message } = decode_at(&tree, &buf, usize::MAX, 0) else {
        panic!("an error");
    };
    assert!(message.contains("no region"), "{message}");
}

#[test]
fn an_undecodable_payload_decodes_to_an_error() {
    let (tree, buf) = annotate(ID);
    let region = first_hint(&tree, |h| matches!(h, DecodeHint::I32 | DecodeHint::U32));
    let blob = region.blob.expect("stream metadata");

    insta::assert_snapshot!(
        json(&decode_blob(blob, &buf[..0], 0, &mut Decoder::default())),
        @r#"{"kind":"error","message":"buffer underflow: needed 1 bytes, but only 0 remain"}"#
    );
}

fn annotate(path: &str) -> (DumpTree, Vec<u8>) {
    let buf = fs::read(path).unwrap();
    let (tree, err) = annotate_tile(&buf);
    assert!(err.is_none(), "annotate_tile: {err:?}");
    (tree, buf)
}

fn first_hint(tree: &DumpTree, want: impl Fn(DecodeHint) -> bool) -> &Region {
    tree.regions
        .iter()
        .find(|r| matches!(r.blob, Some(blob) if want(blob.hint)))
        .expect("a matching payload")
}

fn decode(region: &Region, buf: &[u8], max_values: usize) -> DecodedBlob {
    let blob = region.blob.expect("stream metadata");
    let bytes = &buf[region.offset..region.offset + region.len];
    decode_blob(blob, bytes, max_values, &mut Decoder::default())
}

fn json(decoded: &DecodedBlob) -> String {
    serde_json::to_string(decoded).unwrap()
}
