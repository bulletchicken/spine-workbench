//! `ChainPass`: stages 4 to 6 over every film's strip.

#[path = "chain_survey.rs"]
mod survey_work;
#[path = "chain_sweep.rs"]
mod sweep_work;

use super::frames;
use crate::chain_search::{
    dynamic_program,
    paired::{self, SurveyView},
    survey::{self, Anchors, FilmSearch},
    two_views::{self, ViewGeometry},
};
use crate::handoffs::Strip;
use crate::{confidence, level_naming};
use shapes::auto_pass::{
    ChainAnswer, ChainOptions, FilmChain, FilmPassResult, PassWarning, PassWarningCode, Stage,
    Step, TwoFilmCheck,
};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::naming::ChosenBy;
use shapes::watch_frames::{OutlineQuad, WatchFrame};

// Calibration can carry a large budget across unlike stages. Bound the
// row work even then, leaving a cancellation point within every sweep.
const MAX_SWEEP_UNITS: u32 = 64;

struct FilmWork {
    strip: Strip,
    warnings: Vec<PassWarning>,
    survey: Option<survey_work::Survey>,
    search: Option<FilmSearch>,
}

#[derive(Clone, Copy)]
enum Phase {
    Start(usize),
    Survey(usize),
    Pair,
    Bending(usize),
    Lock(usize),
    Naming(usize),
    Confidence(usize),
    Outlines(usize),
    Done,
}

/// A whole chain run owns its inputs. Work units pause the forward sweep
/// every 16 lattice rows; all frames are emitted at algorithm checkpoints.
/// Lattice construction, posterior settlement and paired refinement are
/// atomic stage calls. Dropping or cancelling a pass releases its work.
pub struct ChainPass {
    films: Vec<FilmWork>,
    options: ChainOptions,
    phase: Phase,
    two_films: Option<TwoFilmCheck>,
    outlines: Vec<OutlineQuad>,
    answer: Option<ChainAnswer>,
    failure: Option<EngineError>,
    fraction: f64,
    stage: Stage,
}

impl ChainPass {
    pub fn new(results: Vec<FilmPassResult>, options: ChainOptions) -> Result<Self, EngineError> {
        if results.is_empty() || results.len() > 4 {
            return Err(refused("a chain pass needs one to four distinct films"));
        }
        let mut films: Vec<FilmWork> = Vec::with_capacity(results.len());
        for result in results {
            let strip = Strip::from_bytes(&result.strip)?;
            if strip.film != result.film
                || strip.scale_assumed != result.scale_assumed
                || films.iter().any(|film| film.strip.film == result.film)
                || result
                    .warnings
                    .iter()
                    .any(|warning| warning.film != result.film)
            {
                return Err(refused("the film pass result and strip do not agree"));
            }
            films.push(FilmWork {
                strip,
                warnings: result.warnings,
                survey: Some(survey_work::Survey::new()),
                search: None,
            });
        }
        Ok(Self {
            films,
            options,
            phase: Phase::Start(0),
            two_films: None,
            outlines: Vec::new(),
            answer: None,
            failure: None,
            fraction: 0.0,
            stage: Stage::Chain,
        })
    }

