//! Checks every inspector link in the spec pages still names a tile and a region of it.
#![cfg(not(feature = "hotpath"))]

use std::fs;
use std::path::{Path, PathBuf};

use mlt_core::dump;

/// The pages that point readers at the inspector.
const PAGES: [&str; 3] = [
    "../../docs/encodings.md",
    "../../docs/specification/v1.md",
    "../../docs/specification/v2.md",
];

/// One `?fixture=<key>` link, with the `&at=<label>` region it names, if any.
struct Link {
    page: &'static str,
    fixture: String,
    at: Option<String>,
}

#[test]
fn every_inspector_link_names_a_tile_and_a_region() {
    let links = links();
    assert!(!links.is_empty(), "the pages carry no inspector links");

    for link in &links {
        let path = fixture(&link.fixture);
        let bytes = fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {} names {}: {e}",
                link.page,
                link.fixture,
                path.display()
            )
        });
        let Some(label) = &link.at else { continue };

        let (tree, err) = dump::annotate_tile(&bytes);
        assert!(
            err.is_none(),
            "{}: {} did not annotate",
            link.page,
            link.fixture
        );
        let hits = tree
            .regions
            .iter()
            .filter(|region| names(&region.label, label))
            .count();
        assert_eq!(
            hits, 1,
            "{}: `at={label}` matches {hits} regions of {}, so the link names nothing \
             definite. Pick a label that appears once.",
            link.page, link.fixture
        );
    }
}

/// The same rule the app resolves `at` by: whole, or up to the first detail.
fn names(label: &str, wanted: &str) -> bool {
    label == wanted
        || label.starts_with(&format!("{wanted} "))
        || label.starts_with(&format!("{wanted}["))
}

fn fixture(key: &str) -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../test/synthetic")).join(key)
}

/// Every `inspector/app/?fixture=...` link the pages hold.
fn links() -> Vec<Link> {
    let mut out = Vec::new();
    for page in PAGES {
        let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(page))
            .unwrap_or_else(|e| panic!("{page}: {e}"));
        for rest in text.split("inspector/app/?").skip(1) {
            let query = &rest[..rest.find(')').unwrap_or(rest.len())];
            let mut fixture = None;
            let mut at = None;
            for pair in query.split('&') {
                match pair.split_once('=') {
                    Some(("fixture", v)) => fixture = Some(unescape(v)),
                    Some(("at", v)) => at = Some(unescape(v)),
                    _ => {}
                }
            }
            if let Some(fixture) = fixture {
                out.push(Link { page, fixture, at });
            }
        }
    }
    out
}

/// The few percent-escapes these links use, which is all a doc link needs.
fn unescape(raw: &str) -> String {
    raw.replace("%2F", "/")
        .replace("%20", " ")
        .replace("%22", "\"")
        .replace("%5B", "[")
        .replace("%5D", "]")
}

/// A link that leaves the block it sits in takes the block's own body with it: the
/// text after it reparses as an indented code block, so a tab renders empty and its
/// prose turns into literal markdown. The build still succeeds, hence this check.
#[test]
fn every_inspector_link_stays_inside_its_block() {
    for page in PAGES {
        let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(page))
            .unwrap_or_else(|e| panic!("{page}: {e}"));
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if !line.contains("inspector/app/?fixture=") {
                continue;
            }
            let Some(above) = lines[..at].iter().rev().find(|l| !l.trim().is_empty()) else {
                continue;
            };
            let (deep, over) = (indent(line), indent(above));
            let opens = ["=== ", "??? ", "!!! ", "+++ "]
                .iter()
                .any(|p| above.trim_start().starts_with(p));
            assert!(
                if opens { deep > over } else { deep >= over },
                "{page}:{}: the link is indented {deep} under a line indented {over} \
                 ({:?}), so it falls out of that block. Indent it into the body.",
                at + 1,
                above.trim()
            );
        }
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
