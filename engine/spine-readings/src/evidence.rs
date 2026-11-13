//! The contract curve floor and its full bibliographic record, compiled in.
//! The floor names contract curves; it does not filter level-rule geometry.

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PublishedValue {
    Scalar { value: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unit {
    Degrees,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadStatus {
    FullText,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub authors: String,
    pub year: u16,
    pub title: String,
    pub journal: String,
    pub volume: String,
    #[serde(deserialize_with = "nullable")]
    pub issue: Option<String>,
    pub pages: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub key: String,
    pub citation: Citation,
    pub pmid: String,
    pub doi: String,
    pub pmcid: String,
    pub read: ReadStatus,
    #[serde(deserialize_with = "nullable")]
    pub n: Option<String>,
    pub population: String,
    pub label: String,
    pub url: String,
    pub notes: String,
}

fn nullable<'de, D, T>(deserialiser: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserialiser)
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub value: PublishedValue,
    pub unit: Unit,
    pub source: String,
    pub sources: Vec<Source>,
}

fn parse(text: &str) -> Result<Evidence, String> {
    let evidence: Evidence = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let PublishedValue::Scalar { value } = evidence.value;
    if !value.is_finite() || value <= 0.0 {
        return Err("the curve floor must be finite and positive".into());
    }
    if evidence
        .sources
        .iter()
        .filter(|s| s.key == evidence.source)
        .count()
        != 1
    {
        return Err("the curve floor must resolve to one source".into());
    }
    Ok(evidence)
}

/// The strict embedded table, read once without runtime file access.
pub fn table() -> &'static Evidence {
    static TABLE: OnceLock<Evidence> = OnceLock::new();
    TABLE.get_or_init(|| {
        parse(include_str!("../evidence/curve-floor.json"))
            .expect("embedded curve evidence is well formed")
    })
}

/// Cobb magnitude at which a nominal curve enters the contract output.
pub fn curve_floor_deg() -> f64 {
    let PublishedValue::Scalar { value } = table().value;
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn the_floor_retains_its_supplied_guideline_and_bibliography() {
        assert_eq!(curve_floor_deg(), 10.0);
        assert_eq!(table().unit, Unit::Degrees);
        let source = &table().sources[0];
        assert_eq!(source.pmid, "29435499");
        assert_eq!(source.doi, "10.1186/s13013-017-0145-8");
        assert_eq!(source.pmcid, "PMC5795289");
        assert_eq!(source.citation.year, 2018);
        assert_eq!(source.citation.volume, "13");
        assert_eq!(source.citation.pages, "3");
        assert!(source.citation.authors.ends_with("Kotwicki T, et al."));
        assert_eq!(source.read, ReadStatus::FullText);
        assert_eq!(source.label, "guideline");
    }

    #[test]
    fn evidence_unknown_fields_missing_fields_and_broken_links_are_refused() {
        let original: Value =
            serde_json::from_str(include_str!("../evidence/curve-floor.json")).unwrap();
        for path in ["", "/value", "/sources/0", "/sources/0/citation"] {
            let mut changed = original.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("extra".into(), json!(true));
            assert!(parse(&changed.to_string()).is_err(), "{path}");
        }
        for (path, field) in [
            ("", "unit"),
            ("/sources/0", "n"),
            ("/sources/0/citation", "issue"),
        ] {
            let mut changed = original.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(parse(&changed.to_string()).is_err(), "{path}/{field}");
        }
        let mut changed = original;
        changed["source"] = json!("missing");
        assert!(parse(&changed.to_string()).is_err());
    }
}
