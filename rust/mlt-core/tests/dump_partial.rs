//! Checks the partial tree `mlt_core::dump::annotate_tile` returns when a walk bails.

use std::fs;

use mlt_core::dump::{DumpTree, UNANNOTATED, annotate_tile, filter_layer};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/props_shared_dict_no_child_name.mlt"
);

/// The byte the cut tiles below stop at, inside a dictionary stream's payload.
const CUT: usize = 50;

/// Where the walk gives up on a tile cut at [`CUT`]: the end of that payload's header.
const PREFIX_END: usize = 40;

#[test]
fn a_tile_cut_inside_a_stream_payload_seals_every_container_it_left_open() {
    let (tree, err) = annotate_tile(&cut_inside_layer(&whole_tile(), CUT));

    assert!(err.is_some(), "the walk must still report why it bailed");
    insta::assert_snapshot!(container_spans(&tree), @r#"
    layer[0] 0..40
      schema 11..18
        column[0] 11..12
        column[1] 12..18
          column[0] 16..18
      column data 18..40
        column[0] Geometry 18..30
          meta 19..24
            header 19..23
          stream[0] 24..30
            header 24..28
        column[1] SharedDict "a" 30..40
          dict_stream[0] 31..36
            header 31..35
          dict_stream[1] 36..40
            header 36..40
    "#);
}

#[test]
fn a_tile_cut_inside_a_stream_payload_hands_the_rest_to_one_unannotated_leaf() {
    let buf = cut_inside_layer(&whole_tile(), CUT);
    let (tree, _) = annotate_tile(&buf);

    let last = tree.regions.last().expect("the synthetic leaf");
    assert_eq!(last.label, UNANNOTATED);
    assert_eq!(last.offset, PREFIX_END);
    assert_eq!(last.offset + last.len, buf.len());
    assert!(!last.container);
    assert_eq!(
        tree.regions
            .iter()
            .filter(|r| r.label == UNANNOTATED)
            .count(),
        1
    );
}

#[test]
fn filter_layer_on_a_sealed_partial_tree_returns_the_bailed_layers_regions() {
    let (tree, _) = annotate_tile(&cut_inside_layer(&whole_tile(), CUT));
    let layer = filter_layer(&tree, 0).expect("layer 0");

    assert_eq!(layer.buf_len, tree.buf_len);
    assert_eq!(layer.regions.len(), tree.regions.len() - 1);
    assert_eq!(layer.regions.first().expect("the layer").label, "layer[0]");
    assert!(layer.regions.iter().all(|r| r.label != UNANNOTATED));
}

#[test]
fn every_cut_point_in_the_tile_seals_into_a_tree_that_partitions_its_buffer() {
    let whole = whole_tile();

    for keep in 2..whole.len() {
        for buf in [whole[..keep].to_vec(), cut_inside_layer(&whole, keep)] {
            let (tree, _) = annotate_tile(&buf);
            assert_eq!(leaf_coverage(&tree), buf.len(), "cut at {keep} bytes");
        }
    }
}

#[test]
fn a_whole_tile_walks_without_an_error_or_an_unannotated_leaf() {
    let buf = whole_tile();
    let (tree, err) = annotate_tile(&buf);

    assert!(err.is_none(), "{err:?}");
    assert_eq!(leaf_coverage(&tree), buf.len());
    assert!(tree.regions.iter().all(|r| r.label != UNANNOTATED));
}

fn whole_tile() -> Vec<u8> {
    fs::read(FIXTURE).unwrap()
}

/// Every container as `label offset..end`, indented by depth.
fn container_spans(tree: &DumpTree) -> String {
    tree.regions
        .iter()
        .filter(|r| r.container)
        .map(|r| {
            format!(
                "{:width$}{} {}..{}",
                "",
                r.label,
                r.offset,
                r.offset + r.len,
                width = r.depth * 2
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// How far the leaves reach, panicking on the first gap or overlap.
fn leaf_coverage(tree: &DumpTree) -> usize {
    let mut leaves: Vec<(usize, usize)> = tree
        .regions
        .iter()
        .filter(|r| !r.container)
        .map(|r| (r.offset, r.len))
        .collect();
    leaves.sort_unstable();

    let mut cursor = 0;
    for (offset, len) in leaves {
        assert_eq!(offset, cursor, "gap or overlap at offset {offset}");
        cursor += len;
    }
    cursor
}

/// `buf` cut to `keep` bytes, with layer 0's size varint patched to match.
///
/// An unpatched cut bails on the layer body's `take`, two regions in.
fn cut_inside_layer(buf: &[u8], keep: usize) -> Vec<u8> {
    let (size, head) = read_varint(buf);
    let kept_body = keep - head;
    assert!(kept_body < size, "the cut must land inside the layer body");

    let mut out = write_varint(kept_body);
    assert_eq!(
        out.len(),
        head,
        "the patched size must keep its varint width"
    );
    out.extend_from_slice(&buf[head..keep]);
    out
}

/// The varint at the head of `buf`, and how many bytes it occupies.
fn read_varint(buf: &[u8]) -> (usize, usize) {
    let mut value = 0;
    for (i, byte) in buf.iter().enumerate() {
        value |= usize::from(byte & 0x7F) << (7 * i);
        if byte & 0x80 == 0 {
            return (value, i + 1);
        }
    }
    panic!("unterminated varint");
}

fn write_varint(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    while value >= 0x80 {
        out.push(u8::try_from(value & 0x7F).expect("seven bits") | 0x80);
        value >>= 7;
    }
    out.push(u8::try_from(value).expect("under 0x80"));
    out
}
