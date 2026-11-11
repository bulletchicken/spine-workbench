//! Spine-scale ridges and the bounded cost of crossing each upright cell.

use crate::handoffs::FilmEvidence;
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::hessian::{Hessian, normalised_gradient};
use darkroom::pyramid::shrink;
use darkroom::sampling::bilinear;
use shapes::auto_pass::{PassWarning, PassWarningCode};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::film_geometry::QuarterTurns;
use shapes::pixels::{Px, PxRect};

use super::smooth::Point;

/// The image and scalar cost searched by the sixteen-neighbour graph.
pub(crate) struct CostMap {
    pub image: GreyImage<f32>,
    pub fine: GreyImage<f32>,
    pub spacing: f64,
    pub ridge: Vec<f32>,
    pub sigma: Vec<f32>,
    pub along: Vec<Point>,
    pub cost: Vec<f32>,
    pub top: Vec<usize>,
    pub bottom: Vec<usize>,
    pub top_cropped: bool,
    pub bottom_cropped: bool,
    pub neck_y: Option<f64>,
    pub crest_y: Option<f64>,
    pub size_factor: f64,
    pub warnings: Vec<PassWarning>,
}

pub(crate) fn percentile(values: &[f32], fraction: f64) -> f32 {
    let mut values: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if values.is_empty() {
        return 0.0;
    }
    let i = ((values.len() - 1) as f64 * fraction).round() as usize;
    *values.select_nth_unstable_by(i, f32::total_cmp).1
}

pub(crate) fn warning(code: PassWarningCode, film: FilmKind) -> PassWarning {
    PassWarning {
        code,
        film,
        level: None,
        region: None,
    }
}

fn turn(image: &GreyImage<f32>, turns: u8) -> GreyImage<f32> {
    let (w, h) = (image.width(), image.height());
    let (ow, oh) = if turns.is_multiple_of(2) {
        (w, h)
    } else {
        (h, w)
    };
    let map = |x: f64, y: f64| match turns {
        1 => (y, f64::from(h - 1) - x),
        2 => (f64::from(w - 1) - x, f64::from(h - 1) - y),
        3 => (f64::from(w - 1) - y, x),
        _ => (x, y),
    };
    let mut pixels = Vec::with_capacity((ow * oh) as usize);
    for y in 0..oh {
        for x in 0..ow {
            let (sx, sy) = map(f64::from(x), f64::from(y));
            pixels.push(
                *image
                    .get(sx as u32, sy as u32)
                    .expect("permutation is in bounds"),
            );
        }
    }
    let (ox, oy) = map(0.0, 0.0);
    let (ax, ay) = map(1.0, 0.0);
    let (dx, dy) = map(0.0, 1.0);
    let origin = image.film_point(ox, oy);
    let across = image.film_point(ax, ay);
    let down = image.film_point(dx, dy);
    GreyImage::new(
        ow,
        oh,
        pixels,
        Placement {
            origin,
            step_across: Px::new(across.x() - origin.x(), across.y() - origin.y()),
            step_down: Px::new(down.x() - origin.x(), down.y() - origin.y()),
        },
    )
    .expect("permutation keeps the pixel count")
}

fn upright_turn(evidence: &FilmEvidence, turns: QuarterTurns) -> u8 {
    let across = evidence.image.placement().step_across;
    let current = if across.x().abs() >= across.y().abs() {
        if across.x() >= 0.0 { 0 } else { 2 }
    } else if across.y() < 0.0 {
        1
    } else {
        3
    };
    (turns.get() + 4 - current) % 4
}

fn profile_blur(values: &[f32], sigma: f64) -> Vec<f32> {
    let grid =
        GreyImage::new(1, values.len() as u32, values.to_vec(), Placement::FILM).expect("column");
    darkroom::blur::gaussian(&grid, sigma)
        .expect("finite positive sigma")
        .pixels()
        .to_vec()
}

