//! Complete proposals use only generated geometry and exact synthetic film marks.

use forward_model::rotation::matrix_from_rotation_vector;
use implant_layout::{curves, evidence::tables, propose_layouts, rods, screws::sizing};
use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use nalgebra::Vector3;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::implant_layout::{
    AnchorKind, CannotTellCode, ChildFacts, ImplantLayout, LayoutChange, LayoutFilm, LayoutInput,
    LayoutProposals, LookCode, NamingDoubt, Readable, ValueStatus,
};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::spine_fit::{
    FilmPlacement, FilmToFit, FitAngles, FitFromFilms, FitLengths, FitRmsMm, FitStandardDeviations,
    FitStopReason, FittedVertebra, InferredFit, MarkSpread, SpineFit, TypicalSource,
};
use shapes::spine_readings::{BodySide, FitTier};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn mark(truth: TrueMark) -> Mark {
    Mark {
        at: truth.at,
        source: MarkSource::Hand,
        pinned: false,
        inferred: !truth.visible,
    }
}
fn pedicle(truth: TrueMark) -> PedicleMark {
    if truth.visible {
        PedicleMark::Visible {
            at: truth.at,
            source: MarkSource::Hand,
            pinned: false,
            inferred: false,
            ring_score: None,
        }
    } else {
        PedicleMark::NotVisible {
            source: MarkSource::Hand,
        }
    }
}
fn film(answer: &FilmAnswer, template: Template, size: [u32; 2]) -> FilmToFit {
    let vertebrae: Vec<_> = answer
        .vertebrae
        .iter()
        .filter(|v| v.level >= Level::T1)
        .enumerate()
        .map(|(i, v)| VertebraOnFilm {
            id: i as u32 + 1,
            level: v.level,
            marks: match v.marks {
                TrueMarks::Front {
                    sup_l,
                    sup_r,
                    inf_l,
                    inf_r,
                    ped_l,
                    ped_r,
                } => VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: mark(sup_l),
                        sup_r: mark(sup_r),
                        inf_l: mark(inf_l),
                        inf_r: mark(inf_r),
                        ped_l: pedicle(ped_l),
                        ped_r: pedicle(ped_r),
                    },
                },
                TrueMarks::Side {
                    sup_a,
                    sup_p,
                    inf_a,
                    inf_p,
                } => VertebraMarks::Side {
                    points: SideMarks {
                        sup_a: mark(sup_a),
                        sup_p: mark(sup_p),
                        inf_a: mark(inf_a),
                        inf_p: mark(inf_p),
                    },
                },
                TrueMarks::Bending { sup, inf } => VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: sup.map(mark),
                        inf: inf.map(mark),
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
        .collect();
    let [a, _, b] = answer.landmarks.sacrum_top;
    FilmToFit {
        marks: FilmMarks {
            film: answer.kind,
            vertebrae,
            landmarks: Landmarks {
                sacrum_top: Some(if answer.kind == FilmKind::Side {
                    SacrumTop::Side {
                        sac_a: mark(a),
                        sac_p: mark(b),
                    }
                } else {
                    SacrumTop::Front {
                        sac_l: mark(a),
                        sac_r: mark(b),
                    }
                }),
                ..Landmarks::NONE
            },
            naming: LevelNaming {
                template,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: answer.geometry.image_right.unwrap(),
        },
        geometry: answer.geometry.clone(),
        size,
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 0.5,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}
fn fixture(name: RecipeName) -> (LayoutInput, Vec<FilmToFit>) {
    fixture_seed(name, 1)
}

fn fixture_seed(name: RecipeName, seed: u32) -> (LayoutInput, Vec<FilmToFit>) {
    let recipe = MadeUpRecipe {
        name,
        seed,
        tier: Tier::T0,
    };
    let patient = make_up_patient(name, seed).expect("the named recipe stacks");
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(seed))
        .iter()
        .map(|r| FilmLayout::new(r, spine_for(&patient, r.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let films: Vec<_> = key
        .films
        .iter()
        .zip(&layouts)
        .map(|(a, l)| film(a, key.template, l.size))
        .collect();
    let vertebrae = key
        .poses
        .iter()
        .filter(|p| p.level >= Level::T1)
        .map(|pose| {
            let r = matrix_from_rotation_vector(&pose.rotation.into());
            FittedVertebra {
                level: pose.level,
                pose: *pose,
                angles: FitAngles {
                    side_tilt_rad: libm::atan2(-r[(1, 2)], r[(2, 2)]),
                    forward_lean_rad: libm::asin(r[(0, 2)].clamp(-1.0, 1.0)),
                    twist_rad: libm::atan2(-r[(0, 1)], r[(0, 0)]),
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
    let layout_film = |i: usize| LayoutFilm {
        marks: films[i].marks.clone(),
        geometry: films[i].geometry.clone(),
        size: films[i].size,
    };
    (
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
                lengths: FitLengths::Measured,
                typical_curve_deg: None,
                front_film: FilmPlacement {
                    mm_per_px: [1.0; 2],
                    origin_px: [0.0; 2],
                    facing_assumed: false,
                },
                side_film: Some(FilmPlacement {
                    mm_per_px: [1.0; 2],
                    origin_px: [0.0; 2],
                    facing_assumed: false,
                }),
            },
            front: Some(layout_film(0)),
            bend_left: Some(layout_film(2)),
            bend_right: Some(layout_film(3)),
            child: ChildFacts {
                age_years: Some(14.0),
                weight_kg: None,
                activity: None,
            },
            naming: NamingDoubt::AsRead,
            versions: 32,
            seed,
            pedicle_child_share: None,
        },
        films,
    )
}

fn sources(keys: &[String]) {
    assert!(!keys.is_empty(), "a table-derived family has sources");
    for key in keys {
        assert!(tables().source(key).is_some(), "unresolved source {key}");
    }
}
fn band(b: &shapes::spine_readings::Band) {
    assert!(b.p10.is_finite() && b.p50.is_finite() && b.p90.is_finite());
    assert!(b.p10 <= b.p50 && b.p50 <= b.p90);
}
fn check_layout(input: &LayoutInput, layout: &ImplantLayout) {
    assert!((Level::T1..=Level::L5).contains(&layout.upper));
    assert!((Level::T1..=Level::L5).contains(&layout.lower));
    assert!(layout.upper < layout.lower);
    let levels: Vec<_> = input
        .fit
        .vertebrae
        .iter()
        .map(|v| v.level)
        .filter(|l| layout.upper <= *l && *l <= layout.lower)
        .collect();
    assert_eq!(layout.anchors.len(), 2 * levels.len());
    let dgrid = sizing::diameter_grid();
    let lgrid = sizing::length_grid();
    for (i, a) in layout.anchors.iter().enumerate() {
        assert_eq!(a.level, levels[i / 2]);
        assert_eq!(
            a.side,
            if i % 2 == 0 {
                BodySide::Left
            } else {
                BodySide::Right
            }
        );
        sources(&a.sources);
        assert!(a.entry_mm.iter().all(|x| x.is_finite()));
        assert_ne!(a.entry_status, ValueStatus::Missing);
        assert!(a.direction_unit.iter().all(|x| x.is_finite()));
        let norm = a.direction_unit.iter().map(|x| x * x).sum::<f64>();
        assert!((norm - 1.0).abs() < 1e-10);
        band(&a.convergence_deg);
        if let Some(d) = a.diameter_mm {
            assert!(dgrid.iter().any(|g| g.mm == d));
            assert_eq!(a.diameter_status, ValueStatus::FromTables);
        } else {
            assert!(a.kind != AnchorKind::Screw || !a.looks.is_empty());
        }
        if let Some(l) = a.length_mm {
            assert!(lgrid.iter().any(|g| g.mm == l));
            assert_eq!(a.length_status, ValueStatus::FromTables);
        } else {
            assert!(a.kind != AnchorKind::Screw || !a.looks.is_empty());
        }
        for d in &a.diameter_also_seen {
            assert!(dgrid.iter().any(|g| g.mm == *d));
        }
        for l in &a.length_also_seen {
            assert!(lgrid.iter().any(|g| g.mm == *l));
        }
        if let Some(v) = &a.medial_angle {
            assert!(v.angle_deg.is_finite());
            assert!(tables().source(&v.source).is_some());
        } else {
            assert!(a.kind != AnchorKind::Screw || !a.looks.is_empty());
        }
        for v in &a.medial_angles_beside {
            assert!(tables().source(&v.source).is_some());
        }
        if let Some(r) = a.width_ratio {
            assert!(r.is_finite() && r > 0.0);
        }
        for r in &a.diameters_beside {
            assert!(tables().source(&r.source).is_some());
        }
        assert_eq!(
            a.look_sources.iter().map(|s| s.code).collect::<Vec<_>>(),
            a.looks
        );
        if let Some(b) = &a.screw_to_pedicle {
            band(b);
        } else {
            assert!(a.kind != AnchorKind::Screw || !a.looks.is_empty());
        }
        assert!(a.chord_mm.is_some() || !a.looks.is_empty());
        assert!(a.chord_share.is_some() || !a.looks.is_empty());
        assert!(a.usual_length_mm.is_some() || !a.looks.is_empty());
        for b in [&a.pedicle.outer_width_mm, &a.pedicle.inner_width_mm] {
            if let Some(b) = b {
                band(b);
                assert_ne!(a.pedicle.status, ValueStatus::Missing);
            } else {
                assert!(!a.looks.is_empty());
            }
        }
        assert!(a.pedicle.versions_too_narrow <= input.versions);
        assert!(a.pedicle.versions_inner_under_1_mm <= input.versions);
        // Only the chosen rule's grid counts and this pedicle's outer
        // width/channel counts can raise the narrow look. A pattern can
        // clear the displayed diameter without changing those counts.
        let no_diameter = !a
            .wider_than_pedicle
            .iter()
            .any(|d| d.versions * 10 <= input.versions);
        let own_narrow = no_diameter
            || a.pedicle.versions_too_narrow * 10 > input.versions
            || a.pedicle.versions_inner_under_1_mm * 10 > input.versions;
        assert_eq!(
            a.looks.contains(&LookCode::MayBeTooNarrow),
            own_narrow,
            "{:?} {:?}",
            a.level,
            a.side
        );
    }
    assert_eq!(layout.rods.len(), 2);
    let reading = curves::measure(input, &input.fit).unwrap();
    let target = rods::target_spine(&input.fit, &levels, &reading).unwrap();
    for (side, rod) in [BodySide::Left, BodySide::Right]
        .into_iter()
        .zip(&layout.rods)
    {
        assert_eq!(rod.side, side);
        sources(&rod.sources);
        assert!(
            rods::choices()
                .iter()
                .any(|c| c.material == rod.material && c.diameter_mm == rod.diameter_mm)
        );
        assert_eq!(rod.overbend_deg, 0.0);
        assert!(rod.target_kyphosis_deg.is_finite());
        assert_ne!(rod.target_kyphosis_status, ValueStatus::Missing);
        assert!(rod.length_mm > 0.0 && rod.points_mm.len() >= 2);
        assert!(rod.points_mm.iter().flatten().all(|x| x.is_finite()));
        for a in layout
            .anchors
            .iter()
            .filter(|a| a.side == side && a.kind != AnchorKind::None)
        {
            let standing = &input
                .fit
                .vertebrae
                .iter()
                .find(|v| v.level == a.level)
                .unwrap()
                .pose;
            let changed = target.poses.iter().find(|v| v.level == a.level).unwrap();
            let head =
                Vector3::from(a.entry_mm) - Vector3::from(a.direction_unit) * rod.head_offset_mm;
            let local = matrix_from_rotation_vector(&standing.rotation.into()).transpose()
                * (head - Vector3::from(standing.centre));
            let moved = Vector3::from(changed.centre)
                + matrix_from_rotation_vector(&changed.rotation.into()) * local;
            assert!(
                rod.points_mm
                    .iter()
                    .any(|p| (Vector3::from(*p) - moved).norm() < 1e-7),
                "{:?} {:?} target head is a contour knot",
                a.level,
                a.side
            );
        }
    }
    assert!(layout.released_levels.iter().all(|l| levels.contains(l)));
}

/// This project's own numerical tolerance for a rod that can be bent and
/// drawn: the largest turn, in degrees, between consecutive samples (at
/// most 2 mm apart) of the contoured or the standing line. The made-up
/// recipes below, the 90 degree one included, now stay under 4.4 degrees;
/// before entries lined up and the target bent its joints by one shared
/// angle they reached 6.9, and 87 on the 90 degree recipe.
const MAX_TURN_PER_SAMPLE_DEG: f64 = 7.0;

fn largest_turn_deg(points: &[[f64; 3]]) -> f64 {
    points
        .windows(3)
        .map(|p| {
            let u = Vector3::from(p[1]) - Vector3::from(p[0]);
            let v = Vector3::from(p[2]) - Vector3::from(p[1]);
            libm::atan2(u.cross(&v).norm(), u.dot(&v)).to_degrees()
        })
        .fold(0.0, f64::max)
}

fn rods_turn_smoothly(layout: &ImplantLayout) {
    for rod in &layout.rods {
        for (line, points) in [
            ("contoured", &rod.points_mm),
            ("standing", &rod.standing_points_mm),
        ] {
            let turn = largest_turn_deg(points);
            assert!(
                turn <= MAX_TURN_PER_SAMPLE_DEG,
                "{} {:?} {line} rod turns {turn} degrees in one sample",
                layout.id,
                rod.side
            );
        }
    }
}

/// This test's own bound: a screw more than 60 degrees off its vertebra's
/// forward axis would be drawn lying across the bone.
const MAX_SCREW_OFF_FORWARD_DEG: f64 = 60.0;

/// In each vertebra's own frame the left seat stays left of the right one,
/// and every screw, an added end level's included, enters its bone from
/// behind rather than lying across it.
fn seats_keep_their_sides_and_screws_enter_their_bone(input: &LayoutInput, layout: &ImplantLayout) {
    for level in layout.anchors.iter().map(|a| a.level) {
        let pose = input
            .fit
            .vertebrae
            .iter()
            .find(|v| v.level == level)
            .unwrap()
            .pose;
        let turn = matrix_from_rotation_vector(&pose.rotation.into());
        let forward = turn * Vector3::x();
        let left = turn * Vector3::y();
        let seat = |side| {
            let a = layout
                .anchors
                .iter()
                .find(|a| a.level == level && a.side == side)?;
            let rod = layout.rods.iter().find(|r| r.side == side)?;
            (a.kind != AnchorKind::None).then(|| {
                Vector3::from(a.entry_mm) - Vector3::from(a.direction_unit) * rod.head_offset_mm
            })
        };
        if let (Some(l), Some(r)) = (seat(BodySide::Left), seat(BodySide::Right)) {
            assert!(
                (l - r).dot(&left) > 0.0,
                "{} {level:?} seats swap sides",
                layout.id
            );
        }
        for a in layout
            .anchors
            .iter()
            .filter(|a| a.level == level && a.kind == AnchorKind::Screw)
        {
            let off = libm::acos(
                Vector3::from(a.direction_unit)
                    .dot(&forward)
                    .clamp(-1.0, 1.0),
            );
            assert!(
                off.to_degrees() <= MAX_SCREW_OFF_FORWARD_DEG,
                "{} {level:?} {:?} screw lies {} degrees off its forward axis",
                layout.id,
                a.side,
                off.to_degrees()
            );
        }
    }
}

fn hook_beside_screw(layout: &ImplantLayout) -> bool {
    [BodySide::Left, BodySide::Right].into_iter().any(|side| {
        let kinds: Vec<_> = layout
            .anchors
            .iter()
            .filter(|a| a.side == side && a.kind != AnchorKind::None)
            .map(|a| a.kind)
            .collect();
        kinds
            .windows(2)
            .any(|p| p.contains(&AnchorKind::Hook) && p.contains(&AnchorKind::Screw))
    })
}

fn check_proposal(input: &LayoutInput, p: &LayoutProposals) {
    assert_eq!(p.versions, input.versions);
    assert_eq!(
        p.tier,
        if input.fit.side_film.is_some() {
            FitTier::TwoFilms
        } else {
            FitTier::FrontFilmOnly
        }
    );
    assert_eq!(
        p.classification.iter().map(|c| c.versions).sum::<u32>(),
        input.versions
    );
    for c in &p.curves {
        band(&c.cobb_deg);
        assert!(!c.upper_end.is_empty());
        assert!(!c.lower_end.is_empty());
        assert!(!c.apex.is_empty());
        assert!(c.versions_major <= input.versions && c.versions_found <= input.versions);
        if let Some(b) = &c.bending_cobb_deg {
            band(b);
        } else {
            assert!(
                p.cannot_tell
                    .iter()
                    .any(|x| x.code == CannotTellCode::NoBendingFilms)
            );
        }
    }
    for r in &p.rules {
        sources(&r.sources);
        assert_eq!(
            r.counts.levels.iter().map(|x| x.versions).sum::<u32>()
                + r.counts.unread.iter().map(|x| x.versions).sum::<u32>(),
            input.versions
        );
        for i in &r.inputs {
            if let Some(x) = i.value {
                assert!(x.is_finite());
                assert_ne!(i.status, ValueStatus::Missing);
            } else {
                assert_eq!(i.status, ValueStatus::Missing);
                assert!(!p.cannot_tell.is_empty());
            }
            assert!(i.sd.is_none_or(|x| x.is_finite() && x >= 0.0));
            assert!(i.threshold.is_none_or(f64::is_finite));
        }
        if let Some(f) = r.flip {
            assert!(f.change_in_sd.is_finite());
            assert!(f.change_value.is_finite());
        }
    }
    for s in &p.selective {
        sources(&s.sources);
        for c in &s.criteria {
            if let Some(v) = &c.value {
                band(v);
            } else {
                assert!(matches!(c.holds, Readable::NotReadable { .. }));
            }
        }
    }
    for pair in &p.pairs {
        assert!(pair.upper < pair.lower);
        assert!(pair.versions_holding <= input.versions);
        for odds in &pair.stop_short {
            assert!(odds.odds_ratio > 0.0);
            assert!(tables().source(&odds.source).is_some());
        }
        for check in &pair.checks {
            assert!(tables().source(&check.source).is_some());
        }
    }
    sources(&p.choices.sources);
    for d in &p.choices.screw_diameters_mm {
        assert!(sizing::diameter_grid().iter().any(|g| g.mm == *d));
    }
    for l in &p.choices.screw_lengths_mm {
        assert!(sizing::length_grid().iter().any(|g| g.mm == *l));
    }
    for d in &p.choices.rod_diameters_mm {
        assert!(rods::choices().iter().any(|c| c.diameter_mm == *d));
    }
    for m in &p.choices.rod_materials {
        assert!(rods::choices().iter().any(|c| c.material == *m));
    }
    if let Some(first) = p.layouts.first() {
        assert_eq!(first.id, "first");
        assert_eq!(first.change, LayoutChange::First {});
        assert!(p.layouts.len() <= 12);
        for alternative in p.layouts.iter().skip(1) {
            match alternative.change {
                LayoutChange::EndsAt { level } => {
                    assert_eq!(alternative.upper, first.upper);
                    assert_eq!(alternative.lower, level);
                    assert_ne!(level, first.lower);
                }
                LayoutChange::StartsAt { level } => {
                    assert_eq!(alternative.upper, level);
                    assert_eq!(alternative.lower, first.lower);
                    assert_ne!(level, first.upper);
                }
                LayoutChange::StifferRods {} | LayoutChange::SofterRods {} => {
                    assert_eq!(
                        (alternative.upper, alternative.lower),
                        (first.upper, first.lower)
                    );
                    assert_eq!(alternative.anchors, first.anchors);
                    for (rod, original) in alternative.rods.iter().zip(&first.rods) {
                        assert_eq!(rod.points_mm, original.points_mm);
                        assert_eq!(rod.overbend_deg, original.overbend_deg);
                        assert!(
                            rod.material != original.material
                                || rod.diameter_mm != original.diameter_mm
                        );
                    }
                }
                LayoutChange::FewerScrews {} | LayoutChange::HooksAtTop {} => {
                    assert_eq!(
                        (alternative.upper, alternative.lower),
                        (first.upper, first.lower)
                    );
                    assert_eq!(alternative.rods[0].material, first.rods[0].material);
                    assert_eq!(alternative.rods[0].diameter_mm, first.rods[0].diameter_mm);
                    for (anchor, original) in alternative.anchors.iter().zip(&first.anchors) {
                        assert_eq!(anchor.entry_mm, original.entry_mm);
                        assert_eq!(anchor.direction_unit, original.direction_unit);
                    }
                }
                _ => panic!("each returned alternative changes one named feature"),
            }
        }
    } else {
        assert!(p.cannot_tell.iter().any(|x| matches!(
            x.code,
            CannotTellCode::NoCurve | CannotTellCode::NoLevelMeetsTheRule
        )));
    }
    for layout in &p.layouts {
        check_layout(input, layout);
    }
    let json = serde_json::to_value(p).unwrap();
    assert_eq!(serde_json::from_value::<LayoutProposals>(json).unwrap(), *p);
}

macro_rules! recipe_case {
    ($name:ident, $recipe:ident) => {
        #[test]
        fn $name() {
            let (input, _) = fixture(RecipeName::$recipe);
            let p = propose_layouts(&input).expect("the named recipe returns proposals");
            check_proposal(&input, &p);
            p.layouts.iter().for_each(rods_turn_smoothly);
            for layout in &p.layouts {
                seats_keep_their_sides_and_screws_enter_their_bone(&input, layout);
            }
        }
    };
}
recipe_case!(a_random_spine_returns_a_complete_proposal, Random);
recipe_case!(a_straight_spine_returns_a_complete_proposal, Straight);
recipe_case!(
    a_right_thoracic_spine_returns_a_complete_proposal,
    RightThoracic40
);
recipe_case!(
    a_double_major_spine_returns_a_complete_proposal,
    DoubleMajor55To45
);
recipe_case!(
    a_lumbar_left_spine_returns_a_complete_proposal,
    LumbarLeft30
);
recipe_case!(a_severe_spine_returns_a_complete_proposal, Severe90);
recipe_case!(six_lumbar_bodies_return_a_complete_proposal, SixLumbar);
recipe_case!(eleven_ribs_return_a_complete_proposal, ElevenRibs);
recipe_case!(an_upside_down_film_returns_a_complete_proposal, UpsideDown);
recipe_case!(a_mirrored_front_returns_a_complete_proposal, MirroredFront);
recipe_case!(
    a_left_facing_side_returns_a_complete_proposal,
    SideFacingLeft
);
recipe_case!(heavy_shoulders_return_a_complete_proposal, HeavyShoulders);

#[test]
fn a_front_film_without_scale_runs_through_the_built_in_typical_fit() {
    let (mut input, mut films) = fixture(RecipeName::RightThoracic40);
    films[0].geometry.mm_per_px = None;
    films[0].geometry.mm_per_px_vertical = None;
    films[0].geometry.scale = None;
    for v in &mut films[0].marks.vertebrae {
        if let VertebraMarks::Front { points } = &mut v.marks {
            points.ped_l = PedicleMark::NotVisible {
                source: MarkSource::Hand,
            };
            points.ped_r = PedicleMark::NotVisible {
                source: MarkSource::Hand,
            };
        }
    }
    input.fit = joint_fit::fit_from_films(&FitFromFilms {
        front: films[0].clone(),
        side: None,
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    input.front = Some(LayoutFilm {
        marks: films[0].marks.clone(),
        geometry: films[0].geometry.clone(),
        size: films[0].size,
    });
    input.bend_left = None;
    input.bend_right = None;
    assert_eq!(input.fit.lengths, FitLengths::TypicalSize);
    let p = propose_layouts(&input).unwrap();
    check_proposal(&input, &p);
    assert_eq!(p.tier, FitTier::FrontFilmOnly);
    assert!(!p.layouts.is_empty());
    assert!(
        p.cannot_tell
            .iter()
            .any(|c| c.code == CannotTellCode::NoSideFilm)
    );
    assert!(
        p.cannot_tell
            .iter()
            .any(|c| c.code == CannotTellCode::TwistNotMeasured)
    );
    for a in &p.layouts[0].anchors {
        assert_eq!(a.entry_status, ValueStatus::Typical);
    }
}
#[test]
fn two_generated_films_run_through_the_joint_fit() {
    let (mut input, films) = fixture(RecipeName::RightThoracic40);
    input.fit = joint_fit::fit_from_films(&FitFromFilms {
        front: films[0].clone(),
        side: Some(films[1].clone()),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    let p = propose_layouts(&input).unwrap();
    check_proposal(&input, &p);
    assert_eq!(p.tier, FitTier::TwoFilms);
    assert!(!p.layouts.is_empty());
}
#[test]
#[ignore = "fits both film tiers at 32 versions; run explicitly in release"]
fn the_forty_degree_curve_keeps_its_narrow_looks_on_the_counted_pedicles() {
    let (mut input, films) = fixture(RecipeName::RightThoracic40);
    // The command-line recipe supplies no age and uses 32 versions at seed 1.
    input.child.age_years = None;
    for front_only in [false, true] {
        input.fit = joint_fit::fit_from_films(&FitFromFilms {
            front: films[0].clone(),
            side: (!front_only).then(|| films[1].clone()),
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        })
        .unwrap();
        let p = propose_layouts(&input).unwrap();
        check_proposal(&input, &p);
        assert_eq!(
            p.tier,
            if front_only {
                FitTier::FrontFilmOnly
            } else {
                FitTier::TwoFilms
            }
        );
        let first = &p.layouts[0];
        // Front only, 28 of 32 versions cannot read the end rules (no side
        // film). The other four split 2-2 between T5 and T6 and 2-2 between
        // T11 and T12; the cranial mode tie-break gives T5, and T5-T12 beats
        // T5-T11 (one holding version each) on the caudal tie-break.
        // One holding version now uses the main-curve sketch at T5-T12.
        assert_eq!(first.anchors.len(), if front_only { 16 } else { 14 });
        assert_eq!(
            (first.upper, first.lower),
            if front_only {
                (Level::T5, Level::T12)
            } else {
                (Level::T5, Level::T11)
            }
        );
        assert_eq!(p.first_is_sketch, front_only);
        if front_only {
            assert!(p.upper_ends.is_empty() && p.lower_ends.is_empty());
            assert!(
                !p.cannot_tell
                    .iter()
                    .any(|c| c.code == CannotTellCode::EndsUnsettled)
            );
        }
        let mut expected = vec![
            (Level::T7, BodySide::Left),
            (Level::T8, BodySide::Left),
            (Level::T9, BodySide::Left),
        ];
        expected.insert(0, (Level::T5, BodySide::Left));
        if !front_only {
            expected.insert(1, (Level::T5, BodySide::Right));
        }
        assert_eq!(
            first
                .anchors
                .iter()
                .filter(|a| a.looks.contains(&LookCode::MayBeTooNarrow))
                .map(|a| (a.level, a.side))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn exact_lumbar_poses_keep_end_plus_one_at_l4_in_every_version() {
    let (input, _) = fixture(RecipeName::LumbarLeft30);
    let reading = implant_layout::level_rules::measure(&input).unwrap();
    let structural = reading
        .regions
        .iter()
        .find(|r| r.region == implant_layout::level_rules::FusionRegion::Structural)
        .unwrap();
    let end = structural
        .votes
        .iter()
        .find(|v| v.rule == shapes::implant_layout::LevelRule::EndPlusOne)
        .unwrap();
    // The exact lumbar curve's lower end is L3; adding one anatomical
    // level gives L4. With zero pose spread every version gives that answer.
    assert!(end.counts.unread.is_empty());
    assert_eq!(
        end.counts.levels,
        vec![shapes::spine_readings::LevelCount {
            level: Level::L4,
            versions: input.versions,
        }]
    );
}

#[test]
fn every_field_family_keeps_its_status_source_or_absence_code() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.naming = NamingDoubt::MayBeOneOff;
    input.bend_left = None;
    input.bend_right = None;
    let p = propose_layouts(&input).unwrap();
    check_proposal(&input, &p);
    assert_eq!(
        p.naming_scenarios
            .iter()
            .map(|s| s.shift)
            .collect::<Vec<_>>(),
        vec![-1, 1]
    );
    for scenario in &p.naming_scenarios {
        for c in &scenario.changes {
            assert_ne!(c.level, c.renamed_as);
            for mm in [
                c.diameter_from_mm,
                c.diameter_to_mm,
                c.length_from_mm,
                c.length_to_mm,
            ]
            .into_iter()
            .flatten()
            {
                assert!(mm.is_finite() && mm > 0.0);
            }
        }
    }
}
#[test]
fn all_placements_mirror_with_the_spine() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.front = None;
    input.bend_left = None;
    input.bend_right = None;
    let mut mirror = input.clone();
    for v in &mut mirror.fit.vertebrae {
        v.pose.centre[1] = -v.pose.centre[1];
        v.pose.rotation[0] = -v.pose.rotation[0];
        v.pose.rotation[2] = -v.pose.rotation[2];
        v.pose.wedge_front = -v.pose.wedge_front;
        v.angles.side_tilt_rad = -v.angles.side_tilt_rad;
        v.angles.twist_rad = -v.angles.twist_rad;
    }
    let a = propose_layouts(&input).unwrap();
    let b = propose_layouts(&mirror).unwrap();
    assert_eq!(a.layouts.len(), b.layouts.len());
    for a in &a.layouts {
        let b = b
            .layouts
            .iter()
            .find(|b| b.id == a.id)
            .expect("the same change exists under reflection");
        assert_eq!((a.upper, a.lower), (b.upper, b.lower));
        for x in &a.anchors {
            let y = b
                .anchors
                .iter()
                .find(|y| y.level == x.level && y.side != x.side)
                .unwrap();
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.diameter_mm, y.diameter_mm);
            assert_eq!(x.length_mm, y.length_mm);
            for (i, sign) in [1.0, -1.0, 1.0].into_iter().enumerate() {
                assert!((x.entry_mm[i] - sign * y.entry_mm[i]).abs() < 1e-8);
                assert!((x.direction_unit[i] - sign * y.direction_unit[i]).abs() < 1e-8);
            }
        }
        for x in &a.rods {
            let y = b.rods.iter().find(|y| y.side != x.side).unwrap();
            assert_eq!(x.points_mm.len(), y.points_mm.len());
            for (p, q) in x.points_mm.iter().zip(&y.points_mm) {
                for (i, sign) in [1.0, -1.0, 1.0].into_iter().enumerate() {
                    assert!((p[i] - sign * q[i]).abs() < 1e-7);
                }
            }
        }
    }
}

#[test]
fn a_marked_sacrum_remains_readable_at_the_fits_typical_scale() {
    let (mut input, mut films) = fixture(RecipeName::RightThoracic40);
    films[0].geometry.mm_per_px = None;
    films[0].geometry.mm_per_px_vertical = None;
    films[0].geometry.scale = None;
    input.fit = joint_fit::fit_from_films(&FitFromFilms {
        front: films[0].clone(),
        side: None,
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    input.front = Some(LayoutFilm {
        marks: films[0].marks.clone(),
        geometry: films[0].geometry.clone(),
        size: films[0].size,
    });
    // A marked line and a body drawn at the same provisional size support
    // body-to-body comparisons. They cannot support millimetre thresholds.
    let reading = curves::measure(&input, &input.fit).unwrap();
    assert!(reading.csvl_y_mm.is_ok(), "a marked sacrum is present");
}

#[test]
fn an_odd_key_level_cap_is_independent_of_physical_side() {
    let (input, _) = fixture(RecipeName::RightThoracic40);
    let readings = implant_layout::readings::readings(&input, 0..input.versions).unwrap();
    let levels = [Level::T5, Level::T6, Level::T7, Level::T8];
    let mut first = implant_layout::screws::anchors(
        &input,
        &readings.versions.versions,
        &readings.measured,
        &levels,
    )
    .unwrap();
    // Synthetic occupied heads isolate pattern handedness from table refusals.
    for a in &mut first {
        a.diameter_mm = Some(4.0);
        a.length_mm = Some(25.0);
        a.direction_status = ValueStatus::FromTables;
    }
    let mut mirrored = first.clone();
    implant_layout::patterns::apply(
        &mut first,
        &levels,
        Some(Level::T6),
        implant_layout::patterns::Pattern::KeyLevels,
    )
    .unwrap();
    implant_layout::patterns::apply(
        &mut mirrored,
        &levels,
        Some(Level::T6),
        implant_layout::patterns::Pattern::KeyLevels,
    )
    .unwrap();
    for a in &first {
        let reflected = mirrored
            .iter()
            .find(|b| b.level == a.level && b.side != a.side)
            .unwrap();
        assert_eq!(
            a.kind, reflected.kind,
            "a partial pair must swap physical side"
        );
    }
}

#[test]
fn the_no_film_scale_reason_crosses_the_contract() {
    let code: CannotTellCode = serde_json::from_str("\"no-film-scale\"")
        .expect("the dimensional-rule refusal has a contract code");
    assert_eq!(serde_json::to_string(&code).unwrap(), "\"no-film-scale\"");
}

#[test]
fn an_absent_width_has_missing_provenance() {
    let estimate =
        implant_layout::pedicles::estimate(&[None], shapes::implant_layout::CurveSide::Outside);
    assert_eq!(estimate.outer_width_mm, None);
    assert_eq!(estimate.inner_width_mm, None);
    assert_eq!(estimate.status, ValueStatus::Missing);
}

#[test]
#[ignore = "fits a full front film; run explicitly in release"]
fn a_front_only_fit_keeps_a_drawable_placement() {
    let (mut input, films) = fixture_seed(RecipeName::RightThoracic40, 2);
    input.fit = joint_fit::fit_from_films(&FitFromFilms {
        front: films[0].clone(),
        side: None,
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    input.child.age_years = None;
    let p = propose_layouts(&input).unwrap();
    assert!(!p.layouts.is_empty(), "{:?}; {:?}", p.cannot_tell, p.curves);
}

fn hand_input(input: &LayoutInput, first: &ImplantLayout) -> shapes::implant_layout::RebuildInput {
    use shapes::implant_layout::{AnchorChoice, RebuildInput};
    RebuildInput {
        layout: input.clone(),
        upper: first.upper,
        lower: first.lower,
        anchors: first
            .anchors
            .iter()
            .map(|a| AnchorChoice {
                level: a.level,
                side: a.side,
                kind: a.kind,
                diameter_mm: a.diameter_mm,
                length_mm: a.length_mm,
            })
            .collect(),
        rod_material: first.rods[0].material,
        rod_diameter_mm: first.rods[0].diameter_mm,
    }
}

#[test]
fn rod_pair_choices_are_exactly_the_pairs_that_can_be_rebuilt() {
    use shapes::errors::{EngineError, ErrorCode};
    use shapes::implant_layout::RodMaterial;
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 1;
    let proposals = propose_layouts(&input).unwrap();
    let mut request = hand_input(&input, &proposals.layouts[0]);
    assert!(!proposals.choices.rods.is_empty());
    assert_eq!(proposals.choices.rods.len(), rods::choices().len());
    for (choice, sourced) in proposals.choices.rods.iter().zip(rods::choices()) {
        assert_eq!(choice.material, sourced.material);
        assert_eq!(choice.diameter_mm, sourced.diameter_mm);
        request.rod_material = choice.material;
        request.rod_diameter_mm = choice.diameter_mm;
        let rebuilt = implant_layout::rebuild_layout(&request).unwrap();
        assert_eq!(rebuilt.rods.len(), 2);
        for rod in &rebuilt.rods {
            assert_eq!(rod.material, choice.material);
            assert_eq!(rod.diameter_mm, choice.diameter_mm);
        }
    }
    for &material in &proposals.choices.rod_materials {
        for &diameter_mm in &proposals.choices.rod_diameters_mm {
            if !proposals
                .choices
                .rods
                .iter()
                .any(|rod| rod.material == material && rod.diameter_mm == diameter_mm)
            {
                request.rod_material = material;
                request.rod_diameter_mm = diameter_mm;
                let error =
                    EngineError::from(implant_layout::rebuild_layout(&request).unwrap_err());
                assert_eq!(error.code, ErrorCode::CaseInvalidValue);
                assert!(error.message.contains("rod-not-in-choices"));
            }
        }
    }
    request.rod_material = RodMaterial::StainlessSteel;
    request.rod_diameter_mm = 5.5;
    assert!(!proposals.choices.rods.iter().any(|rod| {
        rod.material == request.rod_material && rod.diameter_mm == request.rod_diameter_mm
    }));
    let error = EngineError::from(implant_layout::rebuild_layout(&request).unwrap_err());
    assert_eq!(error.code, ErrorCode::CaseInvalidValue);
    assert!(error.message.contains("rod-not-in-choices"));
}

#[test]
fn hand_choices_reproduce_the_first_placement_and_its_draws() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let p = propose_layouts(&input).unwrap();
    assert!(!p.first_is_sketch);
    assert!(!p.upper_ends.is_empty() && !p.lower_ends.is_empty());
    let first = p.layouts.into_iter().next().unwrap();
    let mut rebuilt = implant_layout::rebuild_layout(&hand_input(&input, &first)).unwrap();
    assert_eq!(rebuilt.id, "changed-by-hand");
    assert_eq!(rebuilt.change, LayoutChange::ChangedByHand {});
    rebuilt.id = first.id.clone();
    rebuilt.change = first.change;
    assert_eq!(rebuilt, first);
}

#[test]
fn moving_an_end_by_hand_adds_complete_anchor_and_rod_data() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let p = propose_layouts(&input).unwrap();
    let first = &p.layouts[0];
    let mut request = hand_input(&input, first);
    request.anchors.clear();
    let lower = input
        .fit
        .vertebrae
        .iter()
        .position(|v| v.level == first.lower)
        .unwrap()
        + 1;
    request.lower = input.fit.vertebrae[lower].level;
    let rebuilt = implant_layout::rebuild_layout(&request).unwrap();
    for anchor in rebuilt.anchors.iter().filter(|a| a.level == request.lower) {
        assert_eq!(anchor.kind, AnchorKind::Screw);
        assert!(anchor.diameter_mm.is_some() && anchor.length_mm.is_some());
        assert!(anchor.screw_here.is_some());
        assert!(!anchor.wider_than_pedicle.is_empty());
        assert_eq!(anchor.look_sources.len(), anchor.looks.len());
        sources(&anchor.sources);
        sources(&anchor.pedicle.sources);
    }
    assert!(
        rebuilt
            .rods
            .iter()
            .all(|r| r.points_mm.len() == r.standing_points_mm.len())
    );
    assert!(!rebuilt.released_levels.contains(&request.lower));
}

#[test]
fn hand_sizes_keep_the_ratio_direction_and_population_counts() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let p = propose_layouts(&input).unwrap();
    let first = &p.layouts[0];
    let mut request = hand_input(&input, first);
    for anchor in &mut request.anchors {
        anchor.diameter_mm = None;
        anchor.length_mm = None;
    }
    let index = first
        .anchors
        .iter()
        .position(|a| a.kind == AnchorKind::Screw)
        .unwrap();
    let original = &first.anchors[index];
    let diameter = *p.choices.screw_diameters_mm.last().unwrap();
    assert_ne!(Some(diameter), original.diameter_mm);
    let length = *p
        .choices
        .screw_lengths_mm
        .iter()
        .find(|&&mm| Some(mm) != original.length_mm)
        .unwrap();
    request.anchors[index].diameter_mm = Some(diameter);
    request.anchors[index].length_mm = Some(length);
    // The catalogue floor gives every pedicle with a full width ensemble a
    // table size, so a second pedicle is given sizes other than its own.
    let other = (0..first.anchors.len())
        .find(|&i| i != index && first.anchors[i].screw_here.is_some())
        .unwrap();
    let table = first.anchors[other].screw_here.clone().unwrap();
    request.anchors[other].kind = AnchorKind::Screw;
    request.anchors[other].diameter_mm = p
        .choices
        .screw_diameters_mm
        .iter()
        .copied()
        .find(|&mm| mm != table.diameter_mm);
    request.anchors[other].length_mm = p
        .choices
        .screw_lengths_mm
        .iter()
        .copied()
        .find(|&mm| mm != table.length_mm);
    let rebuilt = implant_layout::rebuild_layout(&request).unwrap();
    let changed = &rebuilt.anchors[index];
    assert_eq!(changed.diameter_mm, Some(diameter));
    assert_eq!(changed.length_mm, Some(length));
    assert_eq!(changed.screw_here, original.screw_here);
    assert_eq!(changed.diameter_status, ValueStatus::Entered);
    assert_eq!(changed.length_status, ValueStatus::Entered);
    assert!(changed.diameter_sources.is_empty() && changed.length_sources.is_empty());
    assert!(changed.diameter_also_seen.is_empty() && changed.length_also_seen.is_empty());
    assert_eq!(changed.wider_than_pedicle, original.wider_than_pedicle);
    assert_eq!(changed.direction_unit, original.direction_unit);
    let expected =
        original.screw_to_pedicle.unwrap().p50 * diameter / original.diameter_mm.unwrap();
    assert!((changed.screw_to_pedicle.unwrap().p50 - expected).abs() < 1e-12);
    assert!(
        changed
            .looks
            .contains(&shapes::implant_layout::LookCode::MayBeTooNarrow)
    );
    assert_eq!(changed.look_sources.len(), changed.looks.len());
    let filled = &rebuilt.anchors[other];
    assert_eq!(filled.kind, AnchorKind::Screw);
    assert!(filled.medial_angle.is_some() && filled.screw_to_pedicle.is_some());
    assert_eq!(filled.screw_here, Some(table));
    assert_eq!(filled.diameter_status, ValueStatus::Entered);
    assert_eq!(filled.length_status, ValueStatus::Entered);
    assert!(filled.diameter_sources.is_empty() && filled.length_sources.is_empty());
    assert!(filled.diameter_also_seen.is_empty() && filled.length_also_seen.is_empty());
}

#[test]
fn hooks_and_empty_choices_keep_the_table_size_and_its_sources() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let first = propose_layouts(&input).unwrap().layouts.remove(0);
    let mut request = hand_input(&input, &first);
    let mut screws = first
        .anchors
        .iter()
        .enumerate()
        .filter(|(_, a)| a.kind == AnchorKind::Screw);
    let indices = [screws.next().unwrap().0, screws.next().unwrap().0];
    for (index, kind) in indices
        .into_iter()
        .zip([AnchorKind::Hook, AnchorKind::None])
    {
        request.anchors[index].kind = kind;
        request.anchors[index].diameter_mm = None;
        request.anchors[index].length_mm = None;
    }
    let rebuilt = implant_layout::rebuild_layout(&request).unwrap();
    for index in indices {
        let original = &first.anchors[index];
        let changed = &rebuilt.anchors[index];
        assert_eq!(changed.kind, request.anchors[index].kind);
        assert!(changed.diameter_mm.is_none() && changed.length_mm.is_none());
        assert!(changed.screw_here.is_some());
        assert_eq!(changed.screw_here, original.screw_here);
        assert_eq!(changed.diameter_sources, original.diameter_sources);
        assert_eq!(changed.length_sources, original.length_sources);
        sources(&changed.diameter_sources);
        sources(&changed.length_sources);
    }
}

#[test]
fn hand_choices_refuse_unfitted_crossed_short_and_off_grid_inputs() {
    use shapes::errors::{EngineError, ErrorCode};
    use shapes::implant_layout::RodMaterial;
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let first = propose_layouts(&input).unwrap().layouts.remove(0);
    let request = hand_input(&input, &first);
    let mut cases = Vec::new();
    let mut add = |f: fn(&mut shapes::implant_layout::RebuildInput)| {
        let mut case = request.clone();
        f(&mut case);
        cases.push(case);
    };
    add(|r| r.upper = Level::C1);
    add(|r| r.lower = Level::L7);
    add(|r| r.lower = r.upper);
    add(|r| std::mem::swap(&mut r.upper, &mut r.lower));
    add(|r| r.anchors[0].level = Level::C1);
    add(|r| r.anchors[0].diameter_mm = Some(4.1));
    add(|r| r.anchors[0].length_mm = Some(26.0));
    add(|r| r.anchors[0].diameter_mm = Some(f64::NAN));
    add(|r| {
        r.anchors[0].kind = AnchorKind::Hook;
        r.anchors[0].diameter_mm = Some(4.0);
        r.anchors[0].length_mm = None;
    });
    add(|r| {
        r.anchors[0].kind = AnchorKind::None;
        r.anchors[0].diameter_mm = None;
        r.anchors[0].length_mm = Some(25.0);
    });
    add(|r| r.rod_diameter_mm = 5.6);
    add(|r| r.rod_material = RodMaterial::StainlessSteel);
    add(|r| r.anchors.push(r.anchors[0]));
    for side in [BodySide::Left, BodySide::Right] {
        let mut case = request.clone();
        let mut occupied = false;
        for a in case.anchors.iter_mut().filter(|a| a.side == side) {
            a.kind = if occupied {
                AnchorKind::None
            } else {
                AnchorKind::Hook
            };
            a.diameter_mm = None;
            a.length_mm = None;
            occupied = true;
        }
        cases.push(case);
    }
    for case in cases {
        let error = EngineError::from(implant_layout::rebuild_layout(&case).unwrap_err());
        assert_eq!(error.code, ErrorCode::CaseInvalidValue, "{}", error.message);
    }
}

#[test]
fn fitted_cervical_extra_lumbar_and_gapped_spans_are_refused() {
    use shapes::errors::{EngineError, ErrorCode};
    let (mut input, _) = fixture(RecipeName::SixLumbar);
    input.versions = 1;
    let first = propose_layouts(&input).unwrap().layouts.remove(0);
    let mut request = hand_input(&input, &first);
    request.anchors.clear();
    let mut cervical = request.layout.fit.vertebrae[0].clone();
    cervical.level = Level::C7;
    cervical.pose.level = Level::C7;
    cervical.pose.centre[2] += cervical.pose.height;
    request.layout.fit.vertebrae.insert(0, cervical);
    assert!(
        request
            .layout
            .fit
            .vertebrae
            .iter()
            .any(|v| v.level == Level::L6)
    );
    let mut cases = Vec::new();
    let mut cervical = request.clone();
    cervical.upper = Level::C7;
    cases.push(cervical);
    let mut extra_lumbar = request.clone();
    extra_lumbar.lower = Level::L6;
    cases.push(extra_lumbar);
    request.upper = Level::T5;
    request.lower = Level::T8;
    request
        .layout
        .fit
        .vertebrae
        .retain(|v| v.level != Level::T6);
    cases.push(request);
    for case in cases {
        assert!(
            case.layout
                .fit
                .vertebrae
                .iter()
                .any(|v| v.level == case.upper)
        );
        assert!(
            case.layout
                .fit
                .vertebrae
                .iter()
                .any(|v| v.level == case.lower)
        );
        let error = EngineError::from(implant_layout::rebuild_layout(&case).unwrap_err());
        assert_eq!(error.code, ErrorCode::CaseInvalidValue);
        assert!(error.message.contains("instrumented-span-not-fitted"));
    }
}

#[test]
fn rods_turn_smoothly_where_a_hook_meets_a_screw() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let proposals = propose_layouts(&input).unwrap();
    assert!(
        proposals.layouts.iter().any(hook_beside_screw),
        "the made-up placements include a hook beside a screw"
    );
    proposals.layouts.iter().for_each(rods_turn_smoothly);
    // By hand: hooks at the top of both rods and one in the middle of
    // each, so every hook sits between screws or above one.
    let first = &proposals.layouts[0];
    let mut request = hand_input(&input, first);
    for side in [BodySide::Left, BodySide::Right] {
        let screws: Vec<_> = (0..request.anchors.len())
            .filter(|&i| {
                request.anchors[i].side == side && request.anchors[i].kind == AnchorKind::Screw
            })
            .collect();
        for i in [screws[0], screws[screws.len() / 2]] {
            let anchor = &mut request.anchors[i];
            anchor.kind = AnchorKind::Hook;
            anchor.diameter_mm = None;
            anchor.length_mm = None;
        }
    }
    let hooked = implant_layout::rebuild_layout(&request).unwrap();
    assert!(hook_beside_screw(&hooked));
    rods_turn_smoothly(&hooked);
    // A hook seats where the screw it replaces did, so neither line moves.
    for (rod, before) in hooked.rods.iter().zip(&first.rods) {
        assert_eq!(rod.side, before.side);
        for (after, before) in [
            (&rod.points_mm, &before.points_mm),
            (&rod.standing_points_mm, &before.standing_points_mm),
        ] {
            assert_eq!(after.len(), before.len());
            for (p, q) in after.iter().zip(before) {
                assert!((Vector3::from(*p) - Vector3::from(*q)).norm() < 1e-9);
            }
        }
    }
}

/// The fit and every film keep only the vertebrae `keep` accepts, as when
/// a film is cut off or a vertebra is not counted.
fn keep_levels(input: &mut LayoutInput, keep: impl Fn(Level) -> bool) {
    input.fit.vertebrae.retain(|v| keep(v.level));
    for film in [
        &mut input.front,
        &mut input.bend_left,
        &mut input.bend_right,
    ]
    .into_iter()
    .flatten()
    {
        film.marks.vertebrae.retain(|v| keep(v.level));
    }
}

fn typical_rods_without_a_reading(layout: &ImplantLayout) {
    for rod in &layout.rods {
        assert_eq!(rod.target_kyphosis_status, ValueStatus::Typical);
        assert_eq!(rod.target_kyphosis_deg, 0.0);
    }
}

#[test]
fn a_fit_without_t5_or_t12_still_gets_a_placement() {
    for (name, keep) in [
        (
            "cut off above T6",
            (|l| l >= Level::T6) as fn(Level) -> bool,
        ),
        ("no T12", |l| l != Level::T12),
    ] {
        let (mut input, _) = fixture(RecipeName::RightThoracic40);
        input.versions = 8;
        keep_levels(&mut input, keep);
        let p = propose_layouts(&input).unwrap();
        assert!(!p.layouts.is_empty(), "{name}: {:?}", p.cannot_tell);
        for layout in &p.layouts {
            check_layout(&input, layout);
            typical_rods_without_a_reading(layout);
        }
        let mut request = hand_input(&input, &p.layouts[0]);
        request.anchors.clear();
        (request.upper, request.lower) = (Level::L1, Level::L4);
        typical_rods_without_a_reading(&implant_layout::rebuild_layout(&request).unwrap());
    }
}

#[test]
fn a_bending_film_counted_on_its_own_keeps_the_placements() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 8;
    input.naming = NamingDoubt::MayBeOneOff;
    let usual = propose_layouts(&input).unwrap();
    assert_eq!(usual.naming_scenarios.len(), 2);
    // The left bending film counts an extra thoracic: its L1 to L5 are
    // named T13 and L1 to L4, which the front's count has no name for.
    let mut extra = input.clone();
    let bend = &mut extra.bend_left.as_mut().unwrap().marks;
    bend.naming.template = Template::ExtraThoracic;
    for v in &mut bend.vertebrae {
        v.level = match v.level {
            Level::L1 => Level::T13,
            Level::L2 => Level::L1,
            Level::L3 => Level::L2,
            Level::L4 => Level::L3,
            Level::L5 => Level::L4,
            level => level,
        };
    }
    let p = propose_layouts(&extra).unwrap();
    assert!(!p.layouts.is_empty());
    assert_eq!(
        p.naming_scenarios.len(),
        2,
        "each film moves along its own count"
    );
    // A bending film whose lowest name ends the list: one step down has no
    // name, so only the renaming details are lost, never the placements.
    let mut last = input.clone();
    let lowest = last.bend_left.as_mut().unwrap().marks.vertebrae.last_mut();
    let lowest = lowest.unwrap();
    assert_eq!(lowest.level, Level::L5);
    assert_eq!(Level::ALL.last(), Some(&Level::L7));
    lowest.level = Level::L7;
    let p = propose_layouts(&last).unwrap();
    assert!(!p.layouts.is_empty());
    assert!(p.naming_scenarios.is_empty());
}

#[test]
fn a_hand_hook_without_a_table_direction_is_refused() {
    use shapes::errors::{EngineError, ErrorCode};
    use shapes::implant_layout::AnchorChoice;
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 1;
    let first = propose_layouts(&input).unwrap().layouts.remove(0);
    let mut request = hand_input(&input, &first);
    request.layout.child.age_years = Some(10.0);
    request.upper = Level::T5;
    request.lower = Level::L2;
    request.anchors = vec![AnchorChoice {
        level: Level::L1,
        side: BodySide::Left,
        kind: AnchorKind::Hook,
        diameter_mm: None,
        length_mm: None,
    }];
    let error = EngineError::from(implant_layout::rebuild_layout(&request).unwrap_err());
    assert_eq!(error.code, ErrorCode::CaseInvalidValue);
    assert!(error.message.contains("hook-direction-missing"));
}

#[test]
fn a_sketch_keeps_the_rule_end_counts_empty() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 1;
    let mut readings = implant_layout::proposal::measure(&input).unwrap();
    readings.regions.clear();
    let major = readings.nominal.major.unwrap();
    readings.nominal.curves[major].upper_index = 9;
    readings.nominal.curves[major].lower_index = 10;
    let sketch = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(!sketch.layouts.is_empty());
    assert!(sketch.first_is_sketch);
    assert!(sketch.upper_ends.is_empty() && sketch.lower_ends.is_empty());
    assert!(
        !sketch
            .cannot_tell
            .iter()
            .any(|c| c.code == CannotTellCode::EndsUnsettled)
    );
}

/// The entries were lined up over each span alone, so moving one end, in
/// a placement beside the first or by hand, slid the screws of levels the
/// change never touched. They are lined up over the whole fitted column.
#[test]
fn moving_an_end_never_moves_a_shared_entry() {
    let (mut input, _) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    let proposals = propose_layouts(&input).unwrap();
    let first = &proposals.layouts[0];
    let entries = |layout: &ImplantLayout| -> Vec<_> {
        layout
            .anchors
            .iter()
            .map(|a| (a.level, a.side, a.entry_mm))
            .collect()
    };
    let shared = |a: &ImplantLayout, b: &ImplantLayout| {
        let (a, b) = (entries(a), entries(b));
        let common: Vec<_> = a
            .iter()
            .filter(|x| b.iter().any(|y| (y.0, y.1) == (x.0, x.1)))
            .collect();
        assert!(common.len() >= 4);
        for x in common {
            assert_eq!(Some(x), b.iter().find(|y| (y.0, y.1) == (x.0, x.1)));
        }
    };
    let moved_ends: Vec<_> = proposals
        .layouts
        .iter()
        .filter(|l| (l.upper, l.lower) != (first.upper, first.lower))
        .collect();
    assert_eq!(moved_ends.len(), 4);
    for layout in moved_ends {
        shared(first, layout);
        let mut request = hand_input(&input, first);
        request.anchors.clear();
        (request.upper, request.lower) = (layout.upper, layout.lower);
        shared(first, &implant_layout::rebuild_layout(&request).unwrap());
    }
}

// FNV-1a of complete serialised outputs, captured before the measurement split.
fn serialised_fingerprint(value: &impl serde::Serialize) -> (usize, u64) {
    let bytes = serde_json::to_vec(value).unwrap();
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    (bytes.len(), hash)
}

fn hand_change_fingerprints(front_only: bool, fitted: bool) -> [(usize, u64); 8] {
    let (mut input, films) = fixture(RecipeName::RightThoracic40);
    input.versions = 4;
    if fitted {
        input.bend_left = None;
        input.bend_right = None;
        input.fit = joint_fit::fit_from_films(&FitFromFilms {
            front: films[0].clone(),
            side: (!front_only).then(|| films[1].clone()),
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        })
        .unwrap();
    }
    if front_only && !fitted {
        input.fit.side_film = None;
        input.bend_left = None;
        input.bend_right = None;
    }
    let proposals = propose_layouts(&input).unwrap();
    assert_eq!(
        proposals.tier,
        if front_only {
            FitTier::FrontFilmOnly
        } else {
            FitTier::TwoFilms
        }
    );
    let first = &proposals.layouts[0];
    let original = hand_input(&input, first);
    let mut requests = vec![original.clone()];
    // Keep every retained hand choice; added levels use their table choices.
    for (upper, offset) in [(true, -1_isize), (true, 1), (false, -1), (false, 1)] {
        let mut request = original.clone();
        let end = if upper { first.upper } else { first.lower };
        let index = input
            .fit
            .vertebrae
            .iter()
            .position(|v| v.level == end)
            .unwrap();
        let shifted = input.fit.vertebrae[index.checked_add_signed(offset).unwrap()].level;
        if upper {
            request.upper = shifted;
        } else {
            request.lower = shifted;
        }
        request
            .anchors
            .retain(|a| request.upper <= a.level && a.level <= request.lower);
        requests.push(request);
    }
    let screw = original
        .anchors
        .iter()
        .position(|a| a.kind == AnchorKind::Screw)
        .unwrap();
    let mut kind = original.clone();
    kind.anchors[screw].kind = AnchorKind::Hook;
    kind.anchors[screw].diameter_mm = None;
    kind.anchors[screw].length_mm = None;
    requests.push(kind);
    let mut size = original;
    let anchor = &mut size.anchors[screw];
    anchor.diameter_mm = Some(
        sizing::diameter_grid()
            .into_iter()
            .find(|g| Some(g.mm) != anchor.diameter_mm)
            .unwrap()
            .mm,
    );
    anchor.length_mm = Some(
        sizing::length_grid()
            .into_iter()
            .find(|g| Some(g.mm) != anchor.length_mm)
            .unwrap()
            .mm,
    );
    requests.push(size);
    let mut fingerprints = vec![serialised_fingerprint(&proposals)];
    fingerprints.extend(
        requests.iter().map(|request| {
            serialised_fingerprint(&implant_layout::rebuild_layout(request).unwrap())
        }),
    );
    fingerprints.try_into().unwrap()
}

#[test]
#[ignore = "fits a full front film; run explicitly in release"]
fn front_only_hand_changes_keep_their_serialised_bytes() {
    assert_eq!(
        hand_change_fingerprints(true, true),
        [
            (611587, 13774154800999321479),
            (63888, 17619063237871066832),
            (71954, 12139043153922690622),
            (55740, 7511198657723376360),
            (55679, 988401265798016133),
            (72443, 13537662036664151598),
            (63760, 5207833755706780307),
            (63655, 4332988658384256470),
        ]
    );
}

#[test]
#[ignore = "fits two full films; run explicitly in release"]
fn two_film_hand_changes_keep_their_serialised_bytes() {
    assert_eq!(
        hand_change_fingerprints(false, true),
        [
            (557671, 8915511845704622265),
            (57750, 343529669024308921),
            (66168, 2774808426912564653),
            (49601, 16932612553952717781),
            (49555, 5229104112533522712),
            (66386, 14119666237342783203),
            (57622, 14529783802682069550),
            (57517, 16003584600432466489),
        ]
    );
}

#[test]
fn exact_pose_hand_changes_keep_their_serialised_bytes() {
    assert_eq!(
        hand_change_fingerprints(false, false),
        [
            (634777, 6399587130604658601),
            (66078, 315608697238697520),
            (74464, 2817601403946281840),
            (58064, 8181873827660707217),
            (57458, 10711073004944921213),
            (74260, 8389079364235742548),
            (65945, 2509951005274521112),
            (65900, 16979981033600005629),
        ]
    );
}

#[test]
fn exact_pose_front_only_hand_changes_keep_their_serialised_bytes() {
    assert_eq!(
        hand_change_fingerprints(true, false),
        [
            (619883, 11644877264924909714),
            (65237, 995587915205777560),
            (73634, 7053437676410274103),
            (57238, 8516466936557249277),
            (56713, 4974182565430281113),
            (73446, 13553452555020867349),
            (65104, 6704329383720397792),
            (65059, 16863788549520358919),
        ]
    );
}

/// Built at assembly from made-up readings, rather than fitted: only the
/// first N versions retain their regional modal rules.
fn sparse_rules(holding: usize) -> (LayoutInput, implant_layout::level_rules::LevelReadings) {
    let (input, _) = fixture(RecipeName::RightThoracic40);
    let mut readings = implant_layout::proposal::measure(&input).unwrap();
    for region in &mut readings.regions {
        for rules in region.version_rules.iter_mut().skip(holding) {
            rules.clear();
        }
    }
    (input, readings)
}

#[test]
fn a_tenth_keeps_the_rule_proposal_bytes() {
    let (input, readings) = sparse_rules(4);
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert_eq!(
        serialised_fingerprint(&result),
        (637684, 3989943222286645900)
    );
    assert!(!result.first_is_sketch);
    let first = &result.layouts[0];
    assert_eq!((first.upper, first.lower), (Level::T5, Level::T12));
    assert_eq!(
        result
            .pairs
            .iter()
            .find(|p| p.upper == first.upper && p.lower == first.lower)
            .unwrap()
            .versions_holding,
        4
    );
}

/// The rules hold T6 to T12 in every version. A factor on the standing
/// sagittal angles bent this target past a quarter turn and refused it, so
/// the placement fell to T5 to T12, held by no version, after every sketch
/// was refused the same way. One shared added angle builds the rule pair.
#[test]
fn the_rule_pair_a_sagittal_factor_refused_now_keeps_its_proposal_bytes() {
    let (input, _) = fixture_seed(RecipeName::DoubleMajor55To45, 4);
    let readings = implant_layout::proposal::measure(&input).unwrap();
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert_eq!(
        serialised_fingerprint(&result),
        (566274, 9493619728016298689)
    );
    assert!(!result.first_is_sketch);
    let first = &result.layouts[0];
    assert_eq!((first.upper, first.lower), (Level::T6, Level::T12));
    assert_eq!(
        result
            .pairs
            .iter()
            .find(|p| p.upper == first.upper && p.lower == first.lower)
            .unwrap()
            .versions_holding,
        input.versions
    );
    // Without the rules the sketch from the curve ends, which the factor
    // also refused span by span, now builds as well.
    let mut fallback = readings;
    fallback.regions.clear();
    let sketch = implant_layout::proposal::assemble(&input, &fallback).unwrap();
    assert!(sketch.first_is_sketch);
    assert!(!sketch.layouts.is_empty());
}

/// At ten the tables give no lumbar width, so the lumbar curve's sketches,
/// T12 to L4 and T12 to L5, hold one screw a side and are refused. Each
/// refusal drops that sketch and tries the next; with none left the rules'
/// thin pair stays, as a placement and not as a sketch.
#[test]
fn a_refused_sketch_keeps_the_rule_pair() {
    let (mut input, _) = fixture_seed(RecipeName::LumbarLeft30, 4);
    input.child.age_years = Some(10.0);
    let mut readings = implant_layout::proposal::measure(&input).unwrap();
    for region in &mut readings.regions {
        for rules in region.version_rules.iter_mut().skip(1) {
            rules.clear();
        }
    }
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(!result.first_is_sketch);
    let first = &result.layouts[0];
    assert_eq!((first.upper, first.lower), (Level::T11, Level::L3));
    let pair = result
        .pairs
        .iter()
        .find(|p| p.upper == first.upper && p.lower == first.lower)
        .unwrap();
    assert!(u64::from(pair.versions_holding) * 10 < u64::from(input.versions));
    assert!(!result.upper_ends.is_empty() && !result.lower_ends.is_empty());
    // The same sketches with no rules at all: each is refused, so nothing
    // is laid out.
    readings.regions.clear();
    let sketch = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(sketch.layouts.is_empty() && sketch.pairs.is_empty());
}

/// Built at assembly, not fitted: the regional rules hold in one version.
#[test]
fn sparse_rule_ends_use_the_same_sketch_as_absent_rules() {
    let (input, mut readings) = sparse_rules(1);
    let rules = implant_layout::pairs::build(&input, &readings).unwrap();
    assert_eq!(
        implant_layout::pairs::first_pair(&rules.pairs)
            .unwrap()
            .versions_holding,
        1
    );
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(result.first_is_sketch);
    assert!(result.upper_ends.is_empty() && result.lower_ends.is_empty());
    assert!(
        !result
            .cannot_tell
            .iter()
            .any(|c| c.code == CannotTellCode::EndsUnsettled)
    );
    let curve = &readings.nominal.curves[readings.nominal.major.unwrap()];
    let names = input.front.as_ref().unwrap().marks.naming.template.names();
    let lower = readings.nominal.vertebrae[curve.lower_index].level;
    let next = names[names.iter().position(|l| *l == lower).unwrap() + 1];
    assert_eq!(
        (result.layouts[0].upper, result.layouts[0].lower),
        (readings.nominal.vertebrae[curve.upper_index].level, next)
    );
    readings.regions.clear();
    let absent = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&absent).unwrap()
    );
}

/// Built at assembly, not fitted: with six lumbar vertebrae and no rule
/// pair, a main curve ending at L6 is sketched down to L5, not given up.
#[test]
fn a_sketch_holds_its_ends_within_t1_to_l5() {
    let (mut input, _) = fixture(RecipeName::SixLumbar);
    input.versions = 4;
    let mut readings = implant_layout::proposal::measure(&input).unwrap();
    readings.regions.clear();
    let major = readings
        .nominal
        .major
        .expect("the made-up spine has a curve");
    let l6 = readings
        .nominal
        .vertebrae
        .iter()
        .position(|v| v.level == Level::L6);
    readings.nominal.curves[major].lower_index = l6.expect("six lumbar vertebrae are fitted");
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(result.first_is_sketch, "{:?}", result.cannot_tell);
    let upper = readings.nominal.vertebrae[readings.nominal.curves[major].upper_index].level;
    assert_eq!(
        (result.layouts[0].upper, result.layouts[0].lower),
        (upper, Level::L5)
    );
    result.layouts.iter().for_each(|l| check_layout(&input, l));
}

/// Built at assembly, not fitted: the sketch adds T1, absent from one
/// version, while the rules' T5–T12 span and its alternatives remain readable.
#[test]
fn an_invalid_sketch_trial_keeps_the_drawable_rule_pair() {
    let (input, mut readings) = sparse_rules(4);
    let major = readings.nominal.major.unwrap();
    readings.nominal.curves[major].upper_index = 0;
    readings.versions.versions[0]
        .fit
        .vertebrae
        .retain(|v| v.level != Level::T1);
    let baseline = implant_layout::proposal::assemble(&input, &readings).unwrap();
    for region in &mut readings.regions {
        for rules in region.version_rules.iter_mut().skip(1) {
            rules.clear();
        }
    }
    let result = implant_layout::proposal::assemble(&input, &readings).unwrap();
    assert!(!result.first_is_sketch);
    assert_eq!(result.layouts, baseline.layouts);
    let mut expected = baseline;
    for pair in &mut expected.pairs {
        pair.versions_holding = pair.versions_holding.min(1);
    }
    for count in expected
        .upper_ends
        .iter_mut()
        .chain(&mut expected.lower_ends)
    {
        count.versions = count.versions.min(1);
    }
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    assert_eq!(
        (result.layouts[0].upper, result.layouts[0].lower),
        (Level::T5, Level::T12)
    );
    assert_eq!(
        result
            .pairs
            .iter()
            .find(|p| p.upper == Level::T5 && p.lower == Level::T12)
            .unwrap()
            .versions_holding,
        1
    );
    readings.regions.clear();
    assert!(matches!(
        implant_layout::proposal::assemble(&input, &readings),
        Err(implant_layout::LayoutError::InvalidValue { .. })
    ));
}