    /// A zero-unit poll never advances or emits a frame. Once failed or
    /// cancelled, every later call returns the same error.
    pub fn advance(&mut self, work_units: u32) -> Result<Step, EngineError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let mut emitted = Vec::new();
        for _ in 0..work_units.min(MAX_SWEEP_UNITS) {
            if matches!(self.phase, Phase::Done) {
                break;
            }
            let batch_sweep = self.is_sweeping();
            if let Err(mut error) = self.one_unit(&mut emitted) {
                if error.code == ErrorCode::ToolConstraintsConflict {
                    error =
                        EngineError::new(ErrorCode::PassNoSpineFound, "no finite chain was found");
                }
                self.failure = Some(error.clone());
                self.films.clear();
                return Err(error);
            }
            // A budget learned during cheap row blocks must not combine
            // the next lattice, posterior or joint solve into one call.
            if !batch_sweep || !self.is_sweeping() {
                break;
            }
        }
        Ok(Step {
            done: self.answer.is_some(),
            stage: self.stage,
            fraction: self.fraction,
            frames: emitted,
        })
    }

    fn is_sweeping(&self) -> bool {
        if let Phase::Survey(index) = self.phase {
            self.films[index]
                .survey
                .as_ref()
                .is_some_and(survey_work::Survey::is_sweeping)
        } else {
            false
        }
    }

    /// The result is available only after the final outline checkpoint.
    pub fn result(&self) -> Result<ChainAnswer, EngineError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        self.answer
            .clone()
            .ok_or_else(|| refused("the chain pass is unfinished"))
    }

    pub fn cancel(&mut self) {
        self.failure = Some(EngineError::new(
            ErrorCode::PassCancelled,
            "the chain pass was cancelled",
        ));
        self.films.clear();
        self.outlines.clear();
        self.answer = None;
    }

    fn one_unit(&mut self, emitted: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        match self.phase {
            Phase::Start(index) => {
                emitted.push(WatchFrame::StageStarted {
                    film: self.films[index].strip.film,
                    stage: Stage::Chain,
                });
                self.phase = Phase::Survey(index);
            }
            Phase::Survey(index) => {
                let film = &mut self.films[index];
                let survey = film.survey.as_mut().expect("unfinished film survey");
                let result = survey.advance(&film.strip, &self.options, emitted)?;
                self.fraction = self
                    .fraction
                    .max(0.8 * (index as f64 + survey.fraction()) / self.films.len() as f64);
                if let Some(search) = result {
                    self.films[index].search = Some(search);
                    self.films[index].survey = None;
                    self.phase = if index + 1 < self.films.len() {
                        Phase::Start(index + 1)
                    } else {
                        Phase::Pair
                    };
                }
            }
            Phase::Pair => {
                self.pair(emitted)?;
                self.phase = Phase::Bending(0);
            }
            Phase::Bending(index) => {
                self.bending(index)?;
                self.phase = if index + 1 < self.films.len() {
                    Phase::Bending(index + 1)
                } else {
                    Phase::Lock(0)
                };
            }
            Phase::Lock(index) => {
                let film = &mut self.films[index];
                let search = film.search.as_mut().expect("finished survey");
                for warning in &film.warnings {
                    if !search.settled.boundaries.warnings.contains(warning) {
                        search.settled.boundaries.warnings.push(warning.clone());
                    }
                }
                search.settled = confidence::settle(
                    search.settled.boundaries.clone(),
                    &film.strip,
                    &search.naming,
                );
                for warning in &search.settled.boundaries.warnings {
                    emitted.push(WatchFrame::Warning {
                        film: film.strip.film,
                        stage: Stage::Chain,
                        warning: warning.clone(),
                    });
                }
                emitted.push(WatchFrame::ChainLocked {
                    film: film.strip.film,
                    stage: Stage::Chain,
                    bodies: search
                        .settled
                        .boundaries
                        .bodies
                        .iter()
                        .rev()
                        .map(|body| frames::locked_body(&film.strip, body))
                        .collect(),
                });
                self.fraction = 0.9 + 0.1 * (index + 1) as f64 / self.films.len() as f64;
                self.phase = if index + 1 < self.films.len() {
                    Phase::Lock(index + 1)
                } else {
                    Phase::Naming(0)
                };
            }
            Phase::Naming(index) => {
                let film = &self.films[index];
                let search = film.search.as_ref().expect("finished survey");
                self.stage = Stage::Naming;
                self.fraction = (index + 1) as f64 / self.films.len() as f64;
                emitted.push(WatchFrame::StageStarted {
                    film: film.strip.film,
                    stage: Stage::Naming,
                });
                emitted.push(level_naming::frame(
                    &search.settled.boundaries,
                    &search.naming,
                ));
                self.phase = if index + 1 < self.films.len() {
                    Phase::Naming(index + 1)
                } else {
                    Phase::Confidence(0)
                };
            }
            Phase::Confidence(index) => {
                let film = &self.films[index];
                let search = film.search.as_ref().expect("finished survey");
                self.stage = Stage::Confidence;
                self.fraction = index as f64 / self.films.len() as f64;
                emitted.push(WatchFrame::StageStarted {
                    film: film.strip.film,
                    stage: Stage::Confidence,
                });
                emitted.push(confidence::frame(&search.settled));
                self.outlines.clear();
                self.phase = Phase::Outlines(index);
            }
            Phase::Outlines(index) => {
                let film = &self.films[index];
                let search = film.search.as_ref().expect("finished survey");
                let chain = &search.settled.boundaries;
                let body = &chain.bodies[chain.bodies.len() - 1 - self.outlines.len()];
                self.outlines
                    .push(frames::outline(&film.strip, body, chain.polarity)?);
                self.outlines.sort_by_key(|quad| quad.level);
                let is_final = self.outlines.len() == chain.bodies.len();
                emitted.push(WatchFrame::Outlines {
                    film: film.strip.film,
                    stage: Stage::Confidence,
                    quads: self.outlines.clone(),
                    is_final,
                });
                self.fraction = (index as f64
                    + self.outlines.len() as f64 / chain.bodies.len() as f64)
                    / self.films.len() as f64;
                if is_final {
                    if index + 1 < self.films.len() {
                        self.phase = Phase::Confidence(index + 1);
                    } else {
                        self.complete();
                    }
                }
            }
            Phase::Done => {}
        }
        Ok(())
    }

    fn index(&self, kind: FilmKind) -> Option<usize> {
        self.films.iter().position(|film| film.strip.film == kind)
    }

    fn pair(&mut self, emitted: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        let (Some(front), Some(side)) = (self.index(FilmKind::Front), self.index(FilmKind::Side))
        else {
            return Ok(());
        };
        let use_pixels =
            self.films[front].strip.scale_assumed || self.films[side].strip.scale_assumed;
        let front_search = self.films[front].search.take().expect("front survey");
        let side_search = self.films[side].search.take().expect("side survey");
        let front_view = SurveyView {
            strip: &self.films[front].strip,
            geometry: geometry(&self.films[front].strip, &front_search, use_pixels)?,
            survey: front_search,
        };
        let side_view = SurveyView {
            strip: &self.films[side].strip,
            geometry: geometry(&self.films[side].strip, &side_search, use_pixels)?,
            survey: side_search,
        };
        let paired = paired::settle(front_view, side_view, &self.options)?;
        emitted.extend(
            paired
                .frames
                .into_iter()
                .filter(|frame| matches!(frame, WatchFrame::JointProgress { .. })),
        );
        self.two_films = Some(paired.check);
        self.films[front].search = Some(paired.front);
        self.films[side].search = Some(paired.side);
        Ok(())
    }

    fn bending(&mut self, index: usize) -> Result<(), EngineError> {
        if !matches!(
            self.films[index].strip.film,
            FilmKind::BendLeft | FilmKind::BendRight
        ) {
            return Ok(());
        }
        let Some(front) = self.index(FilmKind::Front) else {
            self.dropped_bending(index);
            return Ok(());
        };
        let common_units =
            !self.films[index].strip.scale_assumed && !self.films[front].strip.scale_assumed;
        let front = self.films[front].search.as_ref().expect("finished front");
        let template = front.naming.template;
        let heights = front
            .solution
            .states
            .iter()
            .map(|state| state.height as f64 * front.evidence.step)
            .collect::<Vec<_>>();
        let film = &mut self.films[index];
        let search = film.search.take().expect("finished bending survey");
        let problem = survey::problem(
            &search.evidence,
            template,
            self.options.population,
            Anchors::Both,
        );
        let solution = if common_units {
            two_views::solve_bending(&problem, search.evidence.step, &heights)?
        } else {
            // Keep the front's count even when unknown scale prevents a
            // height prior. Guessed millimetres are not shared units.
            two_views::BendingSolution {
                solution: dynamic_program::solve(&problem)?,
                conditioned: problem,
                scale: 1.0,
                dropped: true,
            }
        };
        let mut selection = search.selection;
        selection.selected = template;
        let mut options = self.options.clone();
        options.template = Some(template);
        let mut search = survey::finish(
            &film.strip,
            search.evidence,
            solution.conditioned,
            solution.solution,
            selection,
            &options,
        )?;
        if self.options.template.is_none() {
            search.naming.chosen_by = ChosenBy::Engine;
        }
        film.search = Some(search);
        if solution.dropped {
            self.dropped_bending(index);
        }
        Ok(())
    }

    fn dropped_bending(&mut self, index: usize) {
        let film = &mut self.films[index];
        film.warnings.push(PassWarning {
            code: PassWarningCode::BendingCouplingDropped,
            film: film.strip.film,
            level: None,
            region: None,
        });
    }

    fn complete(&mut self) {
        let films = self
            .films
            .iter()
            .map(|film| {
                let search = film.search.as_ref().expect("finished film");
                FilmChain {
                    film: film.strip.film,
                    naming: search.naming.clone(),
                    chain: search.settled.to_bytes(),
                    warnings: search.settled.boundaries.warnings.clone(),
                }
            })
            .collect();
        self.answer = Some(ChainAnswer {
            films,
            two_films: self.two_films.clone(),
        });
        self.phase = Phase::Done;
        self.fraction = 1.0;
    }
}