impl CostMap {
    /// A bounded image grid (at least 2 units/cell), with the fine image
    /// retained at at least 1 unit/cell for normal-profile refinement.
    pub fn build(evidence: &FilmEvidence, turns: QuarterTurns) -> Result<Self, EngineError> {
        let placement = evidence.image.placement();
        let placement_finite = [placement.origin, placement.step_across, placement.step_down]
            .iter()
            .all(|p| p.x().is_finite() && p.y().is_finite());
        if evidence.valid.width() != evidence.image.width()
            || evidence.valid.height() != evidence.image.height()
            || evidence.valid.placement() != placement
            || !placement_finite
            || placement.image_point(placement.origin).is_none()
            || evidence.image.pixels().iter().any(|v| !v.is_finite())
            || evidence.valid.pixels().iter().any(|&v| v > 1)
        {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the image and validity grids do not describe one finite film",
            ));
        }
        let mm = evidence.mm_per_px;
        if !mm.is_finite() || mm <= 0.0 || evidence.image.width() < 8 || evidence.image.height() < 8
        {
            return Err(EngineError::new(
                ErrorCode::PassFilmTooSmall,
                "the evidence grid cannot be traced",
            ));
        }
        let q = upright_turn(evidence, turns);
        let fine_factor = (1.0 / mm).max(1.0);
        let coarse_factor = (2.0 / mm).max(1.0);
        let fine = turn(
            &shrink(&evidence.image, fine_factor, fine_factor).expect("positive shrink"),
            q,
        );
        let image = turn(
            &shrink(&evidence.image, coarse_factor, coarse_factor).expect("positive shrink"),
            q,
        );
        let mask = turn(
            &shrink(&evidence.valid, coarse_factor, coarse_factor).expect("positive shrink"),
            q,
        );
        let spacing = mm * coarse_factor;
        let (w, h) = (image.width() as usize, image.height() as usize);
        if w < 8 || h < 12 {
            return Err(EngineError::new(
                ErrorCode::PassFilmTooSmall,
                "the upright grid is too short",
            ));
        }
        let n = w * h;
        let p10 = percentile(image.pixels(), 0.10);
        let p90 = percentile(image.pixels(), 0.90);
        if p90 - p10 < 1e-5 {
            return Err(EngineError::new(
                ErrorCode::PassNoSpineFound,
                "the upright image has no ridge contrast",
            ));
        }
        let threshold = p10 + 0.12 * (p90 - p10);
        let mut left = vec![0usize; h];
        let mut right = vec![w - 1; h];
        let mut widths = vec![0.0; h];
        for y in 0..h {
            let row = &image.pixels()[y * w..(y + 1) * w];
            let l = row.iter().position(|&v| v > threshold);
            let r = row.iter().rposition(|&v| v > threshold);
            if let (Some(l), Some(r)) = (l, r) {
                left[y] = l;
                right[y] = r;
                widths[y] = ((r - l + 1) as f64 * spacing) as f32;
            }
        }
        let widths = profile_blur(&widths, 10.0 / spacing);
        let field_top = (0..h)
            .find(|&y| mask.pixels()[y * w..(y + 1) * w].iter().any(|&v| v > 0.5))
            .unwrap_or(0);
        let field_bottom = (0..h)
            .rev()
            .find(|&y| mask.pixels()[y * w..(y + 1) * w].iter().any(|&v| v > 0.5))
            .unwrap_or(h - 1);
        let body_fills = left.iter().filter(|&&x| x < 2).count() > h / 2
            && right.iter().filter(|&&x| x + 2 >= w).count() > h / 2;
        let upper_end = (field_top + (field_bottom - field_top) * 2 / 5).min(h - 1);
        let max_upper = widths[field_top..=upper_end]
            .iter()
            .copied()
            .fold(0.0f32, f32::max);
        let top_cropped = body_fills || widths[field_top] >= 0.6 * max_upper;
        let neck_y = if top_cropped {
            None
        } else {
            let neck_end = field_top + (field_bottom - field_top) * 3 / 10;
            let neck = (field_top..=neck_end)
                .min_by(|&a, &b| widths[a].total_cmp(&widths[b]))
                .unwrap_or(field_top);
            let shoulder = (neck..=upper_end)
                .find(|&y| widths[y] >= 0.85 * max_upper)
                .unwrap_or(upper_end);
            (neck + 1..=shoulder)
                .max_by(|&a, &b| {
                    (widths[a] - widths[a - 1]).total_cmp(&(widths[b] - widths[b - 1]))
                })
                .map(|y| y as f64 * spacing)
        };
        let q70 = percentile(image.pixels(), 0.70);
        let front = evidence.film != FilmKind::Side;
        let mut mass = vec![0.0f32; h];
        for y in 0..h {
            let middle = (left[y] + right[y]) as f64 * 0.5;
            for x in left[y]..=right[y] {
                let offset = (x as f64 - middle).abs() * spacing;
                if !front || (40.0..=150.0).contains(&offset) {
                    mass[y] += (image.pixels()[y * w + x] - q70).max(0.0);
                }
            }
        }
        let mass = profile_blur(&mass, 15.0 / spacing);
        let lower_start = field_top + (field_bottom - field_top) * 45 / 100;
        let lower_end = field_top + (field_bottom - field_top) * 85 / 100;
        let low = (lower_start..=lower_end)
            .min_by(|&a, &b| mass[a].total_cmp(&mass[b]))
            .unwrap_or(lower_start);
        let high = mass[low..=field_bottom]
            .iter()
            .copied()
            .fold(0.0f32, f32::max);
        let crest_y = if high > 1e-5 && high - mass[low] >= 0.25 * high {
            (low..=field_bottom)
                .find(|&y| mass[y] >= mass[low] + 0.5 * (high - mass[low]))
                .map(|y| y as f64 * spacing)
        } else {
            None
        };
        let size_factor = match (neck_y, crest_y) {
            (Some(neck), Some(crest)) => ((crest - neck) / 470.0).clamp(0.45, 1.2),
            _ => 1.0,
        };
        // End hints are deliberately not a hard crop: a shoulder edge or
        // an early bone-mass rise can imitate the neck or iliac crest.
        // Retain the complete usable field so that such a false hint does
        // not remove vertebrae from the later chain search.
        let start_row = field_top;
        let end_row = field_bottom;
        let requested_end =
            crest_y.map(|crest| crest + 0.25 * (crest - neck_y.unwrap_or(crest - 470.0)));
        let bottom_cropped = requested_end.is_none()
            || requested_end.is_some_and(|end| end > field_bottom as f64 * spacing + 5.0);
        let zone = |row: usize, fraction: f64| -> Vec<usize> {
            let middle = (left[row] + right[row]) as f64 * 0.5;
            let half = (right[row] - left[row]) as f64 * 0.5 * fraction;
            let a = (middle - half).ceil() as usize;
            let b = (middle + half).floor() as usize;
            (a..=b)
                .filter(|&x| mask.pixels()[row * w + x] > 0.5)
                .map(|x| row * w + x)
                .collect()
        };
        let top = (start_row..end_row)
            .map(|row| zone(row, if front { 0.6 } else { 0.8 }))
            .find(|cells| !cells.is_empty())
            .unwrap_or_default();
        let bottom = (start_row + 1..=end_row)
            .rev()
            .map(|row| zone(row, if front { 0.5 } else { 0.8 }))
            .find(|cells| !cells.is_empty())
            .unwrap_or_default();
        let mut warnings = Vec::new();
        if neck_y.is_none() || crest_y.is_none() {
            warnings.push(warning(PassWarningCode::SizeUnknown, evidence.film));
        }
        if top_cropped {
            warnings.push(warning(PassWarningCode::TopCropped, evidence.film));
        }
        if bottom_cropped {
            warnings.push(warning(PassWarningCode::BottomCropped, evidence.film));
        }
        let mut ridge = vec![0.0f32; n];
        let mut sigma = vec![12.0f32; n];
        let mut along = vec![Point { x: 0.0, y: 1.0 }; n];
        let scales = if front {
            [7.0, 8.75, 10.9, 13.7, 17.1, 21.4, 26.7]
        } else {
            [6.0, 7.5, 9.4, 11.7, 14.6, 18.3, 22.9]
        };
        let scale_factor = size_factor.clamp(0.6, 1.1);
        let broad = shrink(&image, 2.0, 2.0).expect("positive shrink");
        for scale in scales {
            let scale = scale * scale_factor;
            let (source, grid_spacing) = if scale < 10.0 {
                (&image, spacing)
            } else {
                (&broad, 2.0 * spacing)
            };
            let hessian = Hessian::at_scale(source, scale / grid_spacing).expect("bounded scale");
            let gradient =
                normalised_gradient(source, scale / grid_spacing).expect("bounded scale");
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    let film = image.film_point(x as f64, y as f64);
                    let (sx, sy) = source.image_point(film).expect("invertible placement");
                    let sx = sx.clamp(0.0, f64::from(source.width() - 1));
                    let sy = sy.clamp(0.0, f64::from(source.height() - 1));
                    let sample =
                        |map: &GreyImage<f32>| f64::from(bilinear(map, sx, sy).unwrap_or(0.0));
                    let (a, b, c) = (
                        sample(&hessian.xx),
                        sample(&hessian.xy),
                        sample(&hessian.yy),
                    );
                    let gap = libm::hypot(a - c, 2.0 * b);
                    let lambda1 = 0.5 * (a + c - gap);
                    let lambda2 = 0.5 * (a + c + gap);
                    if lambda1 >= -1e-9 {
                        continue;
                    }
                    let edge_ratio =
                        libm::hypot(sample(&gradient.x), sample(&gradient.y)) / -lambda1;
                    let response = -lambda1
                        * libm::exp(-2.0 * lambda2 * lambda2 / (lambda1 * lambda1))
                        * (1.0 - edge_ratio / 0.75).clamp(0.0, 1.0);
                    if response > f64::from(ridge[i]) {
                        ridge[i] = response as f32;
                        sigma[i] = scale as f32;
                        let theta = 0.5 * libm::atan2(2.0 * b, a - c);
                        along[i] = Point {
                            x: libm::cos(theta),
                            y: libm::sin(theta),
                        };
                    }
                }
            }
        }
        let normaliser = percentile(&ridge, 0.99).max(1e-6);
        let mut cost = vec![5.0f32; n];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                ridge[i] = (ridge[i] / normaliser).min(1.0);
                let side_border = (x.min(w - 1 - x) as f64) * spacing < 8.0;
                if mask.pixels()[i] < 0.5 || side_border {
                    continue;
                }
                cost[i] = 0.03 + ((1.0 - ridge[i]) * (1.0 - ridge[i]));
                if front && !body_fills {
                    let middle = 0.5 * (left[y] + right[y]) as f64;
                    let half = 0.5 * (right[y] - left[y]).max(1) as f64;
                    cost[i] += (4.0
                        * libm::pow(((x as f64 - middle).abs() / half - 0.6).max(0.0), 2.0))
                        as f32;
                }
            }
        }
        Ok(Self {
            image,
            fine,
            spacing,
            ridge,
            sigma,
            along,
            cost,
            top,
            bottom,
            top_cropped,
            bottom_cropped,
            neck_y,
            crest_y,
            size_factor,
            warnings,
        })
    }

    pub fn sample(&self, values: &[f32], p: Point) -> f64 {
        let x = (p.x / self.spacing).clamp(0.0, f64::from(self.image.width() - 1));
        let y = (p.y / self.spacing).clamp(0.0, f64::from(self.image.height() - 1));
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let w = self.image.width() as usize;
        let jx = (ix + 1).min(w - 1);
        let jy = (iy + 1).min(self.image.height() as usize - 1);
        let (fx, fy) = (x - ix as f64, y - iy as f64);
        (1.0 - fy)
            * ((1.0 - fx) * f64::from(values[iy * w + ix]) + fx * f64::from(values[iy * w + jx]))
            + fy * ((1.0 - fx) * f64::from(values[jy * w + ix])
                + fx * f64::from(values[jy * w + jx]))
    }

    pub fn film_point(&self, p: Point) -> Px {
        self.image
            .film_point(p.x / self.spacing, p.y / self.spacing)
    }

    pub fn zone_bounds(&self, zone: &[usize]) -> PxRect {
        let w = self.image.width() as usize;
        let mut bounds = PxRect {
            left: f64::INFINITY,
            top: f64::INFINITY,
            right: f64::NEG_INFINITY,
            bottom: f64::NEG_INFINITY,
        };
        for &i in zone {
            let p = self.image.film_point((i % w) as f64, (i / w) as f64);
            bounds.left = bounds.left.min(p.x());
            bounds.right = bounds.right.max(p.x());
            bounds.top = bounds.top.min(p.y());
            bounds.bottom = bounds.bottom.max(p.y());
        }
        bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_a_grid_keeps_each_original_film_pixel() {
        let image = GreyImage::new(3, 2, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0], Placement::FILM)
            .expect("sizes");
        for q in 0..4 {
            let upright = turn(&image, q);
            for y in 0..upright.height() {
                for x in 0..upright.width() {
                    let original = upright.film_point(f64::from(x), f64::from(y));
                    let expected = image
                        .get(original.x() as u32, original.y() as u32)
                        .expect("source pixel");
                    assert_eq!(upright.get(x, y), Some(expected));
                }
            }
        }
    }
}
