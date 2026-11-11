//! Synthetic strips from stacked vertebrae projected through the forward
//! model. The sizes below are deliberately independent of the search's
//! anatomical tables; no film-stage implementation is involved.

use auto_pass::chain_search::{
    dynamic_program::{self, BodyState},
    gap_score::LatticeEvidence,
    posterior,
    survey::{self, Anchors},
    two_views::{ViewGeometry, solve_pair},
};
use auto_pass::handoffs::{Barcode, Polarity, SettledChain, Strip, StripEvidence, StripRow};
use forward_model::cameras::{Camera, parallel::ParallelCamera};
use forward_model::project::{front_marks, side_marks};
use made_up_spines::body_sizes::VertebraSize;
use made_up_spines::curves::{CoronalProfile, CurveGenerator, SagittalAngles, SagittalProfile};
use made_up_spines::recipes::VertebralShare;
use made_up_spines::seeded::Stream;
use made_up_spines::stack_from_l5::{StackPlan, stack};
use nalgebra::Vector2;
use shapes::auto_pass::{Acquisition, ChainOptions, Population};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::levels::{Level, Region};
use shapes::naming::{NamingFlag, Template};
use shapes::pixels::Px;
use shapes::watch_frames::WatchFrame;

struct Fixture {
    strip: Strip,
    barcode: Barcode,
    levels: Vec<Level>,
    plates: Vec<[f64; 2]>,
}

fn fixture(template: Template, film: FilmKind, noise: f64, seed: u64) -> Fixture {
    let levels = template.levels();
    let thoracic = levels
        .iter()
        .filter(|l| l.region() == Region::Thoracic)
        .count();
    let lumbar = levels
        .iter()
        .filter(|l| l.region() == Region::Lumbar)
        .count();
    let sizes: Vec<_> = levels
        .iter()
        .map(|level| {
            // Simple integer ramps and independent disc fractions. They are
            // known geometry, not copied from the solver's reference table.
            let (height, disc) = match level.region() {
                Region::Cervical => (16.0, 4.0),
                Region::Thoracic => (18.0 + ((level.order() - Level::T1.order()) / 2) as f64, 5.0),
                Region::Lumbar => (26.0 + ((level.order() - Level::L1.order()) / 2) as f64, 8.0),
            };
            VertebraSize {
                height,
                disc_below: disc,
                top_half_width: 15.0,
                bottom_half_width: 15.0,
                top_half_depth: 15.0,
                bottom_half_depth: 15.0,
                pedicle_width: 5.0,
                pedicle_height: 8.0,
            }
        })
        .collect();
    let coronal = CoronalProfile::new(CurveGenerator::Bump, &[], levels.len() as f64)
        .expect("a straight coronal profile");
    let sagittal = SagittalProfile::new(
        &SagittalAngles {
            sacral_slope: 0.0,
            lordosis: 0.0,
            kyphosis: 0.0,
            upper_thoracic: 0.0,
            cervical_lordosis: 0.0,
        },
        thoracic,
        lumbar,
    );
    let spine = stack(&StackPlan {
        levels: &levels,
        sizes: &sizes,
        coronal: &coronal,
        sagittal: &sagittal,
        share: VertebralShare {
            coronal_thoracic: 0.0,
            coronal_lumbar: 0.0,
            sagittal_thoracic: 0.0,
            sagittal_lumbar: 0.0,
        },
        trunk_lean: 0.0,
        pelvic_incidence_deg: 50.0,
        pelvic_obliquity_deg: 0.0,
        length_mm: 450.0,
        coronal_balance: None,
    })
    .expect("the made-up spine stacks");
    let upper = spine.vertebrae[0].centre.z + spine.vertebrae[0].height * 0.5;
    let origin_y = upper + 19.5;
    let rows = (origin_y - spine.pelvis.sacrum_top.centre.z + 35.5) as usize;
    let cols = 81;
    let facing = if film == FilmKind::Side {
        ImageRight::Anterior
    } else {
        ImageRight::PatientLeft
    };
    let camera = ParallelCamera::for_film(
        &FilmGeometry {
            mm_per_px: Some(1.0),
            image_right: Some(facing),
            ..FilmGeometry::UNKNOWN
        },
        [cols as u32, rows as u32],
        Vector2::new(40.0, origin_y),
    )
    .unwrap();
    let plates: Vec<_> = spine
        .vertebrae
        .iter()
        .map(|body| {
            if film == FilmKind::Side {
                let m = side_marks(body, &camera).unwrap();
                [
                    0.5 * (m.sup_a.at.y() + m.sup_p.at.y()),
                    0.5 * (m.inf_a.at.y() + m.inf_p.at.y()),
                ]
            } else {
                let m = front_marks(body, &camera).unwrap();
                [
                    0.5 * (m.sup_l.at.y() + m.sup_r.at.y()),
                    0.5 * (m.inf_l.at.y() + m.inf_r.at.y()),
                ]
            }
        })
        .collect();
    let sacrum = camera.project(&spine.pelvis.sacrum_top.centre).y;
    let mut values = vec![0.2; rows * cols];
    let mut ridge = vec![0.0; rows * cols];
    let mut random = Stream::new("chain-strip-noise", seed);
    for row in 0..rows {
        let bone = plates
            .iter()
            .any(|p| row as f64 > p[0] && (row as f64) < p[1]);
        let coherent_noise = random.normal(0.0, noise * 0.5);
        for col in 0..cols {
            let across = (col as f64 - 40.0).abs();
            let inside = across <= 15.0;
            let on_sacrum = row as f64 > sacrum && across <= 25.0;
            let intensity = if (bone && inside) || on_sacrum {
                0.9
            } else {
                0.2
            };
            values[row * cols + col] =
                (intensity + coherent_noise + random.normal(0.0, noise)).clamp(0.01, 1.49) as f32;
            if row as f64 > plates[1][0]
                && (row as f64) < plates[thoracic][1]
                && (16.0..=37.0).contains(&across)
            {
                ridge[row * cols + col] = 1.0;
            }
        }
    }
    let strip = Strip {
        film,
        rows: rows as u32,
        cols: cols as u32,
        row_step: 1.0,
        col_step: 1.0,
        scale_assumed: false,
        values,
        valid: vec![1; rows * cols],
        layout: (0..rows)
            .map(|row| StripRow {
                centre: Px::new(40.0, row as f64),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                half_width: 15.0,
                z: -(row as f64),
            })
            .collect(),
        image_right: Some(facing),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence {
            ridge: Some(ridge),
            ridge_angle: Some(vec![std::f32::consts::FRAC_PI_4; rows * cols]),
            ..StripEvidence::default()
        },
    };
    let barcode = barcode(&strip);
    Fixture {
        strip,
        barcode,
        levels,
        plates,
    }
}

