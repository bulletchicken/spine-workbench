//! Anchor patterns on plain instrumented levels. Pattern selection never
//! chooses the upper or lower instrumented vertebra. Missing screw sizes
//! substitute a hook at the upper end and leave other pedicles empty.

use crate::LayoutError;
use crate::evidence::Table;
use crate::pedicles::{cell, central};
use shapes::implant_layout::{AnchorKind, AnchorProposal, ValueStatus};
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    AllPedicles,
    KeyLevels,
    HooksAtTop,
}

/// Apply the pattern and the substitution rule, retaining every pedicle
/// in top-to-bottom, left-before-right order. Empty and hook anchors have
/// no screw sizes. Existing look codes remain even at empty pedicles.
///
/// This project's key-level inference honours the density cap when a
/// short span cannot contain every bilateral key level: endpoints first,
/// apex next, then their neighbours, followed by alternating sides. A
/// bilateral key pair is never split to fill an odd slot in the cap.
/// This priority is not a paper rule.
pub fn apply(
    anchors: &mut [AnchorProposal],
    levels: &[Level],
    apex: Option<Level>,
    pattern: Pattern,
) -> Result<Vec<String>, LayoutError> {
    if levels.is_empty()
        || levels.windows(2).any(|w| w[0] >= w[1])
        || anchors.len() != 2 * levels.len()
        || anchors.iter().enumerate().any(|(i, a)| {
            a.level != levels[i / 2]
                || a.side
                    != if i % 2 == 0 {
                        BodySide::Left
                    } else {
                        BodySide::Right
                    }
        })
    {
        return Err(LayoutError::InvalidValue {
            field: "pattern".into(),
            reason: "bilateral-order-required".into(),
        });
    }
    let mut sources = Vec::new();
    let mut selected = vec![true; anchors.len()];
    if pattern == Pattern::KeyLevels {
        let c = cell(Table::AnchorPatterns, "density", "mimo-low-max")
            .ok_or_else(|| LayoutError::Refused("missing-density-cap".into()))?;
        let density =
            central(c).ok_or_else(|| LayoutError::Refused("density-cap-not-scalar".into()))?;
        sources.push(c.source.clone());
        let cap = (libm::floor(levels.len() as f64 * density) as usize).min(anchors.len());
        let mut priority = Vec::new();
        let mut pair = |i: usize| {
            for side in 0..2 {
                let index = 2 * i + side;
                if !priority.contains(&index) {
                    priority.push(index);
                }
            }
        };
        pair(0);
        pair(levels.len() - 1);
        let apex = apex.and_then(|level| levels.iter().position(|&l| l == level));
        if let Some(i) = apex {
            pair(i);
        }
        if levels.len() > 1 {
            pair(1);
            pair(levels.len() - 2);
        }
        if let Some(i) = apex {
            if i > 0 {
                pair(i - 1);
            }
            if i + 1 < levels.len() {
                pair(i + 1);
            }
        }
        for i in 0..levels.len() {
            let index = 2 * i + i % 2;
            if !priority.contains(&index) {
                priority.push(index);
            }
        }
        selected.fill(false);
        for &i in priority.iter().take(cap - cap % 2) {
            selected[i] = true;
        }
    }
    if pattern == Pattern::HooksAtTop {
        for key in ["hook-pjk-odds", "junction-index-reduction"] {
            if let Some(c) = cell(Table::AnchorPatterns, key, "hooks-at-uiv") {
                sources.push(c.source.clone());
            }
        }
    }
    for (i, anchor) in anchors.iter_mut().enumerate() {
        let top = anchor.level == levels[0];
        anchor.kind = if !selected[i] {
            AnchorKind::None
        } else if top && pattern == Pattern::HooksAtTop {
            AnchorKind::Hook
        } else if anchor.diameter_mm.is_none()
            || anchor.length_mm.is_none()
            || anchor.direction_status == ValueStatus::Missing
        {
            if top {
                AnchorKind::Hook
            } else {
                AnchorKind::None
            }
        } else {
            AnchorKind::Screw
        };
        if anchor.kind != AnchorKind::Screw {
            anchor.diameter_mm = None;
            anchor.length_mm = None;
            anchor.medial_angle = None;
            anchor.diameter_also_seen.clear();
            anchor.length_also_seen.clear();
            anchor.screw_to_pedicle = None;
            anchor.diameter_status = ValueStatus::Missing;
            anchor.length_status = ValueStatus::Missing;
        }
        anchor.sources.extend(sources.iter().cloned());
        anchor.sources.sort();
        anchor.sources.dedup();
    }
    Ok(sources)
}
