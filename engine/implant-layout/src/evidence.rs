//! Published placement tables, embedded and parsed once. Coordinates match
//! exactly: absent cells return `None`. This module never chooses a source,
//! combines sexes, converts units, interpolates, or applies a placement rule.
//!
//! Start with [`tables`], then [`Evidence::lookup`]. Each cell retains its
//! published unit, source key, note section and qualifications. Read status
//! records what the research note read, rather than a new primary-source check.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;
use shapes::implant_layout::CurveSide;
use shapes::levels::Level;

/// One file per table, in a stable iteration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Table {
    PedicleWidths,
    InnerWidths,
    InnerWalls,
    Chords,
    MedialAngles,
    ConcaveRatios,
    ChannelGrades,
    ScrewGrids,
    DiameterRules,
    LengthRules,
    LevelRuleThresholds,
    StopShortOdds,
    RodChoices,
    AnchorPatterns,
    PlacementChecks,
    /// Gao's short pedicle-only measurement, distinct from a full chord.
    PedicleLengths,
    /// Published sagittal or inner/outer pedicle height observations.
    PedicleHeights,
    /// Observed screw counts, percentages and intervals in Balikci.
    ScrewUse,
}

impl Table {
    /// All tables, in a stable order.
    pub const ALL: [Self; 18] = [
        Self::PedicleWidths,
        Self::InnerWidths,
        Self::InnerWalls,
        Self::Chords,
        Self::MedialAngles,
        Self::ConcaveRatios,
        Self::ChannelGrades,
        Self::ScrewGrids,
        Self::DiameterRules,
        Self::LengthRules,
        Self::LevelRuleThresholds,
        Self::StopShortOdds,
        Self::RodChoices,
        Self::AnchorPatterns,
        Self::PlacementChecks,
        Self::PedicleLengths,
        Self::PedicleHeights,
        Self::ScrewUse,
    ];

    fn text(self) -> &'static str {
        match self {
            Self::PedicleWidths => include_str!("../evidence/pedicle-widths.json"),
            Self::InnerWidths => include_str!("../evidence/inner-widths.json"),
            Self::InnerWalls => include_str!("../evidence/inner-walls.json"),
            Self::Chords => include_str!("../evidence/chords.json"),
            Self::MedialAngles => include_str!("../evidence/medial-angles.json"),
            Self::ConcaveRatios => include_str!("../evidence/concave-ratios.json"),
            Self::ChannelGrades => include_str!("../evidence/channel-grades.json"),
            Self::ScrewGrids => include_str!("../evidence/screw-grids.json"),
            Self::DiameterRules => include_str!("../evidence/diameter-rules.json"),
            Self::LengthRules => include_str!("../evidence/length-rules.json"),
            Self::LevelRuleThresholds => include_str!("../evidence/level-rule-thresholds.json"),
            Self::StopShortOdds => include_str!("../evidence/stop-short-odds.json"),
            Self::RodChoices => include_str!("../evidence/rod-choices.json"),
            Self::AnchorPatterns => include_str!("../evidence/anchor-patterns.json"),
            Self::PlacementChecks => include_str!("../evidence/placement-checks.json"),
            Self::PedicleLengths => include_str!("../evidence/pedicle-lengths.json"),
            Self::PedicleHeights => include_str!("../evidence/pedicle-heights.json"),
            Self::ScrewUse => include_str!("../evidence/screw-use.json"),
        }
    }
}

/// Published age bands. Choosing a band from an age belongs to callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum AgeBand {
    #[serde(rename = "5-8")]
    FiveToEight,
    #[serde(rename = "9-11")]
    NineToEleven,
    #[serde(rename = "12-14")]
    TwelveToFourteen,
}

/// Original male and female columns, never combined here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sex {
    Male,
    Female,
}

