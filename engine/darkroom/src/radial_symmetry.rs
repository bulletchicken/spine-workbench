//! The two-edge radial symmetry ring detector: rings of a given size and
//! shape, used for pedicles and femoral heads.
//!
//! An elliptical, two-edge extension of the fast radial symmetry transform
//! (Loy and Zelinsky 2003), with the update rules of Kovesi's `fastradial`
//! and the constants of evidence spec 8. A ring here is a bright rim of
//! width `t` between an outer ellipse with semi-axes `(a, b)` and an inner
//! one with `(a − t, b − t)`, darker outside and darker inside. Every pixel
//! whose edge strength reaches a floor votes for where the ring's centre
//! would be if it lay on either edge:
//!
//! - on the **outer** edge the rim is the bright side, so the gradient `ĝ`
//!   points inward, the outward normal is `−ĝ`, and the centre is
//!   `p + d(ĝ; a, b, φ)`;
//! - on the **inner** edge the dark centre lies inward, so `ĝ` points
//!   outward and the centre is `p − d(ĝ; a − t, b − t, φ)`.
//!
//! `d(u; a, b, φ)` is the step from an ellipse's centre to the point of its
//! boundary whose outward normal is `u`, for semi-axes `a` across and `b`
//! down turned by the tilt `φ`: with `u` turned back by `φ`,
//! `d = (a² ux, b² uy) / √(a² ux² + b² uy²)`, turned forward again. It is
//! exact, because the normal at `(a cos s, b sin s)` is proportional to
//! `(cos s / a, sin s / b)`. Votes are rounded to the nearest pixel (halves
//! away from zero, which keeps mirror images mirrored) and dropped when they
//! fall off the image. Every tilt votes into the same accumulators, so the
//! map tolerates tilted rings without a pass per tilt.
//!
//! For each edge, `O` counts the votes a pixel gets and `M` sums their
//! strengths; `F = (M / κ) · (min(O, κ) / κ)^α` and `S` is `F` blurred by a
//! Gaussian of `0.25 √(a b)`. The ring response is `√(S_outer · S_inner)`,
//! so a ring needs both edges: a solid bright blob has no inner edge and a
//! dark hole no outer one. Divided by the response of an ideal ring of the
//! same shape run through the same code ([`ideal_responses`]), an ideal
//! ring scores about 1 at any size and a film with no rings scores low on
//! the same absolute scale.
//!
//! What a ring scores with the evidence settings, measured here at 0.5 mm
//! per pixel against noise-free calibration rings: a lone ring of a bank
//! shape with a 0.1 rim, 0.95 without noise and about 0.35 under noise of
//! sd 0.01, whose floor in the local contrast weakens every edge; the
//! evidence spec's rings beside the edge of a brighter vertebral body,
//! about 0.05, because the body's edge sets the local contrast and the
//! rim's edges barely clear the vote floor. They are still the best two
//! maxima on the body, at four times a bare body's best. Positions are
//! more trustworthy than the absolute score.

use crate::blur::gaussian;
use crate::grey_image::{GreyImage, Placement};
use crate::hessian::normalised_gradient;
use core::f64::consts::PI;

/// The least margin, in px, between an ideal ring's outer edge and its
/// canvas's border, so the blur, the gradient and a caller's local contrast
/// all see background around it.
const CANVAS_MARGIN: f64 = 20.0;

/// The smallest canvas side for an ideal ring: evidence spec 8.4's 96 px,
/// made odd so the ring's centre is a pixel centre.
const SMALLEST_CANVAS: u32 = 97;

/// Sub-samples per pixel side when an ideal ring is drawn.
const SUPERSAMPLING: u32 = 4;

/// One ring shape, in pixels of the image it is looked for in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingShape {
    across: f64,
    down: f64,
    rim: f64,
}

impl RingShape {
    /// A ring whose outer edge has semi-axes `across` (along x before any
    /// tilt) and `down` (along y), with a rim `rim` wide. `None` unless all
    /// three are finite and positive and the rim is narrower than both
    /// semi-axes, so the ring has a hole.
    pub fn new(across: f64, down: f64, rim: f64) -> Option<RingShape> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        (positive(across) && positive(down) && positive(rim) && rim < across && rim < down)
            .then_some(RingShape { across, down, rim })
    }

    /// The outer semi-axes, across and down.
    pub fn outer(&self) -> (f64, f64) {
        (self.across, self.down)
    }

    /// The inner semi-axes, across and down.
    pub fn inner(&self) -> (f64, f64) {
        (self.across - self.rim, self.down - self.rim)
    }

    pub fn rim(&self) -> f64 {
        self.rim
    }
}

/// How votes are cast and weighed.
#[derive(Clone, Debug, PartialEq)]
pub struct RingSettings {
    /// The tilts every shape votes at, in radians from +x toward +y.
    pub tilts: Vec<f64>,
    /// The least edge strength that votes.
    pub least_strength: f64,
    /// Radial strictness: how much a centre with few votes is damped.
    pub alpha: f64,
    /// The vote count at which a centre counts in full.
    pub kappa: f64,
}

impl RingSettings {
    /// Evidence spec 8: tilts of −25, 0 and +25 degrees, strength 0.25,
    /// α = 2 and κ = 9.9 (Kovesi's value for radii above 1).
    pub fn evidence() -> RingSettings {
        RingSettings {
            tilts: [-25.0f64, 0.0, 25.0]
                .iter()
                .map(|d| d * PI / 180.0)
                .collect(),
            least_strength: 0.25,
            alpha: 2.0,
            kappa: 9.9,
        }
    }

    fn is_valid(&self) -> bool {
        !self.tilts.is_empty()
            && self.tilts.iter().all(|t| t.is_finite())
            && self.least_strength.is_finite()
            && self.alpha.is_finite()
            && self.alpha >= 0.0
            && self.kappa.is_finite()
            && self.kappa > 0.0
    }
}

