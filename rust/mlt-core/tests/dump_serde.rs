//! Pins the JSON the annotated dump's model serializes to, which the wasm binding mirrors.

use std::fs;

use insta::assert_snapshot;
#[cfg(feature = "unstable-v2")]
use mlt_core::dump::DecodeHint;
use mlt_core::dump::{DumpTree, Region, annotate_tile};

const SHARED_DICT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/props_shared_dict_no_child_name.mlt"
);

#[cfg(feature = "unstable-v2")]
const F64_ALP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x02/prop_f64_alp.mlt"
);

#[test]
fn a_tree_serializes_its_own_fields_as_camel_case() {
    let tree = DumpTree {
        buf_len: 0,
        regions: Vec::new(),
    };

    assert_snapshot!(
        serde_json::to_string(&tree).unwrap(),
        @r#"{"bufLen":0,"regions":[]}"#
    );
}

#[test]
fn a_bit_packed_meta_region_serializes_its_bit_breakdown() {
    let tree = annotate(SHARED_DICT);
    let region = tree
        .regions
        .iter()
        .find(|r| !r.bits.is_empty())
        .expect("a packed byte");

    assert_snapshot!(json(region));
}

#[test]
fn a_data_blob_region_serializes_its_stream_metadata_as_display_strings() {
    let tree = annotate(SHARED_DICT);
    let region = tree
        .regions
        .iter()
        .find(|r| r.blob.is_some())
        .expect("a stream payload");

    assert_snapshot!(json(region));
}

#[cfg(feature = "unstable-v2")]
#[test]
fn an_alp_hint_serializes_its_parameters_beside_its_tag() {
    let tree = annotate(F64_ALP);
    let region = tree
        .regions
        .iter()
        .find(|r| matches!(r.blob, Some(blob) if matches!(blob.hint, DecodeHint::Alp(_))))
        .expect("an ALP payload");

    assert_snapshot!(json(region));
}

fn annotate(path: &str) -> DumpTree {
    let (tree, err) = annotate_tile(&fs::read(path).unwrap());
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn json(region: &Region) -> String {
    serde_json::to_string_pretty(region).unwrap()
}
