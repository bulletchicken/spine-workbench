//! The named recipes (straight, right-thoracic-40, double-major-55-45,
//! lumbar-left-30, severe-90, six-lumbar, eleven-ribs, upside-down,
//! mirrored-front, side-facing-left, heavy-shoulders, and random) and the
//! ranges every unnamed choice is drawn from.
//!
//! A [`MadeUpRecipe`] (a name, a seed and a tier) becomes a
//! [`SpineRecipe`], every choice about the patient, and four
//! [`FilmRecipe`]s, every choice about how each film was taken. A named
//! recipe fixes what its name says and draws everything else from the seed;
//! `random` draws everything, including the vertebra count, the curve
//! pattern and how each film is turned and faces.
//!
//! Named recipes present their films the usual way (front and bending films
//! with the patient's left on screen right, the side film facing right,
//! all upright, normal shoulders) unless their name says otherwise, so each
//! one tests the thing it is named for.
//!
//! The ranges are made up for made-up spines, except where a source is
//! named: curve sizes and positions follow the Lenke classification's
//! regions (Lenke et al., J Bone Joint Surg Am 2001, PMID 11507116);
//! kyphosis, pelvic incidence and sacral slope follow normal adolescent
//! means and spreads (Mac-Thiong et al., Spine 2004), and
//! the lordosis is whatever balances the patient (see
//! [`crate::patient`]); one spine in ten has an unusual vertebra count, inside
//! the range Carrino et al. report (Radiology 2011, PMID 21436097).
//!
//! **The order of draws is fixed.** Every value is drawn from its stream in
//! the order written here. Reordering, adding or removing a draw changes
//! every made-up spine after it, and every known answer built on them.

use crate::curves::{Curve, CurveGenerator, SagittalAngles};
use crate::seeded::Stream;
use shapes::film::FilmKind;
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::made_up::{CurveRegion, MadeUpRecipe, RecipeName, Tier};
use shapes::naming::Template;

/// Everything about the patient.
#[derive(Clone, Debug, PartialEq)]
pub struct SpineRecipe {
    pub template: Template,
    /// From the top of T1 to the top of S1, straightened, mm.
    pub length_mm: f64,
    pub generator: CurveGenerator,
    /// Top to bottom.
    pub curves: Vec<Curve>,
    pub sagittal: SagittalAngles,
    pub pelvic_incidence_deg: f64,
    /// The pelvis tipped to one side: positive lowers the right.
    pub pelvic_obliquity_deg: f64,
    /// Where C7 sits to the left of the top of the sacrum, mm.
    pub coronal_balance_mm: f64,
    /// Where C7 sits in front of the back corner of the top of the sacrum.
    pub sagittal_balance_mm: f64,
    pub vertebral_share: VertebralShare,
    /// Whether the jitter stream varies each vertebra's size.
    pub jitter_sizes: bool,
    pub short_twelfth_rib: bool,
    pub heavy_shoulders: bool,
    pub bend_left: Bend,
    pub bend_right: Bend,
}

/// How much of the bend between two vertebrae the vertebrae take as wedge
/// (the disc takes the rest), by plane and region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertebralShare {
    pub coronal_thoracic: f64,
    pub coronal_lumbar: f64,
    pub sagittal_thoracic: f64,
    pub sagittal_lumbar: f64,
}

/// The spine as bent for one bending film.
#[derive(Clone, Debug, PartialEq)]
pub struct Bend {
    /// Per curve: the share of its Cobb angle that bending toward its
    /// convexity takes away.
    pub flexibility: Vec<f64>,
    /// Per curve: the share bending away from its convexity adds.
    pub growth: Vec<f64>,
    /// The lean of the trunk toward the bend, degrees.
    pub trunk_lean_deg: f64,
    /// The pelvis tipping toward the bend, degrees.
    pub pelvic_tilt_deg: f64,
}

