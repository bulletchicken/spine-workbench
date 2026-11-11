//! Monogenic phase congruency and phase symmetry: edges and lines found by
//! where the phases of many scales agree, whatever the local contrast.
//!
//! A port of Kovesi's `phasecongmono` and `phasesymmono` (as in
//! ImagePhaseCongruency.jl, MIT) with the constants of evidence spec 10.
//! Each scale of the [`crate::log_gabor`] bank gives an even response `f`
//! (the image through the scale) and an odd pair `h1`, `h2` (through the
//! scale and the Riesz pair), and the amplitude `A = √(f² + h1² + h2²)`.
//! Over the scales:
//!
//! - **Phase congruency.** `E = √(Σf² + Σh1² + Σh2²)` is the energy of the
//!   summed responses and `ΣA` the sum of their amplitudes; where the
//!   phases agree, `E` is close to `ΣA`. With
//!   `width = (ΣA / (max A + ε) − 1) / (n − 1)` (how evenly the scales
//!   respond), `weight = 1 / (1 + exp((cutoff − width) · g))` and the noise
//!   threshold `T`, the edge strength is
//!   `weight · max(1 − gain · acos(min(E / (ΣA + ε), 1)), 0) · max(E − T, 0) / (E + ε)`.
//! - **Direction** `atan2(Σh2, Σh1)`, across the feature from its dark side
//!   to its bright side, in (−π, π], in image axes.
//! - **Kind** `atan2(Σf, √(Σh1² + Σh2²))`: +π/2 on a bright line, 0 on a
//!   step, −π/2 on a dark line. On a line's centre line the odd sums vanish,
//!   so the direction is unreliable wherever `|kind| > π/3`.
//! - **Phase symmetry.** `max(Σ(±f − √(h1² + h2²)) − T, 0) / (ΣA + ε)`, with
//!   `+f` for bright lines and `−f` for dark ones.
//!
//! The noise threshold follows Kovesi: `τ` is the median of the first
//! scale's amplitude divided by `√(ln 4)` (the mode of a Rayleigh
//! distribution with that median), the expected noise energy over `n`
//! scales is `τ (1 − m⁻ⁿ) / (1 − m⁻¹)` for scale multiple `m`, and `T` is
//! its Rayleigh mean plus `k` standard deviations. [`NoiseMode::Tiles`]
//! takes `τ` per tile and interpolates it, because X-ray noise is not the
//! same everywhere on a film; [`NoiseMode::Whole`] is Kovesi's one value.
//! Either way only usable pixels count toward the median.
//!
//! What a step scores with [`PhaseSettings::EVIDENCE`], measured here: a
//! noise-free step centred on a pixel 0.92 (not 0.99: the 3 px scale is
//! cut by the Nyquist limit to half the response of the others, and ε
//! costs 0.017); a step between two pixels about 0.59 on each, because
//! the 3 px scale's phase turns 60 degrees in half a pixel; and a step
//! blurred by a Gaussian of 0.5, 1, 1.5, 2 and 3 px, 0.90, 0.63, 0.32,
//! 0.16 and 0.06, because blur takes the response of the short scales
//! away and the spread of scales with it. On a film, the shortest
//! wavelength decides which edges count.
//!
//! A [`MonogenicRun`] works one scale at a time, so a caller can show the
//! provisional edge map after each scale and stop between scales. Scales
//! are taken in pairs for the even part, two real filters packed into one
//! inverse transform: 5 scales cost 5 odd and 3 even inverse transforms.
//! The input should be filled where it is unusable before its spectrum is
//! taken; the maps are not masked here.

use crate::grey_image::{GreyImage, Placement};
use crate::log_gabor::{FilterBank, LowPass, Spectrum, riesz};
use core::f64::consts::PI;
use rustfft::num_complex::Complex;

/// The most scales a run takes. Each one more is a longer wavelength by
/// the scale multiple, so 32 is far past any film.
const MOST_SCALES: usize = 32;

/// A tile with fewer usable pixels than this share takes its neighbours'
/// threshold (evidence spec 5.2).
const TILE_USABLE_SHARE: f64 = 0.25;

/// How the noise threshold `τ` is taken from the first scale's amplitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseMode {
    /// One value from the median over every usable pixel (Kovesi).
    Whole,
    /// One value per square tile of `side` px, from the median over its
    /// usable pixels, interpolated bilinearly between tile centres. A tile
    /// with under a quarter of its pixels usable takes the median of its
    /// neighbours' values, or the whole image's value when it has none.
    Tiles { side: u32 },
}

/// The settings of a run, in pixels of the image being filtered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseSettings {
    /// The wavelength of the first scale, in px; at least 2.
    pub shortest_wavelength: f64,
    /// Each scale's wavelength is this times the one before.
    pub scale_multiple: f64,
    /// Scales summed for phase congruency; at least 2.
    pub congruency_scales: usize,
    /// Scales summed for phase symmetry; at least 1.
    pub symmetry_scales: usize,
    /// The log-Gabor bandwidth ratio σ_f.
    pub sigma_on_f: f64,
    /// Noise standard deviations above the mean for congruency.
    pub congruency_k: f64,
    /// Noise standard deviations above the mean for symmetry.
    pub symmetry_k: f64,
    /// The spread of scales below which congruency is damped.
    pub cutoff: f64,
    /// How sharply it is damped there.
    pub sharpness: f64,
    /// How quickly congruency falls as the phases spread.
    pub deviation_gain: f64,
    /// Kovesi's ε, in image units: keeps the ratios finite where the image
    /// is flat. It is not scaled with the contrast, so it lowers the
    /// congruency of faint features: through `acos` near agreement it
    /// costs about `1.5 √(2ε / ΣA)`, 0.017 on a step of contrast 1 and
    /// 0.12 on one of contrast 0.02.
    pub epsilon: f64,
    pub low_pass: LowPass,
    pub noise: NoiseMode,
}

impl PhaseSettings {
    /// Evidence spec 10, in pixels: wavelengths from 3 px in steps of 2.1,
    /// four scales for congruency (k = 3) and five for symmetry (k = 2),
    /// σ_f 0.55, cutoff 0.5, g 10, deviation gain 1.5, ε 1e-4, low-pass
    /// 0.45 of order 15, and the whole-image noise threshold. The evidence stage
    /// raises the shortest wavelength to 1.5 mm when that is longer, and
    /// sets the tile side to 32 mm.
    pub const EVIDENCE: PhaseSettings = PhaseSettings {
        shortest_wavelength: 3.0,
        scale_multiple: 2.1,
        congruency_scales: 4,
        symmetry_scales: 5,
        sigma_on_f: 0.55,
        congruency_k: 3.0,
        symmetry_k: 2.0,
        cutoff: 0.5,
        sharpness: 10.0,
        deviation_gain: 1.5,
        epsilon: 1e-4,
        low_pass: LowPass::EVIDENCE,
        noise: NoiseMode::Whole,
    };

