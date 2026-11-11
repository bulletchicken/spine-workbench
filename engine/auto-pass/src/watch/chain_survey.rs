//! One film's polarity checks, count surveys and final settlement.

use super::sweep_work::Sweep;
use crate::chain_search::{
    dynamic_program::{ChainProblem, Solution},
    gap_score::LatticeEvidence,
    survey::{self, Anchors, FilmSearch},
};
use crate::handoffs::{Barcode, Polarity, Strip};
use crate::level_naming::{self, TemplateSelection};
use crate::watch::frames;
use shapes::auto_pass::{ChainOptions, PassWarning, PassWarningCode, Stage};
use shapes::errors::EngineError;
use shapes::naming::Template;
use shapes::watch_frames::WatchFrame;

#[derive(Clone, Copy)]
enum SolveFor {
    Bright,
    Dark,
    Template(usize),
    Selected,
}

#[derive(Clone, Copy)]
enum Phase {
    Barcode,
    Build(Polarity),
    Start(SolveFor),
    Sweep(SolveFor),
    Finish,
    Done,
}

pub(super) struct Survey {
    phase: Phase,
    barcode: Option<Barcode>,
    evidence: Option<LatticeEvidence>,
    bright: Option<(LatticeEvidence, f64)>,
    sweep: Option<Sweep>,
    selection: Option<TemplateSelection>,
    energies: Vec<(Template, f64)>,
    selected: Option<(ChainProblem, Solution)>,
    wide: bool,
    polarity_uncertain: bool,
}

impl Survey {
    pub fn new() -> Self {
        Self {
            phase: Phase::Barcode,
            barcode: None,
            evidence: None,
            bright: None,
            sweep: None,
            selection: None,
            energies: Vec::new(),
            selected: None,
            wide: false,
            polarity_uncertain: false,
        }
    }

    pub fn fraction(&self) -> f64 {
        match self.phase {
            Phase::Barcode | Phase::Build(_) => 0.0,
            Phase::Start(_) => 0.1,
            Phase::Sweep(purpose) => {
                let within = self.sweep.as_ref().map_or(0.0, Sweep::fraction);
                match purpose {
                    SolveFor::Bright => 0.1 + 0.05 * within,
                    SolveFor::Dark => 0.15 + 0.05 * within,
                    SolveFor::Template(index) => 0.2 + 0.1 * (index as f64 + within),
                    SolveFor::Selected => 0.7 + 0.1 * within,
                }
            }
            Phase::Finish => 0.8,
            Phase::Done => 1.0,
        }
    }

    pub fn is_sweeping(&self) -> bool {
        matches!(self.phase, Phase::Sweep(_))
    }

    pub fn advance(
        &mut self,
        strip: &Strip,
        options: &ChainOptions,
        emitted: &mut Vec<WatchFrame>,
    ) -> Result<Option<FilmSearch>, EngineError> {
        match self.phase {
            Phase::Barcode => {
                self.barcode = Some(crate::strip::barcode::build(
                    strip,
                    crate::settings::BARCODE_SIZE_FACTOR,
                )?);
                self.phase = Phase::Build(strip.polarity_hint.unwrap_or(Polarity::BoneBright));
            }
            Phase::Build(polarity) => {
                let barcode = self.barcode.as_ref().expect("barcode precedes lattice");
                let mut evidence = LatticeEvidence::with_tilt_limit(
                    strip,
                    barcode,
                    polarity,
                    if self.wide { 35.0 } else { 20.0 },
                )?;
                if self.polarity_uncertain {
                    evidence
                        .warnings
                        .push(warning(strip, PassWarningCode::PolarityUncertain));
                }
                if self.wide {
                    evidence
                        .warnings
                        .push(warning(strip, PassWarningCode::TiltRangeWidened));
                }
                self.evidence = Some(evidence);
                self.phase = Phase::Start(if self.wide || strip.polarity_hint.is_some() {
                    SolveFor::Template(0)
                } else if polarity == Polarity::BoneBright {
                    SolveFor::Bright
                } else {
                    SolveFor::Dark
                });
            }
            Phase::Start(purpose) => {
                let evidence = self.evidence.as_ref().expect("lattice precedes sweep");
                let template = match purpose {
                    SolveFor::Bright | SolveFor::Dark => Template::Standard,
                    SolveFor::Template(index) => Template::ALL[index],
                    SolveFor::Selected => options.template.unwrap_or(
                        self.selection
                            .as_ref()
                            .expect("survey precedes selection")
                            .selected,
                    ),
                };
                self.sweep = Some(Sweep::new(survey::problem(
                    evidence,
                    template,
                    options.population,
                    Anchors::Both,
                ))?);
                self.phase = Phase::Sweep(purpose);
            }
            Phase::Sweep(purpose) => {
                let sweep = self.sweep.as_mut().expect("started sweep");
                if let Some(checkpoint) = sweep.advance()
                    && matches!(purpose, SolveFor::Template(0))
                    && !self.wide
                {
                    emitted.push(frames::sweep(
                        strip,
                        self.evidence.as_ref().expect("lattice precedes sweep"),
                        &checkpoint,
                    ));
                }
                if sweep.done() {
                    let (problem, solution) = self.sweep.take().expect("started sweep").finish()?;
                    self.solved(strip, purpose, problem, solution)?;
                }
            }
            Phase::Finish => {
                let (problem, solution) = self.selected.take().expect("selected chain");
                let evidence = self.evidence.take().expect("selected evidence");
                let first = evidence.tilts[0];
                let last = *evidence.tilts.last().expect("tilt bank");
                let search = survey::finish(
                    strip,
                    evidence,
                    problem,
                    solution,
                    self.selection.take().expect("ranked counts"),
                    options,
                )?;
                // `finish` returns the exact angles chosen by the tilt
                // settlement, so this is the stage's limit-hit criterion.
                let hits = search
                    .settled
                    .boundaries
                    .bodies
                    .iter()
                    .flat_map(|body| [body.sup.tilt, body.inf.tilt])
                    .filter(|angle| *angle == first || *angle == last)
                    .count();
                if hits > 3 && !self.wide {
                    self.wide = true;
                    self.energies.clear();
                    self.phase = Phase::Build(search.evidence.polarity);
                } else {
                    for score in &search.selection.scores {
                        emitted.push(WatchFrame::TemplateScored {
                            film: strip.film,
                            stage: Stage::Chain,
                            template: score.template,
                            score: score.score,
                            chance: score.chance,
                        });
                    }
                    self.phase = Phase::Done;
                    return Ok(Some(search));
                }
            }
            Phase::Done => {}
        }
        Ok(None)
    }

