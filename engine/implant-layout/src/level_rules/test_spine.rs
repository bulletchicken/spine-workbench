//! Made-up geometry for the level rules' independently worked examples.

use super::{FusionRegion, RuleContext};
use crate::curves::{SpineReading, lumbar::PedicleOutlines};
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::implant_layout::{ChildFacts, LayoutFilm, LayoutInput, NamingDoubt};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::*;
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

pub fn input() -> LayoutInput {
    let tilts: [f64; 17] = [
        -15., -14., -12., -10., -7., -3., 0., 4., 8., 12., 15., 10., 4., -3., -9., -5., 0.,
    ];
    let vertebrae = Level::MEASURED
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let p = VertebraParams {
                centre: nalgebra::Vector3::new(
                    0.,
                    if i == 7 { -18. } else { 0. },
                    500. - i as f64 * 28.,
                ),
                rotation: nalgebra::Vector3::new(tilts[i].to_radians(), 0., 0.),
                height: 20.,
                wedge_front: 0.,
                wedge_side: 0.,
                top_half_width: 15.,
                bottom_half_width: 15.,
                top_half_depth: 10.,
                bottom_half_depth: 10.,
            };
            FittedVertebra {
                level,
                pose: p.pose(level),
                angles: FitAngles {
                    side_tilt_rad: tilts[i].to_radians(),
                    forward_lean_rad: 0.,
                    twist_rad: 0.,
                },
                sd: FitStandardDeviations {
                    centre_mm: [1.; 3],
                    angles_rad: [1_f64.to_radians(); 3],
                },
                inferred: InferredFit {
                    depth: false,
                    width: false,
                    twist: false,
                },
                rms_mm: FitRmsMm {
                    front: Some(0.),
                    side: Some(0.),
                },
                corner_tilt_rad: None,
            }
        })
        .collect();
    let mut front = film(
        FilmKind::Front,
        vec![(
            Level::L5,
            VertebraMarks::Front {
                points: FrontMarks {
                    sup_l: mark(115., 90.),
                    sup_r: mark(85., 90.),
                    inf_l: mark(115., 110.),
                    inf_r: mark(85., 110.),
                    ped_l: PedicleMark::NotVisible {
                        source: MarkSource::Auto,
                    },
                    ped_r: PedicleMark::NotVisible {
                        source: MarkSource::Auto,
                    },
                },
            },
        )],
    );
    front.marks.landmarks.sacrum_top = Some(SacrumTop::Front {
        sac_l: mark(110., 120.),
        sac_r: mark(90., 120.),
    });
    let bends = |kind| {
        film(
            kind,
            Level::MEASURED
                .into_iter()
                .map(|l| {
                    (
                        l,
                        VertebraMarks::Bending {
                            lines: EndplateLines {
                                sup: [mark(110., 100.), mark(90., 100.)],
                                inf: [mark(110., 110.), mark(90., 110.)],
                            },
                        },
                    )
                })
                .collect(),
        )
    };
    LayoutInput {
        fit: SpineFit {
            vertebrae,
            side_scale: 1.,
            side_offset_mm: 0.,
            origin_level: Level::L5,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.,
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
        front: Some(front),
        bend_left: Some(bends(FilmKind::BendLeft)),
        bend_right: Some(bends(FilmKind::BendRight)),
        child: ChildFacts {
            age_years: Some(14.),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 4,
        seed: 9,
        pedicle_child_share: None,
    }
}

pub fn reading(input: &LayoutInput) -> SpineReading {
    crate::curves::measure_with_pedicle_outlines(input, &input.fit, &outlines(input)).unwrap()
}

pub fn outlines(input: &LayoutInput) -> Vec<PedicleOutlines> {
    input
        .fit
        .vertebrae
        .iter()
        .map(|v| {
            let centres = pedicle_centres(&VertebraParams::from(&v.pose));
            PedicleOutlines {
                level: v.level,
                left_y_mm: [centres[0].y - 2., centres[0].y + 2.],
                right_y_mm: [centres[1].y - 2., centres[1].y + 2.],
            }
        })
        .collect()
}

pub struct Fixture {
    pub input: LayoutInput,
    pub reading: SpineReading,
    pub outlines: Vec<PedicleOutlines>,
}
impl Fixture {
    pub fn new() -> Self {
        let input = input();
        let outlines = outlines(&input);
        let reading = reading(&input);
        Self {
            input,
            reading,
            outlines,
        }
    }
    pub fn context(&self) -> RuleContext<'_> {
        RuleContext {
            input: &self.input,
            fit: &self.input.fit,
            reading: &self.reading,
            outlines: &self.outlines,
            region: FusionRegion::Structural,
        }
    }
}

fn mark(x: f64, y: f64) -> Mark {
    Mark::auto(Px::new(x, y))
}
fn film(kind: FilmKind, marks: Vec<(Level, VertebraMarks)>) -> LayoutFilm {
    LayoutFilm {
        geometry: FilmGeometry {
            mm_per_px: Some(1.),
            image_right: Some(ImageRight::PatientLeft),
            ..FilmGeometry::UNKNOWN
        },
        size: [500, 600],
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
            vertebrae: marks
                .into_iter()
                .enumerate()
                .map(|(i, (level, marks))| VertebraOnFilm {
                    id: i as u32,
                    level,
                    marks,
                    confidence: Confidence {
                        score: 1.,
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
fn typical_lengths_refuse_each_comparison_with_published_millimetres() {
    use shapes::implant_layout::{CannotTellCode, LumbarModifier};
    let mut fixture = Fixture::new();
    fixture.input.fit.lengths = FitLengths::TypicalSize;
    // A readable sacrum and a settled modifier isolate the physical-scale
    // requirement from missing marks or classification uncertainty.
    fixture.reading.classification.lumbar_modifier = Ok(LumbarModifier::BetweenPedicles);
    let ctx = fixture.context();
    for rule in [
        super::deviated_vertebra::evaluate,
        super::substantially_touched::evaluate,
        super::stable_minus_one::evaluate,
    ] {
        assert_eq!(rule(&ctx).unwrap().level, Err(CannotTellCode::NoFilmScale));
    }
    let mut reading = fixture.reading.clone();
    super::ensemble::classify_lumbar_against_table_widths(
        &mut reading,
        &fixture.input.fit,
        &fixture.outlines,
    )
    .unwrap();
    assert_eq!(
        reading.classification.lumbar_modifier,
        Err(CannotTellCode::NoFilmScale)
    );
    // Contact with the drawn body and bracketing by its centres compare
    // distances in the same drawing, so they retain a reading.
    assert!(super::last_touched::landmark(&ctx).is_ok());
    assert!(super::stable_minus_one::stable(&ctx).is_ok());
}
