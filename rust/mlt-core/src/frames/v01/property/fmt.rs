use std::fmt;

use crate::utils::FmtOptVec;
use crate::v01::property::scalars::scalar_match;
use crate::v01::{ParsedProperty, Scalar};

/// Custom implementation to ensure values are printed without newlines
impl fmt::Debug for ParsedProperty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(s) => {
                scalar_match!(s, label, v => f.debug_tuple(label).field(&v.name).field(&FmtOptVec(&v.values)).finish())
            }
            Self::Str(v) => f
                .debug_tuple("Str")
                .field(&v.name)
                .field(&FmtOptVec(&v.materialize()))
                .finish(),
            Self::SharedDict(shared_dict) => f
                .debug_tuple("SharedDict")
                .field(&shared_dict.prefix)
                .field(&shared_dict.items)
                .finish(),
        }
    }
}
