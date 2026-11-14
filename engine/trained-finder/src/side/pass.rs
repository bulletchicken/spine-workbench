//! One side film, advanced through bounded intake, inference and decoding units.

use super::{
    decode,
    intake::{Mapping, SideIntake},
    reading::SideReading,
    resize::Resize,
    to_marks,
    weights::WeightSetup,
};
use crate::grid::Grid;
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{Stage, Step, TrainedPassResult};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmKind};
use shapes::film_geometry::ImageRight;
use shapes::pixels::Px;
use shapes::watch_frames::{ChainBody, LevelConfidence, OutlineQuad, Segment, WatchFrame};
use std::rc::Rc;

pub struct SidePass {
    film: Film,
    first: u32,
    intake: Option<SideIntake>,
    resize: Option<Resize>,
    setup: Option<WeightSetup<'static>>,
    picture: Option<Vec<u8>>,
    reading: Option<SideReading>,
    maps: Option<Grid>,
    mapping: Mapping,
    points: [[f64; 2]; 20],
    channel: usize,
    phase: u8,
    stage: Stage,
    fraction: f64,
    result: Option<TrainedPassResult>,
    failure: Option<EngineError>,
}

impl SidePass {
    /// Validates only; pixels and weights are read by later units.
    pub fn new(
        decoded: Rc<DecodedFilm>,
        film: Film,
        first_vertebra_id: u32,
    ) -> Result<Self, EngineError> {
        if !super::available() {
            return Err(EngineError::new(
                ErrorCode::SideFinderUnavailable,
                "the side finder is not in this build",
            ));
        }
        Self::from_record(decoded, film, first_vertebra_id)
    }

    fn from_record(
        decoded: Rc<DecodedFilm>,
        film: Film,
        first_vertebra_id: u32,
    ) -> Result<Self, EngineError> {
        if film.kind != FilmKind::Side {
            return Err(EngineError::new(
                ErrorCode::CaseInvalidValue,
                "the side finder needs a side film",
            ));
        }
        if film.image.width < 64 || film.image.height < 64 {
            return Err(EngineError::new(
                ErrorCode::PassFilmTooSmall,
                "the film is too small for the side finder",
            ));
        }
        if first_vertebra_id.checked_add(4).is_none() {
            return Err(EngineError::new(
                ErrorCode::CaseInvalidValue,
                "vertebra ids exhausted",
            ));
        }
        if film
            .geometry
            .image_right
            .is_some_and(|f| !matches!(f, ImageRight::Anterior | ImageRight::Posterior))
        {
            return Err(EngineError::new(
                ErrorCode::CaseInvalidValue,
                "a side film faces anterior or posterior",
            ));
        }
        if film.geometry.image_right.is_none() || !film.geometry.confirmed.facing {
            return Err(EngineError::new(
                ErrorCode::PassFacingRequired,
                "choose which way the patient faces",
            ));
        }
        let intake = SideIntake::new(decoded, film.clone())?;
        Ok(Self {
            mapping: intake.mapping,
            film,
            first: first_vertebra_id,
            intake: Some(intake),
            resize: None,
            setup: None,
            picture: None,
            reading: None,
            maps: None,
            points: [[0.0; 2]; 20],
            channel: 0,
            phase: 0,
            stage: Stage::Evidence,
            fraction: 0.0,
            result: None,
            failure: None,
        })
    }

    pub fn advance(&mut self, work_units: u32) -> Result<Step, EngineError> {
        self.check_failure()?;
        let mut frames = match super::reserve(3) {
            Ok(frames) => frames,
            Err(error) => {
                self.release();
                self.failure = Some(error.clone());
                return Err(error);
            }
        };
        for _ in 0..work_units {
            if self.result.is_some() || !frames.is_empty() {
                break;
            }
            if let Err(error) = self.unit(&mut frames) {
                self.release();
                self.failure = Some(error.clone());
                return Err(error);
            }
        }
        Ok(Step {
            done: self.result.is_some(),
            stage: self.stage(),
            fraction: self.fraction,
            frames,
        })
    }