/// Complete cell coordinate. `None` means that the published cell has no
/// such dimension; it is never a wildcard. `group` distinguishes cohorts,
/// curve positions, landmarks or techniques documented in the files.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coordinate {
    pub key: String,
    #[serde(default, deserialize_with = "present")]
    pub level: Option<Level>,
    #[serde(default, deserialize_with = "present")]
    pub curve_side: Option<CurveSide>,
    #[serde(default, deserialize_with = "present")]
    pub age_band: Option<AgeBand>,
    #[serde(default, deserialize_with = "present")]
    pub sex: Option<Sex>,
    #[serde(default, deserialize_with = "present")]
    pub group: Option<String>,
}

// Omission is allowed, but an explicit null is not a coordinate or qualifier.
fn present<'de, D, T>(deserialiser: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserialiser).map(Some)
}

/// Units as printed. Percentages are never converted to fractions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unit {
    Mm,
    Degrees,
    Ratio,
    /// A correlation coefficient, distinct from a ratio of measurements.
    Correlation,
    Percent,
    Count,
    Levels,
    ImplantsPerLevel,
    NashMoeGrade,
    Years,
}

/// The form supplied by the note, with no unreported spread added.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PublishedValue {
    Scalar {
        value: f64,
    },
    /// Literal ± notation when the spread statistic is not named.
    /// This must not be treated as an SD or an SE.
    PlusMinus {
        value: f64,
        spread: f64,
    },
    MeanSd {
        mean: f64,
        sd: f64,
    },
    /// Median and, when transcribed, its interquartile range. An omitted
    /// IQR means the supplied audit did not give that number.
    Median {
        value: f64,
        #[serde(default, deserialize_with = "present")]
        iqr: Option<f64>,
    },
    Range {
        lower: f64,
        upper: f64,
    },
    /// Only explicitly enumerated published values, never expanded ranges.
    Grid {
        values: Vec<f64>,
    },
    /// Named bounds remain levels, without conversion to numeric indices.
    LevelRange {
        first: Level,
        last: Level,
    },
    /// An anatomical disc is retained as the two levels the note names.
    Disc {
        upper: Level,
        lower: Level,
    },
}

/// Whether the printed number is available to a placement calculation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Usage {
    #[default]
    Usable,
    /// Retained for inspection because the source contains a known problem.
    PrintedOnly,
}

/// A range reported for the same observation, in the cell's unit.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedRange {
    pub lower: f64,
    pub upper: f64,
}

/// The transcription note's qualification, without repairing its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkKind {
    Impossible,
    Doubtful,
    ProbableCopy,
    ProbableMisprint,
    AmbiguousPrint,
}

/// A source problem and its reported reason. `usage` separately records
/// whether the observation is available to an ordinary lookup.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub kind: MarkKind,
    pub reason: String,
}

/// One sourced cell. The registry supplies n, population and read status.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cell {
    pub coordinate: Coordinate,
    pub value: PublishedValue,
    pub unit: Unit,
    pub source: String,
    pub section: String,
    #[serde(default, deserialize_with = "present")]
    pub qualifier: Option<String>,
    #[serde(default)]
    pub usage: Usage,
    /// The source prints the measurement to a whole millimetre. How a caller
    /// handles that rounding is a project rule, not part of this reader.
    #[serde(default)]
    pub whole_mm: bool,
    /// Number of observed pedicles for this cell, where supplied.
    #[serde(default, deserialize_with = "present")]
    pub n: Option<u32>,
    #[serde(default, deserialize_with = "present")]
    pub range: Option<ObservedRange>,
    /// Literal cell text, including misprints and significance markers.
    #[serde(default, deserialize_with = "present")]
    pub printed: Option<String>,
    /// Decimal places in the printed central value, without inferring
    /// precision from its parsed floating-point representation.
    #[serde(default, deserialize_with = "present")]
    pub decimals: Option<u8>,
    #[serde(default, deserialize_with = "present")]
    pub mark: Option<Mark>,
}

impl Cell {
    /// Why this printed observation is excluded from ordinary lookups.
    /// Doubtful but usable observations instead expose their `mark`.
    pub fn not_for_use_reason(&self) -> Option<&str> {
        (self.usage == Usage::PrintedOnly)
            .then(|| {
                self.mark
                    .as_ref()
                    .map(|mark| mark.reason.as_str())
                    .or(self.qualifier.as_deref())
            })
            .flatten()
    }
}

