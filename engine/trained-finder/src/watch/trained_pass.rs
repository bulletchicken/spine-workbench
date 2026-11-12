//! The trained finder on one front film, in small steps, for the browser.
//!
//! It keeps the rules of the auto pass's steppers
//! (`docs/decisions/stepped-passes-for-the-watchable-sweep.md`): `new` does
//! no image work, so a worker can answer at once; each call does at most
//! the units it is offered; frames come from fixed points in the work,
//! never from where a call happened to stop; and after a failure or a
//! cancel every later call gives the same error. A call also ends after
//! any unit that gave frames, so they are handed over before the next unit
//! runs: when that unit fails, the frames before it have already gone out,
//! whatever the slice size.
//!
//! The first unit reads the film through its record into the upright 8-bit
//! picture the network is shown ([`as_shown::picture`]); the rest are the
//! search's ([`super::search`]). The unit that settles the last corner also
//! turns what was found into marks on the original film.

use super::search::Search;
use crate::intake::as_shown;
use crate::to_marks;
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{Stage, Step, TrainedPassResult};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmKind};
use shapes::film_geometry::ImageRight;
use shapes::levels::Level;
use shapes::watch_frames::{OutlineQuad, WatchFrame};
use std::fmt;
use std::rc::Rc;

/// The fewest pixels a film may have along either side to be searched. It
/// is the hand-built pass's own floor (`engine/auto-pass/src/evidence/
/// prepare.rs`), stated again here because this crate does not depend on
/// that one; a film below it is refused the same way by both finders.
const SMALLEST_SIDE: u32 = 64;

/// One front film's trained pass, part way through.
pub struct TrainedPass {
    decoded: Rc<DecodedFilm>,
    film: Film,
    first_vertebra_id: u32,
    named_for: ImageRight,
    /// Made by the first unit.
    search: Option<Search>,
    result: Option<TrainedPassResult>,
    failure: Option<EngineError>,
    units_done: u32,
}

impl TrainedPass {
    /// Checks the record against the pixels and does no image work, so a
    /// worker can answer at once. Vertebra ids start at
    /// `first_vertebra_id`. A film narrower or shorter than the
    /// hand-built pass will search is refused with `PassFilmTooSmall`.
    pub fn new(
        decoded: Rc<DecodedFilm>,
        film: Film,
        first_vertebra_id: u32,
    ) -> Result<Self, EngineError> {
        if film.kind != FilmKind::Front
            || decoded.facts.kind != film.kind
            || decoded.image.width() != film.image.width
            || decoded.image.height() != film.image.height
        {
            return Err(refused(
                "the trained finder needs a front film whose record matches its pixels",
            ));
        }
        if film.image.width < SMALLEST_SIDE || film.image.height < SMALLEST_SIDE {
            return Err(EngineError::new(
                ErrorCode::PassFilmTooSmall,
                "the film is too small for the trained finder to search",
            ));
        }
        // The names follow the film's facing, as the auto pass's do.
        let named_for = film.geometry.image_right.unwrap_or(ImageRight::PatientLeft);
        if !named_for.suits_front_view() {
            return Err(refused("a front film faces patient left or right"));
        }
        if first_vertebra_id
            .checked_add(Level::MEASURED.len() as u32)
            .is_none()
        {
            return Err(refused("vertebra ids exhausted"));
        }
        Ok(TrainedPass {
            decoded,
            film,
            first_vertebra_id,
            named_for,
            search: None,
            result: None,
            failure: None,
            units_done: 0,
        })
    }

    /// Does at most `work_units` units of work, stopping early after a
    /// unit that gave frames. `advance(0)` only reports, and so does any
    /// call once done.
    pub fn advance(&mut self, work_units: u32) -> Result<Step, EngineError> {
        self.check_failure()?;
        let mut frames = Vec::new();
        for _ in 0..work_units {
            // An error carries no frames, so frames already made go out
            // now rather than risk the next unit failing.
            if self.result.is_some() || !frames.is_empty() {
                break;
            }
            if let Err(error) = self.unit(&mut frames) {
                self.search = None;
                self.failure = Some(error.clone());
                return Err(error);
            }
            self.units_done += 1;
        }
        let (stage, fraction) = self.progress();
        Ok(Step {
            done: self.result.is_some(),
            stage,
            fraction,
            frames,
        })
    }

