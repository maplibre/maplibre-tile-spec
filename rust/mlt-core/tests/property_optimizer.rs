use mlt_core::encoder::{
    EncodeProperties as _, Encoder, StagedLayer01, StagedProperty, group_string_properties,
};
use mlt_core::{PropValue, TileFeature, TileLayer01};

fn str_prop(name: &str, values: &[&str]) -> StagedProperty {
    let owned: Vec<Option<String>> = values.iter().map(|s| Some((*s).to_string())).collect();
    StagedProperty::str(name, owned)
}

/// Build a [`TileLayer01`] from heterogeneous column data (one `Vec<PropValue>` per column).
fn tile_from_cols(cols: &[(&str, Vec<PropValue>)]) -> TileLayer01 {
    let n = cols.first().map_or(0, |(_, v)| v.len());
    let property_names = cols.iter().map(|(name, _)| (*name).to_string()).collect();
    let geom = geo_types::Geometry::<i32>::Point(geo_types::Point::new(0, 0));
    let features = (0..n)
        .map(|i| TileFeature {
            id: None,
            geometry: geom.clone(),
            properties: cols.iter().map(|(_, vals)| vals[i].clone()).collect(),
        })
        .collect();
    TileLayer01 {
        name: "test".to_string(),
        extent: 4096,
        property_names,
        features,
    }
}

/// Convert a `&[&str]` slice into a column of `PropValue::Str` values.
fn str_vals(values: &[&str]) -> Vec<PropValue> {
    values
        .iter()
        .map(|s| PropValue::Str(Some((*s).to_string())))
        .collect()
}

/// Stage a [`TileLayer01`] with `MinHash` grouping and return its properties.
fn stage_props(tile: TileLayer01) -> Vec<StagedProperty> {
    let groups = group_string_properties(&tile);
    StagedLayer01::from_tile(tile, &groups).properties
}

#[test]
fn no_nulls_produces_encoded_output() {
    let props = vec![StagedProperty::u32("pop", vec![Some(1), Some(2), Some(3)])];
    let mut enc = Encoder::default();
    let col_count = props.write_to(&mut enc).unwrap();
    assert_eq!(col_count, 1, "non-null column should write one column");
}

#[test]
fn all_nulls_encodes_without_error() {
    let props = vec![StagedProperty::i32("x", vec![None, None, None])];
    let mut enc = Encoder::default();
    // An all-null column writes 0 columns (skipped), which is valid.
    props.write_to(&mut enc).unwrap();
}

#[test]
fn sequential_u32_encodes_successfully() {
    let props = vec![StagedProperty::u32("id", (0u32..1_000).map(Some).collect())];
    let mut enc = Encoder::default();
    let col_count = props.write_to(&mut enc).unwrap();
    assert_eq!(col_count, 1);
}

#[test]
fn constant_u32_encodes_successfully() {
    let props = vec![StagedProperty::u32("val", vec![Some(42); 500])];
    let mut enc = Encoder::default();
    let col_count = props.write_to(&mut enc).unwrap();
    assert_eq!(col_count, 1);
}

#[test]
fn similar_strings_grouped_into_shared_dict() {
    let vocab = &["Alice", "Bob", "Carol", "Dave"];
    let tile = tile_from_cols(&[("name:en", str_vals(vocab)), ("name:de", str_vals(vocab))]);
    let mut enc = Encoder::default();
    let col_count = stage_props(tile).write_to(&mut enc).unwrap();

    assert_eq!(
        col_count, 1,
        "two similar string columns should be merged into one SharedDict"
    );
}

#[test]
fn multiple_similar_string_columns_grouped() {
    let vocab = &["alpha", "beta", "gamma", "delta"];
    let tile = tile_from_cols(&[
        ("addr:zip", str_vals(vocab)),
        ("addr:street", str_vals(vocab)),
        ("addr:zipcode", str_vals(vocab)),
    ]);
    let mut enc = Encoder::default();
    let col_count = stage_props(tile).write_to(&mut enc).unwrap();

    assert_eq!(
        col_count, 1,
        "three similar string columns should be merged"
    );
}

#[test]
fn dissimilar_strings_stay_scalar() {
    let props = vec![
        str_prop("city:de", &["Munich", "Manheim", "Garching"]),
        str_prop("city:colourado", &["Black", "Red", "Gold"]),
    ];
    let mut enc = Encoder::default();
    let col_count = props.write_to(&mut enc).unwrap();
    assert_eq!(col_count, 2, "dissimilar strings should not be merged");
}

#[test]
fn mixed_scalars_and_grouped_strings() {
    let vocab = &["alpha", "beta", "gamma"];
    let tile = tile_from_cols(&[
        ("id", (1u32..=3).map(|v| PropValue::U32(Some(v))).collect()),
        ("name:en", str_vals(vocab)),
        ("name:de", str_vals(vocab)),
        (
            "count",
            [10i32, 20, 30]
                .iter()
                .map(|&v| PropValue::I32(Some(v)))
                .collect(),
        ),
    ]);
    let mut enc = Encoder::default();
    let col_count = stage_props(tile).write_to(&mut enc).unwrap();
    assert_eq!(col_count, 3, "two scalar + one merged dict");
}

#[test]
fn encode_with_explicit_encoder_works() {
    let props = vec![StagedProperty::u32(
        "id",
        (1_000u32..2_000).map(Some).collect(),
    )];
    let mut enc = Encoder::default();
    let col_count = props.write_to(&mut enc).unwrap();
    assert_eq!(col_count, 1);
}
