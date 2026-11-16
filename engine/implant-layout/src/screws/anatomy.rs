//! Chords and medial angles from the selected published populations.
//! Exact usable cells win; no regional summary fills an individual cell.

use crate::evidence::{AgeBand, Cell, Coordinate, PublishedValue, Sex, Table, tables};
use shapes::implant_layout::{AngleFromTable, CurveSide};
use shapes::levels::Level;

/// A population centre and its reported spread, in the table's units.
/// Chords use mm and angles degrees. Zheng's literal ± is read as an SD
/// under this project's explicit inference; other unlabelled spreads are
/// not silently given that meaning.
#[derive(Clone, Debug, PartialEq)]
pub struct Distribution {
    pub mean: f64,
    pub sd: Option<f64>,
    pub sources: Vec<String>,
    pub nearest_age: bool,
}

/// Read the chord for this age and curve side. The complete younger
/// thoracic table now covers the formerly absent 9–11 band, so its earlier
/// nearest-band rule is dormant. Children outside the printed age bands,
/// lumbar children and nonstandard levels have no chord. Outside a curve,
/// this project's inference averages the two published side columns.
pub fn chord(level: Level, side: CurveSide, age_years: Option<f64>) -> Option<Distribution> {
    if let Some(age) = age_years {
        if !age.is_finite() || age < 0.0 {
            return None;
        }
        if age < 12.0 {
            return zheng(Table::Chords, "pedicle-length", level, age_band(age)?);
        }
    }
    upendra(Table::Chords, "chord", level, side)
}

/// The reported spread printed with a chord mean that `chord` gives for
/// this level and curve side at some age: Upendra's column, or one of
/// Zheng's covered age bands. An anchor keeps the chord mean but not the
/// age, so the mean itself picks the column; none matches, or the column
/// prints no spread, gives `None`.
pub fn chord_spread(level: Level, side: CurveSide, mean: f64) -> Option<f64> {
    core::iter::once(chord(level, side, None))
        .chain(
            [AgeBand::FiveToEight, AgeBand::NineToEleven]
                .into_iter()
                .map(|band| zheng(Table::Chords, "pedicle-length", level, band)),
        )
        .flatten()
        .find(|c| c.mean == mean)
        .and_then(|c| c.sd)
}

/// Upendra's medial-angle population at this level and curve side.
/// This project's interpolation uses the neighbouring usable levels only
/// when the exact level is absent or excluded; printed values are retained.
pub fn medial_angle(level: Level, side: CurveSide) -> Option<Distribution> {
    upendra(Table::MedialAngles, "transverse-angle", level, side)
}

/// Other published techniques beside the chosen angle. Zheng uses only
/// the requested age band; an unspecified age shows its 12–14 comparison
/// cohort. No Zheng number is borrowed for a specified age outside 5–14.
/// The uniform techniques are explicitly regional and have no age column.
pub fn medial_angles_beside(level: Level, age_years: Option<f64>) -> Vec<AngleFromTable> {
    let mut result = Vec::new();
    let band = match age_years {
        Some(age) => age_band(age),
        None => Some(AgeBand::TwelveToFourteen),
    };
    if let Some(value) =
        band.and_then(|band| zheng(Table::MedialAngles, "transverse-angle", level, band))
    {
        result.push(AngleFromTable {
            angle_deg: value.mean,
            source: value.sources[0].clone(),
        });
    }
    let group = if (Level::T1..=Level::T2).contains(&level) {
        Some("t1-t2")
    } else if (Level::T3..=Level::T12).contains(&level) {
        Some("t3-t12")
    } else {
        None
    };
    if let Some(group) = group {
        for name in [group.to_owned(), format!("peng-{group}")] {
            if let Some(cell) = tables().lookup(
                Table::PlacementChecks,
                &Coordinate {
                    key: "uniform-medial-angle".into(),
                    group: Some(name),
                    ..Coordinate::default()
                },
            ) && let PublishedValue::Scalar { value } = cell.value
            {
                result.push(AngleFromTable {
                    angle_deg: value,
                    source: cell.source.clone(),
                });
            }
        }
    }
    result
}