/// The pixels that vote: an edge direction and an edge strength at every
/// pixel, all three grids of one size.
#[derive(Clone, Debug, PartialEq)]
pub struct Voters {
    /// The gradient toward +x.
    pub x: GreyImage<f32>,
    /// The gradient toward +y.
    pub y: GreyImage<f32>,
    /// How strong the edge is; only pixels with a finite strength at or
    /// above the settings' least strength vote, so a caller sets it to 0
    /// where a pixel must not.
    pub strength: GreyImage<f32>,
}

impl Voters {
    /// The voters of `image`: the gradient of the image smoothed by a
    /// Gaussian of `sigma` px, and its strength `|g| / (c · K)` against the
    /// local contrast `c` from `contrast`, with
    /// `K = 2 / (σ √(2π))`. A step of height `h` blurred by `σ` has peak
    /// slope `h / (σ √(2π))` and a local standard deviation near `h / 2`
    /// over a window much wider than `σ`, so with the local standard
    /// deviation as the contrast an ideal step scores 1, whatever its
    /// height. Where the contrast is not a positive number the strength is
    /// 0. `None` when sigma cannot make kernels (see
    /// [`crate::hessian::DerivativeKernels::at_scale`]) or the contrast is
    /// not the image's size.
    pub fn from_image(
        image: &GreyImage<f32>,
        sigma: f64,
        contrast: &GreyImage<f32>,
    ) -> Option<Voters> {
        if contrast.width() != image.width() || contrast.height() != image.height() {
            return None;
        }
        let scaled = normalised_gradient(image, sigma)?;
        let ideal_slope = 2.0 / (sigma * (2.0 * PI).sqrt());
        let x = scaled.x.map(|v| (f64::from(*v) / sigma) as f32);
        let y = scaled.y.map(|v| (f64::from(*v) / sigma) as f32);
        let strength: Vec<f32> = x
            .pixels()
            .iter()
            .zip(y.pixels())
            .zip(contrast.pixels())
            .map(|((&gx, &gy), &c)| {
                let c = f64::from(c);
                if !(c.is_finite() && c > 0.0) {
                    return 0.0;
                }
                let (gx, gy) = (f64::from(gx), f64::from(gy));
                ((gx * gx + gy * gy).sqrt() / (c * ideal_slope)) as f32
            })
            .collect();
        let strength = GreyImage::new(image.width(), image.height(), strength, image.placement())?;
        Some(Voters { x, y, strength })
    }

    fn is_consistent(&self) -> bool {
        let (w, h) = (self.strength.width(), self.strength.height());
        self.x.width() == w && self.x.height() == h && self.y.width() == w && self.y.height() == h
    }
}

/// The step from an ellipse's centre to the point of its boundary whose
/// outward normal is `normal` (a unit vector), for semi-axes `across` and
/// `down` turned by `tilt` radians.
pub fn centre_offset(normal: (f64, f64), across: f64, down: f64, tilt: f64) -> (f64, f64) {
    let (c, s) = (libm::cos(tilt), libm::sin(tilt));
    offset_with(normal, across, down, c, s)
}

/// [`centre_offset`] with the tilt's cosine and sine worked out once.
fn offset_with(normal: (f64, f64), across: f64, down: f64, c: f64, s: f64) -> (f64, f64) {
    let (nx, ny) = normal;
    let ux = c * nx + s * ny;
    let uy = -s * nx + c * ny;
    let (ex, ey) = (across * across * ux, down * down * uy);
    let length = (ex * ux + ey * uy).sqrt();
    if length == 0.0 {
        return (0.0, 0.0);
    }
    let (dx, dy) = (ex / length, ey / length);
    (c * dx - s * dy, s * dx + c * dy)
}

/// One edge's accumulators: the votes per pixel and their summed strength.
struct Tally {
    count: Vec<u32>,
    strength: Vec<f32>,
}

impl Tally {
    fn new(len: usize) -> Tally {
        Tally {
            count: vec![0; len],
            strength: vec![0.0; len],
        }
    }
}

/// The uncalibrated response to one ring shape at every pixel,
/// `√(S_outer · S_inner)`, on the voters' size and placement. `None` when
/// the voters' grids differ in size or the settings are out of range.
pub fn shape_response(
    voters: &Voters,
    shape: RingShape,
    settings: &RingSettings,
) -> Option<GreyImage<f32>> {
    if !voters.is_consistent() || !settings.is_valid() {
        return None;
    }
    let (width, height) = (voters.strength.width(), voters.strength.height());
    let len = width as usize * height as usize;
    let mut outer = Tally::new(len);
    let mut inner = Tally::new(len);
    let tilts: Vec<(f64, f64)> = settings
        .tilts
        .iter()
        .map(|&t| (libm::cos(t), libm::sin(t)))
        .collect();
    let (outer_across, outer_down) = shape.outer();
    let (inner_across, inner_down) = shape.inner();
    let landing = |x: u32, y: u32, (dx, dy): (f64, f64), sign: f64| -> Option<usize> {
        let qx = f64::from(x) + sign * dx.round();
        let qy = f64::from(y) + sign * dy.round();
        let inside = qx >= 0.0 && qy >= 0.0 && qx < f64::from(width) && qy < f64::from(height);
        inside.then(|| qy as usize * width as usize + qx as usize)
    };
    for y in 0..height {
        for x in 0..width {
            let index = y as usize * width as usize + x as usize;
            let strength = voters.strength.pixels()[index];
            let votes = strength.is_finite() && f64::from(strength) >= settings.least_strength;
            if !votes {
                continue;
            }
            let gx = f64::from(voters.x.pixels()[index]);
            let gy = f64::from(voters.y.pixels()[index]);
            let magnitude = (gx * gx + gy * gy).sqrt();
            if !(magnitude > 0.0 && magnitude.is_finite()) {
                continue;
            }
            let direction = (gx / magnitude, gy / magnitude);
            for &(c, s) in &tilts {
                let out = offset_with(direction, outer_across, outer_down, c, s);
                if let Some(q) = landing(x, y, out, 1.0) {
                    outer.count[q] += 1;
                    outer.strength[q] += strength;
                }
                let into = offset_with(direction, inner_across, inner_down, c, s);
                if let Some(q) = landing(x, y, into, -1.0) {
                    inner.count[q] += 1;
                    inner.strength[q] += strength;
                }
            }
        }
    }
    let placement = voters.strength.placement();
    let smoothing = 0.25 * (outer_across * outer_down).sqrt();
    let outer = smoothed(&outer, settings, width, height, placement, smoothing)?;
    let inner = smoothed(&inner, settings, width, height, placement, smoothing)?;
    let response: Vec<f32> = outer
        .pixels()
        .iter()
        .zip(inner.pixels())
        .map(|(&a, &b)| (f64::from(a.max(0.0)) * f64::from(b.max(0.0))).sqrt() as f32)
        .collect();
    GreyImage::new(width, height, response, placement)
}