    /// The answer, once a step has said `done`.
    pub fn result(&self) -> Result<TrainedPassResult, EngineError> {
        self.check_failure()?;
        self.result.clone().ok_or_else(|| {
            EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the trained pass has no finished result yet",
            )
        })
    }

    /// Stops the pass and lets its work go. Every later call, `advance(0)`
    /// included, gives `PassCancelled`.
    pub fn cancel(&mut self) {
        self.search = None;
        self.result = None;
        self.failure = Some(EngineError::new(
            ErrorCode::PassCancelled,
            "the trained pass was cancelled",
        ));
    }

    fn unit(&mut self, frames: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        let Some(search) = &mut self.search else {
            frames.push(WatchFrame::StageStarted {
                film: FilmKind::Front,
                stage: Stage::Evidence,
            });
            let picture = as_shown::picture(&self.decoded, &self.film)?;
            self.search = Some(Search::new(picture));
            return Ok(());
        };
        search.unit(frames)?;
        if search.is_done() {
            let search = self.search.take().expect("the search is here");
            let placement = search.placement();
            let found = search.found().expect("the search is done");
            let marks = to_marks::film_marks(
                &found.vertebrae,
                placement,
                self.named_for,
                self.first_vertebra_id,
            );
            frames.push(WatchFrame::Outlines {
                film: FilmKind::Front,
                stage: Stage::Confidence,
                quads: marks
                    .vertebrae
                    .iter()
                    .map(|vertebra| OutlineQuad::new(vertebra.level, &vertebra.marks))
                    .collect(),
                is_final: true,
            });
            self.result = Some(TrainedPassResult {
                marks,
                warnings: found.warnings,
            });
        }
        Ok(())
    }

    fn check_failure(&self) -> Result<(), EngineError> {
        self.failure.clone().map_or(Ok(()), Err)
    }

    /// The stage the last unit worked in, and how far through it.
    fn progress(&self) -> (Stage, f64) {
        match (&self.result, &self.search) {
            (Some(_), _) => (Stage::Confidence, 1.0),
            (None, Some(search)) => search.progress(),
            (None, None) => (Stage::Evidence, 0.0),
        }
    }
}

/// Where the pass has got to, never the film or the network's maps: part
/// way through, a pass holds megabytes of them.
impl fmt::Debug for TrainedPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (stage, fraction) = self.progress();
        f.debug_struct("TrainedPass")
            .field("stage", &stage)
            .field("fraction", &fraction)
            .field("units_done", &self.units_done)
            .field("done", &self.result.is_some())
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

