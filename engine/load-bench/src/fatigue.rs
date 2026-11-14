//! A year of steps compared with a construct's published run-out level.
//!
//! Walking is the standing case scaled, on a fixed locked geometry. The
//! scalar stress at a material section swings from `locked + standing`
//! to `locked + walking_multiple * standing`. Stresses are MPa and may
//! be signed longitudinal stresses (tension positive). Supplying the
//! nonnegative surface envelopes instead gives the bench's conservative
//! envelope comparison, rather than following one material fibre.
//!
//! The run-out reference includes an assumed fixture arm
//! and equal rod sharing. The contouring factor is also extrapolated to
//! materials and diameters beyond its source; it is supplied per version.

use crate::evidence::{Evidence, TableKind, Unit, Value};
use crate::rod_statics::goodman_equivalent;
use shapes::implant_layout::RodMaterial;

/// Plain per-version inputs. Sampling belongs to the version builder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkingInput {
    pub steps_per_day: f64,
    pub walking_multiple: f64,
    pub contour_knock_down: f64,
    /// The same quantile of the selected ultimate range as the version's
    /// yield quantile; never the historical calibration's fixed ultimate.
    pub ultimate_mpa: f64,
}

/// One section's stress cycle; MPa throughout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressCycle {
    pub amplitude_mpa: f64,
    pub mean_mpa: f64,
    /// Absent when the mean reaches ultimate or the arithmetic overflows.
    pub equivalent_mpa: Option<f64>,
}

/// A published construct reference, with the supplied contouring factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FatigueReference {
    pub equivalent_mpa: f64,
    pub after_contouring_mpa: f64,
    pub run_out_cycles: f64,
}

/// A missing reference and a non-finite conversion stay distinct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FatigueComparison {
    Ratio(f64),
    NoFatigueLevel,
    NoFiniteGoodman,
}

