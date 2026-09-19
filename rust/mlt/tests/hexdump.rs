//! Checks what `mlt hexdump` prints for a tile the walker cannot finish.

// hotpath appends its profile to both streams, which no exact snapshot can survive.
#![cfg(not(feature = "hotpath"))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test/synthetic/0x01/props_shared_dict_no_child_name.mlt"
);

#[test]
fn a_truncated_tile_prints_its_annotated_prefix_and_the_unannotated_rest() {
    let out = hexdump(truncated_fixture());

    insta::assert_snapshot!(String::from_utf8(out.stdout).unwrap(), @r"
    00000000                                                                     | layer[0] (2 B)
    00000000  51                                                Q                |   size: 81 (varint) - tag + body
    00000001  01                                                .                |   tag: 0x01 -> Tag01
    00000002  06 6c 61 79 65 72 31 50 02 04 1e 01 61 01 1d 00   .layer1P....a... | <unannotated> [18 B]
    00000012  02 30                                             .0               |
    ");
}

#[test]
fn a_truncated_tile_reports_the_error_and_exits_non_zero() {
    let out = hexdump(truncated_fixture());

    assert!(!out.status.success());
    insta::assert_snapshot!(
        String::from_utf8(out.stderr).unwrap(),
        @"Error: unexpected end of input (unable to take 80 bytes)"
    );
}

#[test]
fn a_layer_the_walk_never_reached_reports_the_walk_error_not_a_range_error() {
    let out = hexdump_layer(truncated_fixture(), 1);

    assert!(!out.status.success());
    insta::assert_snapshot!(String::from_utf8(out.stdout).unwrap(), @"");
    insta::assert_snapshot!(
        String::from_utf8(out.stderr).unwrap(),
        @"Error: unexpected end of input (unable to take 80 bytes)"
    );
}

#[test]
fn a_layer_past_the_end_of_a_whole_tile_is_still_out_of_range() {
    let out = hexdump_layer(Path::new(FIXTURE), 1);

    assert!(!out.status.success());
    insta::assert_snapshot!(
        String::from_utf8(out.stderr).unwrap(),
        @"Error: layer index 1 out of range"
    );
}

fn hexdump(tile: &Path) -> Output {
    mlt_hexdump(tile, &[])
}

fn hexdump_layer(tile: &Path, idx: usize) -> Output {
    mlt_hexdump(tile, &["--layer", &idx.to_string()])
}

/// CI exports `RUST_BACKTRACE=1`, and an inherited backtrace on stderr would break every snapshot here.
fn mlt_hexdump(tile: &Path, extra_args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlt"))
        .args(["hexdump", tile.to_str().unwrap()])
        .args(["--color", "never", "--no-bits"])
        .args(extra_args)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("mlt hexdump")
}

/// The fixture's first 20 bytes, which stop inside the layer body its size varint promises.
fn truncated_fixture() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("truncated.mlt");
        fs::write(&path, &fs::read(FIXTURE).unwrap()[..20]).unwrap();
        path
    })
}