/// A source's measurement definition or table wording, not a placement rule.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub key: String,
    pub source: String,
    pub section: String,
    pub text: String,
}

/// How much of the source the research note reports reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadStatus {
    FullText,
    Abstract,
    Secondary,
}

/// Bibliographic fields as supplied by the citation audit. Null issue,
/// volume or pages remain absent; they are never filled from another source.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    #[serde(deserialize_with = "nullable")]
    pub authors: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub year: Option<u16>,
    #[serde(deserialize_with = "nullable")]
    pub title: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub journal: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub volume: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub issue: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub pages: Option<String>,
}

/// Common source-record form shared with the load bench. Read status for
/// bibliographic-only lookups does not imply the numbers were audited.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub key: String,
    pub citation: Citation,
    #[serde(deserialize_with = "nullable")]
    pub pmid: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub doi: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub pmcid: Option<String>,
    pub read: ReadStatus,
    #[serde(deserialize_with = "nullable")]
    pub n: Option<String>,
    pub population: String,
    pub label: String,
    #[serde(deserialize_with = "nullable")]
    pub url: Option<String>,
    /// The former section and qualifier text, plus audit provenance.
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableFile {
    cells: Vec<Cell>,
    #[serde(default)]
    definitions: Vec<Definition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesFile {
    #[serde(deserialize_with = "source_entries")]
    sources: BTreeMap<String, Source>,
}

fn nullable<'de, D, T>(deserialiser: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserialiser)
}

// Unlike ordinary map deserialisation, a repeated source key is an error.
fn source_entries<'de, D>(deserialiser: D) -> Result<BTreeMap<String, Source>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, Source>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a source catalogue with unique keys")
        }

        fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut entries = BTreeMap::new();
            while let Some(value) = access.next_element::<Source>()? {
                let key = value.key.clone();
                if entries.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "repeated source key: {key}"
                    )));
                }
            }
            Ok(entries)
        }
    }
    deserialiser.deserialize_seq(Entries)
}

/// Immutable tables with exact lookups and stable iteration.
#[derive(Debug)]
pub struct Evidence {
    tables: BTreeMap<Table, Vec<Cell>>,
    sources: BTreeMap<String, Source>,
    definitions: BTreeMap<Table, Vec<Definition>>,
}

impl Evidence {
    /// Exact usable coordinate, or no value. Printed-only observations are
    /// excluded. Neighbouring levels, ages, sexes and sides are never substituted.
    pub fn lookup(&self, table: Table, coordinate: &Coordinate) -> Option<&Cell> {
        self.cells(table)
            .iter()
            .find(|cell| cell.coordinate == *coordinate && cell.usage == Usage::Usable)
    }

    /// Cells in file order, including alternative cohorts and printed-only
    /// observations. Inspect `usage` before using a raw cell in a calculation.
    pub fn cells(&self, table: Table) -> &[Cell] {
        &self.tables[&table]
    }

    /// Resolve a source key. Unknown keys give no value.
    pub fn source(&self, key: &str) -> Option<&Source> {
        self.sources.get(key)
    }

    /// Sources in key order.
    pub fn sources(&self) -> &BTreeMap<String, Source> {
        &self.sources
    }

    /// Published definitions and table wording, in file order.
    pub fn definitions(&self, table: Table) -> &[Definition] {
        &self.definitions[&table]
    }

