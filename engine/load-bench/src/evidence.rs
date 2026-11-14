//! Compiled evidence in the units and precision transcribed in the notes.
//! [`Evidence::read`] parses all six files strictly; [`Evidence::value`]
//! performs an exact lookup. It never interpolates, chooses a neighbouring
//! row, supplies a missing number, or silently converts a unit.
//!
//! `Bench` rows hold chosen inputs or labelled priors. `Beside` preserves
//! conflicting evidence; `Input` holds inputs behind an inference. Read
//! status describes the research note's access, not a new paper inspection.
//! See `evidence/README.md` for keys, provenance and known gaps.

use serde::Deserialize;
use shapes::implant_layout::RodMaterial;

/// The five numerical tables, in a stable order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    JointSprings,
    RodMaterials,
    AnchorStrength,
    DailyLoads,
    Fatigue,
}

impl TableKind {
    pub const ALL: [Self; 5] = [
        Self::JointSprings,
        Self::RodMaterials,
        Self::AnchorStrength,
        Self::DailyLoads,
        Self::Fatigue,
    ];
}

/// Chosen for the bench, shown beside it, or retained to redo an inference.
/// This classification does not select or combine cells automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Usage {
    Bench,
    Beside,
    Input,
}

/// A reported number, or a calculation/choice made in the notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Basis {
    Published,
    OwnInference,
}

/// Units as written. `Unstated` does not turn presumed units into facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Unit {
    #[serde(rename = "N/mm")]
    NPerMm,
    #[serde(rename = "N·mm/deg")]
    NmmPerDegree,
    #[serde(rename = "Nm/deg")]
    NmPerDegree,
    #[serde(rename = "kN/mm")]
    KnPerMm,
    #[serde(rename = "GPa")]
    Gpa,
    #[serde(rename = "MPa")]
    Mpa,
    #[serde(rename = "ksi")]
    Ksi,
    #[serde(rename = "N")]
    N,
    #[serde(rename = "N·m")]
    Nm,
    #[serde(rename = "N·m²")]
    NmSquared,
    #[serde(rename = "mm")]
    Mm,
    #[serde(rename = "mm³")]
    MmCubed,
    #[serde(rename = "mm⁴")]
    MmToFourth,
    #[serde(rename = "kg")]
    Kg,
    #[serde(rename = "kg/m³")]
    KgPerCubicMetre,
    #[serde(rename = "mg/cm³")]
    MgPerCubicCentimetre,
    #[serde(rename = "%")]
    Percent,
    #[serde(rename = "ratio")]
    Ratio,
    #[serde(rename = "fraction-of-body-weight")]
    BodyWeightFraction,
    #[serde(rename = "steps/day")]
    StepsPerDay,
    #[serde(rename = "cycles")]
    Cycles,
    #[serde(rename = "million cycles/year")]
    MillionCyclesPerYear,
    #[serde(rename = "reversals")]
    Reversals,
    #[serde(rename = "million reversals")]
    MillionReversals,
    #[serde(rename = "people")]
    People,
    #[serde(rename = "rods")]
    Rods,
    #[serde(rename = "deg")]
    Degrees,
    #[serde(rename = "days/year")]
    DaysPerYear,
    #[serde(rename = "Hz")]
    Hz,
    #[serde(rename = "min")]
    Minutes,
    #[serde(rename = "W")]
    Watts,
    #[serde(rename = "unstated")]
    Unstated,
}

/// One cell. Endpoints, SDs and SEs have separate terms, each with its
/// source and note section. `approximate` preserves the note's qualification.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Value {
    pub term: String,
    pub value: f64,
    pub unit: Unit,
    pub source: String,
    pub section: String,
    pub basis: Basis,
    pub approximate: bool,
}

/// One setting/material/population. Read `notes` before combining cells:
/// axis mappings, adult borrowing and inference assumptions live here.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Row {
    pub key: String,
    pub usage: Usage,
    pub description: String,
    pub notes: String,
    pub values: Vec<Value>,
}

/// A known absence; no numerical cell is fabricated for it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MissingValue {
    pub key: String,
    pub reason: String,
}

/// Strict JSON table, public for inspection and refusal tests.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Table {
    pub rows: Vec<Row>,
    pub missing: Vec<MissingValue>,
}

/// How far the research note read the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Read {
    FullText,
    Abstract,
    Secondary,
}

/// Bibliography as available in the notes. Missing titles or journals
/// stay `None`; a descriptive note heading is not a paper title.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Citation {
    pub authors: String,
    pub year: u32,
    pub title: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
}

