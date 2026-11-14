//! Reflection changes curve handedness and exchanges the physical sides.
//! Catalogue selections must follow the corresponding pedicle.

use implant_layout::readings::readings;
use implant_layout::screws::anchor;
use shapes::implant_layout::{ChildFacts, LayoutInput, NamingDoubt};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use shapes::spine_readings::BodySide;

fn input(reflected: bool, seed: u32) -> LayoutInput {
    let sign = if reflected { -1.0 } else { 1.0 };
    let vertebrae = [Level::T7, Level::T8, Level::T9, Level::T10, Level::T11]
        .into_iter()
        .enumerate()
        .map(|(i, level)| {
            let u = i as f64 - 2.0;
            let tilt = sign * u * 10.0_f64.to_radians();
            FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [
                        0.0,
                        -sign * 20.0 * (1.0 - u * u / 4.0),
                        300.0 - 25.0 * i as f64,
                    ],
                    rotation: [tilt, 0.0, 0.0],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    top_half_depth: 12.0,
                    bottom_half_width: 15.0,
                    bottom_half_depth: 12.0,
                },
                angles: FitAngles {
                    side_tilt_rad: tilt,
                    forward_lean_rad: 0.0,
                    twist_rad: 0.0,
                },
                sd: FitStandardDeviations {
                    centre_mm: [0.0; 3],
                    angles_rad: [0.0; 3],
                },
                inferred: InferredFit {
                    depth: false,
                    width: false,
                    twist: false,
                },
                rms_mm: FitRmsMm {
                    front: Some(0.0),
                    side: Some(0.0),
                },
                corner_tilt_rad: None,
            }
        })
        .collect();
    LayoutInput {
        fit: SpineFit {
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            origin_level: Level::T11,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
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
        front: None,
        bend_left: None,
        bend_right: None,
        child: ChildFacts {
            age_years: Some(14.0),
            weight_kg: None,
            activity: None,
        },
        naming: NamingDoubt::AsRead,
        versions: 32,
        seed,
        pedicle_child_share: None,
    }
}

#[test]
fn every_catalogue_size_follows_its_pedicle_under_reflection() {
    // Explicit user requirement: mirror the spine, exchange L/R sizes.
    // No fit noise is supplied, so the nominal and version curves mirror
    // exactly. At the T9 apex, Gao outer widths and Upendra chords depend
    // on curve side only; the physical-side wall thickness does not enter
    // either catalogue selection. Aorta-code asymmetry is not compared.
    for seed in 0..16 {
        let original = input(false, seed);
        let mirrored = input(true, seed);
        let a = readings(&original, 0..32).unwrap();
        let b = readings(&mirrored, 0..32).unwrap();
        assert_eq!(a.nominal.curves[0].apex_index, 2);
        assert_eq!(b.nominal.curves[0].apex_index, 2);
        assert_eq!(a.nominal.curves[0].convex, BodySide::Right);
        assert_eq!(b.nominal.curves[0].convex, BodySide::Left);
        for (first, second) in [
            (BodySide::Left, BodySide::Right),
            (BodySide::Right, BodySide::Left),
        ] {
            let left = anchor(
                &original,
                &a.versions.versions,
                &a.measured,
                Level::T9,
                first,
            )
            .unwrap();
            let right = anchor(
                &mirrored,
                &b.versions.versions,
                &b.measured,
                Level::T9,
                second,
            )
            .unwrap();
            assert_eq!(left.pedicle.curve_side, right.pedicle.curve_side);
            assert_eq!(left.pedicle.outer_width_mm, right.pedicle.outer_width_mm);
            assert_eq!(left.screw_to_pedicle, right.screw_to_pedicle);
            assert_eq!(left.convergence_deg, right.convergence_deg);
            assert_eq!(
                (
                    left.diameter_mm,
                    left.length_mm,
                    left.diameter_also_seen,
                    left.length_also_seen
                ),
                (
                    right.diameter_mm,
                    right.length_mm,
                    right.diameter_also_seen,
                    right.length_also_seen
                ),
                "seed {seed}, original {first:?} versus reflected {second:?}"
            );
        }
    }
}
