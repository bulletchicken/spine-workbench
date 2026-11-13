//! Contract output: fixed nominal ends, paired version angles and counts.

use crate::ReadingsError;
use crate::bands::band;
use crate::curves::geometry::{lower_coronal_tilt_deg, upper_coronal_tilt_deg};
use crate::curves::{self, ReadReason};
use crate::readings::versions;
use crate::sagittal::{level_index, plates, sacrum_tilt};
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::{
    BetweenReadings, CurveReading, FitTier, LevelCount, NotMeasured, PairReading, Reading,
    ReadingsInput, SagittalReadings, SpineReadings, VertebraReading,
};
use std::collections::BTreeMap;

struct Frame<'a> {
    fit: &'a SpineFit,
    front: Vec<[f64; 2]>,
    side: Vec<[f64; 2]>,
}

impl<'a> Frame<'a> {
    fn new(fit: &'a SpineFit) -> Result<Self, ReadingsError> {
        let front: Vec<_> = fit
            .vertebrae
            .iter()
            .map(|v| [upper_coronal_tilt_deg(v), lower_coronal_tilt_deg(v)])
            .collect();
        let side = plates(fit);
        if front.iter().chain(&side).flatten().any(|v| !v.is_finite())
            || fit.vertebrae.iter().any(|v| {
                [v.angles.forward_lean_rad, v.angles.twist_rad]
                    .iter()
                    .any(|a| !a.to_degrees().is_finite())
                    || v.corner_tilt_rad
                        .is_some_and(|a| !a.to_degrees().is_finite())
            })
        {
            return Err(ReadingsError::InvalidValue {
                field: "fit".into(),
                reason: "non-finite-angle".into(),
            });
        }
        Ok(Self { fit, front, side })
    }

    fn front_pair(&self, upper: usize, lower: usize) -> f64 {
        self.front[lower][1] - self.front[upper][0]
    }

    fn side_pair(&self, upper: usize, lower: usize) -> f64 {
        self.side[upper][0] - self.side[lower][1]
    }
}

fn missing(reason: NotMeasured, typical_deg: Option<f64>) -> Reading {
    Reading::NotMeasured {
        reason,
        typical_deg,
    }
}

fn measured(value_deg: f64, values: Vec<f64>) -> Reading {
    Reading::Measured {
        value_deg,
        band: band(&values).expect("a finite angle in every version"),
    }
}

fn side_missing_reason(fit: &SpineFit) -> NotMeasured {
    if fit.side_film.is_some() {
        NotMeasured::NotOnSideFilm
    } else {
        NotMeasured::NoSideFilm
    }
}

fn side_is_measured(fit: &SpineFit, indices: &[usize]) -> bool {
    fit.side_film.is_some() && indices.iter().all(|&i| !fit.vertebrae[i].inferred.depth)
}

fn twist(nominal: &Frame<'_>, samples: &[Frame<'_>], index: usize) -> Reading {
    let v = &nominal.fit.vertebrae[index];
    let value = v.angles.twist_rad.to_degrees();
    if v.inferred.twist {
        missing(NotMeasured::TwistNotMeasured, Some(value))
    } else {
        measured(
            value,
            samples
                .iter()
                .map(|s| s.fit.vertebrae[index].angles.twist_rad.to_degrees())
                .collect(),
        )
    }
}

fn side_pair(nominal: &Frame<'_>, samples: &[Frame<'_>], a: usize, b: usize, sign: f64) -> Reading {
    let value = sign * nominal.side_pair(a, b);
    if !side_is_measured(nominal.fit, &[a, b]) {
        missing(side_missing_reason(nominal.fit), Some(value))
    } else {
        measured(
            value,
            samples.iter().map(|s| sign * s.side_pair(a, b)).collect(),
        )
    }
}

fn regional(nominal: &Frame<'_>, samples: &[Frame<'_>], a: Level, b: Level, sign: f64) -> Reading {
    match (level_index(nominal.fit, a), level_index(nominal.fit, b)) {
        (Some(a), Some(b)) => side_pair(nominal, samples, a, b, sign),
        _ if nominal.fit.side_film.is_none() => missing(NotMeasured::NoSideFilm, None),
        _ => missing(NotMeasured::NotFitted, None),
    }
}

fn reason(reason: ReadReason) -> NotMeasured {
    match reason {
        ReadReason::NoSacrum => NotMeasured::NoSacrum,
        ReadReason::NoSideFilm => NotMeasured::NoSideFilm,
        ReadReason::NoBendingFilms => NotMeasured::NoBendingFilms,
        ReadReason::TwistNotMeasured => NotMeasured::TwistNotMeasured,
        ReadReason::NotFitted => NotMeasured::NotFitted,
    }
}

