//! One front film, or a front and side pair, fitted with the typical spine
//! for what the films cannot show, against the made-up answer keys.

#[expect(
    dead_code,
    reason = "both test files share the answer keys; this one uses some"
)]
mod answer_keys;

use answer_keys::{angles, input, median};
use core::f64::consts::PI;
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::project::front_marks;
use forward_model::vertebra_params::VertebraParams;
use joint_fit::{FitError, fit_from_films, fit_spine};
use made_up_spines::seeded::Stream;
use nalgebra::{Vector2, Vector3};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::levels::Level;
use shapes::made_up::{AnswerKey, RecipeName, VertebraPose};
use shapes::marks::{FrontMarks, MarkSource, PedicleMark};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FilmToFit, FitFromFilms, FitLengths, FittedVertebra, MarkSpread, SpineFit, TwistReading,
    TypicalSource,
};
use shapes::vertebra::{FilmMarks, VertebraMarks};

/// Corner spread as a share of each vertebra's height on the film. The
/// finder's median corner error is 0.12 of a height as a distance, which is
/// about 0.10 along each axis.
const SHARE: f64 = 0.1;

/// The front film alone, with the built-in typical spine.
fn front_only(name: RecipeName, seed: u32, spread: MarkSpread) -> (FitFromFilms, AnswerKey) {
    let (input, key) = input(name, seed, 0.0);
    (
        FitFromFilms {
            front: FilmToFit {
                facing_assumed: false,
                marks: input.front,
                geometry: input.front_geometry,
                size: input.front_size,
                mark_spread: spread,
            },
            side: None,
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        },
        key,
    )
}

fn front_vertebrae(marks: &mut FilmMarks) -> impl Iterator<Item = &mut FrontMarks> {
    marks.vertebrae.iter_mut().map(|v| match &mut v.marks {
        VertebraMarks::Front { points } => points,
        _ => unreachable!("front film"),
    })
}

/// As the trained finder gives them: no pedicles.
fn hide_pedicles(marks: &mut FilmMarks) {
    for points in front_vertebrae(marks) {
        for pedicle in [&mut points.ped_l, &mut points.ped_r] {
            *pedicle = PedicleMark::NotVisible {
                source: MarkSource::Auto,
            };
        }
    }
}

/// Each corner moved along each film axis by a normal step of `share`
/// times its vertebra's height on the film.
fn shake_corners(marks: &mut FilmMarks, share: f64, seed: u64) {
    let mut stream = Stream::new("corners as a share of height", seed);
    for points in front_vertebrae(marks) {
        let middle = |a: Px, b: Px| ((a.x() + b.x()) * 0.5, (a.y() + b.y()) * 0.5);
        let top = middle(points.sup_l.at, points.sup_r.at);
        let bottom = middle(points.inf_l.at, points.inf_r.at);
        let sd = share * libm::hypot(top.0 - bottom.0, top.1 - bottom.1);
        for mark in [
            &mut points.sup_l,
            &mut points.sup_r,
            &mut points.inf_l,
            &mut points.inf_r,
        ] {
            let (dx, dy) = (stream.normal(0.0, sd), stream.normal(0.0, sd));
            mark.at = Px::new(mark.at.x() + dx, mark.at.y() + dy);
        }
    }
}

/// The typical forward lean at a level and its spread, in degrees, for a
/// curve of `curve_deg`: the built-in file's side profiles interpolated by
/// curve size and held beyond the last row.
fn typical_lean_deg(level: Level, curve_deg: f64) -> [f64; 2] {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../typical-spine/typical-spine.json"))
            .expect("the typical spine reads");
    let row = Level::MEASURED
        .iter()
        .position(|&l| l == level)
        .map_or(0, |i| i + 1);
    let profiles = file["sideProfiles"].as_array().expect("profiles");
    let at = |p: &serde_json::Value, key: &str| p[key][row].as_f64().expect("a number");
    let curve = |p: &serde_json::Value| p["curveDeg"].as_f64().expect("a curve");
    let above = profiles
        .iter()
        .position(|p| curve(p) > curve_deg)
        .unwrap_or(profiles.len());
    let (a, b) = match above {
        0 => (&profiles[0], &profiles[0]),
        i if i == profiles.len() => (&profiles[i - 1], &profiles[i - 1]),
        i => (&profiles[i - 1], &profiles[i]),
    };
    let t = if curve(b) > curve(a) {
        (curve_deg - curve(a)) / (curve(b) - curve(a))
    } else {
        0.0
    };
    ["forwardLeanDeg", "sdDeg"].map(|key| at(a, key) + t * (at(b, key) - at(a, key)))
}