    pub fn result(&self) -> Result<TrainedPassResult, EngineError> {
        self.check_failure()?;
        self.result.clone().ok_or_else(super::lost)
    }

    pub fn cancel(&mut self) {
        if self.failure.is_some() {
            return;
        }
        self.release();
        self.result = None;
        self.failure = Some(EngineError::new(
            ErrorCode::PassCancelled,
            "the side pass was cancelled",
        ));
    }

    fn check_failure(&self) -> Result<(), EngineError> {
        self.failure.clone().map_or(Ok(()), Err)
    }

    fn release(&mut self) {
        self.intake = None;
        self.resize = None;
        self.setup = None;
        self.picture = None;
        self.reading = None;
        self.maps = None;
    }

    fn stage(&self) -> Stage {
        self.stage
    }

    fn started(frames: &mut Vec<WatchFrame>, stage: Stage) {
        frames.push(WatchFrame::StageStarted {
            film: FilmKind::Side,
            stage,
        });
    }

    fn unit(&mut self, frames: &mut Vec<WatchFrame>) -> Result<(), EngineError> {
        self.stage = match self.phase {
            0..=4 => Stage::Evidence,
            5..=7 => Stage::Chain,
            8..=9 => Stage::Naming,
            _ => Stage::Confidence,
        };
        match self.phase {
            0 => {
                Self::started(frames, Stage::Evidence);
                self.phase = 1;
            }
            1 => {
                if self.intake.as_mut().ok_or_else(super::lost)?.advance()? {
                    let (picture, _) = self.intake.take().ok_or_else(super::lost)?.finish()?;
                    self.resize = Some(Resize::new(picture, self.mapping.letterbox)?);
                    self.phase = 2;
                    self.fraction = 0.1;
                }
            }
            2 => {
                if self.resize.as_mut().ok_or_else(super::lost)?.advance()? {
                    self.picture = Some(self.resize.take().ok_or_else(super::lost)?.finish()?);
                    self.setup = Some(WeightSetup::compiled()?);
                    self.phase = 3;
                    self.fraction = 0.2;
                }
            }
            3 => {
                let setup = self.setup.as_mut().ok_or_else(super::lost)?;
                let done = setup.advance()?;
                self.fraction =
                    0.2 + 0.1 * f64::from(setup.units_done()) / f64::from(WeightSetup::units());
                if done {
                    let net = Rc::new(self.setup.take().ok_or_else(super::lost)?.finish()?);
                    self.reading = Some(net.start(self.picture.take().ok_or_else(super::lost)?)?);
                    self.phase = 4;
                }
            }
            4 => {
                let reading = self.reading.as_mut().ok_or_else(super::lost)?;
                reading.advance(1)?;
                self.fraction =
                    0.3 + 0.7 * f64::from(reading.units_done()) / f64::from(reading.units());
                if reading.is_done() {
                    self.maps = Some(self.reading.take().ok_or_else(super::lost)?.into_maps()?);
                    self.phase = 5;
                }
            }
            5 => {
                Self::started(frames, Stage::Chain);
                self.phase = 6;
                self.fraction = 0.0;
            }
            6 => {
                let p = decode::channel(self.maps.as_ref().ok_or_else(super::lost)?, self.channel)?;
                self.points[self.channel] = self.mapping.to_film(p.map(f64::from));
                self.channel += 1;
                self.fraction = self.channel as f64 / 20.0;
                if self.channel == 20 {
                    self.maps = None;
                    self.phase = 7;
                }
            }
            7 => {
                let mut bodies = super::reserve(5)?;
                for (i, level) in to_marks::LEVELS.into_iter().enumerate() {
                    let [a, p, ia, ip] = std::array::from_fn(|j| Px(self.points[i * 4 + j]));
                    bodies.push(ChainBody {
                        level,
                        sup: Segment { a, b: p },
                        inf: Segment { a: ia, b: ip },
                        inferred: false,
                    });
                }
                frames.push(WatchFrame::ChainLocked {
                    film: FilmKind::Side,
                    stage: Stage::Chain,
                    bodies,
                });
                self.phase = 8;
            }
            8 => {
                Self::started(frames, Stage::Naming);
                self.phase = 9;
                self.fraction = 0.0;
            }
            9 => {
                frames.push(WatchFrame::LevelsNamed {
                    film: FilmKind::Side,
                    stage: Stage::Naming,
                    naming: crate::to_marks::estimated_naming(),
                    levels: to_marks::LEVELS.to_vec(),
                });
                self.phase = 10;
                self.fraction = 1.0;
            }
            10 => {
                Self::started(frames, Stage::Confidence);
                self.phase = 11;
                self.fraction = 0.0;
            }
            11 => {
                let marks = to_marks::film_marks(self.points, &self.film, self.first)?;
                frames.push(WatchFrame::Confidence {
                    film: FilmKind::Side,
                    stage: Stage::Confidence,
                    levels: marks
                        .vertebrae
                        .iter()
                        .map(|v| LevelConfidence {
                            level: v.level,
                            score: v.confidence.score,
                            band: v.confidence.band,
                        })
                        .collect(),
                });
                frames.push(WatchFrame::Outlines {
                    film: FilmKind::Side,
                    stage: Stage::Confidence,
                    quads: marks
                        .vertebrae
                        .iter()
                        .map(|v| OutlineQuad::new(v.level, &v.marks))
                        .collect(),
                    is_final: true,
                });
                self.result = Some(TrainedPassResult {
                    marks,
                    warnings: Vec::new(),
                });
                self.release();
                self.fraction = 1.0;
            }
            _ => return Err(super::lost()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::case_file_v1::CaseFileV1;
    use shapes::confidence::{ConfidenceBand, ConfidenceReason};
    use shapes::film::{FilmFacts, FilmImage, PixelValues, SourceFormat};
    use shapes::film_geometry::FilmGeometry;
    use shapes::marks::MarkSource;
    fn stored(w: u32, h: u32, bits: u8, pixels: Vec<u16>) -> DecodedFilm {
        let image = GreyImage::new(w, h, pixels, Placement::FILM).unwrap();
        let window = film_reader::thumbnail::default_window(&image, bits, &PixelValues::AS_STORED);
        DecodedFilm {
            image,
            facts: FilmFacts {
                kind: FilmKind::Side,
                width: w,
                height: h,
                bits_stored: bits,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: vec![],
                default_window: window,
                exif_quarter_turns: None,
            },
        }
    }

    fn record(decoded: &DecodedFilm) -> Film {
        let mut film = Film {
            kind: FilmKind::Side,
            image: FilmImage {
                path: FilmKind::Side.png_path().into(),
                width: decoded.image.width(),
                height: decoded.image.height(),
                bits_stored: decoded.facts.bits_stored,
            },
            source: decoded.facts.source,
            kept_tags: decoded.facts.kept_tags.clone(),
            values: decoded.facts.values.clone(),
            flags: vec![],
            blanked: vec![],
            geometry: FilmGeometry {
                image_right: Some(ImageRight::Posterior),
                ..FilmGeometry::UNKNOWN
            },
        };
        film.geometry.confirmed.facing = true;
        film
    }

    fn drawn() -> SidePass {
        let decoded = Rc::new(stored(384, 512, 8, vec![0; 384 * 512]));
        let film = record(&decoded);
        let mut pass = SidePass::from_record(decoded, film, 10).unwrap();
        let mut maps = Grid::zeros(20, 128, 96).unwrap();
        for channel in 0..20 {
            let y = 20 + channel / 4 * 15 + if channel % 4 >= 2 { 8 } else { 0 };
            let x = if channel % 2 == 0 { 30 } else { 50 };
            maps.row_mut(channel, y)[x] = 100.0;
        }
        pass.release();
        pass.maps = Some(maps);
        pass.phase = 5;
        pass
    }

    #[test]
    fn drawn_maps_give_the_same_frames_and_reviewable_marks_in_every_slice_size() {
        let mut answers = Vec::new();
        for units in [1, 7, u32::MAX] {
            let mut pass = drawn();
            assert!(!pass.advance(0).unwrap().done);
            assert_eq!(pass.result().unwrap_err().code, ErrorCode::EngineOutOfSync);
            let mut frames = Vec::new();
            loop {
                let step = pass.advance(units).unwrap();
                frames.extend(step.frames);
                if step.done {
                    break;
                }
            }
            let result = pass.result().unwrap();
            assert_eq!(pass.advance(5).unwrap().frames, []);
            assert_eq!(pass.result().unwrap(), result);
            assert!(pass.maps.is_none() && pass.intake.is_none());
            let marks = &result.marks;
            assert_eq!(marks.named_for, ImageRight::Posterior);
            assert_eq!(
                marks.vertebrae.iter().map(|v| v.id).collect::<Vec<_>>(),
                [10, 11, 12, 13, 14]
            );
            for v in &marks.vertebrae {
                assert_eq!(v.confidence.band, ConfidenceBand::Low);
                assert_eq!(v.confidence.reasons, [ConfidenceReason::LevelUncertain]);
                assert_eq!(v.confidence.score, 0.0);
                assert!(!v.confidence.calibrated);
                assert!(
                    v.marks
                        .corners()
                        .iter()
                        .all(|m| m.source == MarkSource::Auto && !m.pinned && !m.inferred)
                );
            }
            assert_eq!(
                marks.vertebrae[0].marks.corners().map(|m| m.at),
                [
                    Px::new(121.5, 81.5),
                    Px::new(201.5, 81.5),
                    Px::new(201.5, 113.5),
                    Px::new(121.5, 113.5)
                ]
            );
            let mut case = CaseFileV1::new("0.1.0");
            case.films.push(pass.film.clone());
            case.marks.push(marks.clone());
            case.next_vertebra_id = 15;
            let json = shapes::reading::write_case_json(&case);
            assert_eq!(shapes::reading::read_case_json(&json).unwrap(), case);
            answers.push((result, frames));
        }
        assert_eq!(answers[0], answers[1]);
        assert_eq!(answers[0], answers[2]);
    }

    #[test]
    fn cancellation_and_failure_release_work_and_remain_sticky() {
        let mut pass = drawn();
        pass.cancel();
        assert!(pass.maps.is_none());
        for units in [0, 1, 100] {
            assert_eq!(
                pass.advance(units).unwrap_err().code,
                ErrorCode::PassCancelled
            );
        }
        assert_eq!(pass.result().unwrap_err().code, ErrorCode::PassCancelled);
        let mut pass = drawn();
        pass.maps.as_mut().unwrap().row_mut(0, 0)[0] = f32::NAN;
        pass.advance(1).unwrap();
        let error = pass.advance(1).unwrap_err();
        pass.cancel();
        assert_eq!(pass.advance(0).unwrap_err(), error);
        assert_eq!(pass.result().unwrap_err(), error);
        assert!(pass.maps.is_none());
    }

    #[test]
    fn facing_is_explicit_and_bad_records_are_refused_before_work() {
        let decoded = Rc::new(stored(384, 512, 8, vec![0; 384 * 512]));
        let mut film = record(&decoded);
        film.geometry.confirmed.facing = false;
        assert_eq!(
            SidePass::from_record(decoded.clone(), film.clone(), 1)
                .err()
                .unwrap()
                .code,
            ErrorCode::PassFacingRequired
        );
        film.geometry.confirmed.facing = true;
        film.geometry.image_right = Some(ImageRight::PatientLeft);
        assert_eq!(
            SidePass::from_record(decoded.clone(), film.clone(), 1)
                .err()
                .unwrap()
                .code,
            ErrorCode::CaseInvalidValue
        );
        film.geometry.image_right = Some(ImageRight::Anterior);
        assert!(SidePass::from_record(decoded.clone(), film.clone(), 1).is_ok());
        assert_eq!(
            SidePass::from_record(decoded.clone(), film.clone(), u32::MAX)
                .err()
                .unwrap()
                .code,
            ErrorCode::CaseInvalidValue
        );
        if !super::super::available() {
            assert_eq!(
                SidePass::new(decoded, film, 1).err().unwrap().code,
                ErrorCode::SideFinderUnavailable
            );
        }
    }
}
