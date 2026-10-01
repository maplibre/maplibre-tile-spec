//! Typed columns parsed out of string properties that pack structured values, and the strings formatted back.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context as _, Result as AnyResult, anyhow, ensure};
use mlt_core::{
    MValue, MValueKey, NestedKey, NestedKind, NestedValue, PropKind, PropValue, PropertyKey,
    TileFeature, TileFeatureBuilder, TileLayer, TileLayerBuilder,
};
use serde::Deserialize;

/// How each layer's string properties are parsed, read from a `--config` TOML file.
///
/// ```toml
/// [layers.items]
/// ids = { split = ",", kind = "u64" }
/// ```
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    #[serde(default)]
    layers: BTreeMap<String, BTreeMap<String, FieldForm>>,
}

/// The syntax a string property is written in.
///
/// Every form formats back to the exact string it parsed, which [`FieldConfig::apply`] checks.
/// Only the combinations with exactly one string form load.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(try_from = "FormToml")]
pub struct FieldForm {
    split: Split,
    kind: ScalarKind,
    running_sum: bool,
    into: Column,
}

/// A field's table in the `--config` file, before its keys are checked against each other.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct FormToml {
    /// Where one value ends and the next begins.
    split: Split,
    /// The type of each value.
    kind: ScalarKind,
    /// Whether each value after the first is written as its difference from the one before.
    #[serde(default)]
    running_sum: bool,
    /// The column the values land in.
    #[serde(default)]
    into: Column,
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
        if kind != ScalarKind::Str
            && let Split::At(c) = split
            && (c.is_ascii_digit() || matches!(c, '+' | '-'))
        {
            return Err("a number kind cannot split at a digit or sign");
        }
        if kind == ScalarKind::Str {
            if running_sum {
                return Err("a running sum needs a number kind");
            }
            if split == Split::Sign {
                return Err("splitting at signs needs a number kind");
            }
        }
        Ok(Self {
            split,
            kind,
            running_sum,
            into,
        })
    }
}

/// The column a parsed property becomes.
#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Column {
    /// A nested list per feature.
    #[default]
    List,
    /// One value per vertex.
    MValue,
}

/// Where one value ends and the next begins.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(try_from = "String")]
pub enum Split {
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

/// The type of each parsed value.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScalarKind {
    I32,
    U32,
    I64,
    U64,
    Str,
}

impl ScalarKind {
    fn prop_kind(self) -> PropKind {
        match self {
            Self::I32 => PropKind::I32,
            Self::U32 => PropKind::U32,
            Self::I64 => PropKind::I64,
            Self::U64 => PropKind::U64,
            Self::Str => PropKind::Str,
        }
    }
}

