//! Analytic rectangles give independent endplate, id and naming answers.

use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason};
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
};
use shapes::film_geometry::ImageRight;
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template};
use shapes::pixels::Px;
use shapes::toolbox::{
    BoundaryWandInput, MissReason, ToolPreview, ToolPreviewInput, ToolWarning, WandKind,
};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
use toolbox::boundary_wand;

pub fn film(kind: FilmKind, width: u32, height: u32) -> DecodedFilm {
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                if (20..=100).contains(&x)
                    && ((40..80).contains(&y) || (90..130).contains(&y) || (140..180).contains(&y))
                {
                    40000
                } else {
                    10000
                }
            })
        })
        .collect();
    DecodedFilm {
        image: GreyImage::new(width, height, pixels, Placement::FILM).expect("grid"),
        facts: FilmFacts {
            kind,
            width,
            height,
            bits_stored: 16,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: vec![],
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

pub fn marks(intervals: &[(f64, f64, Level)]) -> FilmMarks {
    let levels: Vec<_> = intervals.iter().map(|x| x.2).collect();
    FilmMarks {
        film: FilmKind::Side,
        vertebrae: intervals
            .iter()
            .enumerate()
            .map(|(i, &(top, bottom, level))| VertebraOnFilm {
                id: i as u32 + 1,
                level,
                marks: VertebraMarks::Side {
                    points: SideMarks {
                        sup_a: Mark::auto(Px::new(20.0, top)),
                        sup_p: Mark::auto(Px::new(100.0, top)),
                        inf_p: Mark::auto(Px::new(100.0, bottom)),
                        inf_a: Mark::auto(Px::new(20.0, bottom)),
                    },
                },
                confidence: Confidence {
                    score: 0.95,
                    band: ConfidenceBand::High,
                    reasons: vec![],
                    calibrated: false,
                    endplate_spread_px: None,
                    margin: None,
                },
                outline: None,
            })
            .collect(),
        landmarks: Landmarks::NONE,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: vec![],
            from_top: Some(levels.clone()),
            from_bottom: Some(levels),
            flags: vec![],
            l5_confirmed_on: None,
        },
        named_for: ImageRight::Anterior,
    }
}

fn preview(film: &DecodedFilm, marks: &FilmMarks, at: Px) -> ToolPreview {
    boundary_wand::preview(
        film,
        None,
        None,
        &ToolPreviewInput {
            case_version: 27,
            tool: WandKind::BoundaryWand,
            at,
            marks: marks.clone(),
            new_vertebra_id: 99,
        },
    )
    .expect("preview")
}
fn click(film: &DecodedFilm, marks: &FilmMarks, at: Px) -> shapes::toolbox::BoundaryWandOutput {
    boundary_wand::apply(
        film,
        None,
        None,
        &BoundaryWandInput {
            case_version: 27,
            marks: marks.clone(),
            at,
            new_vertebra_id: 99,
        },
    )
    .expect("edit")
}

#[test]
fn split_recovers_the_two_known_bodies_and_merge_restores_outer_geometry() {
    let f = film(FilmKind::Side, 160, 240);
    let original = marks(&[(40.0, 130.0, Level::T8), (140.0, 180.0, Level::T9)]);
    let at = Px::new(60.0, 85.0);
    let ToolPreview::Split {
        case_version,
        vertebra,
        line,
        levels_after,
    } = preview(&f, &original, at)
    else {
        panic!("split preview")
    };
    assert_eq!((case_version, vertebra), (27, 1));
    let split = click(&f, &original, at);
    assert_eq!(split.case_version, 27);
    assert_eq!(split.new_vertebra, Some(99));
    assert_eq!(split.marks.vertebrae.len(), 3);
    let a = split.marks.vertebrae[0].marks.corners();
    let b = split.marks.vertebrae[1].marks.corners();
    // Grid spacing is 0.8 px; smoothing spans three samples. Two pixels
    // covers sampling plus that smoothing's edge-location uncertainty.
    assert!((a[2].at.y() - 80.0).abs() < 2.0);
    assert!((b[0].at.y() - 90.0).abs() < 2.0);
    assert_eq!(
        line,
        [
            Px::new(20.0, (a[3].at.y() + b[0].at.y()) * 0.5),
            Px::new(100.0, (a[2].at.y() + b[1].at.y()) * 0.5)
        ]
    );
    assert!(levels_after.iter().all(|entry| {
        split
            .marks
            .vertebrae
            .iter()
            .any(|v| v.id == entry.vertebra && v.level == entry.level)
    }));
    assert_eq!(
        split
            .marks
            .vertebrae
            .iter()
            .map(|v| v.level)
            .collect::<Vec<_>>(),
        vec![Level::T8, Level::T9, Level::T10]
    );
    assert!(split.marks.vertebrae[..2].iter().all(|v| {
        v.confidence
            .reasons
            .contains(&ConfidenceReason::EditedByUser)
    }));
    assert!(
        !split.marks.vertebrae[2]
            .confidence
            .reasons
            .contains(&ConfidenceReason::EditedByUser)
    );
    let merged = click(&f, &split.marks, at);
    assert_eq!(merged.removed_vertebra, Some(99));
    for (a, b) in merged.marks.vertebrae.iter().zip(&original.vertebrae) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.level, b.level);
        assert_eq!(a.marks, b.marks);
    }
    assert_eq!(merged.marks.naming, original.naming);
}