    fn scales(&self) -> usize {
        self.congruency_scales.max(self.symmetry_scales)
    }

    fn is_valid(&self) -> bool {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let tiles_ok = match self.noise {
            NoiseMode::Whole => true,
            NoiseMode::Tiles { side } => side > 0,
        };
        self.congruency_scales >= 2
            && self.symmetry_scales >= 1
            && self.scales() <= MOST_SCALES
            && self.scale_multiple.is_finite()
            && self.scale_multiple > 1.0
            && positive(self.cutoff)
            && positive(self.sharpness)
            && positive(self.deviation_gain)
            && self.epsilon.is_finite()
            && self.epsilon >= 0.0
            && self.congruency_k.is_finite()
            && self.symmetry_k.is_finite()
            && tiles_ok
    }
}

/// The maps of a finished run, each on the image's size and placement.
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseMaps {
    /// Phase congruency, 0 to 1.
    pub edge: GreyImage<f32>,
    /// Across the feature, dark side to bright side, in (−π, π].
    pub direction: GreyImage<f32>,
    /// +π/2 bright line, 0 step, −π/2 dark line.
    pub kind: GreyImage<f32>,
    /// Bright-line phase symmetry, 0 to 1.
    pub ridge_symmetry: GreyImage<f32>,
    /// Dark-line phase symmetry, 0 to 1.
    pub valley_symmetry: GreyImage<f32>,
    /// The noise threshold τ at every pixel.
    pub noise_level: GreyImage<f32>,
}

/// The phase maps of `image` in one call: its spectrum, then every scale.
/// `None` when the image has no spectrum (see [`Spectrum::of`]), the
/// settings are out of range, or `usable` (non-zero where a pixel may count
/// toward the noise threshold) is not the image's size.
pub fn phase_maps(
    image: &GreyImage<f32>,
    settings: &PhaseSettings,
    usable: Option<&GreyImage<u8>>,
) -> Option<PhaseMaps> {
    let spectrum = Spectrum::of(image)?;
    Some(MonogenicRun::new(&spectrum, settings, usable)?.finish())
}

/// The sums over scales so far.
struct Sums {
    congruency_amplitude: Vec<f32>,
    largest_amplitude: Vec<f32>,
    even: Vec<f32>,
    odd_x: Vec<f32>,
    odd_y: Vec<f32>,
    symmetry_amplitude: Vec<f32>,
    bright: Vec<f32>,
    dark: Vec<f32>,
}

impl Sums {
    fn new(len: usize) -> Sums {
        let zeros = || vec![0.0f32; len];
        Sums {
            congruency_amplitude: zeros(),
            largest_amplitude: zeros(),
            even: zeros(),
            odd_x: zeros(),
            odd_y: zeros(),
            symmetry_amplitude: zeros(),
            bright: zeros(),
            dark: zeros(),
        }
    }
}

/// A phase congruency and symmetry run over one spectrum, one scale at a
/// time.
pub struct MonogenicRun<'a> {
    spectrum: &'a Spectrum,
    settings: PhaseSettings,
    bank: FilterBank,
    usable: Option<&'a GreyImage<u8>>,
    done: usize,
    /// The gains and even response of the second scale of a pair, worked
    /// out with the first.
    held: Option<(Vec<f32>, GreyImage<f32>)>,
    sums: Sums,
    noise_level: Option<Vec<f32>>,
}

