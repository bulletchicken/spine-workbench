//! Hand readings from fusion-levels §§2a–2d. All geometry is synthetic.
//! A premeasured curve isolates the landmark rule from end detection.

use implant_layout::curves::lumbar::PedicleOutlines;
use implant_layout::curves::{Classification, Curve, SpineReading, VertebraReading};
use implant_layout::evidence::{Coordinate, PublishedValue, Table, tables};
use implant_layout::level_rules::{self, FusionRegion, RuleContext};
use shapes::implant_layout::{
    CannotTellCode, ChildFacts, CurveType, LayoutInput, LumbarModifier, NamingDoubt, Readable,
    SagittalModifier,
};
use shapes::levels::Level;
use shapes::made_up::{CurveRegion, VertebraPose};
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn threshold(group: &str, key: &str) -> f64 {
    let cell = tables()
        .lookup(
            Table::LevelRuleThresholds,
            &Coordinate {
                key: key.into(),
                group: Some(group.into()),
                ..Coordinate::default()
            },
        )
        .unwrap();
    let PublishedValue::Scalar { value } = cell.value else {
        panic!("scalar rule threshold")
    };
    value
}

struct HandSpine {
    input: LayoutInput,
    reading: SpineReading,
    outlines: Vec<PedicleOutlines>,
}

impl HandSpine {
    fn new(lower: Level) -> Self {
        let levels = [
            Level::T10,
            Level::T11,
            Level::T12,
            Level::L1,
            Level::L2,
            Level::L3,
            Level::L4,
            Level::L5,
        ];
        let vertebrae = levels
            .iter()
            .enumerate()
            .map(|(i, &level)| FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [0.0, 100.0, 300.0 - 30.0 * i as f64],
                    rotation: [0.0; 3],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    bottom_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: 0.0,
                    forward_lean_rad: 0.0,
                    twist_rad: 3.0_f64.to_radians(),
                },
                sd: FitStandardDeviations {
                    centre_mm: [1.0; 3],
                    angles_rad: [1.0_f64.to_radians(); 3],
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
            })
            .collect();
        let input = LayoutInput {
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
            front: None,
            bend_left: None,
            bend_right: None,
            child: ChildFacts {
                age_years: Some(14.0),
                weight_kg: None,
                activity: None,
            },
            naming: NamingDoubt::AsRead,
            versions: 8,
            seed: 27,
            pedicle_child_share: None,
        };
        let reading = SpineReading {
            vertebrae: input
                .fit
                .vertebrae
                .iter()
                .map(|v| VertebraReading {
                    level: v.level,
                    centre_mm: v.pose.centre,
                    upper_tilt_deg: 0.0,
                    lower_tilt_deg: 0.0,
                    twist_deg: Ok(3.0),
                    disc_below_deg: Ok(0.0),
                    centre_line_offset_mm: Ok(100.0),
                })
                .collect(),
            curves: vec![Curve {
                upper_index: 0,
                lower_index: levels.iter().position(|&l| l == lower).unwrap(),
                apex_index: 1,
                region: Some(CurveRegion::MainThoracic),
                convex: BodySide::Left,
                cobb_deg: 30.0,
                signed_cobb_deg: -30.0,
                apical_deviation_mm: 20.0,
                bending_cobb_deg: Ok(0.0),
                straightening_bend: Ok(shapes::film::FilmKind::BendLeft),
                bending_disc_below_deg: Ok(0.0),
                structural: Ok(true),
            }],
            major: Some(0),
            csvl_y_mm: Ok(0.0),
            kyphosis_t2_t5_deg: Ok(0.0),
            kyphosis_t5_t12_deg: Ok(0.0),
            kyphosis_t10_l2_deg: Ok(0.0),
            l4_tilt_deg: Ok(0.0),
            classification: Classification {
                curve_type: Ok(CurveType::MainThoracic),
                lumbar_modifier: Ok(LumbarModifier::BetweenPedicles),
                sagittal_modifier: Ok(SagittalModifier::Normal),
            },
            minor_curve_minimum_deg: None,
            sources: Vec::new(),
        };
        let outlines = levels
            .iter()
            .map(|&level| PedicleOutlines {
                level,
                left_y_mm: [108.0, 112.0],
                right_y_mm: [88.0, 92.0],
            })
            .collect();
        Self {
            input,
            reading,
            outlines,
        }
    }

    fn index(&self, level: Level) -> usize {
        self.reading
            .vertebrae
            .iter()
            .position(|v| v.level == level)
            .unwrap()
    }

    fn offset(&mut self, level: Level, y: f64) {
        let i = self.index(level);
        self.input.fit.vertebrae[i].pose.centre[1] = y;
        self.reading.vertebrae[i].centre_mm[1] = y;
        self.reading.vertebrae[i].centre_line_offset_mm = Ok(y);
        self.outlines[i].left_y_mm = [y + 8.0, y + 12.0];
        self.outlines[i].right_y_mm = [y - 12.0, y - 8.0];
    }

    fn context(&self) -> RuleContext<'_> {
        RuleContext {
            input: &self.input,
            fit: &self.input.fit,
            reading: &self.reading,
            outlines: &self.outlines,
            region: FusionRegion::Structural,
        }
    }
}

