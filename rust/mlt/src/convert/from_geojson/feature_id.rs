//! The `GeoJSON` feature `id` as an MLT feature id.

use anyhow::{Result as AnyResult, bail};
use geojson::Feature;
use geojson::feature::Id;

/// The feature `id` as an MLT feature id, which must be a non-negative integer
/// when present.
pub(super) fn feature_id(feature: &Feature) -> AnyResult<Option<u64>> {
    let Some(id) = feature.id.as_ref() else {
        return Ok(None);
    };
    match id {
        Id::Number(n) => match n.as_u64() {
            Some(id) => Ok(Some(id)),
            None => bail!("id {n} is not a non-negative integer, which is all an MLT id can hold"),
        },
        Id::String(s) => bail!(
            "id {s:?} is a string, but an MLT id must be a non-negative integer; drop it or move it to a property"
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn with_id(id: &serde_json::Value) -> Feature {
        serde_json::from_value(
            json!({ "type": "Feature", "id": id, "geometry": null, "properties": {} }),
        )
        .unwrap()
    }

    #[test]
    fn non_negative_integer_ids_are_kept_and_an_absent_id_is_none() {
        assert_eq!(feature_id(&with_id(&json!(42))).unwrap(), Some(42));
        assert_eq!(feature_id(&with_id(&json!(0))).unwrap(), Some(0));
        let no_id: Feature =
            serde_json::from_value(json!({ "type": "Feature", "geometry": null })).unwrap();
        assert_eq!(feature_id(&no_id).unwrap(), None);
    }

    #[test]
    fn other_ids_are_rejected() {
        let errors: Vec<String> = [json!(-1), json!(3.5), json!("abc")]
            .iter()
            .map(|id| feature_id(&with_id(id)).unwrap_err().to_string())
            .collect();
        insta::assert_snapshot!(errors.join("\n"), @r#"
        id -1 is not a non-negative integer, which is all an MLT id can hold
        id 3.5 is not a non-negative integer, which is all an MLT id can hold
        id "abc" is a string, but an MLT id must be a non-negative integer; drop it or move it to a property
        "#);
    }
}