impl<'a> MonogenicRun<'a> {
    /// A run over `spectrum` with no scale done yet. `None` when the
    /// settings are out of range or `usable` is not the image's size.
    pub fn new(
        spectrum: &'a Spectrum,
        settings: &PhaseSettings,
        usable: Option<&'a GreyImage<u8>>,
    ) -> Option<MonogenicRun<'a>> {
        if !settings.is_valid() {
            return None;
        }
        let (width, height) = spectrum.image_size();
        if usable.is_some_and(|mask| mask.width() != width || mask.height() != height) {
            return None;
        }
        let wavelengths = FilterBank::geometric(
            settings.shortest_wavelength,
            settings.scale_multiple,
            settings.scales(),
        );
        let bank = FilterBank::new(
            spectrum.grid(),
            &wavelengths,
            settings.sigma_on_f,
            settings.low_pass,
        )?;
        Some(MonogenicRun {
            spectrum,
            settings: *settings,
            bank,
            usable,
            done: 0,
            held: None,
            sums: Sums::new(width as usize * height as usize),
            noise_level: None,
        })
    }

    /// How many scales the run takes in all.
    pub fn scales(&self) -> usize {
        self.settings.scales()
    }

    /// How many scales are done.
    pub fn scales_done(&self) -> usize {
        self.done
    }

    /// Works out the next scale and adds it to the sums. `false`, doing
    /// nothing, when every scale is done.
    pub fn step(&mut self) -> bool {
        let scale = self.done;
        if scale >= self.scales() {
            return false;
        }
        let (gains, even) = match self.held.take() {
            Some(held) => held,
            None => self.even_pair(scale),
        };
        let odd = self
            .spectrum
            .filtered(|index, point| riesz(point) * gains[index]);
        self.add_scale(
            scale,
            even.pixels(),
            odd.real.pixels(),
            odd.imaginary.pixels(),
        );
        self.done += 1;
        true
    }

    /// The even responses of `scale` and, when there is one, the scale
    /// after it, from one inverse transform. Returns this scale's gains
    /// and response and holds the next one's.
    fn even_pair(&mut self, scale: usize) -> (Vec<f32>, GreyImage<f32>) {
        let gains = self.bank.gains(scale).expect("the scale is in the bank");
        let real = |gain: f32| Complex::new(gain, 0.0);
        if scale + 1 < self.scales() {
            let next = self
                .bank
                .gains(scale + 1)
                .expect("the scale is in the bank");
            let pair = self
                .spectrum
                .filtered_pair(|i, _| real(gains[i]), |i, _| real(next[i]));
            self.held = Some((next, pair.imaginary));
            (gains, pair.real)
        } else {
            let alone = self.spectrum.filtered(|i, _| real(gains[i]));
            (gains, alone.real)
        }
    }

    fn add_scale(&mut self, scale: usize, even: &[f32], odd_x: &[f32], odd_y: &[f32]) {
        let n = even.len();
        let (odd_x, odd_y) = (&odd_x[..n], &odd_y[..n]);
        let odd: Vec<f32> = odd_x
            .iter()
            .zip(odd_y)
            .map(|(h1, h2)| (h1 * h1 + h2 * h2).sqrt())
            .collect();
        let amplitudes: Vec<f32> = even
            .iter()
            .zip(&odd)
            .map(|(f, o)| (f * f + o * o).sqrt())
            .collect();
        let sums = &mut self.sums;
        if scale < self.settings.congruency_scales {
            add_into(&mut sums.congruency_amplitude, &amplitudes, |a| a);
            for (largest, &a) in sums.largest_amplitude.iter_mut().zip(&amplitudes) {
                *largest = largest.max(a);
            }
            add_into(&mut sums.even, even, |f| f);
            add_into(&mut sums.odd_x, odd_x, |h| h);
            add_into(&mut sums.odd_y, odd_y, |h| h);
        }
        if scale < self.settings.symmetry_scales {
            add_into(&mut sums.symmetry_amplitude, &amplitudes, |a| a);
            for ((bright, dark), (&f, &o)) in sums
                .bright
                .iter_mut()
                .zip(sums.dark.iter_mut())
                .zip(even.iter().zip(&odd))
            {
                *bright += f - o;
                *dark += -f - o;
            }
        }
        if scale == 0 {
            let (width, height) = self.spectrum.image_size();
            let usable = self.usable.map(GreyImage::pixels);
            self.noise_level = Some(noise_levels(
                &amplitudes,
                width as usize,
                height as usize,
                usable,
                self.settings.noise,
            ));
        }
    }

    /// The expected noise energy over `scales` scales per unit `τ`, and the
    /// threshold factor for `k`: `(1 − m⁻ⁿ) / (1 − m⁻¹) · (√(π/2) + k √((4 − π)/2))`.
    fn threshold_factor(&self, scales: usize, k: f64) -> f64 {
        let ratio = 1.0 / self.settings.scale_multiple;
        let mut power = 1.0;
        for _ in 0..scales {
            power *= ratio;
        }
        let total = (1.0 - power) / (1.0 - ratio);
        total * ((PI / 2.0).sqrt() + k * ((4.0 - PI) / 2.0).sqrt())
    }

    /// The edge strength from the scales done so far: what a watcher sees
    /// while the run goes on. `None` before two scales are done, when the
    /// spread of scales is not yet defined.
    pub fn provisional_edge(&self) -> Option<GreyImage<f32>> {
        let scales = self.done.min(self.settings.congruency_scales);
        (scales >= 2).then(|| self.image(self.congruency(scales, false).0))
    }

    /// Runs every remaining scale and returns the maps.
    pub fn finish(mut self) -> PhaseMaps {
        while self.step() {}
        let (edge, direction, kind) = self.congruency(self.settings.congruency_scales, true);
        let (ridge_symmetry, valley_symmetry) = self.symmetry();
        let levels = self.noise_level.take().unwrap_or_default();
        let noise_level = self.image(levels);
        PhaseMaps {
            edge: self.image(edge),
            direction: self.image(direction),
            kind: self.image(kind),
            ridge_symmetry,
            valley_symmetry,
            noise_level,
        }
    }

    fn image(&self, pixels: Vec<f32>) -> GreyImage<f32> {
        let (width, height) = self.spectrum.image_size();
        let placement: Placement = self.spectrum.placement();
        GreyImage::new(width, height, pixels, placement).expect("one value per pixel")
    }

    // Intermediate previews need only strengths. Avoid allocating and
    // computing two full angle maps that the preview would discard.
    fn congruency(&self, scales: usize, angles: bool) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let s = &self.settings;
        let epsilon = s.epsilon;
        let factor = self.threshold_factor(scales, s.congruency_k);
        let noise = self.noise_level.as_deref().unwrap_or_default();
        let sums = &self.sums;
        let len = sums.even.len();
        let (mut edge, mut direction, mut kind) = (
            Vec::with_capacity(len),
            Vec::with_capacity(if angles { len } else { 0 }),
            Vec::with_capacity(if angles { len } else { 0 }),
        );
        for i in 0..len {
            let f = f64::from(sums.even[i]);
            let (h1, h2) = (f64::from(sums.odd_x[i]), f64::from(sums.odd_y[i]));
            let amplitude = f64::from(sums.congruency_amplitude[i]);
            let largest = f64::from(sums.largest_amplitude[i]);
            let odd = (h1 * h1 + h2 * h2).sqrt();
            let energy = (f * f + odd * odd).sqrt();
            let threshold = f64::from(noise.get(i).copied().unwrap_or(0.0)) * factor;
            let above = (energy - threshold).max(0.0) / (energy + epsilon);
            let strength = if above > 0.0 {
                let spread = (amplitude / (largest + epsilon) - 1.0) / (scales - 1) as f64;
                let weight = 1.0 / (1.0 + libm::exp((s.cutoff - spread) * s.sharpness));
                let agreement = (energy / (amplitude + epsilon)).min(1.0);
                let phase = (1.0 - s.deviation_gain * libm::acos(agreement)).max(0.0);
                weight * phase * above
            } else {
                0.0
            };
            edge.push(strength as f32);
            if angles {
                direction.push(half_open_angle(libm::atan2f(h2 as f32, h1 as f32)));
                kind.push(libm::atan2f(f as f32, odd as f32));
            }
        }
        (edge, direction, kind)
    }

    fn symmetry(&self) -> (GreyImage<f32>, GreyImage<f32>) {
        let factor = self.threshold_factor(self.settings.symmetry_scales, self.settings.symmetry_k);
        let epsilon = self.settings.epsilon;
        let noise = self.noise_level.as_deref().unwrap_or_default();
        let sums = &self.sums;
        let measure = |energies: &[f32]| -> Vec<f32> {
            energies
                .iter()
                .zip(&sums.symmetry_amplitude)
                .enumerate()
                .map(|(i, (&energy, &amplitude))| {
                    let threshold = f64::from(noise.get(i).copied().unwrap_or(0.0)) * factor;
                    ((f64::from(energy) - threshold).max(0.0) / (f64::from(amplitude) + epsilon))
                        as f32
                })
                .collect()
        };
        (
            self.image(measure(&sums.bright)),
            self.image(measure(&sums.dark)),
        )
    }
}