fn barcode(strip: &Strip) -> Barcode {
    let cols = strip.cols as usize;
    let values: Vec<f32> = strip
        .values
        .chunks_exact(cols)
        .map(|row| row[cols / 2 - 4..=cols / 2 + 4].iter().sum::<f32>() / 9.0)
        .collect();
    let high_pass = values.iter().map(|v| 2.0 * (*v - 0.55)).collect();
    Barcode {
        film: strip.film,
        valid: vec![1; strip.rows as usize],
        values,
        high_pass,
    }
}

fn options(template: Option<Template>) -> ChainOptions {
    ChainOptions {
        acquisition: Acquisition::Simultaneous,
        population: Population::Adolescent,
        template,
    }
}

#[test]
fn zero_noise_recovers_every_projected_endplate_and_watch_order() {
    let f = fixture(Template::Standard, FilmKind::Front, 0.0, 7);
    let result = survey::search(&f.strip, &f.barcode, &options(None)).unwrap();
    assert_eq!(result.naming.template, Template::Standard);
    for (body, plates) in result.settled.boundaries.bodies.iter().zip(&f.plates) {
        assert_eq!([body.sup.row, body.inf.row], *plates, "{:?}", body.level);
        assert!(!body.inferred);
    }
    assert_eq!(result.settled.boundaries.bodies.len(), f.plates.len());
    let mut sweep_levels = Vec::new();
    let mut scored = 0;
    for frame in &result.frames {
        match frame {
            WatchFrame::Sweep { level, heat, .. } => {
                sweep_levels.push(*level);
                assert_eq!(heat.len(), 256);
            }
            WatchFrame::TemplateScored { .. } => scored += 1,
            WatchFrame::ChainLocked { bodies, .. } => assert_eq!(
                bodies.iter().map(|b| b.level).collect::<Vec<_>>(),
                f.levels.iter().rev().copied().collect::<Vec<_>>()
            ),
            WatchFrame::LevelsNamed { levels, .. } => assert_eq!(*levels, f.levels),
            _ => {}
        }
    }
    assert_eq!(sweep_levels, f.levels);
    assert_eq!(scored, 5);
    assert_eq!(
        SettledChain::from_bytes(&result.settled.to_bytes()).unwrap(),
        result.settled
    );
    assert!(
        result
            .settled
            .confidence
            .iter()
            .all(|c| c.band_fits_score() && !c.calibrated)
    );
    for bodies in result.settled.boundaries.bodies.windows(2) {
        for across in [-9.0, 9.0] {
            let upper = bodies[0].inf.row + across * libm::tan(bodies[0].inf.tilt);
            let lower = bodies[1].sup.row + across * libm::tan(bodies[1].sup.tilt);
            assert!(lower - upper >= 0.5 - 1.0e-12);
        }
    }
}