fn truth(key: &AnswerKey, level: Level) -> &VertebraPose {
    key.poses
        .iter()
        .find(|pose| pose.level == level)
        .expect("the level is in the key")
}

#[test]
fn one_front_film_with_exact_marks_measures_what_it_shows() {
    // Measured largest errors over the four recipes, every vertebra:
    // centre y 0.304 mm, centre z 0.172 mm, side tilt 2.57 degrees,
    // half-widths 3.49% and height 4.58%. The film shows height only as
    // height times the cosine of forward lean, and lean here is the typical
    // spine's, so height carries the gap between the made-up lean and the
    // typical one. The pedicles read the most turned vertebrae (20 to 27
    // degrees) a few degrees short, and the film then reads part of the
    // turn as side tilt and a narrower body. On a turned body a gap in lean
    // also reads as side tilt: the made-up spines draw their kyphosis
    // whatever the curve's size, while the typical side profile for the
    // double major's 54 degree curve is flatter, 5 to 7 degrees off the
    // made-up lean at T11 to L2, where the largest tilt error sits. Centres
    // are measured from the origin's underside, which carries the origin's
    // own tilt error. Limits: 0.4 mm, 0.25 mm, 3 degrees, 4% and 7%.
    let mut largest = [0.0_f64; 5];
    for name in [
        RecipeName::Straight,
        RecipeName::RightThoracic40,
        RecipeName::LumbarLeft30,
        RecipeName::DoubleMajor55To45,
    ] {
        let (input, key) = front_only(
            name,
            3,
            MarkSpread::Millimetres {
                sd_mm: 1.0,
                shared_sd_mm: 0.0,
            },
        );
        let fit = fit_from_films(&input).expect("one front film fits");
        assert!(fit.converged, "{name:?}: {:?}", fit.stop_reason);
        assert_eq!(fit.lengths, FitLengths::Measured);
        assert_eq!(fit.origin_level, Level::L5);
        assert_eq!(fit.vertebrae.len(), key.poses.len());
        for fitted in &fit.vertebrae {
            let true_pose = truth(&key, fitted.level);
            assert!(fitted.inferred.depth && !fitted.inferred.width);
            assert!(fitted.rms_mm.side.is_none());
            let errors = [
                (fitted.pose.centre[1] - true_pose.centre[1]).abs(),
                (fitted.pose.centre[2] - true_pose.centre[2]).abs(),
                (fitted.angles.side_tilt_rad - angles(true_pose)[0])
                    .abs()
                    .to_degrees(),
                (fitted.pose.top_half_width / true_pose.top_half_width - 1.0)
                    .abs()
                    .max((fitted.pose.bottom_half_width / true_pose.bottom_half_width - 1.0).abs()),
                (fitted.pose.height / true_pose.height - 1.0).abs(),
            ];
            for (most, error) in largest.iter_mut().zip(errors) {
                *most = most.max(error);
            }
        }
        assert_eq!(fit.side_scale, 1.0);
        assert_eq!(fit.side_offset_mm, 0.0);
    }
    eprintln!(
        "one front film, exact marks: largest y, z mm, tilt deg, width share, height share {largest:?}"
    );
    for (got, limit) in largest.into_iter().zip([0.4, 0.25, 3.0, 0.04, 0.07]) {
        assert!(got < limit, "{got} against {limit}");
    }
}

