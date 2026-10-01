//! Typed columns parsed out of string properties that pack structured values, and the strings formatted back.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use anyhow::{Context as _, Result as AnyResult, anyhow, ensure};
use mlt_core::{
    MValue, MValueKey, MltResult, NestedKey, NestedKind, NestedValue, PropKind, PropValue,
    PropertyKey, TileFeature, TileFeatureBuilder, TileLayer, TileLayerBuilder,
};
use serde::Deserialize;

/// How each layer's string properties are parsed, read from a `--fields` TOML file.
///
/// ```toml
/// [layers.items]
/// ids = { split = ",", kind = "u64" }
/// ```
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    #[serde(default)]
    layers: BTreeMap<String, BTreeMap<String, FieldForm>>,
    /// The configured fields that some applied layer held, by layer name.
    #[serde(skip)]
    seen: Mutex<BTreeMap<String, BTreeSet<String>>>,
}

/// How one string property is written, and the column it becomes.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, try_from = "FormToml")]
struct FieldForm {
    syntax: Syntax,
    into: Column,
}

/// The syntax a string property is written in.
///
/// Every syntax formats back to the exact string it parsed, which [`FieldConfig::apply`] checks.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Syntax {
    Numbers {
        split: Split,
        kind: IntKind,
        /// Whether each value after the first is written as its difference from the one before.
        running_sum: bool,
    },
    Strings {
        split: char,
    },
}

/// A field's table in the `--fields` file, before its keys are checked against each other.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct FormToml {
    /// Where one value ends and the next begins.
    split: Split,
    /// The type of each value.
    kind: KindToml,
    /// Whether each value after the first is written as its difference from the one before.
    #[serde(default)]
    running_sum: bool,
    /// The column the values land in.
    #[serde(default)]
    into: Column,
}

/// The `kind` of a field's table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
enum KindToml {
    I32,
    U32,
    I64,
    U64,
    Str,
}

impl TryFrom<FormToml> for FieldForm {
    type Error = &'static str;

    fn try_from(form: FormToml) -> Result<Self, Self::Error> {
        let FormToml {
            split,
            kind,
            running_sum,
            into,
        } = form;
        let kind = match kind {
            KindToml::I32 => Some(IntKind::I32),
            KindToml::U32 => Some(IntKind::U32),
            KindToml::I64 => Some(IntKind::I64),
            KindToml::U64 => Some(IntKind::U64),
            KindToml::Str => None,
        };
        let syntax = match (kind, split) {
            (Some(_), Split::At(c)) if c.is_ascii_digit() || matches!(c, '+' | '-') => {
                return Err("a number kind cannot split at a digit or sign");
            }
            (Some(kind), split) => Syntax::Numbers {
                split,
                kind,
                running_sum,
            },
            (None, _) if running_sum => return Err("a running sum needs a number kind"),
            (None, Split::Sign) => return Err("splitting at signs needs a number kind"),
            (None, Split::At(split)) => Syntax::Strings { split },
        };
        Ok(Self { syntax, into })
    }
}

/// The column a parsed property becomes.
#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
enum Column {
    /// A nested list per feature.
    #[default]
    List,
    /// One value per vertex.
    MValue,
}

/// Where one value ends and the next begins.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, try_from = "String")]
enum Split {
    /// `"5,6"`: at each occurrence of the character.
    At(char),
    /// `"10+1-2"`: before each `+` or `-` after the first character, which every later value carries.
    Sign,
}

impl TryFrom<String> for Split {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let mut chars = s.chars();
        match (s.as_str(), chars.next(), chars.next()) {
            ("sign", ..) => Ok(Self::Sign),
            (_, Some(c), None) => Ok(Self::At(c)),
            _ => Err(format!("split is \"sign\" or one character, not {s:?}")),
        }
    }
}

impl Split {
    fn tokens(self, s: &str) -> Vec<&str> {
        match self {
            Self::At(c) => s.split(c).collect(),
            Self::Sign => {
                let mut tokens = Vec::new();
                let mut start = 0;
                for (i, c) in s.char_indices().skip(1) {
                    if matches!(c, '+' | '-') {
                        tokens.push(&s[start..i]);
                        start = i;
                    }
                }
                tokens.push(&s[start..]);
                tokens
            }
        }
    }

    fn join(self, tokens: &[String]) -> String {
        match self {
            Self::At(c) => tokens.join(&c.to_string()),
            Self::Sign => {
                let mut s = String::new();
                for (i, token) in tokens.iter().enumerate() {
                    if i > 0 && !token.starts_with('-') {
                        s.push('+');
                    }
                    s.push_str(token);
                }
                s
            }
        }
    }
}