/// Measure the spine as fitted and the same quantities over 1 to 256
/// versions. Curves are found on the nominal spine; their Cobb bands use
/// those fixed end vertebrae even when a version chooses different ends.
pub fn measure_spine(input: &ReadingsInput) -> Result<SpineReadings, ReadingsError> {
    if !(1..=256).contains(&input.versions) {
        return Err(ReadingsError::VersionsOutOfRange {
            versions: input.versions,
        });
    }
    let ensemble = versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )?;
    let nominal = Frame::new(&input.fit)?;
    let samples = ensemble
        .versions
        .iter()
        .map(|v| Frame::new(&v.fit))
        .collect::<Result<Vec<_>, _>>()?;
    let geometry = curves::measure(input, &input.fit)?;
    let sample_geometry = ensemble
        .versions
        .iter()
        .map(|v| curves::measure(input, &v.fit))
        .collect::<Result<Vec<_>, _>>()?;
    let vertebrae = input
        .fit
        .vertebrae
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let coronal = |fit: &SpineFit| {
                let v = &fit.vertebrae[i];
                v.corner_tilt_rad
                    .map_or_else(|| upper_coronal_tilt_deg(v), f64::to_degrees)
            };
            let forward = v.angles.forward_lean_rad.to_degrees();
            VertebraReading {
                level: v.level,
                coronal_tilt: measured(
                    coronal(&input.fit),
                    samples.iter().map(|s| coronal(s.fit)).collect(),
                ),
                forward_lean: if side_is_measured(&input.fit, &[i]) {
                    measured(
                        forward,
                        samples
                            .iter()
                            .map(|s| s.fit.vertebrae[i].angles.forward_lean_rad.to_degrees())
                            .collect(),
                    )
                } else {
                    missing(side_missing_reason(&input.fit), Some(forward))
                },
                twist: twist(&nominal, &samples, i),
            }
        })
        .collect();
    let mut front = Vec::new();
    let mut side = Vec::new();
    for a in 0..input.fit.vertebrae.len() {
        for b in a + 1..input.fit.vertebrae.len() {
            let pair = |angle| PairReading {
                upper: input.fit.vertebrae[a].level,
                lower: input.fit.vertebrae[b].level,
                angle,
            };
            front.push(pair(measured(
                nominal.front_pair(a, b),
                samples.iter().map(|s| s.front_pair(a, b)).collect(),
            )));
            side.push(pair(side_pair(&nominal, &samples, a, b, 1.0)));
        }
    }
    let lordosis_l1_s1 = if input.fit.side_film.is_none() {
        // The fit has no sacral body or typical sacrum top to supply a value.
        missing(NotMeasured::NoSideFilm, None)
    } else if let Some(index) = level_index(&input.fit, Level::L1) {
        let sacrum = sacrum_tilt(input);
        if !side_is_measured(&input.fit, &[index]) {
            missing(
                NotMeasured::NotOnSideFilm,
                sacrum.map(|sacrum| sacrum - nominal.side[index][0]),
            )
        } else if let Some(sacrum) = sacrum {
            let value = sacrum - nominal.side[index][0];
            measured(
                value,
                samples.iter().map(|s| sacrum - s.side[index][0]).collect(),
            )
        } else {
            missing(NotMeasured::NoSacrum, None)
        }
    } else {
        missing(NotMeasured::NotFitted, None)
    };
    let sagittal = SagittalReadings {
        kyphosis_t2_t5: regional(&nominal, &samples, Level::T2, Level::T5, 1.0),
        kyphosis_t5_t12: regional(&nominal, &samples, Level::T5, Level::T12, 1.0),
        kyphosis_t10_l2: regional(&nominal, &samples, Level::T10, Level::L2, 1.0),
        lordosis_l1_l5: regional(&nominal, &samples, Level::L1, Level::L5, -1.0),
        lordosis_l1_s1,
    };
    let floor_deg = crate::evidence::curve_floor_deg();
    let mut nominal_curves: Vec<_> = geometry
        .curves
        .iter()
        .filter(|c| {
            c.region.is_some()
                && nominal.front_pair(c.upper_index, c.lower_index).abs() >= floor_deg
        })
        .collect();
    nominal_curves.sort_by(|a, b| b.cobb_deg.total_cmp(&a.cobb_deg));
    let curves = nominal_curves
        .into_iter()
        .map(|curve| {
            let region = curve.region.expect("regional curves only");
            let (a, b, apex) = (curve.upper_index, curve.lower_index, curve.apex_index);
            let mut upper = BTreeMap::new();
            let mut lower = BTreeMap::new();
            let mut apices = BTreeMap::new();
            let versions_found = samples
                .iter()
                .filter(|sample| sample.front_pair(a, b).abs() >= floor_deg)
                .count() as u32;
            for sample in &sample_geometry {
                if let Some(found) = sample
                    .curves
                    .iter()
                    .filter(|c| c.region == Some(region) && c.convex == curve.convex)
                    .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg))
                {
                    for (counts, index) in [
                        (&mut upper, found.upper_index),
                        (&mut lower, found.lower_index),
                        (&mut apices, found.apex_index),
                    ] {
                        *counts.entry(sample.vertebrae[index].level).or_insert(0) += 1;
                    }
                }
            }
            // Bending films carry fixed marks, without a fitted covariance of
            // their own. The same fixed ends therefore give a singleton band.
            let bending_cobb = match curve.bending_cobb_deg {
                Ok(value) => {
                    let signed = value * curve.signed_cobb_deg.signum();
                    measured(signed, vec![signed; input.versions as usize])
                }
                Err(why) => missing(reason(why), None),
            };
            CurveReading {
                region,
                convex: curve.convex,
                upper_end: input.fit.vertebrae[a].level,
                lower_end: input.fit.vertebrae[b].level,
                apex: input.fit.vertebrae[apex].level,
                upper_end_seen: counts(upper),
                lower_end_seen: counts(lower),
                apex_seen: counts(apices),
                versions_found,
                cobb: measured(
                    nominal.front_pair(a, b),
                    samples.iter().map(|s| s.front_pair(a, b)).collect(),
                ),
                apex_twist: twist(&nominal, &samples, apex),
                bending_cobb,
            }
        })
        .collect();
    Ok(SpineReadings {
        tier: if input.fit.side_film.is_some() {
            FitTier::TwoFilms
        } else {
            FitTier::FrontFilmOnly
        },
        versions: input.versions,
        vertebrae,
        sagittal,
        curves,
        between: BetweenReadings {
            levels: input.fit.vertebrae.iter().map(|v| v.level).collect(),
            front,
            side,
        },
    })
}

fn counts(values: BTreeMap<Level, u32>) -> Vec<LevelCount> {
    values
        .into_iter()
        .map(|(level, versions)| LevelCount { level, versions })
        .collect()
}
