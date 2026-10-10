use std::fs;
use std::path::Path;

use mlt_core::Parser;
use mlt_core::dump::{Region, annotate_tile};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/synthetic/0x01" as bytes_0x01 => matches_the_inspector }
test_each_path! { for ["mlt"] in "../test/synthetic/0x01-rust" as bytes_0x01_rust => matches_the_inspector }
test_each_path! { for ["mlt"] in "../test/synthetic/0x02" as bytes_0x02 => matches_the_inspector }

fn matches_the_inspector([path]: [&Path; 1]) {
    let buffer = fs::read(path).unwrap();
    let Ok(layers) = Parser::default().parse_layers(&buffer) else {
        return;
    };
    let parsed: Vec<_> = layers
        .iter()
        .filter_map(mlt_core::Layer::bytes)
        .map(|b| [b.size(), b.geometry(), b.properties(), b.ids()].map(|n| n as usize))
        .collect();
    let (tree, err) = annotate_tile(&buffer);
    assert!(err.is_none(), "{}: {err:?}", path.display());

    assert_eq!(parsed, inspector_bytes(&tree.regions), "{}", path.display());
}

/// The classification `inspector/src/tileStats.ts` applies to the annotated regions.
fn inspector_bytes(regions: &[Region]) -> Vec<[usize; 4]> {
    let mut layers = Vec::new();
    for (at, layer) in regions.iter().enumerate() {
        if layer.depth != 0 || !layer.label.starts_with("layer[") {
            continue;
        }
        let kids = regions[at + 1..]
            .iter()
            .take_while(|r| r.depth > 0)
            .filter(|r| r.depth == 1);
        let mut size = layer.len;
        let (mut geometry, mut properties, mut ids) = (0, 0, 0);
        for kid in kids {
            if kid.label == "size" {
                size -= kid.len;
            }
            if !kid.container {
                continue;
            }
            let typ = kid
                .label
                .strip_prefix("column[")
                .or_else(|| kid.label.strip_prefix("m_value["))
                .and_then(|rest| rest.split(' ').nth(1));
            match (kid.label.as_str(), typ) {
                ("geometry", _) | (_, Some("Geometry")) => geometry += kid.len,
                (_, Some("Id" | "OptId" | "LongId" | "OptLongId")) => ids += kid.len,
                (_, Some(_)) => properties += kid.len,
                _ => {}
            }
        }
        layers.push([size, geometry, properties, ids]);
    }
    layers
}
