//! Removing one boundary keeps the upper id and the two outer endplates.

use super::geometry::*;
use shapes::errors::{EngineError, ErrorCode};
use shapes::marks::PedicleMark;
use shapes::toolbox::ToolWarning;
use shapes::vertebra::{FilmMarks, VertebraMarks};

pub(super) fn apply(
    marks: &mut FilmMarks,
    index: usize,
    warnings: &mut Vec<ToolWarning>,
) -> Result<u32, EngineError> {
    let upper = &marks.vertebrae[index];
    let lower = &marks.vertebrae[index + 1];
    let a = upper.marks.corners();
    let b = lower.marks.corners();
    let corners = [a[0], a[1], b[2], b[3]];
    if !valid(corners.map(|m| m.at)) {
        return Err(EngineError::new(
            ErrorCode::ToolMergeInvalid,
            "These outlines cross; fix them with pins first",
        )
        .with_detail("merge-invalid"));
    }
    let removed = lower.id;
    let pins = [a[2], a[3], b[0], b[1]].iter().filter(|m| m.pinned).count();
    if pins > 0 {
        warnings.push(ToolWarning::PinsRemoved { count: pins as u32 });
    }
    let upper = &mut marks.vertebrae[index];
    put_corners(&mut upper.marks, corners);
    if let VertebraMarks::Front { points } = &mut upper.marks {
        for pedicle in [&mut points.ped_l, &mut points.ped_r] {
            if let PedicleMark::Visible {
                pinned: false,
                inferred,
                ..
            } = pedicle
            {
                *inferred = true;
            }
        }
    }
    upper.outline = None;
    marks.vertebrae.remove(index + 1);
    Ok(removed)
}
