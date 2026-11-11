//! Femoral head centres from circles fitted below the plateau.
//!
//! Gradient-directed circle votes are accumulated one radius at a time on
//! a coarse upright crop. A candidate must have inward edge support over
//! three quarters of its circumference before a least-squares circle fit
//! can produce a mark. The 16–30 mm radius range and support cutoffs are
//! experimental; absence of support leaves the landmark absent.

use crate::handoffs::{ChainBoundaries, FilmPassCache};
use crate::points::back_to_film::{facing, strip_to_film};
use darkroom::blur::gaussian;
use darkroom::gradients::{Gradients, scharr};
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::sampling::bilinear;
use nalgebra::{Matrix3, Vector3};
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::landmarks::{HipCentre, HipSide};
use shapes::marks::Mark;
use shapes::pixels::Px;

/// An upright, square-cell crop below the sacral plateau. Its grey values
/// are bone-bright in 0–1, and its placement maps cells to original pixels.
pub struct HipSearchView<'a> {
    pub image: &'a GreyImage<f32>,
    pub valid: &'a GreyImage<u8>,
    /// Millimetres, or assumed millimetres, per crop cell.
    pub units_per_pixel: f64,
    /// The plateau centre's column on this crop.
    pub midline_column: f64,
    /// Radius limits in length units: 16–30 mm, or 0.5–1.2 body periods
    /// when the film has no measured scale.
    pub radii: [f64; 2],
}

#[derive(Clone, Copy)]
struct Circle {
    x: f64,
    y: f64,
    radius: f64,
    score: f64,
}

/// Search the crop, returning at most one head per front-film side or two
/// unpaired side-film heads. Bending films never receive hip marks.
pub fn find(view: &HipSearchView<'_>, film: FilmKind, named_for: ImageRight) -> Vec<HipCentre> {
    if film.is_bending()
        || (film == FilmKind::Front) != named_for.suits_front_view()
        || !valid_view(view)
    {
        return Vec::new();
    }
    let Some(smooth) = gaussian(view.image, 0.8) else {
        return Vec::new();
    };
    let gradients = scharr(&smooth);
    let mut magnitudes: Vec<f64> = gradients
        .x
        .pixels()
        .iter()
        .zip(gradients.y.pixels())
        .zip(view.valid.pixels())
        .filter_map(|((&x, &y), &valid)| {
            let magnitude = libm::hypot(f64::from(x), f64::from(y));
            (valid != 0 && magnitude.is_finite()).then_some(magnitude)
        })
        .collect();
    if magnitudes.is_empty() {
        return Vec::new();
    }
    magnitudes.sort_by(f64::total_cmp);
    let threshold = (0.15 * magnitudes[(magnitudes.len() - 1) * 9 / 10]).max(0.01);
    let width = view.image.width();
    let height = view.image.height();
    let first = (view.radii[0] / view.units_per_pixel).ceil() as u32;
    let last = (view.radii[1] / view.units_per_pixel).floor() as u32;
    let mut circles: Vec<Circle> = Vec::new();
    for radius in first..=last {
        let mut votes = vec![0.0f32; width as usize * height as usize];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let index = y as usize * width as usize + x as usize;
                if view.valid.pixels()[index] == 0 {
                    continue;
                }
                let gx = f64::from(gradients.x.pixels()[index]);
                let gy = f64::from(gradients.y.pixels()[index]);
                let magnitude = libm::hypot(gx, gy);
                if magnitude < threshold || !magnitude.is_finite() {
                    continue;
                }
                let cx = f64::from(x) + f64::from(radius) * gx / magnitude;
                let cy = f64::from(y) + f64::from(radius) * gy / magnitude;
                vote(&mut votes, width, height, cx, cy);
            }
        }
        let raw = GreyImage::new(width, height, votes, view.image.placement()).expect("same size");
        let accumulator = gaussian(&raw, 1.0).expect("finite sigma");
        let mut peaks = accumulator.pixels().to_vec();
        // Retain a few separate maxima per radius. Circumference support,
        // rather than relative rank on this film, decides visibility.
        for _ in 0..4 {
            let Some((index, &score)) = peaks.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))
            else {
                break;
            };
            if score < 2.0 {
                break;
            }
            let x = (index % width as usize) as f64;
            let y = (index / width as usize) as f64;
            let candidate = Circle {
                x,
                y,
                radius: f64::from(radius),
                score: f64::from(score),
            };
            if let Some(refined) = refine(view, &gradients, candidate, threshold) {
                circles.push(refined);
            }
            for sy in (y as i64 - 4).max(0)..=(y as i64 + 4).min(i64::from(height) - 1) {
                for sx in (x as i64 - 4).max(0)..=(x as i64 + 4).min(i64::from(width) - 1) {
                    peaks[sy as usize * width as usize + sx as usize] = 0.0;
                }
            }
        }
    }
    circles.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut distinct: Vec<Circle> = Vec::new();
    for circle in circles {
        if distinct.iter().all(|other| {
            libm::hypot(circle.x - other.x, circle.y - other.y) * view.units_per_pixel > 2.0
        }) {
            distinct.push(circle);
        }
    }
    choose(view, film, named_for, &distinct)
}

