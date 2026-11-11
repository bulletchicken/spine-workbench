//! Resolve a typed pin target without creating a mark or changing its name.

use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::landmarks::{HipSide, LandmarkName, SacrumTop};
use shapes::marks::{EndplateLine, LineEnd, Mark, MarkName, MarkSource, PedicleMark};
use shapes::toolbox::{PinAction, PinTarget};
use shapes::vertebra::{FilmMarks, VertebraMarks};

pub(super) struct Target {
    pub vertebra: Option<usize>,
    pub corner: Option<(usize, usize)>,
}

fn invalid() -> EngineError {
    EngineError::new(
        ErrorCode::ToolPinInvalid,
        "this pin target is not present on the film",
    )
}

fn change(mark: &mut Mark, action: PinAction) {
    match action {
        PinAction::Move { to, .. } => {
            mark.at = to;
            mark.pinned = true;
            mark.source = MarkSource::Pin;
            mark.inferred = false;
        }
        PinAction::Unpin { .. } => mark.pinned = false,
    }
}

fn change_pedicle(pedicle: &mut PedicleMark, action: PinAction) -> Result<(), EngineError> {
    let PedicleMark::Visible {
        at,
        source,
        pinned,
        inferred,
        ring_score,
    } = pedicle
    else {
        return Err(invalid());
    };
    match action {
        PinAction::Move { to, .. } => {
            *at = to;
            *source = MarkSource::Pin;
            *pinned = true;
            *inferred = false;
            *ring_score = None;
        }
        PinAction::Unpin { .. } => *pinned = false,
    }
    Ok(())
}

pub(super) fn apply(marks: &mut FilmMarks, action: PinAction) -> Result<Target, EngineError> {
    let target = match action {
        PinAction::Move { target, .. } | PinAction::Unpin { target } => target,
    };
    let id = match target {
        PinTarget::Vertebra { vertebra, .. } | PinTarget::Endplate { vertebra, .. } => vertebra,
        PinTarget::Landmark { landmark } => {
            change_landmark(marks, landmark, action)?;
            return Ok(Target {
                vertebra: None,
                corner: None,
            });
        }
    };
    let index = marks
        .vertebrae
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| {
            EngineError::new(
                ErrorCode::ToolUnknownVertebra,
                "the pin names an absent vertebra",
            )
        })?;
    let body = &mut marks.vertebrae[index];
    let corner = match (&mut body.marks, target) {
        (VertebraMarks::Front { points }, PinTarget::Vertebra { mark, .. }) => {
            let (point, corner) = match mark {
                MarkName::SupL => (&mut points.sup_l, 0),
                MarkName::SupR => (&mut points.sup_r, 1),
                MarkName::InfR => (&mut points.inf_r, 2),
                MarkName::InfL => (&mut points.inf_l, 3),
                MarkName::PedL | MarkName::PedR => {
                    change_pedicle(
                        if mark == MarkName::PedL {
                            &mut points.ped_l
                        } else {
                            &mut points.ped_r
                        },
                        action,
                    )?;
                    return Ok(Target {
                        vertebra: Some(index),
                        corner: None,
                    });
                }
                _ => return Err(invalid()),
            };
            change(point, action);
            corner
        }
        (VertebraMarks::Side { points }, PinTarget::Vertebra { mark, .. }) => {
            let (point, corner) = match mark {
                MarkName::SupA => (&mut points.sup_a, 0),
                MarkName::SupP => (&mut points.sup_p, 1),
                MarkName::InfP => (&mut points.inf_p, 2),
                MarkName::InfA => (&mut points.inf_a, 3),
                _ => return Err(invalid()),
            };
            change(point, action);
            corner
        }
        (VertebraMarks::Bending { lines }, PinTarget::Endplate { line, end, .. }) => {
            let side = usize::from(end == LineEnd::PatientRight);
            let (point, corner) = match line {
                EndplateLine::Sup => (&mut lines.sup[side], side),
                EndplateLine::Inf => (&mut lines.inf[side], 3 - side),
            };
            change(point, action);
            corner
        }
        _ => return Err(invalid()),
    };
    Ok(Target {
        vertebra: Some(index),
        corner: Some((index, corner)),
    })
}

fn change_landmark(
    marks: &mut FilmMarks,
    name: LandmarkName,
    action: PinAction,
) -> Result<(), EngineError> {
    let suits = match name {
        LandmarkName::SacL | LandmarkName::SacR => marks.film != FilmKind::Side,
        LandmarkName::SacA
        | LandmarkName::SacP
        | LandmarkName::HipFirst
        | LandmarkName::HipSecond => marks.film == FilmKind::Side,
        LandmarkName::HipLeft | LandmarkName::HipRight => marks.film == FilmKind::Front,
    };
    if !suits {
        return Err(invalid());
    }
    let point = match name {
        LandmarkName::SacL | LandmarkName::SacR => {
            let Some(SacrumTop::Front { sac_l, sac_r }) = &mut marks.landmarks.sacrum_top else {
                return Err(invalid());
            };
            if name == LandmarkName::SacL {
                sac_l
            } else {
                sac_r
            }
        }
        LandmarkName::SacA | LandmarkName::SacP => {
            let Some(SacrumTop::Side { sac_a, sac_p }) = &mut marks.landmarks.sacrum_top else {
                return Err(invalid());
            };
            if name == LandmarkName::SacA {
                sac_a
            } else {
                sac_p
            }
        }
        LandmarkName::HipLeft
        | LandmarkName::HipRight
        | LandmarkName::HipFirst
        | LandmarkName::HipSecond => {
            let hip = match name {
                LandmarkName::HipLeft => marks
                    .landmarks
                    .hips
                    .iter_mut()
                    .find(|hip| hip.side == HipSide::Left),
                LandmarkName::HipRight => marks
                    .landmarks
                    .hips
                    .iter_mut()
                    .find(|hip| hip.side == HipSide::Right),
                LandmarkName::HipFirst => marks
                    .landmarks
                    .hips
                    .iter_mut()
                    .find(|hip| matches!(hip.side, HipSide::Unpaired | HipSide::Merged)),
                LandmarkName::HipSecond => marks
                    .landmarks
                    .hips
                    .iter_mut()
                    .filter(|hip| hip.side == HipSide::Unpaired)
                    .nth(1),
                _ => unreachable!(),
            }
            .ok_or_else(invalid)?;
            if matches!(action, PinAction::Move { .. }) {
                hip.radius_px = None;
            }
            &mut hip.centre
        }
    };
    change(point, action);
    Ok(())
}
