//! Hand-specified findings for small made-up cases. Checks return categorical
//! answers, so all comparisons are exact; no numerical tolerance is needed.

use super::*;
use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason};
use shapes::film::{Film, FilmImage, ModalityMap, PixelValues, SourceFormat};
use shapes::film_geometry::{Confirmations, FilmGeometry, ImageRight, ScaleSource};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks};

const MEASURED: [Level; 17] = [
    Level::T1,
    Level::T2,
    Level::T3,
    Level::T4,
    Level::T5,
    Level::T6,
    Level::T7,
    Level::T8,
    Level::T9,
    Level::T10,
    Level::T11,
    Level::T12,
    Level::L1,
    Level::L2,
    Level::L3,
    Level::L4,
    Level::L5,
];

fn film(kind: FilmKind) -> Film {
    Film {
        kind,
        image: FilmImage {
            path: kind.png_path().into(),
            width: 1000,
            height: 4000,
            bits_stored: 8,
        },
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues {
            offset: 0,
            inverted: false,
            inverted_by_user: false,
            modality: ModalityMap::Linear {
                slope: 1.0,
                intercept: 0.0,
            },
        },
        flags: Vec::new(),
        blanked: Vec::new(),
        geometry: FilmGeometry {
            mm_per_px: Some(0.2),
            scale: Some(ScaleSource::KnownObject {
                centre: Px::new(50.0, 50.0),
                radius_px: 50.0,
                diameter_mm: 20.0,
            }),
            image_right: Some(if kind == FilmKind::Side {
                ImageRight::Anterior
            } else {
                ImageRight::PatientLeft
            }),
            confirmed: Confirmations {
                upright: true,
                left_right: kind != FilmKind::Side,
                facing: kind == FilmKind::Side,
                scale: true,
                polarity: true,
                privacy: true,
            },
            ..FilmGeometry::UNKNOWN
        },
    }
}

fn vertebra(id: u32, level: Level, kind: FilmKind) -> VertebraOnFilm {
    let y = f64::from(id) * 20.0;
    let corners = [
        Mark::auto(Px::new(60.0, y)),
        Mark::auto(Px::new(40.0, y)),
        Mark::auto(Px::new(40.0, y + 10.0)),
        Mark::auto(Px::new(60.0, y + 10.0)),
    ];
    let pedicle = PedicleMark::Visible {
        at: Px::new(50.0, y + 5.0),
        source: MarkSource::Auto,
        pinned: false,
        inferred: false,
        ring_score: Some(0.9),
    };
    VertebraOnFilm {
        id,
        level,
        marks: if kind == FilmKind::Front {
            VertebraMarks::Front {
                points: FrontMarks {
                    sup_l: corners[0],
                    sup_r: corners[1],
                    inf_r: corners[2],
                    inf_l: corners[3],
                    ped_l: pedicle,
                    ped_r: pedicle,
                },
            }
        } else {
            VertebraMarks::Side {
                points: SideMarks {
                    sup_a: corners[0],
                    sup_p: corners[1],
                    inf_p: corners[2],
                    inf_a: corners[3],
                },
            }
        },
        confidence: Confidence {
            score: 0.95,
            band: ConfidenceBand::High,
            reasons: Vec::new(),
            calibrated: false,
            endplate_spread_px: None,
            margin: None,
        },
        outline: None,
    }
}

fn marks(kind: FilmKind, first_id: u32) -> FilmMarks {
    FilmMarks {
        film: kind,
        vertebrae: MEASURED
            .into_iter()
            .enumerate()
            .map(|(index, level)| vertebra(first_id + index as u32, level, kind))
            .collect(),
        landmarks: Landmarks::NONE,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: Vec::new(),
            from_top: Some(MEASURED.to_vec()),
            from_bottom: Some(MEASURED.to_vec()),
            flags: Vec::new(),
            l5_confirmed_on: Some(first_id + 16),
        },
        named_for: if kind == FilmKind::Side {
            ImageRight::Anterior
        } else {
            ImageRight::PatientLeft
        },
    }
}

fn complete() -> CaseFileV1 {
    let mut case = CaseFileV1::new("0.1.0");
    case.films = vec![film(FilmKind::Front), film(FilmKind::Side)];
    case.marks = vec![marks(FilmKind::Front, 1), marks(FilmKind::Side, 101)];
    case.next_vertebra_id = 118;
    case
}

fn expected(
    kind: CheckKind,
    severity: Severity,
    film: Option<FilmKind>,
    vertebra_id: Option<u32>,
    level: Option<Level>,
) -> CheckFinding {
    CheckFinding {
        kind,
        severity,
        film,
        vertebra_id,
        level,
    }
}