/// Everything about how one film was taken.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmRecipe {
    pub kind: FilmKind,
    pub tier: Tier,
    /// Square pixels, mm a side: 0.25 to 0.35, coarser than most
    /// detectors but finer than the 0.35 mm the auto pass works at, so a
    /// case paints in seconds even in the browser.
    pub mm_per_px: f64,
    pub image_right: ImageRight,
    pub quarter_turns: QuarterTurns,
    /// Room above the top of C2 and below the femoral heads, mm.
    pub margin_above_mm: f64,
    pub margin_below_mm: f64,
    /// The film's upright width as a share of its upright height.
    pub width_ratio: f64,
    /// How far the film's centre sits from the spine's, across, mm.
    pub across_offset_mm: f64,
    pub lead_marker: Option<MarkerPlacement>,
    pub ruler: Option<RulerPlacement>,
    pub ball: Option<BallPlacement>,
    /// The share of each edge outside the collimated field: left, top,
    /// right, bottom of the upright film.
    pub collimation: Option<[f64; 4]>,
    /// Bone dark instead of bright.
    pub inverted: bool,
    pub detector: DetectorRecipe,
}

/// Where the lead letter goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerPlacement {
    /// Height of the letter, mm.
    pub height_mm: f64,
    /// From the top of the upright film to the letter's top, as a share of
    /// the film's height.
    pub down: f64,
}

/// Where the ruler lies: along one side of the upright film.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerPlacement {
    pub length_mm: f64,
    /// On the screen-left edge of the upright film, or the right.
    pub on_left: bool,
    /// The top end's distance from the top of the film, share of height.
    pub down: f64,
}

/// Where the calibration ball lies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallPlacement {
    pub diameter_mm: f64,
    /// Across the upright film and down it, as shares of its size.
    pub across: f64,
    pub down: f64,
}

/// How the detector turns the beam into grey levels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DetectorRecipe {
    /// A clean drawing: log attenuation, windowed, 16-bit, no noise.
    Clean,
    /// A real detector.
    Physical(Physics),
}

/// The physics of a real detector and its processing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Physics {
    /// The mean count in the exposure-control region near T12–L1.
    pub target_counts: f64,
    /// Scattered over primary radiation, averaged over the body.
    pub scatter_to_primary: f64,
    pub scatter_sigma_mm: f64,
    /// The scintillator's blur, sd in mm.
    pub blur_sigma_mm: f64,
    /// Electronic noise, sd in counts as a share of `sqrt(target_counts)`.
    pub electronic_noise: f64,
    /// The heel effect: relative brightness change from top to bottom.
    pub heel: f64,
    pub unsharp_amount: f64,
    pub unsharp_sigma_mm: f64,
    pub gamma: f64,
    /// The percentiles of the field mapped to black and white.
    pub window: [f64; 2],
    /// 16 or 12; 8 with lossy compression.
    pub bits: u8,
    /// The quality of the lossy 8-bit variant, when it applies.
    pub lossy_quality: Option<f64>,
}

/// The full count of a spine with `template`: thoracic, then lumbar.
pub fn counts(template: Template) -> (usize, usize) {
    match template {
        Template::Standard => (12, 5),
        Template::ExtraThoracic => (13, 5),
        Template::ExtraLumbar => (12, 6),
        Template::MissingThoracic => (11, 5),
        Template::MissingLumbar => (12, 4),
    }
}

