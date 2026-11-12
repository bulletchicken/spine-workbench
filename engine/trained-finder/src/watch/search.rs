//! The finder's work on one upright 8-bit picture, in small units.
//!
//! The order is the training code's own: the whole picture is read; a
//! frame is chosen from that reading; the frame's crop is read again only
//! when the frame is not the whole picture; the chain is picked from the
//! last reading's peaks and mapped back to the whole picture; each
//! outline's corners are settled on the full-size picture, filled-in ones
//! too; and the vertebrae are put in order by the mean height of their
//! settled corners. It departs from the training code in one place: a
//! picture with too little evidence to frame, whose chain holds no link as
//! strong as the framing asks of a peak, finds nothing rather than weak
//! guesses.
//!
//! One unit is one of the network's own units, one corner patch, or one of
//! the small steps between (starting a reading, picking the chain, naming).
//! How many units there are depends on what the readings find (whether the
//! picture is framed and read again, and how many vertebrae the chain
//! picks), not on the picture's size, and never on how the units are
//! spread over calls; every frame comes from a fixed point in that order.

use crate::chain;
use crate::corners::{patch, settle};
use crate::framing::{self, Frame, FrameReason};
use crate::intake::{as_shown, shrink};
use crate::outline::{ChainLink, Quad, Reading};
use crate::reading;
use crate::spine_net::SpineReading;
use crate::weights::Weights;
use darkroom::grey_image::{GreyImage, Placement};
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::confidence::ConfidenceBand;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::levels::Level;
use shapes::watch_frames::{ChainBody, LevelConfidence, Segment, WatchFrame};

/// What the finder found on one picture, stage by stage.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The first reading, of the whole picture.
    pub first: Reading,
    /// Where the second reading looked: the whole picture when there was
    /// no second reading.
    pub frame: Frame,
    /// The second reading, of the frame's crop, when the frame is not the
    /// whole picture.
    pub second: Option<Reading>,
    /// The chain, in fractions of the picture the last reading read (the
    /// crop when there was a second reading).
    pub chain: Vec<ChainLink>,
    /// Each vertebra found, its corners settled, top to bottom.
    pub vertebrae: Vec<FoundVertebra>,
    pub warnings: Vec<PassWarning>,
}

/// One vertebra the finder settled on.
#[derive(Clone, Debug, PartialEq)]
pub struct FoundVertebra {
    /// 0 for T1 up to 16 for L5.
    pub level_index: usize,
    /// The network did not see it: the chain placed it halfway between its
    /// two neighbours.
    pub filled_in: bool,
    /// The network's score for it; 0 for a filled-in one.
    pub score: f64,
    /// Its settled corners going round (upper left, upper right, lower
    /// right, lower left), in pixels of the picture measured from its
    /// outer top-left corner, so the centre of the top-left pixel is (0.5,
    /// 0.5). [`as_shown::to_film_pixels`] takes them back to the film.
    pub corners: Quad,
}

impl FoundVertebra {
    pub fn level(&self) -> Level {
        Level::MEASURED[self.level_index]
    }
}

/// The search, part way through.
pub(crate) struct Search {
    picture: GreyImage<u8>,
    next: Phase,
    first: Option<Reading>,
    frame: Option<Frame>,
    second: Option<Reading>,
    chain: Vec<ChainLink>,
    /// The chain's outlines on the picture, in the pixels
    /// [`FoundVertebra::corners`] uses, as the corner network is handed them.
    outlines: Vec<Quad>,
    settled: Vec<Quad>,
    warnings: Vec<PassWarning>,
    stage: Stage,
    fraction: f64,
}

