//! A lumbar apex does not require a nonzero curve. A modifier needs finite
//! pedicle outlines; every dimension here is an explicit synthetic fixture.
//! No film in this test is real.

use implant_layout::curves::lumbar::{PedicleOutlines, apex};
use implant_layout::curves::{measure, measure_with_pedicle_outlines};
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::implant_layout::{
    CannotTellCode, ChildFacts, LayoutFilm, LayoutInput, LumbarModifier, NamingDoubt,
};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::{Level, Region};
use shapes::made_up::{CurveRegion, VertebraPose};
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn registered_front() -> LayoutFilm {
    // Patient-left is screen-right. Both the L5 lower endplate and the
    // sacrum have a horizontal midpoint of pixel 50 at one mm per pixel.
    let mark = |x, y| Mark {
        at: Px::new(x, y),
        source: MarkSource::Hand,
        pinned: false,
        inferred: false,
    };
    LayoutFilm {
        marks: FilmMarks {
            film: FilmKind::Front,
            vertebrae: vec![VertebraOnFilm {
                id: 1,
                level: Level::L5,
                marks: VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: mark(65.0, 430.0),
                        sup_r: mark(35.0, 430.0),
                        inf_l: mark(65.0, 450.0),
                        inf_r: mark(35.0, 450.0),
                        ped_l: PedicleMark::NotVisible {
                            source: MarkSource::Hand,
                        },
                        ped_r: PedicleMark::NotVisible {
                            source: MarkSource::Hand,
                        },
                    },
                },
                confidence: Confidence {
                    score: 1.0,
                    band: ConfidenceBand::High,
                    reasons: Vec::new(),
                    calibrated: false,
                    endplate_spread_px: None,
                    margin: None,
                },
                outline: None,
            }],
            landmarks: Landmarks {
                sacrum_top: Some(SacrumTop::Front {
                    sac_l: mark(70.0, 460.0),
                    sac_r: mark(30.0, 460.0),
                }),
                ..Landmarks::NONE
            },
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: ImageRight::PatientLeft,
        },
        geometry: FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        size: [100, 500],
    }
}

fn thoracic_curve_with_a_balanced_lumbar_segment() -> LayoutInput {
    let vertebrae = Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(index, level)| {
            let tilt_deg: f64 = match level {
                Level::T1 | Level::T2 | Level::T3 | Level::T4 | Level::T5 => -20.0,
                Level::T6 => -10.0,
                Level::T8 | Level::T10 => 10.0,
                Level::T9 => 20.0,
                _ => 0.0,
            };
            FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [
                        0.0,
                        if level == Level::T7 { -10.0 } else { 0.0 },
                        410.0 - 25.0 * index as f64,
                    ],
                    rotation: [tilt_deg.to_radians(), 0.0, 0.0],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_width: 15.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: tilt_deg.to_radians(),
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
        })
        .collect();
    LayoutInput {
        fit: SpineFit {
            vertebrae,
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
        },
        front: Some(registered_front()),
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: None,
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 1,
        seed: 0,
        pedicle_child_share: None,
    }
}

fn with_csvl(offset_mm: f64) -> LayoutInput {
    let mut input = thoracic_curve_with_a_balanced_lumbar_segment();
    let front = input.front.as_mut().unwrap();
    let Some(SacrumTop::Front { sac_l, sac_r }) = &mut front.marks.landmarks.sacrum_top else {
        panic!("the fixture has front sacrum marks");
    };
    for mark in [sac_l, sac_r] {
        mark.at = Px::new(mark.at.x() + offset_mm, mark.at.y());
    }
    input
}

fn synthetic_outlines(input: &LayoutInput) -> Vec<PedicleOutlines> {
    // These are supplied geometry for this hand-built spine, not widths
    // estimated from a population or invented for an absent input.
    // Right pedicle [-12,-8], left [8,12], relative to each body centre:
    // A is strictly inside (-8,8), B touches a pedicle or the body.
    input
        .fit
        .vertebrae
        .iter()
        .filter(|v| v.level.region() == Region::Lumbar)
        .map(|v| {
            let y = v.pose.centre[1];
            PedicleOutlines {
                level: v.level,
                left_y_mm: [y + 8.0, y + 12.0],
                right_y_mm: [y - 12.0, y - 8.0],
            }
        })
        .collect()
}

#[test]
fn a_straight_lumbar_segment_has_an_apex_without_invented_outlines() {
    // The centred, untilted lumbar centres tie at zero deviation from
    // their regional end-centre chord. The first tied level is L1.
    // Centres alone do not bound the pedicles' finite medial edges, so
    // they cannot establish A against B even though CSVL is readable.
    let input = with_csvl(0.0);
    let reading = measure(&input, &input.fit).unwrap();
    assert_eq!(reading.csvl_y_mm, Ok(0.0));
    assert!(
        reading.curves.iter().any(|curve| {
            curve.region == Some(CurveRegion::MainThoracic) && curve.cobb_deg > 39.0
        })
    );
    assert!(
        reading
            .curves
            .iter()
            .all(|curve| curve.region != Some(CurveRegion::ThoracolumbarLumbar))
    );
    assert_eq!(
        apex(&reading, &input.fit).map(|i| input.fit.vertebrae[i].level),
        Ok(Level::L1)
    );
    assert_eq!(
        reading.classification.lumbar_modifier,
        Err(CannotTellCode::NoPedicleTable)
    );
}

