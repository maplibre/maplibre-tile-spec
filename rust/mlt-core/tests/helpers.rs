//! Shared test helpers for mlt-core integration tests.

use std::borrow::Cow;

use mlt_core::v01::{ParsedProperty, ParsedStrings};

/// Build a [`ParsedProperty::Str`] from a list of optional owned strings.
///
/// This is a test-only helper for constructing the decoded form of a string
/// column directly from owned data, without a wire-format round-trip.
#[must_use]
pub fn parsed_str_prop(name: &str, values: Vec<Option<String>>) -> ParsedProperty<'_> {
    let mut lengths = Vec::with_capacity(values.len());
    let mut data = String::new();
    let mut end = 0_i32;
    for value in values {
        match value {
            Some(value) => {
                end = end
                    .checked_add(i32::try_from(value.len()).expect("string too long"))
                    .expect("decoded string corpus exceeds supported i32 range");
                lengths.push(end);
                data.push_str(&value);
            }
            None => lengths.push(-end - 1),
        }
    }
    ParsedProperty::Str(ParsedStrings::new(name, lengths, Cow::Owned(data)))
}
