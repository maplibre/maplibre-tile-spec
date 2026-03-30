use std::fmt;
use std::marker::PhantomData;

use derive_where::derive_where;

use crate::v01::{
    EncodedScalar, FsstStrEncoder, IntEncoder, ParsedScalar, PresenceStream, PropertyEncoder,
    RawScalar, ScalarEncoder, ScalarValueEncoder, StagedScalar, StrEncoder,
};

impl ScalarEncoder {
    #[must_use]
    pub fn str(presence: PresenceStream, string_lengths: IntEncoder) -> Self {
        let enc = StrEncoder::Plain { string_lengths };
        Self {
            presence,
            value: ScalarValueEncoder::String(enc),
        }
    }
    /// Create a property encoder with integer encoding
    #[must_use]
    pub fn int(presence: PresenceStream, enc: IntEncoder) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Int(enc),
        }
    }
    /// Create a property encoder with FSST string encoding
    #[must_use]
    pub fn str_fsst(
        presence: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::Fsst(FsstStrEncoder {
                symbol_lengths,
                dict_lengths,
            })),
        }
    }

    /// Create a property encoder with deduplicated plain dictionary string encoding.
    /// Encodes unique strings once; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_dict(
        presence: PresenceStream,
        string_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::Dict {
                string_lengths,
                offsets,
            }),
        }
    }

    /// Create a property encoder with deduplicated FSST dictionary string encoding.
    /// FSST-compresses unique strings; per-feature offsets index into the dictionary.
    #[must_use]
    pub fn str_fsst_dict(
        presence: PresenceStream,
        symbol_lengths: IntEncoder,
        dict_lengths: IntEncoder,
        offsets: IntEncoder,
    ) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::String(StrEncoder::FsstDict {
                fsst: FsstStrEncoder {
                    symbol_lengths,
                    dict_lengths,
                },
                offsets,
            }),
        }
    }
    /// Create a property encoder for boolean values
    #[must_use]
    pub fn bool(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Bool,
        }
    }
    /// Create a property encoder for float values
    #[must_use]
    pub fn float(presence: PresenceStream) -> Self {
        Self {
            presence,
            value: ScalarValueEncoder::Float,
        }
    }
}

/// FIXME: uncertain why we need this, delete?
impl From<ScalarEncoder> for PropertyEncoder {
    fn from(encoder: ScalarEncoder) -> Self {
        Self::Scalar(encoder)
    }
}

/// A type-constructor family that maps each scalar primitive type to a container.
///
/// Implement this for a marker struct to define how each primitive (`bool`,
/// `i8`, …, `f64`) is stored in a [`Scalar<F>`] enum. The GAT bound
/// `Of<T>: Clone + Debug + PartialEq` ensures `Scalar<F>` can derive those
/// traits via `derive_where` without adding any spurious bound on `F` itself.
pub trait ScalarFamily {
    type Of<T: Copy + PartialEq + fmt::Debug>: Clone + fmt::Debug + PartialEq;
}

/// Tags a value with its scalar primitive kind (bool, integer, or float).
/// Does NOT include string or shared-dictionary types.
///
/// The type parameter `F: ScalarFamily` determines what is stored in each
/// variant. Common instantiations:
/// - `Scalar<RawScalarFam<'a>>` — all variants hold `RawScalar<'a>`
/// - `Scalar<EncodedScalarFam>` — all variants hold `EncodedScalar`
/// - `Scalar<ParsedScalarFam<'a>>` — `Bool` holds `ParsedScalar<'a, bool>`, etc.
/// - `Scalar<StagedScalarFam>` — `Bool` holds `StagedScalar<bool>`, etc.
/// - `Scalar<OptionFam>` — `Bool` holds `Option<bool>`, etc. (for `PropValue`)
/// - `Scalar<IdentityFam>` — `Bool` holds `bool`, etc. (for `PropValueRef`)
#[derive_where(Clone, Debug, PartialEq)]
pub enum Scalar<F: ScalarFamily> {
    Bool(F::Of<bool>),
    I8(F::Of<i8>),
    U8(F::Of<u8>),
    I32(F::Of<i32>),
    U32(F::Of<u32>),
    I64(F::Of<i64>),
    U64(F::Of<u64>),
    F32(F::Of<f32>),
    F64(F::Of<f64>),
}

