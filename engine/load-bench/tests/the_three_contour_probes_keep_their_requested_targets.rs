//! The current default keeps the explicit zero-extra contour semantics.
mod realistic_cases;

use implant_layout::rods;
use shapes::implant_layout::AnchorKind;
use shapes::made_up::RecipeName;

#[test]
fn the_current_default_matches_an_explicit_zero_extra_contour() {
    let case = realistic_cases::build(RecipeName::RightThoracic40, 1).unwrap();
    let mut occupied = case.anchors.clone();
    for anchor in &mut occupied {
        anchor.kind = AnchorKind::Screw;
    }
    let choice = rods::choices()
        .into_iter()
        .find(|c| {
            c.material == case.rod_proposals[0].material
                && c.diameter_mm == case.rod_proposals[0].diameter_mm
        })
        .unwrap();
    let explicit = rods::propose_with_overbend(
        &case.input.fit,
        &case.levels,
        &case.reading,
        &occupied,
        &choice,
        0.0,
    )
    .unwrap();
    assert_eq!(case.rod_proposals, explicit);
    for anchor in &case.construct.anchors {
        let pose = &case.target_poses[anchor.body];
        let head = pose.centre_mm + pose.rotation * anchor.point_mm;
        assert!(
            case.construct.rods[anchor.rod]
                .points_mm
                .iter()
                .any(|p| (*p - head).norm() < 1e-9)
        );
    }
}