/// The spine recipe for `recipe` on one attempt, all from `seed`.
pub fn spine_recipe(name: RecipeName, seed: u64) -> SpineRecipe {
    let mut draw = Stream::new("recipe", seed);
    let template = match name {
        RecipeName::SixLumbar => Template::ExtraLumbar,
        RecipeName::ElevenRibs => Template::MissingThoracic,
        RecipeName::Random if draw.chance(0.10) => [
            Template::ExtraThoracic,
            Template::ExtraLumbar,
            Template::MissingThoracic,
            Template::MissingLumbar,
        ][draw.pick(&[1.0; 4])],
        _ => Template::Standard,
    };
    // Positions in the lumbar spine move with the thoracic count.
    let lumbar_shift = counts(template).0 as f64 - 12.0;

    let age = draw.between(10.0, 18.0);
    let female = draw.chance(0.8);
    let grown = if female {
        90.0 * ((age - 10.0) / 5.0).clamp(0.0, 1.0)
    } else {
        110.0 * ((age - 10.0) / 7.0).clamp(0.0, 1.0)
    };
    let length_mm = (340.0 + grown + draw.normal(0.0, 15.0)).clamp(300.0, 500.0);

    let generator = match name {
        RecipeName::Random if draw.chance(0.5) => CurveGenerator::Spline,
        _ => CurveGenerator::Bump,
    };
    let curves = match name {
        RecipeName::Straight => Vec::new(),
        RecipeName::RightThoracic40 => {
            let upper = draw.whole(4, 5);
            let lower = draw.whole(10, 12);
            vec![curve(
                &mut draw,
                CurveRegion::MainThoracic,
                upper,
                lower,
                40.0,
                true,
            )]
        }
        RecipeName::DoubleMajor55To45 => {
            let upper = draw.whole(3, 5);
            let shared = draw.whole(10, 11);
            let lower = draw.whole(14, 15);
            vec![
                curve(
                    &mut draw,
                    CurveRegion::MainThoracic,
                    upper,
                    shared,
                    55.0,
                    true,
                ),
                curve(
                    &mut draw,
                    CurveRegion::ThoracolumbarLumbar,
                    shared,
                    lower,
                    45.0,
                    false,
                ),
            ]
        }
        RecipeName::LumbarLeft30 => {
            let upper = draw.whole(9, 11);
            let lower = draw.whole(14, 15);
            vec![curve(
                &mut draw,
                CurveRegion::ThoracolumbarLumbar,
                upper,
                lower,
                30.0,
                false,
            )]
        }
        RecipeName::Severe90 => {
            let upper = draw.whole(2, 3);
            let lower = draw.whole(11, 12);
            vec![curve(
                &mut draw,
                CurveRegion::MainThoracic,
                upper,
                lower,
                90.0,
                true,
            )]
        }
        _ => random_curves(&mut draw),
    };
    let curves: Vec<Curve> = curves
        .into_iter()
        .map(|c| shift_lumbar(c, lumbar_shift))
        .collect();

    let kyphosis = draw.normal(43.0, 10.4).clamp(10.0, 70.0);
    let (pelvic_incidence_deg, sacral_slope) = pelvis(&mut draw);
    let sagittal = SagittalAngles {
        sacral_slope,
        // Solved when the spine is stacked, so the patient stands balanced.
        lordosis: 0.0,
        kyphosis,
        upper_thoracic: draw.between(0.0, 8.0),
        cervical_lordosis: draw.between(10.0, 30.0),
    };
    let pelvic_obliquity_deg = draw.normal(0.0, 1.5);
    let coronal_balance_mm = draw.normal(0.0, 10.0);
    let sagittal_balance_mm = draw.normal(-20.0, 25.0);
    // Severe curves wedge their vertebrae more: their discs could not take
    // the bend alone.
    let severe = curves.iter().any(|c| c.cobb_deg > 50.0);
    let (thoracic_share, lumbar_share) = if severe {
        ((0.55, 0.8), (0.4, 0.6))
    } else {
        ((0.3, 0.6), (0.1, 0.4))
    };
    let vertebral_share = VertebralShare {
        coronal_thoracic: draw.between(thoracic_share.0, thoracic_share.1),
        coronal_lumbar: draw.between(lumbar_share.0, lumbar_share.1),
        sagittal_thoracic: draw.between(0.3, 0.7),
        sagittal_lumbar: draw.between(0.1, 0.3),
    };
    let has_t12 = template != Template::MissingThoracic;
    let short_twelfth_rib = draw.chance(0.15) && has_t12;
    let heavy_shoulders = match name {
        RecipeName::HeavyShoulders => true,
        RecipeName::Random => draw.chance(0.1),
        _ => false,
    };
    let flexibility: Vec<f64> = curves
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let structural = index == main_curve(&curves) || c.cobb_deg >= 25.0;
            match (structural, c.region) {
                (true, CurveRegion::ThoracolumbarLumbar) => draw.between(0.5, 0.9),
                (true, _) => draw.between(0.2, 0.55),
                (false, _) => draw.between(0.6, 0.95),
            }
        })
        .collect();
    let bend = |draw: &mut Stream| Bend {
        flexibility: flexibility.clone(),
        growth: curves.iter().map(|_| draw.between(0.0, 0.3)).collect(),
        trunk_lean_deg: draw.between(10.0, 20.0),
        pelvic_tilt_deg: draw.between(0.0, 8.0),
    };
    let bend_left = bend(&mut draw);
    let bend_right = bend(&mut draw);

    SpineRecipe {
        template,
        length_mm,
        generator,
        curves,
        sagittal,
        pelvic_incidence_deg,
        pelvic_obliquity_deg,
        coronal_balance_mm,
        sagittal_balance_mm,
        vertebral_share,
        jitter_sizes: true,
        short_twelfth_rib,
        heavy_shoulders,
        bend_left,
        bend_right,
    }
}

