//! Counts and percentile ranges over already measured versions. Curves
//! are matched by region and convex side. If small alternating extrema
//! put several in one group, only its largest curve votes in that version.
//! All curves remain available in the per-version readings.
//!
//! Percentiles use this project's own linear interpolation at p*(n-1).
//! Missing values are not converted to zero and unreadable tests never
//! count as met. Mixed readable and unreadable structural tests produce a
//! share over the full version count; the unread reasons remain in
//! `cannot_tell` because the frozen `Readable` shape has no mixed variant.

use super::{Reading, SpineReading};
use shapes::implant_layout::{CannotTell, CannotTellCode, CurveTypeCount, LayoutCurve, Readable};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
#[cfg(test)]
use shapes::spine_readings::Band;
use shapes::spine_readings::{BodySide, LevelCount};
use std::collections::{BTreeMap, BTreeSet};

/// Contract-ready measurements, counts and unread reasons. `sources`
/// resolve through the placement evidence reader.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementSummary {
    pub curves: Vec<LayoutCurve>,
    pub classification: Vec<CurveTypeCount>,
    pub cannot_tell: Vec<CannotTell>,
    pub sources: Vec<String>,
}

/// Aggregate versions in their given order. A curve contributes at most
/// once to each (region, convexity) group in each version.
pub fn summarise(readings: &[SpineReading]) -> MeasurementSummary {
    let mut curves = Vec::new();
    let mut classification: Vec<CurveTypeCount> = Vec::new();
    let mut unread: BTreeMap<CannotTellCode, BTreeSet<Level>> = BTreeMap::new();
    let mut sources = BTreeSet::new();
    for reading in readings {
        sources.extend(reading.sources.iter().cloned());
        let value = &reading.classification;
        let parts = (
            value.curve_type.ok(),
            value.lumbar_modifier.ok(),
            value.sagittal_modifier.ok(),
        );
        if let Some(count) = classification.iter_mut().find(|count| {
            (
                count.curve_type,
                count.lumbar_modifier,
                count.sagittal_modifier,
            ) == parts
        }) {
            count.versions += 1;
        } else {
            classification.push(CurveTypeCount {
                curve_type: parts.0,
                lumbar_modifier: parts.1,
                sagittal_modifier: parts.2,
                versions: 1,
            });
        }
        for result in [
            reading.csvl_y_mm,
            reading.kyphosis_t2_t5_deg,
            reading.kyphosis_t5_t12_deg,
            reading.kyphosis_t10_l2_deg,
            reading.l4_tilt_deg,
        ] {
            record(&mut unread, result, &[]);
        }
        record(&mut unread, value.curve_type, &[]);
        record(&mut unread, value.lumbar_modifier, &[]);
        record(&mut unread, value.sagittal_modifier, &[]);
        for vertebra in &reading.vertebrae {
            record(&mut unread, vertebra.twist_deg, &[vertebra.level]);
        }
        for curve in &reading.curves {
            let levels: Vec<Level> = [curve.upper_index, curve.lower_index]
                .into_iter()
                .filter_map(|index| reading.vertebrae.get(index).map(|vertebra| vertebra.level))
                .collect();
            record(&mut unread, curve.bending_cobb_deg, &levels);
            record(&mut unread, curve.structural, &levels);
            if curve.bending_cobb_deg.is_ok() {
                unread
                    .entry(CannotTellCode::StandingBends)
                    .or_default()
                    .extend(levels);
            }
        }
    }
    for region in [
        CurveRegion::ProximalThoracic,
        CurveRegion::MainThoracic,
        CurveRegion::ThoracolumbarLumbar,
    ] {
        for convex in [BodySide::Left, BodySide::Right] {
            let mut upper = BTreeMap::new();
            let mut lower = BTreeMap::new();
            let mut apex = BTreeMap::new();
            let mut cobb = Vec::new();
            let mut bending = Vec::new();
            let mut major = 0;
            let mut met = 0;
            let mut readable = false;
            let mut reason = None;
            for reading in readings {
                let curve = reading
                    .curves
                    .iter()
                    .enumerate()
                    .filter(|(_, curve)| curve.region == Some(region) && curve.convex == convex)
                    .max_by(|(_, a), (_, b)| a.cobb_deg.total_cmp(&b.cobb_deg));
                let Some((index, curve)) = curve else {
                    continue;
                };
                cobb.push(curve.cobb_deg);
                if let Ok(value) = curve.bending_cobb_deg {
                    bending.push(value);
                }
                major += u32::from(reading.major == Some(index));
                match curve.structural {
                    Ok(holds) => {
                        readable = true;
                        met += u32::from(holds);
                    }
                    Err(code) => {
                        reason.get_or_insert(code);
                    }
                }
                for (counts, index) in [
                    (&mut upper, curve.upper_index),
                    (&mut lower, curve.lower_index),
                    (&mut apex, curve.apex_index),
                ] {
                    if let Some(vertebra) = reading.vertebrae.get(index) {
                        *counts.entry(vertebra.level).or_insert(0) += 1;
                    }
                }
            }
            if let Some(cobb_deg) = band(&cobb) {
                curves.push(LayoutCurve {
                    region,
                    convex,
                    versions_found: cobb.len() as u32,
                    versions_major: major,
                    upper_end: level_counts(upper),
                    lower_end: level_counts(lower),
                    apex: level_counts(apex),
                    cobb_deg,
                    bending_cobb_deg: band(&bending),
                    structural: if readable {
                        Readable::Share { versions: met }
                    } else {
                        Readable::NotReadable {
                            reason: reason.unwrap_or(CannotTellCode::NoCurve),
                        }
                    },
                });
            }
        }
    }
    MeasurementSummary {
        curves,
        classification,
        cannot_tell: unread
            .into_iter()
            .map(|(code, levels)| CannotTell {
                code,
                levels: levels.into_iter().collect(),
            })
            .collect(),
        sources: sources.into_iter().collect(),
    }
}