/// The type each parsed number is stored as.
#[derive(Debug, Clone, Copy, PartialEq)]
enum IntKind {
    I32,
    U32,
    I64,
    U64,
}

impl IntKind {
    fn name(self) -> &'static str {
        self.prop_kind().into()
    }

    fn prop_kind(self) -> PropKind {
        match self {
            Self::I32 => PropKind::I32,
            Self::U32 => PropKind::U32,
            Self::I64 => PropKind::I64,
            Self::U64 => PropKind::U64,
        }
    }

    /// `value` as a leaf of this kind, or [`None`] if it does not fit.
    fn leaf(self, value: i128) -> Option<PropValue> {
        Some(match self {
            Self::I32 => PropValue::I32(Some(value.try_into().ok()?)),
            Self::U32 => PropValue::U32(Some(value.try_into().ok()?)),
            Self::I64 => PropValue::I64(Some(value.try_into().ok()?)),
            Self::U64 => PropValue::U64(Some(value.try_into().ok()?)),
        })
    }

    /// `values` as an m-value of this kind, or [`None`] if one does not fit.
    fn m_value(self, values: &[i128]) -> Option<MValue> {
        fn fit<T: TryFrom<i128>>(values: &[i128]) -> Option<Vec<T>> {
            values.iter().map(|&v| T::try_from(v).ok()).collect()
        }
        Some(match self {
            Self::I32 => MValue::I32(Some(fit(values)?)),
            Self::U32 => MValue::U32(Some(fit(values)?)),
            Self::I64 => MValue::I64(Some(fit(values)?)),
            Self::U64 => MValue::U64(Some(fit(values)?)),
        })
    }
}

impl Syntax {
    fn split(self) -> Split {
        match self {
            Self::Numbers { split, .. } => split,
            Self::Strings { split } => Split::At(split),
        }
    }

    fn prop_kind(self) -> PropKind {
        match self {
            Self::Numbers { kind, .. } => kind.prop_kind(),
            Self::Strings { .. } => PropKind::Str,
        }
    }

    /// `s` as its values, checked to format back to `s`.
    fn parse(self, s: &str) -> AnyResult<Values> {
        let tokens = self.split().tokens(s);
        let values = match self {
            Self::Numbers {
                kind, running_sum, ..
            } => Values::Numbers(kind, numbers(&tokens, kind, running_sum)?),
            Self::Strings { .. } => {
                Values::Strings(tokens.into_iter().map(str::to_owned).collect())
            }
        };
        let back = self.format(&values);
        ensure!(back == s, "{s:?} formats back as {back:?}");
        Ok(values)
    }

    /// `s` as one value per vertex of `feature`, checked to format back to `s`.
    fn parse_per_vertex(self, s: &str, feature: &TileFeature) -> AnyResult<Values> {
        let values = self.parse(s)?;
        let (count, vertices) = (values.len(), feature.vertex_count());
        ensure!(count == vertices, "{count} values for {vertices} vertices");
        Ok(values)
    }

    /// `values` written in this syntax, checked to parse back to `values`.
    fn write(self, values: &Values) -> AnyResult<String> {
        let s = self.format(values);
        ensure!(
            self.parse(&s)? == *values,
            "{values:?} formats as {s:?}, which parses to other values"
        );
        Ok(s)
    }

    /// `values` written in this syntax.
    fn format(self, values: &Values) -> String {
        let running_sum = matches!(
            self,
            Self::Numbers {
                running_sum: true,
                ..
            }
        );
        let tokens = match values {
            Values::Numbers(_, v) => number_tokens(v, running_sum),
            Values::Strings(v) => v.clone(),
        };
        self.split().join(&tokens)
    }

    /// `numbers` read back for this syntax, or [`None`] for a null.
    fn numbers(self, numbers: Option<Vec<i128>>) -> AnyResult<Option<Values>> {
        match self {
            Self::Numbers { kind, .. } => Ok(numbers.map(|v| Values::Numbers(kind, v))),
            Self::Strings { .. } => Err(anyhow!("integers where strings belong")),
        }
    }

    /// `strings` read back for this syntax, or [`None`] for a null.
    fn strings(self, strings: Option<Vec<String>>) -> AnyResult<Option<Values>> {
        match self {
            Self::Strings { .. } => Ok(strings.map(Values::Strings)),
            Self::Numbers { kind, .. } => {
                Err(anyhow!("strings where {} values belong", kind.name()))
            }
        }
    }
}