#[test]
fn a_straight_lumbar_segment_with_finite_outlines_has_an_a_modifier() {
    // Zero lies in the open gap (-8,8), irrespective of which tied
    // lumbar level supplies the apex. No nonzero lumbar curve is needed.
    let input = with_csvl(0.0);
    let outlines = synthetic_outlines(&input);
    let reading = measure_with_pedicle_outlines(&input, &input.fit, &outlines).unwrap();
    assert_eq!(reading.csvl_y_mm, Ok(0.0));
    assert_eq!(
        reading.classification.lumbar_modifier,
        Ok(LumbarModifier::BetweenPedicles)
    );
}

#[test]
fn finite_pedicle_edges_distinguish_a_from_b_before_the_centres() {
    // The template centres are +/-0.78*15 = +/-11.7, but the supplied
    // pedicles start at +/-8. A CSVL at 11 crosses a pedicle despite
    // remaining between the two centres. Body support is [-15,15].
    for (csvl, expected) in [
        (7.75, LumbarModifier::BetweenPedicles),
        (8.0, LumbarModifier::TouchesApex),
        (11.0, LumbarModifier::TouchesApex),
        (15.0, LumbarModifier::TouchesApex),
        (16.0, LumbarModifier::ClearOfApex),
    ] {
        let input = with_csvl(csvl);
        let outlines = synthetic_outlines(&input);
        let reading = measure_with_pedicle_outlines(&input, &input.fit, &outlines).unwrap();
        assert_eq!(reading.csvl_y_mm, Ok(csvl));
        assert_eq!(reading.classification.lumbar_modifier, Ok(expected));

        let mut mirrored = with_csvl(-csvl);
        for v in &mut mirrored.fit.vertebrae {
            v.pose.centre[1] *= -1.0;
            v.pose.rotation[0] *= -1.0;
            v.pose.rotation[2] *= -1.0;
            v.pose.wedge_front *= -1.0;
            v.angles.side_tilt_rad *= -1.0;
            v.angles.twist_rad *= -1.0;
        }
        let mirrored_outlines = synthetic_outlines(&mirrored);
        let reflected =
            measure_with_pedicle_outlines(&mirrored, &mirrored.fit, &mirrored_outlines).unwrap();
        assert_eq!(reflected.classification.lumbar_modifier, Ok(expected));
    }
}

#[test]
fn a_lumbar_apex_can_be_read_without_an_alternating_tilt_extreme() {
    let mut input = with_csvl(0.0);
    input
        .fit
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L3)
        .unwrap()
        .pose
        .centre[1] = 9.0;
    let outlines = synthetic_outlines(&input);
    let reading = measure_with_pedicle_outlines(&input, &input.fit, &outlines).unwrap();
    // Regional endpoints L1 and L5 have y=0. L3 is 9 mm from their
    // chord, and its right pedicle spans [-3,1], intersecting CSVL=0.
    assert_eq!(
        apex(&reading, &input.fit).map(|i| input.fit.vertebrae[i].level),
        Ok(Level::L3)
    );
    assert_eq!(
        reading.classification.lumbar_modifier,
        Ok(LumbarModifier::TouchesApex)
    );
}

#[test]
fn absent_or_degenerate_outlines_never_become_zero_width_pedicles() {
    let input = with_csvl(0.0);
    let mut outlines = synthetic_outlines(&input);
    // L1 wins the straight segment's tied apex. An L2 outline cannot
    // supply its missing boundaries.
    outlines.retain(|outline| outline.level != Level::L1);
    let reading = measure_with_pedicle_outlines(&input, &input.fit, &outlines).unwrap();
    assert_eq!(
        reading.classification.lumbar_modifier,
        Err(CannotTellCode::NoPedicleTable)
    );
    let valid = synthetic_outlines(&input);
    for interval in [
        [8.0, 8.0],
        [12.0, 8.0],
        [f64::NAN, 12.0],
        [8.0, f64::INFINITY],
    ] {
        let mut invalid = valid.clone();
        invalid[0].left_y_mm = interval;
        assert!(measure_with_pedicle_outlines(&input, &input.fit, &invalid).is_err());
    }
    let mut duplicate = valid.clone();
    duplicate.push(valid[0]);
    assert!(measure_with_pedicle_outlines(&input, &input.fit, &duplicate).is_err());
    let mut unfitted = valid;
    unfitted[0].level = Level::C7;
    assert!(measure_with_pedicle_outlines(&input, &input.fit, &unfitted).is_err());
}

#[test]
fn overlapping_projected_pedicles_have_no_a_gap() {
    let input = with_csvl(0.0);
    let mut outlines = synthetic_outlines(&input);
    // Finite silhouettes overlap across the CSVL. Their interval centres
    // still straddle it, but there is no space between their medial edges.
    outlines[0].left_y_mm = [-1.0, 12.0];
    outlines[0].right_y_mm = [-12.0, 1.0];
    let reading = measure_with_pedicle_outlines(&input, &input.fit, &outlines).unwrap();
    assert_eq!(
        reading.classification.lumbar_modifier,
        Ok(LumbarModifier::TouchesApex)
    );
}
