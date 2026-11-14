//! Byte-preserving triangle filtering and pixel-centre letterboxing.

use super::{lost, reserve};
use shapes::errors::{EngineError, ErrorCode};

const PIXELS: usize = 65_536;
const TAPS: usize = 1 << 20;

/// Independently rounded dimensions and the actual centre scales.
#[derive(Clone, Copy, Debug)]
pub struct Letterbox {
    pub width: usize,
    pub height: usize,
    pub resized_width: usize,
    pub resized_height: usize,
    pub left: usize,
    pub top: usize,
    pub sx: f64,
    pub sy: f64,
    pub target_width: usize,
    pub target_height: usize,
}

impl Letterbox {
    pub fn new(
        width: usize,
        height: usize,
        target_width: usize,
        target_height: usize,
    ) -> Result<Self, EngineError> {
        if [width, height, target_width, target_height]
            .iter()
            .any(|&n| n == 0 || n > shapes::film::MAX_FILM_SIDE as usize)
        {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "invalid side picture dimensions",
            ));
        }
        let ratio = (target_width as f64 / width as f64).min(target_height as f64 / height as f64);
        let nw = ((width as f64 * ratio).round_ties_even() as usize).max(1);
        let nh = ((height as f64 * ratio).round_ties_even() as usize).max(1);
        Ok(Self {
            width,
            height,
            resized_width: nw,
            resized_height: nh,
            left: (target_width - nw) / 2,
            top: (target_height - nh) / 2,
            sx: nw as f64 / width as f64,
            sy: nh as f64 / height as f64,
            target_width,
            target_height,
        })
    }

    pub fn to_model(&self, p: [f64; 2]) -> [f64; 2] {
        [
            (p[0] + 0.5) * self.sx - 0.5 + self.left as f64,
            (p[1] + 0.5) * self.sy - 0.5 + self.top as f64,
        ]
    }
    pub fn from_model(&self, p: [f64; 2]) -> [f64; 2] {
        [
            (p[0] - self.left as f64 + 0.5) / self.sx - 0.5,
            (p[1] - self.top as f64 + 0.5) / self.sy - 0.5,
        ]
    }
}

struct Filter {
    first: usize,
    weights: Vec<i32>,
}

fn filters(input: usize, output: usize) -> Result<Vec<Filter>, EngineError> {
    let mut filters = reserve(output)?;
    let scale = input as f64 / output as f64;
    let support = scale.max(1.0);
    let step = 1.0 / support;
    for index in 0..output {
        let centre = (index as f64 + 0.5) * scale;
        let first = ((centre - support + 0.5) as i64).max(0) as usize;
        let last = ((centre + support + 0.5) as i64).min(input as i64) as usize;
        let mut raw = reserve(last - first)?;
        for at in first..last {
            let t = ((at as f64 - centre + 0.5) * step).abs();
            raw.push(if t < 1.0 { 1.0 - t } else { 0.0 });
        }
        let total = raw.iter().fold(0.0, |sum, &w| sum + w);
        let mut weights = reserve(raw.len())?;
        for w in raw {
            let w = if total != 0.0 { w / total } else { w };
            let scaled = w * f64::from(1u32 << 22);
            weights.push(if w < 0.0 {
                (-0.5 + scaled) as i32
            } else {
                (0.5 + scaled) as i32
            });
        }
        filters.push(Filter { first, weights });
    }
    Ok(filters)
}

/// An incremental resize. Each unit writes at most 65,536 pixels and
/// reads at most 1,048,576 filter taps, even on a very wide shrink row.
pub struct Resize {
    pub letterbox: Letterbox,
    pixels: Vec<u8>,
    out: Vec<u8>,
    across: Vec<Filter>,
    down: Vec<Filter>,
    across_first: bool,
    stage: u8,
    width: usize,
    height: usize,
    last_pixels: usize,
    last_taps: usize,
}