/// The values one string parses into, each number checked to fit its kind.
#[derive(Debug, PartialEq)]
enum Values {
    Numbers(IntKind, Vec<i128>),
    Strings(Vec<String>),
}

impl Values {
    fn len(&self) -> usize {
        match self {
            Self::Numbers(_, v) => v.len(),
            Self::Strings(v) => v.len(),
        }
    }

    fn into_m_value(self) -> AnyResult<MValue> {
        match self {
            Self::Numbers(kind, v) => kind
                .m_value(&v)
                .ok_or_else(|| anyhow!("{v:?} do not fit {}", kind.name())),
            Self::Strings(v) => Ok(MValue::Str(Some(v))),
        }
    }

    /// These values as the leaves of a nested list.
    fn into_list(self) -> AnyResult<NestedValue> {
        let leaves = match self {
            Self::Numbers(kind, v) => v
                .into_iter()
                .map(|x| {
                    kind.leaf(x)
                        .ok_or_else(|| anyhow!("{x} does not fit {}", kind.name()))
                })
                .collect::<AnyResult<Vec<_>>>()?,
            Self::Strings(v) => v.into_iter().map(|s| PropValue::Str(Some(s))).collect(),
        };
        Ok(NestedValue::list(leaves.into_iter().map(NestedValue::Leaf)))
    }

    /// The values [`Self::into_m_value`] wrote for `syntax`, or [`None`] for a null.
    fn from_m_value(syntax: Syntax, value: &MValue) -> AnyResult<Option<Self>> {
        fn ints<T: Copy + Into<i128>>(values: Option<&Vec<T>>) -> Option<Vec<i128>> {
            values.map(|v| v.iter().map(|&x| x.into()).collect())
        }
        let numbers = match value {
            MValue::I8(v) => ints(v.as_ref()),
            MValue::U8(v) => ints(v.as_ref()),
            MValue::I32(v) => ints(v.as_ref()),
            MValue::U32(v) => ints(v.as_ref()),
            MValue::I64(v) => ints(v.as_ref()),
            MValue::U64(v) => ints(v.as_ref()),
            MValue::Str(v) => return syntax.strings(v.clone()),
            MValue::Bool(_) | MValue::F32(_) | MValue::F64(_) => {
                return Err(anyhow!("{value:?} holds neither integers nor strings"));
            }
        };
        syntax.numbers(numbers)
    }

    /// The values [`Self::into_list`] wrote for `syntax`, or [`None`] for a null.
    fn from_list(syntax: Syntax, value: &NestedValue) -> AnyResult<Option<Self>> {
        let NestedValue::List(items) = value else {
            return Err(anyhow!("{value:?} is not a list"));
        };
        let Some(items) = items else {
            return Ok(None);
        };
        let leaves = items.iter().map(|item| match item {
            NestedValue::Leaf(leaf) => Ok(leaf),
            NestedValue::List(_) | NestedValue::Map(_) => Err(anyhow!("{item:?} is not a leaf")),
        });
        match syntax {
            Syntax::Numbers { .. } => {
                let numbers = leaves
                    .map(|leaf| integer(leaf?))
                    .collect::<AnyResult<_>>()?;
                syntax.numbers(Some(numbers))
            }
            Syntax::Strings { .. } => {
                let strings = leaves.map(|leaf| string(leaf?)).collect::<AnyResult<_>>()?;
                syntax.strings(Some(strings))
            }
        }
    }
}

fn string(leaf: &PropValue) -> AnyResult<String> {
    let PropValue::Str(Some(s)) = leaf else {
        return Err(anyhow!("{leaf:?} is not a string"));
    };
    Ok(s.clone())
}

fn integer(leaf: &PropValue) -> AnyResult<i128> {
    let number = match leaf {
        PropValue::I8(v) => v.map(i128::from),
        PropValue::U8(v) => v.map(i128::from),
        PropValue::I32(v) => v.map(i128::from),
        PropValue::U32(v) => v.map(i128::from),
        PropValue::I64(v) => v.map(i128::from),
        PropValue::U64(v) => v.map(i128::from),
        PropValue::Bool(_) | PropValue::F32(_) | PropValue::F64(_) | PropValue::Str(_) => None,
    };
    number.ok_or_else(|| anyhow!("{leaf:?} is not an integer"))
}

