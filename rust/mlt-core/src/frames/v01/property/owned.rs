use super::{
    ParsedProperty, ParsedScalar, ParsedScalarFam, ParsedSharedDict, ParsedSharedDictItem,
    ParsedStrings, Scalar, StagedProperty, StagedScalar, StagedScalarFam, StagedSharedDict,
    StagedSharedDictItem, StagedStrings, scalar_match,
};

impl<T: Copy + PartialEq + std::fmt::Debug> PartialEq<StagedScalar<T>> for ParsedScalar<'_, T> {
    fn eq(&self, other: &StagedScalar<T>) -> bool {
        let Self { name, values } = self;
        let StagedScalar {
            name: other_name,
            values: other_values,
        } = other;
        name == other_name && values == other_values
    }
}

impl<T: Copy + PartialEq + std::fmt::Debug> PartialEq<ParsedScalar<'_, T>> for StagedScalar<T> {
    fn eq(&self, other: &ParsedScalar<'_, T>) -> bool {
        other == self
    }
}

impl PartialEq<StagedStrings> for ParsedStrings<'_> {
    fn eq(&self, other: &StagedStrings) -> bool {
        let Self {
            name,
            lengths,
            data,
        } = self;
        let StagedStrings {
            name: other_name,
            lengths: other_lengths,
            data: other_data,
        } = other;
        name == other_name && lengths == other_lengths && **data == *other_data
    }
}

impl PartialEq<ParsedStrings<'_>> for StagedStrings {
    fn eq(&self, other: &ParsedStrings<'_>) -> bool {
        other == self
    }
}

impl PartialEq<StagedSharedDictItem> for ParsedSharedDictItem<'_> {
    fn eq(&self, other: &StagedSharedDictItem) -> bool {
        let Self { suffix, ranges } = self;
        let StagedSharedDictItem {
            suffix: other_suffix,
            ranges: other_ranges,
        } = other;
        suffix == other_suffix && ranges == other_ranges
    }
}

impl PartialEq<ParsedSharedDictItem<'_>> for StagedSharedDictItem {
    fn eq(&self, other: &ParsedSharedDictItem<'_>) -> bool {
        other == self
    }
}

impl PartialEq<StagedSharedDict> for ParsedSharedDict<'_> {
    fn eq(&self, other: &StagedSharedDict) -> bool {
        let Self {
            prefix,
            data,
            items,
        } = self;
        let StagedSharedDict {
            prefix: other_prefix,
            data: other_data,
            items: other_items,
        } = other;
        prefix == other_prefix
            && **data == *other_data
            && items.len() == other_items.len()
            && items.iter().zip(other_items).all(|(a, b)| a == b)
    }
}

impl PartialEq<ParsedSharedDict<'_>> for StagedSharedDict {
    fn eq(&self, other: &ParsedSharedDict<'_>) -> bool {
        other == self
    }
}

impl PartialEq<Scalar<StagedScalarFam>> for Scalar<ParsedScalarFam<'_>> {
    fn eq(&self, other: &Scalar<StagedScalarFam>) -> bool {
        scalar_match!(self, other, a, b => a == b, else false)
    }
}

impl PartialEq<Scalar<ParsedScalarFam<'_>>> for Scalar<StagedScalarFam> {
    fn eq(&self, other: &Scalar<ParsedScalarFam<'_>>) -> bool {
        other == self
    }
}

impl PartialEq<StagedProperty> for ParsedProperty<'_> {
    fn eq(&self, other: &StagedProperty) -> bool {
        match (self, other) {
            (Self::Scalar(a), StagedProperty::Scalar(b)) => a == b,
            (Self::Str(a), StagedProperty::Str(b)) => a == b,
            (Self::SharedDict(a), StagedProperty::SharedDict(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialEq<ParsedProperty<'_>> for StagedProperty {
    fn eq(&self, other: &ParsedProperty<'_>) -> bool {
        other == self
    }
}
