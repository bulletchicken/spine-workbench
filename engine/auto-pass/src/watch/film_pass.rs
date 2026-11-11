//! Stages 1 to 3 on one film, keeping their real, ordered checkpoints.
//!
//! The stage entry points are synchronous. A work unit is therefore one
//! complete stage, and every call stops at that stage's boundary even if
//! more units were offered. Cancellation takes effect between stages;
//! interrupting a filter or path search needs a resumable stage entry point.

use crate::handoffs::{CentreLine, FilmEvidence, FilmPassCache};
use crate::settings::BARCODE_SIZE_FACTOR;
use crate::{centre_line, evidence, strip};
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{FilmPassResult, PassWarning, Stage, Step};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::Film;
use shapes::watch_frames::WatchFrame;
use std::rc::Rc;

/// A film pass owns its intermediate maps and shares the immutable pixels
/// with the film's desk. The completed cache can be moved back to that desk.
#[derive(Debug)]
pub struct FilmPass {
    decoded: Rc<DecodedFilm>,
    film: Film,
    next: Phase,
    evidence: Option<FilmEvidence>,
    line: Option<CentreLine>,
    cache: Option<FilmPassCache>,
    result: Option<FilmPassResult>,
    failure: Option<EngineError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Evidence,
    CentreLine,
    Strip,
    Complete,
}

impl Phase {
    fn stage(self) -> Stage {
        match self {
            Self::Evidence => Stage::Evidence,
            Self::CentreLine => Stage::CentreLine,
            Self::Strip | Self::Complete => Stage::Strip,
        }
    }
}

impl FilmPass {
    /// Prepare a pass without computing any image maps. The record is a
    /// snapshot of the geometry, brightness and exclusions for this run.
    pub fn new(decoded: Rc<DecodedFilm>, film: Film) -> Result<Self, EngineError> {
        if decoded.facts.kind != film.kind
            || decoded.image.width() != film.image.width
            || decoded.image.height() != film.image.height
        {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "The film record does not match the decoded pixels",
            ));
        }
        Ok(Self {
            decoded,
            film,
            next: Phase::Evidence,
            evidence: None,
            line: None,
            cache: None,
            result: None,
            failure: None,
        })
    }

    /// Complete at most one stage. Zero units inspect progress without
    /// doing work. Frames are never thinned here: their checkpoint sequence
    /// is the same for every positive budget and includes every strip row.
    pub fn advance(&mut self, work_units: u32) -> Result<Step, EngineError> {
        self.check_failure()?;
        let stage = self.next.stage();
        if work_units == 0 || self.next == Phase::Complete {
            return Ok(Step {
                done: self.next == Phase::Complete,
                stage,
                fraction: if self.next == Phase::Complete {
                    1.0
                } else {
                    0.0
                },
                frames: Vec::new(),
            });
        }
        match self.compute_next() {
            Ok(frames) => Ok(Step {
                done: self.next == Phase::Complete,
                stage,
                fraction: 1.0,
                frames,
            }),
            Err(error) => {
                self.discard_work();
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }

    /// The completed worker baton. Reading it does not consume the cache,
    /// so the bridge can return the baton and retain the maps for points.
    pub fn result(&self) -> Result<FilmPassResult, EngineError> {
        self.check_failure()?;
        self.result.clone().ok_or_else(not_finished)
    }

    /// Borrow the completed maps, centre line, strip and barcode.
    pub fn cache(&self) -> Result<&FilmPassCache, EngineError> {
        self.check_failure()?;
        self.cache.as_ref().ok_or_else(not_finished)
    }

    /// Move the completed cache into the film's desk. The small result
    /// remains available after the move; the cache can only be taken once.
    pub fn take_cache(&mut self) -> Result<FilmPassCache, EngineError> {
        self.check_failure()?;
        self.cache.take().ok_or_else(not_finished)
    }

    /// Stop between stage calls and release partial maps. A stopped pass
    /// never returns an answer or any more frames, including on zero units.
    pub fn cancel(&mut self) {
        self.discard_work();
        self.failure = Some(EngineError::new(
            ErrorCode::PassCancelled,
            "The film pass was cancelled",
        ));
    }

    fn compute_next(&mut self) -> Result<Vec<WatchFrame>, EngineError> {
        match self.next {
            Phase::Evidence => {
                let (evidence, frames) = evidence::build_for_film(&self.decoded, &self.film)?;
                self.evidence = Some(evidence);
                self.next = Phase::CentreLine;
                Ok(frames)
            }
            Phase::CentreLine => {
                let evidence = self.evidence.as_ref().expect("evidence stage completed");
                let (line, mut frames) =
                    centre_line::trace_with_checkpoints(evidence, &self.film.geometry)?;
                frames.extend(
                    line.warnings
                        .iter()
                        .cloned()
                        .map(|warning| WatchFrame::Warning {
                            film: self.film.kind,
                            stage: Stage::CentreLine,
                            warning,
                        }),
                );
                self.line = Some(line);
                self.next = Phase::Strip;
                Ok(frames)
            }
            Phase::Strip => {
                let evidence = self.evidence.as_ref().expect("evidence stage completed");
                let line = self.line.as_ref().expect("centre line stage completed");
                let strip =
                    strip::unroll::build(&self.decoded.image, evidence, line, &self.film.geometry)?;
                // The worker baton has no size factor. Use the same fixed
                // barcode scale as the receiving chain pass.
                let barcode = strip::barcode::build(&strip, BARCODE_SIZE_FACTOR)?;
                let frames = strip::checkpoints::frames(&strip, &barcode);
                let mut warnings = Vec::new();
                for warning in evidence
                    .warnings
                    .iter()
                    .chain(&line.warnings)
                    .chain(strip::warnings(&strip).iter())
                {
                    push_warning(&mut warnings, warning);
                }
                self.result = Some(FilmPassResult {
                    film: self.film.kind,
                    strip: strip.to_bytes(),
                    scale_assumed: strip.scale_assumed,
                    warnings,
                });
                self.cache = Some(FilmPassCache {
                    evidence: self.evidence.take().expect("evidence stage completed"),
                    centre_line: self.line.take().expect("centre line stage completed"),
                    strip,
                    barcode,
                });
                self.next = Phase::Complete;
                Ok(frames)
            }
            Phase::Complete => Ok(Vec::new()),
        }
    }

    fn check_failure(&self) -> Result<(), EngineError> {
        self.failure.clone().map_or(Ok(()), Err)
    }

    fn discard_work(&mut self) {
        self.evidence = None;
        self.line = None;
        self.cache = None;
        self.result = None;
    }
}

fn not_finished() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfSync,
        "The film pass has no completed result available",
    )
}

