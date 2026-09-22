//! Pins the values and the JSON `dump::decode_blob` produces, which the wasm binding hands to JS.

use std::fs;

use mlt_core::Decoder;
use mlt_core::dump::{DecodeHint, DecodedBlob, DumpTree, Region, annotate_tile, decode_blob};

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
