//! Unconstrained count survey and the frozen stage hand-offs. Film edits
//! have no entry here: a later constrained solve uses the cached naming.

use super::{
    dynamic_program::{self, ChainProblem, Solution},
    gap_score::LatticeEvidence,
    height_prior::HeightPrior,
    posterior, tilts,
};
use crate::{
    confidence::per_vertebra,
    handoffs::{
        Barcode, BodyCertainty, ChainBoundaries, EndplateOnStrip, FoundBody, Polarity,
        SettledChain, Strip,
    },
    level_naming::{self, AnchorEvidence, TemplateSelection},
};
use shapes::{
    auto_pass::{ChainOptions, PassWarning, PassWarningCode, Population, Stage},
    errors::EngineError,
    naming::{ChosenBy, LevelNaming, Template},
    pixels::Px,
    watch_frames::{ChainBody, Segment, WatchFrame},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchors {
    Both,
    Top,
    Bottom,
}

/// Everything a later worker or constrained solve needs, without reaching
/// into the film stages. The evidence and naming are reusable.
pub struct FilmSearch {
    pub settled: SettledChain,
    pub naming: LevelNaming,
    pub frames: Vec<WatchFrame>,
    pub evidence: LatticeEvidence,
    pub problem: ChainProblem,
    pub solution: Solution,
    pub selection: TemplateSelection,
}

pub fn search(
    strip: &Strip,
    barcode: &Barcode,
    options: &ChainOptions,
) -> Result<FilmSearch, EngineError> {
    let mut evidence = LatticeEvidence::build(
        strip,
        barcode,
        strip.polarity_hint.unwrap_or(Polarity::BoneBright),
    )?;
    if strip.polarity_hint.is_none() {
        let dark = LatticeEvidence::build(strip, barcode, Polarity::BoneDark)?;
        let energy = |e: &LatticeEvidence| {
            dynamic_program::solve(&problem(
                e,
                Template::Standard,
                options.population,
                Anchors::Both,
            ))
            .map(|s| s.energy)
        };
        let bright_energy = energy(&evidence)?;
        let dark_energy = energy(&dark)?;
        if dark_energy < bright_energy {
            evidence = dark;
        }
        if (bright_energy - dark_energy).abs() < 5.0 {
            evidence
                .warnings
                .push(warning(strip, PassWarningCode::PolarityUncertain, None));
        }
    }
    let mut result = survey(strip, evidence, options)?;
    let tilt = tilts::settle(
        &result.evidence,
        &result.solution.states,
        sacral_row(
            &result.evidence,
            &result.solution,
            options.population,
            result.naming.template,
        ),
    );
    if tilt.limit_hits > 3 {
        let mut wide =
            LatticeEvidence::with_tilt_limit(strip, barcode, result.evidence.polarity, 35.0)?;
        wide.warnings.extend(
            result
                .evidence
                .warnings
                .iter()
                .filter(|warning| warning.code == PassWarningCode::PolarityUncertain)
                .cloned(),
        );
        wide.warnings
            .push(warning(strip, PassWarningCode::TiltRangeWidened, None));
        result = survey(strip, wide, options)?;
    }
    Ok(result)
}

fn survey(
    strip: &Strip,
    evidence: LatticeEvidence,
    options: &ChainOptions,
) -> Result<FilmSearch, EngineError> {
    let mut energies = Vec::new();
    for template in Template::ALL {
        let p = problem(&evidence, template, options.population, Anchors::Both);
        energies.push((template, dynamic_program::solve(&p)?.energy));
    }
    let selection = level_naming::rank_templates(&energies, 1.0)?;
    let template = options.template.unwrap_or(selection.selected);
    // Keep one count's state tables, rather than five complete graphs.
    let p = problem(&evidence, template, options.population, Anchors::Both);
    let solution = dynamic_program::solve(&p)?;
    finish(strip, evidence, p, solution, selection, options)
}

/// Finish a selected or conditioned solution. The anchor runs remain
/// unconstrained and the count probabilities remain the original survey.
pub fn finish(
    strip: &Strip,
    evidence: LatticeEvidence,
    p: ChainProblem,
    solution: Solution,
    selection: TemplateSelection,
    options: &ChainOptions,
) -> Result<FilmSearch, EngineError> {
    let template = options.template.unwrap_or(selection.selected);
    let certainty = posterior::infer(&p, &solution.states, evidence.step, 1.0)?.certainty;
    let boundaries = boundaries(
        strip,
        &evidence,
        &solution,
        template,
        options.population,
        &certainty,
    );
    let top_p = problem(&evidence, template, options.population, Anchors::Top);
    let bottom_p = problem(&evidence, template, options.population, Anchors::Bottom);
    let top_solution = dynamic_program::solve(&top_p)?;
    let bottom_solution = dynamic_program::solve(&bottom_p)?;
    // These private chains are used only for overlap matching; confidence
    // belongs to the returned chain, so each anchored chain also receives
    // its own posterior rather than borrowing the final placement's.
    let top_certainty =
        posterior::infer(&top_p, &top_solution.states, evidence.step, 1.0)?.certainty;
    let bottom_certainty =
        posterior::infer(&bottom_p, &bottom_solution.states, evidence.step, 1.0)?.certainty;
    let top = boundaries_for_count(&evidence, &top_solution, template, &top_certainty);
    let bottom = boundaries_for_count(&evidence, &bottom_solution, template, &bottom_certainty);
    let rib = solution.states[1];
    let first_rib = evidence.rib_strength(rib.top);
    let competing = (rib.top.saturating_sub(rib.height)..(rib.top + rib.height).min(evidence.rows))
        .filter(|&row| row.abs_diff(rib.top) > rib.height / 2)
        .map(|row| evidence.rib_strength(row))
        .max_by(f64::total_cmp);
    let anchor = AnchorEvidence {
        first_rib,
        competing_first_rib: competing,
        sacrum: sacral_row(&evidence, &solution, options.population, template)
            .map_or(0.0, |r| evidence.sacrum_strength(r)),
        ridge_present: strip.evidence.ridge.is_some() && strip.evidence.ridge_angle.is_some(),
    };
    let mut naming = level_naming::name_chain(&boundaries, &selection, &top, &bottom, anchor);
    if options.template.is_some() {
        naming.chosen_by = ChosenBy::User;
    }
    let settled = per_vertebra::settle(boundaries, strip, &naming);
    let mut frames = vec![WatchFrame::StageStarted {
        film: strip.film,
        stage: Stage::Chain,
    }];
    for checkpoint in &solution.sweeps {
        let levels = template.levels();
        let tentative = checkpoint
            .path
            .iter()
            .enumerate()
            .map(|(v, s)| {
                let (_, a, b) = evidence.body_edges(s.top, s.height);
                chain_body(
                    strip,
                    levels[v],
                    evidence.native_row(s.top as f64),
                    evidence.native_row((s.top + s.height) as f64),
                    [evidence.tilts[a], evidence.tilts[b]],
                    !evidence.observed[s.top] || !evidence.observed[s.top + s.height],
                )
            })
            .collect();
        frames.push(WatchFrame::Sweep {
            film: strip.film,
            stage: Stage::Chain,
            level: levels[checkpoint.body],
            heat: heat(
                &checkpoint.boundary_energy[evidence.padding..evidence.rows - evidence.padding],
            ),
            tentative,
        });
    }
    for score in &selection.scores {
        frames.push(WatchFrame::TemplateScored {
            film: strip.film,
            stage: Stage::Chain,
            template: score.template,
            score: score.score,
            chance: score.chance,
        });
    }
    frames.push(WatchFrame::ChainLocked {
        film: strip.film,
        stage: Stage::Chain,
        bodies: settled
            .boundaries
            .bodies
            .iter()
            .rev()
            .map(|b| {
                chain_body(
                    strip,
                    b.level,
                    b.sup.row,
                    b.inf.row,
                    [b.sup.tilt, b.inf.tilt],
                    b.inferred,
                )
            })
            .collect(),
    });
    frames.push(WatchFrame::StageStarted {
        film: strip.film,
        stage: Stage::Naming,
    });
    frames.push(level_naming::frame(&settled.boundaries, &naming));
    frames.push(WatchFrame::StageStarted {
        film: strip.film,
        stage: Stage::Confidence,
    });
    frames.push(per_vertebra::frame(&settled));
    for w in &settled.boundaries.warnings {
        frames.push(WatchFrame::Warning {
            film: strip.film,
            stage: Stage::Chain,
            warning: w.clone(),
        });
    }
    Ok(FilmSearch {
        settled,
        naming,
        frames,
        evidence,
        problem: p,
        solution,
        selection,
    })
}

/// Build the separable energy tables. Heights/gaps are lattice cells;
/// priors receive length units. No candidate pruning is performed.
pub fn problem(
    e: &LatticeEvidence,
    template: Template,
    population: Population,
    anchors: Anchors,
) -> ChainProblem {
    let h = e.max_height - e.min_height + 1;
    let d = e.max_gap - e.min_gap + 1;
    let size = e.rows * h;
    let levels = template.levels();
    let prior = HeightPrior::new(template, population, e.scale_assumed);
    let mut edge = vec![f64::INFINITY; size];
    for (i, value) in edge.iter_mut().enumerate() {
        let top = i / h;
        let height = e.min_height + i % h;
        if top + height < e.rows {
            *value = -e.body_edges(top, height).0 + e.pedicle(top, height);
        }
    }
    let unary = (0..levels.len())
        .map(|v| {
            edge.iter()
                .enumerate()
                .map(|(i, edge)| {
                    edge + prior.absolute(v, (e.min_height + i % h) as f64 * e.step)
                        + if v == 1 && anchors != Anchors::Bottom {
                            e.rib(i / h)
                        } else {
                            0.0
                        }
                })
                .collect()
        })
        .collect();
    let ratio = (0..levels.len() - 1)
        .map(|v| {
            (0..h * h)
                .map(|i| {
                    prior.ratio(
                        v,
                        (e.min_height + i / h) as f64 * e.step,
                        (e.min_height + i % h) as f64 * e.step,
                    )
                })
                .collect()
        })
        .collect();
    let disc = (0..levels.len() - 1)
        .map(|v| {
            (0..d * h)
                .map(|i| {
                    prior.disc(
                        v,
                        (e.min_gap + i / h) as f64 * e.step,
                        (e.min_height + i % h) as f64 * e.step,
                    )
                })
                .collect()
        })
        .collect();
    let gap = (0..e.rows * d)
        .map(|i| e.gap(i / d, e.min_gap + i % d))
        .collect();
    let mut start = vec![f64::INFINITY; size];
    let mut end = vec![f64::INFINITY; size];
    for i in 0..size {
        let top = i / h;
        let height = e.min_height + i % h;
        let bot = top + height;
        if bot >= e.rows {
            continue;
        }
        if anchors == Anchors::Bottom {
            start[i] = 0.0;
        }
        for gap in e.min_gap..=e.max_gap {
            if top >= gap && anchors != Anchors::Bottom {
                start[i] = start[i].min(
                    e.gap(top - gap, gap)
                        + prior.start(gap as f64 * e.step, height as f64 * e.step),
                );
            }
            if bot + gap < e.rows {
                end[i] = end[i].min(
                    e.gap(bot, gap)
                        + prior.end(gap as f64 * e.step, height as f64 * e.step)
                        + if anchors == Anchors::Top {
                            0.0
                        } else {
                            e.sacrum(bot + gap).0
                        },
                );
            }
        }
    }
    ChainProblem {
        rows: e.rows,
        min_height: e.min_height,
        height_count: h,
        min_gap: e.min_gap,
        gap_count: d,
        unary,
        ratio,
        disc,
        gap,
        start,
        end,
    }
}

fn sacral_row(e: &LatticeEvidence, s: &Solution, pop: Population, t: Template) -> Option<usize> {
    let body = *s.states.last()?;
    let bot = body.top + body.height;
    let prior = HeightPrior::new(t, pop, e.scale_assumed);
    (e.min_gap..=e.max_gap)
        .filter(|&d| bot + d < e.rows)
        .min_by(|&a, &b| {
            let cost = |d| {
                e.gap(bot, d)
                    + prior.end(d as f64 * e.step, body.height as f64 * e.step)
                    + e.sacrum(bot + d).0
            };
            cost(a).total_cmp(&cost(b))
        })
        .map(|d| bot + d)
}

fn boundaries(
    strip: &Strip,
    e: &LatticeEvidence,
    s: &Solution,
    t: Template,
    pop: Population,
    certainty: &[BodyCertainty],
) -> ChainBoundaries {
    let mut chain = boundaries_for_count(e, s, t, certainty);
    let sacrum = sacral_row(e, s, pop, t);
    let tilts = tilts::settle(e, &s.states, sacrum);
    for (v, body) in chain.bodies.iter_mut().enumerate() {
        body.sup.tilt = tilts.angles[v][0];
        body.inf.tilt = tilts.angles[v][1];
        let state = s.states[v];
        let sup_angle = e
            .tilts
            .iter()
            .position(|&angle| angle == body.sup.tilt)
            .expect("a searched tilt");
        let inf_angle = e
            .tilts
            .iter()
            .position(|&angle| angle == body.inf.tilt)
            .expect("a searched tilt");
        body.sup.row = e.top_native_row(state.top, sup_angle);
        body.inf.row = e.bottom_native_row(state.top + state.height, inf_angle);
        if tilts.inconsistent[v]
            || (e
                .tilts
                .last()
                .is_some_and(|last| *last > 20.1_f64.to_radians())
                && tilts.angles[v]
                    .iter()
                    .any(|angle| angle.abs() >= 35.0_f64.to_radians() - 1e-12))
        {
            chain.warnings.push(warning(
                strip,
                PassWarningCode::TiltInconsistent,
                Some(body.level),
            ));
        }
    }
    // Native-cell maxima can move two opposing endplates toward each
    // other. Keep the feasible lattice pair when refinement would use up
    // the tilt pass's clearance, rather than returning crossing lines.
    for v in 1..chain.bodies.len() {
        let upper = chain.bodies[v - 1].inf;
        let lower = chain.bodies[v].sup;
        let clearance = 0.6
            * e.half_width[s.states[v].top]
            * (libm::tan(lower.tilt) - libm::tan(upper.tilt)).abs()
            + 0.5;
        if (lower.row - upper.row) * strip.row_step < clearance {
            chain.bodies[v - 1].inf.row =
                e.native_row((s.states[v - 1].top + s.states[v - 1].height) as f64);
            chain.bodies[v].sup.row = e.native_row(s.states[v].top as f64);
        }
    }
    chain.sacrum = sacrum
        .filter(|&row| e.observed[row] && e.sacrum_strength(row) > 0.0)
        .map(|r| EndplateOnStrip {
            row: e.native_row(r as f64),
            tilt: tilts.sacral_tilt.unwrap_or_else(|| e.sacrum(r).1),
        });
    if let (Some(plateau), Some(plateau_row)) = (chain.sacrum, sacrum) {
        let last = chain.bodies.last_mut().expect("a solved chain has bodies");
        let clearance = 0.6
            * e.half_width[plateau_row]
            * (libm::tan(plateau.tilt) - libm::tan(last.inf.tilt)).abs()
            + 0.5;
        if (plateau.row - last.inf.row) * strip.row_step < clearance {
            let state = s.states.last().expect("a solved chain has states");
            last.inf.row = e.native_row((state.top + state.height) as f64);
        }
    }
    chain
}

fn boundaries_for_count(
    e: &LatticeEvidence,
    s: &Solution,
    t: Template,
    certainty: &[BodyCertainty],
) -> ChainBoundaries {
    let bodies = t
        .levels()
        .into_iter()
        .zip(&s.states)
        .zip(certainty)
        .map(|((level, state), &certainty)| {
            let (_, a, b) = e.body_edges(state.top, state.height);
            FoundBody {
                level,
                sup: EndplateOnStrip {
                    row: e.native_row(state.top as f64),
                    tilt: e.tilts[a],
                },
                inf: EndplateOnStrip {
                    row: e.native_row((state.top + state.height) as f64),
                    tilt: e.tilts[b],
                },
                inferred: !e.observed[state.top] || !e.observed[state.top + state.height],
                certainty,
            }
        })
        .collect();
    ChainBoundaries {
        film: e.film,
        template: t,
        polarity: e.polarity,
        bodies,
        sacrum: None,
        energy: s.energy,
        warnings: e.warnings.clone(),
    }
}

fn warning(
    strip: &Strip,
    code: PassWarningCode,
    level: Option<shapes::levels::Level>,
) -> PassWarning {
    PassWarning {
        code,
        film: strip.film,
        level,
        region: None,
    }
}

fn chain_body(
    strip: &Strip,
    level: shapes::levels::Level,
    sup: f64,
    inf: f64,
    angles: [f64; 2],
    inferred: bool,
) -> ChainBody {
    ChainBody {
        level,
        sup: segment(strip, sup, angles[0]),
        inf: segment(strip, inf, angles[1]),
        inferred,
    }
}

fn segment(strip: &Strip, row: f64, tilt: f64) -> Segment {
    let width = strip.layout[row.max(0.0).min(f64::from(strip.rows) - 1.0) as usize].half_width;
    let width = if width > 0.0 { width } else { 15.0 };
    let columns = width / strip.col_step;
    let point = |c: f64| {
        let r = row + c * strip.col_step * libm::tan(tilt) / strip.row_step;
        let index = (r.floor() as isize).clamp(0, strip.rows as isize - 2) as usize;
        let fraction = r - index as f64;
        let a = strip.layout[index];
        let b = strip.layout[index + 1];
        Px::new(
            a.centre.x()
                + fraction * (b.centre.x() - a.centre.x())
                + c * (a.across.x() + fraction * (b.across.x() - a.across.x())),
            a.centre.y()
                + fraction * (b.centre.y() - a.centre.y())
                + c * (a.across.y() + fraction * (b.across.y() - a.across.y())),
        )
    };
    Segment {
        a: point(-columns),
        b: point(columns),
    }
}

fn heat(energy: &[f64]) -> Vec<u8> {
    let best = energy.iter().copied().fold(f64::INFINITY, f64::min);
    let mut bins = vec![f64::INFINITY; 256];
    for (i, &value) in energy.iter().enumerate() {
        let bin = i * 256 / energy.len();
        bins[bin] = bins[bin].min(value);
    }
    bins.iter()
        .map(|&value| {
            if best.is_finite() && value.is_finite() {
                (255.0 * libm::exp(-(value - best))).round() as u8
            } else {
                0
            }
        })
        .collect()
}
