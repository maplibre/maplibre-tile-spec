//! Verifies the annotated-dump walker (`mlt_core::dump::annotate_tile`) against
//! every synthetic fixture: its leaf regions must partition the whole buffer
//! with no gaps or overlaps, and it must agree with the real parser on whether
//! a file is well-formed.

use std::fs;
use std::path::Path;

use mlt_core::Parser;
use mlt_core::dump::{DumpTree, annotate_tile};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/synthetic/0x01" as dump_0x01 => check }
test_each_path! { for ["mlt"] in "../test/synthetic/0x02" as dump_0x02 => check }

fn check([path]: [&Path; 1]) {
    let buffer = fs::read(path).unwrap();
    let parse_ok = Parser::default().parse_layers(&buffer).is_ok();
    let tree = annotate_tile(&buffer);

    match (parse_ok, tree) {
        // Well-formed per the real parser → the walker must succeed and cover everything.
        (true, Ok(tree)) => assert_full_coverage(&tree, buffer.len(), path),
        (true, Err(e)) => {
            panic!("{}: parser succeeded but annotate_tile failed: {e}", path.display())
        }
        // Malformed → the walker may error too; it must never panic (reaching here proves it didn't).
        (false, _) => {}
    }
}

/// Assert the leaf regions tile `buf_len` exactly: start at 0, contiguous, end at `buf_len`.
fn assert_full_coverage(tree: &DumpTree, buf_len: usize, path: &Path) {
    let mut leaves: Vec<(usize, usize)> = tree
        .regions
        .iter()
        .filter(|r| !r.container)
        .map(|r| (r.offset, r.len))
        .collect();
    leaves.sort_unstable();

    assert_eq!(tree.buf_len, buf_len, "{}: buf_len mismatch", path.display());
    assert!(!leaves.is_empty(), "{}: no leaf regions", path.display());
    assert_eq!(leaves[0].0, 0, "{}: first leaf not at offset 0", path.display());

    let mut cursor = 0usize;
    for (offset, len) in &leaves {
        assert_eq!(
            *offset,
            cursor,
            "{}: gap/overlap at offset {offset} (expected {cursor})",
            path.display()
        );
        cursor += len;
    }
    assert_eq!(
        cursor,
        buf_len,
        "{}: leaves end at {cursor}, expected {buf_len}",
        path.display()
    );
}
