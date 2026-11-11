//! Hand-worked naming and confidence cases, independent of search priors.

use auto_pass::confidence::per_vertebra;
use auto_pass::handoffs::{
    BodyCertainty, ChainBoundaries, EndplateOnStrip, FoundBody, Polarity, Strip, StripEvidence,
    StripRow,
};
use auto_pass::level_naming::{self, AnchorEvidence};
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::confidence::{ConfidenceBand, ConfidenceReason};
use shapes::errors::ErrorCode;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::levels::Level;
use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template};
use shapes::pixels::{Px, PxRect};
use shapes::watch_frames::WatchFrame;

fn chain(template: Template, start: f64) -> ChainBoundaries {
    ChainBoundaries {
        film: FilmKind::Front,
        template,
        polarity: Polarity::BoneBright,
        bodies: template
            .levels()
            .iter()
            .enumerate()
            .map(|(i, &level)| FoundBody {
                level,
                sup: EndplateOnStrip {
                    row: start + 25.0 * i as f64,
                    tilt: 0.0,
                },
                inf: EndplateOnStrip {
                    row: start + 25.0 * i as f64 + 20.0,
                    tilt: 0.0,
                },
                inferred: false,
                certainty: BodyCertainty {
                    posterior: 0.97,
                    sup_spread: 2.0,
                    inf_spread: 3.0,
                    margin: 8.0,
                },
            })
            .collect(),
        sacrum: Some(EndplateOnStrip {
            row: start + 25.0 * template.levels().len() as f64,
            tilt: 0.0,
        }),
        energy: -10.0,
        warnings: Vec::new(),
    }
}

fn anchors() -> AnchorEvidence {
    AnchorEvidence {
        first_rib: 4.0,
        competing_first_rib: Some(1.0),
        sacrum: 4.0,
        ridge_present: true,
    }
}

fn energies(best: Template, energy: f64) -> Vec<(Template, f64)> {
    Template::ALL
        .into_iter()
        .map(|template| (template, if template == best { energy } else { 0.0 }))
        .collect()
}

fn named(chain: &ChainBoundaries) -> LevelNaming {
    let selection = level_naming::rank_templates(&energies(chain.template, -10.0), 1.0).unwrap();
    level_naming::name_chain(chain, &selection, chain, chain, anchors())
}

#[test]
fn equal_evidence_gives_the_experimental_prior_probabilities() {
    let selection = level_naming::rank_templates(&energies(Template::Standard, 0.0), 1.0).unwrap();
    assert_eq!(selection.selected, Template::Standard);
    let expected = [
        (Template::Standard, 0.90),
        (Template::ExtraLumbar, 0.035),
        (Template::MissingLumbar, 0.025),
        (Template::ExtraThoracic, 0.020),
        (Template::MissingThoracic, 0.020),
    ];
    for (template, expected) in expected {
        let actual = selection
            .scores
            .iter()
            .find(|score| score.template == template)
            .unwrap();
        // Analytically softmax(log(prior)) is prior because these sum to 1.
        assert!((actual.chance - expected).abs() < 1e-14);
    }
    assert!(
        selection
            .scores
            .windows(2)
            .all(|pair| pair[0].score <= pair[1].score)
    );
}

#[test]
fn six_lumbar_and_eleven_rib_readings_are_never_silently_standard() {
    for template in [Template::ExtraLumbar, Template::MissingThoracic] {
        // -10 overcomes each variant's prior penalty and the 3-unit margin.
        let selection = level_naming::rank_templates(&energies(template, -10.0), 1.0).unwrap();
        assert_eq!(selection.selected, template);
        let chain = chain(template, 0.0);
        let naming = level_naming::name_chain(&chain, &selection, &chain, &chain, anchors());
        assert_eq!(naming.template, template);
        assert_eq!(naming.flags, [NamingFlag::NonStandardCount]);
        assert_eq!(naming.from_top, Some(template.levels()));
        assert_eq!(naming.from_bottom, Some(template.levels()));
        assert_eq!(naming.chosen_by, ChosenBy::Engine);
        assert_eq!(naming.l5_confirmed_on, None);
        assert_eq!(
            chain.bodies.len(),
            if template == Template::ExtraLumbar {
                19
            } else {
                17
            }
        );
        assert_eq!(
            chain.bodies.last().unwrap().level,
            if template == Template::ExtraLumbar {
                Level::L6
            } else {
                Level::L5
            }
        );
    }
}

