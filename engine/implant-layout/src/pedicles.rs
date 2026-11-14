//! Population pedicle widths, never measurements of the fitted bone.
//! The shared-child split, sex combination, side-factor interpolation,
//! whole-millimetre rounding and lower truncation are project inferences.

use crate::LayoutError;
use crate::curves::SpineReading;
use crate::curves::summary::band;
use crate::evidence::{AgeBand, Cell, Coordinate, PublishedValue, Sex, Table, tables};
use crate::readings::{NamedSampler, Version};
use shapes::implant_layout::{CurveSide, PedicleEstimate, ValueStatus};
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

/// This project's default share of width variance common to a child.
pub const DEFAULT_CHILD_SHARE: f64 = 0.5;
/// This project's lower truncation of an outer-width draw, in mm.
pub const MIN_OUTER_MM: f64 = 0.5;
/// This project's half-unit rounding uncertainty for whole-mm cells.
pub const ROUNDING_HALF_MM: f64 = 0.5;

/// Position supplied by a curve reading. Distances are in vertebral levels,
/// not mm. `neutral_distance` is the apex-to-neutral distance on this side
/// of the apex, falling back to that curve end when twist is unreadable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvePosition {
    pub curve_side: CurveSide,
    pub apex_distance: f64,
    pub neutral_distance: Option<f64>,
}

impl CurvePosition {
    pub const OUTSIDE: Self = Self {
        curve_side: CurveSide::Outside,
        apex_distance: 0.0,
        neutral_distance: None,
    };
}

/// A table normal and the cells whose rounding affects it. Every cell
/// contributes its own uniform term per version, shared by its readers.
#[derive(Clone, Debug, PartialEq)]
pub struct WidthPrior {
    pub mean_mm: f64,
    pub sd_mm: f64,
    pub sources: Vec<String>,
    rounding: Vec<(String, f64)>,
}

/// A drawn outer and inner width, in mm, with all source keys.
#[derive(Clone, Debug, PartialEq)]
pub struct WidthDraw {
    pub outer_mm: f64,
    pub inner_mm: f64,
    pub sources: Vec<String>,
}

/// This project's mean of male/female columns and larger printed spread.
/// Both cells must exist; a missing column is never filled from its mate.
pub fn combine_sexes(male: &Cell, female: &Cell) -> Option<WidthPrior> {
    combine_columns(male, female)
}

// Project inference for unlabelled sex or curve side: equal-weight means
// and the larger printed spread, retaining both cells' rounding terms.
fn combine_columns(male: &Cell, female: &Cell) -> Option<WidthPrior> {
    let (m, ms) = normal(male)?;
    let (f, fs) = normal(female)?;
    let mut rounding = Vec::new();
    for cell in [male, female] {
        if cell.whole_mm {
            rounding.push((cell_name(cell), 0.5));
        }
    }
    Some(WidthPrior {
        mean_mm: 0.5 * (m + f),
        sd_mm: ms.max(fs),
        sources: unique([male.source.clone(), female.source.clone()]),
        rounding,
    })
}

/// Select the actual covered child band, without nearest-band width use.
/// Fractional ages are completed years: [5,9), [9,12), [12,15).
pub fn age_band(age: f64) -> Option<AgeBand> {
    if (5.0..9.0).contains(&age) {
        Some(AgeBand::FiveToEight)
    } else if (9.0..12.0).contains(&age) {
        Some(AgeBand::NineToEleven)
    } else if (12.0..15.0).contains(&age) {
        Some(AgeBand::TwelveToFourteen)
    } else {
        None
    }
}

/// Outer-width prior from the exact population row. Thoracic ages 12+
/// (or absent) use Gao controls; lumbar ages 12+ use Upendra's curve side.
/// Under-12 lumbar, uncovered child ages, nonstandard levels and absent
/// side columns return no value. Outside a curve, this project's prior
/// combines the two side columns. No body-size multiplier is applied.
pub fn width_prior(level: Level, age: Option<f64>, side: CurveSide) -> Option<WidthPrior> {
    if age.is_some_and(|age| !age.is_finite() || age < 0.0) {
        return None;
    }
    let young = age.is_some_and(|age| age < 12.0);
    if (Level::T1..=Level::T12).contains(&level) {
        if young {
            let age_band = Some(age_band(age?)?);
            let coordinate = |sex| Coordinate {
                key: "outer-width".into(),
                level: Some(level),
                age_band,
                sex: Some(sex),
                group: Some("zheng-children".into()),
                ..Coordinate::default()
            };
            return combine_sexes(
                tables().lookup(Table::PedicleWidths, &coordinate(Sex::Male))?,
                tables().lookup(Table::PedicleWidths, &coordinate(Sex::Female))?,
            );
        }
        prior_from_cell(tables().lookup(
            Table::PedicleWidths,
            &Coordinate {
                key: "outer-width".into(),
                level: Some(level),
                group: Some("gao-controls".into()),
                ..Coordinate::default()
            },
        )?)
    } else if (Level::L1..=Level::L5).contains(&level) && !young {
        let column = |side| {
            tables().lookup(
                Table::PedicleWidths,
                &Coordinate {
                    key: "outer-width".into(),
                    level: Some(level),
                    curve_side: Some(side),
                    group: Some("upendra-ais".into()),
                    ..Coordinate::default()
                },
            )
        };
        if side == CurveSide::Outside {
            combine_columns(column(CurveSide::Concave)?, column(CurveSide::Convex)?)
        } else {
            prior_from_cell(column(side)?)
        }
    } else {
        None
    }
}