    fn read() -> Result<Self, String> {
        let sources: SourcesFile = serde_json::from_str(include_str!("../evidence/sources.json"))
            .map_err(|error| format!("sources: {error}"))?;
        let mut tables = BTreeMap::new();
        let mut definitions = BTreeMap::new();
        for table in Table::ALL {
            let file: TableFile = serde_json::from_str(table.text())
                .map_err(|error| format!("{table:?}: {error}"))?;
            for (index, cell) in file.cells.iter().enumerate() {
                if cell.coordinate.key.trim().is_empty() || cell.section.trim().is_empty() {
                    return Err(format!("{table:?}: unnamed cell or section"));
                }
                if !sources.sources.contains_key(&cell.source) {
                    return Err(format!("{table:?}: unresolved source {}", cell.source));
                }
                if cell.n == Some(0)
                    || cell.range.is_some_and(|range| range.lower > range.upper)
                    || cell
                        .mark
                        .as_ref()
                        .is_some_and(|mark| mark.reason.trim().is_empty())
                    || (cell.usage == Usage::PrintedOnly && cell.not_for_use_reason().is_none())
                {
                    return Err(format!(
                        "{table:?}: incomplete observation {:?}",
                        cell.coordinate
                    ));
                }
                if file.cells[..index]
                    .iter()
                    .any(|earlier| earlier.coordinate == cell.coordinate)
                {
                    return Err(format!(
                        "{table:?}: repeated coordinate {:?}",
                        cell.coordinate
                    ));
                }
                let ordered = match &cell.value {
                    PublishedValue::PlusMinus { spread, .. } => *spread >= 0.0,
                    PublishedValue::MeanSd { sd, .. } => *sd >= 0.0,
                    PublishedValue::Median { iqr, .. } => iqr.is_none_or(|spread| spread >= 0.0),
                    PublishedValue::Range { lower, upper } => lower <= upper,
                    PublishedValue::Grid { values } => {
                        !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
                    }
                    PublishedValue::LevelRange { first, last } => first <= last,
                    PublishedValue::Disc { upper, lower } => upper < lower,
                    PublishedValue::Scalar { .. } => true,
                };
                if !ordered {
                    return Err(format!("{table:?}: malformed value {:?}", cell.coordinate));
                }
            }
            for definition in &file.definitions {
                if [&definition.key, &definition.section, &definition.text]
                    .iter()
                    .any(|text| text.trim().is_empty())
                    || !sources.sources.contains_key(&definition.source)
                {
                    return Err(format!(
                        "{table:?}: incomplete definition {}",
                        definition.key
                    ));
                }
            }
            tables.insert(table, file.cells);
            definitions.insert(table, file.definitions);
        }
        for (key, source) in &sources.sources {
            if [
                &source.key,
                &source.population,
                &source.label,
                &source.notes,
            ]
            .iter()
            .any(|field| field.trim().is_empty())
            {
                return Err(format!("{key}: incomplete source entry"));
            }
            if source
                .citation
                .title
                .as_deref()
                .is_none_or(|title| title.trim().is_empty())
            {
                return Err(format!("{key}: missing source title"));
            }
            if ![
                "peer-reviewed",
                "conference-abstract",
                "guideline",
                "research-note-summary",
            ]
            .contains(&source.label.as_str())
            {
                return Err(format!("{key}: unknown source label"));
            }
            if matches!(
                source.label.as_str(),
                "peer-reviewed" | "conference-abstract"
            ) && source.pmid.is_none()
                && source.doi.is_none()
            {
                return Err(format!("{key}: identified publication has no identifier"));
            }
            if let Some(pmid) = &source.pmid
                && (pmid.is_empty() || !pmid.bytes().all(|byte| byte.is_ascii_digit()))
            {
                return Err(format!("{key}: malformed PMID"));
            }
        }
        Ok(Self {
            tables,
            sources: sources.sources,
            definitions,
        })
    }
}