#[test]
fn a_film_without_a_scale_gives_the_same_angles_at_a_typical_size() {
    // A parallel film scales every length alike, so only the priors given
    // in millimetres (the weak centre smoothness) can tell the two fits
    // apart. Measured largest difference in any named angle or wedge:
    // 0.0222 degrees. Limit 0.05 degrees.
    let spread = MarkSpread::ShareOfHeight {
        share: SHARE,
        shared_share: 0.0,
    };
    let (scaled, _) = front_only(RecipeName::RightThoracic40, 3, spread);
    let mut unscaled = scaled.clone();
    unscaled.front.geometry.mm_per_px = None;
    let with_scale = fit_from_films(&scaled).expect("the scaled film fits");
    let without = fit_from_films(&unscaled).expect("the film without a scale fits");
    assert_eq!(with_scale.lengths, FitLengths::Measured);
    assert_eq!(without.lengths, FitLengths::TypicalSize);
    assert!(without.converged);
    let mut largest = 0.0_f64;
    for (a, b) in with_scale.vertebrae.iter().zip(&without.vertebrae) {
        assert_eq!(a.level, b.level);
        for (x, y) in [
            (a.angles.side_tilt_rad, b.angles.side_tilt_rad),
            (a.angles.forward_lean_rad, b.angles.forward_lean_rad),
            (a.angles.twist_rad, b.angles.twist_rad),
            (a.pose.wedge_front, b.pose.wedge_front),
            (a.pose.wedge_side, b.pose.wedge_side),
        ] {
            largest = largest.max((x - y).abs().to_degrees());
        }
    }
    eprintln!("without a scale: largest angle difference {largest:.6} degrees");
    assert!(largest < 0.05, "{largest} degrees");
}

#[test]
fn noisy_corners_report_an_honest_spread() {
    // Corners shaken by 0.1 of each vertebra's film height along each axis,
    // with the same spread given to the fit, on two recipes and two seeds.
    // Measured median actual error over reported sd: centre y 1.46,
    // centre z 0.60, side tilt 0.69. Smallest forward-lean sd over its
    // typical spread, for the side profile the measured curve chose: 0.961.
    // Limits: 0.5 to 2, and 0.9.
    let mut ratios: [Vec<f64>; 3] = core::array::from_fn(|_| Vec::new());
    let mut lean_share = f64::INFINITY;
    for name in [RecipeName::RightThoracic40, RecipeName::LumbarLeft30] {
        for seed in 1..=2 {
            let (mut input, key) = front_only(
                name,
                seed,
                MarkSpread::ShareOfHeight {
                    share: SHARE,
                    shared_share: 0.0,
                },
            );
            hide_pedicles(&mut input.front.marks);
            shake_corners(&mut input.front.marks, SHARE, u64::from(seed));
            let fit = fit_from_films(&input).expect("noisy corners fit");
            assert!(fit.converged, "{name:?} seed {seed}");
            let curve = fit
                .typical_curve_deg
                .expect("the curve that chose the profile");
            for fitted in &fit.vertebrae {
                let true_pose = truth(&key, fitted.level);
                let errors = [
                    fitted.pose.centre[1] - true_pose.centre[1],
                    fitted.pose.centre[2] - true_pose.centre[2],
                    fitted.angles.side_tilt_rad - angles(true_pose)[0],
                ];
                let sds = [
                    fitted.sd.centre_mm[1],
                    fitted.sd.centre_mm[2],
                    fitted.sd.angles_rad[0],
                ];
                for ((list, error), sd) in ratios.iter_mut().zip(errors).zip(sds) {
                    assert!(sd.is_finite() && sd > 0.0);
                    list.push(error.abs() / sd);
                }
                lean_share = lean_share.min(
                    fitted.sd.angles_rad[1] / typical_lean_deg(fitted.level, curve)[1].to_radians(),
                );
            }
        }
    }
    let medians = ratios.each_mut().map(|list| median(list));
    eprintln!(
        "noisy corners: median actual/sd for y, z, tilt {medians:?}; smallest lean sd share {lean_share:.4}"
    );
    for ratio in medians {
        assert!((0.5..2.0).contains(&ratio), "actual/sd {ratio}");
    }
    assert!(lean_share >= 0.9, "lean sd share {lean_share}");
}