/// `tokens` as numbers of `kind`, each after the first added to the one before when `running_sum` is set.
fn numbers(tokens: &[&str], kind: IntKind, running_sum: bool) -> AnyResult<Vec<i128>> {
    let fits = |v: i128| kind.leaf(v).is_some();
    let mut values: Vec<i128> = Vec::with_capacity(tokens.len());
    for token in tokens {
        let number: i128 = token
            .parse()
            .with_context(|| format!("{token:?} is not a number"))?;
        let value = match values.last() {
            Some(&prev) if running_sum => prev
                .checked_add(number)
                .filter(|&v| fits(v))
                .ok_or_else(|| anyhow!("{prev} plus {token} overflows"))?,
            _ => {
                ensure!(fits(number), "{token:?} does not fit {}", kind.name());
                number
            }
        };
        values.push(value);
    }
    Ok(values)
}

/// `values` as tokens, each after the first written as its difference from the one before when `running_sum` is set.
fn number_tokens(values: &[i128], running_sum: bool) -> Vec<String> {
    let mut prev: Option<i128> = None;
    values
        .iter()
        .map(|&v| {
            let token = match prev {
                Some(p) if running_sum => (v - p).to_string(),
                _ => v.to_string(),
            };
            prev = Some(v);
            token
        })
        .collect()
}

impl FieldConfig {
    pub fn load(path: &Path) -> AnyResult<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        text.parse()
            .with_context(|| format!("parsing {}", path.display()))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// `layer` with its configured string properties replaced by the columns they parse into.
    pub fn apply(&self, layer: TileLayer) -> AnyResult<TileLayer> {
        let Some(forms) = self.layers.get(layer.name()) else {
            return Ok(layer);
        };
        self.see(&layer, forms);
        parse_fields(&layer, forms).with_context(|| format!("layer {}", layer.name()))
    }

    /// `layer` with the columns [`Self::apply`] parsed formatted back into the string properties they came from.
    pub fn restore(&self, layer: TileLayer) -> AnyResult<TileLayer> {
        let Some(forms) = self.layers.get(layer.name()) else {
            return Ok(layer);
        };
        format_fields(&layer, forms).with_context(|| format!("layer {}", layer.name()))
    }

    /// The configured layers and fields that no layer passed to [`Self::apply`] held.
    #[must_use]
    pub fn unused(&self) -> Vec<String> {
        let seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let mut unused = Vec::new();
        for (layer, forms) in &self.layers {
            let Some(held) = seen.get(layer) else {
                unused.push(format!("layer {layer}"));
                continue;
            };
            let missing = forms.keys().filter(|name| !held.contains(*name));
            unused.extend(missing.map(|name| format!("field {name} of layer {layer}")));
        }
        unused
    }

    /// Record `layer` and which of `forms` its properties hold.
    fn see(&self, layer: &TileLayer, forms: &BTreeMap<String, FieldForm>) {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let held = seen.entry(layer.name().to_owned()).or_default();
        let names = layer.property_names().iter();
        held.extend(names.filter(|name| forms.contains_key(*name)).cloned());
    }
}

impl FromStr for FieldConfig {
    type Err = toml::de::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        toml::from_str(text)
    }
}

/// Where one source property lands in the parsed layer.
enum Parsed {
    Property(PropertyKey),
    List(NestedKey, Syntax),
    MValue(MValueKey, Syntax),
}

fn parse_fields(layer: &TileLayer, forms: &BTreeMap<String, FieldForm>) -> AnyResult<TileLayer> {
    for name in forms.keys() {
        ensure!(
            !layer.m_value_names().contains(name) && !layer.nested_names().contains(name),
            "{name} is already a typed column"
        );
    }
    let mut out = layer.builder_like();
    let mut targets = Vec::with_capacity(layer.property_names().len());
    for (name, &kind) in layer.property_names().iter().zip(layer.property_kinds()) {
        let Some(&FieldForm { syntax, into }) = forms.get(name) else {
            targets.push(Parsed::Property(out.add_property(name, kind)?));
            continue;
        };
        ensure!(
            kind == PropKind::Str,
            "{name} holds {} values, but only strings are parsed",
            <&str>::from(kind)
        );
        let kind = syntax.prop_kind();
        targets.push(match into {
            Column::List => {
                let list = NestedKind::list(NestedKind::Leaf(kind));
                Parsed::List(out.add_nested(name, list)?, syntax)
            }
            Column::MValue => Parsed::MValue(out.add_m_value(name, kind)?, syntax),
        });
    }
    let m_keys = out.add_m_values_like(layer)?;
    let nested_keys = out.add_nested_like(layer)?;

    for feature in layer.features() {
        let mut row = out.feature_like(feature);
        for ((target, value), name) in targets
            .iter()
            .zip(feature.properties())
            .zip(layer.property_names())
        {
            match (target, value) {
                (Parsed::Property(key), value) => {
                    row.property(*key, value.clone())?;
                }
                (Parsed::List(key, syntax), PropValue::Str(Some(s))) => {
                    let list = syntax
                        .parse(s)
                        .and_then(Values::into_list)
                        .with_context(|| field_context(name, feature))?;
                    row.nested(*key, list)?;
                }
                (Parsed::MValue(key, syntax), PropValue::Str(Some(s))) => {
                    let m_value = syntax
                        .parse_per_vertex(s, feature)
                        .and_then(Values::into_m_value)
                        .with_context(|| field_context(name, feature))?;
                    row.m_value(*key, m_value)?;
                }
                // A missing value stays null in its new column.
                (Parsed::List(..) | Parsed::MValue(..), _) => {}
            }
        }
        for (key, value) in m_keys.iter().zip(feature.m_values()) {
            row.m_value(*key, value.clone())?;
        }
        for (key, value) in nested_keys.iter().zip(feature.nested()) {
            row.nested(*key, value.clone())?;
        }
        row.finish()?;
    }
    Ok(out.finish())
}