fn record<T>(
    unread: &mut BTreeMap<CannotTellCode, BTreeSet<Level>>,
    reading: Reading<T>,
    levels: &[Level],
) {
    if let Err(code) = reading {
        unread.entry(code).or_default().extend(levels);
    }
}

fn level_counts(counts: BTreeMap<Level, u32>) -> Vec<LevelCount> {
    counts
        .into_iter()
        .map(|(level, versions)| LevelCount { level, versions })
        .collect()
}

pub use spine_readings::bands::band;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::{Classification, Curve, VertebraReading};
    use shapes::implant_layout::{CurveType, SagittalModifier};

    fn reading(cobb: f64, structural: Reading<bool>) -> SpineReading {
        SpineReading {
            vertebrae: [Level::T6, Level::T8, Level::T10]
                .into_iter()
                .map(|level| VertebraReading {
                    level,
                    centre_mm: [0.0; 3],
                    upper_tilt_deg: 0.0,
                    lower_tilt_deg: 0.0,
                    twist_deg: Err(CannotTellCode::TwistNotMeasured),
                    disc_below_deg: Ok(0.0),
                    centre_line_offset_mm: Err(CannotTellCode::NoSacrum),
                })
                .collect(),
            curves: vec![Curve {
                upper_index: 0,
                lower_index: 2,
                apex_index: 1,
                region: Some(CurveRegion::MainThoracic),
                convex: BodySide::Right,
                cobb_deg: cobb,
                signed_cobb_deg: cobb,
                apical_deviation_mm: -10.0,
                bending_cobb_deg: Err(CannotTellCode::NoBendingFilms),
                straightening_bend: Err(CannotTellCode::NoBendingFilms),
                bending_disc_below_deg: Err(CannotTellCode::NoBendingFilms),
                structural,
            }],
            major: Some(0),
            csvl_y_mm: Err(CannotTellCode::NoSacrum),
            kyphosis_t2_t5_deg: Err(CannotTellCode::NoSideFilm),
            kyphosis_t5_t12_deg: Err(CannotTellCode::NoSideFilm),
            kyphosis_t10_l2_deg: Err(CannotTellCode::NoSideFilm),
            l4_tilt_deg: Err(CannotTellCode::NoCurve),
            classification: Classification {
                curve_type: Ok(CurveType::MainThoracic),
                lumbar_modifier: Err(CannotTellCode::NoSacrum),
                sagittal_modifier: Err(CannotTellCode::NoSideFilm),
            },
            minor_curve_minimum_deg: None,
            sources: Vec::new(),
        }
    }

    #[test]
    fn percentile_interpolation_has_hand_worked_answers() {
        assert_eq!(
            band(&[30.0, 10.0, 20.0]),
            Some(Band {
                p10: 12.0,
                p50: 20.0,
                p90: 28.0
            })
        );
        assert_eq!(
            band(&[7.0]),
            Some(Band {
                p10: 7.0,
                p50: 7.0,
                p90: 7.0
            })
        );
        assert_eq!(band(&[]), None);
        assert_eq!(band(&[f64::NAN]), None);
    }

    #[test]
    fn unread_versions_never_increase_a_structural_count() {
        let mut third = reading(30.0, Err(CannotTellCode::NoBendingFilms));
        third.classification.sagittal_modifier = Ok(SagittalModifier::Normal);
        let summary = summarise(&[reading(10.0, Ok(true)), reading(20.0, Ok(false)), third]);
        assert_eq!(summary.curves.len(), 1);
        let curve = &summary.curves[0];
        assert_eq!(curve.versions_found, 3);
        assert_eq!(curve.versions_major, 3);
        assert_eq!(curve.structural, Readable::Share { versions: 1 });
        assert_eq!(
            curve.upper_end,
            vec![LevelCount {
                level: Level::T6,
                versions: 3
            }]
        );
        assert_eq!(curve.bending_cobb_deg, None);
        assert_eq!(
            summary
                .classification
                .iter()
                .map(|value| value.versions)
                .sum::<u32>(),
            3
        );
        assert!(
            summary
                .cannot_tell
                .iter()
                .any(|value| value.code == CannotTellCode::NoBendingFilms)
        );
        assert!(
            summary
                .cannot_tell
                .iter()
                .any(|value| value.code == CannotTellCode::NoSideFilm)
        );
        assert_eq!(
            summary
                .cannot_tell
                .iter()
                .find(|value| value.code == CannotTellCode::TwistNotMeasured)
                .unwrap()
                .levels,
            vec![Level::T6, Level::T8, Level::T10]
        );
    }
}