#[test]
fn paired_surveys_keep_the_frozen_outputs_and_joint_frame_order() {
    use auto_pass::chain_search::paired::{self, SurveyView};
    let front = fixture(Template::Standard, FilmKind::Front, 0.0, 31);
    let side = fixture(Template::Standard, FilmKind::Side, 0.0, 31);
    let f = survey::search(&front.strip, &front.barcode, &options(None)).unwrap();
    let s = survey::search(&side.strip, &side.barcode, &options(None)).unwrap();
    let geometry = |search: &survey::FilmSearch| ViewGeometry {
        z: search.evidence.z.clone(),
        observed: search.evidence.observed.clone(),
        scale_assumed: false,
    };
    let front_geometry = geometry(&f);
    let side_geometry = geometry(&s);
    let expected_prefix: Vec<_> = [&f, &s]
        .into_iter()
        .flat_map(|search| {
            search
                .frames
                .iter()
                .take_while(|frame| !matches!(frame, WatchFrame::ChainLocked { .. }))
                .cloned()
        })
        .collect();
    let paired = paired::settle(
        SurveyView {
            strip: &front.strip,
            survey: f,
            geometry: front_geometry,
        },
        SurveyView {
            strip: &side.strip,
            survey: s,
            geometry: side_geometry,
        },
        &options(None),
    )
    .unwrap();
    assert_eq!(paired.front.naming.template, Template::Standard);
    assert_eq!(paired.side.naming.template, Template::Standard);
    assert_eq!(paired.independent_front.boundaries.bodies.len(), 18);
    assert_eq!(paired.independent_side.boundaries.bodies.len(), 18);
    // Identical z projections have exactly zero coupling, hence a zero
    // certificate within a 1e-9 allowance for f64 sum reassociation.
    assert!(paired.check.certified_gap < 1e-9);
    assert!(paired.check.exact && !paired.check.disagree);
    let first_lock = paired
        .frames
        .iter()
        .position(|frame| matches!(frame, WatchFrame::ChainLocked { .. }))
        .unwrap();
    let last_joint = paired
        .frames
        .iter()
        .rposition(|frame| matches!(frame, WatchFrame::JointProgress { .. }))
        .unwrap();
    assert!(last_joint < first_lock);
    assert_eq!(
        &paired.frames[..expected_prefix.len()],
        expected_prefix.as_slice()
    );
    for result in [&paired.front, &paired.side] {
        assert_eq!(
            SettledChain::from_bytes(&result.settled.to_bytes()).unwrap(),
            result.settled
        );
        assert_eq!(result.naming.chosen_by, shapes::naming::ChosenBy::Engine);
        let named = result
            .frames
            .iter()
            .find_map(|frame| match frame {
                WatchFrame::LevelsNamed { naming, levels, .. } => Some((naming, levels)),
                _ => None,
            })
            .unwrap();
        assert_eq!(named.0, &result.naming);
        assert_eq!(named.1, &front.levels);
        assert!(
            result
                .settled
                .confidence
                .iter()
                .all(|confidence| confidence.band_fits_score())
        );
    }
}

#[test]
fn nonstandard_made_up_counts_are_never_silently_standard() {
    for template in [Template::ExtraLumbar, Template::MissingThoracic] {
        let f = fixture(template, FilmKind::Front, 0.0, 9);
        let result = survey::search(&f.strip, &f.barcode, &options(None)).unwrap();
        assert!(
            result.naming.flags.iter().any(|flag| matches!(
                flag,
                NamingFlag::NonStandardCount
                    | NamingFlag::CloseAlternative
                    | NamingFlag::EndsDisagree
            )),
            "{template:?}: {:?}",
            result.naming
        );
        if template == Template::ExtraLumbar {
            assert!(result.naming.flags.contains(&NamingFlag::NonStandardCount));
            assert_eq!(result.settled.boundaries.bodies.len(), f.plates.len());
        }
        assert!(
            result
                .naming
                .alternatives
                .iter()
                .any(|choice| choice.template == template)
        );
    }
}