#[test]
fn hover_hit_table_uses_body_margins_and_gap_tolerance() {
    let f = film(FilmKind::Side, 160, 240);
    let m = marks(&[(40.0, 80.0, Level::T8), (90.0, 130.0, Level::T9)]);
    for (at, reason) in [
        (Px::new(60.0, 47.2), MissReason::NearEndplate),
        (Px::new(60.0, 48.0), MissReason::NearEndplate),
        (Px::new(130.0, 60.0), MissReason::OffSpine),
        (Px::new(60.0, 20.0), MissReason::OutsideChain),
        (Px::new(60.0, 160.0), MissReason::OutsideChain),
        (Px::new(-2.0, 80.0), MissReason::OutsideFilm),
    ] {
        assert_eq!(
            preview(&f, &m, at),
            ToolPreview::Nothing {
                case_version: 27,
                reason
            }
        );
    }
    assert!(matches!(
        preview(&f, &m, Px::new(60.0, 85.0)),
        ToolPreview::Merge {
            upper: 1,
            lower: 2,
            ..
        }
    ));
    assert!(matches!(
        preview(&f, &m, Px::new(104.0, 85.0)),
        ToolPreview::Merge { .. }
    ));
    assert!(matches!(
        preview(&f, &m, Px::new(60.0, 60.0)),
        ToolPreview::Split { .. }
    ));
    assert!(matches!(
        preview(&f, &m, Px::new(60.0, 50.8)),
        ToolPreview::Split { .. }
    ));
    let mut touching = m.clone();
    if let VertebraMarks::Side { points } = &mut touching.vertebrae[1].marks {
        points.sup_a.at.0[1] = 80.0;
        points.sup_p.at.0[1] = 80.0;
    }
    assert!(matches!(
        preview(&f, &touching, Px::new(60.0, 80.0)),
        ToolPreview::Merge { .. }
    ));
    if let VertebraMarks::Side { points } = &mut touching.vertebrae[1].marks {
        points.sup_a.at.0[1] = 76.0;
        points.sup_p.at.0[1] = 76.0;
    }
    assert!(matches!(
        preview(&f, &touching, Px::new(60.0, 78.0)),
        ToolPreview::Merge { .. }
    ));
}