/// Read the compiled tables once, without runtime file access.
///
/// Panics if the bundled files violate their schema or source references;
/// table tests exercise the same reader before these files ship.
pub fn tables() -> &'static Evidence {
    static TABLES: OnceLock<Evidence> = OnceLock::new();
    TABLES.get_or_init(|| Evidence::read().expect("the bundled placement evidence reads strictly"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn unknown_fields_are_refused_at_every_table_boundary() {
        for table in Table::ALL {
            let value: Value = serde_json::from_str(table.text()).expect("table JSON");
            for path in ["", "/cells/0", "/cells/0/coordinate", "/cells/0/value"] {
                let mut changed = value.clone();
                changed
                    .pointer_mut(path)
                    .and_then(Value::as_object_mut)
                    .expect("a populated table boundary")
                    .insert("extra".into(), json!(1));
                assert!(
                    serde_json::from_value::<TableFile>(changed).is_err(),
                    "{table:?} {path}"
                );
            }
        }
        for value in [
            json!({"kind":"scalar", "value":1, "extra":1}),
            json!({"kind":"plus-minus", "value":1, "spread":1, "extra":1}),
            json!({"kind":"mean-sd", "mean":1, "sd":1, "extra":1}),
            json!({"kind":"median", "value":1, "iqr":2, "extra":1}),
            json!({"kind":"range", "lower":1, "upper":2, "extra":1}),
            json!({"kind":"grid", "values":[1,2], "extra":1}),
            json!({"kind":"level-range", "first":"T4", "last":"T5", "extra":1}),
            json!({"kind":"disc", "upper":"L1", "lower":"L2", "extra":1}),
        ] {
            assert!(serde_json::from_value::<PublishedValue>(value).is_err());
        }
    }

    #[test]
    fn unknown_source_fields_and_read_statuses_are_refused() {
        let value: Value =
            serde_json::from_str(include_str!("../evidence/sources.json")).expect("source JSON");
        for path in ["", "/sources/0", "/sources/0/citation"] {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .and_then(Value::as_object_mut)
                .expect("source object")
                .insert("extra".into(), json!(1));
            assert!(serde_json::from_value::<SourcesFile>(changed).is_err());
        }
        let mut source = value["sources"][0].clone();
        source["read"] = json!("unopened");
        assert!(serde_json::from_value::<Source>(source).is_err());
        let entry = value["sources"][0].to_string();
        let repeated = format!("{{\"sources\":[{entry},{entry}]}}");
        assert!(serde_json::from_str::<SourcesFile>(&repeated).is_err());
        for field in [
            "key", "pmid", "doi", "pmcid", "read", "citation", "n", "url", "notes",
        ] {
            let mut changed = value["sources"][0].clone();
            changed.as_object_mut().expect("source").remove(field);
            assert!(
                serde_json::from_value::<Source>(changed).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn missing_dimensions_and_spreads_are_not_null_cells() {
        assert!(serde_json::from_value::<Coordinate>(json!({"key":"chord","level":null})).is_err());
        assert!(
            serde_json::from_value::<PublishedValue>(json!({"kind":"scalar","value":null}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<PublishedValue>(json!({"kind":"mean-sd","mean":1})).is_err()
        );
        assert!(
            serde_json::from_value::<PublishedValue>(json!({"kind":"median","value":1,"iqr":null}))
                .is_err()
        );
        assert!(serde_json::from_value::<Usage>(json!("unreliable-but-use-it")).is_err());
        for table in Table::ALL {
            assert!(!table.text().contains("null"), "{table:?}");
        }
        assert!(std::ptr::eq(tables(), tables()));
    }

    #[test]
    fn observation_metadata_and_definitions_are_strict() {
        let observation = json!({
            "coordinate":{"key":"width"}, "value":{"kind":"mean-sd","mean":3,"sd":1},
            "unit":"mm", "source":"paper", "section":"table 1",
            "n":12,"range":{"lower":1,"upper":5},"printed":"3","decimals":0,
            "mark":{"kind":"doubtful","reason":"the note questions this spread"}
        });
        assert!(serde_json::from_value::<Cell>(observation.clone()).is_ok());
        for path in ["/range", "/mark"] {
            let mut changed = observation.clone();
            changed
                .pointer_mut(path)
                .and_then(Value::as_object_mut)
                .unwrap()
                .insert("extra".into(), json!(1));
            assert!(serde_json::from_value::<Cell>(changed).is_err());
        }
        for field in ["n", "range", "printed", "decimals", "mark"] {
            let mut changed = observation.clone();
            changed[field] = Value::Null;
            assert!(serde_json::from_value::<Cell>(changed).is_err(), "{field}");
        }
        let definition = json!({"key":"width","source":"paper","section":"methods","text":"outer cortex to outer cortex"});
        assert!(serde_json::from_value::<Definition>(definition.clone()).is_ok());
        let mut changed = definition;
        changed["extra"] = json!(1);
        assert!(serde_json::from_value::<Definition>(changed).is_err());
    }
}