#[test]
fn twist_readings_are_followed_where_given() {
    // Readings at the true twist with a 1 degree spread, on the levels
    // where the made-up curve turns most. Measured largest gap between the
    // fitted twist and its reading: 0.002 degrees. Limit 0.5 degrees, far
    // below the true twists there (8 to 18 degrees), which the typical
    // twist for the curve would otherwise pull toward its own value.
    let (mut input, key) = front_only(
        RecipeName::RightThoracic40,
        3,
        MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
    );
    hide_pedicles(&mut input.front.marks);
    let read = [Level::T7, Level::T8, Level::T9];
    input.twist_readings = read
        .iter()
        .map(|&level| TwistReading {
            level,
            twist_rad: angles(truth(&key, level))[2],
            sd_rad: 1.0_f64.to_radians(),
        })
        .collect();
    let fit = fit_from_films(&input).expect("readings fit");
    assert!(fit.converged);
    let mut largest = 0.0_f64;
    for fitted in &fit.vertebrae {
        let given = input
            .twist_readings
            .iter()
            .find(|r| r.level == fitted.level);
        assert_eq!(fitted.inferred.twist, given.is_none(), "{:?}", fitted.level);
        if let Some(reading) = given {
            assert!(reading.twist_rad.abs().to_degrees() > 3.0);
            largest = largest.max(
                (fitted.angles.twist_rad - reading.twist_rad)
                    .abs()
                    .to_degrees(),
            );
        }
    }
    eprintln!("twist readings: largest gap {largest:.6} degrees");
    assert!(largest < 0.5, "{largest} degrees");
}

#[test]
fn too_few_vertebrae_and_a_missing_scale_are_named_refusals() {
    let (input, _) = front_only(
        RecipeName::Straight,
        3,
        MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
    );
    let mut one = input.clone();
    one.front.marks.vertebrae.truncate(1);
    assert_eq!(fit_from_films(&one), Err(FitError::TooFewVertebrae));
    let mut none = input.clone();
    none.front.marks.vertebrae.clear();
    assert_eq!(fit_from_films(&none), Err(FitError::NoMarks));
    let mut unscaled = input.clone();
    unscaled.front.geometry.mm_per_px = None;
    unscaled.typical = TypicalSource::None;
    assert_eq!(
        fit_from_films(&unscaled),
        Err(FitError::MissingScale(FilmKind::Front))
    );
    let mut untypical = input.clone();
    untypical.typical = TypicalSource::None;
    assert_eq!(fit_from_films(&untypical), Err(FitError::Underconstrained));
    let mut spread = input.clone();
    for mark_spread in [
        MarkSpread::ShareOfHeight {
            share: 0.0,
            shared_share: 0.0,
        },
        MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: -0.1,
        },
    ] {
        spread.front.mark_spread = mark_spread;
        assert_eq!(
            fit_from_films(&spread),
            Err(FitError::InvalidMarkSpread(FilmKind::Front))
        );
    }
    let reading = TwistReading {
        level: Level::T8,
        twist_rad: 0.0,
        sd_rad: 0.1,
    };
    for readings in [
        vec![TwistReading {
            sd_rad: 0.0,
            ..reading
        }],
        vec![TwistReading {
            level: Level::C7,
            ..reading
        }],
        vec![reading, reading],
    ] {
        let mut read = input.clone();
        read.twist_readings = readings;
        assert_eq!(fit_from_films(&read), Err(FitError::InvalidTwistReadings));
    }
}

#[test]
fn both_films_without_the_typical_spine_match_the_two_film_fit() {
    // The same rows, the same start and the same solver: only the reported
    // spread differs, because it is not rescaled by the residual. Measured
    // largest difference: 0 mm and 0 radians. Limit 1e-9.
    let (input, _) = input(RecipeName::LumbarLeft30, 2, 1.0);
    let spread = MarkSpread::Millimetres {
        sd_mm: 1.0,
        shared_sd_mm: 0.0,
    };
    let films = FitFromFilms {
        front: FilmToFit {
            facing_assumed: false,
            marks: input.front.clone(),
            geometry: input.front_geometry.clone(),
            size: input.front_size,
            mark_spread: spread,
        },
        side: Some(FilmToFit {
            facing_assumed: false,
            marks: input.side.clone(),
            geometry: input.side_geometry.clone(),
            size: input.side_size,
            mark_spread: spread,
        }),
        typical: TypicalSource::None,
        twist_readings: Vec::new(),
    };
    let two = fit_spine(&input).expect("the two-film fit");
    let from_films = fit_from_films(&films).expect("the same films through the new entry");
    assert_eq!(from_films.lengths, FitLengths::Measured);
    assert_eq!(from_films.typical_curve_deg, None);
    assert_eq!(two.iterations, from_films.iterations);
    let mut largest = 0.0_f64;
    for (a, b) in two.vertebrae.iter().zip(&from_films.vertebrae) {
        assert_eq!(a.level, b.level);
        assert_eq!(a.inferred, b.inferred);
        assert_eq!(a.corner_tilt_rad, b.corner_tilt_rad);
        for (x, y) in a.pose.centre.into_iter().zip(b.pose.centre) {
            largest = largest.max((x - y).abs());
        }
        for (x, y) in a.pose.rotation.into_iter().zip(b.pose.rotation) {
            largest = largest.max((x - y).abs());
        }
        for (x, y) in answer_keys::sizes(&a.pose)
            .into_iter()
            .zip(answer_keys::sizes(&b.pose))
        {
            largest = largest.max((x - y).abs());
        }
    }
    eprintln!("both films through the new entry: largest difference {largest:e}");
    assert!(largest < 1e-9, "{largest}");
    assert!((two.side_scale - from_films.side_scale).abs() < 1e-9);
    assert!((two.side_offset_mm - from_films.side_offset_mm).abs() < 1e-9);
    assert_eq!(
        (two.front_film, two.side_film),
        (from_films.front_film, from_films.side_film)
    );
}