#[test]
fn confirmed_l5_stays_named_and_upper_names_shift() {
    let f = film(FilmKind::Side, 160, 240);
    let mut m = marks(&[
        (40.0, 130.0, Level::L3),
        (140.0, 180.0, Level::L4),
        (190.0, 230.0, Level::L5),
    ]);
    m.naming.l5_confirmed_on = Some(3);
    let out = click(&f, &m, Px::new(60.0, 85.0));
    assert_eq!(
        out.marks
            .vertebrae
            .iter()
            .map(|v| v.level)
            .collect::<Vec<_>>(),
        vec![Level::L2, Level::L3, Level::L4, Level::L5]
    );
    assert_eq!(out.marks.naming.l5_confirmed_on, Some(3));
    assert_eq!(out.marks.vertebrae[3].id, 3);
    assert!(out.warnings.contains(&ToolWarning::NamingConflict));
}

#[test]
fn retiring_the_confirmed_l5_clears_confirmation() {
    let f = film(FilmKind::Side, 160, 240);
    let mut m = marks(&[(40.0, 80.0, Level::L4), (90.0, 130.0, Level::L5)]);
    m.naming.l5_confirmed_on = Some(2);
    let out = click(&f, &m, Px::new(60.0, 85.0));
    assert_eq!(out.marks.naming.l5_confirmed_on, None);
    assert!(out.warnings.contains(&ToolWarning::NamingConflict));
}

#[test]
fn names_run_out_before_either_preview_or_commit_can_edit() {
    let f = film(FilmKind::Side, 160, 240);
    let mut m = marks(&[(40.0, 130.0, Level::L7)]);
    m.naming.flags.push(NamingFlag::NonStandardCount);
    assert_eq!(
        preview(&f, &m, Px::new(60.0, 85.0)),
        ToolPreview::Nothing {
            case_version: 27,
            reason: MissReason::NamesRunOut
        }
    );
    assert!(
        boundary_wand::apply(
            &f,
            None,
            None,
            &BoundaryWandInput {
                case_version: 27,
                marks: m,
                at: Px::new(60.0, 85.0),
                new_vertebra_id: 99
            }
        )
        .is_err()
    );
}

#[test]
fn split_preserves_outer_pins_and_merge_reports_retired_pins() {
    let f = film(FilmKind::Side, 160, 240);
    let mut m = marks(&[(40.0, 130.0, Level::T8)]);
    if let VertebraMarks::Side { points } = &mut m.vertebrae[0].marks {
        points.sup_a.pinned = true;
        points.inf_p.pinned = true;
    }
    let mut split = click(&f, &m, Px::new(60.0, 85.0));
    assert_eq!(
        split.marks.vertebrae[0].marks.corners()[0],
        m.vertebrae[0].marks.corners()[0]
    );
    assert_eq!(
        split.marks.vertebrae[1].marks.corners()[2],
        m.vertebrae[0].marks.corners()[2]
    );
    if let VertebraMarks::Side { points } = &mut split.marks.vertebrae[0].marks {
        points.inf_a.pinned = true;
    }
    if let VertebraMarks::Side { points } = &mut split.marks.vertebrae[1].marks {
        points.sup_p.pinned = true;
    }
    let merged = click(&f, &split.marks, Px::new(60.0, 85.0));
    assert!(
        merged
            .warnings
            .contains(&ToolWarning::PinsRemoved { count: 2 })
    );
    assert_eq!(
        merged.marks.vertebrae[0].marks.corners(),
        m.vertebrae[0].marks.corners()
    );
}

#[test]
fn all_bending_films_have_no_boundary_action() {
    for kind in [FilmKind::BendLeft, FilmKind::BendRight] {
        let f = film(kind, 160, 240);
        let mut m = marks(&[(40.0, 130.0, Level::T8)]);
        m.film = kind;
        m.named_for = ImageRight::PatientRight;
        let q = m.vertebrae[0].marks.corners();
        m.vertebrae[0].marks = VertebraMarks::Bending {
            lines: EndplateLines {
                sup: [q[0], q[1]],
                inf: [q[3], q[2]],
            },
        };
        assert_eq!(
            preview(&f, &m, Px::new(60.0, 85.0)),
            ToolPreview::Nothing {
                case_version: 27,
                reason: MissReason::NotSupportedOnFilm
            }
        );
    }
}