impl<F: ScalarFamily> Scalar<F> {
    /// Apply a function to the inner value, producing a `Scalar<G>` of the same variant.
    pub fn try_map<G, E, B>(self, mut f: B) -> Result<Scalar<G>, E>
    where
        G: ScalarFamily,
        B: ScalarMapFn<F, G, E>,
    {
        Ok(match self {
            Self::Bool(v) => Scalar::Bool(f.map_bool(v)?),
            Self::I8(v) => Scalar::I8(f.map_i8(v)?),
            Self::U8(v) => Scalar::U8(f.map_u8(v)?),
            Self::I32(v) => Scalar::I32(f.map_i32(v)?),
            Self::U32(v) => Scalar::U32(f.map_u32(v)?),
            Self::I64(v) => Scalar::I64(f.map_i64(v)?),
            Self::U64(v) => Scalar::U64(f.map_u64(v)?),
            Self::F32(v) => Scalar::F32(f.map_f32(v)?),
            Self::F64(v) => Scalar::F64(f.map_f64(v)?),
        })
    }
}

/// Helper trait for [`Scalar::try_map`]: maps each variant's value from
/// `F::Of<T>` to `G::Of<T>` (potentially fallibly).
pub trait ScalarMapFn<F: ScalarFamily, G: ScalarFamily, E = std::convert::Infallible> {
    fn map_bool(&mut self, v: F::Of<bool>) -> Result<G::Of<bool>, E>;
    fn map_i8(&mut self, v: F::Of<i8>) -> Result<G::Of<i8>, E>;
    fn map_u8(&mut self, v: F::Of<u8>) -> Result<G::Of<u8>, E>;
    fn map_i32(&mut self, v: F::Of<i32>) -> Result<G::Of<i32>, E>;
    fn map_u32(&mut self, v: F::Of<u32>) -> Result<G::Of<u32>, E>;
    fn map_i64(&mut self, v: F::Of<i64>) -> Result<G::Of<i64>, E>;
    fn map_u64(&mut self, v: F::Of<u64>) -> Result<G::Of<u64>, E>;
    fn map_f32(&mut self, v: F::Of<f32>) -> Result<G::Of<f32>, E>;
    fn map_f64(&mut self, v: F::Of<f64>) -> Result<G::Of<f64>, E>;
}