#[test]
fn the_largest_curve_chooses_the_side_profile_and_turns_the_bodies() {
    // The double major, one front film, exact marks, no pedicles. Measured:
    // largest curve 53.69 degrees against the answer key's largest Cobb
    // angle of 53.64. At the 16 levels where the side profile for that
    // curve differs from the one for no curve by more than 3 degrees (T1 to
    // T7: flatter at the top; T9 to L5: leaning further forward below), the
    // fitted lean moved from the no-curve profile toward the chosen one by
    // 0.91 (T11) to 1.01 of the difference, the rest held by the stacking
    // rows and the film. Only the largest curve, the thoracic one, gives a
    // typical twist; the lumbar curve and the smaller ones keep a twist of
    // zero give or take the no-curve spread. The 5 vertebrae turned by
    // over 1 degree, T7 to T11, all turn the way the made-up spine turns
    // them, negative in a curve that bulges to the right, by 0.52 to 0.66
    // of the made-up twist. Limits: curve within 3 degrees, lean moved
    // over half way, every sign as built.
    //
    // Made-up spines tie twist to the curve by construction (the apex
    // twist is the curve's size times a drawn factor, turned toward the
    // convexity), so agreeing with their twist proves the sign convention
    // and nothing about real spines.
    let (mut input, key) = front_only(
        RecipeName::DoubleMajor55To45,
        3,
        MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
    );
    hide_pedicles(&mut input.front.marks);
    let fit = fit_from_films(&input).expect("one front film fits");
    assert!(fit.converged, "{:?}", fit.stop_reason);
    let curve = fit
        .typical_curve_deg
        .expect("the curve that chose the profile");
    let cobb = key
        .angles
        .frontal_cobb
        .iter()
        .map(|c| c.degrees)
        .fold(0.0, f64::max);
    let mut moved = Vec::new();
    let mut turned = Vec::new();
    for fitted in &fit.vertebrae {
        let [straight, _] = typical_lean_deg(fitted.level, 0.0);
        let [chosen, _] = typical_lean_deg(fitted.level, curve);
        let lean = fitted.angles.forward_lean_rad.to_degrees();
        if (chosen - straight).abs() > 3.0 {
            moved.push((fitted.level, (lean - straight) / (chosen - straight)));
        }
        let true_twist = angles(truth(&key, fitted.level))[2].to_degrees();
        let twist = fitted.angles.twist_rad.to_degrees();
        assert!(fitted.inferred.twist);
        if twist.abs() > 1.0 {
            turned.push((fitted.level, true_twist, twist));
        }
    }
    eprintln!(
        "largest curve {curve:.3} against the key's {cobb:.3} degrees; lean moved toward the chosen row {moved:?}; twist true, fitted {turned:?}"
    );
    assert!((curve - cobb).abs() < 3.0, "{curve} against {cobb}");
    assert!(!moved.is_empty());
    for (level, share) in &moved {
        assert!(*share > 0.5, "{level:?}: {share}");
    }
    assert!(!turned.is_empty());
    for (level, true_twist, twist) in &turned {
        assert!(
            true_twist * twist > 0.0,
            "{level:?}: {true_twist} against {twist}"
        );
    }
}