/// This project's linear change between the apex +/-1 and neutral +/-1
/// zones. The zones are constant; between them the distance controls the
/// weight. The apical zone takes priority in an overlap; beyond it the
/// neutral value applies.
pub fn side_factor(apex_draw: f64, neutral_draw: f64, distance: f64, neutral: f64) -> f64 {
    let weight = if distance <= 1.0 {
        0.0
    } else if neutral <= 2.0 {
        1.0
    } else {
        ((distance - 1.0) / (neutral - 2.0)).clamp(0.0, 1.0)
    };
    apex_draw + weight * (neutral_draw - apex_draw)
}

/// This project's shared-child/per-pedicle normal combination. Endpoints
/// allow sensitivity checks without changing either marginal variance.
pub fn shared_normal(child: f64, pedicle: f64, child_share: f64) -> f64 {
    libm::sqrt(child_share) * child + libm::sqrt(1.0 - child_share) * pedicle
}

/// This project's latent identity for a pedicle in a curve. The sampler's
/// two independent slots are interpreted in a canonical right-convex
/// frame: concave uses the left slot, convex the right. Reflecting a curve
/// exchanges physical sides without replacing its realised child. Outside
/// a curve there is no curve-relative identity, so physical slots remain.
/// This changes only the coupling of draws, never their marginal priors.
pub fn latent_side(side: BodySide, curve_side: CurveSide) -> BodySide {
    match curve_side {
        CurveSide::Concave => BodySide::Left,
        CurveSide::Convex => BodySide::Right,
        CurveSide::Outside => side,
    }
}

/// Draw the outer width of one pedicle for one version, for downstream
/// outline rules. Inputs are level, physical side, age, curve position,
/// version and the same seeded sampler as the ensemble. Returns `None`
/// wherever the required table/position is absent. Width is in mm and
/// always from tables. It is truncated below at [`MIN_OUTER_MM`].
pub fn drawn_outer_width(
    level: Level,
    side: BodySide,
    age: Option<f64>,
    position: CurvePosition,
    version: &Version,
    sampler: &NamedSampler,
    child_share: Option<f64>,
) -> Result<Option<f64>, LayoutError> {
    Ok(
        draw_width(level, side, age, position, version, sampler, child_share)?
            .map(|draw| draw.outer_mm),
    )
}