/// The index of the largest curve.
pub fn main_curve(curves: &[Curve]) -> usize {
    curves
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.cobb_deg.total_cmp(&b.1.cobb_deg))
        .map_or(0, |(index, _)| index)
}

/// A curve between two end vertebrae, its apex drawn near the middle (the
/// bump generator puts it exactly there) and its twist per degree drawn.
fn curve(
    draw: &mut Stream,
    region: CurveRegion,
    upper: i32,
    lower: i32,
    cobb_deg: f64,
    convex_right: bool,
) -> Curve {
    let (upper_end, lower_end) = (f64::from(upper), f64::from(lower));
    let middle = 0.5 * (upper_end + lower_end);
    let reach = 0.15 * (lower_end - upper_end);
    Curve {
        region,
        upper_end,
        lower_end,
        apex: middle + draw.between(-reach, reach),
        cobb_deg,
        convex_right,
        twist_per_cobb: draw.between(0.2, 0.5),
    }
}

/// Moves a curve's lumbar positions with the thoracic count: a lumbar
/// curve stays on the same lumbar vertebrae when T12 is missing or T13
/// added.
fn shift_lumbar(mut curve: Curve, shift: f64) -> Curve {
    let moved = |t: f64| if t > 11.5 { t + shift } else { t };
    curve.upper_end = moved(curve.upper_end);
    curve.apex = moved(curve.apex);
    curve.lower_end = moved(curve.lower_end);
    curve
}

/// Pelvic incidence and sacral slope, normal adolescent values, drawn again
/// until the pelvic tilt `PI − SS` lies between −5° and 25°.
fn pelvis(draw: &mut Stream) -> (f64, f64) {
    let mut drawn = (48.4, 41.2);
    for _ in 0..64 {
        drawn = (draw.normal(48.4, 11.2), draw.normal(41.2, 8.5));
        let tilt = drawn.0 - drawn.1;
        if (-5.0..=25.0).contains(&tilt) && drawn.1 > 15.0 {
            return drawn;
        }
    }
    drawn
}

/// A curve pattern for the random recipe and the recipes named for
/// something else: none in ten, otherwise a Lenke type with its usual
/// share (type 1 most often), main curve size and position drawn.
fn random_curves(draw: &mut Stream) -> Vec<Curve> {
    let pattern = draw.pick(&[0.10, 0.43, 0.13, 0.13, 0.05, 0.11, 0.05]);
    let main_cobb = match draw.pick(&[0.55, 0.38, 0.07]) {
        0 => draw.between(10.0, 45.0),
        1 => draw.between(45.0, 90.0),
        _ => draw.between(90.0, 110.0),
    };
    let thoracic_right = draw.chance(0.9);
    let ratio_minor = draw.between(0.3, 0.6);
    let ratio_major = draw.between(0.7, 1.0);
    let pt_upper = draw.whole(0, 1);
    let mt_upper = draw.whole(3, 5);
    let mt_lower = draw.whole(10, 12);
    let ll_lower = draw.whole(14, 15);
    let add = |draw: &mut Stream,
               curves: &mut Vec<Curve>,
               region: CurveRegion,
               upper: i32,
               lower: i32,
               cobb: f64,
               right: bool| {
        curves.push(curve(draw, region, upper, lower, cobb, right));
    };
    let mut curves = Vec::new();
    let pt = CurveRegion::ProximalThoracic;
    let mt = CurveRegion::MainThoracic;
    let ll = CurveRegion::ThoracolumbarLumbar;
    match pattern {
        // Straight enough: a small curve that never reaches 10°.
        0 => {
            let cobb = draw.between(0.0, 10.0);
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                cobb,
                thoracic_right,
            );
        }
        // Type 1: a main thoracic curve, small compensatory curves.
        1 => {
            add(
                draw,
                &mut curves,
                pt,
                pt_upper,
                mt_upper,
                main_cobb * ratio_minor * 0.6,
                !thoracic_right,
            );
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb,
                thoracic_right,
            );
            add(
                draw,
                &mut curves,
                ll,
                mt_lower,
                ll_lower,
                main_cobb * ratio_minor,
                !thoracic_right,
            );
        }
        // Type 2: double thoracic.
        2 => {
            add(
                draw,
                &mut curves,
                pt,
                pt_upper,
                mt_upper,
                main_cobb * ratio_major * 0.8,
                !thoracic_right,
            );
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb,
                thoracic_right,
            );
        }
        // Type 3: double major.
        3 => {
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb,
                thoracic_right,
            );
            add(
                draw,
                &mut curves,
                ll,
                mt_lower,
                ll_lower,
                main_cobb * ratio_major,
                !thoracic_right,
            );
        }
        // Type 4: triple major.
        4 => {
            add(
                draw,
                &mut curves,
                pt,
                pt_upper,
                mt_upper,
                main_cobb * ratio_major * 0.8,
                !thoracic_right,
            );
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb,
                thoracic_right,
            );
            add(
                draw,
                &mut curves,
                ll,
                mt_lower,
                ll_lower,
                main_cobb * ratio_major,
                !thoracic_right,
            );
        }
        // Type 5: a main thoracolumbar or lumbar curve.
        5 => {
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb * ratio_minor,
                !thoracic_right,
            );
            add(
                draw,
                &mut curves,
                ll,
                mt_lower,
                ll_lower,
                main_cobb,
                thoracic_right,
            );
        }
        // Type 6: lumbar main, thoracic structural.
        _ => {
            add(
                draw,
                &mut curves,
                mt,
                mt_upper,
                mt_lower,
                main_cobb * ratio_major,
                thoracic_right,
            );
            add(
                draw,
                &mut curves,
                ll,
                mt_lower,
                ll_lower,
                main_cobb,
                !thoracic_right,
            );
        }
    }
    curves
}

