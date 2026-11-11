//! Settle two completed unconstrained surveys with one shared count.
//! The original count alternatives remain available when the films differ.

use super::{
    survey::{self, Anchors, FilmSearch},
    two_views::{self, JointSolution, ViewGeometry},
};
use crate::{
    confidence::per_vertebra,
    handoffs::{SettledChain, Strip},
};
use shapes::{
    auto_pass::{ChainOptions, PassWarning, PassWarningCode, Stage, TwoFilmCheck},
    errors::{EngineError, ErrorCode},
    film::FilmKind,
    naming::NamingFlag,
    watch_frames::WatchFrame,
};

/// Geometry must describe this survey's lattice. When either scale is
/// assumed, both z arrays must be original film pixel heights, as required
/// by `ViewGeometry`; assumed millimetres must never be labelled pixels.
pub struct SurveyView<'a> {
    pub strip: &'a Strip,
    pub survey: FilmSearch,
    pub geometry: ViewGeometry,
}

pub struct PairedSearch {
    pub front: FilmSearch,
    pub side: FilmSearch,
    pub independent_front: SettledChain,
    pub independent_side: SettledChain,
    pub check: TwoFilmCheck,
    /// The complete chronological stream: original survey sweeps, joint
    /// checkpoints, then both final chain locks and settlements.
    pub frames: Vec<WatchFrame>,
}

pub fn settle(
    front: SurveyView<'_>,
    side: SurveyView<'_>,
    options: &ChainOptions,
) -> Result<PairedSearch, EngineError> {
    if front.strip.film != FilmKind::Front || side.strip.film != FilmKind::Side {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "paired surveys must be front and side",
        ));
    }
    let front_strip = front.strip;
    let side_strip = side.strip;
    let f = front.survey;
    let s = side.survey;
    let independent_front = f.settled.clone();
    let independent_side = s.settled.clone();
    let prefix = |survey: &FilmSearch| {
        survey
            .frames
            .iter()
            .take_while(|frame| !matches!(frame, WatchFrame::ChainLocked { .. }))
            .cloned()
            .collect::<Vec<_>>()
    };
    let mut frames = prefix(&f);
    frames.extend(prefix(&s));
    let counts_disagree = f.naming.template != s.naming.template;
    let mut counts = vec![options.template.unwrap_or(f.naming.template)];
    if options.template.is_none() && counts_disagree {
        counts.push(s.naming.template);
    }
    let mut best: Option<(shapes::naming::Template, JointSolution)> = None;
    for template in counts {
        let fp = survey::problem(&f.evidence, template, options.population, Anchors::Both);
        let sp = survey::problem(&s.evidence, template, options.population, Anchors::Both);
        let joint = two_views::solve_pair(
            &fp,
            &sp,
            &front.geometry,
            &side.geometry,
            &template.levels(),
            options.acquisition,
        )?;
        if best
            .as_ref()
            .is_none_or(|(_, previous)| joint.upper < previous.upper)
        {
            best = Some((template, joint));
        }
    }
    let (template, joint) = best.expect("at least one count");
    let mut f_selection = f.selection;
    let mut s_selection = s.selection;
    f_selection.selected = template;
    s_selection.selected = template;
    let mut front = survey::finish(
        front.strip,
        f.evidence,
        joint.front_conditioned,
        joint.front,
        f_selection,
        options,
    )?;
    let mut side = survey::finish(
        side.strip,
        s.evidence,
        joint.side_conditioned,
        joint.side,
        s_selection,
        options,
    )?;
    for result in [&mut front, &mut side] {
        if counts_disagree {
            result.naming.flags.push(NamingFlag::FilmsDisagreeOnCount);
        }
        if joint.check.disagree {
            result.settled.boundaries.warnings.push(PassWarning {
                code: PassWarningCode::FilmsDisagree,
                film: result.settled.boundaries.film,
                level: None,
                region: None,
            });
        }
    }
    // Refresh caps and naming frames after adding the joint flags.
    refresh(&mut front, front_strip);
    refresh(&mut side, side_strip);
    let front_lock = front
        .frames
        .iter()
        .position(|frame| matches!(frame, WatchFrame::ChainLocked { .. }))
        .expect("a finished survey locks");
    let side_lock = side
        .frames
        .iter()
        .position(|frame| matches!(frame, WatchFrame::ChainLocked { .. }))
        .expect("a finished survey locks");
    frames.extend(joint.frames);
    frames.extend_from_slice(&front.frames[front_lock..]);
    frames.extend_from_slice(&side.frames[side_lock..]);
    Ok(PairedSearch {
        front,
        side,
        independent_front,
        independent_side,
        check: joint.check,
        frames,
    })
}

fn refresh(search: &mut FilmSearch, strip: &Strip) {
    search.settled = per_vertebra::settle(search.settled.boundaries.clone(), strip, &search.naming);
    for frame in &mut search.frames {
        match frame {
            WatchFrame::LevelsNamed { naming, .. } => *naming = search.naming.clone(),
            WatchFrame::Confidence { .. } => *frame = per_vertebra::frame(&search.settled),
            _ => {}
        }
    }
    if let Some(warning) = search
        .settled
        .boundaries
        .warnings
        .iter()
        .find(|w| w.code == PassWarningCode::FilmsDisagree)
    {
        search.frames.push(WatchFrame::Warning {
            film: strip.film,
            stage: Stage::Chain,
            warning: warning.clone(),
        });
    }
}
