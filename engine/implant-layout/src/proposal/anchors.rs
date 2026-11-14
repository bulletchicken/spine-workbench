//! Apply patterns in a curve-relative frame and retain an absence code for
//! the secondary diameter rules as well as the selected primary rule.

use crate::{
    LayoutError,
    patterns::{self, Pattern},
};
use shapes::implant_layout::{AnchorProposal, LookCode};
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
        // Project inference: a secondary width rule giving no grid size
        // also needs a look, even when the primary rule gives one.
        if anchor
            .diameters_beside
            .iter()
            .any(|r| r.diameter_mm.is_none())
        {
            anchor.looks.push(LookCode::MayBeTooNarrow);
            anchor.looks.sort();
            anchor.looks.dedup();
        }
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
