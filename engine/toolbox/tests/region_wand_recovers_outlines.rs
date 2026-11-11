//! Scripted clicks against independently painted body edges, in pixel units.
mod region_films;
use region_films::*;
use shapes::{
    confidence::ConfidenceReason,
    errors::ErrorCode,
    pixels::Px,
    toolbox::{RegionWandInput, ToolPreview, ToolPreviewInput, WandClick, WandKind, WandPolarity},
    vertebra::VertebraMarks,
};

#[test]
fn scripted_add_and_subtract_recover_rotated_and_child_outlines() {
    for (h, w) in [(40., 64.), (28., 45.), (180., 270.)] {
        for angle in [-0.25, 0., 0.25] {
            for top in [true, false] {
                let (film, mut marks, truth) = fixture(800, 6000, h, w, angle, false);
                let index = 8;
                let q = truth[index];
                let polarity = if top {
                    WandPolarity::Add
                } else {
                    WandPolarity::Subtract
                };
                perturb(&mut marks, index, if top { 0.4 } else { 0.5 }, top);
                let old = marks.clone();
                let vertebra = marks.vertebrae[index].id;
                let at = if top {
                    midpoint(
                        midpoint(q[0], q[1]),
                        midpoint(
                            marks.vertebrae[index].marks.corners()[0].at,
                            marks.vertebrae[index].marks.corners()[1].at,
                        ),
                    )
                } else {
                    midpoint(
                        midpoint(q[2], q[3]),
                        midpoint(
                            marks.vertebrae[index].marks.corners()[2].at,
                            marks.vertebrae[index].marks.corners()[3].at,
                        ),
                    )
                };
                let preview = toolbox::preview(
                    &film,
                    None,
                    None,
                    &ToolPreviewInput {
                        case_version: 17,
                        tool: WandKind::RegionWand,
                        at,
                        marks: marks.clone(),
                        new_vertebra_id: 99,
                    },
                )
                .unwrap();
                assert_eq!(
                    preview,
                    if top {
                        ToolPreview::Add {
                            case_version: 17,
                            vertebra,
                        }
                    } else {
                        ToolPreview::Subtract {
                            case_version: 17,
                            vertebra,
                        }
                    }
                );
                let result = toolbox::region_wand::apply(
                    &film,
                    None,
                    None,
                    &RegionWandInput {
                        case_version: 17,
                        marks,
                        vertebra,
                        clicks: vec![WandClick { at, polarity }],
                    },
                )
                .unwrap();
                assert_eq!(result.case_version, 17);
                assert_eq!(result.changed, [vertebra]);
                let fitted = result.marks.vertebrae[index].marks.corners();
                for (got, want) in fitted.iter().zip(q) {
                    let dx = got.at.x() - want.x();
                    let dy = got.at.y() - want.y();
                    // 6% of body height accommodates the bounded working grid and
                    // antialiased raster; the deliberately missing strip is 40%.
                    assert!(
                        (dx * dx + dy * dy).sqrt() <= 0.06 * h,
                        "h={h} angle={angle} top={top}: {:?} != {want:?}",
                        got.at
                    );
                }
                for i in 0..old.vertebrae.len() {
                    if i != index {
                        assert_eq!(result.marks.vertebrae[i], old.vertebrae[i]);
                    }
                }
                assert!(
                    result.marks.vertebrae[index]
                        .confidence
                        .reasons
                        .contains(&ConfidenceReason::EditedByUser)
                );
                assert!(!result.growth.is_empty() && result.growth.len() <= 4);
                assert!(result.growth.iter().all(|t| t.width <= 160
                    && t.height <= 160
                    && t.data.len() == (t.width * t.height) as usize));
                assert_eq!(result.outline.first(), result.outline.last());
            }
        }
    }
}

#[test]
fn inverted_intensity_recovers_the_same_corners_and_a_pin_stays_exact() {
    let mut answers = Vec::new();
    for inverted in [false, true] {
        let (film, mut marks, truth) = fixture(400, 1600, 40., 64., 0., inverted);
        perturb(&mut marks, 8, 0.4, true);
        if let VertebraMarks::Side { points } = &mut marks.vertebrae[8].marks {
            points.inf_a.pinned = true;
        }
        let pinned = marks.vertebrae[8].marks.corners()[3];
        let at = Px::new(200., truth[8][0].y() + 8.);
        let answer = toolbox::region_wand::apply(
            &film,
            None,
            None,
            &RegionWandInput {
                case_version: 7,
                marks,
                vertebra: 9,
                clicks: vec![WandClick {
                    at,
                    polarity: WandPolarity::Add,
                }],
            },
        )
        .unwrap();
        assert_eq!(answer.marks.vertebrae[8].marks.corners()[3], pinned);
        answers.push(answer);
    }
    for (a, b) in answers[0].marks.vertebrae[8]
        .marks
        .corners()
        .iter()
        .zip(answers[1].marks.vertebrae[8].marks.corners())
    {
        assert!((a.at.x() - b.at.x()).abs() < 0.01 && (a.at.y() - b.at.y()).abs() < 0.01);
    }
    assert_eq!(answers[0].warnings, answers[1].warnings);
}

#[test]
fn stale_target_or_opposite_polarity_cannot_do_an_unpreviewed_edit() {
    let (film, marks, truth) = fixture(400, 1600, 40., 64., 0., false);
    let at = midpoint(truth[8][0], truth[8][2]);
    for (vertebra, polarity) in [(8, WandPolarity::Subtract), (9, WandPolarity::Add)] {
        let error = toolbox::region_wand::apply(
            &film,
            None,
            None,
            &RegionWandInput {
                case_version: 7,
                marks: marks.clone(),
                vertebra,
                clicks: vec![WandClick { at, polarity }],
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ToolRefused);
    }
}

#[test]
fn bending_films_and_fully_pinned_outlines_preview_no_action() {
    use shapes::film::FilmKind;
    use shapes::marks::EndplateLines;
    use shapes::toolbox::MissReason;
    let (mut film, mut marks, truth) = fixture(400, 1600, 40., 64., 0., false);
    let at = midpoint(truth[8][0], truth[8][2]);
    if let VertebraMarks::Side { points } = &mut marks.vertebrae[8].marks {
        for m in [
            &mut points.sup_a,
            &mut points.sup_p,
            &mut points.inf_a,
            &mut points.inf_p,
        ] {
            m.pinned = true;
        }
    }
    let make = |marks| ToolPreviewInput {
        case_version: 23,
        tool: WandKind::RegionWand,
        at,
        marks,
        new_vertebra_id: 50,
    };
    assert_eq!(
        toolbox::preview(&film, None, None, &make(marks.clone())).unwrap(),
        ToolPreview::Nothing {
            case_version: 23,
            reason: MissReason::AllPinned
        }
    );
    for kind in [FilmKind::BendLeft, FilmKind::BendRight] {
        film.facts.kind = kind;
        marks.film = kind;
        for v in &mut marks.vertebrae {
            let q = v.marks.corners();
            v.marks = VertebraMarks::Bending {
                lines: EndplateLines {
                    sup: [q[0], q[1]],
                    inf: [q[3], q[2]],
                },
            };
        }
        assert_eq!(
            toolbox::preview(&film, None, None, &make(marks.clone())).unwrap(),
            ToolPreview::Nothing {
                case_version: 23,
                reason: MissReason::NotSupportedOnFilm
            }
        );
    }
}