/// Draw both widths. Normal coordinates follow [`latent_side`], so a
/// reflected curve retains the corresponding realised population draws.
/// The canonical sampler slot must be available in the supplied version.
/// The medial wall uses Chiu's physical side and region;
/// the lateral wall uses that physical side's pooled value. Neither wall
/// spread is drawn: only their central thicknesses are used.
pub fn draw_width(
    level: Level,
    side: BodySide,
    age: Option<f64>,
    position: CurvePosition,
    version: &Version,
    sampler: &NamedSampler,
    child_share: Option<f64>,
) -> Result<Option<WidthDraw>, LayoutError> {
    let share = child_share.unwrap_or(DEFAULT_CHILD_SHARE);
    if version.index >= sampler.count()
        || !version.z_child.is_finite()
        || version.pedicles.iter().any(|d| !d.z_pedicle.is_finite())
    {
        return Err(invalid("invalid-version-draw"));
    }
    if !share.is_finite()
        || !(0.0..=1.0).contains(&share)
        || !position.apex_distance.is_finite()
        || position.apex_distance < 0.0
        || position
            .neutral_distance
            .is_some_and(|d| !d.is_finite() || d < 0.0)
    {
        return Err(invalid("pedicle-position-or-child-share"));
    }
    let Some(prior) = width_prior(level, age, position.curve_side) else {
        return Ok(None);
    };
    let latent = latent_side(side, position.curve_side);
    let Some(draw) = version.pedicle(level, latent) else {
        return Ok(None);
    };
    let Some(index) = version.fit.vertebrae.iter().position(|v| v.level == level) else {
        return Ok(None);
    };
    let mut sources = prior.sources.clone();
    let factor = if level <= Level::T12 && position.curve_side == CurveSide::Concave {
        let Some((apex, a_sd, a_source)) = ratio("apex-one") else {
            return Ok(None);
        };
        sources.push(a_source);
        let z = sampler.normal(
            &format!("versions-pedicles/{index}/{latent:?}/side-factor"),
            version.index,
        )?;
        let apex_draw = apex + a_sd * z;
        if position.apex_distance <= 1.0 {
            apex_draw
        } else {
            let Some(neutral) = position.neutral_distance else {
                return Ok(None);
            };
            let Some((mean, sd, source)) = ratio("neutral-one") else {
                return Ok(None);
            };
            sources.push(source);
            side_factor(apex_draw, mean + sd * z, position.apex_distance, neutral)
        }
    } else {
        1.0
    };
    let mut rounding = 0.0;
    for (cell, weight) in &prior.rounding {
        rounding += weight
            * ROUNDING_HALF_MM
            * (2.0
                * sampler.uniform(&format!("versions-pedicles/rounding/{cell}"), version.index)?
                - 1.0);
    }
    let outer_mm = (prior.mean_mm * factor
        + prior.sd_mm * shared_normal(version.z_child, draw.z_pedicle, share)
        + rounding)
        .max(MIN_OUTER_MM);
    let Some((walls, wall_sources)) = wall_sum(level, side) else {
        return Ok(None);
    };
    sources.extend(wall_sources);
    Ok(Some(WidthDraw {
        outer_mm,
        inner_mm: (outer_mm - walls).max(0.0),
        sources: unique(sources),
    }))
}

/// Central medial plus lateral wall thickness for the physical side and
/// region. No aggregate medial thickness replaces a missing regional cell.
pub fn wall_sum(level: Level, side: BodySide) -> Option<(f64, Vec<String>)> {
    let region = match (side, level) {
        (BodySide::Right, l) if (Level::T1..=Level::T3).contains(&l) => "chiu-right-t1-t3",
        (BodySide::Right, l) if (Level::T4..=Level::T10).contains(&l) => "chiu-right-t4-t10",
        (BodySide::Right, l) if (Level::T11..=Level::L5).contains(&l) && l != Level::T13 => {
            "chiu-right-t11-l5"
        }
        (BodySide::Left, l) if (Level::T1..=Level::T3).contains(&l) => "chiu-left-t1-t3",
        (BodySide::Left, l) if (Level::T4..=Level::T7).contains(&l) => "chiu-left-t4-t7",
        (BodySide::Left, l) if (Level::T8..=Level::L5).contains(&l) && l != Level::T13 => {
            "chiu-left-t8-l5"
        }
        _ => return None,
    };
    let medial = cell(Table::InnerWalls, "medial-wall-thickness", region)?;
    let lateral = cell(
        Table::InnerWalls,
        "lateral-wall-thickness",
        match side {
            BodySide::Left => "chiu-left",
            BodySide::Right => "chiu-right",
        },
    )?;
    Some((
        central(medial)? + central(lateral)?,
        unique([medial.source.clone(), lateral.source.clone()]),
    ))
}

/// Summarise a complete ensemble. Missing width in any version leaves
/// both bands absent, rather than reporting a conditional population.
pub fn estimate(draws: &[Option<WidthDraw>], side: CurveSide) -> PedicleEstimate {
    let complete = !draws.is_empty() && draws.iter().all(Option::is_some);
    let outer: Vec<_> = draws.iter().flatten().map(|d| d.outer_mm).collect();
    let inner: Vec<_> = draws.iter().flatten().map(|d| d.inner_mm).collect();
    let threshold = cell(
        Table::DiameterRules,
        "outer-width-required-above",
        "expansion-25",
    )
    .and_then(central);
    let inner_floor = inner_channel_floor();
    PedicleEstimate {
        sources: Vec::new(),
        outer_width_mm: complete.then(|| band(&outer)).flatten(),
        inner_width_mm: complete.then(|| band(&inner)).flatten(),
        versions_too_narrow: threshold
            .map_or(0, |t| outer.iter().filter(|&&w| w < t).count() as u32),
        versions_inner_under_1_mm: inner_floor
            .map_or(0, |t| inner.iter().filter(|&&w| w < t).count() as u32),
        curve_side: side,
        status: if complete {
            ValueStatus::FromTables
        } else {
            ValueStatus::Missing
        },
    }
}