fn push_warning(warnings: &mut Vec<PassWarning>, warning: &PassWarning) {
    if !warnings.contains(warning) {
        warnings.push(warning.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::{
        DisplayWindow, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat, WindowFunction,
        WindowSource,
    };
    use shapes::film_geometry::FilmGeometry;

    fn constant_film() -> (Rc<DecodedFilm>, Film) {
        let facts = FilmFacts {
            kind: FilmKind::Front,
            width: 64,
            height: 64,
            bits_stored: 8,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            default_window: DisplayWindow::CentreWidth {
                centre: 128.0,
                width: 256.0,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        };
        let film = Film {
            kind: facts.kind,
            image: FilmImage {
                path: facts.kind.png_path().to_owned(),
                width: facts.width,
                height: facts.height,
                bits_stored: facts.bits_stored,
            },
            source: facts.source,
            kept_tags: None,
            values: facts.values.clone(),
            flags: Vec::new(),
            blanked: Vec::new(),
            geometry: FilmGeometry {
                mm_per_px: Some(0.5),
                ..FilmGeometry::UNKNOWN
            },
        };
        let decoded = Rc::new(DecodedFilm {
            image: GreyImage::new(64, 64, vec![128; 64 * 64], Placement::FILM).unwrap(),
            facts,
        });
        (decoded, film)
    }

    #[test]
    fn zero_units_do_no_work_and_cancel_never_runs_the_flat_film() {
        let (decoded, film) = constant_film();
        let mut pass = FilmPass::new(decoded.clone(), film).unwrap();
        assert_eq!(Rc::strong_count(&decoded), 2);
        assert_eq!(
            pass.advance(0).unwrap(),
            Step {
                done: false,
                stage: Stage::Evidence,
                fraction: 0.0,
                frames: Vec::new(),
            }
        );
        assert_eq!(pass.result().unwrap_err().code, ErrorCode::EngineOutOfSync);
        pass.cancel();
        for units in [0, 1, u32::MAX] {
            assert_eq!(
                pass.advance(units).unwrap_err().code,
                ErrorCode::PassCancelled
            );
        }
        assert_eq!(pass.result().unwrap_err().code, ErrorCode::PassCancelled);
        assert_eq!(
            pass.take_cache().unwrap_err().code,
            ErrorCode::PassCancelled
        );
    }

    #[test]
    fn a_failed_stage_stays_failed_without_repeating_work() {
        let (decoded, film) = constant_film();
        let mut pass = FilmPass::new(decoded, film).unwrap();
        // A constant image has zero intensity range, independently of the
        // orchestration: the evidence stage must reject it as flat.
        let error = pass.advance(1).unwrap_err();
        assert_eq!(error.code, ErrorCode::PassFlatFilm);
        assert_eq!(pass.advance(10).unwrap_err(), error);
        assert_eq!(pass.result().unwrap_err(), error);
        assert_eq!(pass.cache().unwrap_err(), error);
    }

    #[test]
    fn a_record_for_another_film_is_refused_before_any_work() {
        let (decoded, mut film) = constant_film();
        film.kind = FilmKind::Side;
        assert_eq!(
            FilmPass::new(decoded, film).unwrap_err().code,
            ErrorCode::EngineOutOfSync
        );
    }
}
