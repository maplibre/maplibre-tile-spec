//! The columnar accessors of a parsed layer agree with its per-feature iterator.
//!
//! Bindings such as `mlt-wasm` read ids and properties column by column through the public
//! API, so every type they match on has to be nameable outside the crate.

use std::path::Path;

use mlt_core::{
    Decoder, LendingIterator as _, ParsedId, ParsedLayer, ParsedLayer01, ParsedProperty,
    ParsedSharedDict, ParsedStrings, Parser, PresentValues, PropValueRef,
};
use test_each_file::test_each_path;

test_each_path! { for ["mlt"] in "../test/synthetic" as columnar_access => columnar_matches_rows }

fn columnar_matches_rows([path]: [&Path; 1]) {
    let data = std::fs::read(path).unwrap();
    let layers = Parser::default().parse_layers(&data).unwrap();
    let mut dec = Decoder::default();
    for layer in layers {
        let parsed = layer.decode_all(&mut dec).unwrap();
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "ParsedLayer is non_exhaustive"
        )]
        let layer: &ParsedLayer01<'_> = match &parsed {
            ParsedLayer::Tag01(l) => l,
            ParsedLayer::Tag02(l) => l.layer(),
            _ => continue,
        };
        assert_layer(layer);
    }
}

fn assert_layer(layer: &ParsedLayer01<'_>) {
    let n = layer.feature_count();

    let mut row_ids = Vec::with_capacity(n);
    let mut rows: Vec<Vec<Option<PropValueRef<'_>>>> = Vec::with_capacity(n);
    let mut iter = layer.iter_features();
    while let Some(feature) = iter.next() {
        let feature = feature.unwrap();
        row_ids.push(feature.id());
        rows.push(feature.iter_all_properties().collect());
    }
    assert_eq!(rows.len(), n);

    let ids: Option<&ParsedId<'_>> = layer.id();
    let column_ids = ids.map_or_else(|| vec![None; n], |ids| ids.materialize());
    assert_eq!(column_ids, row_ids, "ids");

    let columns: Vec<Vec<Option<PropValueRef<'_>>>> = layer
        .properties()
        .iter()
        .flat_map(|p| columns(p, n))
        .collect();
    let names: Vec<String> = layer.iterate_prop_names().map(|n| n.to_string()).collect();
    assert_eq!(columns.len(), names.len(), "one name per column");
    for (c, column) in columns.iter().enumerate() {
        assert_eq!(column.len(), n, "column {}", names[c]);
        for (f, value) in column.iter().enumerate() {
            // Debug output tells NaN apart from nothing and -0.0 from 0.0, where `==` does not.
            assert_eq!(
                format!("{value:?}"),
                format!("{:?}", rows[f][c]),
                "column {} feature {f}",
                names[c]
            );
        }
    }
}

/// Every column a property holds, one value per feature: a shared dictionary holds one per child.
fn columns<'p>(property: &'p ParsedProperty<'_>, n: usize) -> Vec<Vec<Option<PropValueRef<'p>>>> {
    fn scalar<'p, T: Copy + PartialEq + Into<PropValueRef<'p>>>(
        presence: &PresentValues<'_, T>,
    ) -> Vec<Vec<Option<PropValueRef<'p>>>> {
        vec![
            presence
                .iter_optional()
                .map(|v| v.map(Into::into))
                .collect(),
        ]
    }
    fn strings<'p>(s: &'p ParsedStrings<'_>, n: usize) -> Vec<Vec<Option<PropValueRef<'p>>>> {
        let n = u32::try_from(n).unwrap();
        vec![(0..n).map(|i| s.get(i).map(PropValueRef::Str)).collect()]
    }
    fn shared<'p>(d: &'p ParsedSharedDict<'_>, n: usize) -> Vec<Vec<Option<PropValueRef<'p>>>> {
        d.items()
            .iter()
            .map(|item| {
                assert_eq!(item.feature_count(), n);
                (0..n)
                    .map(|i| item.get(d, i).map(PropValueRef::Str))
                    .collect()
            })
            .collect()
    }
    match property {
        ParsedProperty::Bool(v) => scalar(v),
        ParsedProperty::I8(v) => scalar(v),
        ParsedProperty::U8(v) => scalar(v),
        ParsedProperty::I32(v) => scalar(v),
        ParsedProperty::U32(v) => scalar(v),
        ParsedProperty::I64(v) => scalar(v),
        ParsedProperty::U64(v) => scalar(v),
        ParsedProperty::F32(v) => scalar(v),
        ParsedProperty::F64(v) => scalar(v),
        ParsedProperty::Str(s) => strings(s, n),
        ParsedProperty::SharedDict(d) => shared(d, n),
        other => panic!("unhandled property type {:?}", other.kind()),
    }
}