    fn solved(
        &mut self,
        strip: &Strip,
        purpose: SolveFor,
        problem: ChainProblem,
        solution: Solution,
    ) -> Result<(), EngineError> {
        match purpose {
            SolveFor::Bright => {
                self.bright = Some((
                    self.evidence.take().expect("bright evidence"),
                    solution.energy,
                ));
                self.phase = Phase::Build(Polarity::BoneDark);
            }
            SolveFor::Dark => {
                let (bright, energy) = self.bright.take().expect("bright solve before dark");
                if energy <= solution.energy {
                    self.evidence = Some(bright);
                }
                self.polarity_uncertain = (energy - solution.energy).abs() < 5.0;
                if self.polarity_uncertain {
                    self.evidence
                        .as_mut()
                        .expect("chosen polarity")
                        .warnings
                        .push(warning(strip, PassWarningCode::PolarityUncertain));
                }
                self.phase = Phase::Start(SolveFor::Template(0));
            }
            SolveFor::Template(index) => {
                self.energies.push((Template::ALL[index], solution.energy));
                if index + 1 < Template::ALL.len() {
                    self.phase = Phase::Start(SolveFor::Template(index + 1));
                } else {
                    self.selection = Some(level_naming::rank_templates(&self.energies, 1.0)?);
                    self.phase = Phase::Start(SolveFor::Selected);
                }
            }
            SolveFor::Selected => {
                self.selected = Some((problem, solution));
                self.phase = Phase::Finish;
            }
        }
        Ok(())
    }
}

fn warning(strip: &Strip, code: PassWarningCode) -> PassWarning {
    PassWarning {
        code,
        film: strip.film,
        level: None,
        region: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{StripEvidence, StripRow};
    use shapes::auto_pass::{Acquisition, Population};
    use shapes::film::FilmKind;
    use shapes::film_geometry::ImageRight;
    use shapes::pixels::Px;

    fn strip(dark: bool, degrees: f64) -> Strip {
        let rows = 600;
        let cols = 41;
        let slope = libm::tan(degrees * std::f64::consts::PI / 180.0);
        let mut values = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let along = f64::from(row) - (f64::from(col) - 20.0) * slope;
                let bone = (0..18).any(|body| {
                    let top = 25.0 + 30.0 * f64::from(body);
                    along >= top && along < top + 24.0
                });
                let value = if bone { 0.9 } else { 0.1 };
                values.push(if dark { 1.0 - value } else { value });
            }
        }
        Strip {
            film: FilmKind::Side,
            rows,
            cols,
            row_step: 1.0,
            col_step: 1.0,
            scale_assumed: false,
            values,
            valid: vec![1; (rows * cols) as usize],
            layout: (0..rows)
                .map(|row| StripRow {
                    centre: Px::new(20.0, f64::from(row)),
                    across: Px::new(1.0, 0.0),
                    along: Px::new(0.0, 1.0),
                    curvature: 0.0,
                    half_width: 15.0,
                    z: -f64::from(row),
                })
                .collect(),
            image_right: Some(ImageRight::Anterior),
            polarity_hint: (!dark).then_some(Polarity::BoneBright),
            evidence: StripEvidence::default(),
        }
    }

    #[test]
    fn bounded_survey_matches_the_independent_stage_for_polarities_and_wide_tilts() {
        let options = ChainOptions {
            acquisition: Acquisition::Unknown,
            population: Population::Adult,
            template: Some(Template::Standard),
        };
        for (dark, degrees) in [(false, 0.0), (true, 30.0)] {
            let strip = strip(dark, degrees);
            let barcode =
                crate::strip::barcode::build(&strip, crate::settings::BARCODE_SIZE_FACTOR).unwrap();
            let direct = survey::search(&strip, &barcode, &options).unwrap();
            let mut bounded = Survey::new();
            let mut frames = Vec::new();
            let answer = loop {
                if let Some(answer) = bounded.advance(&strip, &options, &mut frames).unwrap() {
                    break answer;
                }
            };
            assert_eq!(answer.settled, direct.settled);
            assert_eq!(answer.naming, direct.naming);
            assert_eq!(answer.solution.forward, direct.solution.forward);
            assert_eq!(answer.settled.boundaries.bodies.len(), 18);
            let sweeps = frames
                .iter()
                .filter_map(|frame| {
                    if let WatchFrame::Sweep { level, .. } = frame {
                        Some(*level)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(sweeps, Template::Standard.levels());
            if dark {
                assert_eq!(answer.evidence.polarity, Polarity::BoneDark);
                assert!(
                    answer
                        .evidence
                        .warnings
                        .iter()
                        .any(|warning| warning.code == PassWarningCode::TiltRangeWidened)
                );
            }
        }
    }
}
