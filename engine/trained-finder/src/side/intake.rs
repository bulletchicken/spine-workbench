//! Banded opening greys, record blanking, upright turns and canonical facing.

use super::resize::Letterbox;
use super::{lost, reserve};
use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{DisplayWindow, Film, ModalityMap, WindowFunction, WindowSource};
use shapes::film_geometry::ImageRight;
use std::rc::Rc;

const BAND: usize = 65_536;

/// The exact stored-centre to model-centre transform, and its inverse.
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub letterbox: Letterbox,
    width: usize,
    height: usize,
    turns: u8,
    flip: bool,
}

impl Mapping {
    pub fn new(
        width: usize,
        height: usize,
        turns: u8,
        facing: ImageRight,
    ) -> Result<Self, EngineError> {
        if turns > 3 || !matches!(facing, ImageRight::Anterior | ImageRight::Posterior) {
            return Err(EngineError::new(
                ErrorCode::CaseInvalidValue,
                "the side picture needs an upright anterior or posterior facing",
            ));
        }
        let (w, h) = if turns.is_multiple_of(2) {
            (width, height)
        } else {
            (height, width)
        };
        Ok(Self {
            letterbox: Letterbox::new(w, h, 384, 512)?,
            width,
            height,
            turns,
            flip: facing == ImageRight::Anterior,
        })
    }

    pub fn to_model(&self, p: [f64; 2]) -> [f64; 2] {
        let [x, y] = p;
        let w = self.width as f64;
        let h = self.height as f64;
        let [mut x, y] = match self.turns {
            0 => [x, y],
            1 => [h - 1.0 - y, x],
            2 => [w - 1.0 - x, h - 1.0 - y],
            _ => [y, w - 1.0 - x],
        };
        if self.flip {
            x = self.letterbox.width as f64 - 1.0 - x;
        }
        self.letterbox.to_model([x, y])
    }

    pub fn to_film(&self, p: [f64; 2]) -> [f64; 2] {
        let [mut x, y] = self.letterbox.from_model(p);
        if self.flip {
            x = self.letterbox.width as f64 - 1.0 - x;
        }
        let w = self.width as f64;
        let h = self.height as f64;
        match self.turns {
            0 => [x, y],
            1 => [y, h - 1.0 - x],
            2 => [w - 1.0 - x, h - 1.0 - y],
            _ => [w - 1.0 - y, x],
        }
    }

    fn stored_index(&self, index: usize) -> usize {
        let mut x = index % self.letterbox.width;
        let y = index / self.letterbox.width;
        if self.flip {
            x = self.letterbox.width - 1 - x;
        }
        let (x, y) = match self.turns {
            0 => (x, y),
            1 => (y, self.height - 1 - x),
            2 => (self.width - 1 - x, self.height - 1 - y),
            _ => (self.width - 1 - y, x),
        };
        y * self.width + x
    }
}

#[derive(Clone, Copy)]
enum Stage {
    HistogramStart,
    Histogram,
    Window,
    Lookup,
    Pixels,
    Blank,
    Turn,
    Done,
}

/// Intake retains the decoded film by Rc and never changes its pixels.
pub struct SideIntake {
    decoded: Rc<DecodedFilm>,
    film: Film,
    pub mapping: Mapping,
    stage: Stage,
    histogram: Vec<u32>,
    cursor: usize,
    view_film: Option<DecodedFilm>,
    levels: Vec<u8>,
    pixels: Vec<u8>,
    turned: Vec<u8>,
    blank: usize,
    blank_cursor: usize,
    last_work: usize,
}

