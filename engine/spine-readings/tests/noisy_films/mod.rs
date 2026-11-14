//! Saved made-up angles are independent of the fit and of the reader.

use made_up_spines::answer_key::{answer_key, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use made_up_spines::seeded::Stream;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::landmarks::Landmarks;
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{FilmToFit, FitFromFilms, MarkSpread, TypicalSource};
use shapes::spine_readings::{Reading, ReadingsInput};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

fn mark(value: TrueMark) -> Mark {
    Mark {
        at: value.at,
        source: MarkSource::Hand,
        pinned: false,
        inferred: !value.visible,
    }
}

fn pedicle(value: TrueMark) -> PedicleMark {
    if value.visible {
        PedicleMark::Visible {
            at: value.at,
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

pub fn film(answer: &FilmAnswer, template: Template, size: [u32; 2]) -> FilmToFit {
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
                TrueMarks::Bending { .. } => unreachable!("standing views"),
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
    let levels = vertebrae.iter().map(|v| v.level).collect::<Vec<_>>();
    FilmToFit {
        marks: FilmMarks {
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
            named_for: answer.geometry.image_right.unwrap(),
        },
        geometry: answer.geometry.clone(),
        size,
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}

pub fn noisy(exact: &FitFromFilms, seed: u64, scale: f64, shared_on: bool) -> FitFromFilms {
    let mut request = exact.clone();
    let mut stream = Stream::new("trained front shared noise", seed);
    let (share, shared_share) = (0.11 * scale, if shared_on { 0.08 * scale } else { 0.0 });
    let has_side = request.side.is_some();
    let mut views: Vec<&mut FilmToFit> = vec![&mut request.front];
    if has_side {
        views.push(request.side.as_mut().unwrap());
    }
    for view in views {
        let spacing = view.geometry.mm_per_px.unwrap();
        let trained_view = view.marks.film == shapes::film::FilmKind::Front;
        if trained_view {
            view.mark_spread = MarkSpread::ShareOfHeight {
                share,
                shared_share,
            };
            view.marks.naming.from_top = None;
            view.marks.naming.from_bottom = None;
            view.marks.naming.flags = vec![shapes::naming::NamingFlag::Estimated];
        }
        for v in &mut view.marks.vertebrae {
            let corners = v.marks.corners();
            let top = (
                (corners[0].at.x() + corners[1].at.x()) / 2.0,
                (corners[0].at.y() + corners[1].at.y()) / 2.0,
            );
            let bottom = (
                (corners[2].at.x() + corners[3].at.x()) / 2.0,
                (corners[2].at.y() + corners[3].at.y()) / 2.0,
            );
            let height = libm::hypot(top.0 - bottom.0, top.1 - bottom.1) * spacing;
            let own_sd = if trained_view { share * height } else { 1.0 };
            let shared_sd = if trained_view {
                shared_share * height
            } else {
                0.0
            };
            let shared = if shared_sd > 0.0 {
                [
                    stream.normal(0.0, shared_sd) / spacing,
                    stream.normal(0.0, shared_sd) / spacing,
                ]
            } else {
                [0.0, 0.0]
            };
            let mut move_mark = |mark: &mut Mark, multiplier: f64| {
                let sd = own_sd * multiplier * if mark.inferred { 3.0 } else { 1.0 } / spacing;
                mark.at = Px::new(
                    mark.at.x() + stream.normal(0.0, sd) + shared[0],
                    mark.at.y() + stream.normal(0.0, sd) + shared[1],
                );
            };
            match &mut v.marks {
                VertebraMarks::Front { points } => {
                    if trained_view {
                        points.ped_l = PedicleMark::NotVisible {
                            source: MarkSource::Auto,
                        };
                        points.ped_r = PedicleMark::NotVisible {
                            source: MarkSource::Auto,
                        };
                    }
                    for mark in [
                        &mut points.sup_l,
                        &mut points.sup_r,
                        &mut points.inf_l,
                        &mut points.inf_r,
                    ] {
                        move_mark(mark, 1.0);
                    }
                }
                VertebraMarks::Side { points } => {
                    for mark in [
                        &mut points.sup_a,
                        &mut points.sup_p,
                        &mut points.inf_a,
                        &mut points.inf_p,
                    ] {
                        move_mark(mark, 1.0);
                    }
                }
                _ => unreachable!(),
            }
        }
        view.geometry.mm_per_px = None;
        view.geometry.mm_per_px_vertical = None;
    }
    if let Some(side) = request.side.as_mut()
        && let Some(shapes::landmarks::SacrumTop::Side { sac_a, sac_p }) =
            side.marks.landmarks.sacrum_top.as_mut()
    {
        let spacing = exact.side.as_ref().unwrap().geometry.mm_per_px.unwrap();
        let mut sacrum_noise = Stream::new("side sacrum mark noise", seed);
        for mark in [sac_a, sac_p] {
            mark.at = Px::new(
                mark.at.x() + sacrum_noise.normal(0.0, 1.0) / spacing,
                mark.at.y() + sacrum_noise.normal(0.0, 1.0) / spacing,
            );
        }
    }
    request
}

pub const RECIPES: [RecipeName; 3] = [
    RecipeName::RightThoracic40,
    RecipeName::DoubleMajor55To45,
    RecipeName::LumbarLeft30,
];

pub fn exact(name: RecipeName, side: bool) -> FitFromFilms {
    let recipe = MadeUpRecipe {
        name,
        seed: 4,
        tier: Tier::T0,
    };
    let patient = make_up_patient(name, recipe.seed).unwrap();
    let layouts: Vec<_> = film_recipes(&recipe, u64::from(recipe.seed))
        .iter()
        .map(|r| FilmLayout::new(r, spine_for(&patient, r.kind)))
        .collect();
    let key = answer_key(&recipe, &patient, &layouts);
    let mut side_film = film(&key.films[1], key.template, layouts[1].size);
    // A made-up sacral plateau spanning 60 mm across at 35 degrees, below the lumbar marks.
    // Its two marked ends carry the same independent 1 mm spread as the side corners.
    let spacing = side_film.geometry.mm_per_px.unwrap();
    let centre = side_film.marks.vertebrae.last().unwrap().marks.corners()[2].at;
    let half = 30.0 / spacing;
    let rise = half * libm::tan(35.0_f64.to_radians());
    side_film.marks.landmarks.sacrum_top = Some(shapes::landmarks::SacrumTop::Side {
        sac_a: Mark::auto(Px::new(centre.x() + half, centre.y() + 50.0 + rise)),
        sac_p: Mark::auto(Px::new(centre.x() - half, centre.y() + 50.0 - rise)),
    });
    FitFromFilms {
        front: film(&key.films[0], key.template, layouts[0].size),
        side: side.then_some(side_film),
        typical: TypicalSource::BuiltIn,
        twist_readings: Vec::new(),
    }
}

pub fn input(request: &FitFromFilms, fit: shapes::spine_fit::SpineFit) -> ReadingsInput {
    ReadingsInput {
        fit,
        front: Some(request.front.clone()),
        side: request.side.clone(),
        bend_left: None,
        bend_right: None,
        versions: 32,
        seed: 7,
    }
}

pub fn values(reading: Reading) -> Option<(f64, f64)> {
    match reading {
        Reading::Measured { value_deg, band } => Some((
            value_deg,
            (value_deg - band.p10)
                .abs()
                .max((band.p90 - value_deg).abs()),
        )),
        _ => None,
    }
}

pub fn main_curve(
    readings: &shapes::spine_readings::SpineReadings,
    truth: &shapes::spine_readings::CurveReading,
) -> Option<shapes::spine_readings::CurveReading> {
    readings
        .curves
        .iter()
        .filter(|c| c.region == truth.region && c.convex == truth.convex)
        .max_by(|a, b| {
            values(a.cobb)
                .unwrap()
                .0
                .abs()
                .total_cmp(&values(b.cobb).unwrap().0.abs())
        })
        .cloned()
}

pub fn overlap(
    a: &shapes::spine_readings::CurveReading,
    b: &shapes::spine_readings::CurveReading,
) -> bool {
    let names = Template::Standard.names();
    let index = |level| names.iter().position(|l| *l == level).unwrap();
    let (au, al, bu, bl) = (
        index(a.upper_end),
        index(a.lower_end),
        index(b.upper_end),
        index(b.lower_end),
    );
    let intersection = al.min(bl).saturating_sub(au.max(bu));
    let union = al.max(bl) - au.min(bu);
    a.convex == b.convex && intersection > 0 && 2 * intersection >= union
}
