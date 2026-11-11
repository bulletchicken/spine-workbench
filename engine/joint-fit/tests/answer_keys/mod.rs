//! Inputs drawn from the made-up spine's saved answer key, without painting films.

use forward_model::cameras::parallel::ParallelCamera;
use forward_model::rotation::matrix_from_rotation_vector;
use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::click_noise::clicked_vertebrae;
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{
    AnswerKey, FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks, VertebraPose,
};
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::FitInput;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

pub fn input(name: RecipeName, seed: u32, noise_mm: f64) -> (FitInput, AnswerKey) {
    with_pixel_aspect(name, seed, noise_mm, 1.0)
}

pub fn anisotropic_input(name: RecipeName, seed: u32) -> (FitInput, AnswerKey) {
    with_pixel_aspect(name, seed, 0.0, 2.0)
}

fn with_pixel_aspect(
    name: RecipeName,
    seed: u32,
    noise_mm: f64,
    aspect: f64,
) -> (FitInput, AnswerKey) {
    let recipe = MadeUpRecipe {
        name,
        seed,
        tier: Tier::T0,
    };
    let patient = make_up_patient(name, seed).expect("the named recipe stacks");
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(seed))
        .iter()
        .map(|film| {
            let mut layout = FilmLayout::new(film, spine_for(&patient, film.kind));
            if aspect != 1.0 {
                layout.geometry.mm_per_px_vertical =
                    layout.geometry.mm_per_px.map(|mm| mm * aspect);
                layout.camera =
                    ParallelCamera::for_film(&layout.geometry, layout.size, layout.camera.origin)
                        .expect("the anisotropic camera is known");
            }
            layout
        })
        .collect();
    let mut key = answer_key(&recipe, &patient, &layouts);
    // T1 to L5 is the 17-vertebra fit; C7 belongs to the search's chain.
    key.poses.retain(|pose| pose.level >= Level::T1);
    for film in &mut key.films {
        film.vertebrae
            .retain(|vertebra| vertebra.level >= Level::T1);
    }
    let front = marks(&key.films[0], key.template, noise_mm, u64::from(seed) + 83);
    let side = marks(&key.films[1], key.template, noise_mm, u64::from(seed) + 83);
    (
        FitInput {
            front,
            side,
            front_geometry: key.films[0].geometry.clone(),
            side_geometry: key.films[1].geometry.clone(),
            front_size: layouts[0].size,
            side_size: layouts[1].size,
        },
        key,
    )
}

fn marks(answer: &FilmAnswer, template: Template, noise_mm: f64, seed: u64) -> FilmMarks {
    let vertebrae: Vec<_> = clicked_vertebrae(answer, noise_mm, seed)
        .iter()
        .enumerate()
        .map(|(index, truth)| VertebraOnFilm {
            id: 1 + index as u32,
            level: truth.level,
            marks: match truth.marks {
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
                TrueMarks::Bending { .. } => panic!("only standing films are fitted"),
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
    let levels: Vec<_> = vertebrae.iter().map(|vertebra| vertebra.level).collect();
    FilmMarks {
        film: answer.kind,
        vertebrae,
        landmarks: Landmarks::NONE,
        naming: LevelNaming {
            template,
            chosen_by: ChosenBy::User,
            alternatives: Vec::new(),
            from_top: Some(levels.clone()),
            from_bottom: Some(levels),
            flags: Vec::new(),
            l5_confirmed_on: None,
        },
        named_for: answer
            .geometry
            .image_right
            .expect("the answer knows its facing"),
    }
}

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

/// Independent reading of R = Rx(tilt) Ry(lean) Rz(twist): R02 = sin(lean),
/// R12/R22 = -tan(tilt), R01/R00 = -tan(twist), within the made-up ranges.
pub fn angles(pose: &VertebraPose) -> [f64; 3] {
    let rotation = matrix_from_rotation_vector(&pose.rotation.into());
    [
        libm::atan2(-rotation[(1, 2)], rotation[(2, 2)]),
        libm::asin(rotation[(0, 2)].clamp(-1.0, 1.0)),
        libm::atan2(-rotation[(0, 1)], rotation[(0, 0)]),
    ]
}

pub fn sizes(pose: &VertebraPose) -> [f64; 5] {
    [
        pose.height,
        pose.top_half_width,
        pose.top_half_depth,
        pose.bottom_half_width,
        pose.bottom_half_depth,
    ]
}

pub fn distance(first: [f64; 3], second: [f64; 3]) -> f64 {
    first
        .into_iter()
        .zip(second)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

pub fn median(values: &mut [f64]) -> f64 {
    assert!(!values.is_empty());
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        0.5 * (values[mid - 1] + values[mid])
    } else {
        values[mid]
    }
}

/// Three identical upright bodies: every required prior is exactly zero.
/// At 0.5 mm/px, front origin (200,400), side origin (300,500), each
/// half-width is 30 px, half-depth 20 px, half-height 20 px. Pedicles
/// are 0.78*15=11.7 mm to either side and 0.18*20=3.6 mm above centre.
pub fn upright_input() -> FitInput {
    let (mut input, _) = input(RecipeName::Straight, 3, 0.0);
    input.front.vertebrae.retain(|v| v.level >= Level::L3);
    input.side.vertebrae.retain(|v| v.level >= Level::L3);
    input.front_size = [1000, 1000];
    input.side_size = [1000, 1000];
    input.front_geometry = FilmGeometry {
        mm_per_px: Some(0.5),
        image_right: Some(ImageRight::PatientLeft),
        ..FilmGeometry::UNKNOWN
    };
    input.side_geometry = FilmGeometry {
        mm_per_px: Some(0.5),
        image_right: Some(ImageRight::Anterior),
        ..FilmGeometry::UNKNOWN
    };
    input.front.named_for = ImageRight::PatientLeft;
    input.side.named_for = ImageRight::Anterior;
    let at = |x, y| Mark::auto(Px::new(x, y));
    for (index, z) in [70.0, 40.0, 10.0].into_iter().enumerate() {
        let pedicle = |x| PedicleMark::Visible {
            at: Px::new(x, 400.0 - 2.0 * (z + 3.6)),
            source: MarkSource::Hand,
            pinned: false,
            inferred: false,
            ring_score: None,
        };
        input.front.vertebrae[index].marks = VertebraMarks::Front {
            points: FrontMarks {
                sup_l: at(230.0, 400.0 - 2.0 * (z + 10.0)),
                sup_r: at(170.0, 400.0 - 2.0 * (z + 10.0)),
                inf_l: at(230.0, 400.0 - 2.0 * (z - 10.0)),
                inf_r: at(170.0, 400.0 - 2.0 * (z - 10.0)),
                ped_l: pedicle(223.4),
                ped_r: pedicle(176.6),
            },
        };
        input.side.vertebrae[index].marks = VertebraMarks::Side {
            points: SideMarks {
                sup_a: at(320.0, 500.0 - 2.0 * (z + 10.0)),
                sup_p: at(280.0, 500.0 - 2.0 * (z + 10.0)),
                inf_a: at(320.0, 500.0 - 2.0 * (z - 10.0)),
                inf_p: at(280.0, 500.0 - 2.0 * (z - 10.0)),
            },
        };
    }
    for film in [&mut input.front, &mut input.side] {
        let levels: Vec<_> = film.vertebrae.iter().map(|v| v.level).collect();
        film.naming.from_top = Some(levels.clone());
        film.naming.from_bottom = Some(levels);
    }
    input
}