#[test]
fn front_pedicles_keep_their_piece_and_unseen_state() {
    let f = film(FilmKind::Front, 160, 240);
    let mut m = marks(&[(40.0, 130.0, Level::T8)]);
    m.film = FilmKind::Front;
    m.named_for = ImageRight::PatientRight;
    let q = m.vertebrae[0].marks.corners();
    let ped_l = PedicleMark::Visible {
        at: Px::new(35.0, 60.0),
        source: MarkSource::Pin,
        pinned: true,
        inferred: false,
        ring_score: None,
    };
    let ped_r = PedicleMark::NotVisible {
        source: MarkSource::Hand,
    };
    m.vertebrae[0].marks = VertebraMarks::Front {
        points: FrontMarks {
            sup_l: q[0],
            sup_r: q[1],
            inf_r: q[2],
            inf_l: q[3],
            ped_l,
            ped_r,
        },
    };
    let out = click(&f, &m, Px::new(60.0, 85.0));
    let VertebraMarks::Front { points: a } = out.marks.vertebrae[0].marks else {
        panic!()
    };
    let VertebraMarks::Front { points: b } = out.marks.vertebrae[1].marks else {
        panic!()
    };
    assert_eq!(a.ped_l, ped_l);
    assert_eq!(a.ped_r, ped_r);
    assert_eq!(b.ped_r, ped_r);
    assert!(matches!(
        b.ped_l,
        PedicleMark::Visible {
            inferred: true,
            pinned: false,
            ..
        }
    ));
}

#[test]
fn too_short_parent_uses_joint_window_and_clears_neighbour_edit_reasons() {
    let f = film(FilmKind::Side, 200, 460);
    let mut m = marks(&[
        (40.0, 120.0, Level::T6),
        (140.0, 160.0, Level::T7),
        (180.0, 260.0, Level::T8),
        (280.0, 360.0, Level::T9),
    ]);
    for body in &mut m.vertebrae {
        body.confidence.reasons.push(ConfidenceReason::EditedByUser);
    }
    let out = click(&f, &m, Px::new(60.0, 150.0));
    assert_eq!(out.marks.vertebrae.len(), 5);
    for body in &out.marks.vertebrae {
        let aimed = body.id == 2 || body.id == 99;
        if aimed {
            assert!(
                body.confidence
                    .reasons
                    .contains(&ConfidenceReason::EditedByUser)
            );
        } else if m
            .vertebrae
            .iter()
            .find(|v| v.id == body.id)
            .expect("old")
            .marks
            != body.marks
        {
            assert!(
                !body
                    .confidence
                    .reasons
                    .contains(&ConfidenceReason::EditedByUser)
            );
        }
    }
}

