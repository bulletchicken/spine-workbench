//! Apply patterns in a curve-relative frame and refresh retained look sources.

use crate::{
    LayoutError,
    patterns::{self, Pattern},
};
use shapes::implant_layout::AnchorProposal;
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

pub(super) fn apply(
    anchors: &mut [AnchorProposal],
    levels: &[Level],
    apex: Option<Level>,
    convex: Option<BodySide>,
    pattern: Pattern,
) -> Result<(), LayoutError> {
    // This project's canonical right-convex frame makes alternating
    // non-key sides reflect with the curve. Bilateral keys stay paired.
    let reflected = pattern == Pattern::KeyLevels && convex == Some(BodySide::Left);
    if reflected {
        swap(anchors);
    }
    patterns::apply(anchors, levels, apex, pattern)?;
    if reflected {
        swap(anchors);
    }
    for anchor in anchors {
        crate::screws::refresh_look_sources(anchor);
    }
    Ok(())
}

fn swap(anchors: &mut [AnchorProposal]) {
    for pair in anchors.as_chunks_mut::<2>().0 {
        pair.swap(0, 1);
        pair[0].side = BodySide::Left;
        pair[1].side = BodySide::Right;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screws::{refresh_look_sources, sizing};
    use shapes::implant_layout::{
        AnchorKind, CurveSide, DiameterRule, LookCode, PedicleEstimate, ValueStatus,
    };
    use shapes::spine_readings::Band;

    #[test]
    fn a_missing_secondary_size_keeps_its_null_without_adding_a_narrow_look() {
        let grid = sizing::diameter_grid();
        // W=4 mm admits 5 mm at 1.25, but 0.80 allows only 3.2 mm,
        // below the 3.5 mm grid floor. The inner channel is 2.5 mm.
        let chosen = sizing::choose_diameter(&[4.0; 10], &grid, 1.25, 0.1).unwrap();
        let secondary = sizing::choose_diameter(&[4.0; 10], &grid, 0.80, 0.1).unwrap();
        assert_eq!(chosen.mm, Some(5.0));
        assert_eq!(secondary.mm, None);
        let constant = |mm| Band {
            p10: mm,
            p50: mm,
            p90: mm,
        };
        let mut anchor = AnchorProposal {
            level: Level::T8,
            side: BodySide::Left,
            kind: AnchorKind::Screw,
            entry_mm: [0.0; 3],
            entry_status: ValueStatus::FromTemplate,
            direction_unit: [1.0, 0.0, 0.0],
            direction_status: ValueStatus::FromTables,
            medial_angle: None,
            medial_angles_beside: Vec::new(),
            convergence_deg: constant(0.0),
            diameter_mm: chosen.mm,
            diameter_also_seen: chosen.also_seen,
            diameter_status: ValueStatus::FromTables,
            width_ratio: Some(1.25),
            diameters_beside: vec![DiameterRule {
                width_ratio: 0.80,
                diameter_mm: secondary.mm,
                source: "misenhimer-1989".into(),
            }],
            screw_to_pedicle: Some(constant(1.25)),
            chord_mm: Some(40.0),
            chord_share: Some(0.8),
            length_mm: Some(30.0),
            length_also_seen: Vec::new(),
            length_status: ValueStatus::FromTables,
            usual_length_mm: Some([30.0, 35.0]),
            pedicle: PedicleEstimate {
                outer_width_mm: Some(constant(4.0)),
                inner_width_mm: Some(constant(2.5)),
                versions_too_narrow: 0,
                versions_inner_under_1_mm: 0,
                curve_side: CurveSide::Concave,
                status: ValueStatus::FromTables,
                sources: vec!["gao-2017".into()],
            },
            wider_than_pedicle: Vec::new(),
            screw_here: None,
            looks: vec![LookCode::ConcaveApexNeedsCt],
            diameter_sources: vec!["sato-2022".into()],
            length_sources: vec!["upendra-2010".into()],
            direction_sources: vec!["upendra-2010".into()],
            look_sources: Vec::new(),
            sources: vec!["misenhimer-1989".into()],
        };
        refresh_look_sources(&mut anchor);
        let mut anchors = vec![anchor.clone(), anchor];
        anchors[1].side = BodySide::Right;
        let original = anchors.clone();
        apply(
            &mut anchors,
            &[Level::T8],
            Some(Level::T8),
            Some(BodySide::Right),
            Pattern::AllPedicles,
        )
        .unwrap();
        assert_eq!(anchors, original);
        let value = serde_json::to_value(&anchors[0]).unwrap();
        assert_eq!(value["looks"], serde_json::json!(["concave-apex-needs-ct"]));
        assert_eq!(
            value["diametersBeside"],
            serde_json::json!([
                {"widthRatio": 0.80, "diameterMm": null, "source": "misenhimer-1989"}
            ])
        );
        assert_eq!(anchors[0].look_sources.len(), 1);
    }
}