/// Curve position for a measured version. Shared endpoints choose the
/// nearer apex, with top-to-bottom order breaking ties. Neutral vertebrae
/// are the nearer-zero member of a measured twist sign crossing; the
/// nearest such level outward from the apex supplies the distance. This
/// project's fallback is the end vertebra on that side of the apex.
pub fn position_in_curve(reading: &SpineReading, level: Level, side: BodySide) -> CurvePosition {
    let Some(index) = reading.vertebrae.iter().position(|v| v.level == level) else {
        return CurvePosition::OUTSIDE;
    };
    let curve = reading
        .curves
        .iter()
        .filter(|c| c.upper_index <= index && index <= c.lower_index)
        .fold(None, |found: Option<&crate::curves::Curve>, c| {
            if found.is_none_or(|f| {
                level_distance(reading, index, c.apex_index)
                    < level_distance(reading, index, f.apex_index)
            }) {
                Some(c)
            } else {
                found
            }
        });
    let Some(curve) = curve else {
        return CurvePosition::OUTSIDE;
    };
    let apex = curve.apex_index;
    let mut neutral = None;
    for i in curve.upper_index..curve.lower_index {
        let (Ok(a), Ok(b)) = (
            reading.vertebrae[i].twist_deg,
            reading.vertebrae[i + 1].twist_deg,
        ) else {
            continue;
        };
        if a * b <= 0.0 {
            let j = if a.abs() < b.abs() { i } else { i + 1 };
            if (index <= apex && j <= apex) || (index > apex && j >= apex) {
                let distance = level_distance(reading, j, apex);
                if neutral.is_none_or(|previous| distance < previous) {
                    neutral = Some(distance);
                }
            }
        }
    }
    CurvePosition {
        curve_side: if side == curve.convex {
            CurveSide::Convex
        } else {
            CurveSide::Concave
        },
        apex_distance: level_distance(reading, index, apex),
        neutral_distance: Some(neutral.unwrap_or_else(|| {
            level_distance(
                reading,
                if index <= apex {
                    curve.upper_index
                } else {
                    curve.lower_index
                },
                apex,
            )
        })),
    }
}

// This project's distance reading counts labelled levels even when an
// intervening vertebra was not fitted. T13 counts only if it is present;
// the standard T12/L1 transition otherwise has one interval.
fn level_distance(reading: &SpineReading, a: usize, b: usize) -> f64 {
    let a = reading.vertebrae[a].level;
    let b = reading.vertebrae[b].level;
    let mut distance = a.order().abs_diff(b.order());
    if a.min(b) <= Level::T12
        && a.max(b) >= Level::L1
        && !reading.vertebrae.iter().any(|v| v.level == Level::T13)
    {
        distance -= 1;
    }
    distance as f64
}

pub(crate) fn cell(table: Table, key: &str, group: &str) -> Option<&'static Cell> {
    tables().lookup(
        table,
        &Coordinate {
            key: key.into(),
            group: Some(group.into()),
            ..Coordinate::default()
        },
    )
}

pub(crate) fn central(cell: &Cell) -> Option<f64> {
    match cell.value {
        PublishedValue::Scalar { value } | PublishedValue::PlusMinus { value, .. } => Some(value),
        PublishedValue::MeanSd { mean, .. } => Some(mean),
        _ => None,
    }
}

pub(crate) fn inner_channel_floor() -> Option<f64> {
    // The grade's upper bound, not a new literal clinical threshold.
    cell(Table::ChannelGrades, "inner-diameter-max", "grade-4").and_then(central)
}

fn normal(cell: &Cell) -> Option<(f64, f64)> {
    match cell.value {
        PublishedValue::MeanSd { mean, sd } => Some((mean, sd)),
        // This project reads Gao/Zheng +/- and the Sato width
        // ratios as SDs. This interpretation stays out of the reader.
        PublishedValue::PlusMinus { value, spread } => Some((value, spread)),
        _ => None,
    }
}
fn ratio(group: &str) -> Option<(f64, f64, String)> {
    let c = cell(Table::ConcaveRatios, "concave-convex-ratio", group)?;
    let (mean, sd) = normal(c)?;
    Some((mean, sd, c.source.clone()))
}
fn prior_from_cell(cell: &Cell) -> Option<WidthPrior> {
    let (mean_mm, sd_mm) = normal(cell)?;
    Some(WidthPrior {
        mean_mm,
        sd_mm,
        sources: vec![cell.source.clone()],
        rounding: if cell.whole_mm {
            vec![(cell_name(cell), 1.0)]
        } else {
            Vec::new()
        },
    })
}
fn cell_name(cell: &Cell) -> String {
    format!("{:?}", cell.coordinate)
}
pub(crate) fn unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values: Vec<_> = values.into_iter().collect();
    values.sort();
    values.dedup();
    values
}
fn invalid(reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: "pedicles".into(),
        reason: reason.into(),
    }
}