fn refused(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineOutOfSync, message)
}

/// Recover original pixel heights from the frozen layout when either
/// view has no scale. The two film-axis spacings follow from the row and
/// column vectors; the z map identifies the upright axis and its sign.
fn geometry(strip: &Strip, search: &FilmSearch, pixels: bool) -> Result<ViewGeometry, EngineError> {
    let spacing = if pixels { pixel_spacing(strip)? } else { 1.0 };
    Ok(ViewGeometry {
        z: search.evidence.z.iter().map(|z| z / spacing).collect(),
        observed: search.evidence.observed.clone(),
        scale_assumed: pixels,
    })
}

fn pixel_spacing(strip: &Strip) -> Result<f64, EngineError> {
    let first = strip.layout[0];
    let sx = 1.0
        / libm::hypot(
            first.across.x() / strip.col_step,
            first.along.x() / strip.row_step,
        );
    let sy = 1.0
        / libm::hypot(
            first.across.y() / strip.col_step,
            first.along.y() / strip.row_step,
        );
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 {
        return Err(refused(
            "the strip has no finite pixel spacing for paired films",
        ));
    }
    // A square pixel has the same height under every quarter turn. The
    // allowance is only for rounding in the orthogonal layout vectors.
    if (sx - sy).abs() <= 32.0 * f64::EPSILON * sx.max(sy) {
        return Ok(sx);
    }
    let residual = |x: bool, spacing: f64| {
        let mut positive = 0.0;
        let mut negative = 0.0;
        for row in &strip.layout {
            let coordinate = if x {
                row.centre.x() - first.centre.x()
            } else {
                row.centre.y() - first.centre.y()
            };
            let z = row.z - first.z;
            positive += (z - coordinate * spacing).abs();
            negative += (z + coordinate * spacing).abs();
        }
        positive.min(negative)
    };
    let x_error = residual(true, sx);
    let y_error = residual(false, sy);
    let magnitude = strip
        .layout
        .iter()
        .map(|row| (row.z - first.z).abs())
        .sum::<f64>();
    let roundoff = 64.0 * f64::EPSILON * (1.0 + magnitude);
    // At exactly 45 physical degrees both stored axes can explain z.
    // The frozen strip has no quarter-turn field, so unequal spacings
    // cannot honestly be converted to upright pixels in that case.
    if !x_error.is_finite()
        || !y_error.is_finite()
        || (x_error - y_error).abs() <= roundoff
        || x_error.min(y_error) > roundoff
    {
        return Err(refused(
            "the strip does not identify an upright pixel scale for paired films",
        ));
    }
    Ok(if x_error < y_error { sx } else { sy })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{StripEvidence, StripRow};
    use shapes::pixels::Px;

    fn layout(turns: u32, diagonal: bool, square: bool) -> Strip {
        let rotate = |mut x: f64, mut y: f64| {
            for _ in 0..turns {
                (x, y) = (y, -x);
            }
            Px::new(x, y)
        };
        let (tx, ty) = if diagonal {
            let v = libm::sqrt(0.5);
            (v, v)
        } else {
            (0.6, 0.8)
        };
        let sy = if square { 0.5 } else { 1.0 };
        Strip {
            film: FilmKind::Front,
            rows: 4,
            cols: 3,
            row_step: 1.0,
            col_step: 1.0,
            scale_assumed: true,
            values: vec![0.5; 12],
            valid: vec![1; 12],
            image_right: None,
            polarity_hint: None,
            evidence: StripEvidence::default(),
            layout: (0..4)
                .map(|row| {
                    let distance = f64::from(row);
                    StripRow {
                        centre: rotate(distance * tx / 0.5, distance * ty / sy),
                        across: rotate(ty / 0.5, -tx / sy),
                        along: rotate(tx / 0.5, ty / sy),
                        curvature: 0.0,
                        half_width: 1.0,
                        z: -distance * ty,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn unknown_scale_pairing_keeps_original_pixel_heights_under_turns() {
        for turns in 0..4 {
            // Upright vertical pixels are 1 length unit for this detector;
            // storing it on any side must preserve that same spacing.
            assert!((pixel_spacing(&layout(turns, false, false)).unwrap() - 1.0).abs() < 1e-14);
            assert!((pixel_spacing(&layout(turns, true, true)).unwrap() - 0.5).abs() < 1e-14);
            assert_eq!(
                pixel_spacing(&layout(turns, true, false)).unwrap_err().code,
                ErrorCode::EngineOutOfSync
            );
        }
    }
}
