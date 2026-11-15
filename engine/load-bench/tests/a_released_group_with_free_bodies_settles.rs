//! A long screw-connected block can contain vertebrae with no heads.

use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::implant_layout::{AnchorKind, ChildFacts, LayoutFilm, LayoutInput, NamingDoubt};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::load_bench::BenchInput;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming};
use shapes::spine_fit::{FilmToFit, FitFromFilms, MarkSpread, TypicalSource};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

#[test]
#[ignore = "32-version nonlinear release regression; run with --release"]
fn every_version_settles_with_free_bodies_inside_a_long_released_group() {
    let recipe = MadeUpRecipe {
        name: RecipeName::DoubleMajor55To45,
        seed: 2,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).unwrap();
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|film| FilmLayout::new(film, spine_for(&patient, film.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let films: Vec<_> = key
        .films
        .iter()
        .zip(&layouts)
        .map(|(answer, layout)| film(answer, key.template, layout.size))
        .collect();
    let fit = joint_fit::fit_from_films(&FitFromFilms {
        front: films[0].clone(),
        side: Some(films[1].clone()),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    })
    .unwrap();
    let layout_film = |film: &FilmToFit| LayoutFilm {
        marks: film.marks.clone(),
        geometry: film.geometry.clone(),
        size: film.size,
    };
    let child = ChildFacts {
        age_years: None,
        weight_kg: None,
        activity: None,
    };
    let proposals = implant_layout::propose_layouts(&LayoutInput {
        fit: fit.clone(),
        front: Some(layout_film(&films[0])),
        bend_left: Some(layout_film(&films[2])),
        bend_right: Some(layout_film(&films[3])),
        child,
        naming: NamingDoubt::AsRead,
        versions: 32,
        seed: recipe.seed,
        pedicle_child_share: None,
    })
    .unwrap();
    let placement = proposals
        .layouts
        .into_iter()
        .find(|layout| layout.id == "starts-at-T4")
        .expect("the long upper extension");
    assert_eq!((placement.upper, placement.lower), (Level::T4, Level::L3));
    assert_eq!(placement.rods.len(), 2);
    for level in [Level::T4, Level::L3] {
        assert!(
            placement
                .anchors
                .iter()
                .any(|a| a.level == level && a.kind == AnchorKind::Screw)
        );
    }
    for level in [Level::T5, Level::T6] {
        let anchors: Vec<_> = placement
            .anchors
            .iter()
            .filter(|a| a.level == level)
            .collect();
        assert_eq!(anchors.len(), 2);
        assert!(anchors.iter().all(|a| a.kind == AnchorKind::None));
    }
    // A bilateral screw pair connects the rods into one block. Each hook
    // sits on a body already screwed to it, so no hook joins distinct groups.
    // Release retains only the two horizontal T1 constraint rows.
    assert!(placement.anchors.iter().any(|a| {
        a.kind == AnchorKind::Screw
            && placement
                .anchors
                .iter()
                .any(|b| b.level == a.level && b.side != a.side && b.kind == AnchorKind::Screw)
    }));
    assert!(placement.anchors.iter().any(|a| a.kind == AnchorKind::Hook));
    for hook in placement
        .anchors
        .iter()
        .filter(|a| a.kind == AnchorKind::Hook)
    {
        assert!(
            placement
                .anchors
                .iter()
                .any(|a| { a.level == hook.level && a.kind == AnchorKind::Screw })
        );
    }
    let batch = load_bench::run_load_bench(&BenchInput {
        fit,
        layouts: vec![placement],
        child,
        seed: recipe.seed,
        versions: 32,
        version_from: 0,
        version_count: 32,
    })
    .unwrap();
    assert_eq!(batch.runs.len(), 32);
    let unsettled: Vec<_> = batch
        .runs
        .iter()
        .filter(|r| !r.settled)
        .map(|r| r.version)
        .collect();
    assert!(unsettled.is_empty(), "unsettled versions: {unsettled:?}");
    assert!(batch.middle[0].rods_in.is_some());
}

fn film(answer: &FilmAnswer, template: shapes::naming::Template, size: [u32; 2]) -> FilmToFit {
    let vertebrae: Vec<_> = answer
        .vertebrae
        .iter()
        .filter(|v| v.level >= Level::T1)
        .enumerate()
        .map(|(index, truth)| VertebraOnFilm {
            id: index as u32 + 1,
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
    let levels: Vec<_> = vertebrae.iter().map(|v| v.level).collect();
    let [first, _, second] = answer.landmarks.sacrum_top;
    let sacrum_top = Some(if answer.kind == shapes::film::FilmKind::Side {
        SacrumTop::Side {
            sac_a: mark(first),
            sac_p: mark(second),
        }
    } else {
        SacrumTop::Front {
            sac_l: mark(first),
            sac_r: mark(second),
        }
    });
    FilmToFit {
        marks: FilmMarks {
            film: answer.kind,
            vertebrae,
            landmarks: Landmarks {
                c7: None,
                sacrum_top,
                hips: Vec::new(),
            },
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
                .expect("a made-up film has a facing"),
        },
        geometry: answer.geometry.clone(),
        size,
        // Exact marks, with a stated allowance for reading their position.
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 0.5,
            shared_sd_mm: 0.0,
        },
        // A made-up film states its facing.
        facing_assumed: false,
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