/// A spine built by hand, T1 to L5, every body upright except for its side
/// tilt: one curve bulging to the patient's left, whose end vertebrae sit in
/// straight runs of equal tilt, +20 degrees from T1 to T4 and −20 degrees
/// from L1 to L5, joined smoothly from T4 to L1. Each centre sits 25 mm
/// above the one below along the pair's mean axis.
fn hand_made_curve() -> Vec<VertebraParams> {
    let tilts: Vec<f64> = (0..17)
        .map(|k| {
            let along = ((k as f64 - 3.0) / 9.0).clamp(0.0, 1.0);
            20.0_f64.to_radians() * libm::cos(PI * along)
        })
        .collect();
    // rotX(θ) sends the body's axis to (0, −sin θ, cos θ).
    let axis = |tilt: f64| Vector3::new(0.0, -libm::sin(tilt), libm::cos(tilt));
    let mut centres = vec![Vector3::new(0.0, 0.0, 11.0); 17];
    for k in (0..16).rev() {
        centres[k] = centres[k + 1] + (axis(tilts[k]) + axis(tilts[k + 1])).normalize() * 25.0;
    }
    tilts
        .iter()
        .zip(centres)
        .enumerate()
        .map(|(k, (&tilt, centre))| {
            let half_width = 15.0 + 0.5 * k as f64;
            VertebraParams {
                centre,
                rotation: Vector3::new(tilt, 0.0, 0.0),
                height: 18.0 + 0.4 * k as f64,
                wedge_front: 0.0,
                wedge_side: 0.0,
                top_half_width: half_width,
                top_half_depth: 0.8 * half_width,
                bottom_half_width: half_width,
                bottom_half_depth: 0.8 * half_width,
            }
        })
        .collect()
}

/// The hand-made spine on a front film facing the patient's left, with no
/// pedicles, fitted with the typical spine.
fn hand_made_film() -> FitFromFilms {
    let geometry = FilmGeometry {
        mm_per_px: Some(0.3),
        image_right: Some(ImageRight::PatientLeft),
        ..FilmGeometry::UNKNOWN
    };
    let size = [1200, 2600];
    let camera = ParallelCamera::for_film(&geometry, size, Vector2::new(250.0, 2300.0))
        .expect("a known camera");
    let (template, _) = front_only(
        RecipeName::Straight,
        3,
        MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
    );
    let mut input = FitFromFilms {
        front: FilmToFit {
            facing_assumed: false,
            geometry,
            size,
            ..template.front
        },
        ..template
    };
    input.front.marks.named_for = ImageRight::PatientLeft;
    input.front.marks.vertebrae.retain(|v| v.level >= Level::T1);
    for (v, body) in input
        .front
        .marks
        .vertebrae
        .iter_mut()
        .zip(hand_made_curve())
    {
        v.marks = VertebraMarks::Front {
            points: front_marks(&body, &camera).expect("front marks"),
        };
    }
    assert_eq!(input.front.marks.vertebrae.len(), 17);
    hide_pedicles(&mut input.front.marks);
    input
}

/// The same marks with each left corner's name given to the right one, and
/// back, and each pixel moved by `x`: a film of the mirrored spine.
fn mirrored(input: &FitFromFilms, facing: ImageRight, x: impl Fn(f64) -> f64) -> FitFromFilms {
    let mut film = input.clone();
    film.front.geometry.image_right = Some(facing);
    film.front.marks.named_for = facing;
    for points in front_vertebrae(&mut film.front.marks) {
        std::mem::swap(&mut points.sup_l, &mut points.sup_r);
        std::mem::swap(&mut points.inf_l, &mut points.inf_r);
        for mark in [
            &mut points.sup_l,
            &mut points.sup_r,
            &mut points.inf_l,
            &mut points.inf_r,
        ] {
            mark.at = Px::new(x(mark.at.x()), mark.at.y());
        }
    }
    film
}

