//! Fits are compared with poses saved by the independent spine generator.

mod answer_keys;

use answer_keys::{angles, anisotropic_input, distance, input, median, sizes, upright_input};
use forward_model::endplates::Endplates;
use forward_model::vertebra_params::VertebraParams;
use joint_fit::{FitError, fit_spine};
use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::marks::{MarkSource, PedicleMark};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks};

#[test]
fn perfect_marks_recover_the_saved_poses() {
    // Limits: 0.05 mm for each centre, 2 degrees for every angle and 2.5%
    // for every size. Measured maxima with these priors: 0.031 mm, 1.71
    // degrees and 2.02%, both on the most curved recipes; the straight spine
    // comes back within 0.003 mm, 0.008 degrees and 0.23%. Centres are pinned
    // by the corners. Twist, the two wedges and the sizes are not: on a
    // strongly turned vertebra the films barely tell them apart, so even
    // perfect marks leave a direction the weak priors decide. The reported
    // standard deviations cover that (see the honest-uncertainty test).
    let mut failures = Vec::new();
    for name in [
        RecipeName::Straight,
        RecipeName::RightThoracic40,
        RecipeName::LumbarLeft30,
        RecipeName::DoubleMajor55To45,
        RecipeName::UpsideDown,
        RecipeName::MirroredFront,
        RecipeName::SideFacingLeft,
    ] {
        let (input, key) = input(name, 3, 0.0);
        let fit = fit_spine(&input).expect("perfect marks fit");
        assert!(fit.converged, "{name:?}: {} iterations", fit.iterations);
        assert_eq!(fit.origin_level, Level::L5);
        assert_eq!(fit.vertebrae.len(), key.poses.len());
        let mut largest_position = 0.0_f64;
        let mut largest_angle = 0.0_f64;
        let mut largest_size = 0.0_f64;
        for (fitted, truth) in fit.vertebrae.iter().zip(&key.poses) {
            assert_eq!(fitted.level, truth.level);
            largest_position = largest_position.max(distance(fitted.pose.centre, truth.centre));
            let got = [
                fitted.angles.side_tilt_rad,
                fitted.angles.forward_lean_rad,
                fitted.angles.twist_rad,
                fitted.pose.wedge_front,
                fitted.pose.wedge_side,
            ];
            let true_angles = angles(truth);
            let expected = [
                true_angles[0],
                true_angles[1],
                true_angles[2],
                truth.wedge_front,
                truth.wedge_side,
            ];
            for (got, expected) in got.into_iter().zip(expected) {
                largest_angle = largest_angle.max((got - expected).abs().to_degrees());
            }
            for (got, expected) in sizes(&fitted.pose).into_iter().zip(sizes(truth)) {
                largest_size = largest_size.max((got / expected - 1.0).abs());
            }
        }
        eprintln!(
            "{name:?}: largest centre {largest_position:.6} mm, angle {largest_angle:.6} deg, size {:.6}%",
            largest_size * 100.0
        );
        if largest_position >= 0.05 || largest_angle >= 2.0 || largest_size >= 0.025 {
            failures.push(format!("{name:?}: centre {largest_position} mm, angle {largest_angle} degrees, relative size {largest_size}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn twenty_seeded_noisy_spines_have_accurate_poses_and_honest_uncertainty() {
    // Independent 1 mm Gaussian clicks on four recipes and five seeds.
    // The limits are the specified 2 mm, 2/2/4 degrees and median
    // absolute-error/standard-deviation ratio between 0.5 and 2.
    // Measured medians with weak priors: centre 1.046342 mm; side tilt
    // 1.290252 degrees, forward lean 1.210083 degrees, twist 2.444156
    // degrees. Median actual-error/sd: centre 1.269896, named angles 1.208203.
    // External release timing of these same 20 fits, excluding fixture
    // creation: median 13.944 ms, maximum 28.691 ms (limit 200 ms).
    let mut positions = Vec::new();
    let mut angle_errors: [Vec<f64>; 3] = core::array::from_fn(|_| Vec::new());
    let mut centre_ratios = Vec::new();
    let mut angle_ratios = Vec::new();
    for name in [
        RecipeName::Straight,
        RecipeName::RightThoracic40,
        RecipeName::LumbarLeft30,
        RecipeName::DoubleMajor55To45,
    ] {
        for seed in 1..=5 {
            let (input, key) = input(name, seed, 1.0);
            let fit = fit_spine(&input).expect("noisy marks fit");
            assert!(
                fit.converged,
                "{name:?} seed {seed}: {} iterations",
                fit.iterations
            );
            assert_eq!(fit.vertebrae.len(), key.poses.len());
            for (fitted, truth) in fit.vertebrae.iter().zip(&key.poses) {
                assert_eq!(fitted.level, truth.level);
                positions.push(distance(fitted.pose.centre, truth.centre));
                let got_angles = [
                    fitted.angles.side_tilt_rad,
                    fitted.angles.forward_lean_rad,
                    fitted.angles.twist_rad,
                ];
                let true_angles = angles(truth);
                for axis in 0..3 {
                    let centre_sd = fitted.sd.centre_mm[axis];
                    let angle_sd = fitted.sd.angles_rad[axis];
                    assert!(centre_sd.is_finite() && centre_sd > 0.0);
                    assert!(angle_sd.is_finite() && angle_sd > 0.0);
                    centre_ratios
                        .push((fitted.pose.centre[axis] - truth.centre[axis]).abs() / centre_sd);
                    let error = (got_angles[axis] - true_angles[axis]).abs();
                    angle_errors[axis].push(error.to_degrees());
                    angle_ratios.push(error / angle_sd);
                }
            }
        }
    }
    let position = median(&mut positions);
    let angle = angle_errors.each_mut().map(|values| median(values));
    let centre_ratio = median(&mut centre_ratios);
    let angle_ratio = median(&mut angle_ratios);
    eprintln!(
        "20 noisy spines: median position {position:.6} mm; angles {angle:?} deg; actual/sd centre {centre_ratio:.6}, angle {angle_ratio:.6}"
    );
    assert!(position < 2.0, "median position {position} mm");
    for (got, limit) in angle.into_iter().zip([2.0, 2.0, 4.0]) {
        assert!(got < limit, "median angle {got} degrees, limit {limit}");
    }
    assert!(
        (0.5..2.0).contains(&centre_ratio),
        "centre actual/sd {centre_ratio}"
    );
    assert!(
        (0.5..2.0).contains(&angle_ratio),
        "angle actual/sd {angle_ratio}"
    );
}

#[test]
fn a_missing_side_view_is_inferred_within_five_millimetres() {
    let (mut input, key) = input(RecipeName::RightThoracic40, 3, 0.0);
    input
        .side
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::T8);
    let fit = fit_spine(&input).expect("one missing side view still fits");
    let fitted = fit
        .vertebrae
        .iter()
        .find(|vertebra| vertebra.level == Level::T8)
        .unwrap();
    let truth = key
        .poses
        .iter()
        .find(|pose| pose.level == Level::T8)
        .unwrap();
    assert!(fit.converged);
    assert!(fitted.inferred.depth);
    assert!(!fitted.inferred.width);
    assert!(fitted.rms_mm.side.is_none());
    assert!(distance(fitted.pose.centre, truth.centre) < 5.0);
    assert!((fitted.pose.top_half_depth - truth.top_half_depth).abs() < 5.0);
    assert!((fitted.pose.bottom_half_depth - truth.bottom_half_depth).abs() < 5.0);
}

#[test]
fn a_missing_front_view_is_inferred_within_five_millimetres() {
    let (mut input, key) = input(RecipeName::RightThoracic40, 3, 0.0);
    input
        .front
        .vertebrae
        .retain(|vertebra| vertebra.level != Level::T8);
    let fit = fit_spine(&input).expect("one missing front view still fits");
    let fitted = fit
        .vertebrae
        .iter()
        .find(|vertebra| vertebra.level == Level::T8)
        .unwrap();
    let truth = key
        .poses
        .iter()
        .find(|pose| pose.level == Level::T8)
        .unwrap();
    assert!(fit.converged);
    assert!(fitted.inferred.width && fitted.inferred.twist);
    assert!(!fitted.inferred.depth);
    assert!(fitted.rms_mm.front.is_none());
    assert!(distance(fitted.pose.centre, truth.centre) < 5.0);
}

#[test]
fn hidden_pedicles_infer_twist_and_keep_the_fit_convergent() {
    let (mut input, _) = input(RecipeName::RightThoracic40, 3, 0.0);
    let vertebra = input
        .front
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::T8)
        .unwrap();
    let VertebraMarks::Front { points } = &mut vertebra.marks else {
        unreachable!()
    };
    points.ped_l = PedicleMark::NotVisible {
        source: MarkSource::Hand,
    };
    points.ped_r = PedicleMark::NotVisible {
        source: MarkSource::Hand,
    };
    let fit = fit_spine(&input).expect("neighbours supply the missing twist");
    assert!(fit.converged);
    let fitted = fit.vertebrae.iter().find(|v| v.level == Level::T8).unwrap();
    assert!(fitted.inferred.twist);
}

#[test]
fn the_lowest_level_replaces_l5_as_origin_when_l5_is_absent() {
    let (mut input, _) = input(RecipeName::Straight, 3, 0.0);
    input.front.vertebrae.retain(|v| v.level != Level::L5);
    input.side.vertebrae.retain(|v| v.level != Level::L5);
    let fit = fit_spine(&input).expect("a cropped spine fits");
    assert_eq!(fit.origin_level, Level::L4);
    let last = fit.vertebrae.last().unwrap();
    let underside = Endplates::of(&VertebraParams::from(&last.pose))
        .bottom
        .centre;
    // Translation to the chosen underside involves only a few arithmetic
    // operations; 1e-9 mm allows their round-off.
    assert!(underside.norm() < 1e-9);
}

#[test]
fn the_same_marks_give_the_same_result() {
    let (input, _) = input(RecipeName::LumbarLeft30, 2, 1.0);
    let first = fit_spine(&input).expect("the first fit");
    let second = fit_spine(&input).expect("the second fit");
    assert_eq!(first, second);
}

#[test]
fn missing_scale_and_facing_are_named_refusals() {
    let (input, _) = input(RecipeName::Straight, 3, 0.0);
    let mut missing = input.clone();
    missing.front_geometry.mm_per_px = None;
    assert_eq!(
        fit_spine(&missing),
        Err(FitError::MissingScale(FilmKind::Front))
    );
    let mut missing = input.clone();
    missing.side_geometry.mm_per_px = None;
    assert_eq!(
        fit_spine(&missing),
        Err(FitError::MissingScale(FilmKind::Side))
    );
    let mut missing = input.clone();
    missing.front_geometry.image_right = None;
    assert_eq!(
        fit_spine(&missing),
        Err(FitError::MissingFacing(FilmKind::Front))
    );
    let mut missing = input;
    missing.side_geometry.image_right = None;
    assert_eq!(
        fit_spine(&missing),
        Err(FitError::MissingFacing(FilmKind::Side))
    );
}

#[test]
fn film_geometry_variants_match_the_same_upright_spine() {
    // Geometry-only names draw their curves through the same branch as
    // HeavyShoulders. That recipe changes the painting's shoulders, not
    // the stored spine poses, and keeps the films upright and normally
    // facing. Random consumes extra count/generator draws and is not the
    // matching base. Assert the saved poses before comparing any fit.
    let (base_input, base_key) = input(RecipeName::HeavyShoulders, 3, 0.0);
    let hidden: Vec<_> = base_input
        .front
        .vertebrae
        .iter()
        .filter_map(|v| {
            let VertebraMarks::Front { points } = &v.marks else {
                unreachable!()
            };
            (points.ped_l.mark().is_none() && points.ped_r.mark().is_none()).then_some(v.level)
        })
        .collect();
    eprintln!(
        "Matched base true curves: {:?}; levels without placed pedicles: {hidden:?}",
        base_key.angles.frontal_cobb
    );
    let base = fit_spine(&base_input).expect("the matched upright spine fits");
    assert!(base.converged);
    for name in [
        RecipeName::UpsideDown,
        RecipeName::MirroredFront,
        RecipeName::SideFacingLeft,
    ] {
        let (changed_input, changed_key) = input(name, 3, 0.0);
        assert_eq!(
            base_key.poses, changed_key.poses,
            "{name:?} must show the same spine"
        );
        let changed = fit_spine(&changed_input).expect("the geometry variant fits");
        assert!(changed.converged, "{name:?}: {:?}", changed.stop_reason);
        assert_eq!(base.vertebrae.len(), changed.vertebrae.len());
        let mut position = 0.0_f64;
        let mut angle = 0.0_f64;
        let mut size = 0.0_f64;
        for (a, b) in base.vertebrae.iter().zip(&changed.vertebrae) {
            assert_eq!(a.level, b.level);
            assert_eq!(a.inferred, b.inferred);
            position = position.max(distance(a.pose.centre, b.pose.centre));
            for (a, b) in angles(&a.pose).into_iter().zip(angles(&b.pose)) {
                angle = angle.max((a - b).abs());
            }
            angle = angle.max((a.pose.wedge_front - b.pose.wedge_front).abs());
            angle = angle.max((a.pose.wedge_side - b.pose.wedge_side).abs());
            for (a, b) in sizes(&a.pose).into_iter().zip(sizes(&b.pose)) {
                size = size.max((a - b).abs());
            }
        }
        eprintln!(
            "{name:?} against matched upright base: centre {position:e} mm, angle {angle:e} rad, size {size:e} mm; iterations {}/{}",
            base.iterations, changed.iterations
        );
        // Camera reflections/half-turns are pixel-metric isometries.
        // These allowances cover only central-difference round-off.
        assert!(position < 1e-4, "{name:?}: centre difference {position} mm");
        assert!(angle < 1e-5, "{name:?}: angle difference {angle} rad");
        assert!(size < 1e-4, "{name:?}: size difference {size} mm");
        assert!((base.side_scale - changed.side_scale).abs() < 1e-6);
        assert!((base.side_offset_mm - changed.side_offset_mm).abs() < 1e-4);
        assert!((base.rms_mm - changed.rms_mm).abs() < 1e-5);
    }
}

#[test]
fn turned_anisotropic_pixels_keep_the_same_physical_fit() {
    // The generator's answer key is recomputed using anisotropic cameras.
    // An anisotropic stretch of old corner marks would be a different
    // problem: the forward model defines its corner extremum in pixel
    // axes, so stretching changes the selected point on an oblique rim.
    let (mut input, _) = anisotropic_input(RecipeName::Straight, 3);
    let original = fit_spine(&input).expect("the original film fit");
    store_quarter_turn(
        &mut input.front,
        &mut input.front_geometry,
        &mut input.front_size,
        1,
    );
    store_quarter_turn(
        &mut input.side,
        &mut input.side_geometry,
        &mut input.side_size,
        3,
    );
    let changed = fit_spine(&input).expect("stored turns and non-square pixels fit");
    assert!(changed.converged);
    for (a, b) in original.vertebrae.iter().zip(&changed.vertebrae) {
        // Only coordinates changed. Central-difference round-off can move
        // the numerical minimum slightly; 1e-4 mm and 1e-5 rad are far
        // smaller than the requested reconstruction tolerances.
        assert!(distance(a.pose.centre, b.pose.centre) < 1e-4);
        for (a, b) in angles(&a.pose).into_iter().zip(angles(&b.pose)) {
            assert!((a - b).abs() < 1e-5);
        }
        for (a, b) in sizes(&a.pose).into_iter().zip(sizes(&b.pose)) {
            assert!((a - b).abs() < 1e-4);
        }
    }
}

/// Store upright pixels with the supplied quarter turn and swap their spacings.
fn store_quarter_turn(
    marks: &mut FilmMarks,
    geometry: &mut FilmGeometry,
    size: &mut [u32; 2],
    turns: u8,
) {
    assert_eq!(geometry.quarter_turns.get(), 0);
    let upright = *size;
    let mm = geometry
        .mm_per_px
        .expect("the answer's known pixel spacing");
    let vertical = geometry.mm_per_px_vertical.unwrap_or(mm);
    let spacing = [mm, vertical];
    *size = if turns.is_multiple_of(2) {
        upright
    } else {
        [upright[1], upright[0]]
    };
    let place = |at: &mut Px| {
        let (u, v) = (at.x(), at.y());
        *at = match turns {
            0 => Px::new(u, v),
            1 => Px::new(v, f64::from(size[1] - 1) - u),
            2 => Px::new(f64::from(size[0] - 1) - u, f64::from(size[1] - 1) - v),
            _ => Px::new(f64::from(size[0] - 1) - v, u),
        };
    };
    for vertebra in &mut marks.vertebrae {
        match &mut vertebra.marks {
            VertebraMarks::Front { points } => {
                for mark in [
                    &mut points.sup_l,
                    &mut points.sup_r,
                    &mut points.inf_l,
                    &mut points.inf_r,
                ] {
                    place(&mut mark.at);
                }
                for pedicle in [&mut points.ped_l, &mut points.ped_r] {
                    if let PedicleMark::Visible { at, .. } = pedicle {
                        place(at);
                    }
                }
            }
            VertebraMarks::Side { points } => {
                for mark in [
                    &mut points.sup_a,
                    &mut points.sup_p,
                    &mut points.inf_a,
                    &mut points.inf_p,
                ] {
                    place(&mut mark.at);
                }
            }
            VertebraMarks::Bending { .. } => unreachable!(),
        }
    }
    geometry.quarter_turns = QuarterTurns::new(turns).unwrap();
    let stored = if turns.is_multiple_of(2) {
        spacing
    } else {
        [spacing[1], spacing[0]]
    };
    geometry.mm_per_px = Some(stored[0]);
    geometry.mm_per_px_vertical = Some(stored[1]);
}

#[test]
fn side_scale_and_different_lowest_level_are_registered() {
    let (mut input, key) = input(RecipeName::Straight, 3, 0.0);
    input.side.vertebrae.retain(|v| v.level != Level::L5);
    input.side_geometry.mm_per_px = input.side_geometry.mm_per_px.map(|mm| mm / 1.03);
    let l4 = key
        .poses
        .iter()
        .find(|pose| pose.level == Level::L4)
        .unwrap();
    let true_offset = Endplates::of(&VertebraParams::from(l4)).bottom.centre.z;
    let fitted = fit_spine(&input).expect("the global scale and vertical registration fit");
    // The prior pulls the scale toward 1, and the absent L5 side view is
    // inferred. Allow 0.5% scale and 0.5 mm registration for that bias.
    assert!(
        (fitted.side_scale - 1.03).abs() < 0.005,
        "{}",
        fitted.side_scale
    );
    assert!(
        (fitted.side_offset_mm - true_offset).abs() < 0.5,
        "{} against {true_offset}",
        fitted.side_offset_mm
    );
}

#[test]
fn an_inferred_corner_moves_the_fit_less_than_a_placed_corner() {
    let (mut input, _) = input(RecipeName::RightThoracic40, 3, 0.0);
    let reference = fit_spine(&input).expect("the reference fit");
    let index = input
        .front
        .vertebrae
        .iter()
        .position(|v| v.level == Level::T8)
        .unwrap();
    let mm = input.front_geometry.mm_per_px.unwrap();
    let VertebraMarks::Front { points } = &mut input.front.vertebrae[index].marks else {
        unreachable!()
    };
    points.sup_l.at = Px::new(points.sup_l.at.x() + 8.0 / mm, points.sup_l.at.y());
    let placed = fit_spine(&input).expect("the misplaced corner fit");
    let VertebraMarks::Front { points } = &mut input.front.vertebrae[index].marks else {
        unreachable!()
    };
    points.sup_l.inferred = true;
    let inferred = fit_spine(&input).expect("the inferred corner fit");
    let centre = reference.vertebrae[index].pose.centre;
    let moved = distance(placed.vertebrae[index].pose.centre, centre);
    let inferred_move = distance(inferred.vertebrae[index].pose.centre, centre);
    assert!(
        inferred_move < moved,
        "inferred {inferred_move} mm, placed {moved} mm"
    );
}

#[test]
fn malformed_geometry_and_marks_are_refused() {
    let (input, _) = input(RecipeName::Straight, 3, 0.0);
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut bad = input.clone();
        bad.front_geometry.mm_per_px = Some(value);
        assert_eq!(
            fit_spine(&bad),
            Err(FitError::InvalidScale(FilmKind::Front))
        );
        let mut bad = input.clone();
        bad.side_geometry.mm_per_px_vertical = Some(value);
        assert_eq!(fit_spine(&bad), Err(FitError::InvalidScale(FilmKind::Side)));
    }
    let mut bad = input.clone();
    bad.front_size[0] = 0;
    assert_eq!(fit_spine(&bad), Err(FitError::InvalidSize(FilmKind::Front)));
    let mut bad = input.clone();
    bad.front_geometry.image_right = Some(ImageRight::Anterior);
    assert_eq!(
        fit_spine(&bad),
        Err(FitError::InvalidFacing(FilmKind::Front))
    );
    let mut bad = input.clone();
    bad.side.vertebrae[1].level = bad.side.vertebrae[0].level;
    assert_eq!(fit_spine(&bad), Err(FitError::InvalidMarks(FilmKind::Side)));
    let mut bad = input.clone();
    bad.front.vertebrae.swap(0, 1);
    assert_eq!(
        fit_spine(&bad),
        Err(FitError::InvalidMarks(FilmKind::Front))
    );
    let mut bad = input.clone();
    let VertebraMarks::Side { points } = &mut bad.side.vertebrae[0].marks else {
        unreachable!()
    };
    points.sup_a.at = Px::new(f64::NAN, 0.0);
    assert_eq!(fit_spine(&bad), Err(FitError::InvalidMarks(FilmKind::Side)));
    let mut bad = input;
    bad.front.vertebrae.clear();
    bad.side.vertebrae.clear();
    assert_eq!(fit_spine(&bad), Err(FitError::NoMarks));
}

#[test]
fn exact_marks_and_zero_priors_recover_the_hand_worked_spine() {
    let fit = fit_spine(&upright_input()).expect("the consistent analytic problem fits");
    assert!(fit.converged);
    // Every residual has a simultaneous zero. Allow 1e-7 mm and 1e-9
    // radians for arithmetic and central-difference rounding, much less
    // than the perfect-answer limits.
    for (vertebra, z) in fit.vertebrae.iter().zip([70.0, 40.0, 10.0]) {
        assert!(distance(vertebra.pose.centre, [0.0, 0.0, z]) < 1e-7);
        for angle in angles(&vertebra.pose) {
            assert!(angle.abs() < 1e-9);
        }
        assert!(vertebra.pose.wedge_front.abs() < 1e-9);
        assert!(vertebra.pose.wedge_side.abs() < 1e-9);
        for (got, expected) in sizes(&vertebra.pose)
            .into_iter()
            .zip([20.0, 15.0, 10.0, 15.0, 10.0])
        {
            assert!((got - expected).abs() < 1e-7);
        }
    }
    assert!(fit.rms_mm < 1e-7);
    assert!((fit.side_scale - 1.0).abs() < 1e-9);
    assert!(fit.side_offset_mm.abs() < 1e-7);
}

#[test]
fn a_single_complete_vertebra_still_has_a_fit() {
    let mut input = upright_input();
    input.front.vertebrae.retain(|v| v.level == Level::L5);
    input.side.vertebrae.retain(|v| v.level == Level::L5);
    let fit = fit_spine(&input).expect("a complete pair of views determines one vertebra");
    assert!(fit.converged);
    assert_eq!(fit.vertebrae.len(), 1);
    assert!(distance(fit.vertebrae[0].pose.centre, [0.0, 0.0, 10.0]) < 1e-7);
}

#[test]
fn two_levels_cannot_fill_a_missing_position_by_second_difference() {
    let mut input = upright_input();
    input.front.vertebrae.retain(|v| v.level >= Level::L4);
    input.side.vertebrae.retain(|v| v.level == Level::L5);
    // L4's front-to-back position is absent from every mark residual;
    // with only two levels there is no centre second-difference prior.
    assert_eq!(fit_spine(&input), Err(FitError::Underconstrained));
}