#[test]
fn noise_degrades_precision_without_losing_the_body_order() {
    let measured = |fixture: &Fixture| {
        let evidence =
            LatticeEvidence::build(&fixture.strip, &fixture.barcode, Polarity::BoneBright).unwrap();
        let problem = survey::problem(
            &evidence,
            Template::Standard,
            Population::Adolescent,
            Anchors::Both,
        );
        let solution = dynamic_program::solve(&problem).unwrap();
        let uncertainty = posterior::infer(&problem, &solution.states, evidence.step, 1.0).unwrap();
        (evidence, solution, uncertainty)
    };
    let clean = fixture(Template::Standard, FilmKind::Front, 0.0, 12);
    let (_, _, clean_posterior) = measured(&clean);
    let clean_spread: f64 = clean_posterior
        .certainty
        .iter()
        .map(|c| c.sup_spread + c.inf_spread)
        .sum();
    let mut noisy_spread = 0.0;
    for seed in [12, 29] {
        let noisy = fixture(Template::Standard, FilmKind::Front, 0.25, seed);
        let (evidence, result, posterior) = measured(&noisy);
        for ((body, truth), certainty) in result
            .states
            .iter()
            .zip(&noisy.plates)
            .zip(&posterior.certainty)
        {
            let sup = evidence.native_row(body.top as f64);
            let inf = evidence.native_row((body.top + body.height) as f64);
            // Noise SD 0.25 against contrast 0.7, including row-correlated
            // noise, should move a plate by at most two 1-mm cells.
            assert!(
                (sup - truth[0]).abs() <= 2.0 && (inf - truth[1]).abs() <= 2.0,
                "seed {seed}, {:?}: {:?} vs {truth:?}",
                body,
                [sup, inf]
            );
            noisy_spread += certainty.sup_spread + certainty.inf_spread;
        }
    }
    assert!(noisy_spread / 2.0 > clean_spread);
}

fn paired_trap(corrupted: FilmKind) -> (usize, usize, bool) {
    let mut front = fixture(Template::Standard, FilmKind::Front, 0.0, 17);
    let mut side = fixture(Template::Standard, FilmKind::Side, 0.0, 17);
    let damaged = if corrupted == FilmKind::Front {
        &mut front
    } else {
        &mut side
    };
    let cols = damaged.strip.cols as usize;
    let first = damaged.plates[0];
    let sacrum = damaged.plates.last().unwrap()[1] + 8.0;
    // Hide C7 as shoulders can, and make a false disc through the sacrum.
    // There are now eighteen visible body-like bands starting at T1.
    // Treating those as C7..L5 assigns the wrong name to every band.
    for row in 0..damaged.strip.rows as usize {
        let hidden = row as f64 > first[0] && (row as f64) < first[1];
        let false_disc = row as f64 > sacrum + 27.0;
        if hidden || false_disc {
            for col in 25..=55 {
                damaged.strip.values[row * cols + col] = 0.2;
            }
        }
    }
    damaged.barcode = barcode(&damaged.strip);
    let front_evidence =
        LatticeEvidence::build(&front.strip, &front.barcode, Polarity::BoneBright).unwrap();
    let side_evidence =
        LatticeEvidence::build(&side.strip, &side.barcode, Polarity::BoneBright).unwrap();
    let make_problem =
        |e| survey::problem(e, Template::Standard, Population::Adolescent, Anchors::Both);
    let front_problem = make_problem(&front_evidence);
    let side_problem = make_problem(&side_evidence);
    let geometry = |e: &LatticeEvidence| ViewGeometry {
        z: e.z.clone(),
        observed: e.observed.clone(),
        scale_assumed: false,
    };
    let joint = solve_pair(
        &front_problem,
        &side_problem,
        &geometry(&front_evidence),
        &geometry(&side_evidence),
        &front.levels,
        Acquisition::Simultaneous,
    )
    .unwrap();
    let errors = |states: &[BodyState], e: &LatticeEvidence, f: &Fixture| {
        states
            .iter()
            .zip(&f.plates)
            .filter(|(state, truth)| {
                let found = e.native_row(state.top as f64 + state.height as f64 * 0.5);
                let centre = 0.5 * (truth[0] + truth[1]);
                (found - centre).abs() > 0.5 * (truth[1] - truth[0])
            })
            .count()
    };
    let independent_errors = errors(&joint.independent_front.states, &front_evidence, &front)
        + errors(&joint.independent_side.states, &side_evidence, &side);
    let joint_errors = errors(&joint.front.states, &front_evidence, &front)
        + errors(&joint.side.states, &side_evidence, &side);
    assert!(
        independent_errors > 0,
        "the single-film trap must cause an indexing error"
    );
    assert!(joint.upper >= joint.lower - 1.0e-9);
    assert_eq!(
        joint.independent_front.states,
        dynamic_program::solve(&front_problem).unwrap().states
    );
    (independent_errors, joint_errors, joint.check.disagree)
}

#[test]
fn the_second_projection_corrects_an_off_by_one_naming_trap() {
    let (independent, paired, _) = paired_trap(FilmKind::Side);
    assert!(
        paired < independent,
        "independent {independent}, paired {paired}"
    );
}

#[test]
fn both_block_orders_correct_a_front_naming_trap() {
    let (independent, paired, flagged) = paired_trap(FilmKind::Front);
    assert!(
        paired <= independent || flagged,
        "independent {independent}, paired {paired}"
    );
    assert!(
        paired < independent,
        "independent {independent}, paired {paired}"
    );
}