impl FieldForm {
    /// `s` as its values, checked to format back to `s`.
    fn parse(self, s: &str) -> AnyResult<Values> {
        let tokens = self.split.tokens(s);
        let sum = self.running_sum;
        let values = match self.kind {
            ScalarKind::I32 => Values::I32(numbers(&tokens, sum)?),
            ScalarKind::U32 => Values::U32(numbers(&tokens, sum)?),
            ScalarKind::I64 => Values::I64(numbers(&tokens, sum)?),
            ScalarKind::U64 => Values::U64(numbers(&tokens, sum)?),
            ScalarKind::Str => Values::Str(tokens.into_iter().map(str::to_owned).collect()),
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

    /// `values` written in this form, checked to parse back to `values`.
    fn write(self, values: &Values) -> AnyResult<String> {
        let s = self.format(values);
        ensure!(
            self.parse(&s)? == *values,
            "{values:?} formats as {s:?}, which parses to other values"
        );
        Ok(s)
    }

    /// `values` written in this form.
    fn format(self, values: &Values) -> String {
        let tokens: Vec<String> = match values {
            Values::I32(v) => number_tokens(v, self.running_sum),
            Values::U32(v) => number_tokens(v, self.running_sum),
            Values::I64(v) => number_tokens(v, self.running_sum),
            Values::U64(v) => number_tokens(v, self.running_sum),
            Values::Str(v) => v.clone(),
        };
        self.split.join(&tokens)
    }
}

/// The values one string parses into.
#[derive(Debug, PartialEq)]
enum Values {
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),
    Str(Vec<String>),
}

impl Values {
    fn len(&self) -> usize {
        match self {
            Self::I32(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::Str(v) => v.len(),
        }
    }

    fn into_m_value(self) -> MValue {
        match self {
            Self::I32(v) => MValue::I32(Some(v)),
            Self::U32(v) => MValue::U32(Some(v)),
            Self::I64(v) => MValue::I64(Some(v)),
            Self::U64(v) => MValue::U64(Some(v)),
            Self::Str(v) => MValue::Str(Some(v)),
        }
    }

    /// These values as the leaves of a nested list.
    fn into_list(self) -> NestedValue {
        fn leaves<T>(values: Vec<T>, leaf: fn(Option<T>) -> PropValue) -> NestedValue {
            NestedValue::list(values.into_iter().map(|v| NestedValue::Leaf(leaf(Some(v)))))
        }
        match self {
            Self::I32(v) => leaves(v, PropValue::I32),
            Self::U32(v) => leaves(v, PropValue::U32),
            Self::I64(v) => leaves(v, PropValue::I64),
            Self::U64(v) => leaves(v, PropValue::U64),
            Self::Str(v) => leaves(v, PropValue::Str),
        }
    }

    /// The values [`Self::into_m_value`] wrote as `kind`, or [`None`] for a null.
    fn from_m_value(kind: ScalarKind, value: &MValue) -> AnyResult<Option<Self>> {
        Ok(match (kind, value) {
            (ScalarKind::I32, MValue::I32(v)) => v.clone().map(Self::I32),
            (ScalarKind::U32, MValue::U32(v)) => v.clone().map(Self::U32),
            (ScalarKind::I64, MValue::I64(v)) => v.clone().map(Self::I64),
            (ScalarKind::U64, MValue::U64(v)) => v.clone().map(Self::U64),
            (ScalarKind::Str, MValue::Str(v)) => v.clone().map(Self::Str),
            _ => return Err(anyhow!("{value:?} is not a {kind:?} m-value")),
        })
    }

    /// The values [`Self::into_list`] wrote as `kind`, or [`None`] for a null.
    fn from_list(kind: ScalarKind, value: &NestedValue) -> AnyResult<Option<Self>> {
        let NestedValue::List(items) = value else {
            return Err(anyhow!("{value:?} is not a list"));
        };
        let Some(items) = items else {
            return Ok(None);
        };
        let mut values = Self::empty(kind);
        for item in items {
            let NestedValue::Leaf(leaf) = item else {
                return Err(anyhow!("{item:?} is not a {kind:?} value"));
            };
            match (&mut values, leaf) {
                (Self::I32(v), PropValue::I32(Some(x))) => v.push(*x),
                (Self::U32(v), PropValue::U32(Some(x))) => v.push(*x),
                (Self::I64(v), PropValue::I64(Some(x))) => v.push(*x),
                (Self::U64(v), PropValue::U64(Some(x))) => v.push(*x),
                (Self::Str(v), PropValue::Str(Some(x))) => v.push(x.clone()),
                _ => return Err(anyhow!("{leaf:?} is not a {kind:?} value")),
            }
        }
        Ok(Some(values))
    }

    fn empty(kind: ScalarKind) -> Self {
        match kind {
            ScalarKind::I32 => Self::I32(Vec::new()),
            ScalarKind::U32 => Self::U32(Vec::new()),
            ScalarKind::I64 => Self::I64(Vec::new()),
            ScalarKind::U64 => Self::U64(Vec::new()),
            ScalarKind::Str => Self::Str(Vec::new()),
        }
    }
}

/// `tokens` as numbers, each after the first added to the one before when `running_sum` is set.
fn numbers<T>(tokens: &[&str], running_sum: bool) -> AnyResult<Vec<T>>
where
    T: FromStr + Copy + Into<i128> + TryFrom<i128> + Display,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let mut values: Vec<T> = Vec::with_capacity(tokens.len());
    for token in tokens {
        let value = match values.last() {
            Some(&prev) if running_sum => {
                let sum = prev.into() + number::<i128>(token)?;
                T::try_from(sum).map_err(|_| anyhow!("{prev} plus {token} overflows"))?
            }
            _ => number(token)?,
        };
        values.push(value);
    }
    Ok(values)
}

/// `values` as tokens, each after the first written as its difference from the one before when `running_sum` is set.
fn number_tokens<T: Copy + Into<i128> + Display>(values: &[T], running_sum: bool) -> Vec<String> {
    let mut prev: Option<i128> = None;
    values
        .iter()
        .map(|&v| {
            let token = match prev {
                Some(p) if running_sum => (v.into() - p).to_string(),
                _ => v.to_string(),
            };
            prev = Some(v.into());
            token
        })
        .collect()
}

fn number<T: FromStr>(s: &str) -> AnyResult<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    s.parse().with_context(|| format!("{s:?} is not a number"))
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
        parse_fields(&layer, forms).with_context(|| format!("layer {}", layer.name()))
    }

    /// `layer` with the columns [`Self::apply`] parsed formatted back into the string properties they came from.
    pub fn restore(&self, layer: TileLayer) -> AnyResult<TileLayer> {
        let Some(forms) = self.layers.get(layer.name()) else {
            return Ok(layer);
        };
        format_fields(&layer, forms).with_context(|| format!("layer {}", layer.name()))
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
    List(NestedKey, FieldForm),
    MValue(MValueKey, FieldForm),
}

