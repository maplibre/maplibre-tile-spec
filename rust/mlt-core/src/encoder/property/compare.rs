use crate::decoder::{
    ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
};
use crate::encoder::{
    StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};

impl<T: Copy + PartialEq> PartialEq<StagedScalar<T>> for ParsedScalar<'_, T> {
    fn eq(&self, other: &StagedScalar<T>) -> bool {
        let Self { name, values } = self;
        let StagedScalar {
            name: other_name,
            values: other_values,
        } = other;
        name == other_name && values == other_values
    }
}

impl<T: Copy + PartialEq> PartialEq<ParsedScalar<'_, T>> for StagedScalar<T> {
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

impl PartialEq<StagedProperty> for ParsedProperty<'_> {
    fn eq(&self, other: &StagedProperty) -> bool {
        match (self, other) {
            (Self::Bool(a), StagedProperty::Bool(b)) => a == b,
            (Self::I8(a), StagedProperty::I8(b)) => a == b,
            (Self::U8(a), StagedProperty::U8(b)) => a == b,
            (Self::I32(a), StagedProperty::I32(b)) => a == b,
            (Self::U32(a), StagedProperty::U32(b)) => a == b,
            (Self::I64(a), StagedProperty::I64(b)) => a == b,
            (Self::U64(a), StagedProperty::U64(b)) => a == b,
            (Self::F32(a), StagedProperty::F32(b)) => a == b,
            (Self::F64(a), StagedProperty::F64(b)) => a == b,
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
