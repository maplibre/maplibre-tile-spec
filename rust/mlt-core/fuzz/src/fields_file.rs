//! `mlt convert --fields` files, shared by the field fuzzers.

use arbitrary::Unstructured;

/// The `split` values forms draw from, which include ones a number kind refuses.
pub(crate) const SPLITS: [&str; 9] = [",", ";", " ", "|", "é", "+", "-", "1", "sign"];

/// A `--fields` file with `forms` under `layer`.
pub(crate) fn fields_file(layer: &str, forms: toml::Table) -> String {
    let mut layers = toml::Table::new();
    layers.insert(layer.to_owned(), forms.into());
    let mut file = toml::Table::new();
    file.insert("layers".to_owned(), layers.into());
    file.to_string()
}

/// A form table, leaving out the keys given as [`None`].
pub(crate) fn form_table(
    split: &str,
    kind: &str,
    running_sum: Option<bool>,
    into: Option<&str>,
) -> toml::Table {
    let mut form = toml::Table::new();
    form.insert("split".to_owned(), split.into());
    form.insert("kind".to_owned(), kind.into());
    if let Some(running_sum) = running_sum {
        form.insert("running-sum".to_owned(), running_sum.into());
    }
    if let Some(into) = into {
        form.insert("into".to_owned(), into.into());
    }
    form
}

/// One value the way packed fields write them, or a near miss.
pub(crate) fn token(u: &mut Unstructured<'_>) -> arbitrary::Result<String> {
    Ok(match u.int_in_range(0..=6u8)? {
        0 => u.arbitrary::<i64>()?.to_string(),
        1 => u.arbitrary::<u64>()?.to_string(),
        2 => u.arbitrary::<i8>()?.to_string(),
        3 => format!("+{}", u.arbitrary::<u8>()?),
        4 => (*u.choose(&[
            "-0",
            "00",
            "01",
            "-2147483648",
            "2147483647",
            "4294967295",
            "4294967296",
            "-9223372036854775808",
            "9223372036854775807",
            "18446744073709551615",
            "18446744073709551616",
        ])?)
        .to_owned(),
        5 => String::new(),
        _ => (0..u.int_in_range(1..=3u8)?)
            .map(|_| {
                u.choose(&['0', '7', '+', '-', ',', ';', ' ', 'é', 'a', '\0'])
                    .copied()
            })
            .collect::<arbitrary::Result<_>>()?,
    })
}
