use super::{
    EncodedPresence, EncodedProperty, EncodedScalar, EncodedSharedDict, EncodedSharedDictChild,
    EncodedStrings, FsstData, NameRef, OwnedEncodedPresence, OwnedEncodedProperty,
    OwnedEncodedScalar, OwnedEncodedSharedDict, OwnedEncodedSharedDictChild, OwnedEncodedStrings,
    OwnedFsstData, OwnedName, OwnedPlainData, OwnedSharedDictEncoding, OwnedStringsEncoding,
    PlainData, SharedDictEncoding, StringsEncoding,
};
use crate::v01::Stream;

impl NameRef<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedName {
        OwnedName(self.0.to_string())
    }
}

impl EncodedPresence<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedPresence {
        OwnedEncodedPresence(self.0.as_ref().map(Stream::to_owned))
    }
}

impl EncodedSharedDictChild<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedSharedDictChild {
        OwnedEncodedSharedDictChild {
            name: self.name.to_owned(),
            presence: self.presence.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl PlainData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedPlainData {
        OwnedPlainData {
            lengths: self.lengths.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl FsstData<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedFsstData {
        OwnedFsstData {
            symbol_lengths: self.symbol_lengths.to_owned(),
            symbol_table: self.symbol_table.to_owned(),
            lengths: self.lengths.to_owned(),
            corpus: self.corpus.to_owned(),
        }
    }
}

impl SharedDictEncoding<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedSharedDictEncoding {
        match self {
            Self::Plain(data) => OwnedSharedDictEncoding::Plain(data.to_owned()),
            Self::FsstPlain(data) => OwnedSharedDictEncoding::FsstPlain(data.to_owned()),
        }
    }
}

impl StringsEncoding<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedStringsEncoding {
        match self {
            Self::Plain(data) => OwnedStringsEncoding::Plain(data.to_owned()),
            Self::Dictionary {
                plain_data,
                offsets,
            } => OwnedStringsEncoding::Dictionary {
                plain_data: plain_data.to_owned(),
                offsets: offsets.to_owned(),
            },
            Self::FsstPlain(data) => OwnedStringsEncoding::FsstPlain(data.to_owned()),
            Self::FsstDictionary { fsst_data, offsets } => OwnedStringsEncoding::FsstDictionary {
                fsst_data: fsst_data.to_owned(),
                offsets: offsets.to_owned(),
            },
        }
    }
}

impl EncodedScalar<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedScalar {
        OwnedEncodedScalar {
            name: self.name.to_owned(),
            presence: self.presence.to_owned(),
            data: self.data.to_owned(),
        }
    }
}

impl EncodedStrings<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedStrings {
        OwnedEncodedStrings {
            name: self.name.to_owned(),
            presence: self.presence.to_owned(),
            encoding: self.encoding.to_owned(),
        }
    }
}

impl EncodedSharedDict<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedSharedDict {
        OwnedEncodedSharedDict {
            name: self.name.to_owned(),
            encoding: self.encoding.to_owned(),
            children: self
                .children
                .iter()
                .map(EncodedSharedDictChild::to_owned)
                .collect(),
        }
    }
}

impl EncodedProperty<'_> {
    #[must_use]
    pub fn to_owned(&self) -> OwnedEncodedProperty {
        match self {
            Self::Bool(s) => OwnedEncodedProperty::Bool(s.to_owned()),
            Self::I8(s) => OwnedEncodedProperty::I8(s.to_owned()),
            Self::U8(s) => OwnedEncodedProperty::U8(s.to_owned()),
            Self::I32(s) => OwnedEncodedProperty::I32(s.to_owned()),
            Self::U32(s) => OwnedEncodedProperty::U32(s.to_owned()),
            Self::I64(s) => OwnedEncodedProperty::I64(s.to_owned()),
            Self::U64(s) => OwnedEncodedProperty::U64(s.to_owned()),
            Self::F32(s) => OwnedEncodedProperty::F32(s.to_owned()),
            Self::F64(s) => OwnedEncodedProperty::F64(s.to_owned()),
            Self::Str(s) => OwnedEncodedProperty::Str(s.to_owned()),
            Self::SharedDict(s) => OwnedEncodedProperty::SharedDict(s.to_owned()),
        }
    }
}