/// Source metadata. Textual `n` preserves compound sample descriptions.
/// Identifiers are never inferred from a PMC link. `label` distinguishes
/// own inferences from peer-reviewed evidence; a missing `n` stays `None`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub key: String,
    pub citation: Citation,
    pub pmid: Option<String>,
    pub doi: Option<String>,
    pub pmcid: Option<String>,
    /// The supplied audit that identified this bibliographic record.
    /// `None` means no record for this paper was supplied in that audit.
    pub record_note: Option<String>,
    pub read: Read,
    pub n: Option<String>,
    pub population: String,
    pub label: String,
    pub url: Option<String>,
    pub notes: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sources {
    sources: Vec<Source>,
}

/// Bundled tables and bibliography; no runtime file access.
#[derive(Debug)]
pub struct Evidence {
    tables: [Table; 5],
    sources: Vec<Source>,
}

/// The chosen paired strength ranges, in MPa, with sourced endpoints.
/// A version must use the same quantile for yield and ultimate strength,
/// as set in the rod-strength decision. These are exact endpoint lookups;
/// this reader does not sample or interpolate them.
#[derive(Clone, Copy, Debug)]
pub struct StrengthPair<'a> {
    pub yield_lower: &'a Value,
    pub yield_upper: &'a Value,
    pub ultimate_lower: &'a Value,
    pub ultimate_upper: &'a Value,
}

impl Evidence {
    /// Read bundled JSON with unknown fields refused at every object.
    /// Parse errors reach the caller and are never replaced by defaults.
    pub fn read() -> Result<Self, serde_json::Error> {
        Ok(Self {
            tables: [
                serde_json::from_str(include_str!("../evidence/joint-springs.json"))?,
                serde_json::from_str(include_str!("../evidence/rod-materials.json"))?,
                serde_json::from_str(include_str!("../evidence/anchor-strength.json"))?,
                serde_json::from_str(include_str!("../evidence/daily-loads.json"))?,
                serde_json::from_str(include_str!("../evidence/fatigue.json"))?,
            ],
            sources: serde_json::from_str::<Sources>(include_str!("../evidence/sources.json"))?
                .sources,
        })
    }

    /// Inspect a table, preserving file order.
    pub fn table(&self, kind: TableKind) -> &Table {
        &self.tables[match kind {
            TableKind::JointSprings => 0,
            TableKind::RodMaterials => 1,
            TableKind::AnchorStrength => 2,
            TableKind::DailyLoads => 3,
            TableKind::Fatigue => 4,
        }]
    }

    /// Exact row key; no region, alloy or level fallback.
    pub fn row(&self, kind: TableKind, key: &str) -> Option<&Row> {
        self.table(kind).rows.iter().find(|row| row.key == key)
    }

    /// Exact row and term. Missing evidence returns `None`.
    pub fn value(&self, kind: TableKind, key: &str, term: &str) -> Option<&Value> {
        self.row(kind, key)?
            .values
            .iter()
            .find(|cell| cell.term == term)
    }

    /// Chosen yield/ultimate pair: Prasad body text for titanium, Table 1
    /// for cobalt chrome and stainless steel. Young's modulus remains an
    /// independent exact lookup; the conflicting titanium Table 1 row is
    /// retained beside the chosen pair. Missing endpoints return `None`.
    pub fn bench_strengths(&self, material: RodMaterial) -> Option<StrengthPair<'_>> {
        let key = match material {
            RodMaterial::TitaniumAlloy => "titanium-alloy-body-text",
            RodMaterial::CobaltChrome => "cobalt-chrome",
            RodMaterial::StainlessSteel => "stainless-steel",
        };
        Some(StrengthPair {
            yield_lower: self.value(TableKind::RodMaterials, key, "yield-lower")?,
            yield_upper: self.value(TableKind::RodMaterials, key, "yield-upper")?,
            ultimate_lower: self.value(TableKind::RodMaterials, key, "uts-lower")?,
            ultimate_upper: self.value(TableKind::RodMaterials, key, "uts-upper")?,
        })
    }

    /// Resolve a source key without aliases or inferred identifiers.
    pub fn source(&self, key: &str) -> Option<&Source> {
        self.sources.iter().find(|source| source.key == key)
    }

    /// Inspect the bibliography in its recorded order.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
}

/// Convert N·mm/degree to N·mm/radian: a radian is 180/π degrees.
/// The stored cell stays unchanged. This supplies no moment-axis mapping.
pub fn nmm_per_degree_to_nmm_per_radian(value: f64) -> f64 {
    value * (180.0 / core::f64::consts::PI)
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_degree_of_a_radian_spring_carries_the_same_moment() {
        // Closed form: π N·mm/degree = 180 N·mm/radian.
        assert!(
            (super::nmm_per_degree_to_nmm_per_radian(core::f64::consts::PI) - 180.0).abs() < 1e-12
        );
    }

    #[test]
    fn the_bibliography_envelope_refuses_unknown_fields() {
        assert!(serde_json::from_str::<super::Sources>(r#"{"sources":[],"extra":1}"#).is_err());
    }
}
