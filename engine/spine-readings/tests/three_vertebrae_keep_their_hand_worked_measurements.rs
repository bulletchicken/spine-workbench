//! Answers follow directly from the patient frame and the half-wedge
//! equations in docs/conventions/signs-and-wedges.md and SYNTHESIS 2.3.
//! Every film and vertebra here is made up.

use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::marks::{EndplateLines, FrontMarks, Mark, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{FilmToFit, MarkSpread};
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;
use shapes::spine_readings::ReadingsInput;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
use spine_readings::curves::ReadReason;
use spine_readings::curves::{geometry::regional_kyphosis, measure};

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-10,
        "{actual} versus {expected}"
    );
}

fn vertebra(level: Level, z: f64, y: f64, tilt: f64, wedge: f64) -> FittedVertebra {
    FittedVertebra {
        level,
        pose: VertebraPose {
            level,
            centre: [0.0, y, z],
            rotation: [tilt.to_radians(), 0.0, 0.0],
            height: 20.0,
            wedge_front: wedge.to_radians(),
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        },
        angles: FitAngles {
            side_tilt_rad: tilt.to_radians(),
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
    }
}

fn input(vertebrae: Vec<FittedVertebra>) -> ReadingsInput {
    ReadingsInput {
        fit: SpineFit {
            origin_level: vertebrae.last().unwrap().level,
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: shapes::spine_fit::FitLengths::Measured,
            typical_curve_deg: None,
            front_film: shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
            },
            side_film: Some(shapes::spine_fit::FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
            }),
        },
        front: None,
        side: None,
        bend_left: None,
        bend_right: None,
        versions: 32,
        seed: 9,
    }
}

fn three() -> ReadingsInput {
    input(vec![
        vertebra(Level::T5, 100.0, 0.0, -12.0, 4.0),
        vertebra(Level::T8, 70.0, -9.0, 0.0, 0.0),
        vertebra(Level::T12, 40.0, 0.0, 18.0, -6.0),
    ])
}

#[test]
fn upper_top_and_lower_bottom_give_thirty_one_degrees() {
    // Upper top = -12+4/2 = -10; lower bottom = 18-(-6)/2 = 21.
    // End-centre chord y=0, so the middle body is 9 mm to the right.
    let original = three();
    let reading = measure(&original, &original.fit).unwrap();
    assert_eq!(reading.curves.len(), 1);
    let curve = &reading.curves[0];
    assert_eq!(
        (curve.upper_index, curve.lower_index, curve.apex_index),
        (0, 2, 1)
    );
    near(reading.vertebrae[0].upper_tilt_deg, -10.0);
    near(reading.vertebrae[2].lower_tilt_deg, 21.0);
    near(curve.cobb_deg, 31.0);
    near(curve.signed_cobb_deg, 31.0);
    near(curve.apical_deviation_mm, -9.0);
    assert_eq!(curve.convex, BodySide::Right);
    near(reading.vertebrae[0].disc_below_deg.unwrap(), -14.0);
    near(reading.vertebrae[1].disc_below_deg.unwrap(), -15.0);
    assert_eq!(
        reading.vertebrae[2].disc_below_deg,
        Err(ReadReason::NotFitted)
    );

    let mut mirrored = original.clone();
    for v in &mut mirrored.fit.vertebrae {
        v.pose.centre[1] *= -1.0;
        v.pose.rotation[0] *= -1.0;
        v.pose.wedge_front *= -1.0;
        v.angles.side_tilt_rad *= -1.0;
    }
    let reflected = measure(&mirrored, &mirrored.fit).unwrap();
    near(reflected.curves[0].cobb_deg, 31.0);
    near(reflected.curves[0].signed_cobb_deg, -31.0);
    near(reflected.curves[0].apical_deviation_mm, 9.0);
    assert_eq!(reflected.curves[0].convex, BodySide::Left);

    let mut renamed = original.clone();
    for v in &mut renamed.fit.vertebrae {
        v.level = v.level.below().unwrap();
        v.pose.level = v.level;
    }
    renamed.fit.origin_level = renamed.fit.origin_level.below().unwrap();
    let shifted = measure(&renamed, &renamed.fit).unwrap();
    for (a, b) in reading.vertebrae.iter().zip(&shifted.vertebrae) {
        assert_eq!(a.centre_mm, b.centre_mm);
        assert_eq!(a.upper_tilt_deg.to_bits(), b.upper_tilt_deg.to_bits());
        assert_eq!(a.lower_tilt_deg.to_bits(), b.lower_tilt_deg.to_bits());
        assert_eq!(a.disc_below_deg, b.disc_below_deg);
    }
    near(shifted.curves[0].cobb_deg, 31.0);
    assert_eq!(shifted.curves[0].apex_index, 1);
}

#[test]
fn sagittal_half_wedges_give_twenty_four_degrees_of_kyphosis() {
    // Top = 15+4/2 = 17; bottom = -10-(-6)/2 = -7; 17-(-7)=24.
    // Positive rotY tips the column anteriorly going up, per conventions.
    let mut input = three();
    for (v, (lean, wedge)) in
        input
            .fit
            .vertebrae
            .iter_mut()
            .zip([(15.0_f64, 4.0_f64), (0.0, 0.0), (-10.0, -6.0)])
    {
        v.pose.rotation = [0.0, lean.to_radians(), 0.0];
        v.pose.wedge_front = 0.0;
        v.pose.wedge_side = wedge.to_radians();
        v.angles.side_tilt_rad = 0.0;
        v.angles.forward_lean_rad = lean.to_radians();
    }
    near(
        regional_kyphosis(&input.fit, Level::T5, Level::T12).unwrap(),
        24.0,
    );
    near(
        measure(&input, &input.fit)
            .unwrap()
            .kyphosis_t5_t12_deg
            .unwrap(),
        24.0,
    );
    input.fit.vertebrae[2].rms_mm.side = None;
    assert_eq!(
        regional_kyphosis(&input.fit, Level::T5, Level::T12),
        Err(ReadReason::NoSideFilm)
    );
}