/// The largest difference between `fit` and the mirror image of `other`:
/// centre y, side tilt, twist, front wedge and the corners' tilt change
/// sign, everything else is equal, standard deviations included.
fn mirror_gap(fit: &SpineFit, other: &SpineFit) -> f64 {
    assert_eq!(fit.vertebrae.len(), other.vertebrae.len());
    assert_eq!(fit.origin_level, other.origin_level);
    let curve = |fit: &SpineFit| fit.typical_curve_deg.expect("a curve");
    let mut largest = (curve(fit) - curve(other)).abs();
    for (a, b) in fit.vertebrae.iter().zip(&other.vertebrae) {
        assert_eq!(a.level, b.level);
        let [x, y, z] = b.pose.centre;
        let [rx, ry, rz] = b.pose.rotation;
        let [tilt, lean, twist] = [
            b.angles.side_tilt_rad,
            b.angles.forward_lean_rad,
            b.angles.twist_rad,
        ];
        let corner_tilt = |v: &FittedVertebra| v.corner_tilt_rad.expect("front marks");
        let pairs = [
            (a.pose.centre, [x, -y, z]),
            (a.pose.rotation, [-rx, ry, -rz]),
            (
                [a.pose.height, a.pose.wedge_front, a.pose.wedge_side],
                [b.pose.height, -b.pose.wedge_front, b.pose.wedge_side],
            ),
            (
                [
                    a.pose.top_half_width,
                    a.pose.top_half_depth,
                    a.pose.bottom_half_width,
                ],
                [
                    b.pose.top_half_width,
                    b.pose.top_half_depth,
                    b.pose.bottom_half_width,
                ],
            ),
            (
                [a.pose.bottom_half_depth, corner_tilt(a), 0.0],
                [b.pose.bottom_half_depth, -corner_tilt(b), 0.0],
            ),
            (
                [
                    a.angles.side_tilt_rad,
                    a.angles.forward_lean_rad,
                    a.angles.twist_rad,
                ],
                [-tilt, lean, -twist],
            ),
            (a.sd.centre_mm, b.sd.centre_mm),
            (a.sd.angles_rad, b.sd.angles_rad),
        ];
        for (p, q) in pairs {
            for (u, v) in p.into_iter().zip(q) {
                largest = largest.max((u - v).abs());
            }
        }
    }
    largest
}

#[test]
fn a_mirrored_film_gives_the_mirrored_fit() {
    // The hand-made curve, whose end vertebrae sit in runs of equal tilt,
    // fitted as drawn, as the same pixels said to face the patient's
    // right, and as a picture flipped left to right: both describe the
    // mirrored spine. Measured largest difference from the exact mirror
    // image, in mm, radians and degrees of curve: 3.7e-10 (same pixels)
    // and 3.0e-10 (flipped picture). Limit 1e-9. The 40 degree curve gives
    // its apex a typical twist of 16.2 degrees.
    //
    // The curves and every typical value come out exactly mirrored. The
    // solver's tests for taking a step and for stopping compare costs to
    // their last digits, and a mirrored film adds its rows up in another
    // order, so two mirrored solves can stop an iteration apart and then
    // differ by the last step: this spine drawn at the middle of the film
    // stops after 8 and 6 iterations and differs by 4.0e-7 mm.
    let drawn = hand_made_film();
    let fit = fit_from_films(&drawn).expect("the drawn film fits");
    assert!(fit.converged);
    let last = f64::from(drawn.front.size[0]) - 1.0;
    let same_pixels = mirrored(&drawn, ImageRight::PatientRight, |x| x);
    let flipped = mirrored(&drawn, ImageRight::PatientLeft, |x| last - x);
    let gaps = [same_pixels, flipped].map(|film| {
        let other = fit_from_films(&film).expect("the mirrored film fits");
        assert!(other.converged);
        mirror_gap(&fit, &other)
    });
    let twist: Vec<(Level, f64)> = fit
        .vertebrae
        .iter()
        .map(|v| (v.level, v.angles.twist_rad.to_degrees()))
        .collect();
    eprintln!(
        "mirrored film: largest gaps {gaps:?}; curve {:?} degrees; twist {twist:?}",
        fit.typical_curve_deg
    );
    for gap in gaps {
        assert!(gap < 1e-9, "{gap}");
    }
}

