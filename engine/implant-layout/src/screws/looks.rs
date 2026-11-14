//! Per-anchor look triggers and whole-film cannot-tell codes.

use super::*;

/// This project's minimum holding share for settled instrumented ends.
pub const ENDS_HOLDING_SHARE: f64 = 0.6;

/// Explicit per-pedicle look inputs. Fractions are over the whole ensemble.
/// Geometry angles and ranges are in degrees. `missing_table` covers an
/// absent required width, chord, angle or curve-side cell: the frozen look
/// catalogue has only one missing-table member.
#[derive(Clone, Copy, Debug)]
pub struct LookInputs {
    pub level: Level,
    pub side: BodySide,
    pub position: CurvePosition,
    pub right_thoracic_curve: bool,
    pub missing_table: bool,
    pub no_diameter: bool,
    pub outer_narrow_share: f64,
    pub inner_narrow_share: f64,
    pub convergence: Band,
    pub max_abs_twist_deg: f64,
    pub shorter_than_grid: bool,
    pub nearest_chord_age: bool,
}

/// Trigger the CT/table look codes independently of anchor pattern.
/// Published limits are read from the evidence; alpha is this project's
/// own count threshold. Returned sources cover every published condition.
pub fn look_codes(input: LookInputs) -> Result<(Vec<LookCode>, Vec<String>), LayoutError> {
    let mut looks = Vec::new();
    let mut sources = Vec::new();
    if input.position.curve_side == CurveSide::Concave
        && input.position.apex_distance <= 1.0
        && (Level::T3..=Level::T9).contains(&input.level)
    {
        looks.push(LookCode::ConcaveApexNeedsCt);
        if let Some(c) = cell(
            Table::PlacementChecks,
            "patients-under-outer-width",
            "apex-one-under-4mm",
        ) {
            sources.push(c.source.clone());
        }
    }
    if input.no_diameter
        || input.outer_narrow_share > sizing::DEFAULT_ALPHA
        || input.inner_narrow_share > sizing::DEFAULT_ALPHA
    {
        looks.push(LookCode::MayBeTooNarrow);
    }
    if input.right_thoracic_curve && input.side == BodySide::Left {
        for group in ["right-thoracic-left-upper", "right-thoracic-left-lower"] {
            if let Some(c) = cell(Table::PlacementChecks, "aorta-warning-levels", group)
                && let PublishedValue::LevelRange { first, last } = c.value
                && (first..=last).contains(&input.level)
            {
                looks.push(LookCode::NearTheAorta);
                sources.push(c.source.clone());
            }
        }
    }
    let visibility = required_scalar(
        Table::PlacementChecks,
        "pedicle-method-visibility-limit",
        "cerny-pedicle-shadow",
    )?;
    sources.push(visibility.1);
    if input.convergence.p90 - input.convergence.p10 > visibility.0
        || input.max_abs_twist_deg > visibility.0
    {
        looks.push(LookCode::DirectionUnreadable);
    }
    if input.shorter_than_grid {
        looks.push(LookCode::ShorterThanTables);
    }
    if input.missing_table {
        looks.push(LookCode::NoTableForThisAge);
    }
    if input.nearest_chord_age {
        looks.push(LookCode::ChordFromNearestAge);
    }
    looks.sort();
    looks.dedup();
    Ok((looks, unique(sources)))
}

/// Film/context codes of 2.13. The first-pair count is supplied by the
/// caller; screw construction itself does not evaluate a level rule.
/// No history field exists in this version, so no history code is invented.
pub fn context_looks(input: &LayoutInput, first_pair_versions: Option<u32>) -> Vec<CannotTell> {
    let mut output = Vec::new();
    let mut add = |code, levels| output.push(CannotTell { code, levels });
    if first_pair_versions
        .is_some_and(|n| f64::from(n) < ENDS_HOLDING_SHARE * f64::from(input.versions))
    {
        add(CannotTellCode::EndsUnsettled, Vec::new());
    }
    if input.naming == NamingDoubt::MayBeOneOff {
        add(CannotTellCode::NamesMayBeOneOff, Vec::new());
    }
    if crate::curves::geometry::csvl(input, &input.fit).is_err() {
        add(CannotTellCode::NoSacrum, Vec::new());
    }
    if input.bend_left.is_none() || input.bend_right.is_none() {
        add(CannotTellCode::NoBendingFilms, Vec::new());
    }
    if input.bend_left.is_some() || input.bend_right.is_some() {
        add(CannotTellCode::StandingBends, Vec::new());
    }
    let twist: Vec<_> = input
        .fit
        .vertebrae
        .iter()
        .filter(|v| v.inferred.twist)
        .map(|v| v.level)
        .collect();
    if !twist.is_empty() {
        add(CannotTellCode::TwistNotMeasured, twist);
    }
    output
}