enum Phase {
    /// Shrink the whole picture and start the first reading.
    StartFirst,
    /// The first reading; its last unit also reads the maps and frames.
    First(SpineReading<'static>),
    /// Crop to the frame, shrink, and start the second reading.
    StartSecond,
    /// The second reading; its last unit also reads the maps.
    Second(SpineReading<'static>),
    Chain,
    Naming,
    /// One outline's corners settled per unit, top to bottom.
    Corners,
    Done,
}

impl Search {
    /// Does no work: the first unit starts the first reading.
    pub(crate) fn new(picture: GreyImage<u8>) -> Search {
        Search {
            picture,
            next: Phase::StartFirst,
            first: None,
            frame: None,
            second: None,
            chain: Vec::new(),
            outlines: Vec::new(),
            settled: Vec::new(),
            warnings: Vec::new(),
            stage: Stage::Evidence,
            fraction: 0.0,
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        matches!(self.next, Phase::Done)
    }

    /// The stage the last unit worked in, and how far through it.
    pub(crate) fn progress(&self) -> (Stage, f64) {
        (self.stage, self.fraction)
    }

    /// Where the picture sits on the film.
    pub(crate) fn placement(&self) -> Placement {
        self.picture.placement()
    }

    /// Does one unit of work, adding any frames it reaches to `frames`.
    /// Nothing once done. After an error the search is spent.
    pub(crate) fn unit(&mut self, frames: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        self.next = match std::mem::replace(&mut self.next, Phase::Done) {
            Phase::StartFirst => Phase::First(start_reading(&self.picture)?),
            Phase::First(mut reading) => {
                reading.advance(1)?;
                self.fraction = 0.5 * reading_share(&reading);
                if reading.is_done() {
                    self.after_first(read(reading)?, frames)
                } else {
                    Phase::First(reading)
                }
            }
            Phase::StartSecond => {
                let frame = self.frame.expect("the first reading framed the picture");
                Phase::Second(start_reading(&frame.crop(&self.picture)?)?)
            }
            Phase::Second(mut reading) => {
                reading.advance(1)?;
                self.fraction = 0.5 + 0.5 * reading_share(&reading);
                if reading.is_done() {
                    self.second = Some(read(reading)?);
                    self.fraction = 1.0;
                    Phase::Chain
                } else {
                    Phase::Second(reading)
                }
            }
            Phase::Chain => {
                self.pick_chain(frames)?;
                Phase::Naming
            }
            Phase::Naming => {
                self.stage = Stage::Naming;
                frames.push(WatchFrame::StageStarted {
                    film: FilmKind::Front,
                    stage: Stage::Naming,
                });
                frames.push(WatchFrame::LevelsNamed {
                    film: FilmKind::Front,
                    stage: Stage::Naming,
                    naming: crate::to_marks::estimated_naming(),
                    levels: self.chain.iter().map(link_level).collect(),
                });
                Phase::Corners
            }
            Phase::Corners => self.settle_next(frames)?,
            Phase::Done => Phase::Done,
        };
        Ok(())
    }

    /// What was found, once done.
    pub(crate) fn found(self) -> Option<Found> {
        if !self.is_done() {
            return None;
        }
        let height = f64::from(self.picture.height());
        let mut vertebrae: Vec<FoundVertebra> = self
            .chain
            .iter()
            .zip(self.settled)
            .map(|(link, corners)| FoundVertebra {
                level_index: link.level_index,
                filled_in: link.filled_in,
                score: link.score,
                corners,
            })
            .collect();
        // The reference's last step: a stable sort by the mean height of
        // the settled corners, as fractions of the picture.
        let mean_down = |vertebra: &FoundVertebra| {
            let q = vertebra.corners.map(|corner| corner[1] / height);
            (((q[0] + q[1]) + q[2]) + q[3]) / 4.0
        };
        vertebrae.sort_by(|a, b| mean_down(a).total_cmp(&mean_down(b)));
        Some(Found {
            first: self.first?,
            frame: self.frame?,
            second: self.second,
            chain: self.chain,
            vertebrae,
            warnings: self.warnings,
        })
    }

    /// Frames the picture from the first reading and says what comes next.
    fn after_first(&mut self, first: Reading, frames: &mut Vec<WatchFrame>) -> Phase {
        let size = [self.picture.width(), self.picture.height()];
        let frame = framing::frame_from_reading(size, &first);
        if matches!(
            frame.reason,
            FrameReason::InsufficientEvidence | FrameReason::InsufficientCentralEvidence
        ) {
            let warning = PassWarning {
                code: PassWarningCode::LowEvidence,
                film: FilmKind::Front,
                level: None,
                region: None,
            };
            frames.push(WatchFrame::Warning {
                film: FilmKind::Front,
                stage: Stage::Evidence,
                warning: warning.clone(),
            });
            self.warnings.push(warning);
        }
        self.first = Some(first);
        self.frame = Some(frame);
        if frame.whole_film() {
            self.fraction = 1.0;
            Phase::Chain
        } else {
            self.fraction = 0.5;
            Phase::StartSecond
        }
    }

    fn pick_chain(&mut self, frames: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        self.stage = Stage::Chain;
        self.fraction = 1.0;
        frames.push(WatchFrame::StageStarted {
            film: FilmKind::Front,
            stage: Stage::Chain,
        });
        let last = self
            .second
            .as_ref()
            .or(self.first.as_ref())
            .expect("the picture has been read");
        let frame = self.frame.expect("the first reading framed the picture");
        // The one place the engine departs from the reference pipeline
        // (`docs/decisions/a-trained-finder-for-front-films.md`). When the
        // first reading had too few peaks at the framing's proposal score
        // to frame the film, and no link of the chain reaches that score
        // either, the chain is weak guesses where there is most likely no
        // spine, as on a blank film: the reference hands them back, and
        // here nothing is found. A link scores what its peak scored, or 0
        // when filled in, so when no peak reaches the score the chain is
        // known to be guesses without being picked. The film was left
        // whole, so the last reading is the first.
        let threshold = framing::proposal_threshold();
        let little_evidence = frame.reason == FrameReason::InsufficientEvidence;
        let chain = if little_evidence && last.peaks.iter().all(|peak| peak.score < threshold) {
            Vec::new()
        } else {
            chain::select(&last.peaks)?
        };
        let only_guesses = little_evidence && chain.iter().all(|link| link.score < threshold);
        if chain.is_empty() || only_guesses {
            return Err(EngineError::new(
                ErrorCode::PassNoSpineFound,
                "the trained finder found no vertebrae on the front film",
            ));
        }
        let size = [self.picture.width(), self.picture.height()].map(f64::from);
        // As the reference does it: crop fractions to film fractions, then
        // times the picture's size.
        self.outlines = chain
            .iter()
            .map(|link| {
                link.corners.map(|corner| {
                    let on_film = frame.to_film(corner);
                    [on_film[0] * size[0], on_film[1] * size[1]]
                })
            })
            .collect();
        let placement = self.picture.placement();
        frames.push(WatchFrame::ChainLocked {
            film: FilmKind::Front,
            stage: Stage::Chain,
            // From the lowest upward, the order the auto pass locks them in.
            bodies: chain
                .iter()
                .zip(&self.outlines)
                .rev()
                .map(|(link, quad)| {
                    let at = |corner: usize| as_shown::to_film_pixels(placement, quad[corner]);
                    ChainBody {
                        level: link_level(link),
                        sup: Segment { a: at(0), b: at(1) },
                        inf: Segment { a: at(3), b: at(2) },
                        inferred: link.filled_in,
                    }
                })
                .collect(),
        });
        self.chain = chain;
        Ok(())
    }

    /// Settles the next outline's corners; the first also starts the
    /// confidence stage.
    fn settle_next(&mut self, frames: &mut Vec<WatchFrame>) -> Result<Phase, EngineError> {
        if self.settled.is_empty() {
            self.stage = Stage::Confidence;
            self.fraction = 0.0;
            frames.push(WatchFrame::StageStarted {
                film: FilmKind::Front,
                stage: Stage::Confidence,
            });
            frames.push(WatchFrame::Confidence {
                film: FilmKind::Front,
                stage: Stage::Confidence,
                levels: self
                    .chain
                    .iter()
                    .map(|link| LevelConfidence {
                        level: link_level(link),
                        score: link.score,
                        band: ConfidenceBand::for_score(link.score),
                    })
                    .collect(),
            });
        }
        let weights = Weights::baked()?;
        let quad = &self.outlines[self.settled.len()];
        let patch = patch::cut(&self.picture, quad);
        let deltas = weights.corner_net().deltas(&patch);
        self.settled.push(settle::settle(
            quad,
            &deltas,
            &patch,
            weights.corner_blend(),
        ));
        self.fraction = self.settled.len() as f64 / self.outlines.len() as f64;
        Ok(if self.settled.len() == self.outlines.len() {
            Phase::Done
        } else {
            Phase::Corners
        })
    }
}

/// Shrinks `picture` to the network's size and starts a reading of it.
fn start_reading(picture: &GreyImage<u8>) -> Result<SpineReading<'static>, EngineError> {
    Weights::baked()?
        .spine_net()
        .start(&shrink::to_network_size(picture))
}

/// How much of a reading is done, 0 to 1.
fn reading_share(reading: &SpineReading<'_>) -> f64 {
    f64::from(reading.units_done()) / f64::from(reading.units())
}

/// A finished reading's maps read as peaks and outlines.
fn read(reading: SpineReading<'static>) -> Result<Reading, EngineError> {
    let maps = reading.into_maps().ok_or_else(|| {
        EngineError::new(
            ErrorCode::EngineOutOfSync,
            "a reading of the trained finder ended without its maps",
        )
    })?;
    Ok(reading::read(&maps, Weights::baked()?.threshold()))
}

fn link_level(link: &ChainLink) -> Level {
    Level::MEASURED[link.level_index]
}