#[test]
fn complete_confirmed_case_has_no_findings_and_is_unchanged() {
    let case = complete();
    shapes::reading::check_case_file(&case).unwrap();
    let before = case.clone();
    assert_eq!(check_case(&case), Vec::new());
    assert_eq!(case, before);
    assert_eq!(check_case(&CaseFileV1::new("0.1.0")), Vec::new());
}

#[test]
fn missing_level_and_tall_outline_have_distinct_targets() {
    let mut case = complete();
    case.marks[0]
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::T8);
    case.marks[0].vertebrae[7]
        .confidence
        .reasons
        .push(ConfidenceReason::OddHeight);
    assert_eq!(
        check_case(&case),
        vec![
            expected(
                CheckKind::VertebraMissing,
                Severity::Blocking,
                Some(FilmKind::Front),
                Some(9),
                Some(Level::T9)
            ),
            expected(
                CheckKind::VertebraMissing,
                Severity::Blocking,
                Some(FilmKind::Front),
                None,
                Some(Level::T8)
            ),
            expected(
                CheckKind::FilmsDisagree,
                Severity::Blocking,
                None,
                None,
                None
            ),
        ]
    );
}

#[test]
fn presence_uses_each_count_within_the_measured_range() {
    let mut front = marks(FilmKind::Front, 1);
    front.naming.template = Template::ExtraLumbar;
    // C7 and L6 are not required by the measured T1–L5 check.
    assert_eq!(presence::check(&front), Vec::new());
    front.naming.template = Template::ExtraThoracic;
    assert_eq!(
        presence::check(&front),
        vec![expected(
            CheckKind::VertebraMissing,
            Severity::Blocking,
            Some(FilmKind::Front),
            None,
            Some(Level::T13)
        ),]
    );
    front.naming.template = Template::MissingLumbar;
    front
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::L5);
    assert_eq!(presence::check(&front), Vec::new());
    front.naming.template = Template::MissingThoracic;
    front
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::T12);
    assert_eq!(
        presence::check(&front),
        vec![expected(
            CheckKind::VertebraMissing,
            Severity::Blocking,
            Some(FilmKind::Front),
            None,
            Some(Level::L5)
        ),]
    );
    front.film = FilmKind::BendLeft;
    assert_eq!(presence::check(&front), Vec::new());
}

#[test]
fn repeated_and_reversed_levels_are_reported_once_per_film() {
    let mut front = marks(FilmKind::Front, 1);
    front.vertebrae.swap(2, 3);
    let finding = Some(expected(
        CheckKind::LevelsOutOfOrder,
        Severity::Blocking,
        Some(FilmKind::Front),
        None,
        None,
    ));
    assert_eq!(order::check(&front), finding);
    front.vertebrae[0].level = Level::T3;
    front.vertebrae[1].level = Level::T3;
    assert_eq!(order::check(&front), finding);
    front.vertebrae.clear();
    assert_eq!(order::check(&front), None);
}