/// The annual step count and one section's amplitude comparison. Repeated
/// sections on one rod share the same cycle count; their counts are not
/// added together. Sources retain stable first-use order.
#[derive(Clone, Debug, PartialEq)]
pub struct WalkingYear {
    pub cycles_per_year: f64,
    pub cycle: StressCycle,
    pub reference: Option<FatigueReference>,
    pub comparison: FatigueComparison,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FatigueError {
    #[error("fatigue-invalid-input: {0}")]
    InvalidInput(&'static str),
    #[error("fatigue-missing-evidence: {0}")]
    MissingEvidence(&'static str),
}

/// Exact scalar half-range and mean, followed by the shared Goodman
/// conversion. A signed standing stress still has a nonnegative amplitude.
pub fn walking_cycle(
    locked_in_mpa: f64,
    standing_case_mpa: f64,
    walking_multiple: f64,
    ultimate_mpa: f64,
) -> Result<StressCycle, FatigueError> {
    if !locked_in_mpa.is_finite()
        || !standing_case_mpa.is_finite()
        || !walking_multiple.is_finite()
        || walking_multiple < 1.0
        || !ultimate_mpa.is_finite()
        || ultimate_mpa <= 0.0
    {
        return Err(FatigueError::InvalidInput("stress-cycle"));
    }
    let signed_half_range = (walking_multiple - 1.0) * standing_case_mpa / 2.0;
    let amplitude_mpa = signed_half_range.abs();
    let mean_mpa = locked_in_mpa + standing_case_mpa + signed_half_range;
    if !amplitude_mpa.is_finite() || !mean_mpa.is_finite() {
        return Err(FatigueError::InvalidInput("stress-overflow"));
    }
    Ok(StressCycle {
        amplitude_mpa,
        mean_mpa,
        equivalent_mpa: goodman_equivalent(amplitude_mpa, mean_mpa, ultimate_mpa),
    })
}

/// Count one cycle per step and compare its stress swing with the selected
/// construct level times the supplied knock-down. The count does not alter
/// that amplitude ratio: no S–N slope is supplied by the evidence.
pub fn year_of_walking(
    evidence: &Evidence,
    material: RodMaterial,
    input: WalkingInput,
    locked_in_mpa: f64,
    standing_case_mpa: f64,
) -> Result<WalkingYear, FatigueError> {
    if !input.steps_per_day.is_finite()
        || input.steps_per_day < 0.0
        || !input.contour_knock_down.is_finite()
        || input.contour_knock_down <= 0.0
        || input.contour_knock_down > 1.0
    {
        return Err(FatigueError::InvalidInput("walking-input"));
    }
    let cycle = walking_cycle(
        locked_in_mpa,
        standing_case_mpa,
        input.walking_multiple,
        input.ultimate_mpa,
    )?;
    let days = cell(
        evidence,
        TableKind::DailyLoads,
        "year",
        "days",
        Unit::DaysPerYear,
    )?;
    let cycles_per_year = input.steps_per_day * days.value;
    if !cycles_per_year.is_finite() || days.value <= 0.0 {
        return Err(FatigueError::InvalidInput("cycles-overflow"));
    }
    let mut sources = vec![days.source.clone()];
    for (table, key) in [
        (TableKind::DailyLoads, "walking"),
        (TableKind::DailyLoads, "child-steps"),
        (TableKind::Fatigue, "contour-knock-down"),
    ] {
        if let Some(row) = evidence.row(table, key) {
            add_sources(&mut sources, row.values.iter());
        }
    }
    if let Some(pair) = evidence.bench_strengths(material) {
        add_sources(&mut sources, [pair.ultimate_lower, pair.ultimate_upper]);
    }
    let key = match material {
        RodMaterial::TitaniumAlloy => "titanium-alloy",
        RodMaterial::CobaltChrome => "cobalt-chrome",
        RodMaterial::StainlessSteel => {
            return Ok(WalkingYear {
                cycles_per_year,
                cycle,
                reference: None,
                comparison: FatigueComparison::NoFatigueLevel,
                sources,
            });
        }
    };
    let level = cell(
        evidence,
        TableKind::Fatigue,
        key,
        "goodman-equivalent",
        Unit::Mpa,
    )?;
    let run_out = cell(
        evidence,
        TableKind::Fatigue,
        key,
        "run-out-cycles",
        Unit::Cycles,
    )?;
    let after_contouring_mpa = level.value * input.contour_knock_down;
    if level.value <= 0.0 || after_contouring_mpa <= 0.0 || run_out.value <= 0.0 {
        return Err(FatigueError::InvalidInput("fatigue-reference"));
    }
    if let Some(row) = evidence.row(TableKind::Fatigue, key) {
        add_sources(&mut sources, row.values.iter());
    }
    let comparison = match cycle.equivalent_mpa {
        Some(value) if (value / after_contouring_mpa).is_finite() => {
            FatigueComparison::Ratio(value / after_contouring_mpa)
        }
        _ => FatigueComparison::NoFiniteGoodman,
    };
    Ok(WalkingYear {
        cycles_per_year,
        cycle,
        reference: Some(FatigueReference {
            equivalent_mpa: level.value,
            after_contouring_mpa,
            run_out_cycles: run_out.value,
        }),
        comparison,
        sources,
    })
}

fn cell<'a>(
    evidence: &'a Evidence,
    table: TableKind,
    key: &'static str,
    term: &str,
    unit: Unit,
) -> Result<&'a Value, FatigueError> {
    evidence
        .value(table, key, term)
        .filter(|value| value.unit == unit && value.value.is_finite())
        .ok_or(FatigueError::MissingEvidence(key))
}

fn add_sources<'a>(sources: &mut Vec<String>, cells: impl IntoIterator<Item = &'a Value>) {
    for cell in cells {
        if !sources.contains(&cell.source) {
            sources.push(cell.source.clone());
        }
    }
}
