//! The detector: blur, scatter, noise and 16-bit quantisation, plus the
//! lossy-film variant.
//!
//! The painter hands over each pixel's line integral `P` (the attenuation
//! along its ray). A clean drawing (tiers t0 to t2) shows `P` itself,
//! windowed between the 0.5th and 99.5th percentiles of the body (what
//! attenuates the beam at all), as 16-bit grey levels
//! with bone bright. A real detector (t3, t4) goes through the physics:
//!
//! 1. primary `I_p = exp(−P)`, brighter at one end by the heel effect,
//!    nothing outside the collimated field;
//! 2. scatter `I_s`: the primary blurred by tens of millimetres (worked at
//!    an eighth of the resolution), scaled so it is the recipe's share of
//!    the primary over the body;
//! 3. counts: an exposure control sets the incident count `N0` so the
//!    mean count in a 40 mm square at the thoracolumbar junction is the
//!    recipe's target, then every pixel draws a Poisson count of mean
//!    `N0·(I_p + I_s)`;
//! 4. the scintillator blurs the counts (so the noise is correlated), and
//!    the electronics add normal noise;
//! 5. processing: `−ln(n / N0)`, an unsharp mask, a window between two
//!    percentiles of the body within the field, as a detector's own
//!    processing finds the anatomy, a gamma; the field's outside, never
//!    exposed, comes out white; the brightness flipped when the recipe says
//!    so;
//! 6. quantisation to 16 or 12 bits, or to 8 bits through a lossy JPEG-like
//!    step: 8 × 8 discrete cosine transforms quantised with the JPEG
//!    standard's luminance table (ITU-T T.81, Annex K.1) scaled to the
//!    recipe's quality as the IJG library scales it.
//!
//! The counts at t4 are low (80 to 400 at the spine) and the scatter
//! heavy, so the bone stands out little from the noise: these are the
//! hard films.

use super::Canvas;
use crate::recipes::{DetectorRecipe, FilmRecipe, Physics};
use crate::seeded::Stream;
use darkroom::blur::gaussian;
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::normalise::percentile_window;
use darkroom::pyramid::shrink;
use darkroom::sampling::bilinear;
use shapes::pixels::Px;

/// Scatter is worked at this fraction of the film's resolution.
const SCATTER_SHRINK: f64 = 8.0;

/// A film's grey levels and how many bits they use.
#[derive(Clone, Debug, PartialEq)]
pub struct Exposure {
    /// Row-major, `0 ..= 2^bits − 1`, bone bright unless inverted.
    pub values: Vec<u16>,
    pub bits: u8,
}

/// The JPEG luminance quantisation table, Annex K.1, row by row.
pub const LUMINANCE_TABLE: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, //
    12, 12, 14, 19, 26, 58, 60, 55, //
    14, 13, 16, 24, 40, 57, 69, 56, //
    14, 17, 22, 29, 51, 87, 80, 62, //
    18, 22, 37, 56, 68, 109, 103, 77, //
    24, 35, 55, 64, 81, 104, 113, 92, //
    49, 64, 78, 87, 103, 121, 120, 101, //
    72, 92, 95, 98, 112, 100, 103, 99,
];

/// Turns a canvas's line integrals into grey levels. `roi` is the centre
/// of the exposure-control region; `stream` gives the noise.
pub fn expose(canvas: &Canvas, recipe: &FilmRecipe, roi: Px, stream: &mut Stream) -> Exposure {
    match &recipe.detector {
        DetectorRecipe::Clean => clean(canvas),
        DetectorRecipe::Physical(physics) => physical(canvas, recipe, physics, roi, stream),
    }
}

/// Line integrals above this count as the body, for windowing: a few
/// millimetres of bone or a couple of centimetres of soft tissue.
const BODY: f32 = 0.1;

fn clean(canvas: &Canvas) -> Exposure {
    let body: Vec<f32> = canvas.path.iter().copied().filter(|&p| p > BODY).collect();
    let window = percentile_window(
        if body.is_empty() { &canvas.path } else { &body },
        0.005,
        0.995,
    );
    let values = canvas
        .path
        .iter()
        .map(|&p| {
            let v = window.map_or(0.0, |w| w.scale(f64::from(p), 1.0));
            libm::round(v * 65535.0) as u16
        })
        .collect();
    Exposure { values, bits: 16 }
}

