//! Version identities and full JSON stay fixed when the work is repeated.

use implant_layout::{proposal, propose_layouts, readings, screws};
use made_up_spines::patient::make_up_patient;
use shapes::implant_layout::{ChildFacts, LayoutInput, NamingDoubt};
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::spine_fit::{
    FilmPlacement, FitAngles, FitLengths, FitRmsMm, FitStandardDeviations, FitStopReason,
    FittedVertebra, InferredFit, SpineFit,
};
use shapes::spine_readings::BodySide;

fn input() -> LayoutInput {
    let patient = make_up_patient(RecipeName::RightThoracic40, 1).unwrap();
    let vertebrae = patient
        .standing
        .vertebrae
        .iter()
        .zip(&patient.standing.levels)
        .filter(|(_, l)| **l >= Level::T1)
        .map(|(v, l)| {
            let r = v.rotation_matrix();
            FittedVertebra {
                level: *l,
                pose: v.pose(*l),
                angles: FitAngles {
                    side_tilt_rad: libm::atan2(-r[(1, 2)], r[(2, 2)]),
                    forward_lean_rad: libm::asin(r[(0, 2)].clamp(-1.0, 1.0)),
                    twist_rad: libm::atan2(-r[(0, 1)], r[(0, 0)]),
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
            origin_level: Level::L5,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: FitLengths::Measured,
            typical_curve_deg: None,
            front_film: FilmPlacement {
                mm_per_px: [1.0; 2],
                origin_px: [0.0; 2],
                facing_assumed: false,
            },
            side_film: Some(FilmPlacement {
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
        naming: NamingDoubt::MayBeOneOff,
        versions: 32,
        seed: 19,
        pedicle_child_share: None,
    }
}
#[test]
fn identical_input_gives_byte_identical_complete_json() {
    let input = input();
    let first = serde_json::to_vec(&propose_layouts(&input).unwrap()).unwrap();
    let second = serde_json::to_vec(&propose_layouts(&input).unwrap()).unwrap();
    assert_eq!(first, second);
}
#[test]
fn split_versions_produce_identical_pedicle_proposals() {
    let input = input();
    let whole = readings::readings(&input, 0..32).unwrap();
    let first = readings::readings(&input, 0..5).unwrap();
    let middle = readings::readings(&input, 5..29).unwrap();
    let last = readings::readings(&input, 29..32).unwrap();
    let versions = [
        first.versions.versions,
        middle.versions.versions,
        last.versions.versions,
    ]
    .concat();
    let measured = [first.measured, middle.measured, last.measured].concat();
    assert_eq!(whole.versions.versions, versions);
    assert_eq!(whole.measured, measured);
    for side in [BodySide::Left, BodySide::Right] {
        let a = screws::anchor(
            &input,
            &whole.versions.versions,
            &whole.measured,
            Level::T8,
            side,
        )
        .unwrap();
        let b = screws::anchor(&input, &versions, &measured, Level::T8, side).unwrap();
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
    }
}

#[test]
fn split_versions_produce_byte_identical_complete_proposals() {
    let input = input();
    let whole = proposal::measure(&input).unwrap();
    let mut split = whole.clone();
    split.versions.versions = [
        readings::versions(&input.fit, u64::from(input.seed), 32, 0..5)
            .unwrap()
            .versions,
        readings::versions(&input.fit, u64::from(input.seed), 32, 5..29)
            .unwrap()
            .versions,
        readings::versions(&input.fit, u64::from(input.seed), 32, 29..32)
            .unwrap()
            .versions,
    ]
    .concat();
    assert_eq!(whole.versions, split.versions);
    let first = proposal::assemble(&input, &whole).unwrap();
    let batched = proposal::assemble(&input, &split).unwrap();
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&batched).unwrap()
    );
}
