use super::{
    EncodedFsstData, EncodedName, EncodedPlainData, EncodedPresence, EncodedProperty,
    EncodedScalar, EncodedSharedDict, EncodedSharedDictChild, EncodedSharedDictEncoding,
    EncodedStrings, EncodedStringsEncoding, ParsedProperty, ParsedScalar, ParsedSharedDict,
    ParsedSharedDictItem, ParsedStrings, RawFsstData, RawPlainData, RawPresence, RawProperty,
    RawScalar, RawSharedDict, RawSharedDictChild, RawSharedDictEncoding, RawStrings,
    RawStringsEncoding, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings,
};
use crate::v01::RawStream;

impl RawPresence<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedPresence {
        EncodedPresence(self.0.as_ref().map(RawStream::to_owned))
    }
}

impl RawSharedDictChild<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedSharedDictChild {
        EncodedSharedDictChild {
            name: EncodedName(self.name.to_string()),
            presence: self.presence.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl RawPlainData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedPlainData {
        EncodedPlainData {
            lengths: self.lengths.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl RawFsstData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedFsstData {
        EncodedFsstData {
            symbol_lengths: self.symbol_lengths.to_owned(),
            symbol_table: self.symbol_table.to_owned(),
            lengths: self.lengths.to_owned(),
            corpus: self.corpus.to_owned(),
        }
    }
}

impl RawSharedDictEncoding<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedSharedDictEncoding {
        match self {
            Self::Plain(data) => EncodedSharedDictEncoding::Plain(data.to_owned()),
            Self::FsstPlain(data) => EncodedSharedDictEncoding::FsstPlain(data.to_owned()),
        }
    }
}

impl RawStringsEncoding<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedStringsEncoding {
        match self {
            Self::Plain(data) => EncodedStringsEncoding::Plain(data.to_owned()),
            Self::Dictionary {
                plain_data,
                offsets,
            } => EncodedStringsEncoding::Dictionary {
                plain_data: plain_data.to_owned(),
                offsets: offsets.to_owned(),
            },
            Self::FsstPlain(data) => EncodedStringsEncoding::FsstPlain(data.to_owned()),
            Self::FsstDictionary { fsst_data, offsets } => EncodedStringsEncoding::FsstDictionary {
                fsst_data: fsst_data.to_owned(),
                offsets: offsets.to_owned(),
            },
        }
    }
}

impl RawScalar<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedScalar {
        EncodedScalar {
            name: EncodedName(self.name.to_string()),
            presence: self.presence.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl RawStrings<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedStrings {
        EncodedStrings {
            name: EncodedName(self.name.to_string()),
            presence: self.presence.to_owned(),
            encoding: self.encoding.to_owned(),
        }
    }
}

impl RawSharedDict<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedSharedDict {
        EncodedSharedDict {
            name: EncodedName(self.name.to_string()),
            encoding: self.encoding.to_owned(),
            children: self
                .children
                .iter()
                .map(RawSharedDictChild::to_owned)
                .collect(),
        }
    }
}

impl RawProperty<'_> {
    #[must_use]
    pub fn to_owned(&self) -> EncodedProperty {
        match self {
            Self::Bool(s) => EncodedProperty::Bool(s.to_owned()),
            Self::I8(s) => EncodedProperty::I8(s.to_owned()),
            Self::U8(s) => EncodedProperty::U8(s.to_owned()),
            Self::I32(s) => EncodedProperty::I32(s.to_owned()),
            Self::U32(s) => EncodedProperty::U32(s.to_owned()),
            Self::I64(s) => EncodedProperty::I64(s.to_owned()),
            Self::U64(s) => EncodedProperty::U64(s.to_owned()),
            Self::F32(s) => EncodedProperty::F32(s.to_owned()),
            Self::F64(s) => EncodedProperty::F64(s.to_owned()),
            Self::Str(s) => EncodedProperty::Str(s.to_owned()),
            Self::SharedDict(s) => EncodedProperty::SharedDict(s.to_owned()),
        }
    }
}

// ── Cross-type PartialEq between Parsed* (decode-side) and Staged* (encode-side) ──

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