fn physical(
    canvas: &Canvas,
    recipe: &FilmRecipe,
    physics: &Physics,
    roi: Px,
    stream: &mut Stream,
) -> Exposure {
    let layout = canvas.layout;
    let (width, height) = (canvas.width, canvas.height);
    let mm = recipe.mm_per_px;
    let upright_height = f64::from(layout.upright_size[1]);
    let in_field: Vec<bool> = (0..width * height)
        .map(|i| layout.shows(Px::new((i % width) as f64, (i / width) as f64)))
        .collect();

    // 1. The primary.
    let primary: Vec<f32> = canvas
        .path
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            if !in_field[i] {
                return 0.0;
            }
            let (_, v) = layout.film_to_upright((i % width) as f64, (i / width) as f64);
            let heel = 1.0 + physics.heel * (v / upright_height - 0.5);
            (libm::exp(-f64::from(p)) * heel) as f32
        })
        .collect();

    // 2. Scatter, at an eighth of the resolution.
    let image = |pixels: Vec<f32>| {
        GreyImage::new(width as u32, height as u32, pixels, Placement::FILM)
            .expect("one value per pixel")
    };
    let small = shrink(&image(primary.clone()), SCATTER_SHRINK, SCATTER_SHRINK)
        .expect("a shrink factor above one");
    let spread =
        gaussian(&small, physics.scatter_sigma_mm / (SCATTER_SHRINK * mm)).expect("a finite blur");
    let small_place = spread.placement();
    let blurred: Vec<f32> = (0..width * height)
        .map(|i| {
            let film = Px::new((i % width) as f64, (i / width) as f64);
            let (u, v) = small_place.image_point(film).unwrap_or((0.0, 0.0));
            let u = u.clamp(0.0, f64::from(spread.width()) - 1.0);
            let v = v.clamp(0.0, f64::from(spread.height()) - 1.0);
            bilinear(&spread, u, v).unwrap_or(0.0)
        })
        .collect();
    let body: Vec<usize> = (0..width * height)
        .filter(|&i| in_field[i] && canvas.path[i] > 0.5)
        .collect();
    let mean = |values: &[f32], over: &[usize]| {
        over.iter().map(|&i| f64::from(values[i])).sum::<f64>() / over.len().max(1) as f64
    };
    let (body_primary, body_blurred) = (mean(&primary, &body), mean(&blurred, &body));
    let scatter_scale = if body_blurred > 0.0 {
        physics.scatter_to_primary * body_primary / body_blurred
    } else {
        0.0
    };
    let incident: Vec<f64> = (0..width * height)
        .map(|i| {
            if in_field[i] {
                f64::from(primary[i]) + scatter_scale * f64::from(blurred[i])
            } else {
                0.0
            }
        })
        .collect();

    // 3. Counts, under exposure control.
    let half_roi = 20.0 / mm;
    let roi_pixels: Vec<usize> = (0..width * height)
        .filter(|&i| {
            let (x, y) = ((i % width) as f64, (i / width) as f64);
            (x - roi.x()).abs() <= half_roi && (y - roi.y()).abs() <= half_roi && in_field[i]
        })
        .collect();
    let roi_mean =
        roi_pixels.iter().map(|&i| incident[i]).sum::<f64>() / roi_pixels.len().max(1) as f64;
    let n0 = physics.target_counts / roi_mean.max(1e-9);
    let counts: Vec<f32> = incident
        .iter()
        .map(|&mean| stream.poisson(n0 * mean) as f32)
        .collect();

    // 4. The scintillator's blur, then electronic noise.
    let blurred_counts =
        gaussian(&image(counts), physics.blur_sigma_mm / mm).expect("a finite blur");
    let electronic = physics.electronic_noise * physics.target_counts.sqrt();
    let log: Vec<f32> = blurred_counts
        .pixels()
        .iter()
        .map(|&n| {
            let n = f64::from(n) + stream.normal(0.0, electronic);
            (-libm::log(n.max(0.5) / n0)) as f32
        })
        .collect();

    // 5. Processing.
    let smooth =
        gaussian(&image(log.clone()), physics.unsharp_sigma_mm / mm).expect("a finite blur");
    let sharpened: Vec<f32> = log
        .iter()
        .zip(smooth.pixels())
        .map(|(&l, &s)| (f64::from(l) + physics.unsharp_amount * f64::from(l - s)) as f32)
        .collect();
    let field_values: Vec<f32> = sharpened
        .iter()
        .enumerate()
        .filter(|&(i, _)| in_field[i] && canvas.path[i] > BODY)
        .map(|(_, &v)| v)
        .collect();
    let window = percentile_window(&field_values, physics.window[0], physics.window[1]);
    let shown: Vec<f64> = sharpened
        .iter()
        .zip(&in_field)
        .map(|(&v, &inside)| {
            let level = if inside {
                libm::pow(
                    window.map_or(0.0, |w| w.scale(f64::from(v), 1.0)),
                    physics.gamma,
                )
            } else {
                1.0
            };
            if recipe.inverted { 1.0 - level } else { level }
        })
        .collect();

    // 6. Quantisation.
    match physics.lossy_quality {
        Some(quality) => {
            let mut eight: Vec<u8> = shown
                .iter()
                .map(|&v| libm::round(v * 255.0) as u8)
                .collect();
            jpeg_like(&mut eight, width, height, quality);
            Exposure {
                values: eight.into_iter().map(u16::from).collect(),
                bits: 8,
            }
        }
        None => {
            let top = f64::from((1u32 << physics.bits) - 1);
            Exposure {
                values: shown.iter().map(|&v| libm::round(v * top) as u16).collect(),
                bits: physics.bits,
            }
        }
    }
}