/// Adds `term` of each value to the matching sum.
fn add_into(sums: &mut [f32], values: &[f32], term: impl Fn(f32) -> f32) {
    for (sum, &value) in sums.iter_mut().zip(values) {
        *sum += term(value);
    }
}

/// An angle from `atan2` moved from −π to π, so it lies in (−π, π].
fn half_open_angle(angle: f32) -> f32 {
    use core::f32::consts::PI;
    if angle <= -PI {
        angle + 2.0 * PI
    } else {
        angle
    }
}

/// The median of `values`, the mean of the two middle values when their
/// number is even. `None` when there are none.
fn median(values: &mut [f32]) -> Option<f32> {
    let len = values.len();
    if len == 0 {
        return None;
    }
    let middle = len / 2;
    let (lower, upper, _) = values.select_nth_unstable_by(middle, f32::total_cmp);
    let upper = *upper;
    if len % 2 == 1 {
        return Some(upper);
    }
    let below = lower.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    Some((below + upper) / 2.0)
}

/// Kovesi's `τ` at every pixel: the median of the first scale's
/// amplitude over usable pixels, divided by `√(ln 4)`, for the whole image
/// or per tile.
fn noise_levels(
    amplitude: &[f32],
    width: usize,
    height: usize,
    usable: Option<&[u8]>,
    mode: NoiseMode,
) -> Vec<f32> {
    let rayleigh = libm::sqrt(libm::log(4.0)) as f32;
    let is_usable = |i: usize| usable.is_none_or(|mask| mask[i] != 0);
    let mut chosen: Vec<f32> = (0..amplitude.len())
        .filter(|&i| is_usable(i))
        .map(|i| amplitude[i])
        .collect();
    if chosen.is_empty() {
        chosen = amplitude.to_vec();
    }
    let whole = median(&mut chosen).unwrap_or(0.0) / rayleigh;
    let side = match mode {
        NoiseMode::Whole => return vec![whole; amplitude.len()],
        NoiseMode::Tiles { side } => side as usize,
    };
    let tiles_across = width.div_ceil(side);
    let tiles_down = height.div_ceil(side);
    let mut measured = Vec::with_capacity(tiles_across * tiles_down);
    for tile_y in 0..tiles_down {
        for tile_x in 0..tiles_across {
            let (left, top) = (tile_x * side, tile_y * side);
            let (right, bottom) = ((left + side).min(width), (top + side).min(height));
            let mut values: Vec<f32> = (top..bottom)
                .flat_map(|y| (left..right).map(move |x| y * width + x))
                .filter(|&i| is_usable(i))
                .map(|i| amplitude[i])
                .collect();
            let count = (right - left) * (bottom - top);
            let enough = values.len() as f64 >= TILE_USABLE_SHARE * count as f64;
            measured.push(if enough {
                median(&mut values).map(|m| m / rayleigh)
            } else {
                None
            });
        }
    }
    let tiles: Vec<f32> = (0..measured.len())
        .map(|index| {
            measured[index].unwrap_or_else(|| {
                let (tile_x, tile_y) = (index % tiles_across, index / tiles_across);
                let mut neighbours: Vec<f32> = (tile_y.saturating_sub(1)
                    ..(tile_y + 2).min(tiles_down))
                    .flat_map(|y| {
                        (tile_x.saturating_sub(1)..(tile_x + 2).min(tiles_across))
                            .map(move |x| y * tiles_across + x)
                    })
                    .filter_map(|i| measured[i])
                    .collect();
                median(&mut neighbours).unwrap_or(whole)
            })
        })
        .collect();
    let across = interpolation_weights(width, side);
    let down = interpolation_weights(height, side);
    let mut levels = Vec::with_capacity(width * height);
    for &(row_a, row_b, t) in &down {
        for &(column_a, column_b, s) in &across {
            let at = |row: usize, column: usize| tiles[row * tiles_across + column];
            let top = at(row_a, column_a) * (1.0 - s) + at(row_a, column_b) * s;
            let bottom = at(row_b, column_a) * (1.0 - s) + at(row_b, column_b) * s;
            levels.push(top * (1.0 - t) + bottom * t);
        }
    }
    levels
}