/// Expand a single expression over all nine scalar variants of a [`Scalar<F>`].
///
/// # Forms
///
/// **Form 1** – extract the inner value as `$v` and evaluate `$body`:
/// ```ignore
/// scalar_match!(some_scalar, v => v.name)
/// ```
///
/// **Form 2** – re-wrap the inner value into `Scalar<$fam>` of the SAME variant:
/// ```ignore
/// scalar_match!(some_scalar => Scalar<OptionFam>, |v| Some(v))
/// ```
///
/// **Form 3** – bind both variant name (`$label: &'static str`) and inner value `$v`:
/// ```ignore
/// scalar_match!(some_scalar, label, v => f.debug_tuple(label).field(v).finish())
/// ```
///
/// **Form 4** – zip-match two scalars of different families; `$diff` is evaluated when
/// the variants do not match:
/// ```ignore
/// scalar_match!(left, right, a, b => *a == b, else false)
/// ```
macro_rules! scalar_match {
    // Form 1: dispatch and bind inner value as $v
    ($scalar:expr, $v:ident => $body:expr) => {
        match $scalar {
            Scalar::Bool($v) => $body,
            Scalar::I8($v) => $body,
            Scalar::U8($v) => $body,
            Scalar::I32($v) => $body,
            Scalar::U32($v) => $body,
            Scalar::I64($v) => $body,
            Scalar::U64($v) => $body,
            Scalar::F32($v) => $body,
            Scalar::F64($v) => $body,
        }
    };
    // Form 2: dispatch, bind inner value as $v, and re-wrap as Scalar<$fam>
    ($scalar:expr => Scalar<$fam:ty>, |$v:ident| $body:expr) => {
        match $scalar {
            Scalar::Bool($v) => Scalar::<$fam>::Bool($body),
            Scalar::I8($v) => Scalar::<$fam>::I8($body),
            Scalar::U8($v) => Scalar::<$fam>::U8($body),
            Scalar::I32($v) => Scalar::<$fam>::I32($body),
            Scalar::U32($v) => Scalar::<$fam>::U32($body),
            Scalar::I64($v) => Scalar::<$fam>::I64($body),
            Scalar::U64($v) => Scalar::<$fam>::U64($body),
            Scalar::F32($v) => Scalar::<$fam>::F32($body),
            Scalar::F64($v) => Scalar::<$fam>::F64($body),
        }
    };
    // Form 3: dispatch, bind variant name as $label (&'static str) and inner value as $v
    ($scalar:expr, $label:ident, $v:ident => $body:expr) => {
        match $scalar {
            Scalar::Bool($v) => {
                let $label: &'static str = "Bool";
                $body
            }
            Scalar::I8($v) => {
                let $label: &'static str = "I8";
                $body
            }
            Scalar::U8($v) => {
                let $label: &'static str = "U8";
                $body
            }
            Scalar::I32($v) => {
                let $label: &'static str = "I32";
                $body
            }
            Scalar::U32($v) => {
                let $label: &'static str = "U32";
                $body
            }
            Scalar::I64($v) => {
                let $label: &'static str = "I64";
                $body
            }
            Scalar::U64($v) => {
                let $label: &'static str = "U64";
                $body
            }
            Scalar::F32($v) => {
                let $label: &'static str = "F32";
                $body
            }
            Scalar::F64($v) => {
                let $label: &'static str = "F64";
                $body
            }
        }
    };
    // Form 4: zip-match two scalars; $same when variants align, $diff otherwise
    ($left:expr, $right:expr, $a:ident, $b:ident => $same:expr, else $diff:expr) => {
        match ($left, $right) {
            (Scalar::Bool($a), Scalar::Bool($b)) => $same,
            (Scalar::I8($a), Scalar::I8($b)) => $same,
            (Scalar::U8($a), Scalar::U8($b)) => $same,
            (Scalar::I32($a), Scalar::I32($b)) => $same,
            (Scalar::U32($a), Scalar::U32($b)) => $same,
            (Scalar::I64($a), Scalar::I64($b)) => $same,
            (Scalar::U64($a), Scalar::U64($b)) => $same,
            (Scalar::F32($a), Scalar::F32($b)) => $same,
            (Scalar::F64($a), Scalar::F64($b)) => $same,
            _ => $diff,
        }
    };
}
pub(crate) use scalar_match;

impl<'a> Scalar<RawScalarFam<'a>> {
    /// Return a reference to the inner `RawScalar<'a>` regardless of variant.
    /// Works because `RawScalarFam` maps every `T` to the same `RawScalar<'a>`.
    #[inline]
    #[must_use]
    pub fn raw_scalar(&self) -> &RawScalar<'a> {
        scalar_match!(self, v => v)
    }
}

impl Scalar<EncodedScalarFam> {
    /// Return a reference to the inner `EncodedScalar` regardless of variant.
    #[inline]
    #[must_use]
    pub fn encoded_scalar(&self) -> &EncodedScalar {
        scalar_match!(self, v => v)
    }
}