#[test]
fn a_shared_part_of_the_spread_widens_only_the_centre_across_and_up() {
    // The same front film fitted with and without a part of the corner
    // error shared by a whole box, in millimetres and as a share of each
    // vertebra's height on the film: the poses and every other spread are
    // identical, and the spreads of centre y and centre z grow to exactly
    // the root of the sum of the squares.
    let one_mm = MarkSpread::Millimetres {
        sd_mm: 1.0,
        shared_sd_mm: 0.0,
    };
    let (alone, _) = front_only(RecipeName::LumbarLeft30, 3, one_mm);
    let scale = alone.front.geometry.mm_per_px.expect("a scale");
    assert_eq!(alone.front.geometry.mm_per_px_vertical, None);
    let heights_mm: Vec<f64> = alone
        .front
        .marks
        .vertebrae
        .iter()
        .map(|v| {
            let [a, b, c, d] = v.marks.corners().map(|m| m.at);
            let x = (a.x() + b.x() - c.x() - d.x()) * 0.5;
            let y = (a.y() + b.y() - c.y() - d.y()) * 0.5;
            scale * libm::hypot(x, y)
        })
        .collect();
    let cases = [
        (
            one_mm,
            MarkSpread::Millimetres {
                sd_mm: 1.0,
                shared_sd_mm: 0.7,
            },
            vec![0.7; heights_mm.len()],
        ),
        (
            MarkSpread::ShareOfHeight {
                share: SHARE,
                shared_share: 0.0,
            },
            MarkSpread::ShareOfHeight {
                share: SHARE,
                shared_share: 0.08,
            },
            heights_mm.iter().map(|h| 0.08 * h).collect(),
        ),
    ];
    for (without, with, shared_mm) in cases {
        let fit = |spread| {
            let mut input = alone.clone();
            input.front.mark_spread = spread;
            fit_from_films(&input).expect("the film fits")
        };
        let (plain, widened) = (fit(without), fit(with));
        let mut largest = 0.0_f64;
        for ((a, b), shared) in plain
            .vertebrae
            .iter()
            .zip(&widened.vertebrae)
            .zip(&shared_mm)
        {
            assert_eq!(
                (a.pose, a.angles, a.sd.angles_rad, a.sd.centre_mm[0]),
                (b.pose, b.angles, b.sd.angles_rad, b.sd.centre_mm[0])
            );
            for axis in [1, 2] {
                let expected = libm::hypot(a.sd.centre_mm[axis], *shared);
                largest = largest.max((b.sd.centre_mm[axis] - expected).abs());
            }
        }
        assert_eq!(
            (plain.rms_mm, plain.front_film, plain.typical_curve_deg),
            (
                widened.rms_mm,
                widened.front_film,
                widened.typical_curve_deg
            )
        );
        assert!(largest < 1e-12, "{largest}");
    }
}

#[test]
fn the_corners_tilt_is_read_from_the_front_film_alone() {
    // Two films, T1 left off the front film: T1 has no corner tilt, and
    // every other vertebra's is the line from the middle of its right side
    // to the middle of its left, worked here from its pixels on a film
    // facing the patient's left with square pixels.
    let (mut input, _) = input(RecipeName::RightThoracic40, 3, 0.0);
    assert_eq!(
        input.front_geometry.image_right,
        Some(ImageRight::PatientLeft)
    );
    assert_eq!(input.front_geometry.quarter_turns.get(), 0);
    assert_eq!(input.front_geometry.mm_per_px_vertical, None);
    input.front.vertebrae.retain(|v| v.level != Level::T1);
    let fit = fit_spine(&input).expect("the films fit");
    for fitted in &fit.vertebrae {
        let marked = input
            .front
            .vertebrae
            .iter()
            .find(|v| v.level == fitted.level);
        let Some(marked) = marked else {
            assert_eq!(fitted.level, Level::T1);
            assert_eq!(fitted.corner_tilt_rad, None);
            continue;
        };
        let [sup_l, sup_r, inf_r, inf_l] = marked.marks.corners().map(|m| m.at);
        // Picture right is the patient's left; picture down is down.
        let across = (sup_l.x() + inf_l.x()) - (sup_r.x() + inf_r.x());
        let up = (sup_r.y() + inf_r.y()) - (sup_l.y() + inf_l.y());
        let tilt = fitted.corner_tilt_rad.expect("front marks");
        assert!((tilt - libm::atan2(up, across)).abs() < 1e-12, "{tilt}");
    }
}