fn valid_view(view: &HipSearchView<'_>) -> bool {
    let size = (view.image.width(), view.image.height());
    let placement = view.image.placement();
    let determinant = placement.step_across.x() * placement.step_down.y()
        - placement.step_across.y() * placement.step_down.x();
    size.0 > 4
        && size.1 > 4
        && u64::from(size.0) * u64::from(size.1) <= 1_048_576
        && size == (view.valid.width(), view.valid.height())
        && placement == view.valid.placement()
        && [placement.origin, placement.step_across, placement.step_down]
            .iter()
            .all(|p| p.x().is_finite() && p.y().is_finite())
        && determinant.is_finite()
        && determinant != 0.0
        && view.units_per_pixel.is_finite()
        && view.units_per_pixel > 0.0
        && view.midline_column.is_finite()
        && view.radii[0].is_finite()
        && view.radii[1].is_finite()
        && view.radii[0] > 0.0
        && view.radii[1] >= view.radii[0]
        && view.radii[0] / view.units_per_pixel >= 4.0
        && view.radii[1] / view.units_per_pixel <= 128.0
}

fn vote(votes: &mut [f32], width: u32, height: u32, x: f64, y: f64) {
    if x < 0.0 || y < 0.0 || x >= f64::from(width - 1) || y >= f64::from(height - 1) {
        return;
    }
    let left = x.floor() as usize;
    let top = y.floor() as usize;
    let dx = (x - left as f64) as f32;
    let dy = (y - top as f64) as f32;
    for (oy, wy) in [(0, 1.0 - dy), (1, dy)] {
        for (ox, wx) in [(0, 1.0 - dx), (1, dx)] {
            votes[(top + oy) * width as usize + left + ox] += wx * wy;
        }
    }
}

fn refine(
    view: &HipSearchView<'_>,
    gradients: &Gradients,
    guess: Circle,
    threshold: f64,
) -> Option<Circle> {
    let mut points = Vec::new();
    let band = 2.0 / view.units_per_pixel;
    for index in 0..64 {
        let angle = f64::from(index) * core::f64::consts::TAU / 64.0;
        let (nx, ny) = (libm::cos(angle), libm::sin(angle));
        let mut best: Option<(f64, f64, f64)> = None;
        for offset in -4..=4 {
            let radius = guess.radius + f64::from(offset) * band / 4.0;
            let (x, y) = (guess.x + radius * nx, guess.y + radius * ny);
            if bilinear(view.valid, x, y).unwrap_or(0.0) < 1.0 {
                continue;
            }
            let (Some(gx), Some(gy)) = (bilinear(&gradients.x, x, y), bilinear(&gradients.y, x, y))
            else {
                continue;
            };
            let (gx, gy) = (f64::from(gx), f64::from(gy));
            let inward = -gx * nx - gy * ny;
            if inward < threshold || inward < 0.8 * libm::hypot(gx, gy) {
                continue;
            }
            if best.is_none_or(|(_, _, strength)| inward > strength) {
                best = Some((x - guess.x, y - guess.y, inward));
            }
        }
        if let Some((x, y, _)) = best {
            points.push((x, y));
        }
    }
    if points.len() < 48 {
        return None;
    }
    let mut normal = Matrix3::<f64>::zeros();
    let mut rhs = Vector3::<f64>::zeros();
    for &(x, y) in &points {
        let row = Vector3::new(x, y, 1.0);
        normal += row * row.transpose();
        rhs += row * (x * x + y * y);
    }
    let solution = normal.lu().solve(&rhs)?;
    let cx = 0.5 * solution[0];
    let cy = 0.5 * solution[1];
    let radius = libm::sqrt(solution[2] + cx * cx + cy * cy);
    let residual = libm::sqrt(
        points
            .iter()
            .map(|&(x, y)| {
                let d = libm::hypot(x - cx, y - cy) - radius;
                d * d
            })
            .sum::<f64>()
            / points.len() as f64,
    );
    if !radius.is_finite()
        || radius * view.units_per_pixel < view.radii[0] - 1.0
        || radius * view.units_per_pixel > view.radii[1] + 1.0
        || libm::hypot(cx, cy) > band
        || residual * view.units_per_pixel > 1.0
    {
        return None;
    }
    Some(Circle {
        x: guess.x + cx,
        y: guess.y + cy,
        radius,
        score: points.len() as f64 / (1.0 + residual),
    })
}

