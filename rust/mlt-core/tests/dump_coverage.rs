//! Checks `mlt_core::dump::annotate_tile` and `mlt_core::dump::render`

use std::fs;
use std::path::Path;

use mlt_core::Parser;
use mlt_core::dump::{DumpTree, RenderOpts, annotate_tile, render};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/synthetic/0x01" as dump_0x01 => check }
test_each_path! { for ["mlt"] in "../test/synthetic/0x01-rust" as dump_0x01_rust => check }
test_each_path! { for ["mlt"] in "../test/synthetic/0x02" as dump_0x02 => check }
test_each_path! { for ["mlt"] in "../test/synthetic/0x02-java" as dump_0x02_java => check }

fn check([path]: [&Path; 1]) {
    let buffer = fs::read(path).unwrap();
    let parse_ok = Parser::default().parse_layers(&buffer).is_ok();
    let (tree, err) = annotate_tile(&buffer);

    match (parse_ok, err) {
        // Well-formed per the real parser -> the walker must succeed; malformed -> it must
        // bail. Either way its leaves cover the buffer, a bailed walk's through the
        // synthetic leaf, and it must never panic (reaching here proves it didn't).
        (true, None) | (false, Some(_)) => {
            assert_full_coverage(&tree, buffer.len(), path);
            assert_eq!(
                rendered_bytes(&tree, &buffer, path),
                buffer,
                "{}: the hexdump's bytes are not the file's",
                path.display()
            );
        }
        (true, Some(e)) => {
            panic!(
                "{}: parser succeeded but annotate_tile failed: {e}",
                path.display()
            )
        }
        (false, None) => {
            panic!(
                "{}: parser failed but annotate_tile succeeded",
                path.display()
            )
        }
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

    assert_eq!(
        tree.buf_len,
        buf_len,
        "{}: buf_len mismatch",
        path.display()
    );
    assert!(!leaves.is_empty(), "{}: no leaf regions", path.display());
    assert_eq!(
        leaves[0].0,
        0,
        "{}: first leaf not at offset 0",
        path.display()
    );

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

/// The bytes the hexdump printed in its offset and hex columns, panicking on the first row that skips or repeats one.
///
/// `max_blob` is off because the default elides long payloads.
fn rendered_bytes(tree: &DumpTree, buf: &[u8], path: &Path) -> Vec<u8> {
    let opts = RenderOpts {
        max_blob: 0,
        ..RenderOpts::default()
    };
    let mut out = Vec::new();
    render(tree, buf, &opts, &mut out).expect("render");
    let text = String::from_utf8(out).expect("the hexdump is utf-8");

    // "{offset:08x}  " then `width` "xx " cells, both blank on the rows that carry only an annotation.
    let hex = 10..10 + opts.width * 3;
    let mut bytes = Vec::new();
    for line in text.lines() {
        let cells = line
            .get(hex.clone())
            .expect("every row is padded past its hex column");
        if cells.trim().is_empty() {
            continue;
        }
        let offset = usize::from_str_radix(&line[..8], 16).expect("an offset column");
        assert_eq!(
            offset,
            bytes.len(),
            "{}: row at {offset} does not continue the previous one",
            path.display()
        );
        for cell in cells.split_whitespace() {
            bytes.push(u8::from_str_radix(cell, 16).expect("a hex byte"));
        }
    }
    bytes
}