impl SideIntake {
    /// Validates the record but does no image work or weight setup.
    pub fn new(decoded: Rc<DecodedFilm>, film: Film) -> Result<Self, EngineError> {
        let (w, h) = (decoded.image.width(), decoded.image.height());
        if decoded.facts.kind != film.kind
            || decoded.facts.width != w
            || decoded.facts.height != h
            || film.image.width != w
            || film.image.height != h
        {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the side film record does not match the decoded pixels",
            ));
        }
        if film.blanked.iter().any(|b| !b.is_on_film(w, h)) {
            return Err(EngineError::new(
                ErrorCode::CaseInvalidValue,
                "a blanked region lies outside its film",
            ));
        }
        let mapping = Mapping::new(
            w as usize,
            h as usize,
            film.geometry.quarter_turns.get(),
            film.geometry.image_right.ok_or_else(|| {
                EngineError::new(
                    ErrorCode::PassFacingRequired,
                    "choose which way the patient faces",
                )
            })?,
        )?;
        let sigmoid = decoded.facts.bits_stored != 8
            && matches!(
                decoded.facts.default_window,
                DisplayWindow::CentreWidth {
                    function: WindowFunction::Sigmoid,
                    ..
                }
            );
        Ok(Self {
            decoded,
            film,
            mapping,
            stage: if sigmoid {
                Stage::HistogramStart
            } else {
                Stage::Lookup
            },
            histogram: Vec::new(),
            cursor: 0,
            view_film: None,
            levels: Vec::new(),
            pixels: Vec::new(),
            turned: Vec::new(),
            blank: 0,
            blank_cursor: 0,
            last_work: 0,
        })
    }

    pub fn is_done(&self) -> bool {
        matches!(self.stage, Stage::Done)
    }
    pub fn last_work(&self) -> usize {
        self.last_work
    }

    pub fn advance(&mut self) -> Result<bool, EngineError> {
        self.last_work = 0;
        let w = self.mapping.width;
        let h = self.mapping.height;
        match self.stage {
            Stage::HistogramStart => {
                self.histogram = reserve(65_536)?;
                self.histogram.resize(65_536, 0);
                self.last_work = 65_536;
                self.stage = Stage::Histogram;
            }
            Stage::Histogram => {
                let left = w / 5;
                let right = (w * 4 / 5).max(left + 1).min(w);
                let top = h / 10;
                let bottom = (h * 9 / 10).max(top + 1).min(h);
                let count = (right - left) * (bottom - top);
                let end = (self.cursor + BAND).min(count);
                for i in self.cursor..end {
                    let x = left + i % (right - left);
                    let y = top + i / (right - left);
                    self.histogram[usize::from(self.decoded.image.pixels()[y * w + x])] += 1;
                }
                self.last_work = end - self.cursor;
                self.cursor = end;
                if end == count {
                    self.cursor = 0;
                    self.stage = Stage::Window;
                }
            }
            Stage::Window => {
                let left = w / 5;
                let right = (w * 4 / 5).max(left + 1).min(w);
                let top = h / 10;
                let bottom = (h * 9 / 10).max(top + 1).min(h);
                let count = ((right - left) * (bottom - top)) as u64;
                let low_rank = count.div_ceil(100);
                let high_rank = (count * 99).div_ceil(100);
                let mut through = 0u64;
                let mut low = None;
                let mut high = 0;
                for (value, &n) in self.histogram.iter().enumerate() {
                    through += u64::from(n);
                    if low.is_none() && through >= low_rank {
                        low = Some(value);
                    }
                    if through >= high_rank {
                        high = value;
                        break;
                    }
                }
                let first = self.modality(low.ok_or_else(lost)? as f64);
                let last = self.modality(high as f64);
                let mut facts = self.decoded.facts.clone();
                facts.default_window = DisplayWindow::CentreWidth {
                    centre: first / 2.0 + last / 2.0,
                    width: (last - first).abs().max(1.0),
                    function: WindowFunction::LinearExact,
                    from: WindowSource::Percentile,
                };
                self.view_film = Some(DecodedFilm {
                    facts,
                    image: GreyImage::new(1, 1, vec![0], Placement::FILM).ok_or_else(lost)?,
                });
                self.histogram = Vec::new();
                self.last_work = BAND;
                self.stage = Stage::Lookup;
            }
            Stage::Lookup => {
                if self.decoded.facts.bits_stored != 8 {
                    let count = 1usize << self.decoded.facts.bits_stored.clamp(1, 16);
                    if self.levels.is_empty() {
                        self.levels = reserve(count)?;
                    }
                    let view = film_reader::thumbnail::opening_view(
                        self.view_film.as_ref().unwrap_or(&self.decoded),
                    );
                    let end = (self.levels.len() + BAND).min(count);
                    self.last_work = end - self.levels.len();
                    for value in self.levels.len()..end {
                        let grey = (view(value as f64) * 255.0).round() as u8;
                        self.levels
                            .push(if self.decoded.facts.values.inverted_by_user {
                                255 - grey
                            } else {
                                grey
                            });
                    }
                }
                self.view_film = None;
                self.stage = Stage::Pixels;
            }
            Stage::Pixels => {
                if self.pixels.is_empty() {
                    self.pixels = reserve(w * h)?;
                }
                let end = (self.pixels.len() + BAND).min(w * h);
                self.last_work = end - self.pixels.len();
                let flipped = self.film.values.inverted != self.decoded.facts.values.inverted;
                for i in self.pixels.len()..end {
                    let value = self.decoded.image.pixels()[i];
                    let byte = if self.decoded.facts.bits_stored == 8 {
                        value as u8
                    } else {
                        self.levels[usize::from(value).min(self.levels.len() - 1)]
                    };
                    self.pixels.push(if flipped { 255 - byte } else { byte });
                }
                if end == w * h {
                    self.levels = Vec::new();
                    self.stage = Stage::Blank;
                }
            }
            Stage::Blank => {
                if let Some(area) = self.film.blanked.get(self.blank) {
                    let left = (area.left + 0.5).floor().max(0.0) as usize;
                    let right = (area.right + 0.5).ceil().min(w as f64) as usize;
                    let top = (area.top + 0.5).floor().max(0.0) as usize;
                    let bottom = (area.bottom + 0.5).ceil().min(h as f64) as usize;
                    let width = right.saturating_sub(left);
                    let count = width * bottom.saturating_sub(top);
                    let end = (self.blank_cursor + BAND).min(count);
                    for i in self.blank_cursor..end {
                        let x = left + i % width;
                        let y = top + i / width;
                        if area.covers_pixel(x as u32, y as u32) {
                            self.pixels[y * w + x] = 0;
                        }
                    }
                    self.last_work = end - self.blank_cursor;
                    self.blank_cursor = end;
                    if end == count {
                        self.blank += 1;
                        self.blank_cursor = 0;
                    }
                } else {
                    self.stage = Stage::Turn;
                }
            }
            Stage::Turn => {
                if self.turned.is_empty() {
                    self.turned = reserve(w * h)?;
                }
                let end = (self.turned.len() + BAND).min(w * h);
                self.last_work = end - self.turned.len();
                for i in self.turned.len()..end {
                    self.turned.push(self.pixels[self.mapping.stored_index(i)]);
                }
                if end == w * h {
                    self.pixels = Vec::new();
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => {}
        }
        Ok(self.is_done())
    }

    fn modality(&self, pixel: f64) -> f64 {
        let values = &self.decoded.facts.values;
        let maximum = ((1u32 << self.decoded.facts.bits_stored.clamp(1, 16)) - 1) as f64;
        let stored = f64::from(values.offset)
            + if values.inverted {
                maximum - pixel
            } else {
                pixel
            };
        match &values.modality {
            ModalityMap::Linear { slope, intercept } => slope * stored + intercept,
            ModalityMap::Lookup {
                first_mapped,
                table,
                ..
            } => {
                if table.is_empty() {
                    0.0
                } else {
                    f64::from(
                        table[((stored - f64::from(*first_mapped)).max(0.0) as usize)
                            .min(table.len() - 1)],
                    )
                }
            }
        }
    }

    pub fn finish(self) -> Result<(Vec<u8>, Mapping), EngineError> {
        if self.is_done() {
            Ok((self.turned, self.mapping))
        } else {
            Err(lost())
        }
    }
}