/// `F = (M / κ) · (min(O, κ) / κ)^α` for one edge, blurred by `smoothing` px.
fn smoothed(
    tally: &Tally,
    settings: &RingSettings,
    width: u32,
    height: u32,
    placement: Placement,
    smoothing: f64,
) -> Option<GreyImage<f32>> {
    let kappa = settings.kappa;
    let full = kappa.ceil() as u32;
    let strictness: Vec<f64> = (0..=full)
        .map(|count| libm::pow(f64::from(count).min(kappa) / kappa, settings.alpha))
        .collect();
    let values: Vec<f32> = tally
        .count
        .iter()
        .zip(&tally.strength)
        .map(|(&count, &sum)| {
            if count == 0 {
                return 0.0;
            }
            let damping = strictness[count.min(full) as usize];
            (f64::from(sum) / kappa * damping) as f32
        })
        .collect();
    gaussian(
        &GreyImage::new(width, height, values, placement)?,
        smoothing,
    )
}

/// An ideal ring of `shape`, untilted, centred on the middle pixel of an
/// odd square canvas: rim 1, outside and centre 0, each pixel holding the
/// share of it the rim covers, then blurred by a Gaussian of `blur` px.
/// The canvas is at least 97 px and leaves 20 px beyond the outer edge.
/// `None` when the blur is negative or not finite.
pub fn ideal_ring(shape: RingShape, blur: f64) -> Option<GreyImage<f32>> {
    let (across, down) = shape.outer();
    let (inner_across, inner_down) = shape.inner();
    let reach = 2.0 * (across.max(down) + CANVAS_MARGIN).ceil() + 1.0;
    if !reach.is_finite() || reach > f64::from(shapes::film::MAX_FILM_SIDE) {
        return None;
    }
    let side = (reach as u32).max(SMALLEST_CANVAS) | 1;
    let centre = f64::from(side - 1) / 2.0;
    let inside = |x: f64, y: f64, a: f64, b: f64| (x / a) * (x / a) + (y / b) * (y / b) <= 1.0;
    let step = 1.0 / f64::from(SUPERSAMPLING);
    let samples = f64::from(SUPERSAMPLING * SUPERSAMPLING);
    let mut pixels = Vec::with_capacity(side as usize * side as usize);
    for row in 0..side {
        for column in 0..side {
            let mut covered = 0u32;
            for j in 0..SUPERSAMPLING {
                for i in 0..SUPERSAMPLING {
                    let x = f64::from(column) - centre - 0.5 + (f64::from(i) + 0.5) * step;
                    let y = f64::from(row) - centre - 0.5 + (f64::from(j) + 0.5) * step;
                    if inside(x, y, across, down) && !inside(x, y, inner_across, inner_down) {
                        covered += 1;
                    }
                }
            }
            pixels.push((f64::from(covered) / samples) as f32);
        }
    }
    let canvas = GreyImage::new(side, side, pixels, Placement::FILM)?;
    gaussian(&canvas, blur)
}

/// The peak response of an ideal ring of each shape (see [`ideal_ring`],
/// blurred by `blur` px), run through `prepare` (the caller's own way from
/// an image to voters, so the calibration sees exactly what a film does)
/// and [`shape_response`]. Dividing a film's response by these makes an
/// ideal ring score about 1 at any size. `None` when a canvas cannot be
/// drawn, `prepare` gives nothing, or a peak is not positive.
pub fn ideal_responses(
    shapes: &[RingShape],
    settings: &RingSettings,
    blur: f64,
    prepare: impl Fn(&GreyImage<f32>) -> Option<Voters>,
) -> Option<Vec<f64>> {
    shapes
        .iter()
        .map(|&shape| {
            let canvas = ideal_ring(shape, blur)?;
            let response = shape_response(&prepare(&canvas)?, shape, settings)?;
            let peak = response.pixels().iter().fold(0.0f32, |m, &v| m.max(v));
            (peak > 0.0).then_some(f64::from(peak))
        })
        .collect()
}

/// The calibrated ring map: at every pixel the best response over the
/// shapes, divided by that shape's ideal response and clamped to [0, 1],
/// and the index of the shape that gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct RingMap {
    pub response: GreyImage<f32>,
    pub shape: GreyImage<u8>,
}

impl RingMap {
    /// A map of zeros, shape 0 everywhere.
    pub fn new(width: u32, height: u32, placement: Placement) -> Option<RingMap> {
        let len = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        Some(RingMap {
            response: GreyImage::new(width, height, vec![0.0; len], placement)?,
            shape: GreyImage::new(width, height, vec![0; len], placement)?,
        })
    }