#[test]
fn l5_confirmation_settles_an_end_count_disagreement() {
    let mut case = complete();
    case.marks[1].naming.flags = vec![NamingFlag::EndsDisagree];
    case.marks[1].naming.l5_confirmed_on = None;
    assert_eq!(
        check_case(&case),
        vec![
            expected(
                CheckKind::LikelyOffByOne,
                Severity::Warning,
                Some(FilmKind::Side),
                Some(117),
                Some(Level::L5)
            ),
            expected(
                CheckKind::L5Unconfirmed,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
        ]
    );
    case.marks[1].naming.l5_confirmed_on = Some(117);
    assert_eq!(check_case(&case), Vec::new());
    case.marks[1].naming.l5_confirmed_on = None;
    case.marks[1].naming.flags.clear();
    assert_eq!(
        check_case(&case),
        vec![expected(
            CheckKind::L5Unconfirmed,
            Severity::Warning,
            Some(FilmKind::Side),
            None,
            None
        ),]
    );
}

#[test]
fn unusual_count_points_outside_the_template_and_unknown_names_block() {
    let mut front = marks(FilmKind::Front, 1);
    front
        .vertebrae
        .push(vertebra(18, Level::L6, FilmKind::Front));
    front.naming.flags = vec![NamingFlag::NonStandardCount, NamingFlag::NamesUnknown];
    assert_eq!(
        off_by_one::check(&front),
        vec![
            expected(
                CheckKind::NonStandardCount,
                Severity::Warning,
                Some(FilmKind::Front),
                Some(18),
                Some(Level::L6)
            ),
            expected(
                CheckKind::NamesUnknown,
                Severity::Blocking,
                Some(FilmKind::Front),
                None,
                None
            ),
        ]
    );
    front.naming.template = Template::ExtraLumbar;
    front.naming.flags = vec![NamingFlag::NonStandardCount];
    assert_eq!(
        off_by_one::check(&front),
        vec![expected(
            CheckKind::NonStandardCount,
            Severity::Warning,
            Some(FilmKind::Front),
            None,
            None
        ),]
    );
}

#[test]
fn each_hidden_pedicle_is_a_warning_about_its_vertebra() {
    let mut case = complete();
    let VertebraMarks::Front { points } = &mut case.marks[0].vertebrae[2].marks else {
        unreachable!()
    };
    points.ped_l = PedicleMark::NotVisible {
        source: MarkSource::Hand,
    };
    points.ped_r = PedicleMark::NotVisible {
        source: MarkSource::Auto,
    };
    let hidden = expected(
        CheckKind::PedicleNotVisible,
        Severity::Warning,
        Some(FilmKind::Front),
        Some(3),
        Some(Level::T3),
    );
    assert_eq!(check_case(&case), vec![hidden.clone(), hidden]);
}

#[test]
fn films_compare_names_not_ids_or_mark_positions() {
    let mut case = complete();
    assert_eq!(films_agree::check(&case), None);
    case.marks[1].vertebrae.swap(0, 1);
    assert_eq!(
        films_agree::check(&case),
        Some(expected(
            CheckKind::FilmsDisagree,
            Severity::Blocking,
            None,
            None,
            None
        ))
    );
    case.marks[1].film = FilmKind::BendRight;
    assert_eq!(films_agree::check(&case), None);
}

#[test]
fn unknown_or_changed_facing_leaves_mark_names_unconfirmed() {
    let mut case = complete();
    case.films[1].geometry.image_right = None;
    let mismatch = expected(
        CheckKind::MarksNamedForOtherFacing,
        Severity::Blocking,
        Some(FilmKind::Side),
        None,
        None,
    );
    assert_eq!(check_case(&case), vec![mismatch.clone()]);
    case.films[1].geometry.image_right = Some(ImageRight::Posterior);
    assert_eq!(check_case(&case), vec![mismatch]);
    case.marks.pop();
    assert_eq!(check_case(&case), Vec::new());
}

#[test]
fn all_unconfirmed_facts_are_film_specific_even_before_the_pass() {
    let mut case = CaseFileV1::new("0.1.0");
    case.films = vec![
        film(FilmKind::Front),
        film(FilmKind::Side),
        film(FilmKind::BendLeft),
    ];
    for film in &mut case.films {
        film.geometry = FilmGeometry::UNKNOWN;
    }
    let findings = check_case(&case);
    assert_eq!(
        findings,
        vec![
            expected(
                CheckKind::UprightUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Front),
                None,
                None
            ),
            expected(
                CheckKind::LeftRightUnconfirmed,
                Severity::Blocking,
                Some(FilmKind::Front),
                None,
                None
            ),
            expected(
                CheckKind::ScaleUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Front),
                None,
                None
            ),
            expected(
                CheckKind::PolarityUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Front),
                None,
                None
            ),
            expected(
                CheckKind::PrivacyUnchecked,
                Severity::Warning,
                Some(FilmKind::Front),
                None,
                None
            ),
            expected(
                CheckKind::UprightUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
            expected(
                CheckKind::FacingUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
            expected(
                CheckKind::ScaleUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
            expected(
                CheckKind::PolarityUnconfirmed,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
            expected(
                CheckKind::PrivacyUnchecked,
                Severity::Warning,
                Some(FilmKind::Side),
                None,
                None
            ),
            expected(
                CheckKind::UprightUnconfirmed,
                Severity::Warning,
                Some(FilmKind::BendLeft),
                None,
                None
            ),
            expected(
                CheckKind::LeftRightUnconfirmed,
                Severity::Warning,
                Some(FilmKind::BendLeft),
                None,
                None
            ),
            expected(
                CheckKind::ScaleUnconfirmed,
                Severity::Warning,
                Some(FilmKind::BendLeft),
                None,
                None
            ),
            expected(
                CheckKind::PolarityUnconfirmed,
                Severity::Warning,
                Some(FilmKind::BendLeft),
                None,
                None
            ),
            expected(
                CheckKind::PrivacyUnchecked,
                Severity::Warning,
                Some(FilmKind::BendLeft),
                None,
                None
            ),
        ]
    );
    assert_eq!(check_case(&case), findings);
}
