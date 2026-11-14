//! Structural curves and Lenke components read independently in each
//! version. Threshold values and source keys come through the evidence
//! reader. Lumbar modifiers use finite template pedicle outlines where
//! supplied; unresolved distinctions remain unreadable without them.

use super::lumbar::{self, PedicleOutlines};
use super::{Classification, Reading, SpineReading};
use crate::LayoutError;
use crate::evidence::{Coordinate, PublishedValue, Table, tables};
use shapes::implant_layout::{CannotTellCode, CurveType, SagittalModifier};
use shapes::made_up::CurveRegion;
use shapes::spine_fit::SpineFit;
#[cfg(test)]
use shapes::spine_readings::BodySide;

/// Fill the per-curve structural tests and the three classification
/// components. One positive structural criterion is sufficient even when
/// the other film is absent; an absent criterion is never false or met.
pub fn classify(reading: &mut SpineReading, fit: &SpineFit) -> Result<(), LayoutError> {
    classify_with_pedicle_outlines(reading, fit, &[])
}

/// Classify with version-specific finite template silhouettes in patient
/// millimetres. A missing apex outline never becomes a centre-based A/C.
pub fn classify_with_pedicle_outlines(
    reading: &mut SpineReading,
    fit: &SpineFit,
    outlines: &[PedicleOutlines],
) -> Result<(), LayoutError> {
    lumbar::validate(outlines, fit)?;
    let bending_min = threshold("structural-minor", "bending-cobb-min", &mut reading.sources)?;
    let proximal_min = threshold(
        "structural-minor",
        "kyphosis-t2-t5-min",
        &mut reading.sources,
    )?;
    let lower_min = threshold(
        "structural-minor",
        "kyphosis-t10-l2-min",
        &mut reading.sources,
    )?;
    let normal_min = threshold(
        "sagittal-modifier",
        "normal-kyphosis-min",
        &mut reading.sources,
    )?;
    let normal_max = threshold(
        "sagittal-modifier",
        "normal-kyphosis-max",
        &mut reading.sources,
    )?;
    for (index, curve) in reading.curves.iter_mut().enumerate() {
        curve.structural = if reading.major == Some(index) {
            Ok(true)
        } else {
            let sagittal = match curve.region {
                Some(CurveRegion::ProximalThoracic) => reading
                    .kyphosis_t2_t5_deg
                    .map(|value| value >= proximal_min),
                Some(CurveRegion::MainThoracic | CurveRegion::ThoracolumbarLumbar) => {
                    reading.kyphosis_t10_l2_deg.map(|value| value >= lower_min)
                }
                None => Err(CannotTellCode::NotThisCurveType),
            };
            either(
                curve.bending_cobb_deg.map(|value| value >= bending_min),
                sagittal,
            )
        };
    }
    reading.classification = Classification {
        curve_type: curve_type(reading),
        lumbar_modifier: lumbar::modifier(reading, fit, outlines),
        sagittal_modifier: reading.kyphosis_t5_t12_deg.map(|angle| {
            if angle < normal_min {
                SagittalModifier::Hypokyphotic
            } else if angle > normal_max {
                SagittalModifier::Hyperkyphotic
            } else {
                SagittalModifier::Normal
            }
        }),
    };
    reading.sources.sort();
    reading.sources.dedup();
    Ok(())
}

fn threshold(group: &str, key: &str, sources: &mut Vec<String>) -> Result<f64, LayoutError> {
    let cell = tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                key: key.to_owned(),
                group: Some(group.to_owned()),
                ..Coordinate::default()
            },
        )
        .ok_or_else(|| LayoutError::Refused(format!("missing-evidence:{group}:{key}")))?;
    let PublishedValue::Scalar { value } = cell.value else {
        return Err(LayoutError::Refused(format!(
            "non-scalar-evidence:{group}:{key}"
        )));
    };
    sources.push(cell.source.clone());
    Ok(value)
}

fn either(first: Reading<bool>, second: Reading<bool>) -> Reading<bool> {
    match (first, second) {
        (Ok(true), _) | (_, Ok(true)) => Ok(true),
        (Ok(false), Ok(false)) => Ok(false),
        (Err(reason), _) | (_, Err(reason)) => Err(reason),
    }
}