#[test]
fn l4_tilt_and_named_twist_follow_the_patient_axes() {
    // Upper normal tilt = -12+4/2 = -10 degrees. A positive rotation
    // about z turns the anterior body axis towards patient left.
    let mut input = input(vec![vertebra(Level::L4, 10.0, 0.0, -12.0, 4.0)]);
    near(
        measure(&input, &input.fit).unwrap().l4_tilt_deg.unwrap(),
        -10.0,
    );
    let v = &mut input.fit.vertebrae[0];
    v.pose.wedge_front = 0.0;
    v.pose.rotation = [0.0, 0.0, 30.0_f64.to_radians()];
    v.angles.side_tilt_rad = 0.0;
    v.angles.twist_rad = 30.0_f64.to_radians();
    near(
        measure(&input, &input.fit).unwrap().vertebrae[0]
            .twist_deg
            .unwrap(),
        30.0,
    );
    input.fit.vertebrae[0].inferred.twist = true;
    assert_eq!(
        measure(&input, &input.fit).unwrap().vertebrae[0].twist_deg,
        Err(ReadReason::TwistNotMeasured)
    );
}

fn mark(x: f64, y: f64) -> Mark {
    Mark::auto(Px::new(x, y))
}

fn film(kind: FilmKind, marks: Vec<(Level, VertebraMarks)>) -> FilmToFit {
    FilmToFit {
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 0.0,
            shared_sd_mm: 0.0,
        },
        geometry: FilmGeometry {
            mm_per_px: Some(2.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        size: [200, 200],
        marks: FilmMarks {
            film: kind,
            named_for: ImageRight::PatientLeft,
            landmarks: Landmarks::NONE,
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            vertebrae: marks
                .into_iter()
                .enumerate()
                .map(|(i, (level, marks))| VertebraOnFilm {
                    id: i as u32,
                    level,
                    marks,
                    confidence: Confidence {
                        score: 1.0,
                        band: ConfidenceBand::High,
                        reasons: Vec::new(),
                        calibrated: false,
                        endplate_spread_px: None,
                        margin: None,
                    },
                    outline: None,
                })
                .collect(),
        },
    }
}

#[test]
fn a_five_pixel_sacral_offset_means_ten_millimetres_towards_the_right() {
    let mut input = input(vec![vertebra(Level::L5, 10.0, 0.0, 0.0, 0.0)]);
    let mut front = film(
        FilmKind::Front,
        vec![(
            Level::L5,
            VertebraMarks::Front {
                points: FrontMarks {
                    sup_l: mark(60.0, 20.0),
                    sup_r: mark(40.0, 20.0),
                    inf_l: mark(60.0, 40.0),
                    inf_r: mark(40.0, 40.0),
                    ped_l: PedicleMark::NotVisible {
                        source: shapes::marks::MarkSource::Auto,
                    },
                    ped_r: PedicleMark::NotVisible {
                        source: shapes::marks::MarkSource::Auto,
                    },
                },
            },
        )],
    );
    front.marks.landmarks.sacrum_top = Some(SacrumTop::Front {
        sac_l: mark(50.0, 50.0),
        sac_r: mark(40.0, 50.0),
    });
    input.front = Some(front);
    let measured = measure(&input, &input.fit).unwrap();
    near(measured.csvl_y_mm.unwrap(), -10.0);
    near(measured.vertebrae[0].centre_line_offset_mm.unwrap(), 10.0);
    let mut perturbed = input.fit.clone();
    perturbed.vertebrae[0].pose.centre[1] = 7.0;
    let moved = measure(&input, &perturbed).unwrap();
    near(moved.csvl_y_mm.unwrap(), -10.0);
    near(moved.vertebrae[0].centre_line_offset_mm.unwrap(), 17.0);
}

#[test]
fn the_same_ends_on_two_bends_choose_the_smaller_hand_worked_angle() {
    // A 45-degree line against a level line is 45 degrees. Two level
    // endplates give zero, so the other bend supplies the residual.
    let mut input = three();
    let marks = |sloped: bool| {
        vec![
            (
                Level::T5,
                VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: [
                            mark(10.0, if sloped { 20.0 } else { 10.0 }),
                            mark(20.0, 10.0),
                        ],
                        inf: [mark(10.0, 30.0), mark(20.0, 30.0)],
                    },
                },
            ),
            (
                Level::T12,
                VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: [mark(10.0, 50.0), mark(20.0, 50.0)],
                        inf: [mark(10.0, 60.0), mark(20.0, 60.0)],
                    },
                },
            ),
        ]
    };
    input.bend_left = Some(film(FilmKind::BendLeft, marks(true)));
    input.bend_right = Some(film(FilmKind::BendRight, marks(false)));
    near(
        spine_readings::curves::geometry::bending_angle(
            input.bend_left.as_ref().unwrap(),
            Level::T5,
            Level::T12,
        )
        .unwrap(),
        45.0,
    );
    let measured = measure(&input, &input.fit).unwrap();
    near(measured.curves[0].bending_cobb_deg.unwrap(), 0.0);
    assert_eq!(
        measured.curves[0].straightening_bend,
        Ok(FilmKind::BendRight)
    );
    input.bend_right = None;
    assert_eq!(
        measure(&input, &input.fit).unwrap().curves[0].bending_cobb_deg,
        Err(ReadReason::NoBendingFilms)
    );
}