#[test]
fn a_frozen_film_cache_supplies_the_known_endplate_edges() {
    use auto_pass::handoffs::{
        Barcode, CentreLine, CentreLineEnds, CentreSample, FilmEvidence, FilmPassCache, Polarity,
        PolarityCheck, Strip, StripEvidence, StripRow,
    };
    use shapes::film_geometry::QuarterTurns;
    let f = film(FilmKind::Side, 160, 240);
    let grid = |pixels| GreyImage::new(160, 240, pixels, Placement::FILM).expect("grid");
    let zero = grid(vec![0.0f32; 160 * 240]);
    // The cached detector independently supplies two sharp horizontal
    // edges at y=78 and y=92, deliberately different from the raw film.
    let edge = grid(
        (0..240)
            .flat_map(|y| (0..160).map(move |_| if y == 78 || y == 92 { 1.0 } else { 0.0 }))
            .collect(),
    );
    let mask = GreyImage::new(160, 240, vec![1u8; 160 * 240], Placement::FILM).expect("grid");
    let evidence = FilmEvidence {
        film: FilmKind::Side,
        mm_per_px: 1.0,
        scale_assumed: true,
        valid: mask,
        image: zero.clone(),
        phase_edge: edge.clone(),
        phase_direction: zero.clone(),
        phase_kind: zero.clone(),
        gradient_edge: edge,
        gradient_direction: zero.clone(),
        ridge: zero.clone(),
        ridge_orientation: zero.clone(),
        valley: zero.clone(),
        valley_orientation: zero.clone(),
        valley_scale: GreyImage::new(160, 240, vec![0u8; 160 * 240], Placement::FILM)
            .expect("grid"),
        ridge_symmetry: zero.clone(),
        valley_symmetry: zero,
        rings: None,
        hessian_scales_mm: vec![1.0],
        polarity: PolarityCheck {
            found: Polarity::BoneBright,
            confidence: 1.0,
            needs_confirm: false,
        },
        markers: vec![],
        seam_rows: vec![],
        valid_fraction: 1.0,
        coverage: 2.0 / 240.0,
        warnings: vec![],
    };
    let centre_line = CentreLine {
        film: FilmKind::Side,
        quarter_turns: QuarterTurns::NONE,
        scale_assumed: true,
        sample_step: 1.0,
        samples: (0..240)
            .map(|y| CentreSample {
                arc: f64::from(y),
                at: Px::new(60.0, f64::from(y)),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                width: 80.0,
                width_measured: true,
                evidence: 1.0,
            })
            .collect(),
        ends: CentreLineEnds {
            neck_base: None,
            iliac_crest: None,
            top_cropped: false,
            bottom_cropped: false,
            size_factor: 1.0,
        },
        warnings: vec![],
    };
    let strip = Strip {
        film: FilmKind::Side,
        rows: 240,
        cols: 3,
        row_step: 1.0,
        col_step: 1.0,
        scale_assumed: true,
        values: vec![0.0; 240 * 3],
        valid: vec![1; 240 * 3],
        layout: (0..240)
            .map(|y| StripRow {
                centre: Px::new(60.0, f64::from(y)),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                half_width: 40.0,
                z: -f64::from(y),
            })
            .collect(),
        image_right: Some(ImageRight::Anterior),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    };
    let barcode = Barcode {
        film: FilmKind::Side,
        values: vec![0.0; 240],
        high_pass: vec![0.0; 240],
        valid: vec![1; 240],
    };
    let cache = FilmPassCache {
        evidence,
        centre_line,
        strip,
        barcode,
    };
    let m = marks(&[(40.0, 130.0, Level::T8), (140.0, 180.0, Level::T9)]);
    let input = BoundaryWandInput {
        case_version: 27,
        marks: m.clone(),
        at: Px::new(60.0, 85.0),
        new_vertebra_id: 99,
    };
    let result = boundary_wand::apply(&f, None, Some(&cache), &input).expect("cached split");
    let upper = result.marks.vertebrae[0].marks.corners();
    let lower = result.marks.vertebrae[1].marks.corners();
    // Endplate candidates are on a 0.8 px grid, so a 1 px tolerance
    // allows the nearest candidate to either independently known edge.
    assert!((upper[2].at.y() - 78.0).abs() < 1.0, "{:?}", upper);
    assert!((lower[0].at.y() - 92.0).abs() < 1.0, "{:?}", lower);
    let before = boundary_wand::preview(
        &f,
        None,
        Some(&cache),
        &ToolPreviewInput {
            case_version: 27,
            tool: WandKind::BoundaryWand,
            at: input.at,
            marks: m,
            new_vertebra_id: 99,
        },
    )
    .expect("cached preview");
    let ToolPreview::Split { line, .. } = before else {
        panic!("split")
    };
    assert_eq!(
        line[0],
        Px::new(20.0, 0.5 * (upper[3].at.y() + lower[0].at.y()))
    );
}