/// The four films' recipes for a spine, in the order front, side, bend
/// left, bend right.
pub fn film_recipes(recipe: &MadeUpRecipe, seed: u64) -> Vec<FilmRecipe> {
    FilmKind::ALL
        .iter()
        .map(|&kind| film_recipe(recipe, kind, seed))
        .collect()
}

fn film_recipe(recipe: &MadeUpRecipe, kind: FilmKind, seed: u64) -> FilmRecipe {
    let mut draw = Stream::new(&format!("film/{}", kind.name()), seed);
    let tier = recipe.tier;
    let front_view = kind != FilmKind::Side;
    let usual = if front_view {
        ImageRight::PatientLeft
    } else {
        ImageRight::Anterior
    };
    let facing_draw = draw.uniform();
    let turn_draw = draw.uniform();
    let image_right = match recipe.name {
        RecipeName::MirroredFront if kind == FilmKind::Front => ImageRight::PatientRight,
        RecipeName::SideFacingLeft if kind == FilmKind::Side => ImageRight::Posterior,
        RecipeName::Random => {
            let flip = if front_view { 0.1 } else { 0.5 };
            if facing_draw < flip {
                usual.mirrored()
            } else {
                usual
            }
        }
        _ => usual,
    };
    let turns = match recipe.name {
        RecipeName::UpsideDown => 2,
        RecipeName::Random if turn_draw < 0.03 => 2,
        RecipeName::Random if turn_draw < 0.04 => 1,
        RecipeName::Random if turn_draw < 0.05 => 3,
        _ => 0,
    };
    let quarter_turns = QuarterTurns::new(turns).unwrap_or(QuarterTurns::NONE);

    let mm_per_px = draw.between(0.25, 0.35);
    let margin_above_mm = draw.between(30.0, 70.0);
    let margin_below_mm = draw.between(50.0, 90.0);
    let width_ratio = draw.between(0.38, 0.50);
    let across_offset_mm = draw.between(-20.0, 20.0);

    let marker_height = draw.between(15.0, 25.0);
    let marker_down = draw.between(0.03, 0.12);
    let lead_marker = (tier >= Tier::T2).then_some(MarkerPlacement {
        height_mm: marker_height,
        down: marker_down,
    });
    let gadget_chance = if tier >= Tier::T4 { 0.6 } else { 0.5 };
    let ruler_drawn = draw.uniform();
    let ruler = RulerPlacement {
        length_mm: draw.between(200.0, 400.0),
        on_left: draw.chance(0.5),
        down: draw.between(0.15, 0.35),
    };
    let ruler = (tier >= Tier::T3 && ruler_drawn < gadget_chance).then_some(ruler);
    let ball_drawn = draw.uniform();
    let ball = BallPlacement {
        diameter_mm: [25.0, 30.0][draw.pick(&[1.0, 1.0])],
        across: draw.between(0.1, 0.25),
        down: draw.between(0.72, 0.85),
    };
    let ball = (tier >= Tier::T3 && ball_drawn < gadget_chance).then_some(ball);
    let collimated = draw.chance(0.6);
    let insets = [
        draw.between(0.0, 0.08),
        draw.between(0.0, 0.04),
        draw.between(0.0, 0.08),
        draw.between(0.0, 0.04),
    ];
    let collimation = (tier >= Tier::T4 && collimated).then_some(insets);
    let inverted = draw.chance(0.15) && tier >= Tier::T4;
    let physics = physics(&mut draw, tier);
    let detector = if tier >= Tier::T3 {
        DetectorRecipe::Physical(physics)
    } else {
        DetectorRecipe::Clean
    };

    FilmRecipe {
        kind,
        tier,
        mm_per_px,
        image_right,
        quarter_turns,
        margin_above_mm,
        margin_below_mm,
        width_ratio,
        across_offset_mm,
        lead_marker,
        ruler,
        ball,
        collimation,
        inverted,
        detector,
    }
}