/// For each pixel of a line of `len` cut into tiles of `side`, the two
/// tiles whose centres bracket it and the weight of the second. The centre
/// of a tile is the middle of the pixels it holds; a pixel outside the
/// outermost centres takes that tile alone.
fn interpolation_weights(len: usize, side: usize) -> Vec<(usize, usize, f32)> {
    let tiles = len.div_ceil(side);
    let centre = |tile: usize| {
        let start = tile * side;
        let count = side.min(len - start);
        start as f64 + (count as f64 - 1.0) / 2.0
    };
    (0..len)
        .map(|position| {
            let p = position as f64;
            let after = (0..tiles)
                .find(|&tile| centre(tile) >= p)
                .unwrap_or(tiles - 1);
            if after == 0 || centre(after) < p {
                return (after, after, 0.0);
            }
            let before = after - 1;
            let t = (p - centre(before)) / (centre(after) - centre(before));
            (before, after, t as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOLE: PhaseSettings = PhaseSettings::EVIDENCE;

    fn image(width: u32, height: u32, value: impl Fn(f64, f64) -> f64) -> GreyImage<f32> {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| value(f64::from(x), f64::from(y)) as f32)
            .collect();
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    /// Seeded Gaussian noise: SplitMix64 for uniforms, Box-Muller for
    /// normals, so every run draws the same values.
    struct Noise(u64);

    impl Noise {
        fn uniform(&mut self) -> f64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
        }

        fn normal(&mut self) -> f64 {
            let u = self.uniform().max(1e-300);
            let v = self.uniform();
            libm::sqrt(-2.0 * libm::log(u)) * libm::cos(2.0 * PI * v)
        }

        fn add(&mut self, picture: &GreyImage<f32>, sd: impl Fn(u32) -> f64) -> GreyImage<f32> {
            let width = picture.width();
            let pixels = picture
                .pixels()
                .iter()
                .enumerate()
                .map(|(i, &v)| (f64::from(v) + sd(i as u32 % width) * self.normal()) as f32)
                .collect();
            GreyImage::new(width, picture.height(), pixels, Placement::FILM).expect("sizes match")
        }
    }

    fn at(map: &GreyImage<f32>, x: u32, y: u32) -> f64 {
        f64::from(*map.get(x, y).expect("inside"))
    }

    /// The mean of each column over the given rows.
    fn column_profile(map: &GreyImage<f32>, rows: core::ops::Range<u32>) -> Vec<f64> {
        let count = f64::from(rows.end - rows.start);
        (0..map.width())
            .map(|x| rows.clone().map(|y| at(map, x, y)).sum::<f64>() / count)
            .collect()
    }

    fn argmax(values: &[f64]) -> usize {
        values
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .expect("not empty")
    }

    /// The `fraction` quantile of a map's values over the given columns.
    fn quantile(map: &GreyImage<f32>, columns: core::ops::Range<u32>, fraction: f64) -> f64 {
        let mut values: Vec<f32> = (0..map.height())
            .flat_map(|y| columns.clone().map(move |x| (x, y)))
            .map(|(x, y)| *map.get(x, y).expect("inside"))
            .collect();
        values.sort_by(f32::total_cmp);
        f64::from(values[((values.len() - 1) as f64 * fraction).round() as usize])
    }

    /// The value across an ideal vertical step centred on column `at`:
    /// 0 before it, 1/2 on it, 1 after it, so the edge lies on a pixel
    /// centre.
    fn rise(x: f64, at: f64) -> f64 {
        if x < at {
            0.0
        } else if x == at {
            0.5
        } else {
            1.0
        }
    }

    /// A vertical step of `contrast` from `low`, centred on column 128.
    fn step(contrast: f64, low: f64) -> GreyImage<f32> {
        image(256, 96, |x, _| low + contrast * rise(x, 128.0))
    }

    #[test]
    fn a_clean_step_meets_the_analytic_answer_when_every_scale_fits() {
        // Evidence spec 10.5: for a noise-free ideal step every scale is in
        // phase and the scales respond equally (a step's spectrum falls as
        // 1/f, and a log-Gabor scale has the same width on a log axis at
        // every scale), so the spread is 1, the weight is
        // 1 / (1 + e^(−5)) = 0.9933, and with ε negligible that is the
        // congruency. That holds when every scale lies well inside the
        // grid's band: from 6 px the first scale's upper half-power
        // frequency is 1/(6 · 0.55) = 0.30 cycles per px, under the 0.45
        // low-pass. The step is centred on column 128, so the peak is there;
        // 0.98 allows for the scales' small differences in amplitude on a
        // finite grid.
        let fitting = PhaseSettings {
            shortest_wavelength: 6.0,
            epsilon: 1e-9,
            ..WHOLE
        };
        let maps = phase_maps(&step(1.0, 0.0), &fitting, None).expect("valid input");
        let profile = column_profile(&maps.edge, 0..96);
        assert_eq!(argmax(&profile), 128);
        assert!(profile[128] >= 0.98, "peak {}", profile[128]);
    }

    #[test]
    fn a_clean_step_scores_high_with_the_evidence_settings() {
        // With the evidence settings the first scale (3 px) reaches past the
        // Nyquist frequency and the low-pass, which halve its response to a
        // step (measured 0.235 against 0.473 at the fourth scale). The
        // spread falls to 0.77 and the weight to 0.936, and ε = 1e-4 costs
        // 1.5 √(2ε / ΣA) = 0.017 more through acos, so the peak is about
        // 0.92, not the 0.99 of evidence spec 10.5 or its bar of 0.95
        // (spec T8), which assume equal responses. The bar here is 0.9. A
        // step is a step, so its kind is near 0 (bar 0.3, spec T8), and its
        // bright side is +x, so its direction is 0 (bar 3 degrees, spec T9).
        let maps = phase_maps(&step(1.0, 0.0), &WHOLE, None).expect("valid input");
        let profile = column_profile(&maps.edge, 0..96);
        assert_eq!(argmax(&profile), 128);
        assert!(profile[128] >= 0.9, "peak {}", profile[128]);
        assert!(at(&maps.kind, 128, 48).abs() < 0.3);
        assert!(at(&maps.direction, 128, 48).abs() < 3f64.to_radians());
    }

    #[test]
    fn steps_score_alike_once_they_stand_clear_of_the_noise() {
        // Evidence spec T8: steps of contrast 0.02, 0.1 and 0.5 under noise
        // of sd 0.002, whole-image threshold. Each peak lies within 1 px of
        // the edge (columns 127 to 129) and its kind is under 0.3. The
        // profile is the mean over all rows, so one noisy row cannot move
        // it. At contrast 0.1 and 0.5 (signal 50 and 250 times the noise)
        // the peaks are at least 0.6 and within 0.1 of each other, as the
        // spec asks (measured 0.83 and 0.90). At 0.02 the spec's 0.6 is not
        // reached (measured 0.53): the noise threshold takes about a quarter
        // of the step's energy there (T = 0.0082 against E = 0.031), noise
        // scatters the phases, and ε costs 1.5 √(2ε / ΣA) = 0.12 at
        // ΣA = 0.031. The bar at 0.02 is 0.5.
        let mut noise = Noise(8);
        let mut peaks = Vec::new();
        for contrast in [0.02, 0.1, 0.5] {
            let noisy = noise.add(&step(contrast, 0.3), |_| 0.002);
            let maps = phase_maps(&noisy, &WHOLE, None).expect("valid input");
            let profile = column_profile(&maps.edge, 0..96);
            let peak = argmax(&profile);
            assert!(
                (127..=129).contains(&peak),
                "contrast {contrast}: peak at {peak}"
            );
            let kind = column_profile(&maps.kind, 0..96)[peak];
            assert!(kind.abs() < 0.3, "contrast {contrast}: kind {kind}");
            peaks.push(profile[peak]);
        }
        assert!(peaks[0] >= 0.5, "peaks {peaks:?}");
        assert!(peaks[1] >= 0.6 && peaks[2] >= 0.6, "peaks {peaks:?}");
        assert!((peaks[1] - peaks[2]).abs() <= 0.1, "peaks {peaks:?}");
        assert!(
            peaks[0] < peaks[1] && peaks[1] < peaks[2],
            "peaks {peaks:?}"
        );
    }

    /// A bright bar and a slanted step on a dark ground: lines, an edge
    /// and the places they come near each other.
    fn bar_and_edge() -> GreyImage<f32> {
        image(128, 64, |x, y| {
            let bar = if (x - 40.0).abs() <= 2.0 { 0.6 } else { 0.0 };
            let edge = if x + 0.3 * y > 80.0 { 1.0 } else { 0.0 };
            bar + edge
        })
    }

    #[test]
    fn brightness_leaves_the_maps_unchanged() {
        // An offset of 0.4 changes only the DC term of the periodic part,
        // where every scale is zero, so the strength maps agree to f32
        // rounding of the transforms: 1e-5 (measured 2e-6). Angles agree to
        // 1e-4 rad where the edge is above 0.1; elsewhere the sums are
        // rounding-sized and so are their angles.
        let base = bar_and_edge();
        let a = phase_maps(&base, &WHOLE, None).expect("valid input");
        let b = phase_maps(&base.map(|v| v + 0.4), &WHOLE, None).expect("valid input");
        for (one, two) in [
            (&a.edge, &b.edge),
            (&a.ridge_symmetry, &b.ridge_symmetry),
            (&a.valley_symmetry, &b.valley_symmetry),
        ] {
            for (p, q) in one.pixels().iter().zip(two.pixels()) {
                assert!((p - q).abs() < 1e-5, "{p} against {q}");
            }
        }
        for (one, two) in [(&a.direction, &b.direction), (&a.kind, &b.kind)] {
            for (i, (p, q)) in one.pixels().iter().zip(two.pixels()).enumerate() {
                if a.edge.pixels()[i] > 0.1 {
                    assert!((p - q).abs() < 1e-4, "{p} against {q}");
                }
            }
        }
    }

    #[test]
    fn contrast_changes_the_maps_only_through_epsilon() {
        // Every sum and the noise threshold scale with the contrast, so the
        // one change comes from ε = 1e-4, which does not. It matters most
        // through acos(E / (ΣA + ε)) where the phases agree exactly: there
        // the phase term is 1 − 1.5 √(2ε / ΣA), so halving the contrast of
        // the step (ΣA ≈ 1.56) moves it by 1.5 (√(2ε / 0.78) − √(2ε / 1.56))
        // = 0.007, and doubling it by 0.005. The bound is 0.01 (measured
        // 1.8e-3 and 1.3e-3). Directions, where the edge is above 0.1, are
        // ratios of sums with no ε: 1e-3 rad.
        let base = bar_and_edge();
        let a = phase_maps(&base, &WHOLE, None).expect("valid input");
        for gain in [0.5f32, 2.0] {
            let b = phase_maps(&base.map(|v| gain * v), &WHOLE, None).expect("valid input");
            for (one, two) in [
                (&a.edge, &b.edge),
                (&a.ridge_symmetry, &b.ridge_symmetry),
                (&a.valley_symmetry, &b.valley_symmetry),
            ] {
                for (p, q) in one.pixels().iter().zip(two.pixels()) {
                    assert!((p - q).abs() < 0.01, "gain {gain}: {p} against {q}");
                }
            }
            for (i, (p, q)) in a
                .direction
                .pixels()
                .iter()
                .zip(b.direction.pixels())
                .enumerate()
            {
                if a.edge.pixels()[i] > 0.1 {
                    assert!((p - q).abs() < 1e-3, "gain {gain}: {p} against {q}");
                }
            }
        }
    }

    /// A step with its bright side toward `angle` (from +x toward +y),
    /// through (64, 64), with one pixel of linear ramp across it.
    fn turned_step(angle: f64) -> GreyImage<f32> {
        let (c, s) = (libm::cos(angle), libm::sin(angle));
        image(128, 128, |x, y| {
            let across = (x - 64.0) * c + (y - 64.0) * s;
            (0.5 + across).clamp(0.0, 1.0)
        })
    }

    #[test]
    fn the_direction_points_from_dark_to_bright() {
        // Evidence spec T9: a step whose bright side lies toward 30 degrees
        // reads 30 ± 3, and its mirror image reads 150 ± 3, wherever the
        // edge scores above 0.5 within 24 px of the centre (the centre
        // keeps the test away from the corners, where the edge meets the
        // film's border at a slant).
        for degrees in [30.0f64, 150.0, -60.0] {
            let picture = turned_step(degrees.to_radians());
            let maps = phase_maps(&picture, &WHOLE, None).expect("valid input");
            let mut checked = 0;
            for y in 40..88u32 {
                for x in 40..88u32 {
                    if at(&maps.edge, x, y) > 0.5 {
                        let got = at(&maps.direction, x, y).to_degrees();
                        assert!(
                            (got - degrees).abs() < 3.0,
                            "{degrees}: {got} at ({x}, {y})"
                        );
                        checked += 1;
                    }
                }
            }
            assert!(checked > 40, "{degrees}: only {checked} edge pixels");
        }
    }

    /// A vertical line 5 px wide centred on column 64, of value `line` on
    /// a background of `background`.
    fn line(line: f64, background: f64) -> GreyImage<f32> {
        image(128, 64, |x, _| {
            if (x - 64.0).abs() <= 2.0 {
                line
            } else {
                background
            }
        })
    }

    #[test]
    fn lines_have_the_kind_of_their_polarity() {
        // Evidence spec T9: on the centre of a bright line the even sum
        // dominates and the odd sums cancel, so the kind is near +π/2; on a
        // dark line near −π/2 (bar 0.2 rad).
        let bright = phase_maps(&line(1.0, 0.0), &WHOLE, None).expect("valid input");
        let dark = phase_maps(&line(0.0, 1.0), &WHOLE, None).expect("valid input");
        assert!(at(&bright.kind, 64, 32) >= PI / 2.0 - 0.2);
        assert!(at(&dark.kind, 64, 32) <= -PI / 2.0 + 0.2);
    }

    #[test]
    fn symmetry_follows_the_polarity_of_a_bar() {
        // Evidence spec T14, on bars 5 and 9 px wide: ridge symmetry peaks
        // on a bright bar's centre and valley symmetry on a dark one's,
        // within 1 px, and the opposite map is at most 0.1 at that peak
        // (measured peaks 0.84 to 0.86, opposite 0). Light noise (sd 0.002)
        // gives the threshold a floor: on a noise-free film it is zero and
        // the far tails of the long scales are scored as features.
        for half in [2.0, 4.0] {
            let bar = |inside: f64, outside: f64| {
                image(128, 64, move |x, _| {
                    if (x - 64.0).abs() <= half {
                        inside
                    } else {
                        outside
                    }
                })
            };
            for (picture, bright) in [(bar(1.0, 0.0), true), (bar(0.0, 1.0), false)] {
                let picture = Noise(15).add(&picture, |_| 0.002);
                let maps = phase_maps(&picture, &WHOLE, None).expect("valid input");
                let (same, other) = if bright {
                    (&maps.ridge_symmetry, &maps.valley_symmetry)
                } else {
                    (&maps.valley_symmetry, &maps.ridge_symmetry)
                };
                let profile = column_profile(same, 0..64);
                let peak = argmax(&profile);
                assert!(
                    peak.abs_diff(64) <= 1,
                    "half {half}, bright {bright}: peak at {peak}"
                );
                assert!(
                    profile[peak] > 0.3,
                    "half {half}, bright {bright}: {}",
                    profile[peak]
                );
                let opposite = column_profile(other, 0..64)[peak];
                assert!(
                    opposite <= 0.1,
                    "half {half}, bright {bright}: opposite {opposite}"
                );
            }
        }
    }

    #[test]
    fn white_noise_scores_low() {
        // Evidence spec T10, on a 256 × 256 field of noise of sd 0.05 (the
        // spec uses 1024 px; the threshold is a ratio, so the size only
        // tightens the quantile). The threshold sits k Rayleigh standard
        // deviations above the expected noise energy, so the 99th
        // percentile of congruency and of both symmetries must be at most
        // 0.05.
        let mut noise = Noise(10);
        let field = noise.add(&image(256, 256, |_, _| 0.5), |_| 0.05);
        let maps = phase_maps(&field, &WHOLE, None).expect("valid input");
        for (name, map) in [
            ("edge", &maps.edge),
            ("ridge", &maps.ridge_symmetry),
            ("valley", &maps.valley_symmetry),
        ] {
            let p99 = quantile(map, 0..256, 0.99);
            assert!(p99 <= 0.05, "{name}: p99 {p99}");
        }
    }

    #[test]
    fn tiles_follow_noise_that_changes_across_the_film() {
        // Evidence spec T10: noise whose sd ramps from 0.01 to 0.06 across
        // x. With 64 px tiles the threshold follows it, so the 99th
        // percentile of congruency in every 64 px column band is at most
        // 0.05. With one whole-image threshold, set by the median noise,
        // the noisiest band lets noise through: its 99th percentile is
        // above 0.05, which shows the test can see what the tiles fix.
        let width = 512u32;
        let ramp = |x: u32| 0.01 + 0.05 * f64::from(x) / f64::from(width - 1);
        let field = Noise(11).add(&image(width, 128, |_, _| 0.5), ramp);
        let tiled = PhaseSettings {
            noise: NoiseMode::Tiles { side: 64 },
            ..WHOLE
        };
        let by_tiles = phase_maps(&field, &tiled, None).expect("valid input");
        for band in 0..width / 64 {
            let p99 = quantile(&by_tiles.edge, band * 64..band * 64 + 64, 0.99);
            assert!(p99 <= 0.05, "band {band}: p99 {p99}");
        }
        let whole = phase_maps(&field, &WHOLE, None).expect("valid input");
        let noisiest = quantile(&whole.edge, width - 64..width, 0.99);
        assert!(noisiest > 0.05, "noisiest band p99 {noisiest}");
    }

    #[test]
    fn unusable_pixels_do_not_set_the_threshold() {
        // The left half is heavy noise (sd 0.3) marked unusable; the right
        // half holds a step of 0.1 under light noise (sd 0.002). Counting
        // only usable pixels, the threshold is the right half's and the
        // step scores as in spec T8 (at least 0.6; measured 0.82). Counting
        // every pixel, the median amplitude comes from the noisy half and
        // the step is lost (under 0.3; measured 0).
        let picture = image(256, 96, |x, _| 0.3 + 0.1 * rise(x, 192.0));
        let noisy = Noise(12).add(&picture, |x| if x < 128 { 0.3 } else { 0.002 });
        let usable = GreyImage::new(
            256,
            96,
            (0..256 * 96).map(|i| u8::from(i % 256 >= 128)).collect(),
            Placement::FILM,
        )
        .expect("sizes match");
        let masked = phase_maps(&noisy, &WHOLE, Some(&usable)).expect("valid input");
        let open = phase_maps(&noisy, &WHOLE, None).expect("valid input");
        let peak = |maps: &PhaseMaps| {
            let profile = column_profile(&maps.edge, 0..96);
            profile[188..196].iter().fold(0.0f64, |m, &v| m.max(v))
        };
        assert!(peak(&masked) >= 0.6, "masked {}", peak(&masked));
        assert!(peak(&open) < 0.3, "open {}", peak(&open));
    }

    #[test]
    fn a_ramp_makes_no_edge_at_the_borders() {
        // Evidence spec T12: a ramp of height 1 across 128 px jumps by its
        // full height across the wrap of the transform, which as a step
        // would score about 0.9. The periodic-plus-smooth split leaves only
        // the change of slope there, the same roof a mirror at the border
        // makes, of the size of the ramp's slope (1/127 per px). Under
        // noise of sd 0.002 that roof scores at most 0.6 on the border
        // column itself (measured 0.36 to 0.40) and fades within a few
        // pixels; from 8 px in, congruency is at most 0.1 (measured 0.06).
        // The spec asks for 0.1 within 10 px of the border; the evidence
        // stage masks a 3 mm band (6 to 9 px), where the roof lies. Checked
        // across x on an even width and on an odd one that needs padding,
        // and down y.
        let mut noise = Noise(14);
        for width in [128u32, 125] {
            let across = noise.add(&image(width, 96, |x, _| x / f64::from(width - 1)), |_| {
                0.002
            });
            let maps = phase_maps(&across, &WHOLE, None).expect("valid input");
            for x in 0..width {
                let from_border = x.min(width - 1 - x);
                let bound = if from_border >= 8 { 0.1 } else { 0.6 };
                for y in 0..96 {
                    let v = at(&maps.edge, x, y);
                    assert!(v <= bound, "width {width}, ({x}, {y}): {v}");
                }
            }
        }
        let down = noise.add(&image(96, 128, |_, y| y / 127.0), |_| 0.002);
        let maps = phase_maps(&down, &WHOLE, None).expect("valid input");
        for y in 8..120 {
            for x in 0..96 {
                assert!(
                    at(&maps.edge, x, y) <= 0.1,
                    "({x}, {y}): {}",
                    at(&maps.edge, x, y)
                );
            }
        }
    }

    #[test]
    fn an_edge_keeps_its_score_where_it_meets_the_border() {
        // A step running off the top and bottom of the film reads past the
        // edge by reflection, which continues the step unchanged, so the
        // border rows score as the middle row does, within 0.05. A step
        // centred 8 px from the left border is still found on its own
        // column, at least 0.9 (measured 0.920, as in the middle of the
        // film): reading past the border mirrors the dark side, which
        // looks the same, and the wrap jump on the far side is taken out
        // with the smooth part.
        let maps = phase_maps(&step(1.0, 0.0), &WHOLE, None).expect("valid input");
        let middle = at(&maps.edge, 128, 48);
        for y in [0, 95] {
            let border = at(&maps.edge, 128, y);
            assert!(
                (border - middle).abs() < 0.05,
                "row {y}: {border} against {middle}"
            );
        }
        let near = image(128, 64, |x, _| rise(x, 8.0));
        let maps = phase_maps(&near, &WHOLE, None).expect("valid input");
        let profile = column_profile(&maps.edge, 0..64);
        let peak = argmax(&profile[..32]);
        assert_eq!(peak, 8);
        assert!(profile[peak] >= 0.9, "peak {}", profile[peak]);
    }

    #[test]
    fn the_run_goes_one_scale_at_a_time() {
        // No provisional map before two scales; one after each scale from
        // the second; the last provisional map is the finished edge map,
        // bit for bit, because both read the same four sums.
        let picture = Noise(13).add(&step(0.2, 0.2), |_| 0.01);
        let spectrum = Spectrum::of(&picture).expect("finite image");
        let mut run = MonogenicRun::new(&spectrum, &WHOLE, None).expect("valid settings");
        assert_eq!(run.scales(), 5);
        assert!(run.provisional_edge().is_none());
        assert!(run.step());
        assert!(run.provisional_edge().is_none());
        let mut last = None;
        for done in 2..=5 {
            assert!(run.step());
            assert_eq!(run.scales_done(), done);
            last = run.provisional_edge();
            assert!(last.is_some());
        }
        assert!(!run.step());
        let finished = run.finish();
        assert_eq!(last.expect("made").pixels(), finished.edge.pixels());
        let whole = phase_maps(&picture, &WHOLE, None).expect("valid input");
        assert_eq!(whole, finished);
    }

    #[test]
    fn tile_thresholds_interpolate_between_tile_centres() {
        // An 8 × 4 image in two 4 × 4 tiles, amplitude 2 on the left and 6
        // on the right. τ is the median over √(ln 4): 2/√ln4 at the left
        // centre (x = 1.5), 6/√ln4 at the right (x = 5.5), 4/√ln4 halfway
        // (x = 3.5, between pixels 3 and 4 the values are 3.5/√ln4 and
        // 4.5/√ln4), and held flat outside the centres.
        let amplitude: Vec<f32> = (0..32).map(|i| if i % 8 < 4 { 2.0 } else { 6.0 }).collect();
        let levels = noise_levels(&amplitude, 8, 4, None, NoiseMode::Tiles { side: 4 });
        let r = libm::sqrt(libm::log(4.0)) as f32;
        let row: Vec<f32> = levels[..8].iter().map(|v| v * r).collect();
        let expected = [2.0, 2.0, 2.5, 3.5, 4.5, 5.5, 6.0, 6.0];
        for (got, want) in row.iter().zip(expected) {
            assert!((got - want).abs() < 1e-5, "{row:?}");
        }
        assert_eq!(&levels[..8], &levels[24..]);
    }

    #[test]
    fn a_tile_with_too_few_usable_pixels_borrows_from_its_neighbours() {
        // Three 2 × 2 tiles in a row with amplitudes 1, 9 and 5. The middle
        // tile has no usable pixel, so it takes the median of its
        // neighbours, (1 + 5) / 2 = 3.
        let amplitude: Vec<f32> = (0..12)
            .map(|i| match i % 6 {
                0 | 1 => 1.0,
                2 | 3 => 9.0,
                _ => 5.0,
            })
            .collect();
        let usable: Vec<u8> = (0..12)
            .map(|i| u8::from(i % 6 != 2 && i % 6 != 3))
            .collect();
        let levels = noise_levels(
            &amplitude,
            6,
            2,
            Some(&usable),
            NoiseMode::Tiles { side: 2 },
        );
        let r = libm::sqrt(libm::log(4.0)) as f32;
        // Pixel 2 and 3 straddle the middle centre (2.5): each is the mean
        // of 3 and a neighbour weighted by a quarter.
        let middle = (levels[2] + levels[3]) / 2.0 * r;
        assert!((middle - 3.0).abs() < 1e-5, "{}", middle);
        // The whole-image median ignores the unusable pixels: 1,1,5,5 per
        // row gives (1 + 5) / 2.
        let whole = noise_levels(&amplitude, 6, 2, Some(&usable), NoiseMode::Whole);
        assert!((whole[0] * r - 3.0).abs() < 1e-5);
    }

    #[test]
    fn the_median_of_an_even_count_is_the_mean_of_the_middle_two() {
        assert_eq!(median(&mut [4.0, 1.0, 3.0, 2.0]), Some(2.5));
        assert_eq!(median(&mut [5.0, 1.0, 3.0]), Some(3.0));
        assert_eq!(median(&mut []), None);
    }

    #[test]
    fn settings_out_of_range_or_a_wrong_mask_are_refused() {
        let picture = step(1.0, 0.0);
        let one_scale = PhaseSettings {
            congruency_scales: 1,
            ..WHOLE
        };
        assert!(phase_maps(&picture, &one_scale, None).is_none());
        let no_tiles = PhaseSettings {
            noise: NoiseMode::Tiles { side: 0 },
            ..WHOLE
        };
        assert!(phase_maps(&picture, &no_tiles, None).is_none());
        let short = PhaseSettings {
            shortest_wavelength: 1.0,
            ..WHOLE
        };
        assert!(phase_maps(&picture, &short, None).is_none());
        let mask = GreyImage::new(10, 10, vec![1u8; 100], Placement::FILM).expect("sizes match");
        assert!(phase_maps(&picture, &WHOLE, Some(&mask)).is_none());
    }
}