fn refused(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineOutOfSync, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::{
        DisplayWindow, FilmFacts, FilmImage, PixelValues, SourceFormat, WindowFunction,
        WindowSource,
    };
    use shapes::film_geometry::FilmGeometry;

    /// A grey front film of `width` by `height` stored in 8 bits, and its
    /// record.
    fn grey_film_of(width: u32, height: u32) -> (Rc<DecodedFilm>, Film) {
        let facts = FilmFacts {
            kind: FilmKind::Front,
            width,
            height,
            bits_stored: 8,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            default_window: DisplayWindow::CentreWidth {
                centre: 127.5,
                width: 255.0,
                function: WindowFunction::LinearExact,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        };
        let film = Film {
            kind: FilmKind::Front,
            image: FilmImage {
                path: FilmKind::Front.png_path().to_owned(),
                width,
                height,
                bits_stored: 8,
            },
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            blanked: Vec::new(),
            geometry: FilmGeometry::UNKNOWN,
        };
        let pixels = vec![40; width as usize * height as usize];
        let decoded = DecodedFilm {
            image: GreyImage::new(width, height, pixels, Placement::FILM).unwrap(),
            facts,
        };
        (Rc::new(decoded), film)
    }

    /// The smallest film the pass will search.
    fn grey_film() -> (Rc<DecodedFilm>, Film) {
        grey_film_of(SMALLEST_SIDE, 2 * SMALLEST_SIDE)
    }

    #[test]
    fn zero_units_do_no_work_and_a_cancelled_pass_stays_cancelled() {
        let (decoded, film) = grey_film();
        let mut pass = TrainedPass::new(decoded.clone(), film, 1).unwrap();
        let idle = Step {
            done: false,
            stage: Stage::Evidence,
            fraction: 0.0,
            frames: Vec::new(),
        };
        assert_eq!(pass.advance(0).unwrap(), idle);
        assert!(pass.search.is_none(), "no image work before the first unit");
        assert_eq!(pass.result().unwrap_err().code, ErrorCode::EngineOutOfSync);
        // The intake gives the first frame, so the call ends there, though
        // it was offered two units; the next starts the first reading.
        let step = pass.advance(2).unwrap();
        assert_eq!(
            step.frames,
            [WatchFrame::StageStarted {
                film: FilmKind::Front,
                stage: Stage::Evidence,
            }]
        );
        assert!(!step.done);
        assert!(format!("{pass:?}").contains("units_done: 1"));
        assert!(pass.advance(1).unwrap().frames.is_empty());
        let shown = format!("{pass:?}");
        assert!(
            shown.contains("stage: Evidence") && shown.contains("units_done: 2"),
            "{shown}"
        );
        pass.cancel();
        assert!(pass.search.is_none(), "the cancelled work is let go");
        for units in [0, 1, u32::MAX] {
            assert_eq!(
                pass.advance(units).unwrap_err().code,
                ErrorCode::PassCancelled
            );
        }
        assert_eq!(pass.result().unwrap_err().code, ErrorCode::PassCancelled);
    }

    #[test]
    fn a_record_the_finder_cannot_read_is_refused_before_any_work() {
        let (decoded, film) = grey_film();
        let mut side = film.clone();
        side.kind = FilmKind::Side;
        let mut wider = film.clone();
        wider.image.width += 1;
        let mut facing_forward = film.clone();
        facing_forward.geometry.image_right = Some(ImageRight::Anterior);
        for record in [side, wider, facing_forward] {
            let error = TrainedPass::new(decoded.clone(), record, 1).err().unwrap();
            assert_eq!(error.code, ErrorCode::EngineOutOfSync);
        }
        let error = TrainedPass::new(decoded, film, u32::MAX - 3).err().unwrap();
        assert_eq!(error.code, ErrorCode::EngineOutOfSync);
    }

    #[test]
    fn a_film_too_small_to_search_is_refused_before_any_work() {
        for (width, height) in [
            (1, 1),
            (SMALLEST_SIDE - 1, 2 * SMALLEST_SIDE),
            (2 * SMALLEST_SIDE, SMALLEST_SIDE - 1),
            (3, 2000),
            (1, 16384),
            (16384, 1),
        ] {
            let (decoded, film) = grey_film_of(width, height);
            let error = TrainedPass::new(decoded, film, 1).err().unwrap();
            assert_eq!(
                error.code,
                ErrorCode::PassFilmTooSmall,
                "{width} by {height}"
            );
        }
        let (decoded, film) = grey_film_of(SMALLEST_SIDE, SMALLEST_SIDE);
        assert!(TrainedPass::new(decoded, film, 1).is_ok());
    }

    #[test]
    fn a_blanked_box_off_the_film_fails_the_first_unit_for_good() {
        let (decoded, mut film) = grey_film();
        // The film is 64 pixels wide, so its right edge is at 63.5.
        film.blanked = vec![shapes::pixels::PxRect {
            left: 60.0,
            top: 0.0,
            right: 70.0,
            bottom: 10.0,
        }];
        let mut pass = TrainedPass::new(decoded, film, 1).unwrap();
        let error = pass.advance(1).unwrap_err();
        assert_eq!(error.code, ErrorCode::CaseInvalidValue);
        assert_eq!(pass.advance(0).unwrap_err(), error);
        assert_eq!(pass.advance(5).unwrap_err(), error);
        assert_eq!(pass.result().unwrap_err(), error);
    }
}