#[test]
fn a_substantial_touch_above_the_lower_end_is_still_the_most_proximal_touch() {
    // fusion-levels §2a and SYNTHESIS 2.5: MOST PROXIMAL substantially
    // touched vertebra. Unlike SV, this definition has no LEV lower bound.
    // T10/T11 are 100 mm away; T12 is centred on the CSVL; LEV is L1.
    let mut s = HandSpine::new(Level::L1);
    s.offset(Level::T12, 0.0);
    s.offset(Level::L1, 0.0);
    assert_eq!(
        level_rules::substantially_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    assert_eq!(
        level_rules::substantially_touched::landmark(&s.context())
            .map(|index| s.reading.vertebrae[index].level),
        Ok(Level::T12)
    );
}

#[test]
fn a_non_substantial_last_touch_uses_the_body_below_it() {
    // fusion-levels §2b, Qin: if the LAST TOUCHED vertebra is only an
    // nSTV, LIV=nSTV+1. T12's body spans [-2,28] and its right pedicle
    // [1,5], so y=0 touches body only. L1 is clear; L2 is substantial.
    // LTV is T12, hence its next caudal body L1 is the rule's answer.
    let mut s = HandSpine::new(Level::T12);
    s.offset(Level::T12, 13.0);
    s.offset(Level::L2, 0.0);
    assert_eq!(
        level_rules::last_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Ok(Level::T12)
    );
    assert_eq!(
        level_rules::substantially_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Ok(Level::L1)
    );
    assert_eq!(
        level_rules::substantially_touched::landmark(&s.context())
            .map(|index| s.reading.vertebrae[index].level),
        Ok(Level::L1)
    );
}

#[test]
fn one_below_crosses_twelve_to_one_and_never_wraps_below_five() {
    for (end, expected) in [
        (Level::T11, Ok(Level::T12)),
        (Level::T12, Ok(Level::L1)),
        (Level::L4, Ok(Level::L5)),
        (Level::L5, Err(CannotTellCode::NoLevelMeetsTheRule)),
    ] {
        let s = HandSpine::new(end);
        assert_eq!(
            level_rules::end_plus_one::evaluate(&s.context())
                .unwrap()
                .level,
            expected
        );
    }
}

#[test]
fn body_and_pedicle_contact_include_their_edges() {
    // Equal upright 15 mm half-width bodies: y=15 touches the body.
    // Pedicle outlines extend 12 mm: y=12 touches their lateral edge.
    for (y, body, substantial) in [
        (11.9, Level::T12, Level::T12),
        (12.0, Level::T12, Level::T12),
        (12.1, Level::T12, Level::L1),
        (14.9, Level::T12, Level::L1),
        (15.0, Level::T12, Level::L1),
        (15.1, Level::L1, Level::L1),
    ] {
        let mut s = HandSpine::new(Level::T12);
        s.offset(Level::T12, y);
        s.offset(Level::L1, 0.0);
        assert_eq!(
            level_rules::last_touched::evaluate(&s.context())
                .unwrap()
                .level,
            Ok(body)
        );
        assert_eq!(
            level_rules::substantially_touched::evaluate(&s.context())
                .unwrap()
                .level,
            Ok(substantial)
        );
    }
    let mut s = HandSpine::new(Level::T12);
    s.offset(Level::L5, 0.0);
    assert_eq!(
        level_rules::last_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Ok(Level::L5)
    );
    assert_eq!(
        level_rules::substantially_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Ok(Level::L5)
    );
    s.offset(Level::L5, 13.0);
    assert_eq!(
        level_rules::substantially_touched::evaluate(&s.context())
            .unwrap()
            .level,
        Err(CannotTellCode::NoLevelMeetsTheRule)
    );
}

#[test]
fn the_stable_minus_one_step_goes_up_and_its_centre_boundary_counts() {
    // Forward-model centres for this explicit body are +/-0.78*15=11.7.
    // LEV=T12. L1 is bisected if T12 is just outside that bracket.
    for (y, stable, above) in [
        (11.6, Level::T12, Level::T11),
        (11.7, Level::T12, Level::T11),
        (11.8, Level::L1, Level::T12),
    ] {
        let mut s = HandSpine::new(Level::T12);
        s.offset(Level::T12, y);
        s.offset(Level::L1, 0.0);
        for (modifier, expected) in [
            (LumbarModifier::BetweenPedicles, above),
            (LumbarModifier::TouchesApex, above),
            (LumbarModifier::ClearOfApex, stable),
        ] {
            s.reading.classification.lumbar_modifier = Ok(modifier);
            assert_eq!(
                level_rules::stable_minus_one::evaluate(&s.context())
                    .unwrap()
                    .level,
                Ok(expected)
            );
        }
    }
}

#[test]
fn the_published_distance_is_skipped_when_counting_up_from_the_sacrum() {
    // Wang as corrected by the citation audit: first cranial body >10.
    let limit = threshold("deviated-vertebra", "first-cephalad-deviation-min");
    for (y, expected) in [
        (limit - 0.1, Level::T12),
        (limit, Level::T12),
        (limit + 0.1, Level::L5),
    ] {
        let mut s = HandSpine::new(Level::T12);
        for level in [Level::L1, Level::L2, Level::L3, Level::L4, Level::L5] {
            s.offset(level, 0.0);
        }
        s.offset(Level::T12, limit + 1.0);
        s.offset(Level::L5, y);
        assert_eq!(
            level_rules::deviated_vertebra::evaluate(&s.context())
                .unwrap()
                .level,
            Ok(expected)
        );
    }
}

#[test]
fn the_nearer_zero_crossing_and_the_suk_step_have_their_written_directions() {
    for (upper, expected_nv) in [(-2.9, Level::T12), (-3.0, Level::L1), (-3.1, Level::L1)] {
        let mut s = HandSpine::new(Level::T12);
        let i = s.index(Level::T12);
        s.reading.vertebrae[i].twist_deg = Ok(upper);
        let nv = level_rules::suk_neutral::neutral(&s.context()).unwrap();
        assert_eq!(s.reading.vertebrae[nv].level, expected_nv);
    }
    // NV one below LEV -> NV. NV two or three below -> one UP from NV.
    for (nv, expected) in [
        (Level::L1, Level::L1),
        (Level::L2, Level::L1),
        (Level::L3, Level::L2),
    ] {
        let mut s = HandSpine::new(Level::T12);
        let i = s.index(nv);
        s.reading.vertebrae[i].twist_deg = Ok(0.0);
        assert_eq!(
            level_rules::suk_neutral::evaluate(&s.context())
                .unwrap()
                .level,
            Ok(expected)
        );
    }
}

#[test]
fn a_negative_liv_index_flags_and_zero_does_not() {
    // NV=T12; SV=L2. LIV T12 -> -2, L1 -> 0, L2 -> +2.
    let mut s = HandSpine::new(Level::T12);
    let i = s.index(Level::T12);
    s.reading.vertebrae[i].twist_deg = Ok(0.0);
    s.offset(Level::L2, 0.0);
    for (liv, value, flags) in [
        (Level::T12, -2.0, 1),
        (Level::L1, 0.0, 0),
        (Level::L2, 2.0, 0),
    ] {
        assert_eq!(level_rules::liv_index::value(&s.context(), liv), Ok(value));
        assert_eq!(
            level_rules::liv_index::check(&s.context(), liv)
                .unwrap()
                .flagged,
            Readable::Share { versions: flags }
        );
    }
}

#[test]
fn the_lumbar_disc_geometry_never_stands_in_for_the_missing_rotation_grade() {
    let mut s = HandSpine::new(Level::L4);
    s.reading.curves[0].region = Some(CurveRegion::ThoracolumbarLumbar);
    s.reading.curves[0].apex_index = s.index(Level::L1);
    let i = s.index(Level::L3);
    let neutral = threshold("lev-plus-one-5c", "bending-disc-angle");
    for (angle, expected) in [
        (neutral - 0.1, Level::L3),
        (neutral, Level::L3),
        (neutral + 0.1, Level::L4),
    ] {
        s.reading.vertebrae[i].disc_below_deg = Ok(angle);
        assert_eq!(
            level_rules::lenke_l3_l4::geometry(&s.context())
                .unwrap()
                .level,
            Ok(expected)
        );
        assert_eq!(
            level_rules::lenke_l3_l4::evaluate(&s.context())
                .unwrap()
                .level,
            Err(CannotTellCode::NoRotationGrade)
        );
    }
    s.reading.curves[0].apex_index = s.index(Level::L2);
    s.reading.vertebrae[i].disc_below_deg = Ok(neutral - 0.1);
    assert_eq!(
        level_rules::lenke_l3_l4::geometry(&s.context())
            .unwrap()
            .level,
        Ok(Level::L4)
    );
    assert_eq!(
        level_rules::lenke_shoulder::evaluate(&s.context())
            .unwrap()
            .level,
        Err(CannotTellCode::ShouldersOutsideTheModel)
    );
}

fn bend(
    kind: shapes::film::FilmKind,
    upper: Level,
    lower: Level,
    disc: f64,
) -> shapes::implant_layout::LayoutFilm {
    use shapes::confidence::{Confidence, ConfidenceBand};
    use shapes::film_geometry::{FilmGeometry, ImageRight};
    use shapes::landmarks::Landmarks;
    use shapes::marks::{EndplateLines, Mark};
    use shapes::naming::{ChosenBy, LevelNaming, Template};
    use shapes::pixels::Px;
    use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
    let horizontal = [
        Mark::auto(Px::new(100.0, 100.0)),
        Mark::auto(Px::new(120.0, 100.0)),
    ];
    shapes::implant_layout::LayoutFilm {
        size: [300, 500],
        geometry: FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
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
            vertebrae: [upper, lower]
                .into_iter()
                .enumerate()
                .map(|(i, level)| VertebraOnFilm {
                    id: i as u32,
                    level,
                    marks: VertebraMarks::Bending {
                        lines: EndplateLines {
                            sup: horizontal,
                            inf: if i == 0 {
                                [
                                    horizontal[0],
                                    Mark::auto(Px::new(
                                        120.0,
                                        100.0 - 20.0 * libm::tan(disc.to_radians()),
                                    )),
                                ]
                            } else {
                                horizontal
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
                })
                .collect(),
        },
    }
}

#[test]
fn type_five_c_lower_ends_use_the_strict_cobb_limit_and_reachable_neutral() {
    // fusion-levels §2d: >60 degrees or disc cannot reach zero means
    // LEV+1. A disc passing from negative to positive reaches zero.
    // The hand lower end L4 therefore gives L4 or its caudal neighbour L5.
    let mut s = HandSpine::new(Level::L4);
    s.reading.curves[0].region = Some(CurveRegion::ThoracolumbarLumbar);
    s.reading.curves[0].upper_index = s.index(Level::T12);
    s.reading.curves[0].apex_index = s.index(Level::L1);
    s.reading.classification.curve_type = Ok(CurveType::ThoracolumbarLumbar);
    s.reading.classification.lumbar_modifier = Ok(LumbarModifier::ClearOfApex);
    let limit = threshold("lev-plus-one-5c", "tll-cobb-min");
    let neutral = threshold("lev-plus-one-5c", "bending-disc-angle");
    for (cobb, left, right, expected) in [
        (limit - 0.1, neutral - 0.1, neutral + 0.1, Level::L4),
        (limit, neutral - 0.1, neutral + 0.1, Level::L4),
        (limit + 0.1, neutral - 0.1, neutral + 0.1, Level::L5),
        (limit - 0.1, neutral, neutral + 0.1, Level::L4),
        (limit - 0.1, neutral + 0.1, neutral + 0.2, Level::L5),
        (limit - 0.1, neutral - 0.2, neutral - 0.1, Level::L5),
    ] {
        s.reading.curves[0].cobb_deg = cobb;
        s.input.bend_left = Some(bend(
            shapes::film::FilmKind::BendLeft,
            Level::L4,
            Level::L5,
            left,
        ));
        s.input.bend_right = Some(bend(
            shapes::film::FilmKind::BendRight,
            Level::L4,
            Level::L5,
            right,
        ));
        assert_eq!(
            level_rules::lev_plus_one_5c::evaluate(&s.context())
                .unwrap()
                .level,
            Ok(expected)
        );
    }
    // Even >60 cannot create a vote when a required bend is absent.
    s.reading.curves[0].cobb_deg = limit + 0.1;
    for left_missing in [true, false] {
        let mut missing = s.input.clone();
        if left_missing {
            missing.bend_left = None;
        } else {
            missing.bend_right = None;
        }
        let ctx = RuleContext {
            input: &missing,
            ..s.context()
        };
        assert_eq!(
            level_rules::lev_plus_one_5c::evaluate(&ctx).unwrap().level,
            Err(CannotTellCode::NoBendingFilms)
        );
    }
}

#[test]
fn type_five_c_upper_options_step_cephalad_across_the_region_boundary() {
    // fusion-levels §3: posterior options UEV and one above, with no
    // caudal option. L1's predecessor is T12; T12's is T11.
    let mut s = HandSpine::new(Level::L4);
    s.reading.curves[0].region = Some(CurveRegion::ThoracolumbarLumbar);
    s.reading.curves[0].apex_index = s.index(Level::L2);
    s.reading.classification.curve_type = Ok(CurveType::ThoracolumbarLumbar);
    s.reading.classification.lumbar_modifier = Ok(LumbarModifier::ClearOfApex);
    for (uev, above) in [(Level::T12, Level::T11), (Level::L1, Level::T12)] {
        s.reading.curves[0].upper_index = s.index(uev);
        assert_eq!(
            level_rules::uev_5c::evaluate(&s.context()).unwrap().level,
            Ok(uev)
        );
        assert_eq!(
            level_rules::uev_5c::posterior_alternative(&s.context()).unwrap(),
            Ok(above)
        );
    }
}

#[test]
fn renaming_relative_landmarks_preserves_values_and_moves_their_levels_once() {
    // The hand contacts are below T12 in both naming scenarios, so the
    // anatomical TL/L search boundary does not add a new touched body.
    let mut s = HandSpine::new(Level::T12);
    s.offset(Level::L1, 0.0);
    let i = s.index(Level::L1);
    s.reading.vertebrae[i].twist_deg = Ok(0.0);
    let mut lumbar = s.reading.curves[0].clone();
    lumbar.region = Some(CurveRegion::ThoracolumbarLumbar);
    lumbar.upper_index = s.index(Level::T12);
    lumbar.lower_index = s.index(Level::L4);
    lumbar.apex_index = s.index(Level::L2);
    lumbar.cobb_deg = 10.0;
    lumbar.structural = Ok(false);
    s.reading.curves.push(lumbar);
    let before = level_rules::evaluate(&s.context()).unwrap();
    let names = shapes::naming::Template::Standard.names();
    let next = |level| names[names.iter().position(|&l| l == level).unwrap() + 1];
    for v in &mut s.input.fit.vertebrae {
        v.level = next(v.level);
        v.pose.level = v.level;
    }
    s.input.fit.origin_level = next(s.input.fit.origin_level);
    for v in &mut s.reading.vertebrae {
        v.level = next(v.level);
    }
    for o in &mut s.outlines {
        o.level = next(o.level);
    }
    let after = level_rules::evaluate(&s.context()).unwrap();
    for (a, b) in before.iter().zip(after) {
        assert_eq!(b.level, a.level.map(next), "{:?}", a.rule);
        for x in a.inputs.iter().filter(|x| x.value.is_some()) {
            let y = b
                .inputs
                .iter()
                .find(|y| {
                    y.level == x.level.map(next)
                        && y.region == x.region
                        && y.measurement == x.measurement
                        && y.threshold == x.threshold
                })
                .unwrap_or_else(|| panic!("{:?}: no renamed input for {x:?}", a.rule));
            assert_eq!(x.value, y.value);
            if x.level.is_some() {
                assert_eq!(x.sd, y.sd);
            }
        }
    }
}