/// The luminance table scaled for `quality` (1 to 100) as the IJG library
/// scales it: `5000/q` below 50, `200 − 2q` from 50 up, each entry
/// `⌊(Q·scale + 50) / 100⌋` kept between 1 and 255.
pub fn quantisation_table(quality: f64) -> [f64; 64] {
    let q = quality.clamp(1.0, 100.0);
    let scale = if q < 50.0 {
        5000.0 / q
    } else {
        200.0 - 2.0 * q
    };
    LUMINANCE_TABLE.map(|entry| {
        ((f64::from(entry) * scale + 50.0) / 100.0)
            .floor()
            .clamp(1.0, 255.0)
    })
}

/// The JPEG-like loss: each 8 × 8 block (edge blocks padded by repeating
/// their last row and column) shifted by −128, transformed, quantised with
/// [`quantisation_table`], transformed back, shifted back, rounded half
/// away from zero and clamped to 0..=255.
pub fn jpeg_like(pixels: &mut [u8], width: usize, height: usize, quality: f64) {
    let table = quantisation_table(quality);
    let cosines: [[f64; 8]; 8] = core::array::from_fn(|x| {
        core::array::from_fn(|u| {
            libm::cos((2.0 * x as f64 + 1.0) * u as f64 * core::f64::consts::PI / 16.0)
        })
    });
    let c = |u: usize| {
        if u == 0 {
            core::f64::consts::FRAC_1_SQRT_2
        } else {
            1.0
        }
    };
    for by in (0..height).step_by(8) {
        for bx in (0..width).step_by(8) {
            let mut block = [[0.0f64; 8]; 8];
            for (y, row) in block.iter_mut().enumerate() {
                for (x, value) in row.iter_mut().enumerate() {
                    let sy = (by + y).min(height - 1);
                    let sx = (bx + x).min(width - 1);
                    *value = f64::from(pixels[sy * width + sx]) - 128.0;
                }
            }
            let mut coefficients = [[0.0f64; 8]; 8];
            for (v, row) in coefficients.iter_mut().enumerate() {
                for (u, coefficient) in row.iter_mut().enumerate() {
                    let mut sum = 0.0;
                    for (y, block_row) in block.iter().enumerate() {
                        for (x, value) in block_row.iter().enumerate() {
                            sum += value * cosines[x][u] * cosines[y][v];
                        }
                    }
                    let exact = 0.25 * c(u) * c(v) * sum;
                    let step = table[v * 8 + u];
                    *coefficient = libm::round(exact / step) * step;
                }
            }
            for y in 0..8 {
                for x in 0..8 {
                    if by + y >= height || bx + x >= width {
                        continue;
                    }
                    let mut sum = 0.0;
                    for (v, row) in coefficients.iter().enumerate() {
                        for (u, coefficient) in row.iter().enumerate() {
                            sum += c(u) * c(v) * coefficient * cosines[x][u] * cosines[y][v];
                        }
                    }
                    let value = libm::round(0.25 * sum + 128.0).clamp(0.0, 255.0);
                    pixels[(by + y) * width + bx + x] = value as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_fifty_is_the_standard_table_and_a_hundred_is_all_ones() {
        let fifty = quantisation_table(50.0);
        for (scaled, entry) in fifty.iter().zip(LUMINANCE_TABLE) {
            assert_eq!(*scaled, f64::from(entry));
        }
        assert!(quantisation_table(100.0).iter().all(|&q| q == 1.0));
        // Worked by hand at quality 75: scale 50, so 16 → ⌊(800 + 50)/100⌋.
        assert_eq!(quantisation_table(75.0)[0], 8.0);
        assert_eq!(quantisation_table(10.0)[0], 80.0);
    }

    #[test]
    fn a_flat_block_stays_flat_within_its_quantisation_step() {
        // Only the block's mean survives, rounded to the first table
        // entry's step: the level moves by at most half of step / 8.
        for quality in [10.0, 50.0, 75.0, 95.0] {
            let mut pixels = vec![173u8; 8 * 8];
            jpeg_like(&mut pixels, 8, 8, quality);
            assert!(pixels.iter().all(|&p| p == pixels[0]), "{quality}");
            let step = quantisation_table(quality)[0];
            assert!(
                (f64::from(pixels[0]) - 173.0).abs() <= 0.5 * step / 8.0 + 0.5,
                "{quality}"
            );
        }
    }

    #[test]
    fn at_quality_one_hundred_the_round_trip_is_within_one_level() {
        let original: Vec<u8> = (0..16 * 13).map(|i| ((i * 37) % 251) as u8).collect();
        let mut pixels = original.clone();
        jpeg_like(&mut pixels, 16, 13, 100.0);
        for (a, b) in original.iter().zip(&pixels) {
            assert!((i32::from(*a) - i32::from(*b)).abs() <= 1);
        }
    }

    #[test]
    fn low_quality_loses_fine_detail() {
        let original: Vec<u8> = (0..64)
            .map(|i| if (i + i / 8) % 2 == 0 { 100 } else { 156 })
            .collect();
        let mut pixels = original.clone();
        jpeg_like(&mut pixels, 8, 8, 20.0);
        let error: i32 = original
            .iter()
            .zip(&pixels)
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .sum();
        assert!(error > 64 * 10, "{error}");
    }
}
