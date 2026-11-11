//! A bounded crop, deterministic seed precedence and contrast-normalised
//! four-neighbour conductances for the random walker. The optional phase
//! edge multiplier is deliberately off until measured against real films.

use super::{distance_map, geometry::*};
use film_reader::decoded_film::DecodedFilm;
use shapes::{
    errors::{EngineError, ErrorCode},
    pixels::Px,
    toolbox::{ToolWarning, WandClick, WandPolarity},
    watch_frames::{PreviewTile, TileEncoding},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Label {
    Background,
    Foreground,
    Unknown,
}
pub(super) struct Crop {
    pub width: usize,
    pub height: usize,
    pub origin: Px,
    pub factor: f64,
    pub old: Vec<bool>,
    pub free: Vec<bool>,
    pub labels: Vec<Label>,
    pub right: Vec<f32>,
    pub down: Vec<f32>,
    pub warnings: Vec<ToolWarning>,
}
impl Crop {
    pub fn point(&self, x: f64, y: f64) -> Px {
        add(self.origin, Px::new(x * self.factor, y * self.factor))
    }
    pub fn tile(&self, values: &[f32]) -> PreviewTile {
        PreviewTile {
            width: self.width as u32,
            height: self.height as u32,
            origin: self.origin,
            step_across: Px::new(self.factor, 0.),
            step_down: Px::new(0., self.factor),
            encoding: TileEncoding::Unit,
            data: values
                .iter()
                .map(|v| (255. * v.clamp(0., 1.)).round() as u8)
                .collect(),
        }
    }
}

pub(super) fn build(
    film: &DecodedFilm,
    q: &[Px; 4],
    neighbours: &[[Px; 4]],
    clicks: &[WandClick],
) -> Result<Crop, EngineError> {
    let h = height(q);
    let band = 0.20 * h;
    let radius = 0.6 * h.max(width(q));
    let mut lo = Px::new(
        q.iter().map(|p| p.x()).fold(f64::INFINITY, f64::min) - band,
        q.iter().map(|p| p.y()).fold(f64::INFINITY, f64::min) - band,
    );
    let mut hi = Px::new(
        q.iter().map(|p| p.x()).fold(f64::NEG_INFINITY, f64::max) + band,
        q.iter().map(|p| p.y()).fold(f64::NEG_INFINITY, f64::max) + band,
    );
    for c in clicks.iter().filter(|c| c.polarity == WandPolarity::Add) {
        lo = Px::new(lo.x().min(c.at.x() - radius), lo.y().min(c.at.y() - radius));
        hi = Px::new(hi.x().max(c.at.x() + radius), hi.y().max(c.at.y() + radius));
    }
    let initial = (h / 64.).ceil().max(1.);
    let factor = initial
        .max(((hi.x() - lo.x()).max(hi.y() - lo.y()) + 6. * initial + 1.).ceil() / 154.)
        .ceil();
    let x0 = (lo.x() - 3. * factor)
        .floor()
        .max(0.)
        .min(f64::from(film.image.width()));
    let y0 = (lo.y() - 3. * factor)
        .floor()
        .max(0.)
        .min(f64::from(film.image.height()));
    let x1 = (hi.x() + 3. * factor).min(f64::from(film.image.width()) - 1.);
    let y1 = (hi.y() + 3. * factor).min(f64::from(film.image.height()) - 1.);
    let w = (((x1 - x0 + 1.) / factor)
        .ceil()
        .min(((f64::from(film.image.width()) - x0) / factor).floor()))
    .max(0.) as usize;
    let ht = (((y1 - y0 + 1.) / factor)
        .ceil()
        .min(((f64::from(film.image.height()) - y0) / factor).floor()))
    .max(0.) as usize;
    if w < 3 || ht < 3 || w > 160 || ht > 160 {
        return Err(EngineError::new(
            ErrorCode::ToolRefitFailed,
            "the crop cannot hold this outline",
        ));
    }
    let origin = Px::new(x0 + 0.5 * factor - 0.5, y0 + 0.5 * factor - 0.5);
    let point = |x: usize, y: usize| add(origin, Px::new(x as f64 * factor, y as f64 * factor));
    let count = w * ht;
    let mut intensity = Vec::with_capacity(count);
    let mut old = Vec::with_capacity(count);
    let f = factor as usize;
    let stride = film.image.width() as usize;
    for y in 0..ht {
        for x in 0..w {
            // u64 keeps even the largest accepted film's complete blocks exact.
            let mut sum = 0u64;
            for dy in 0..f {
                let start = (y0 as usize + y * f + dy) * stride + x0 as usize + x * f;
                for &v in &film.image.pixels()[start..start + f] {
                    sum += u64::from(v);
                }
            }
            intensity.push(sum as f32 / (f * f) as f32);
            old.push(inside(point(x, y), q));
        }
    }
    let to_body = distance_map::squared(&old, w, ht);
    let to_outside = distance_map::squared(&old.iter().map(|v| !*v).collect::<Vec<_>>(), w, ht);
    let b = (band / factor).round().max(2.);
    let r = radius / factor;
    let seed = (0.04 * h / factor).round().max(1.);
    let seed_clicks: Vec<_> = clicks
        .iter()
        .map(|c| (scale(sub(c.at, origin), 1. / factor), c.polarity))
        .collect();
    let mut labels = vec![Label::Background; count];
    let mut free = vec![false; count];
    for y in 0..ht {
        for x in 0..w {
            let i = y * w + x;
            let p = Px::new(x as f64, y as f64);
            free[i] = to_body[i] <= b * b
                || seed_clicks
                    .iter()
                    .any(|(c, k)| *k == WandPolarity::Add && distance(*c, p) <= r);
            if free[i] {
                labels[i] = Label::Unknown;
            }
            if old[i] && to_outside[i] >= b * b {
                labels[i] = Label::Foreground;
            }
            if labels[i] == Label::Foreground
                && seed_clicks
                    .iter()
                    .any(|(c, k)| *k == WandPolarity::Subtract && distance(*c, p) <= r)
            {
                labels[i] = Label::Unknown;
            }
        }
    }
    let erosion = (0.05 * h / factor).round().max(1.);
    for neighbour in neighbours {
        let mask: Vec<_> = (0..count)
            .map(|i| inside(point(i % w, i / w), neighbour))
            .collect();
        let distances = distance_map::squared(&mask.iter().map(|v| !*v).collect::<Vec<_>>(), w, ht);
        for i in 0..count {
            if mask[i] && distances[i] >= erosion * erosion {
                labels[i] = Label::Background;
            }
        }
    }
    for polarity in [WandPolarity::Add, WandPolarity::Subtract] {
        for y in 0..ht {
            for x in 0..w {
                let p = Px::new(x as f64, y as f64);
                if seed_clicks
                    .iter()
                    .any(|(c, k)| *k == polarity && distance(*c, p) <= seed)
                {
                    labels[y * w + x] = if polarity == WandPolarity::Add {
                        Label::Foreground
                    } else {
                        Label::Background
                    };
                }
            }
        }
    }
    for y in 0..ht {
        for x in 0..w {
            if x == 0 || y == 0 || x + 1 == w || y + 1 == ht {
                labels[y * w + x] = Label::Background;
            }
        }
    }
    if !labels.contains(&Label::Foreground) {
        return Err(EngineError::new(
            ErrorCode::ToolWouldRemoveVertebra,
            "the click would remove the whole vertebra",
        ));
    }
    let mut sorted: Vec<_> = intensity
        .iter()
        .zip(&free)
        .filter(|(_, f)| **f)
        .map(|(v, _)| *v)
        .collect();
    sorted.sort_by(f32::total_cmp);
    if sorted.is_empty() {
        return Err(EngineError::new(
            ErrorCode::ToolNoContrast,
            "the crop has no usable pixels",
        ));
    }
    let percentile = |p: f64| {
        sorted[((p * sorted.len() as f64).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1)]
    };
    let low = percentile(0.02);
    let high = percentile(0.98);
    let spread = high - low;
    if spread < 4. {
        return Err(EngineError::new(
            ErrorCode::ToolNoContrast,
            "the local intensity range is less than four stored units",
        ));
    }
    let saturation = sorted
        .iter()
        .filter(|&&v| v > high + 0.5 * spread || v < low - 0.5 * spread)
        .count() as f64
        / sorted.len() as f64;
    let mut warnings = Vec::new();
    if saturation > 0.02 {
        warnings.push(ToolWarning::BrightOrDarkObject {
            fraction: saturation,
        });
    }
    let grey: Vec<_> = intensity
        .iter()
        .map(|v| ((v - low) / spread).clamp(0., 1.))
        .collect();
    let mean = grey.iter().map(|v| f64::from(*v)).sum::<f64>() / count as f64;
    let sigma = (grey
        .iter()
        .map(|v| {
            let d = f64::from(*v) - mean;
            d * d
        })
        .sum::<f64>()
        / count as f64)
        .sqrt() as f32;
    if sigma < 1e-3 {
        return Err(EngineError::new(
            ErrorCode::ToolNoContrast,
            "the crop has no contrast",
        ));
    }
    let weight = |i: usize, j: usize| {
        let d = grey[i] - grey[j];
        libm::expf((-130. * d * d / (10. * sigma)).max(-30.)) + 1e-6
    };
    let mut right = vec![0.; count];
    let mut down = vec![0.; count];
    for y in 0..ht {
        for x in 0..w {
            let i = y * w + x;
            if x + 1 < w {
                right[i] = weight(i, i + 1);
            }
            if y + 1 < ht {
                down[i] = weight(i, i + w);
            }
        }
    }
    Ok(Crop {
        width: w,
        height: ht,
        origin,
        factor,
        old,
        free,
        labels,
        right,
        down,
        warnings,
    })
}
