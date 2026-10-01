use arbitrary::{Arbitrary, Unstructured};
use mlt::convert::fields::FieldConfig;
use mlt_core::{MValue, NestedKind, NestedValue, PropKind, PropValue, TileFeature, TileLayer};

use crate::field_config::token;
use crate::z::geometry;

/// A layer of typed columns, formatted back into strings by a `--config` and parsed again.
///
/// Values the config cannot write may be refused, but whatever parses must be the values it was formatted from.
#[derive(Debug)]
pub struct FieldValuesInput {
    pub layer: TileLayer,
    pub config: String,
}

const KINDS: [(PropKind, &str); 5] = [
    (PropKind::I32, "i32"),
    (PropKind::U32, "u32"),
    (PropKind::I64, "i64"),
    (PropKind::U64, "u64"),
    (PropKind::Str, "str"),
];

impl Arbitrary<'_> for FieldValuesInput {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let (m_kind, m_name) = *u.choose(&KINDS)?;
        let (n_kind, n_name) = *u.choose(&KINDS)?;
        let mut out = TileLayer::builder("l", 4096).expect("a valid layer");
        let m = out.add_m_value("m", m_kind).expect("a new name");
        let n = out
            .add_nested("n", NestedKind::list(NestedKind::Leaf(n_kind)))
            .expect("a new name");
        for _ in 0..u.int_in_range(1..=4u8)? {
            let geometry = geometry(u)?;
            let vertices = TileFeature::new(geometry.clone()).vertex_count();
            let mut row = out.feature(geometry);
            row.id(u.arbitrary::<Option<u8>>()?.map(u64::from));
            let m_value = if u.arbitrary()? {
                m_values(u, m_kind, vertices)?
            } else {
                MValue::null(m_kind)
            };
            row.m_value(m, m_value).expect("one value per vertex");
            let nested = if u.arbitrary()? {
                let len = u.int_in_range(0..=4)?;
                NestedValue::list(
                    (0..len)
                        .map(|_| Ok(NestedValue::Leaf(leaf(u, n_kind)?)))
                        .collect::<arbitrary::Result<Vec<_>>>()?,
                )
            } else {
                NestedValue::List(None)
            };
            row.nested(n, nested).expect("a declared shape");
            row.finish().expect("a complete row");
        }
        let mut forms = toml::Table::new();
        forms.insert("m".to_owned(), form(u, m_name, "m-value")?.into());
        forms.insert("n".to_owned(), form(u, n_name, "list")?.into());
        let mut layers = toml::Table::new();
        layers.insert("l".to_owned(), forms.into());
        let mut config = toml::Table::new();
        config.insert("layers".to_owned(), layers.into());
        Ok(Self {
            layer: out.finish(),
            config: config.to_string(),
        })
    }
}

fn form(u: &mut Unstructured<'_>, kind: &str, into: &str) -> arbitrary::Result<toml::Table> {
    let mut form = toml::Table::new();
    let split = *u.choose(&[",", ";", " ", "|", "é", "+", "-", "1", "sign"])?;
    form.insert("split".to_owned(), split.into());
    form.insert("kind".to_owned(), kind.into());
    form.insert("running-sum".to_owned(), u.arbitrary::<bool>()?.into());
    form.insert("into".to_owned(), into.into());
    Ok(form)
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the kinds a form parses into are generated"
)]
fn m_values(u: &mut Unstructured<'_>, kind: PropKind, n: usize) -> arbitrary::Result<MValue> {
    fn values<T: for<'a> Arbitrary<'a>>(
        u: &mut Unstructured<'_>,
        n: usize,
    ) -> arbitrary::Result<Vec<T>> {
        (0..n).map(|_| u.arbitrary()).collect()
    }
    Ok(match kind {
        PropKind::I32 => MValue::I32(Some(values(u, n)?)),
        PropKind::U32 => MValue::U32(Some(values(u, n)?)),
        PropKind::I64 => MValue::I64(Some(values(u, n)?)),
        PropKind::U64 => MValue::U64(Some(values(u, n)?)),
        PropKind::Str => MValue::Str(Some(
            (0..n).map(|_| token(u)).collect::<arbitrary::Result<_>>()?,
        )),
        other => unreachable!("{other:?} is not generated"),
    })
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the kinds a form parses into are generated"
)]
fn leaf(u: &mut Unstructured<'_>, kind: PropKind) -> arbitrary::Result<PropValue> {
    Ok(match kind {
        PropKind::I32 => PropValue::I32(Some(u.arbitrary()?)),
        PropKind::U32 => PropValue::U32(Some(u.arbitrary()?)),
        PropKind::I64 => PropValue::I64(Some(u.arbitrary()?)),
        PropKind::U64 => PropValue::U64(Some(u.arbitrary()?)),
        PropKind::Str => PropValue::Str(Some(token(u)?)),
        other => unreachable!("{other:?} is not generated"),
    })
}

impl FieldValuesInput {
    pub fn fuzz(self) {
        let Ok(fields) = self.config.parse::<FieldConfig>() else {
            return;
        };
        let Ok(strings) = fields.restore(self.layer.clone()) else {
            return;
        };
        if let Ok(parsed) = fields.apply(strings) {
            assert_eq!(
                parsed, self.layer,
                "the formatted strings parse to other values"
            );
        }
    }
}
