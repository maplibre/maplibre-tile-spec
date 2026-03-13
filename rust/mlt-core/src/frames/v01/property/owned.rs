use super::{
    EncodedFsstData, EncodedName, EncodedPlainData, EncodedPresence, EncodedProperty,
    EncodedScalar, EncodedSharedDict, EncodedSharedDictChild, EncodedSharedDictEncoding,
    EncodedStrings, EncodedStringsEncoding, RawFsstData, RawPlainData, RawPresence, RawProperty,
    RawScalar, RawSharedDict, RawSharedDictChild, RawSharedDictEncoding, RawStrings,
    RawStringsEncoding,
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