impl Resize {
    pub fn new(pixels: Vec<u8>, letterbox: Letterbox) -> Result<Self, EngineError> {
        if pixels.len() != letterbox.width * letterbox.height {
            return Err(lost());
        }
        Ok(Self {
            letterbox,
            pixels,
            out: Vec::new(),
            across: Vec::new(),
            down: Vec::new(),
            across_first: letterbox.resized_width * letterbox.height
                <= letterbox.width * letterbox.resized_height,
            stage: 0,
            width: letterbox.width,
            height: letterbox.height,
            last_pixels: 0,
            last_taps: 0,
        })
    }

    pub fn is_done(&self) -> bool {
        self.stage == 5
    }
    pub fn last_work(&self) -> (usize, usize) {
        (self.last_pixels, self.last_taps)
    }

    pub fn advance(&mut self) -> Result<bool, EngineError> {
        self.last_pixels = 0;
        self.last_taps = 0;
        match self.stage {
            0 | 1 => {
                let horizontal = self.stage == 0;
                let (input, output) = if horizontal {
                    (self.width, self.letterbox.resized_width)
                } else {
                    (self.height, self.letterbox.resized_height)
                };
                let coefficients = if input == output {
                    Vec::new()
                } else {
                    filters(input, output)?
                };
                // Three passes over each coefficient: make, sum, normalise.
                self.last_taps = 3 * coefficients.iter().map(|f| f.weights.len()).sum::<usize>();
                if horizontal {
                    self.across = coefficients;
                } else {
                    self.down = coefficients;
                }
                self.stage += 1;
            }
            2 | 3 => self.filter()?,
            4 => {
                let b = self.letterbox;
                if self.out.is_empty() {
                    self.out = reserve(b.target_width * b.target_height)?;
                }
                let end = (self.out.len() + PIXELS).min(b.target_width * b.target_height);
                for index in self.out.len()..end {
                    let x = index % b.target_width;
                    let y = index / b.target_width;
                    let value = if x >= b.left
                        && x < b.left + b.resized_width
                        && y >= b.top
                        && y < b.top + b.resized_height
                    {
                        self.pixels[(y - b.top) * b.resized_width + x - b.left]
                    } else {
                        0
                    };
                    self.out.push(value);
                    self.last_pixels += 1;
                }
                if end == b.target_width * b.target_height {
                    self.pixels = Vec::new();
                    self.stage = 5;
                }
            }
            _ => {}
        }
        debug_assert!(self.last_pixels <= PIXELS && self.last_taps <= TAPS);
        Ok(self.is_done())
    }

    fn filter(&mut self) -> Result<(), EngineError> {
        let horizontal = (self.stage == 2) == self.across_first;
        let b = self.letterbox;
        let (width, height) = if horizontal {
            (b.resized_width, self.height)
        } else {
            (self.width, b.resized_height)
        };
        if (width, height) == (self.width, self.height) {
            self.stage += 1;
            return Ok(());
        }
        if self.out.is_empty() {
            self.out = reserve(width * height)?;
        }
        while self.out.len() < width * height && self.last_pixels < PIXELS {
            let index = self.out.len();
            let x = index % width;
            let y = index / width;
            let filter = if horizontal {
                &self.across[x]
            } else {
                &self.down[y]
            };
            if self.last_taps + filter.weights.len() > TAPS {
                break;
            }
            let mut total = 1i64 << 21;
            for (tap, &weight) in filter.weights.iter().enumerate() {
                let index = if horizontal {
                    y * self.width + filter.first + tap
                } else {
                    (filter.first + tap) * self.width + x
                };
                total += i64::from(self.pixels[index]) * i64::from(weight);
            }
            self.out.push((total >> 22).clamp(0, 255) as u8);
            self.last_pixels += 1;
            self.last_taps += filter.weights.len();
        }
        if self.out.len() == width * height {
            self.pixels = std::mem::take(&mut self.out);
            self.width = width;
            self.height = height;
            self.stage += 1;
            if horizontal {
                self.across = Vec::new();
            } else {
                self.down = Vec::new();
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<Vec<u8>, EngineError> {
        if self.is_done() {
            Ok(self.out)
        } else {
            Err(lost())
        }
    }
}
