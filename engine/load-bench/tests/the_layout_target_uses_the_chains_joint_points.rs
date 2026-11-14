//! Cross-crate regression: layout rotations move the bench's material joints.

mod realistic_cases;

use implant_layout::rods;
use load_bench::chain::Pose;
use shapes::made_up::RecipeName;

/// Geometric round-off allowance in patient millimetres.
const JOINT_POINT_TOLERANCE_MM: f64 = 1e-9;

fn check(recipe: RecipeName, seed: u32) {
    let case = realistic_cases::build(recipe, seed).expect("actual main-span layout inputs");
    let target = rods::target_spine(&case.input.fit, &case.levels, &case.reading)
        .expect("joint-only layout target");
    assert_eq!(
        target
            .poses
            .iter()
            .map(|pose| pose.level)
            .collect::<Vec<_>>(),
        case.levels,
        "the direct layout target returns the actual instrumented span"
    );
    let standing = case.chain.standing();
    let mut poses = standing.clone();
    for vertebra in &target.poses {
        let body = case
            .chain
            .reference
            .iter()
            .position(|reference| reference.level == vertebra.level)
            .expect("layout level exists in the same plain chain");
        poses[body] = Pose::from_vertebra(vertebra);
    }
    let first = case
        .chain
        .reference
        .iter()
        .position(|reference| reference.level == case.levels[0])
        .unwrap();
    let last = case
        .chain
        .reference
        .iter()
        .position(|reference| reference.level == *case.levels.last().unwrap())
        .unwrap();
    assert!(last > first, "the actual span contains an internal joint");
    assert!(
        (poses[last].centre_mm - standing[last].centre_mm).norm() <= JOINT_POINT_TOLERANCE_MM,
        "{recipe:?}/{seed}: lowest target centre moved"
    );
    assert!(
        (poses[last].rotation - standing[last].rotation).norm() <= 1e-12,
        "{recipe:?}/{seed}: lowest target orientation moved"
    );
    let evaluated = case.chain.evaluate(&poses).expect("actual target poses");
    // Only joints whose two vertebrae lie inside the instrumented span:
    // the uninstrumented profile is not part of layout's returned target.
    for joint in first..last {
        let translation = evaluated.deformation[joint].fixed_rows::<3>(0);
        assert!(
            translation.norm() <= JOINT_POINT_TOLERANCE_MM,
            "{recipe:?}/{seed}, {:?}/{:?}: material-joint translation {:?} mm",
            case.chain.reference[joint].level,
            case.chain.reference[joint + 1].level,
            translation.as_slice()
        );
    }
}

#[test]
fn right_thoracic_40_seed_1_uses_the_same_material_joint_points() {
    check(RecipeName::RightThoracic40, 1);
}

#[test]
fn right_thoracic_40_seed_2_uses_the_same_material_joint_points() {
    check(RecipeName::RightThoracic40, 2);
}

#[test]
fn double_major_55_to_45_seed_1_uses_the_same_material_joint_points() {
    check(RecipeName::DoubleMajor55To45, 1);
}

#[test]
fn double_major_55_to_45_seed_2_uses_the_same_material_joint_points() {
    check(RecipeName::DoubleMajor55To45, 2);
}

#[test]
fn lumbar_left_30_seed_1_uses_the_same_material_joint_points() {
    check(RecipeName::LumbarLeft30, 1);
}

#[test]
fn lumbar_left_30_seed_2_uses_the_same_material_joint_points() {
    check(RecipeName::LumbarLeft30, 2);
}
