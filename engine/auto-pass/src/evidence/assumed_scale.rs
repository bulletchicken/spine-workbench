//! An anatomical scale from the width of the broad spinal ridge.

use super::{exclude_footprints, upright_index, upright_size};
use crate::handoffs::Polarity;
use darkroom::grey_image::GreyImage;
use darkroom::hessian::{Hessian, normalised_gradient};
use darkroom::pyramid;
use film_reader::decoded_film::DecodedFilm;
use shapes::film_geometry::FilmGeometry;
use shapes::pixels::PxRect;

/// A0's provisional short side is 450 units. Eight Gaussian scales span
/// short_side/150 to short_side/15 on G4, and the median winning sigma is
/// assigned 12 units: a top-hat column 24 units wide at gamma = 1.
/// Locate each row's ridge at the broadest scale before measuring its
/// width. Otherwise a bright cortical line can win as a separate centre.
pub(super) fn estimate(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    blanked: &[PxRect],
    polarity: Polarity,
) -> f64 {
    let short = f64::from(film.image.width().min(film.image.height()));
    let provisional = 450.0 / short;
    let fallback = provisional.clamp(0.01, 2.0);
    let factor = (4.0 / provisional).max(1.0);
    // One area average has the same G4 footprints as four provisional
    // one-unit cells, and retains their exact stored-film placement.
    let Some(grid) = pyramid::shrink(&film.image, factor, factor) else {
        return fallback;
    };
    let (w, h) = (grid.width() as usize, grid.height() as usize);
    if w < 4 || h < 4 {
        return fallback;
    }
    let valid = exclude_footprints(&grid, blanked);
    let orient = |v: f32| {
        if polarity == Polarity::BoneDark {
            -v
        } else {
            v
        }
    };
    let samples: Vec<_> = grid
        .pixels()
        .iter()
        .zip(&valid)
        .filter_map(|(&v, &on)| (on != 0).then_some(orient(v)))
        .collect();
    let low = quantile(&samples, 0.01);
    let high = quantile(&samples, 0.99);
    if high - low <= 1e-6 {
        return fallback;
    }
    let middle = quantile(&samples, 0.5);
    let image = GreyImage::new(
        grid.width(),
        grid.height(),
        grid.pixels()
            .iter()
            .zip(&valid)
            .map(|(&v, &on)| {
                let value = if on != 0 { orient(v) } else { middle };
                ((value - low) / (high - low)).clamp(0.0, 1.5)
            })
            .collect(),
        grid.placement(),
    )
    .expect("same provisional grid");
    let turns = geometry.quarter_turns;
    let (uw, uh) = upright_size(w, h, turns);
    let index = |x, y| upright_index(w, h, turns, x, y);
    let row_usable = |y| (0..uw).any(|x| valid[index(x, y)] != 0);
    let top = (0..uh).find(|&y| row_usable(y)).unwrap_or(0);
    let bottom = (top..uh).rfind(|&y| row_usable(y)).unwrap_or(top);
    let rows = top + (bottom - top) / 3..=top + 2 * (bottom - top) / 3;
    let columns = uw / 4..3 * uw / 4;
    let scales: Vec<_> = (0..8)
        .map(|s| short / 150.0 * libm::pow(10.0, f64::from(s) / 7.0) / factor)
        .collect();
    let mut responses = Vec::with_capacity(scales.len());
    for &sigma in &scales {
        let hessian = Hessian::at_scale(&image, sigma).expect("bounded positive scale");
        let gradient = normalised_gradient(&image, sigma).expect("bounded positive scale");
        let mut response = vec![0.0f32; w * h];
        for y in rows.clone() {
            for x in columns.clone() {
                let i = index(x, y);
                if valid[i] == 0 {
                    continue;
                }
                let a = f64::from(hessian.xx.pixels()[i]);
                let b = f64::from(hessian.xy.pixels()[i]);
                let c = f64::from(hessian.yy.pixels()[i]);
                let gap = libm::hypot(a - c, 2.0 * b);
                let lambda1 = (a + c - gap) * 0.5;
                let lambda2 = (a + c + gap) * 0.5;
                if lambda1 >= -1e-9 {
                    continue;
                }
                // A mostly horizontal line is not the upright column.
                let across = if turns.get().is_multiple_of(2) { a } else { c };
                if across > (lambda1 + lambda2) * 0.5 {
                    continue;
                }
                let magnitude = libm::hypot(
                    f64::from(gradient.x.pixels()[i]),
                    f64::from(gradient.y.pixels()[i]),
                );
                let edge = (1.0 - magnitude / -lambda1 / 0.75).clamp(0.0, 1.0);
                response[i] = (-lambda1
                    * libm::exp(-2.0 * lambda2 * lambda2 / (lambda1 * lambda1))
                    * edge) as f32;
            }
        }
        responses.push(response);
    }
    let mut winning_sigmas = Vec::new();
    let broadest = responses.last().expect("eight scales");
    for y in rows {
        let Some(x) = columns
            .clone()
            .max_by(|&x, &other| broadest[index(x, y)].total_cmp(&broadest[index(other, y)]))
        else {
            continue;
        };
        let i = index(x, y);
        let mut best = (0.0f32, 0usize);
        for (s, response) in responses.iter().enumerate() {
            if response[i] > best.0 {
                best = (response[i], s);
            }
        }
        if best.0 <= 1e-4 {
            continue;
        }
        let scale = best.1;
        let mut log_sigma = libm::log(scales[scale]);
        if scale > 0 && scale + 1 < scales.len() {
            let at = |s: usize| libm::log(f64::from(responses[s][i]).max(1e-12));
            let (a, b, c) = (at(scale - 1), at(scale), at(scale + 1));
            let curvature = a - 2.0 * b + c;
            if curvature < 0.0 {
                log_sigma +=
                    0.5 * (a - c) / curvature * libm::log(scales[scale + 1] / scales[scale]);
            }
        }
        winning_sigmas.push((libm::exp(log_sigma) * factor) as f32);
    }
    let sigma = quantile(&winning_sigmas, 0.5);
    if sigma > 0.0 {
        (12.0 / f64::from(sigma)).clamp(0.01, 2.0)
    } else {
        fallback
    }
}

fn quantile(values: &[f32], fraction: f64) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    *sorted.select_nth_unstable_by(index, f32::total_cmp).1
}