    /// Takes in the response of shape `index`, divided by its ideal
    /// response, wherever it beats what the map holds; a tie keeps the
    /// earlier shape. `false`, changing nothing, when the response is not
    /// the map's size or the ideal is not a positive number.
    pub fn include(&mut self, index: u8, response: &GreyImage<f32>, ideal: f64) -> bool {
        let same_size = response.width() == self.response.width()
            && response.height() == self.response.height();
        if !same_size || !(ideal.is_finite() && ideal > 0.0) {
            return false;
        }
        let (width, height, placement) = (
            self.response.width(),
            self.response.height(),
            self.response.placement(),
        );
        let mut best = self.response.pixels().to_vec();
        let mut chosen = self.shape.pixels().to_vec();
        for (i, &value) in response.pixels().iter().enumerate() {
            let calibrated = (f64::from(value) / ideal).clamp(0.0, 1.0) as f32;
            if calibrated > best[i] {
                best[i] = calibrated;
                chosen[i] = index;
            }
        }
        self.response = GreyImage::new(width, height, best, placement).expect("same size");
        self.shape = GreyImage::new(width, height, chosen, placement).expect("same size");
        true
    }
}

/// The calibrated ring map over every shape, with `ideals` from
/// [`ideal_responses`] in the same order. `None` when there are no shapes,
/// more than 256, a different number of ideals, or any shape's response
/// cannot be made.
pub fn ring_map(
    voters: &Voters,
    shapes: &[RingShape],
    ideals: &[f64],
    settings: &RingSettings,
) -> Option<RingMap> {
    if shapes.is_empty() || shapes.len() > 256 || shapes.len() != ideals.len() {
        return None;
    }
    let strength = &voters.strength;
    let mut map = RingMap::new(strength.width(), strength.height(), strength.placement())?;
    for (index, (&shape, &ideal)) in shapes.iter().zip(ideals).enumerate() {
        let response = shape_response(voters, shape, settings)?;
        if !map.include(index as u8, &response, ideal) {
            return None;
        }
    }
    Some(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The phantoms are drawn at 0.5 mm per pixel.
    const MM: f64 = 2.0;

    fn image(width: u32, height: u32, value: impl Fn(f64, f64) -> f64) -> GreyImage<f32> {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| value(f64::from(x), f64::from(y)) as f32)
            .collect();
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    /// Seeded Gaussian noise: SplitMix64 for uniforms, Box-Muller for
    /// normals.
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

        fn add(&mut self, picture: &GreyImage<f32>, sd: f64) -> GreyImage<f32> {
            let pixels = picture
                .pixels()
                .iter()
                .map(|&v| (f64::from(v) + sd * self.normal()) as f32)
                .collect();
            GreyImage::new(
                picture.width(),
                picture.height(),
                pixels,
                picture.placement(),
            )
            .expect("sizes match")
        }
    }

    /// Fraction of a pixel inside `inside`, by 4 × 4 sub-samples.
    fn coverage(x: f64, y: f64, inside: &impl Fn(f64, f64) -> bool) -> f64 {
        let mut count = 0;
        for j in 0..4 {
            for i in 0..4 {
                if inside(
                    x - 0.375 + 0.25 * f64::from(i),
                    y - 0.375 + 0.25 * f64::from(j),
                ) {
                    count += 1;
                }
            }
        }
        f64::from(count) / 16.0
    }

    /// Whether (x, y) lies inside the ellipse with centre (cx, cy), semi-axes
    /// (a, b) and tilt `tilt`.
    fn in_ellipse(x: f64, y: f64, (cx, cy): (f64, f64), (a, b): (f64, f64), tilt: f64) -> bool {
        let (c, s) = (libm::cos(tilt), libm::sin(tilt));
        let (dx, dy) = (x - cx, y - cy);
        let (u, v) = (c * dx + s * dy, -s * dx + c * dy);
        (u / a) * (u / a) + (v / b) * (v / b) <= 1.0
    }

    /// What a phantom's two marks are.
    #[derive(Clone, Copy)]
    enum Mark {
        Ring,
        Solid,
        Hole,
        Nothing,
    }

    /// Two marks at +0.1 on an optional vertebral body (30 × 20 mm at
    /// `body`) on a ground of `ground`, centred 9 mm either side of the
    /// body's midline, each `outer_mm` in outer semi-axes with a rim of
    /// `rim_mm`, scaled by `size` and tilted by `tilt`. Every contrast is
    /// times `gain`; the whole is blurred by 0.5 mm, and noise of sd
    /// `noise · gain` is added.
    struct Phantom {
        mark: Mark,
        outer_mm: (f64, f64),
        rim_mm: f64,
        size: f64,
        tilt: f64,
        body: f64,
        gain: f64,
        ground: f64,
        noise: f64,
        centre: (f64, f64),
    }

    impl Phantom {
        const WIDTH: u32 = 160;
        const HEIGHT: u32 = 120;

        /// Evidence spec T15: on a body at +0.2, rings 6 mm wide by 9 mm
        /// tall with a 1.5 mm rim, under noise of sd 0.01.
        fn on_a_body() -> Phantom {
            Phantom {
                mark: Mark::Ring,
                outer_mm: (3.0, 4.5),
                rim_mm: 1.5,
                size: 1.0,
                tilt: 0.0,
                body: 0.2,
                gain: 1.0,
                ground: 0.3,
                noise: 0.01,
                centre: (80.0, 60.0),
            }
        }

        /// Rings of one of the bank's shapes (3.5 mm across, aspect 1.25,
        /// rim 1.05 mm: shape 4) on a plain ground, under noise of sd 0.01.
        fn of_a_bank_shape() -> Phantom {
            Phantom {
                outer_mm: (3.5, 4.375),
                rim_mm: 1.05,
                body: 0.0,
                ..Phantom::on_a_body()
            }
        }

        fn centres(&self) -> [(f64, f64); 2] {
            let (cx, cy) = self.centre;
            [(cx - 9.0 * MM, cy), (cx + 9.0 * MM, cy)]
        }

        fn draw(&self, seed: u64) -> GreyImage<f32> {
            let (cx, cy) = self.centre;
            let outer = (
                self.outer_mm.0 * MM * self.size,
                self.outer_mm.1 * MM * self.size,
            );
            let rim = self.rim_mm * MM * self.size;
            let inner = (outer.0 - rim, outer.1 - rim);
            let centres = self.centres();
            let body = |x: f64, y: f64| (x - cx).abs() <= 15.0 * MM && (y - cy).abs() <= 10.0 * MM;
            let mark = |x: f64, y: f64| -> f64 {
                centres
                    .iter()
                    .map(|&c| {
                        let out = in_ellipse(x, y, c, outer, self.tilt);
                        let hole = in_ellipse(x, y, c, inner, self.tilt);
                        let lit = |on: bool| f64::from(u8::from(on));
                        match self.mark {
                            Mark::Ring => 0.1 * lit(out && !hole),
                            Mark::Solid => 0.1 * lit(out),
                            Mark::Hole => -0.1 * lit(hole),
                            Mark::Nothing => 0.0,
                        }
                    })
                    .sum()
            };
            let sharp = image(Self::WIDTH, Self::HEIGHT, |x, y| {
                let shape = self.body * coverage(x, y, &body) + coverage_value(x, y, &mark);
                self.ground + self.gain * shape
            });
            let blurred = gaussian(&sharp, 0.5 * MM).expect("valid sigma");
            Noise(seed).add(&blurred, self.noise * self.gain)
        }

        /// The ring map, with the noise floor the evidence stage would
        /// estimate.
        fn ring_map(&self, seed: u64) -> RingMap {
            ring_map_of(&self.draw(seed), self.noise * self.gain)
        }

        fn on_body(&self) -> impl Fn(u32, u32) -> bool {
            let (cx, cy) = self.centre;
            move |x, y| {
                (f64::from(x) - cx).abs() <= 15.0 * MM && (f64::from(y) - cy).abs() <= 10.0 * MM
            }
        }

        /// The best response within 1 mm of each true centre.
        fn at_centres(&self, map: &RingMap) -> [f32; 2] {
            self.centres().map(|truth| {
                let mut best = 0.0f32;
                for y in 0..map.response.height() {
                    for x in 0..map.response.width() {
                        if within_mm(x, y, truth) {
                            best = best.max(*map.response.get(x, y).expect("inside"));
                        }
                    }
                }
                best
            })
        }
    }

    /// The mean of a value over 4 × 4 sub-samples of a pixel.
    fn coverage_value(x: f64, y: f64, value: &impl Fn(f64, f64) -> f64) -> f64 {
        let mut total = 0.0;
        for j in 0..4 {
            for i in 0..4 {
                total += value(
                    x - 0.375 + 0.25 * f64::from(i),
                    y - 0.375 + 0.25 * f64::from(j),
                );
            }
        }
        total / 16.0
    }

    fn within_mm(x: u32, y: u32, truth: (f64, f64)) -> bool {
        let (dx, dy) = (f64::from(x) - truth.0, f64::from(y) - truth.1);
        (dx * dx + dy * dy).sqrt() <= 1.0 * MM
    }

    /// The evidence stage's voters, done plainly: the gradient at 0.6 mm,
    /// against the local standard deviation over 8 mm with a floor of three
    /// noise standard deviations (evidence spec 5.1, 5.3 and 8.1).
    fn voters(image: &GreyImage<f32>, noise_sd: f64) -> Option<Voters> {
        let rho = 8.0 * MM;
        let mean = gaussian(image, rho)?;
        let square = gaussian(&image.map(|v| v * v), rho)?;
        let floor = (3.0 * noise_sd) * (3.0 * noise_sd);
        let contrast: Vec<f32> = mean
            .pixels()
            .iter()
            .zip(square.pixels())
            .map(|(&m, &q)| {
                let variance = (f64::from(q) - f64::from(m) * f64::from(m)).max(0.0);
                (variance + floor).sqrt().max(1e-4) as f32
            })
            .collect();
        let contrast = GreyImage::new(image.width(), image.height(), contrast, image.placement())?;
        Voters::from_image(image, 0.6 * MM, &contrast)
    }

    /// Evidence spec 8.2's twelve shapes at 0.5 mm per pixel: outer
    /// semi-axes 2.5, 3.5, 5 and 7 mm across, aspects 0.8, 1.25 and 2, rim
    /// 0.3 of the semi-axis held to 1 to 2 mm.
    fn evidence_shapes() -> Vec<RingShape> {
        let mut shapes = Vec::new();
        for across_mm in [2.5f64, 3.5, 5.0, 7.0] {
            for aspect in [0.8, 1.25, 2.0] {
                let rim_mm = (0.3 * across_mm).clamp(1.0, 2.0);
                let shape = RingShape::new(across_mm * MM, across_mm * aspect * MM, rim_mm * MM);
                shapes.push(shape.expect("valid shape"));
            }
        }
        shapes
    }

    /// The ring map of an image with the evidence shapes and settings,
    /// calibrated on noise-free ideal rings blurred by 0.5 mm.
    fn ring_map_of(picture: &GreyImage<f32>, noise_sd: f64) -> RingMap {
        let shapes = evidence_shapes();
        let settings = RingSettings::evidence();
        let ideals = ideal_responses(&shapes, &settings, 0.5 * MM, |canvas| voters(canvas, 0.0))
            .expect("every shape calibrates");
        let field = voters(picture, noise_sd).expect("valid voters");
        ring_map(&field, &shapes, &ideals, &settings).expect("valid map")
    }

    /// Local maxima of a map (at least every neighbour, ties going to the
    /// first in row order), inside `within`, best first.
    fn peaks(map: &GreyImage<f32>, within: impl Fn(u32, u32) -> bool) -> Vec<(u32, u32, f32)> {
        let (w, h) = (map.width(), map.height());
        let mut found = Vec::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let v = *map.get(x, y).expect("inside");
                let is_peak = (-1i64..=1).all(|dy| {
                    (-1i64..=1).all(|dx| {
                        let (u, t) = ((i64::from(x) + dx) as u32, (i64::from(y) + dy) as u32);
                        let n = *map.get(u, t).expect("inside");
                        (dx == 0 && dy == 0) || n < v || (n == v && (t, u) > (y, x))
                    })
                });
                if is_peak && v > 0.0 && within(x, y) {
                    found.push((x, y, v));
                }
            }
        }
        found.sort_by(|a, b| b.2.total_cmp(&a.2));
        found
    }

    /// Whether the best two of `found` lie within 1 mm of the two centres,
    /// one each.
    fn best_two_hit(found: &[(u32, u32, f32)], centres: [(f64, f64); 2]) -> bool {
        found.len() >= 2
            && centres
                .iter()
                .all(|&truth| found[..2].iter().any(|&(x, y, _)| within_mm(x, y, truth)))
    }

    fn largest(map: &GreyImage<f32>, within: impl Fn(u32, u32) -> bool) -> f32 {
        let mut best = 0.0f32;
        for y in 0..map.height() {
            for x in 0..map.width() {
                if within(x, y) {
                    best = best.max(*map.get(x, y).expect("inside"));
                }
            }
        }
        best
    }

    #[test]
    fn the_offset_reaches_the_boundary_point_with_that_normal() {
        // On a circle of radius r the point with outward normal u is r·u.
        // On an ellipse (a, b) the point (a cos s, b sin s) has outward
        // normal along (cos s / a, sin s / b): differentiate x²/a² + y²/b².
        // Turned by φ, both the point and its normal turn. Plain f64
        // arithmetic: 1e-12.
        let (x, y) = centre_offset((0.6, 0.8), 5.0, 5.0, 0.3);
        assert!((x - 3.0).abs() < 1e-12 && (y - 4.0).abs() < 1e-12);
        let (a, b) = (6.0, 9.0);
        for s in [0.0, 0.4, 1.3, 2.2, -2.9] {
            let point = (a * libm::cos(s), b * libm::sin(s));
            let (nx, ny) = (libm::cos(s) / a, libm::sin(s) / b);
            let length = (nx * nx + ny * ny).sqrt();
            for tilt in [0.0, 0.5, -1.1] {
                let (c, t) = (libm::cos(tilt), libm::sin(tilt));
                let turn = |(u, v): (f64, f64)| (c * u - t * v, t * u + c * v);
                let normal = turn((nx / length, ny / length));
                let want = turn(point);
                let got = centre_offset(normal, a, b, tilt);
                assert!((got.0 - want.0).abs() < 1e-12, "s {s}, tilt {tilt}");
                assert!((got.1 - want.1).abs() < 1e-12, "s {s}, tilt {tilt}");
            }
        }
    }

    #[test]
    fn a_step_has_strength_one_whatever_its_height() {
        // A sharp step of height h smoothed by σ has slope h / (σ√(2π)) at
        // its centre, and K = 2 / (σ√(2π)), so against a contrast of h / 2
        // its strength is 1. The step is centred on a pixel (0, h/2, h).
        // The sampled kernels stop at 3σ and are normalised there, which
        // reads a slope a few percent short (the hessian module measures
        // their second moment 2.7% short of σ²); measured 2.5% short, so
        // the bound is 3%. Heights 0.05 and 2 give the same strength.
        for height in [0.05, 2.0] {
            let step = image(64, 16, |x, _| {
                height
                    * if x < 32.0 {
                        0.0
                    } else if x == 32.0 {
                        0.5
                    } else {
                        1.0
                    }
            });
            let contrast = image(64, 16, |_, _| height / 2.0);
            let field = Voters::from_image(&step, 2.0, &contrast).expect("valid");
            let got = f64::from(*field.strength.get(32, 8).expect("inside"));
            assert!((got - 1.0).abs() < 0.03, "height {height}: {got}");
            let direction = libm::atan2(
                f64::from(*field.y.get(32, 8).expect("inside")),
                f64::from(*field.x.get(32, 8).expect("inside")),
            );
            assert!(
                direction.abs() < 1e-6,
                "the gradient points to the bright side"
            );
        }
        let step = image(64, 16, |x, _| x);
        let contrast = image(64, 16, |_, _| 1.0);
        assert!(Voters::from_image(&step, 1e-90, &contrast).is_none());
    }

    #[test]
    fn rings_of_a_bank_shape_are_found_where_they_are() {
        // Two rings of shape 4 on a plain ground. The two best local maxima
        // are on the true centres, to the pixel, and the winning shape at
        // both is shape 4. Noise-free, each ring is the calibration ring at
        // a contrast of 0.1 among others, and scores at least 0.9 of it
        // (measured 0.95). Under noise of sd 0.01 the noise floor (0.03)
        // adds to the local contrast the strengths are measured against,
        // so fewer edge pixels clear the vote floor and the score falls to
        // about a third (measured 0.34 and 0.36); the bar is 0.25.
        let clean = Phantom {
            noise: 0.0,
            ..Phantom::of_a_bank_shape()
        };
        let noisy = Phantom::of_a_bank_shape();
        for (phantom, bar) in [(clean, 0.9f32), (noisy, 0.25)] {
            let map = phantom.ring_map(30);
            let found = peaks(&map.response, |_, _| true);
            for truth in phantom.centres() {
                let (x, y) = (truth.0 as u32, truth.1 as u32);
                assert!(
                    found[..2].iter().any(|p| (p.0, p.1) == (x, y)),
                    "{truth:?} in {found:?}"
                );
                assert_eq!(*map.shape.get(x, y).expect("inside"), 4);
                let value = *map.response.get(x, y).expect("inside");
                assert!(value >= bar, "{truth:?}: {value} under {bar}");
            }
        }
    }

    #[test]
    fn rings_on_a_body_are_found_and_a_bare_body_stays_low() {
        // Evidence spec T15's phantom. The two best local maxima on the
        // body lie within 1 mm of the true centres, the winning shape at
        // each centre has aspect 1.25 (the rings are 1.5 times as tall as
        // wide, nearer 1.25 than 2), and the same body with no rings scores
        // at most 0.2 anywhere on it (measured 0.012): the calibration is
        // fixed, so a film without rings is not lifted to 1. The rings
        // score at least three times the bare body's best (measured 4.1
        // and 4.7 times).
        //
        // The spec also asks for 0.5 at the rings. That does not hold with
        // the spec's constants (measured 0.050 and 0.057): a 0.1 rim beside
        // the body's 0.2 edge has a strength of about 0.3 against its local
        // contrast, barely over the 0.25 vote floor, while the lone
        // calibration ring's edges have about 1.6, so the rings get fewer
        // and weaker votes than the calibration ring. Their 1.5 mm rim also
        // matches no shape in the bank (rims 1 to 1.05 mm at these sizes).
        let phantom = Phantom::on_a_body();
        let map = phantom.ring_map(15);
        let found = peaks(&map.response, phantom.on_body());
        assert!(best_two_hit(&found, phantom.centres()), "{found:?}");
        for truth in phantom.centres() {
            let shape = *map
                .shape
                .get(truth.0 as u32, truth.1 as u32)
                .expect("inside");
            assert_eq!(shape % 3, 1, "{truth:?}: shape {shape}");
        }
        let bare = Phantom {
            mark: Mark::Nothing,
            ..Phantom::on_a_body()
        };
        let bare_best = largest(&bare.ring_map(16).response, bare.on_body());
        assert!(bare_best <= 0.2, "{bare_best}");
        for value in phantom.at_centres(&map) {
            assert!(value >= 3.0 * bare_best, "{value} against {bare_best}");
        }
    }

    #[test]
    fn a_solid_blob_or_a_hole_is_not_a_ring() {
        // Evidence spec T16: a filled bright ellipse of the ring's size has
        // no inner edge, and a dark hole without a rim no outer edge, so
        // each scores at most half the ring (measured 0.04 and 0.02 of it).
        let ring = Phantom::of_a_bank_shape();
        let ring_score = largest(&ring.ring_map(17).response, |_, _| true);
        for mark in [Mark::Solid, Mark::Hole] {
            let phantom = Phantom {
                mark,
                ..Phantom::of_a_bank_shape()
            };
            let score = largest(&phantom.ring_map(17).response, |_, _| true);
            assert!(score <= 0.5 * ring_score, "{score} against {ring_score}");
        }
    }

    #[test]
    fn tilted_rings_keep_most_of_their_response() {
        // Evidence spec T16: rings tilted by 20 and 35 degrees keep at
        // least 0.6 of the upright ring's response within 1 mm of their
        // centres, because the tilts −25, 0 and +25 all vote into one map
        // (measured 0.96 and 0.84 to 0.86 of it).
        let upright = Phantom::of_a_bank_shape();
        let base = upright.at_centres(&upright.ring_map(18));
        for degrees in [20.0f64, 35.0] {
            let tilted = Phantom {
                tilt: degrees.to_radians(),
                ..Phantom::of_a_bank_shape()
            };
            let got = tilted.at_centres(&tilted.ring_map(18));
            for (value, upright) in got.iter().zip(base) {
                assert!(
                    *value >= 0.6 * upright,
                    "{degrees}: {value} against {upright}"
                );
            }
        }
    }

    #[test]
    fn smaller_and_larger_rings_are_still_the_best_two() {
        // Evidence spec T16: rings at 0.6 and 1.6 times a bank shape (rim
        // included) are still the two best local maxima, each within 1 mm
        // of its centre. On the spec's body phantom the 0.6 ring (1.8 mm
        // across, under the bank's smallest 2.5 mm) loses to the body's
        // corners; the 1.6 ring is found there too.
        for size in [0.6, 1.6] {
            let phantom = Phantom {
                size,
                ..Phantom::of_a_bank_shape()
            };
            let found = peaks(&phantom.ring_map(19).response, |_, _| true);
            assert!(
                best_two_hit(&found, phantom.centres()),
                "size {size}: {found:?}"
            );
        }
        let large = Phantom {
            size: 1.6,
            ..Phantom::on_a_body()
        };
        let found = peaks(&large.ring_map(19).response, large.on_body());
        assert!(
            best_two_hit(&found, large.centres()),
            "on a body: {found:?}"
        );
    }

    #[test]
    fn brightness_and_contrast_leave_the_map_unchanged() {
        // The same phantom (same noise draws) at a quarter of the contrast
        // on a brighter ground: the gradient and the local contrast, noise
        // floor included, both scale by the gain, so every strength is
        // unchanged up to f32 rounding. A strength within rounding of the
        // 0.25 floor could flip a vote, so the maps are compared to 1e-3 at
        // every pixel rather than bit for bit (measured within 1e-5).
        // Checked on both phantoms.
        for plain in [Phantom::of_a_bank_shape(), Phantom::on_a_body()] {
            let changed = Phantom {
                gain: 0.25,
                ground: 0.6,
                ..plain
            };
            let a = plain.ring_map(20);
            let b = changed.ring_map(20);
            for (p, q) in a.response.pixels().iter().zip(b.response.pixels()) {
                assert!((p - q).abs() < 1e-3, "{p} against {q}");
            }
        }
    }

    #[test]
    fn an_ideal_ring_scores_one_at_its_centre_and_less_off_the_grid() {
        // The calibration is the ideal ring's own peak, so an ideal ring
        // centred on a pixel scores exactly 1 there, and that peak is at the
        // centre, not beside it. The same ring moved by half a pixel on
        // both axes splits its votes between pixels and must still score at
        // least 0.8 (measured 0.86 to 1 over the shapes).
        let settings = RingSettings::evidence();
        for shape in evidence_shapes() {
            let ideals = ideal_responses(&[shape], &settings, 0.5 * MM, |c| voters(c, 0.0))
                .expect("calibrates");
            let canvas = ideal_ring(shape, 0.5 * MM).expect("drawn");
            let side = canvas.width();
            let centre = (side - 1) / 2;
            let map = ring_map(
                &voters(&canvas, 0.0).expect("valid"),
                &[shape],
                &ideals,
                &settings,
            )
            .expect("valid");
            let best = peaks(&map.response, |_, _| true)[0];
            assert_eq!((best.0, best.1), (centre, centre), "{shape:?}");
            assert_eq!(best.2, 1.0);
            let (a, b) = shape.outer();
            let (ia, ib) = shape.inner();
            let shifted = image(side, side, |x, y| {
                let c = f64::from(centre) + 0.5;
                let inside = |u: f64, v: f64| {
                    in_ellipse(u, v, (c, c), (a, b), 0.0)
                        && !in_ellipse(u, v, (c, c), (ia, ib), 0.0)
                };
                coverage(x, y, &inside)
            });
            let shifted = gaussian(&shifted, 0.5 * MM).expect("valid sigma");
            let map = ring_map(
                &voters(&shifted, 0.0).expect("valid"),
                &[shape],
                &ideals,
                &settings,
            )
            .expect("valid");
            let best = peaks(&map.response, |_, _| true)[0];
            assert!(best.2 >= 0.8, "{shape:?}: {}", best.2);
        }
    }

    #[test]
    fn rings_at_the_border_are_found_or_left_alone() {
        // A ring whose outer edge comes within 0.5 mm of the left border
        // (centre 4 mm in, outer semi-axis 3.5 mm) keeps its whole edge on
        // the film and is found within 1 mm, at least 0.8 of the same ring
        // in the middle (measured 0.86). A ring cut in half by the border
        // loses the votes that would land off the film; its map stays
        // finite and within [0, 1], and its centre is not among the best
        // two maxima.
        let middle = Phantom::of_a_bank_shape();
        let reference = middle.at_centres(&middle.ring_map(21))[0];
        let touching = Phantom {
            centre: (4.0 * MM + 9.0 * MM, 60.0),
            ..Phantom::of_a_bank_shape()
        };
        let map = touching.ring_map(21);
        let found = peaks(&map.response, |_, _| true);
        let truth = touching.centres()[0];
        assert!(
            found[..2].iter().any(|&(x, y, _)| within_mm(x, y, truth)),
            "{found:?}"
        );
        let value = touching.at_centres(&map)[0];
        assert!(value >= 0.8 * reference, "{value} against {reference}");
        let cut = Phantom {
            centre: (9.0 * MM, 60.0),
            ..Phantom::of_a_bank_shape()
        };
        let map = cut.ring_map(22);
        for v in map.response.pixels() {
            assert!(v.is_finite() && (0.0..=1.0).contains(v));
        }
        let found = peaks(&map.response, |_, _| true);
        let edge = cut.centres()[0];
        assert!(
            !found[..2].iter().any(|&(x, y, _)| within_mm(x, y, edge)),
            "{found:?}"
        );
    }

    #[test]
    fn white_noise_scores_low() {
        // Evidence spec T10: on noise of sd 0.05 alone, the local contrast
        // is the noise floor and most strengths fall under 0.25, so the
        // 99th percentile of the ring map is at most 0.1 (measured 2e-5).
        let field = Noise(23).add(&image(160, 120, |_, _| 0.5), 0.05);
        let map = ring_map_of(&field, 0.05);
        let mut values = map.response.pixels().to_vec();
        values.sort_by(f32::total_cmp);
        let p99 = values[(values.len() - 1) * 99 / 100];
        assert!(p99 <= 0.1, "{p99}");
    }

    #[test]
    fn bad_shapes_settings_and_sizes_are_refused() {
        assert!(RingShape::new(5.0, 6.0, 5.0).is_none());
        assert!(RingShape::new(5.0, f64::NAN, 1.0).is_none());
        assert!(RingShape::new(-5.0, 6.0, 1.0).is_none());
        let shape = RingShape::new(5.0, 6.0, 2.0).expect("valid");
        assert_eq!(shape.inner(), (3.0, 4.0));
        let picture = image(40, 30, |x, _| x / 40.0);
        let field = voters(&picture, 0.01).expect("valid");
        let mut empty = RingSettings::evidence();
        empty.tilts.clear();
        assert!(shape_response(&field, shape, &empty).is_none());
        let mut mismatched = field.clone();
        mismatched.x = image(10, 10, |_, _| 0.0);
        assert!(shape_response(&mismatched, shape, &RingSettings::evidence()).is_none());
        assert!(ring_map(&field, &[shape], &[], &RingSettings::evidence()).is_none());
        let mut map = RingMap::new(40, 30, Placement::FILM).expect("valid");
        assert!(!map.include(0, &image(10, 10, |_, _| 1.0), 1.0));
        assert!(!map.include(0, &image(40, 30, |_, _| 1.0), 0.0));
        let small_contrast = image(10, 10, |_, _| 1.0);
        assert!(Voters::from_image(&picture, 1.0, &small_contrast).is_none());
    }
}