#[test]
fn a_close_variant_keeps_standard_and_is_flagged() {
    // Extra-lumbar's prior penalty relative to standard is 3.2470.
    // An energy advantage of 5 gives only 1.7530 adjusted units, below 3.
    let selection =
        level_naming::rank_templates(&energies(Template::ExtraLumbar, -5.0), 1.0).unwrap();
    assert_eq!(selection.selected, Template::Standard);
    assert_eq!(selection.scores[0].template, Template::ExtraLumbar);
    let chain = chain(Template::Standard, 0.0);
    let naming = level_naming::name_chain(&chain, &selection, &chain, &chain, anchors());
    assert!(naming.flags.contains(&NamingFlag::CloseAlternative));
    assert!(!naming.flags.contains(&NamingFlag::NonStandardCount));
}

#[test]
fn independent_end_counts_expose_an_off_by_one_without_reversing_the_lists() {
    let selected = chain(Template::Standard, 25.0);
    let top = chain(Template::Standard, 0.0);
    let selection =
        level_naming::rank_templates(&energies(Template::Standard, -10.0), 1.0).unwrap();
    let naming = level_naming::name_chain(&selected, &selection, &top, &selected, anchors());
    assert!(naming.flags.contains(&NamingFlag::EndsDisagree));
    let from_top = naming.from_top.as_ref().unwrap();
    let from_bottom = naming.from_bottom.as_ref().unwrap();
    assert_eq!(from_top.first(), Some(&Level::T1));
    assert_eq!(from_top.last(), Some(&Level::L6));
    assert_eq!(from_bottom.first(), Some(&Level::C7));
    assert_eq!(from_bottom.last(), Some(&Level::L5));
    for count in [from_top, from_bottom] {
        assert_eq!(count.len(), selected.bodies.len());
        assert!(count.windows(2).all(|pair| pair[0] < pair[1]));
    }
    let WatchFrame::LevelsNamed {
        stage,
        levels,
        naming: reported,
        ..
    } = level_naming::frame(&selected, &naming)
    else {
        panic!("naming frame");
    };
    assert_eq!(stage, Stage::Naming);
    assert_eq!(levels, Template::Standard.levels());
    assert_eq!(reported, naming);
}

#[test]
fn missing_or_competing_anchors_leave_the_names_unconfirmed() {
    let chain = chain(Template::Standard, 0.0);
    let selection =
        level_naming::rank_templates(&energies(Template::Standard, -10.0), 1.0).unwrap();
    let weak = AnchorEvidence {
        first_rib: 1.99,
        sacrum: 1.99,
        ..anchors()
    };
    let naming = level_naming::name_chain(&chain, &selection, &chain, &chain, weak);
    assert_eq!(naming.from_top, None);
    assert_eq!(naming.from_bottom, None);
    assert_eq!(
        naming.flags,
        [
            NamingFlag::TopAnchorWeak,
            NamingFlag::BottomAnchorWeak,
            NamingFlag::NamesUnknown
        ]
    );
    let competing = AnchorEvidence {
        competing_first_rib: Some(3.0),
        ..anchors()
    };
    let naming = level_naming::name_chain(&chain, &selection, &chain, &chain, competing);
    assert_eq!(naming.from_top, None);
    assert_eq!(naming.from_bottom, Some(Template::Standard.levels()));
    assert_eq!(naming.flags, [NamingFlag::TopAnchorWeak]);
    let absent = AnchorEvidence {
        ridge_present: false,
        ..anchors()
    };
    assert_eq!(
        level_naming::name_chain(&chain, &selection, &chain, &chain, absent).flags,
        [NamingFlag::TopAnchorWeak]
    );
}

#[test]
fn malformed_scores_and_an_infeasible_survey_do_not_make_probabilities() {
    let all_infinite: Vec<_> = Template::ALL
        .into_iter()
        .map(|t| (t, f64::INFINITY))
        .collect();
    assert_eq!(
        level_naming::rank_templates(&all_infinite, 1.0)
            .unwrap_err()
            .code,
        ErrorCode::PassNoSpineFound
    );
    assert!(level_naming::rank_templates(&[], 1.0).is_err());
    assert!(level_naming::rank_templates(&energies(Template::Standard, 0.0), 0.0).is_err());
    assert!(level_naming::rank_templates(&energies(Template::Standard, f64::NAN), 1.0).is_err());
}