fn structural_in(reading: &SpineReading, region: CurveRegion) -> Reading<bool> {
    let mut structural = Ok(false);
    for curve in reading
        .curves
        .iter()
        .filter(|curve| curve.region == Some(region))
    {
        structural = either(structural, curve.structural);
    }
    structural
}

fn curve_type(reading: &SpineReading) -> Reading<CurveType> {
    let major = reading
        .major
        .and_then(|index| reading.curves.get(index))
        .ok_or(CannotTellCode::NoCurve)?;
    let proximal = structural_in(reading, CurveRegion::ProximalThoracic)?;
    let thoracic = structural_in(reading, CurveRegion::MainThoracic)?;
    let lumbar = structural_in(reading, CurveRegion::ThoracolumbarLumbar)?;
    match (proximal, thoracic, lumbar, major.region) {
        (true, true, true, _) => Ok(CurveType::TripleMajor),
        (false, true, false, Some(CurveRegion::MainThoracic)) => Ok(CurveType::MainThoracic),
        (true, true, false, _) => Ok(CurveType::DoubleThoracic),
        (false, true, true, Some(CurveRegion::MainThoracic)) => Ok(CurveType::DoubleMajor),
        (false, false, true, Some(CurveRegion::ThoracolumbarLumbar)) => {
            Ok(CurveType::ThoracolumbarLumbar)
        }
        (false, true, true, Some(CurveRegion::ThoracolumbarLumbar)) => {
            Ok(CurveType::ThoracolumbarLumbarMainThoracic)
        }
        _ => Err(CannotTellCode::NotThisCurveType),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::{Curve, VertebraReading};
    use forward_model::pedicles::pedicle_centres;
    use forward_model::vertebra_params::VertebraParams;
    use shapes::implant_layout::LumbarModifier;
    use shapes::levels::Level;
    use shapes::spine_fit::{
        FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    };

    fn curve(region: CurveRegion, structural: Reading<bool>) -> Curve {
        Curve {
            upper_index: 0,
            lower_index: 2,
            apex_index: 1,
            region: Some(region),
            convex: BodySide::Right,
            cobb_deg: 30.0,
            signed_cobb_deg: 30.0,
            apical_deviation_mm: -10.0,
            bending_cobb_deg: Ok(0.0),
            straightening_bend: Err(CannotTellCode::NoBendingFilms),
            bending_disc_below_deg: Err(CannotTellCode::NoBendingFilms),
            structural,
        }
    }

    fn reading() -> SpineReading {
        SpineReading {
            vertebrae: Vec::<VertebraReading>::new(),
            curves: vec![
                curve(CurveRegion::ProximalThoracic, Ok(false)),
                curve(CurveRegion::MainThoracic, Ok(true)),
                curve(CurveRegion::ThoracolumbarLumbar, Ok(false)),
            ],
            major: Some(1),
            csvl_y_mm: Err(CannotTellCode::NoSacrum),
            kyphosis_t2_t5_deg: Ok(0.0),
            kyphosis_t5_t12_deg: Ok(0.0),
            kyphosis_t10_l2_deg: Ok(0.0),
            l4_tilt_deg: Err(CannotTellCode::NoCurve),
            classification: Classification {
                curve_type: Err(CannotTellCode::NoCurve),
                lumbar_modifier: Err(CannotTellCode::NoCurve),
                sagittal_modifier: Err(CannotTellCode::NoSideFilm),
            },
            minor_curve_minimum_deg: None,
            sources: Vec::new(),
        }
    }

    fn empty_fit() -> SpineFit {
        SpineFit {
            vertebrae: Vec::new(),
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::L5,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: shapes::spine_fit::FitLengths::Measured,
            typical_curve_deg: None,
            front_film: shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
                facing_assumed: false,
            },
            side_film: Some(shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
                facing_assumed: false,
            }),
        }
    }

    #[test]
    fn structural_or_does_not_turn_an_absent_test_into_a_vote() {
        let missing = Err(CannotTellCode::NoBendingFilms);
        assert_eq!(either(missing, Ok(true)), Ok(true));
        assert_eq!(either(missing, Ok(false)), missing);
        assert_eq!(either(Ok(false), Ok(false)), Ok(false));
        assert_eq!(
            either(Ok(false), Err(CannotTellCode::NoSideFilm)),
            Err(CannotTellCode::NoSideFilm)
        );
        assert_eq!(either(Ok(true), Err(CannotTellCode::NoSideFilm)), Ok(true));
    }

    #[test]
    fn all_six_types_follow_the_structural_pattern_and_major_region() {
        let mut reading = reading();
        for (flags, major, expected) in [
            ([false, true, false], 1, CurveType::MainThoracic),
            ([true, true, false], 1, CurveType::DoubleThoracic),
            ([false, true, true], 1, CurveType::DoubleMajor),
            ([true, true, true], 1, CurveType::TripleMajor),
            ([false, false, true], 2, CurveType::ThoracolumbarLumbar),
            (
                [false, true, true],
                2,
                CurveType::ThoracolumbarLumbarMainThoracic,
            ),
        ] {
            for (curve, flag) in reading.curves.iter_mut().zip(flags) {
                curve.structural = Ok(flag);
            }
            reading.major = Some(major);
            assert_eq!(curve_type(&reading), Ok(expected));
        }
        reading.curves[0].structural = Err(CannotTellCode::NoBendingFilms);
        assert_eq!(curve_type(&reading), Err(CannotTellCode::NoBendingFilms));
    }

    #[test]
    fn structural_and_sagittal_boundaries_are_read_from_the_tables() {
        let mut source = Vec::new();
        let bend = threshold("structural-minor", "bending-cobb-min", &mut source).unwrap();
        let pt = threshold("structural-minor", "kyphosis-t2-t5-min", &mut source).unwrap();
        let lower = threshold("structural-minor", "kyphosis-t10-l2-min", &mut source).unwrap();
        let minimum = threshold("sagittal-modifier", "normal-kyphosis-min", &mut source).unwrap();
        let maximum = threshold("sagittal-modifier", "normal-kyphosis-max", &mut source).unwrap();
        let mut reading = reading();
        reading.curves[0].bending_cobb_deg = Ok(bend);
        reading.kyphosis_t5_t12_deg = Ok(minimum);
        classify(&mut reading, &empty_fit()).unwrap();
        assert_eq!(reading.curves[0].structural, Ok(true));
        assert_eq!(
            reading.classification.sagittal_modifier,
            Ok(SagittalModifier::Normal)
        );
        reading.curves[0].bending_cobb_deg = Err(CannotTellCode::NoBendingFilms);
        reading.curves[2].bending_cobb_deg = Err(CannotTellCode::NoBendingFilms);
        reading.kyphosis_t2_t5_deg = Ok(pt);
        reading.kyphosis_t10_l2_deg = Ok(lower);
        reading.kyphosis_t5_t12_deg = Ok(maximum);
        classify(&mut reading, &empty_fit()).unwrap();
        assert_eq!(reading.curves[0].structural, Ok(true));
        assert_eq!(reading.curves[2].structural, Ok(true));
        assert_eq!(
            reading.classification.sagittal_modifier,
            Ok(SagittalModifier::Normal)
        );
        reading.kyphosis_t2_t5_deg = Ok(pt - 1.0);
        reading.kyphosis_t10_l2_deg = Err(CannotTellCode::NoSideFilm);
        reading.kyphosis_t5_t12_deg = Err(CannotTellCode::NoSideFilm);
        classify(&mut reading, &empty_fit()).unwrap();
        assert_eq!(
            reading.curves[0].structural,
            Err(CannotTellCode::NoBendingFilms)
        );
        assert_eq!(
            reading.curves[2].structural,
            Err(CannotTellCode::NoBendingFilms)
        );
        assert_eq!(reading.curves[1].structural, Ok(true));
        assert_eq!(
            reading.classification.sagittal_modifier,
            Err(CannotTellCode::NoSideFilm)
        );
        assert!(
            reading
                .sources
                .iter()
                .all(|key| tables().source(key).is_some())
        );
    }

    #[test]
    fn template_lumbar_modifiers_touch_at_their_boundaries_and_mirror() {
        // An upright body centred at y=3 with half-width 15 spans [-12,18].
        // Synthetic finite pedicle silhouettes [11,15] and [-9,-5]
        // leave the open medial gap (-5,11), independently of the body.
        // Their widths are explicit test geometry, not inferred evidence.
        let params = VertebraParams {
            centre: nalgebra::Vector3::new(0.0, 3.0, 100.0),
            rotation: nalgebra::Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        };
        let mut fit = empty_fit();
        fit.vertebrae.push(FittedVertebra {
            level: Level::L2,
            pose: params.pose(Level::L2),
            angles: FitAngles {
                side_tilt_rad: 0.0,
                forward_lean_rad: 0.0,
                twist_rad: 0.0,
            },
            sd: FitStandardDeviations {
                centre_mm: [0.0; 3],
                angles_rad: [0.0; 3],
            },
            inferred: InferredFit {
                depth: false,
                width: false,
                twist: false,
            },
            rms_mm: FitRmsMm {
                front: Some(0.0),
                side: Some(0.0),
            },
            corner_tilt_rad: None,
        });
        let mut reading = reading();
        reading
            .curves
            .retain(|curve| curve.region == Some(CurveRegion::ThoracolumbarLumbar));
        reading.curves[0].apex_index = 0;
        let outlines = [PedicleOutlines {
            level: Level::L2,
            left_y_mm: [11.0, 15.0],
            right_y_mm: [-9.0, -5.0],
        }];
        for (offset, expected) in [
            (3.0, LumbarModifier::BetweenPedicles),
            (10.9, LumbarModifier::BetweenPedicles),
            (11.0, LumbarModifier::TouchesApex),
            (11.5, LumbarModifier::TouchesApex),
            (18.0, LumbarModifier::TouchesApex),
            (18.1, LumbarModifier::ClearOfApex),
        ] {
            reading.csvl_y_mm = Ok(offset);
            assert_eq!(lumbar::modifier(&reading, &fit, &outlines), Ok(expected));
            let mut mirrored = fit.clone();
            mirrored.vertebrae[0].pose.centre[1] *= -1.0;
            reading.csvl_y_mm = Ok(-offset);
            let mirrored_outlines = [PedicleOutlines {
                level: Level::L2,
                left_y_mm: [5.0, 9.0],
                right_y_mm: [-15.0, -11.0],
            }];
            assert_eq!(
                lumbar::modifier(&reading, &mirrored, &mirrored_outlines),
                Ok(expected)
            );
        }
        for offset in [3.0, 11.5, 18.1] {
            reading.csvl_y_mm = Ok(offset);
            assert_eq!(
                lumbar::modifier(&reading, &fit, &[]),
                Err(CannotTellCode::NoPedicleTable)
            );
        }
        for offset in [pedicle_centres(&params)[0].y, 18.0] {
            reading.csvl_y_mm = Ok(offset);
            assert_eq!(
                lumbar::modifier(&reading, &fit, &[]),
                Ok(LumbarModifier::TouchesApex)
            );
        }
        reading.csvl_y_mm = Err(CannotTellCode::NoSacrum);
        assert_eq!(
            lumbar::modifier(&reading, &fit, &outlines),
            Err(CannotTellCode::NoSacrum)
        );
    }

    #[test]
    fn sagittal_modifiers_change_only_outside_the_inclusive_normal_band() {
        let mut source = Vec::new();
        let minimum = threshold("sagittal-modifier", "normal-kyphosis-min", &mut source).unwrap();
        let maximum = threshold("sagittal-modifier", "normal-kyphosis-max", &mut source).unwrap();
        let mut reading = reading();
        for (angle, expected) in [
            (minimum - 1.0, SagittalModifier::Hypokyphotic),
            (minimum, SagittalModifier::Normal),
            (maximum, SagittalModifier::Normal),
            (maximum + 1.0, SagittalModifier::Hyperkyphotic),
        ] {
            reading.kyphosis_t5_t12_deg = Ok(angle);
            classify(&mut reading, &empty_fit()).unwrap();
            assert_eq!(reading.classification.sagittal_modifier, Ok(expected));
        }
    }
}
