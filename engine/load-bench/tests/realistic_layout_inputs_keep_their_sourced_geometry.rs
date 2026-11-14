//! The six mechanics cases use actual layout geometry and retain its gaps.
mod realistic_cases;

use load_bench::evidence::{Evidence, TableKind, Unit};
use load_bench::rigid_rod::HeadKind;
use nalgebra::Vector3;
use shapes::implant_layout::AnchorKind;
use shapes::made_up::RecipeName;

fn check(recipe: RecipeName, seed: u32) {
    let case = realistic_cases::build(recipe, seed).expect("sourced geometric construct");
    assert_eq!(case.construct.anchors.len(), 2 * case.levels.len());
    assert_eq!(case.construct.rods.len(), 2);
    assert!(case.rod_proposals.iter().all(|r| r.overbend_deg == 0.0));
    let first = case.construct.anchors.iter().map(|a| a.body).min().unwrap();
    let last = case.construct.anchors.iter().map(|a| a.body).max().unwrap();
    let target = case.chain.evaluate(&case.target_poses).unwrap();
    for i in first..last {
        assert!(
            target.deformation[i].fixed_rows::<3>(0).norm() <= 1e-9,
            "{recipe:?}/{seed}: target joint {:?}/{:?} translates {:?}",
            case.chain.reference[i].level,
            case.chain.reference[i + 1].level,
            target.deformation[i].fixed_rows::<3>(0)
        );
    }
    assert!((case.target_poses[last].centre_mm - case.guess.poses[last].centre_mm).norm() < 1e-12);
    assert!((case.target_poses[last].rotation - case.guess.poses[last].rotation).norm() < 1e-12);
    let evidence = Evidence::read().unwrap();
    let modulus = evidence
        .value(TableKind::RodMaterials, "cobalt-chrome", "young-modulus")
        .unwrap();
    assert_eq!(modulus.unit, Unit::Gpa);
    for (rod_index, (rod, proposal)) in case
        .construct
        .rods
        .iter()
        .zip(&case.rod_proposals)
        .enumerate()
    {
        assert_eq!(rod.properties.young_modulus_mpa, modulus.value * 1000.0);
        assert_eq!(rod.properties.diameter_mm, proposal.diameter_mm);
        assert_eq!(rod.points_mm.len(), proposal.points_mm.len());
        for (p, q) in rod.points_mm.iter().zip(&proposal.points_mm) {
            assert_eq!(*p, Vector3::from(*q));
        }
        let rod_anchors: Vec<_> = case
            .construct
            .anchors
            .iter()
            .filter(|a| a.rod == rod_index)
            .collect();
        for anchor in &rod_anchors {
            assert_eq!(anchor.kind, HeadKind::Screw);
            let standing = &case.guess.poses[anchor.body];
            let source = case
                .anchors
                .iter()
                .find(|a| {
                    a.level == case.chain.reference[anchor.body].level && a.side == proposal.side
                })
                .unwrap();
            let supplied_head = Vector3::from(source.entry_mm)
                - Vector3::from(source.direction_unit) * proposal.head_offset_mm;
            let assembled_head = standing.centre_mm + standing.rotation * anchor.point_mm;
            assert!((assembled_head - supplied_head).norm() < 1e-12);
            if source.kind == AnchorKind::None {
                assert!(source.diameter_mm.is_none() || source.length_mm.is_none());
            }
        }
        let lowest = rod_anchors.iter().max_by_key(|a| a.body).unwrap();
        let stations = load_bench::rod_beams::polyline_stations(&rod.points_mm).unwrap();
        assert_eq!(lowest.station_mm, *stations.last().unwrap());
    }
    // The second start changes only initial poses/free stations; supplied
    // contours, locked local head placements and physical pin stations stay.
    let target = realistic_cases::layout_target_guess(&case).unwrap();
    for (i, anchor) in case.construct.anchors.iter().enumerate() {
        if case
            .construct
            .anchors
            .iter()
            .filter(|a| a.rod == anchor.rod)
            .all(|a| a.body <= anchor.body)
        {
            assert_eq!(target.stations_mm[i], anchor.station_mm);
        }
    }
}

macro_rules! case {
    ($name:ident, $recipe:expr, $seed:expr) => {
        #[test]
        fn $name() {
            check($recipe, $seed);
        }
    };
}
case!(right_thoracic_seed_one, RecipeName::RightThoracic40, 1);
case!(right_thoracic_seed_two, RecipeName::RightThoracic40, 2);
case!(double_major_seed_one, RecipeName::DoubleMajor55To45, 1);
case!(double_major_seed_two, RecipeName::DoubleMajor55To45, 2);
case!(lumbar_left_seed_one, RecipeName::LumbarLeft30, 1);
case!(lumbar_left_seed_two, RecipeName::LumbarLeft30, 2);