fn choose(
    view: &HipSearchView<'_>,
    film: FilmKind,
    named_for: ImageRight,
    circles: &[Circle],
) -> Vec<HipCentre> {
    let mark = |circle: Circle, side| {
        let placement = view.image.placement();
        // A physical circle projects to an ellipse with anisotropic stored
        // pixels. Its area-equivalent radius is a single rotation-invariant
        // pixel radius; the centre itself maps without approximation.
        let determinant = placement.step_across.x() * placement.step_down.y()
            - placement.step_across.y() * placement.step_down.x();
        HipCentre {
            side,
            centre: Mark::auto(view.image.film_point(circle.x, circle.y)),
            radius_px: Some(circle.radius * libm::sqrt(determinant.abs())),
        }
    };
    if film == FilmKind::Front {
        let mut result = Vec::new();
        for patient_left in [true, false] {
            let positive = patient_left == (named_for == ImageRight::PatientLeft);
            if let Some(circle) = circles
                .iter()
                .find(|circle| (circle.x > view.midline_column) == positive)
            {
                result.push(mark(
                    *circle,
                    if patient_left {
                        HipSide::Left
                    } else {
                        HipSide::Right
                    },
                ));
            }
        }
        result
    } else if let Some(first) = circles.first() {
        if let Some(second) = circles.get(1) {
            if libm::hypot(first.x - second.x, first.y - second.y) * view.units_per_pixel <= 5.0 {
                let merged = Circle {
                    x: (first.x + second.x) * 0.5,
                    y: (first.y + second.y) * 0.5,
                    radius: (first.radius + second.radius) * 0.5,
                    score: first.score,
                };
                return vec![mark(merged, HipSide::Merged)];
            }
            return vec![
                mark(*first, HipSide::Unpaired),
                mark(*second, HipSide::Unpaired),
            ];
        }
        vec![mark(*first, HipSide::Unpaired)]
    } else {
        Vec::new()
    }
}

/// Prepare the bounded coarse crop from a film pass. The stored image is
/// never changed; the crop's placement carries its scale and quarter turns.
pub fn find_for_cache(cache: &FilmPassCache, chain: &ChainBoundaries) -> Vec<HipCentre> {
    if chain.film.is_bending()
        || chain.film != cache.strip.film
        || chain.film != cache.evidence.film
    {
        return Vec::new();
    }
    let Some(plateau) = chain.sacrum else {
        return Vec::new();
    };
    let Ok(named_for) = facing(&cache.strip) else {
        return Vec::new();
    };
    let Some(centre) = strip_to_film(
        &cache.strip,
        plateau.row,
        (f64::from(cache.strip.cols) - 1.0) * 0.5,
    ) else {
        return Vec::new();
    };
    let mut periods: Vec<f64> = chain
        .bodies
        .windows(2)
        .map(|pair| (pair[1].sup.row - pair[0].sup.row) * cache.strip.row_step)
        .filter(|period| period.is_finite() && *period > 0.0)
        .collect();
    periods.sort_by(f64::total_cmp);
    let (step, radii) = if cache.evidence.scale_assumed {
        let Some(&period) = periods.get(periods.len() / 2) else {
            return Vec::new();
        };
        (period / 25.0, [0.5 * period, 1.2 * period])
    } else {
        (1.0, [16.0, 30.0])
    };
    let Some((image, valid, midline_column)) = crop(cache, centre, step) else {
        return Vec::new();
    };
    find(
        &HipSearchView {
            image: &image,
            valid: &valid,
            units_per_pixel: step,
            midline_column,
            radii,
        },
        chain.film,
        named_for,
    )
}

