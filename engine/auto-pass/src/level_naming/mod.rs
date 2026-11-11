//! Stage 5, naming from both ends: which vertebra count fits best, and the
//! names counted from the first rib and from the sacrum. A click on a film
//! never reaches this stage.

pub mod count_from_bottom;
pub mod count_from_top;
pub mod disagreement;

use crate::handoffs::ChainBoundaries;
use shapes::auto_pass::Stage;
use shapes::errors::{EngineError, ErrorCode};
use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template, TemplateChance};
use shapes::watch_frames::WatchFrame;

/// Experimental selection margin, in energy units.
pub const COUNT_MARGIN: f64 = 3.0;

/// An unconstrained template solve and its prior-adjusted score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemplateScore {
    pub template: Template,
    pub energy: f64,
    pub score: f64,
    /// Uncalibrated: counts have different numbers of evidence terms.
    pub chance: f64,
}

/// A survey's counts, sorted best first, and its conservative choice.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateSelection {
    pub selected: Template,
    pub scores: Vec<TemplateScore>,
}

/// Strength of the two independent naming anchors in robust z-score units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorEvidence {
    pub first_rib: f64,
    /// A competing onset within one body of the solved first rib.
    pub competing_first_rib: Option<f64>,
    pub sacrum: f64,
    pub ridge_present: bool,
}

/// Experimental count priors. The 8.2% unusual-count rate in Carrino et
/// al. (PMID 21436097) motivates the total; the split is experimental.
pub const fn template_prior(template: Template) -> f64 {
    match template {
        Template::Standard => 0.90,
        Template::ExtraThoracic | Template::MissingThoracic => 0.02,
        Template::ExtraLumbar => 0.035,
        Template::MissingLumbar => 0.025,
    }
}

/// Compare all five unconstrained solves. No film edit is accepted here:
/// callers cache this survey before applying pin or gap constraints.
/// Positive infinity denotes an infeasible template and receives no mass.
pub fn rank_templates(
    energies: &[(Template, f64)],
    temperature: f64,
) -> Result<TemplateSelection, EngineError> {
    if !temperature.is_finite()
        || temperature <= 0.0
        || energies.len() != Template::ALL.len()
        || Template::ALL
            .iter()
            .any(|template| energies.iter().filter(|(t, _)| t == template).count() != 1)
        || energies
            .iter()
            .any(|(_, energy)| energy.is_nan() || *energy == f64::NEG_INFINITY)
    {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "template ranking needs five distinct counts and a positive temperature",
        ));
    }
    let mut scores: Vec<_> = energies
        .iter()
        .map(|&(template, energy)| TemplateScore {
            template,
            energy,
            score: energy - temperature * libm::log(template_prior(template)),
            chance: 0.0,
        })
        .collect();
    scores.sort_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then(a.template.cmp(&b.template))
    });
    let best = scores[0];
    if !best.score.is_finite() {
        return Err(EngineError::new(
            ErrorCode::PassNoSpineFound,
            "no vertebra count has a finite chain",
        ));
    }
    let standard = scores
        .iter()
        .find(|score| score.template == Template::Standard)
        .expect("the five counts were checked")
        .score;
    let selected = if best.score < standard - COUNT_MARGIN {
        best.template
    } else {
        Template::Standard
    };
    let mass: f64 = scores
        .iter_mut()
        .map(|score| {
            score.chance = libm::exp((best.score - score.score) / temperature);
            score.chance
        })
        .sum();
    for score in &mut scores {
        score.chance /= mass;
    }
    Ok(TemplateSelection { selected, scores })
}

/// Name the chosen chain from independent, unconstrained end-count runs.
/// `top_anchored` has no sacral reward; `bottom_anchored` has neither the
/// first-rib reward nor the C6/C7 start term. The evidence and the runs
/// must belong to this survey, before any film edits. Anchor thresholds
/// (2 z-score units and a 1-unit competing-onset margin) are experimental.
pub fn name_chain(
    chain: &ChainBoundaries,
    selection: &TemplateSelection,
    top_anchored: &ChainBoundaries,
    bottom_anchored: &ChainBoundaries,
    anchors: AnchorEvidence,
) -> LevelNaming {
    let top_weak = !anchors.ridge_present
        || !anchors.first_rib.is_finite()
        || anchors.first_rib < 2.0
        || anchors
            .competing_first_rib
            .is_some_and(|other| other.is_finite() && other >= anchors.first_rib - 1.0);
    let bottom_weak = !anchors.sacrum.is_finite() || anchors.sacrum < 2.0;
    let from_top = (!top_weak)
        .then(|| count_from_top::count(chain, top_anchored))
        .flatten();
    let from_bottom = (!bottom_weak)
        .then(|| count_from_bottom::count(chain, bottom_anchored))
        .flatten();
    let mut flags = Vec::new();
    if chain.template != Template::Standard {
        flags.push(NamingFlag::NonStandardCount);
    }
    if let Some(standard) = selection
        .scores
        .iter()
        .find(|score| score.template == Template::Standard)
        && selection.scores.iter().any(|score| {
            score.template != Template::Standard
                && (score.score - standard.score).abs() <= COUNT_MARGIN
        })
    {
        flags.push(NamingFlag::CloseAlternative);
    }
    if disagreement::ends_disagree(top_anchored, bottom_anchored)
        || matches!((&from_top, &from_bottom), (Some(top), Some(bottom)) if top != bottom)
    {
        flags.push(NamingFlag::EndsDisagree);
    }
    if from_top.is_none() {
        flags.push(NamingFlag::TopAnchorWeak);
    }
    if from_bottom.is_none() {
        flags.push(NamingFlag::BottomAnchorWeak);
    }
    if from_top.is_none() && from_bottom.is_none() {
        flags.push(NamingFlag::NamesUnknown);
    }
    LevelNaming {
        template: chain.template,
        chosen_by: ChosenBy::Engine,
        alternatives: selection
            .scores
            .iter()
            .map(|score| TemplateChance {
                template: score.template,
                chance: score.chance,
            })
            .collect(),
        from_top,
        from_bottom,
        flags,
        l5_confirmed_on: None,
    }
}

/// The naming checkpoint always lists the final bodies head first.
pub fn frame(chain: &ChainBoundaries, naming: &LevelNaming) -> WatchFrame {
    WatchFrame::LevelsNamed {
        film: chain.film,
        stage: Stage::Naming,
        naming: naming.clone(),
        levels: chain.bodies.iter().map(|body| body.level).collect(),
    }
}
