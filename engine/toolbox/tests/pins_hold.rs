//! Known rectangular bodies make pin locations, neighbour distances and
//! the C7 centre independent of the fitting code.

use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason};
use shapes::errors::ErrorCode;
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
};
use shapes::film_geometry::ImageRight;
use shapes::landmarks::{HipCentre, HipSide, LandmarkName, Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::marks::{
    EndplateLine, EndplateLines, FrontMarks, LineEnd, Mark, MarkName, MarkSource, PedicleMark,
    SideMarks,
};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::toolbox::{PinAction, PinInput, PinTarget};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn film(kind: FilmKind) -> DecodedFilm {
    DecodedFilm {
        image: GreyImage::new(300, 800, vec![0; 300 * 800], Placement::FILM).unwrap(),
        facts: FilmFacts {
            kind,
            width: 300,
            height: 800,
            bits_stored: 16,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            default_window: DisplayWindow::CentreWidth {
                centre: 32767.5,
                width: 65535.0,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        },
    }
}

fn chain(kind: FilmKind, count: usize) -> FilmMarks {
    let levels = Template::Standard.levels();
    let vertebrae = (0..count)
        .map(|index| {
            let y = 20.0 + 60.0 * index as f64;
            let q = [
                Px::new(100.0, y),
                Px::new(160.0, y),
                Px::new(160.0, y + 40.0),
                Px::new(100.0, y + 40.0),
            ]
            .map(Mark::auto);
            let pedicle = |x| PedicleMark::Visible {
                at: Px::new(x, y + 12.8),
                source: MarkSource::Auto,
                pinned: false,
                inferred: false,
                ring_score: Some(0.9),
            };
            let marks = match kind {
                FilmKind::Front => VertebraMarks::Front {
                    points: FrontMarks {
                        sup_l: q[0],
                        sup_r: q[1],
                        inf_r: q[2],
                        inf_l: q[3],
                        ped_l: pedicle(106.6),
                        ped_r: pedicle(153.4),
                    },
                },
                FilmKind::Side => VertebraMarks::Side {
                    points: SideMarks {
                        sup_a: q[0],
                        sup_p: q[1],
                        inf_p: q[2],
                        inf_a: q[3],
                    },
                },
                FilmKind::BendLeft | FilmKind::BendRight => VertebraMarks::Bending {
                    lines: EndplateLines {
                        sup: [q[0], q[1]],
                        inf: [q[3], q[2]],
                    },
                },
            };
            VertebraOnFilm {
                id: index as u32 + 1,
                level: levels[index],
                marks,
                confidence: Confidence {
                    score: 0.95,
                    band: ConfidenceBand::High,
                    reasons: vec![ConfidenceReason::EditedByUser],
                    calibrated: false,
                    endplate_spread_px: None,
                    margin: None,
                },
                outline: Some(q.map(|m| m.at).to_vec()),
            }
        })
        .collect::<Vec<_>>();
    let c7 = Landmarks::c7_of(&vertebrae);
    FilmMarks {
        film: kind,
        vertebrae,
        landmarks: Landmarks {
            c7,
            ..Landmarks::NONE
        },
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: Vec::new(),
            from_top: Some(levels[..count].to_vec()),
            from_bottom: Some(levels[..count].to_vec()),
            flags: Vec::new(),
            l5_confirmed_on: None,
        },
        named_for: if kind == FilmKind::Side {
            ImageRight::Posterior
        } else {
            ImageRight::PatientRight
        },
    }
}

fn request(marks: FilmMarks, vertebra: u32, mark: MarkName, to: Px) -> PinInput {
    PinInput {
        case_version: 77,
        marks,
        action: PinAction::Move {
            target: PinTarget::Vertebra { vertebra, mark },
            to,
        },
    }
}

fn distance(a: Px, b: Px) -> f64 {
    libm::hypot(a.x() - b.x(), a.y() - b.y())
}

#[test]
fn the_requested_corner_and_existing_pins_hold_while_neighbours_move() {
    let mut marks = chain(FilmKind::Side, 9);
    if let VertebraMarks::Side { points } = &mut marks.vertebrae[3].marks {
        points.sup_p.pinned = true;
    }
    let fixed = marks.vertebrae[3].marks.corners()[1];
    let input = request(marks.clone(), 5, MarkName::SupA, Px::new(108.0, 260.0));
    let output = toolbox::pins::apply(&film(FilmKind::Side), None, None, &input).unwrap();
    assert_eq!(output.case_version, 77);
    let pin = output.marks.vertebrae[4].marks.corners()[0];
    assert_eq!(pin.at, Px::new(108.0, 260.0));
    assert_eq!(pin.source, MarkSource::Pin);
    assert!(pin.pinned && !pin.inferred);
    assert_eq!(output.marks.vertebrae[3].marks.corners()[1], fixed);
    assert_ne!(
        output.marks.vertebrae[4].marks.corners()[2].at,
        marks.vertebrae[4].marks.corners()[2].at
    );
    assert!(output.changed.contains(&4) && output.changed.contains(&6));
    for (before, after) in marks.vertebrae.iter().zip(&output.marks.vertebrae) {
        if after.id == 5 {
            assert!(
                after
                    .confidence
                    .reasons
                    .contains(&ConfidenceReason::EditedByUser)
            );
        } else if before.marks != after.marks {
            assert!(
                !after
                    .confidence
                    .reasons
                    .contains(&ConfidenceReason::EditedByUser)
            );
        }
    }
    // An 8 px correction is one fifth of this 40 px body height. The
    // experimental fit must not move any unselected corner as far as the
    // requested correction itself. This is a geometric locality bound,
    // not a claim that a prior-only re-solve improves truth.
    let maximum = marks
        .vertebrae
        .iter()
        .zip(&output.marks.vertebrae)
        .flat_map(|(a, b)| {
            a.marks
                .corners()
                .into_iter()
                .zip(b.marks.corners())
                .filter(|(_, new)| !new.pinned)
                .map(|(a, b)| distance(a.at, b.at))
        })
        .fold(0.0f64, f64::max);
    assert!(maximum < 8.0, "unselected maximum {maximum}");
}

#[test]
fn pinning_without_moving_and_unpinning_do_not_run_a_fit() {
    let marks = chain(FilmKind::Front, 3);
    let at = marks.vertebrae[1].marks.corners()[0].at;
    let input = request(marks.clone(), 2, MarkName::SupL, at);
    let pinned = toolbox::pins::apply(&film(FilmKind::Front), None, None, &input).unwrap();
    assert_eq!(pinned.changed, vec![2]);
    for (before, after) in marks.vertebrae.iter().zip(&pinned.marks.vertebrae) {
        assert_eq!(
            before.marks.corners().map(|m| m.at),
            after.marks.corners().map(|m| m.at)
        );
        assert_eq!(before.outline, after.outline);
    }
    let input = PinInput {
        case_version: 78,
        marks: pinned.marks.clone(),
        action: PinAction::Unpin {
            target: PinTarget::Vertebra {
                vertebra: 2,
                mark: MarkName::SupL,
            },
        },
    };
    let unpinned = toolbox::pins::apply(&film(FilmKind::Front), None, None, &input).unwrap();
    assert_eq!(unpinned.changed, vec![2]);
    assert_eq!(unpinned.marks.vertebrae[1].marks.corners()[0].at, at);
    assert!(!unpinned.marks.vertebrae[1].marks.corners()[0].pinned);
    assert_eq!(unpinned.marks.vertebrae[0], marks.vertebrae[0]);
    assert_eq!(unpinned.marks.vertebrae[2], marks.vertebrae[2]);
}

#[test]
fn a_second_drag_holds_the_first_pin_and_pinned_pedicles() {
    let mut marks = chain(FilmKind::Front, 3);
    if let VertebraMarks::Front { points } = &mut marks.vertebrae[1].marks
        && let PedicleMark::Visible { pinned, .. } = &mut points.ped_l
    {
        *pinned = true;
    }
    let old_pedicle = if let VertebraMarks::Front { points } = &marks.vertebrae[1].marks {
        points.ped_l
    } else {
        unreachable!()
    };
    let input = request(marks, 2, MarkName::SupL, Px::new(104.0, 80.0));
    let first = toolbox::pins::apply(&film(FilmKind::Front), None, None, &input).unwrap();
    let first_pin = first.marks.vertebrae[1].marks.corners()[0];
    let second_input = request(first.marks, 2, MarkName::InfR, Px::new(164.0, 120.0));
    let second = toolbox::pins::apply(&film(FilmKind::Front), None, None, &second_input).unwrap();
    assert_eq!(second.marks.vertebrae[1].marks.corners()[0], first_pin);
    assert_eq!(
        second.marks.vertebrae[1].marks.corners()[2].at,
        Px::new(164.0, 120.0)
    );
    if let VertebraMarks::Front { points } = &second.marks.vertebrae[1].marks {
        assert_eq!(points.ped_l, old_pedicle);
        assert!(points.ped_r.mark().unwrap().inferred);
    }
}

#[test]
fn bending_line_ends_are_pinnable_and_c7_follows_the_four_ends() {
    let marks = chain(FilmKind::BendLeft, 2);
    let input = PinInput {
        case_version: 91,
        marks,
        action: PinAction::Move {
            target: PinTarget::Endplate {
                vertebra: 1,
                line: EndplateLine::Sup,
                end: LineEnd::PatientLeft,
            },
            to: Px::new(106.0, 20.0),
        },
    };
    let output = toolbox::pins::apply(&film(FilmKind::BendLeft), None, None, &input).unwrap();
    assert_eq!(output.case_version, 91);
    let corners = output.marks.vertebrae[0].marks.corners();
    assert_eq!(corners[0].at, Px::new(106.0, 20.0));
    let mean = Px::new(
        corners.iter().map(|m| m.at.x()).sum::<f64>() / 4.0,
        corners.iter().map(|m| m.at.y()).sum::<f64>() / 4.0,
    );
    assert_eq!(output.marks.landmarks.c7.unwrap().at, mean);
    assert!(!output.marks.landmarks.c7.unwrap().pinned);
}

#[test]
fn landmarks_and_pedicles_do_not_reshape_a_vertebra() {
    let mut marks = chain(FilmKind::Front, 2);
    marks.landmarks.sacrum_top = Some(SacrumTop::Front {
        sac_l: Mark::auto(Px::new(100.0, 180.0)),
        sac_r: Mark::auto(Px::new(160.0, 180.0)),
    });
    marks.landmarks.hips.push(HipCentre {
        side: HipSide::Left,
        centre: Mark::auto(Px::new(180.0, 220.0)),
        radius_px: Some(20.0),
    });
    for landmark in [LandmarkName::SacL, LandmarkName::HipLeft] {
        let input = PinInput {
            case_version: 5,
            marks: marks.clone(),
            action: PinAction::Move {
                target: PinTarget::Landmark { landmark },
                to: Px::new(110.0, 190.0),
            },
        };
        let output = toolbox::pins::apply(&film(FilmKind::Front), None, None, &input).unwrap();
        assert!(output.changed.is_empty());
        assert_eq!(output.marks.vertebrae, marks.vertebrae);
        let pin = if landmark == LandmarkName::HipLeft {
            output.marks.landmarks.hips[0].centre
        } else if let Some(SacrumTop::Front { sac_l, .. }) = output.marks.landmarks.sacrum_top {
            sac_l
        } else {
            unreachable!()
        };
        assert_eq!(pin.at, Px::new(110.0, 190.0));
        assert!(pin.pinned);
    }
    let input = request(marks.clone(), 1, MarkName::PedL, Px::new(110.0, 30.0));
    let output = toolbox::pins::apply(&film(FilmKind::Front), None, None, &input).unwrap();
    assert_eq!(output.changed, vec![1]);
    assert_eq!(
        output.marks.vertebrae[0].marks.corners(),
        marks.vertebrae[0].marks.corners()
    );
    if let VertebraMarks::Front { points } = output.marks.vertebrae[0].marks {
        assert_eq!(points.ped_l.mark().unwrap().at, Px::new(110.0, 30.0));
        assert!(points.ped_l.mark().unwrap().pinned);
    }
}

#[test]
fn invalid_pin_requests_leave_the_input_unchanged() {
    let marks = chain(FilmKind::Side, 2);
    for (id, name, to, error) in [
        (
            1,
            MarkName::SupL,
            Px::new(110.0, 20.0),
            ErrorCode::ToolPinInvalid,
        ),
        (
            99,
            MarkName::SupA,
            Px::new(110.0, 20.0),
            ErrorCode::ToolUnknownVertebra,
        ),
        (
            1,
            MarkName::SupA,
            Px::new(170.0, 70.0),
            ErrorCode::ToolPinInvalid,
        ),
        (
            1,
            MarkName::SupA,
            Px::new(-10.0, 20.0),
            ErrorCode::ToolClickOutsideFilm,
        ),
        (
            1,
            MarkName::SupA,
            Px::new(f64::NAN, 20.0),
            ErrorCode::ToolClickOutsideFilm,
        ),
    ] {
        let input = request(marks.clone(), id, name, to);
        assert_eq!(
            toolbox::pins::apply(&film(FilmKind::Side), None, None, &input)
                .unwrap_err()
                .code,
            error
        );
        assert_eq!(input.marks, marks);
    }
}

#[test]
fn one_square_matches_the_closed_form_similarity_solution() {
    let mut marks = chain(FilmKind::Side, 1);
    let half_side = 0.25;
    let q = [
        Px::new(-half_side, -half_side),
        Px::new(half_side, -half_side),
        Px::new(half_side, half_side),
        Px::new(-half_side, half_side),
    ]
    .map(Mark::auto);
    marks.vertebrae[0].marks = VertebraMarks::Side {
        points: SideMarks {
            sup_a: q[0],
            sup_p: q[1],
            inf_p: q[2],
            inf_a: q[3],
        },
    };
    marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
    let movement = 0.1;
    let input = request(
        marks,
        1,
        MarkName::SupA,
        Px::new(-half_side + movement, -half_side),
    );
    let output = toolbox::pins::apply(&film(FilmKind::Side), None, None, &input).unwrap();

    // Independent elimination for a square centred at zero, one corner
    // held at (dx,0). Eliminate the three free corner displacements first:
    // d = k Mq. Their effective similarity weight is gamma. The square's
    // summed similarity Gram matrix is diag(8c²,8c²,4,4); the held
    // corner is a rank-two update with two orthogonal rows. The
    // Sherman-Morrison formula gives factor, without a matrix solve.
    let shape = 4.0; // 1 / (0.5 px height)^2
    let regulariser = 1e-6;
    let stay = 0.05 * shape + regulariser;
    let k = shape / (shape + stay);
    let gamma = shape * stay / (shape + stay);
    let rotation_diagonal = 8.0 * half_side * half_side * gamma + regulariser;
    let translation_diagonal = 4.0 * gamma + regulariser;
    let rank_update = 2.0 * half_side * half_side / rotation_diagonal + 1.0 / translation_diagonal;
    let factor = shape * movement / (1.0 + (shape - gamma) * rank_update);
    let horizontal = k * factor / translation_diagonal;
    let vertical = k * 2.0 * half_side * half_side * factor / rotation_diagonal;
    let expected = [
        Px::new(-half_side + movement, -half_side),
        Px::new(half_side + horizontal, -half_side + vertical),
        Px::new(half_side + horizontal - vertical, half_side),
        Px::new(-half_side + horizontal, half_side - vertical),
    ];
    // 1e-10 px allows only rounding in the Cholesky factorisation.
    for (got, want) in output.marks.vertebrae[0]
        .marks
        .corners()
        .into_iter()
        .zip(expected)
    {
        assert!(distance(got.at, want) < 1e-10, "{:?} vs {want:?}", got.at);
    }
}

#[test]
fn prior_only_refit_reports_the_error_it_spreads_to_other_corners() {
    let truth = chain(FilmKind::Side, 7);
    let mut perturbed = truth.clone();
    if let VertebraMarks::Side { points } = &mut perturbed.vertebrae[3].marks {
        points.sup_a.at.0[0] += 8.0;
    }
    let input = request(perturbed, 4, MarkName::SupA, Px::new(100.0, 200.0));
    let output = toolbox::pins::apply(&film(FilmKind::Side), None, None, &input).unwrap();
    let mut total = 0.0;
    let mut count = 0;
    let mut maximum = 0.0f64;
    for (index, (old, new)) in truth
        .vertebrae
        .iter()
        .zip(&output.marks.vertebrae)
        .enumerate()
    {
        for (corner, (a, b)) in old
            .marks
            .corners()
            .iter()
            .zip(new.marks.corners())
            .enumerate()
        {
            if index == 3 && corner == 0 {
                assert_eq!(a.at, b.at);
                continue;
            }
            let error = distance(a.at, b.at);
            maximum = maximum.max(error);
            total += error;
            count += 1;
        }
    }
    eprintln!(
        "pin truth: corrected 8 px error; other-corner mean {:.6} px, max {:.6} px",
        total / f64::from(count),
        maximum
    );
    // A prior-only fit can spread error. Keep that cost below the 8 px
    // error corrected, and report it instead of asserting improvement.
    assert!(maximum > 0.0 && maximum < 8.0);
    assert_eq!(output.marks.vertebrae[3].level, Level::T3);
}