fn strip() -> Strip {
    Strip {
        film: FilmKind::Front,
        rows: 600,
        cols: 3,
        row_step: 2.0,
        col_step: 1.0,
        scale_assumed: false,
        values: vec![0.5; 1800],
        valid: vec![1; 1800],
        layout: (0..600)
            .map(|row| StripRow {
                centre: Px::new(100.0 + 3.0 * row as f64, 4.0 * row as f64),
                across: Px::new(0.8, -0.6),
                along: Px::new(3.0, 4.0),
                curvature: 0.0,
                half_width: 10.0,
                z: -2.0 * row as f64,
            })
            .collect(),
        image_right: Some(ImageRight::PatientLeft),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    }
}

#[test]
fn confidence_uses_posterior_and_converts_length_spreads_to_film_pixels() {
    let chain = chain(Template::Standard, 25.0);
    let naming = named(&chain);
    let settled = per_vertebra::settle(chain, &strip(), &naming);
    let confidence = &settled.confidence[0];
    assert_eq!(confidence.score, 0.97);
    assert_eq!(confidence.band, ConfidenceBand::High);
    assert!(confidence.reasons.is_empty());
    assert!(!confidence.calibrated);
    assert_eq!(confidence.margin, Some(8.0));
    // One row is 2 length units and (3,4) film pixels: 2.5 px per unit.
    let spread = confidence.endplate_spread_px.unwrap();
    assert_eq!(spread.sup, 5.0);
    assert_eq!(spread.inf, 7.5);
    let WatchFrame::Confidence { levels, .. } = per_vertebra::frame(&settled) else {
        panic!("confidence frame");
    };
    assert_eq!(
        levels.iter().map(|level| level.level).collect::<Vec<_>>(),
        Template::Standard.levels()
    );
    assert!(levels.iter().all(|level| level.score == 0.97));
}

#[test]
fn local_warning_caps_only_its_body_and_never_raises_a_low_band() {
    let mut chain = chain(Template::Standard, 25.0);
    chain.bodies[2].certainty.posterior = 0.5;
    chain.bodies[3].inferred = true;
    chain.warnings = [Level::T1, Level::T2]
        .into_iter()
        .map(|level| PassWarning {
            code: PassWarningCode::MetalNearby,
            film: FilmKind::Front,
            level: Some(level),
            region: None,
        })
        .collect();
    let settled = per_vertebra::settle(chain.clone(), &strip(), &named(&chain));
    assert_eq!(settled.confidence[0].band, ConfidenceBand::High);
    assert!(
        !settled.confidence[0]
            .reasons
            .contains(&ConfidenceReason::MetalNearby)
    );
    assert_eq!(settled.confidence[1].band, ConfidenceBand::Medium);
    assert_eq!(settled.confidence[2].band, ConfidenceBand::Low);
    assert!(
        settled.confidence[2]
            .reasons
            .contains(&ConfidenceReason::WeakGap)
    );
    assert_eq!(settled.confidence[3].band, ConfidenceBand::Medium);
    assert!(
        settled.confidence[3]
            .reasons
            .contains(&ConfidenceReason::NoEvidence)
    );
    assert!(settled.confidence.iter().all(|c| c.band_fits_score()));
}

#[test]
fn a_warning_on_another_film_or_elsewhere_on_this_film_does_not_cap_a_body() {
    let chain = chain(Template::Standard, 25.0);
    let naming = named(&chain);
    let strip = strip();
    let warning = PassWarning {
        code: PassWarningCode::TiltInconsistent,
        film: FilmKind::Side,
        level: None,
        region: None,
    };
    assert_eq!(
        per_vertebra::score(
            &chain.bodies[0],
            &strip,
            &naming,
            std::slice::from_ref(&warning)
        )
        .band,
        ConfidenceBand::High
    );
    let elsewhere = PassWarning {
        film: FilmKind::Front,
        region: Some(PxRect {
            left: -200.0,
            right: -100.0,
            top: -200.0,
            bottom: -100.0,
        }),
        ..warning
    };
    assert_eq!(
        per_vertebra::score(&chain.bodies[0], &strip, &naming, &[elsewhere]).band,
        ConfidenceBand::High
    );
}

#[test]
fn naming_uncertainty_does_not_lower_position_probability() {
    let chain = chain(Template::ExtraLumbar, 25.0);
    let naming = named(&chain);
    let confidence = per_vertebra::score(&chain.bodies[0], &strip(), &naming, &[]);
    assert_eq!(confidence.score, 0.97);
    assert_eq!(confidence.band, ConfidenceBand::High);
    assert_eq!(confidence.reasons, [ConfidenceReason::LevelUncertain]);
}