#[test]
fn split_then_merge_preserves_rotated_and_mirrored_outer_corners() {
    for angle in [-0.6, -0.2, 0.0, 0.4, 0.7] {
        for mirror in [-1.0, 1.0] {
            let f = film(FilmKind::Side, 300, 300);
            let mut m = marks(&[(40.0, 130.0, Level::T8)]);
            let transform = |p: Px| {
                let x = (p.x() - 60.0) * mirror;
                let y = p.y() - 85.0;
                Px::new(
                    150.0 + x * libm::cos(angle) - y * libm::sin(angle),
                    150.0 + x * libm::sin(angle) + y * libm::cos(angle),
                )
            };
            if let VertebraMarks::Side { points } = &mut m.vertebrae[0].marks {
                for p in [
                    &mut points.sup_a,
                    &mut points.sup_p,
                    &mut points.inf_a,
                    &mut points.inf_p,
                ] {
                    p.at = transform(p.at);
                }
            }
            let split = click(&f, &m, Px::new(150.0, 150.0));
            let upper = split.marks.vertebrae[0].marks.corners();
            let lower = split.marks.vertebrae[1].marks.corners();
            let at = Px::new(
                (upper[2].at.x() + upper[3].at.x() + lower[0].at.x() + lower[1].at.x()) * 0.25,
                (upper[2].at.y() + upper[3].at.y() + lower[0].at.y() + lower[1].at.y()) * 0.25,
            );
            let merged = click(&f, &split.marks, at);
            // Retained outer corners are copied, so this requires exact
            // equality, including pin and source flags, with no tolerance.
            assert_eq!(merged.marks.vertebrae[0].marks, m.vertebrae[0].marks);
            assert_eq!(merged.marks.vertebrae[0].id, 1);
            assert_eq!(merged.marks.vertebrae[0].level, Level::T8);
        }
    }
}

#[test]
fn invalid_merge_has_the_same_dedicated_error_on_hover_and_click() {
    let f = film(FilmKind::Side, 400, 300);
    let mut m = marks(&[(40.0, 80.0, Level::T8), (90.0, 130.0, Level::T9)]);
    if let VertebraMarks::Side { points } = &mut m.vertebrae[1].marks {
        for mark in [
            &mut points.sup_a,
            &mut points.sup_p,
            &mut points.inf_a,
            &mut points.inf_p,
        ] {
            mark.at.0[0] += 180.0;
        }
    }
    // Both input rectangles have right angles. Joining their outer
    // endplates makes a 26.565-degree angle, below the 30-degree guard.
    assert_same_error(
        &f,
        m,
        Px::new(150.0, 85.0),
        shapes::errors::ErrorCode::ToolMergeInvalid,
    );
}

#[test]
fn infeasible_split_has_the_same_dedicated_error_on_hover_and_click() {
    let f = film(FilmKind::Side, 160, 300);
    let mut m = marks(&[
        (40.0, 120.0, Level::T7),
        (140.0, 160.0, Level::T8),
        (180.0, 260.0, Level::T9),
    ]);
    for index in [0, 2] {
        if let VertebraMarks::Side { points } = &mut m.vertebrae[index].marks {
            points.sup_a.pinned = true;
        }
    }
    // Fixed neighbours leave 60 px. Two bodies need at least 28 px each
    // and the three gaps need 4 px each: 68 px cannot fit in 60 px.
    assert_same_error(
        &f,
        m,
        Px::new(60.0, 150.0),
        shapes::errors::ErrorCode::ToolSplitInfeasible,
    );
}

fn assert_same_error(
    film: &DecodedFilm,
    marks: FilmMarks,
    at: Px,
    code: shapes::errors::ErrorCode,
) {
    let hover = boundary_wand::preview(
        film,
        None,
        None,
        &ToolPreviewInput {
            case_version: 27,
            tool: WandKind::BoundaryWand,
            at,
            marks: marks.clone(),
            new_vertebra_id: 99,
        },
    )
    .expect_err("hover refuses");
    let commit = boundary_wand::apply(
        film,
        None,
        None,
        &BoundaryWandInput {
            case_version: 27,
            marks,
            at,
            new_vertebra_id: 99,
        },
    )
    .expect_err("commit refuses");
    assert_eq!(hover.code, code);
    assert_eq!(hover, commit);
}