/// This project's age-band reading treats whole-year labels as continuous
/// bands [5,9), [9,12), [12,15). It never extrapolates a width or angle.
pub fn age_band(age: f64) -> Option<AgeBand> {
    if !age.is_finite() {
        None
    } else if (5.0..9.0).contains(&age) {
        Some(AgeBand::FiveToEight)
    } else if (9.0..12.0).contains(&age) {
        Some(AgeBand::NineToEleven)
    } else if (12.0..15.0).contains(&age) {
        Some(AgeBand::TwelveToFourteen)
    } else {
        None
    }
}

fn upendra(table: Table, key: &str, level: Level, side: CurveSide) -> Option<Distribution> {
    if side == CurveSide::Outside {
        let concave = upendra(table, key, level, CurveSide::Concave)?;
        let convex = upendra(table, key, level, CurveSide::Convex)?;
        let mut value = interpolate_levels(&concave, &convex, 0.5)?;
        value.sd = concave.sd.zip(convex.sd).map(|(a, b)| a.max(b));
        return Some(value);
    }
    let target = Level::MEASURED.iter().position(|&item| item == level)?;
    let coordinate = |level| Coordinate {
        key: key.into(),
        level: Some(level),
        curve_side: Some(side),
        group: Some("upendra-ais".into()),
        ..Coordinate::default()
    };
    if let Some(cell) = tables().lookup(table, &coordinate(level)) {
        return distribution(cell, false);
    }
    let lower = Level::MEASURED[..target]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, &level)| {
            tables()
                .lookup(table, &coordinate(level))
                .map(|cell| (i, cell))
        })?;
    let upper = Level::MEASURED[target + 1..]
        .iter()
        .enumerate()
        .find_map(|(i, &level)| {
            tables()
                .lookup(table, &coordinate(level))
                .map(|cell| (target + 1 + i, cell))
        })?;
    interpolate_levels(
        &distribution(lower.1, false)?,
        &distribution(upper.1, false)?,
        (target - lower.0) as f64 / (upper.0 - lower.0) as f64,
    )
}

/// This project's own inference: linearly interpolate means and reported
/// SDs by standard-level distance. No SD is invented if either is absent.
pub fn interpolate_levels(
    lower: &Distribution,
    upper: &Distribution,
    fraction: f64,
) -> Option<Distribution> {
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    let mut sources = lower.sources.clone();
    sources.extend(upper.sources.iter().cloned());
    sources.sort();
    sources.dedup();
    Some(Distribution {
        mean: lower.mean + fraction * (upper.mean - lower.mean),
        sd: lower.sd.zip(upper.sd).map(|(a, b)| a + fraction * (b - a)),
        sources,
        nearest_age: lower.nearest_age || upper.nearest_age,
    })
}

fn zheng(table: Table, key: &str, level: Level, band: AgeBand) -> Option<Distribution> {
    let column = |sex| {
        tables().lookup(
            table,
            &Coordinate {
                key: key.into(),
                level: Some(level),
                sex: Some(sex),
                age_band: Some(band),
                group: Some("zheng-children".into()),
                ..Coordinate::default()
            },
        )
    };
    mean_of_sexes(
        &distribution(column(Sex::Male)?, true)?,
        &distribution(column(Sex::Female)?, true)?,
    )
}

/// This project's own inference: use the arithmetic mean of the printed
/// sex columns and the larger reported SD, without pooling sample counts
/// that the source does not give. Both columns must exist.
pub fn mean_of_sexes(male: &Distribution, female: &Distribution) -> Option<Distribution> {
    let mut value = interpolate_levels(male, female, 0.5)?;
    value.mean = (male.mean + female.mean) * 0.5;
    value.sd = male.sd.zip(female.sd).map(|(a, b)| a.max(b));
    Some(value)
}