/// Where one parsed column lands back in the restored layer.
enum Restored<K> {
    Kept(K),
    Formatted(PropertyKey, Syntax),
}

fn format_fields(layer: &TileLayer, forms: &BTreeMap<String, FieldForm>) -> AnyResult<TileLayer> {
    let mut out = layer.builder_like();
    let properties = out.add_properties_like(layer)?;
    let formatted = |into: Column| {
        move |name: &str| {
            forms
                .get(name)
                .filter(|form| form.into == into)
                .map(|form| form.syntax)
        }
    };
    let m_values = restored_columns(
        &mut out,
        layer.m_value_names(),
        layer.m_value_kinds(),
        formatted(Column::MValue),
        |out, name, &kind| out.add_m_value(name, kind),
    )?;
    let nested = restored_columns(
        &mut out,
        layer.nested_names(),
        layer.nested_kinds(),
        formatted(Column::List),
        |out, name, kind| out.add_nested(name, kind.clone()),
    )?;

    for feature in layer.features() {
        let mut row = out.feature_like(feature);
        for (key, value) in properties.iter().zip(feature.properties()) {
            row.property(*key, value.clone())?;
        }
        restore_values(
            &mut row,
            &m_values,
            layer.m_value_names(),
            feature.m_values(),
            feature,
            Values::from_m_value,
            |row, key, value| row.m_value(key, value.clone()).map(drop),
        )?;
        restore_values(
            &mut row,
            &nested,
            layer.nested_names(),
            feature.nested(),
            feature,
            Values::from_list,
            |row, key, value| row.nested(key, value.clone()).map(drop),
        )?;
        row.finish()?;
    }
    Ok(out.finish())
}

/// One column per name, kept as `add` declares it unless `formatted` names a syntax to write it as a string in.
fn restored_columns<K, T>(
    out: &mut TileLayerBuilder,
    names: &[String],
    kinds: &[T],
    formatted: impl Fn(&str) -> Option<Syntax>,
    add: impl Fn(&mut TileLayerBuilder, &str, &T) -> MltResult<K>,
) -> AnyResult<Vec<Restored<K>>> {
    let mut columns = Vec::with_capacity(names.len());
    for (name, kind) in names.iter().zip(kinds) {
        columns.push(match formatted(name) {
            Some(syntax) => Restored::Formatted(out.add_property(name, PropKind::Str)?, syntax),
            None => Restored::Kept(add(out, name, kind)?),
        });
    }
    Ok(columns)
}

/// Copy each kept value of `feature` with `keep`, and write each formatted one as the string `read` finds in it.
fn restore_values<K: Copy, V>(
    row: &mut TileFeatureBuilder<'_>,
    columns: &[Restored<K>],
    names: &[String],
    values: &[V],
    feature: &TileFeature,
    read: fn(Syntax, &V) -> AnyResult<Option<Values>>,
    keep: impl Fn(&mut TileFeatureBuilder<'_>, K, &V) -> MltResult<()>,
) -> AnyResult<()> {
    for ((column, value), name) in columns.iter().zip(values).zip(names) {
        match *column {
            Restored::Kept(key) => keep(row, key, value)?,
            Restored::Formatted(key, syntax) => {
                let s = read(syntax, value)
                    .and_then(|v| v.map(|v| syntax.write(&v)).transpose())
                    .with_context(|| field_context(name, feature))?;
                row.property(key, PropValue::Str(s))?;
            }
        }
    }
    Ok(())
}

fn field_context(name: &str, feature: &TileFeature) -> String {
    match feature.id() {
        Some(id) => format!("field {name} of feature {id}"),
        None => format!("field {name}"),
    }
}