fn crop(
    cache: &FilmPassCache,
    centre: Px,
    step: f64,
) -> Option<(GreyImage<f32>, GreyImage<u8>, f64)> {
    let source = &cache.evidence.image;
    let mask = &cache.evidence.valid;
    let mm = cache.evidence.mm_per_px;
    if source.width() == 0
        || source.height() == 0
        || !mm.is_finite()
        || mm <= 0.0
        || !step.is_finite()
        || step <= 0.0
    {
        return None;
    }
    let (right, down) = match cache.centre_line.quarter_turns.get() {
        0 => (Px::new(1.0, 0.0), Px::new(0.0, 1.0)),
        1 => (Px::new(0.0, -1.0), Px::new(1.0, 0.0)),
        2 => (Px::new(-1.0, 0.0), Px::new(0.0, -1.0)),
        _ => (Px::new(0.0, 1.0), Px::new(-1.0, 0.0)),
    };
    let (u, v) = source.image_point(centre)?;
    let cell_step = |axis: Px| -> Option<Px> {
        let (x, y) = source.image_point(Px::new(centre.x() + axis.x(), centre.y() + axis.y()))?;
        let per_unit = mm * libm::hypot(x - u, y - v);
        (per_unit.is_finite() && per_unit > 0.0).then_some(Px::new(
            axis.x() * step / per_unit,
            axis.y() * step / per_unit,
        ))
    };
    let basis = Placement {
        origin: centre,
        step_across: cell_step(right)?,
        step_down: cell_step(down)?,
    };
    let corners = [
        (0.0, 0.0),
        (f64::from(source.width() - 1), 0.0),
        (0.0, f64::from(source.height() - 1)),
        (
            f64::from(source.width() - 1),
            f64::from(source.height() - 1),
        ),
    ];
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in corners {
        let (x, y) = basis.image_point(source.film_point(x, y))?;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let left = min_x.ceil();
    let width = (max_x.floor() - left + 1.0).max(0.0) as u32;
    let height = (max_y.floor() + 1.0).max(0.0) as u32;
    // One radius accumulator stays below 4 MiB. A pathological scale is
    // refused instead of allocating a full-resolution Hough volume.
    if width < 5 || height < 5 || u64::from(width) * u64::from(height) > 1_048_576 {
        return None;
    }
    let placement = Placement {
        origin: basis.film_point(left, 0.0),
        ..basis
    };
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    let mut valid = Vec::with_capacity(pixels.capacity());
    for y in 0..height {
        for x in 0..width {
            let point = placement.film_point(f64::from(x), f64::from(y));
            let (sx, sy) = source.image_point(point)?;
            let (mx, my) = mask.image_point(point)?;
            let value = bilinear(source, sx, sy);
            let usable = value.is_some_and(f32::is_finite)
                && bilinear(mask, mx, my).is_some_and(|v| v >= 1.0);
            pixels.push(value.filter(|v| v.is_finite()).unwrap_or(0.0));
            valid.push(u8::from(usable));
        }
    }
    Some((
        GreyImage::new(width, height, pixels, placement)?,
        GreyImage::new(width, height, valid, placement)?,
        -left,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{
        Barcode, CentreLine, CentreLineEnds, EndplateOnStrip, FilmEvidence, Polarity,
        PolarityCheck, Strip, StripEvidence, StripRow,
    };
    use shapes::film_geometry::QuarterTurns;
    use shapes::naming::Template;

    fn disks(centres: &[(f64, f64, f64)], placement: Placement) -> GreyImage<f32> {
        let (width, height) = (220, 130);
        let mut values = Vec::new();
        for y in 0..height {
            for x in 0..width {
                // Independent analytic disks with a one-cell linear edge.
                // The expected centre is supplied here, not fitted by the
                // detector or recovered from its own voting geometry.
                let value: f64 = centres
                    .iter()
                    .map(|&(cx, cy, r)| {
                        (r + 0.5 - libm::hypot(f64::from(x) - cx, f64::from(y) - cy))
                            .clamp(0.0, 1.0)
                    })
                    .sum();
                values.push((0.15 + 0.7 * value.min(1.0)) as f32);
            }
        }
        GreyImage::new(width, height, values, placement).expect("same size")
    }

    fn search<'a>(image: &'a GreyImage<f32>, valid: &'a GreyImage<u8>) -> HipSearchView<'a> {
        HipSearchView {
            image,
            valid,
            units_per_pixel: 1.0,
            midline_column: 110.0,
            radii: [16.0, 30.0],
        }
    }

    #[test]
    fn circles_are_found_and_named_after_mapping_to_original_pixels() {
        let placement = Placement {
            origin: Px::new(400.0, 500.0),
            step_across: Px::new(0.0, -2.0),
            step_down: Px::new(2.0, 0.0),
        };
        let image = disks(&[(55.25, 64.5, 18.0), (165.5, 72.25, 28.0)], placement);
        let valid = image.map(|_| 1u8);
        let view = search(&image, &valid);
        for facing in [ImageRight::PatientLeft, ImageRight::PatientRight] {
            let found = find(&view, FilmKind::Front, facing);
            assert_eq!(found.len(), 2);
            for hip in found {
                let positive = (hip.side == HipSide::Left) == (facing == ImageRight::PatientLeft);
                let (cx, cy, r) = if positive {
                    (165.5, 72.25, 28.0)
                } else {
                    (55.25, 64.5, 18.0)
                };
                // The 1 mm Hough grid and edge interpolation allow 2 mm,
                // which is 4 original pixels on this half-size crop.
                let want = Px::new(400.0 + 2.0 * cy, 500.0 - 2.0 * cx);
                assert!(
                    libm::hypot(hip.centre.at.x() - want.x(), hip.centre.at.y() - want.y()) < 4.0,
                    "{:?}, expected {want:?}",
                    hip.centre.at
                );
                assert!((hip.radius_px.expect("fitted") - 2.0 * r).abs() < 4.0);
                assert!(!hip.centre.inferred);
            }
        }
    }

    #[test]
    fn no_circle_is_invented_for_blank_masked_or_bending_views() {
        let blank = disks(&[], Placement::FILM);
        let valid = blank.map(|_| 1u8);
        assert!(
            find(
                &search(&blank, &valid),
                FilmKind::Front,
                ImageRight::PatientLeft
            )
            .is_empty()
        );
        let image = disks(&[(55.25, 64.5, 22.0)], Placement::FILM);
        let masked = image.map(|_| 0u8);
        assert!(
            find(
                &search(&image, &masked),
                FilmKind::Front,
                ImageRight::PatientLeft
            )
            .is_empty()
        );
        assert!(
            find(
                &search(&image, &valid),
                FilmKind::BendLeft,
                ImageRight::PatientLeft
            )
            .is_empty()
        );
        // A single visible front head does not justify inventing its mate.
        let found = find(
            &search(&image, &valid),
            FilmKind::Front,
            ImageRight::PatientLeft,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].side, HipSide::Right);
        let side = find(
            &search(&image, &valid),
            FilmKind::Side,
            ImageRight::Anterior,
        );
        assert_eq!(side.len(), 1, "one rim must not become two fitted heads");
        assert_eq!(side[0].side, HipSide::Unpaired);
    }

    #[test]
    fn side_heads_are_unpaired_and_close_heads_use_their_midpoint() {
        let image = disks(&[], Placement::FILM);
        let valid = image.map(|_| 1u8);
        let view = search(&image, &valid);
        let a = Circle {
            x: 50.0,
            y: 60.0,
            radius: 20.0,
            score: 1.0,
        };
        let b = Circle { x: 54.0, ..a };
        let merged = choose(&view, FilmKind::Side, ImageRight::Anterior, &[a, b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].side, HipSide::Merged);
        // The midpoint of (50,60) and (54,60) is exactly (52,60).
        assert_eq!(merged[0].centre.at, Px::new(52.0, 60.0));
        let separate = choose(
            &view,
            FilmKind::Side,
            ImageRight::Posterior,
            &[a, Circle { x: 70.0, ..b }],
        );
        assert_eq!(separate.len(), 2);
        assert!(separate.iter().all(|head| head.side == HipSide::Unpaired));
    }

    #[test]
    fn the_cache_crop_preserves_turned_anisotropic_film_coordinates() {
        let placement = Placement {
            origin: Px::new(400.0, 700.0),
            step_across: Px::new(0.0, -3.0),
            step_down: Px::new(2.0, 0.0),
        };
        let image = disks(&[(55.25, 64.5, 22.0), (165.5, 72.25, 25.0)], placement);
        let valid = image.map(|_| 1u8);
        let zero = || image.map(|_| 0.0f32);
        let evidence = FilmEvidence {
            film: FilmKind::Front,
            mm_per_px: 1.0,
            scale_assumed: false,
            valid: valid.clone(),
            image: image.clone(),
            phase_edge: zero(),
            phase_direction: zero(),
            phase_kind: zero(),
            gradient_edge: zero(),
            gradient_direction: zero(),
            ridge: zero(),
            ridge_orientation: zero(),
            valley: zero(),
            valley_orientation: zero(),
            valley_scale: image.map(|_| 0u8),
            ridge_symmetry: zero(),
            valley_symmetry: zero(),
            rings: None,
            hessian_scales_mm: Vec::new(),
            polarity: PolarityCheck {
                found: Polarity::BoneBright,
                confidence: 1.0,
                needs_confirm: false,
            },
            markers: Vec::new(),
            seam_rows: Vec::new(),
            valid_fraction: 1.0,
            coverage: 1.0,
            warnings: Vec::new(),
        };
        let strip = Strip {
            film: FilmKind::Front,
            rows: 2,
            cols: 3,
            row_step: 1.0,
            col_step: 1.0,
            scale_assumed: false,
            values: vec![0.0; 6],
            valid: vec![1; 6],
            layout: (0..2)
                .map(|row| StripRow {
                    centre: Px::new(420.0 + 2.0 * f64::from(row), 370.0),
                    across: placement.step_across,
                    along: placement.step_down,
                    curvature: 0.0,
                    half_width: 16.0,
                    z: -f64::from(row),
                })
                .collect(),
            image_right: Some(ImageRight::PatientLeft),
            polarity_hint: Some(Polarity::BoneBright),
            evidence: StripEvidence::default(),
        };
        let cache = FilmPassCache {
            evidence,
            strip,
            centre_line: CentreLine {
                film: FilmKind::Front,
                quarter_turns: QuarterTurns::new(1).expect("quarter turn"),
                scale_assumed: false,
                sample_step: 1.0,
                samples: Vec::new(),
                ends: CentreLineEnds {
                    neck_base: None,
                    iliac_crest: None,
                    top_cropped: false,
                    bottom_cropped: false,
                    size_factor: 1.0,
                },
                warnings: Vec::new(),
            },
            barcode: Barcode {
                film: FilmKind::Front,
                values: Vec::new(),
                high_pass: Vec::new(),
                valid: Vec::new(),
            },
        };
        let mut chain = ChainBoundaries {
            film: FilmKind::Front,
            template: Template::Standard,
            polarity: Polarity::BoneBright,
            bodies: Vec::new(),
            sacrum: Some(EndplateOnStrip {
                row: 0.0,
                tilt: 0.0,
            }),
            energy: 0.0,
            warnings: Vec::new(),
        };
        let hips = find_for_cache(&cache, &chain);
        assert_eq!(hips.len(), 2);
        for hip in hips {
            let (cx, cy) = if hip.side == HipSide::Left {
                (165.5, 72.25)
            } else {
                (55.25, 64.5)
            };
            // The independent affine map is (400 + 2y, 700 − 3x).
            // A 2 mm centre tolerance is at most 6 stored film pixels.
            let expected = Px::new(400.0 + 2.0 * cy, 700.0 - 3.0 * cx);
            assert!(
                libm::hypot(
                    hip.centre.at.x() - expected.x(),
                    hip.centre.at.y() - expected.y()
                ) < 6.0
            );
        }
        chain.sacrum = None;
        assert!(find_for_cache(&cache, &chain).is_empty());
    }
}