fn distribution(cell: &Cell, zheng_sd: bool) -> Option<Distribution> {
    let (mean, sd) = match cell.value {
        PublishedValue::MeanSd { mean, sd } => (mean, Some(sd)),
        PublishedValue::PlusMinus { value, spread } if zheng_sd => (value, Some(spread)),
        PublishedValue::Scalar { value } => (value, None),
        _ => return None,
    };
    Some(Distribution {
        mean,
        sd,
        sources: vec![cell.source.clone()],
        nearest_age: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_published_levels_are_not_replaced_by_a_smooth_line() {
        let coordinate = Coordinate {
            key: "chord".into(),
            level: Some(Level::T10),
            curve_side: Some(CurveSide::Convex),
            group: Some("upendra-ais".into()),
            ..Coordinate::default()
        };
        let cell = tables().lookup(Table::Chords, &coordinate).unwrap();
        let expected = distribution(cell, false).unwrap();
        assert_eq!(
            chord(Level::T10, CurveSide::Convex, Some(15.0)),
            Some(expected)
        );
    }

    #[test]
    fn the_excluded_l4_angle_uses_its_two_usable_neighbours() {
        let lower = medial_angle(Level::L3, CurveSide::Convex).unwrap();
        let upper = medial_angle(Level::L5, CurveSide::Convex).unwrap();
        let actual = medial_angle(Level::L4, CurveSide::Convex).unwrap();
        assert_eq!(actual.mean, (lower.mean + upper.mean) * 0.5);
        assert_eq!(
            actual.sd,
            lower.sd.zip(upper.sd).map(|(a, b)| (a + b) * 0.5)
        );
        assert_eq!(actual.sources, lower.sources);
        assert!(medial_angle(Level::T13, CurveSide::Convex).is_none());
        let concave = medial_angle(Level::T7, CurveSide::Concave).unwrap();
        let convex = medial_angle(Level::T7, CurveSide::Convex).unwrap();
        let outside = medial_angle(Level::T7, CurveSide::Outside).unwrap();
        assert_eq!(outside.mean, (concave.mean + convex.mean) * 0.5);
        assert_eq!(outside.sd, concave.sd.zip(convex.sd).map(|(a, b)| a.max(b)));
    }

    #[test]
    fn childrens_chords_keep_the_mean_and_larger_spread_of_both_columns() {
        let lookup = |sex| {
            let coordinate = Coordinate {
                key: "pedicle-length".into(),
                level: Some(Level::T8),
                age_band: Some(AgeBand::NineToEleven),
                sex: Some(sex),
                group: Some("zheng-children".into()),
                ..Coordinate::default()
            };
            distribution(tables().lookup(Table::Chords, &coordinate).unwrap(), true).unwrap()
        };
        let male = lookup(Sex::Male);
        let female = lookup(Sex::Female);
        let actual = chord(Level::T8, CurveSide::Outside, Some(10.0)).unwrap();
        assert_eq!(actual.mean, (male.mean + female.mean) * 0.5);
        assert_eq!(actual.sd, male.sd.zip(female.sd).map(|(a, b)| a.max(b)));
        assert!(!actual.nearest_age);
        assert!(chord(Level::L1, CurveSide::Concave, Some(10.0)).is_none());
    }

    #[test]
    fn a_chord_mean_finds_the_spread_printed_beside_it() {
        let adult = chord(Level::T10, CurveSide::Convex, None).unwrap();
        assert_eq!(
            chord_spread(Level::T10, CurveSide::Convex, adult.mean),
            adult.sd
        );
        let young = chord(Level::T8, CurveSide::Outside, Some(10.0)).unwrap();
        assert_eq!(
            chord_spread(Level::T8, CurveSide::Outside, young.mean),
            young.sd
        );
        assert_eq!(chord_spread(Level::T10, CurveSide::Convex, -1.0), None);
    }

    #[test]
    fn an_uncovered_age_borrows_neither_a_chord_nor_an_angle() {
        let youngest = chord(Level::T4, CurveSide::Convex, Some(6.0)).unwrap();
        assert!(!youngest.nearest_age);
        assert!(chord(Level::T4, CurveSide::Convex, Some(4.0)).is_none());
        assert_eq!(age_band(4.0), None);
        let beside = medial_angles_beside(Level::T4, Some(4.0));
        assert!(beside.iter().all(|angle| angle.source != "zheng-2009"));
        assert!(
            medial_angles_beside(Level::T12, Some(13.0))
                .iter()
                .any(|angle| angle.angle_deg < 0.0)
        );
    }
}