/// Detector physics. At t3 a fair exposure; at t4 a low dose and heavy
/// scatter, the films that are hardest to read.
fn physics(draw: &mut Stream, tier: Tier) -> Physics {
    let low_contrast = tier >= Tier::T4;
    let target_counts = if low_contrast {
        libm::exp(draw.between(libm::log(80.0), libm::log(400.0)))
    } else {
        libm::exp(draw.between(libm::log(400.0), libm::log(3000.0)))
    };
    let scatter_to_primary = if low_contrast {
        draw.between(0.5, 1.0)
    } else {
        draw.between(0.05, 0.4)
    };
    let scatter_sigma_mm = draw.between(30.0, 80.0);
    let blur_sigma_mm = draw.between(0.15, 0.40);
    let electronic_noise = draw.between(0.0, 0.5);
    let heel = draw.between(-0.15, 0.15);
    let unsharp_amount = draw.between(0.0, 1.5);
    let unsharp_sigma_mm = draw.between(2.0, 10.0);
    let gamma = draw.between(0.7, 1.4);
    let window = [draw.between(0.005, 0.03), draw.between(0.97, 0.995)];
    let twelve_bit = draw.chance(0.4);
    let lossy = draw.chance(0.25) && low_contrast;
    let quality = draw.between(70.0, 95.0);
    Physics {
        target_counts,
        scatter_to_primary,
        scatter_sigma_mm,
        blur_sigma_mm,
        electronic_noise,
        heel,
        unsharp_amount,
        unsharp_sigma_mm,
        gamma,
        window,
        bits: if lossy {
            8
        } else if twelve_bit {
            12
        } else {
            16
        },
        lossy_quality: lossy.then_some(quality),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn made_up(name: RecipeName, tier: Tier) -> MadeUpRecipe {
        MadeUpRecipe {
            name,
            seed: 5,
            tier,
        }
    }

    #[test]
    fn named_curves_ask_for_their_named_angles() {
        let right = spine_recipe(RecipeName::RightThoracic40, 1);
        assert_eq!(right.curves.len(), 1);
        assert_eq!(right.curves[0].cobb_deg, 40.0);
        assert!(right.curves[0].convex_right);
        let double = spine_recipe(RecipeName::DoubleMajor55To45, 1);
        assert_eq!(double.curves.len(), 2);
        assert_eq!(
            (double.curves[0].cobb_deg, double.curves[1].cobb_deg),
            (55.0, 45.0)
        );
        assert_eq!(double.curves[0].lower_end, double.curves[1].upper_end);
        assert!(double.curves[0].convex_right && !double.curves[1].convex_right);
        let lumbar = spine_recipe(RecipeName::LumbarLeft30, 1);
        assert!(!lumbar.curves[0].convex_right);
        assert_eq!(lumbar.curves[0].region, CurveRegion::ThoracolumbarLumbar);
        assert_eq!(
            spine_recipe(RecipeName::Severe90, 1).curves[0].cobb_deg,
            90.0
        );
        assert!(spine_recipe(RecipeName::Straight, 1).curves.is_empty());
    }

    #[test]
    fn named_counts_set_the_template() {
        assert_eq!(
            spine_recipe(RecipeName::SixLumbar, 2).template,
            Template::ExtraLumbar
        );
        let eleven = spine_recipe(RecipeName::ElevenRibs, 2);
        assert_eq!(eleven.template, Template::MissingThoracic);
        assert!(!eleven.short_twelfth_rib);
        assert_eq!(
            spine_recipe(RecipeName::Straight, 2).template,
            Template::Standard
        );
    }

    #[test]
    fn random_spines_vary_and_named_ones_use_the_bump() {
        let templates: Vec<Template> = (0..200)
            .map(|seed| spine_recipe(RecipeName::Random, seed).template)
            .collect();
        let unusual = templates
            .iter()
            .filter(|t| **t != Template::Standard)
            .count();
        // One in ten expected: 20 of 200, give or take.
        assert!((8..=35).contains(&unusual), "{unusual}");
        let splines = (0..200)
            .filter(|&seed| {
                spine_recipe(RecipeName::Random, seed).generator == CurveGenerator::Spline
            })
            .count();
        assert!((70..=130).contains(&splines), "{splines}");
        assert!((0..50).all(
            |seed| spine_recipe(RecipeName::HeavyShoulders, seed).generator == CurveGenerator::Bump
        ));
    }

    #[test]
    fn the_same_seed_gives_the_same_recipe() {
        assert_eq!(
            spine_recipe(RecipeName::Random, 77),
            spine_recipe(RecipeName::Random, 77)
        );
        assert_ne!(
            spine_recipe(RecipeName::Random, 77),
            spine_recipe(RecipeName::Random, 78)
        );
    }

    #[test]
    fn lumbar_curves_stay_on_their_vertebrae_when_the_count_changes() {
        let curve = Curve {
            region: CurveRegion::ThoracolumbarLumbar,
            upper_end: 11.0,
            lower_end: 15.0,
            apex: 13.0,
            cobb_deg: 30.0,
            convex_right: false,
            twist_per_cobb: 0.3,
        };
        let eleven = shift_lumbar(curve, -1.0);
        assert_eq!(
            (eleven.upper_end, eleven.apex, eleven.lower_end),
            (11.0, 12.0, 14.0)
        );
    }

    #[test]
    fn named_presentations_turn_and_mirror_what_their_names_say() {
        let films = film_recipes(&made_up(RecipeName::UpsideDown, Tier::T2), 3);
        assert!(films.iter().all(|f| f.quarter_turns.get() == 2));
        let films = film_recipes(&made_up(RecipeName::MirroredFront, Tier::T2), 3);
        assert_eq!(films[0].image_right, ImageRight::PatientRight);
        assert_eq!(films[2].image_right, ImageRight::PatientLeft);
        let films = film_recipes(&made_up(RecipeName::SideFacingLeft, Tier::T2), 3);
        assert_eq!(films[1].image_right, ImageRight::Posterior);
        let films = film_recipes(&made_up(RecipeName::Straight, Tier::T2), 3);
        assert_eq!(films[0].image_right, ImageRight::PatientLeft);
        assert_eq!(films[1].image_right, ImageRight::Anterior);
        assert!(films.iter().all(|f| f.quarter_turns.get() == 0));
    }

    #[test]
    fn tiers_add_their_contents_in_order() {
        let clean = film_recipes(&made_up(RecipeName::Straight, Tier::T0), 3);
        assert!(clean.iter().all(|f| f.lead_marker.is_none()
            && f.ruler.is_none()
            && f.ball.is_none()
            && f.detector == DetectorRecipe::Clean));
        let t2 = film_recipes(&made_up(RecipeName::Straight, Tier::T2), 3);
        assert!(
            t2.iter()
                .all(|f| f.lead_marker.is_some() && f.detector == DetectorRecipe::Clean)
        );
        let t4: Vec<FilmRecipe> = (0..40)
            .flat_map(|seed| film_recipes(&made_up(RecipeName::Straight, Tier::T4), seed))
            .collect();
        assert!(t4.iter().all(
            |f| matches!(f.detector, DetectorRecipe::Physical(p) if p.target_counts <= 400.0)
        ));
        assert!(t4.iter().any(|f| f.collimation.is_some()));
        assert!(t4.iter().any(|f| f.inverted));
        assert!(
            t4.iter()
                .any(|f| matches!(f.detector, DetectorRecipe::Physical(p) if p.bits == 8))
        );
        assert!(t4.iter().any(|f| f.ruler.is_some()) && t4.iter().any(|f| f.ball.is_some()));
    }
}
