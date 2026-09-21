//! Checks every `docs/snippets/0x0N/*.mlt.hexdump` still matches what `mlt hexdump` prints.
#![cfg(all(feature = "unstable-v2", not(feature = "hotpath")))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use test_each_file::test_each_path;

test_each_path! { for ["hexdump"] in "../docs/snippets/0x01" as docs_0x01 => check }
test_each_path! { for ["hexdump"] in "../docs/snippets/0x02" as docs_0x02 => check }

/// The snippet must be byte-identical to a bare `mlt hexdump`, which is what `just rust::sync-docs-hexdumps` writes.
fn check([snippet]: [&Path; 1]) {
    let fixture = fixture_for(snippet);
    let out = Command::new(env!("CARGO_BIN_EXE_mlt"))
        .args(["hexdump", fixture.to_str().unwrap()])
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("mlt hexdump");

    assert!(
        out.status.success(),
        "{}: {}",
        fixture.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        fs::read_to_string(snippet).unwrap(),
        "{} is stale, run `just rust::sync-docs-hexdumps`",
        snippet.display()
    );
}

/// The `test/synthetic` fixture a snippet is generated from, paired by directory and file stem.
fn fixture_for(snippet: &Path) -> PathBuf {
    let dir = snippet
        .parent()
        .and_then(Path::file_name)
        .expect("a 0x0N directory");
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../test/synthetic"))
        .join(dir)
        .join(snippet.file_stem().expect("a <fixture>.mlt.hexdump name"))
}