/// Manual `Copy` impl for `Scalar<IdentityFam>` (all inner values are primitive `Copy` types).
impl Copy for Scalar<IdentityFam> {}

// ── Helpers for ParsedScalarFam ───────────────────────────────────────────────

impl<'a> Scalar<ParsedScalarFam<'a>> {
    /// Column name, independent of the primitive type.
    #[must_use]
    pub fn name(&self) -> &'a str {
        scalar_match!(self, v => v.name)
    }

    /// A `None`-valued `Scalar<OptionFam>` preserving the same variant (for typed nulls).
    #[must_use]
    pub fn null_option(&self) -> Scalar<OptionFam> {
        scalar_match!(self => Scalar<OptionFam>, |_v| None)
    }
}

// ── Helpers for StagedScalarFam ────────────────────────────────────────────────

impl Scalar<StagedScalarFam> {
    /// Column name, independent of the primitive type.
    #[must_use]
    pub fn name(&self) -> &str {
        scalar_match!(self, v => v.name.as_str())
    }

    /// Whether any value in this column is `None`.
    pub fn has_nulls(&self) -> bool {
        scalar_match!(self, v => v.values.iter().any(Option::is_none))
    }

    /// Per-feature presence booleans (`true` = non-null).
    pub fn presence_bools(&self) -> Vec<bool> {
        scalar_match!(self, v => v.values.iter().map(Option::is_some).collect())
    }

    /// A `None`-valued `Scalar<OptionFam>` preserving the same variant (for typed nulls).
    #[must_use]
    pub fn null_option(&self) -> Scalar<OptionFam> {
        scalar_match!(self => Scalar<OptionFam>, |_v| None)
    }

    /// Number of features in this column (present + null).
    #[must_use]
    pub fn len(&self) -> usize {
        scalar_match!(self, v => v.values.len())
    }

    /// Whether this column has no features.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<Scalar<IdentityFam>> for Scalar<OptionFam> {
    fn from(s: Scalar<IdentityFam>) -> Self {
        scalar_match!(s => Scalar<OptionFam>, |v| Some(v))
    }
}

/// Family for `RawProperty`: all variants hold `RawScalar<'a>`.
/// The variant discriminant (Bool/I8/…) carries the column type; the data is untyped.
pub struct RawScalarFam<'a>(PhantomData<&'a ()>);
impl<'a> ScalarFamily for RawScalarFam<'a> {
    type Of<T: Copy + PartialEq + fmt::Debug> = RawScalar<'a>;
}

/// Family for `EncodedProperty`: all variants hold `EncodedScalar`.
pub struct EncodedScalarFam;
impl ScalarFamily for EncodedScalarFam {
    type Of<T: Copy + PartialEq + fmt::Debug> = EncodedScalar;
}

/// Family for `ParsedProperty`: `Bool` → `ParsedScalar<'a, bool>`, `I8` → `ParsedScalar<'a, i8>`, …
pub struct ParsedScalarFam<'a>(PhantomData<&'a ()>);
impl<'a> ScalarFamily for ParsedScalarFam<'a> {
    type Of<T: Copy + PartialEq + fmt::Debug> = ParsedScalar<'a, T>;
}

/// Family for `StagedProperty`: `Bool` → `StagedScalar<bool>`, `I8` → `StagedScalar<i8>`, …
pub struct StagedScalarFam;
impl ScalarFamily for StagedScalarFam {
    type Of<T: Copy + PartialEq + fmt::Debug> = StagedScalar<T>;
}

/// Family for `PropValue`: each variant holds `Option<T>`.
pub struct OptionFam;
impl ScalarFamily for OptionFam {
    type Of<T: Copy + PartialEq + fmt::Debug> = Option<T>;
}

/// Family for `PropValueRef`: each variant holds `T` directly (identity mapping).
pub struct IdentityFam;
impl ScalarFamily for IdentityFam {
    type Of<T: Copy + PartialEq + fmt::Debug> = T;
}
