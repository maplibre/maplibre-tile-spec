//! Pins the JSON the annotated dump's model serializes to, which the wasm binding mirrors.

use std::fs;

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

    insta::assert_snapshot!(
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

    insta::assert_snapshot!(json(region), @r#"
    {
      "offset": 11,
      "len": 1,
      "depth": 3,
      "label": "type",
      "value": "0x04 Geometry",
      "bits": [
        {
          "hi": 7,
          "lo": 1,
          "raw": 2,
          "meaning": "base type = Geometry"
        },
        {
          "hi": 0,
          "lo": 0,
          "raw": 0,
          "meaning": "not optional: each feature has a non-NULL value"
        }
      ],
      "kind": "meta",
      "container": false,
      "blob": null
    }
    "#);
}

#[test]
fn a_data_blob_region_serializes_its_stream_metadata_as_display_strings() {
    let tree = annotate(SHARED_DICT);
    let region = tree
        .regions
        .iter()
        .find(|r| r.blob.is_some())
        .expect("a stream payload");

    insta::assert_snapshot!(json(region), @r#"
    {
      "offset": 23,
      "len": 1,
      "depth": 4,
      "label": "data",
      "value": null,
      "bits": [],
      "kind": "dataBlob",
      "container": false,
      "blob": {
        "streamType": "length[var-binary]",
        "logical": "int/none",
        "physical": "varint",
        "numValues": 1,
        "hint": {
          "kind": "u32"
        }
      }
    }
    "#);
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

    insta::assert_snapshot!(json(region), @r#"
    {
      "offset": 40,
      "len": 7,
      "depth": 4,
      "label": "data",
      "value": null,
      "bits": [],
      "kind": "dataBlob",
      "container": false,
      "blob": {
        "streamType": "data",
        "logical": "float/alp",
        "physical": "varint",
        "numValues": 4,
        "hint": {
          "kind": "alp",
          "e": 2,
          "f": 0,
          "base": -225
        }
      }
    }
    "#);
}

fn annotate(path: &str) -> DumpTree {
    let (tree, err) = annotate_tile(&fs::read(path).unwrap());
    assert!(err.is_none(), "annotate_tile: {err:?}");
    tree
}

fn json(region: &Region) -> String {
    serde_json::to_string_pretty(region).unwrap()
}