fn parse_fields(layer: &TileLayer, forms: &BTreeMap<String, FieldForm>) -> AnyResult<TileLayer> {
    for name in forms.keys() {
        ensure!(
            !layer.m_value_names().contains(name) && !layer.nested_names().contains(name),
            "{name} is already a typed column"
        );
    }
    let mut out = empty_like(layer)?;
    let mut targets = Vec::with_capacity(layer.property_names().len());
    for (index, name) in layer.property_names().iter().enumerate() {
        let kind = property_kind(layer, index);
        let Some(&form) = forms.get(name) else {
            targets.push(Parsed::Property(out.add_property(name, kind)?));
            continue;
        };
        ensure!(
            kind == PropKind::Str,
            "{name} holds {} values, but only strings are parsed",
            <&str>::from(kind)
        );
        targets.push(match form.into {
            Column::List => {
                let list = NestedKind::list(NestedKind::Leaf(form.kind.prop_kind()));
                Parsed::List(out.add_nested(name, list)?, form)
            }
            Column::MValue => Parsed::MValue(out.add_m_value(name, form.kind.prop_kind())?, form),
        });
    }
    let m_keys = copy_m_value_columns(layer, &mut out)?;
    let nested_keys = copy_nested_columns(layer, &mut out)?;

    for feature in layer.features() {
        let mut row = row_like(&mut out, feature)?;
        for ((target, value), name) in targets
            .iter()
            .zip(feature.properties())
            .zip(layer.property_names())
        {
            match (target, value) {
                (Parsed::Property(key), value) => {
                    row.property(*key, value.clone())?;
                }
                (Parsed::List(key, form), PropValue::Str(Some(s))) => {
                    let values = form
                        .parse(s)
                        .with_context(|| field_context(name, feature))?;
                    row.nested(*key, values.into_list())?;
                }
                (Parsed::MValue(key, form), PropValue::Str(Some(s))) => {
                    let values = form
                        .parse_per_vertex(s, feature)
                        .with_context(|| field_context(name, feature))?;
                    row.m_value(*key, values.into_m_value())?;
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
    Formatted(PropertyKey, FieldForm),
}

fn format_fields(layer: &TileLayer, forms: &BTreeMap<String, FieldForm>) -> AnyResult<TileLayer> {
    let mut out = empty_like(layer)?;
    let properties = (0..layer.property_names().len())
        .map(|index| out.add_property(&layer.property_names()[index], property_kind(layer, index)))
        .collect::<Result<Vec<_>, _>>()?;
    let formatted = |name: &str, into: Column| forms.get(name).filter(|form| form.into == into);
    let m_values = layer
        .m_value_names()
        .iter()
        .zip(layer.m_value_kinds())
        .map(|(name, &kind)| match formatted(name, Column::MValue) {
            Some(&form) => out
                .add_property(name, PropKind::Str)
                .map(|key| Restored::Formatted(key, form)),
            None => out.add_m_value(name, kind).map(Restored::Kept),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let nested = layer
        .nested_names()
        .iter()
        .zip(layer.nested_kinds())
        .map(|(name, kind)| match formatted(name, Column::List) {
            Some(&form) => out
                .add_property(name, PropKind::Str)
                .map(|key| Restored::Formatted(key, form)),
            None => out.add_nested(name, kind.clone()).map(Restored::Kept),
        })
        .collect::<Result<Vec<_>, _>>()?;

    for feature in layer.features() {
        let mut row = row_like(&mut out, feature)?;
        for (key, value) in properties.iter().zip(feature.properties()) {
            row.property(*key, value.clone())?;
        }
        for ((restored, value), name) in m_values
            .iter()
            .zip(feature.m_values())
            .zip(layer.m_value_names())
        {
            match restored {
                Restored::Kept(key) => {
                    row.m_value(*key, value.clone())?;
                }
                Restored::Formatted(key, form) => {
                    let s = Values::from_m_value(form.kind, value)?
                        .map(|v| form.write(&v))
                        .transpose()
                        .with_context(|| field_context(name, feature))?;
                    row.property(*key, PropValue::Str(s))?;
                }
            }
        }
        for ((restored, value), name) in nested
            .iter()
            .zip(feature.nested())
            .zip(layer.nested_names())
        {
            match restored {
                Restored::Kept(key) => {
                    row.nested(*key, value.clone())?;
                }
                Restored::Formatted(key, form) => {
                    let s = Values::from_list(form.kind, value)?
                        .map(|v| form.write(&v))
                        .transpose()
                        .with_context(|| field_context(name, feature))?;
                    row.property(*key, PropValue::Str(s))?;
                }
            }
        }
        row.finish()?;
    }
    Ok(out.finish())
}

/// A builder for a layer with the name, extent and z step of `layer`, and no columns yet.
fn empty_like(layer: &TileLayer) -> AnyResult<TileLayerBuilder> {
    let mut out = TileLayer::builder(layer.name(), layer.extent().get())?;
    if let Some(step) = layer.z_step() {
        out.set_z_step(step)?;
    }
    Ok(out)
}

/// A row with the geometry, z and id of `feature`, and no values yet.
fn row_like<'a>(
    out: &'a mut TileLayerBuilder,
    feature: &TileFeature,
) -> AnyResult<TileFeatureBuilder<'a>> {
    let mut row = out.feature(feature.geometry().clone());
    row.id(feature.id());
    if !feature.z().is_empty() {
        row.z(feature.z().to_vec())?;
    }
    Ok(row)
}

fn property_kind(layer: &TileLayer, index: usize) -> PropKind {
    layer
        .features()
        .first()
        .map_or(PropKind::Str, |f| f.properties()[index].kind())
}

fn copy_m_value_columns(
    layer: &TileLayer,
    out: &mut TileLayerBuilder,
) -> AnyResult<Vec<MValueKey>> {
    Ok(layer
        .m_value_names()
        .iter()
        .zip(layer.m_value_kinds())
        .map(|(name, &kind)| out.add_m_value(name, kind))
        .collect::<Result<Vec<_>, _>>()?)
}

fn copy_nested_columns(layer: &TileLayer, out: &mut TileLayerBuilder) -> AnyResult<Vec<NestedKey>> {
    Ok(layer
        .nested_names()
        .iter()
        .zip(layer.nested_kinds())
        .map(|(name, kind)| out.add_nested(name, kind.clone()))
        .collect::<Result<Vec<_>, _>>()?)
}

fn field_context(name: &str, feature: &TileFeature) -> String {
    match feature.id() {
        Some(id) => format!("field {name} of feature {id}"),
        None => format!("field {name}"),
    }
}
