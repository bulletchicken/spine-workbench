//! The straightened strip, sampled along the centre line and its normals, and
//! the exact maps between strip and film in both directions.

use super::maps::{add, scale, unfolded};
use crate::handoffs::{
    CentreLine, CentreSample, FilmEvidence, Polarity, Strip, StripEvidence, StripRow,
};
use darkroom::grey_image::GreyImage;
use darkroom::normalise::{Window, percentile_window};
use darkroom::sampling::bilinear;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film_geometry::FilmGeometry;
use shapes::pixels::Px;

const STEP: f64 = 0.5;

/// Unroll the original pixels at half-unit spacing. The evidence's mask
/// excludes shutters, markers, seams and borders from every sampling tap.
pub fn build(
    image: &GreyImage<u16>,
    evidence: &FilmEvidence,
    line: &CentreLine,
    geometry: &FilmGeometry,
) -> Result<Strip, EngineError> {
    validate(image, evidence, line, geometry)?;
    let spacing = film_spacing(evidence, geometry)?;
    let window = intensity_window(image, evidence)?;
    let layout = row_layout(line, spacing);
    if layout
        .iter()
        .any(|row| !row.along.x().is_finite() || !row.along.y().is_finite())
    {
        return Err(EngineError::new(
            ErrorCode::PassNoSpineFound,
            "centre line doubles back within a sample",
        ));
    }
    let mut widths: Vec<_> = line.samples.iter().map(|sample| sample.width).collect();
    let rank = ((widths.len() - 1) as f64 * 0.95).round() as usize;
    let (_, width95, _) = widths.select_nth_unstable_by(rank, f64::total_cmp);
    // Keep the measured field of view at every row. A tight bend affects
    // only its concave columns; using its curvature to narrow the entire
    // strip would discard well-separated walls elsewhere on the film.
    // The local Jacobian guard below excludes the folded samples, and the
    // exact maps apply the same guard to fractional positions.
    let half_width = (0.9 * *width95).clamp(25.0, 50.0);
    let cols = 2 * (half_width / STEP).round() as u32 + 1;
    let rows = layout.len() as u32;
    let cells = (rows as usize).checked_mul(cols as usize).ok_or_else(|| {
        EngineError::new(ErrorCode::EngineOutOfMemory, "strip dimensions overflow")
    })?;
    if cells > 4_000_000 {
        return Err(EngineError::new(
            ErrorCode::PassFilmTooSmall,
            "strip exceeds the working size limit",
        ));
    }
    let mut strip = Strip {
        film: line.film,
        rows,
        cols,
        row_step: STEP,
        col_step: STEP,
        scale_assumed: line.scale_assumed,
        values: vec![f32::NAN; cells],
        valid: vec![0; cells],
        layout,
        image_right: geometry.image_right,
        // This frozen field records the evidence's judgement of the source
        // film. Values below are already bone-bright: a consumer must not
        // invert them a second time when this hint is BoneDark.
        polarity_hint: (!evidence.polarity.needs_confirm).then_some(evidence.polarity.found),
        evidence: StripEvidence {
            edge_along: Some(vec![0.0; cells]),
            ring: evidence.rings.as_ref().map(|_| vec![0.0; cells]),
            ridge: Some(vec![0.0; cells]),
            ridge_angle: Some(vec![0.0; cells]),
        },
    };
    let taps = (STEP / spacing.0.min(spacing.1)).ceil().max(1.0) as u32;
    let middle = f64::from(cols - 1) / 2.0;
    for row in 0..rows as usize {
        let layout = strip.layout[row];
        for col in 0..cols as usize {
            let offset = col as f64 - middle;
            if !unfolded(layout.curvature, offset * STEP) {
                continue;
            }
            let at = add(layout.centre, scale(layout.across, offset));
            let Some(raw) = sample_box(image, &evidence.valid, at, layout, taps) else {
                continue;
            };
            let index = row * cols as usize + col;
            let value = match evidence.polarity.found {
                Polarity::BoneBright => window.scale(raw, 1.5),
                Polarity::BoneDark => Window {
                    low: -window.high,
                    high: -window.low,
                }
                .scale(-raw, 1.5),
            };
            strip.values[index] = value as f32;
            strip.valid[index] = 1;
            resample_evidence(&mut strip.evidence, index, evidence, at, layout.along);
        }
    }
    Ok(strip)
}

fn sample_box(
    image: &GreyImage<u16>,
    valid: &GreyImage<u8>,
    at: Px,
    layout: StripRow,
    taps: u32,
) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0;
    for down in 0..taps {
        let along_offset = (f64::from(down) + 0.5) / f64::from(taps) - 0.5;
        for across in 0..taps {
            let across_offset = (f64::from(across) + 0.5) / f64::from(taps) - 0.5;
            let tap = add(
                add(at, scale(layout.along, along_offset)),
                scale(layout.across, across_offset),
            );
            let Some((x, y)) = valid.image_point(tap) else {
                continue;
            };
            if bilinear(valid, x, y).is_none_or(|v| v < 1.0 - 1e-6) {
                continue;
            }
            let Some((u, v)) = image.image_point(tap) else {
                continue;
            };
            if let Some(value) = bilinear(image, u, v) {
                sum += f64::from(value);
                count += 1;
            }
        }
    }
    (count != 0).then_some(sum / f64::from(count))
}

fn validate(
    image: &GreyImage<u16>,
    evidence: &FilmEvidence,
    line: &CentreLine,
    geometry: &FilmGeometry,
) -> Result<(), EngineError> {
    let finite_point = |point: Px| point.x().is_finite() && point.y().is_finite();
    let shape_matches = |map: &GreyImage<f32>| {
        map.width() == evidence.valid.width()
            && map.height() == evidence.valid.height()
            && map.placement() == evidence.valid.placement()
    };
    if line.film != evidence.film
        || line.scale_assumed != evidence.scale_assumed
        || line.scale_assumed != geometry.mm_per_px.is_none()
        || line.quarter_turns != geometry.quarter_turns
        || image.width() == 0
        || image.height() == 0
        || image.image_point(Px::new(0.0, 0.0)).is_none()
        || evidence.valid.width() == 0
        || evidence.valid.height() == 0
        || evidence.valid.image_point(Px::new(0.0, 0.0)).is_none()
        || !shape_matches(&evidence.gradient_edge)
        || !shape_matches(&evidence.gradient_direction)
        || !shape_matches(&evidence.ridge)
        || !shape_matches(&evidence.ridge_orientation)
        || evidence
            .rings
            .as_ref()
            .is_some_and(|rings| !shape_matches(&rings.response))
        || !evidence.mm_per_px.is_finite()
        || evidence.mm_per_px <= 0.0
    {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "film stages have incompatible geometry",
        ));
    }
    if line.samples.len() < 2
        || line.samples[0].arc.abs() > 1e-6
        || line
            .samples
            .last()
            .is_none_or(|s| s.arc < STEP || s.arc > 10_000.0)
        || !line.sample_step.is_finite()
        || line.sample_step <= 0.0
        || line.samples.windows(2).any(|s| s[0].arc >= s[1].arc)
        || line.samples.iter().any(|s| {
            !s.arc.is_finite()
                || !finite_point(s.at)
                || !finite_point(s.along)
                || !s.curvature.is_finite()
                || !s.width.is_finite()
                || s.width < 0.0
                || s.along.x() * s.along.x() + s.along.y() * s.along.y() == 0.0
        })
    {
        return Err(EngineError::new(
            ErrorCode::PassNoSpineFound,
            "centre line cannot be unrolled",
        ));
    }
    Ok(())
}

fn film_spacing(
    evidence: &FilmEvidence,
    geometry: &FilmGeometry,
) -> Result<(f64, f64), EngineError> {
    let spacing = if let Some(horizontal) = geometry.mm_per_px {
        (
            horizontal,
            geometry.mm_per_px_vertical.unwrap_or(horizontal),
        )
    } else {
        // The working grid is isotropic in the film's length unit, so its
        // inverse placement recovers even an assumed native pixel spacing.
        let placement = evidence.image.placement();
        let origin = placement.image_point(Px::new(0.0, 0.0)).ok_or_else(|| {
            EngineError::new(ErrorCode::EngineOutOfSync, "evidence placement is singular")
        })?;
        let x = placement.image_point(Px::new(1.0, 0.0)).ok_or_else(|| {
            EngineError::new(ErrorCode::EngineOutOfSync, "evidence placement is singular")
        })?;
        let y = placement.image_point(Px::new(0.0, 1.0)).ok_or_else(|| {
            EngineError::new(ErrorCode::EngineOutOfSync, "evidence placement is singular")
        })?;
        (
            libm::hypot(x.0 - origin.0, x.1 - origin.1) * evidence.mm_per_px,
            libm::hypot(y.0 - origin.0, y.1 - origin.1) * evidence.mm_per_px,
        )
    };
    if !spacing.0.is_finite()
        || !spacing.1.is_finite()
        || !(0.05..=2.0).contains(&spacing.0)
        || !(0.05..=2.0).contains(&spacing.1)
    {
        return Err(EngineError::new(
            ErrorCode::PassFilmTooSmall,
            "invalid film spacing for unrolling",
        ));
    }
    Ok(spacing)
}

fn intensity_window(
    image: &GreyImage<u16>,
    evidence: &FilmEvidence,
) -> Result<Window, EngineError> {
    let low = *image
        .pixels()
        .iter()
        .min()
        .ok_or_else(|| EngineError::new(ErrorCode::PassFlatFilm, "empty film"))?;
    let high = *image.pixels().iter().max().unwrap_or(&low);
    let mut values = Vec::new();
    for y in 0..evidence.valid.height() {
        for x in 0..evidence.valid.width() {
            if evidence.valid.get(x, y) != Some(&1) {
                continue;
            }
            let point = evidence.valid.film_point(f64::from(x), f64::from(y));
            let Some((u, v)) = image.image_point(point) else {
                continue;
            };
            if let Some(value) = bilinear(image, u, v)
                && value > f32::from(low)
                && value < f32::from(high)
            {
                values.push(match evidence.polarity.found {
                    Polarity::BoneBright => value,
                    Polarity::BoneDark => -value,
                });
            }
        }
    }
    let mut window = percentile_window(&values, 0.01, 0.95).ok_or_else(|| {
        EngineError::new(
            ErrorCode::PassFlatFilm,
            "film has no usable intensity range",
        )
    })?;
    if window.high - window.low <= 1e-6 {
        return Err(EngineError::new(
            ErrorCode::PassFlatFilm,
            "film has no usable intensity range",
        ));
    }
    if evidence.polarity.found == Polarity::BoneDark {
        window = Window {
            low: -window.high,
            high: -window.low,
        };
    }
    Ok(window)
}

fn row_layout(line: &CentreLine, spacing: (f64, f64)) -> Vec<StripRow> {
    let last = line.samples.last().expect("validated centre line");
    let rows = (last.arc / STEP).floor() as usize + 1;
    let mut segment = 0;
    (0..rows)
        .map(|row| {
            let arc = row as f64 * STEP;
            while segment + 2 < line.samples.len() && line.samples[segment + 1].arc < arc {
                segment += 1;
            }
            let a = line.samples[segment];
            let b = line.samples[segment + 1];
            let (centre, tangent, width, curvature) = interpolate(a, b, arc);
            let tx = tangent.x() * spacing.0;
            let ty = tangent.y() * spacing.1;
            let length = libm::hypot(tx, ty);
            StripRow {
                centre,
                across: Px::new(
                    STEP * ty / (length * spacing.0),
                    -STEP * tx / (length * spacing.1),
                ),
                along: Px::new(
                    STEP * tx / (length * spacing.0),
                    STEP * ty / (length * spacing.1),
                ),
                curvature,
                half_width: width / 2.0,
                z: match line.quarter_turns.get() {
                    0 => -(centre.y() - line.samples[0].at.y()) * spacing.1,
                    1 => -(centre.x() - line.samples[0].at.x()) * spacing.0,
                    2 => (centre.y() - line.samples[0].at.y()) * spacing.1,
                    _ => (centre.x() - line.samples[0].at.x()) * spacing.0,
                },
            }
        })
        .collect()
}

fn interpolate(a: CentreSample, b: CentreSample, arc: f64) -> (Px, Px, f64, f64) {
    let gap = b.arc - a.arc;
    let t = ((arc - a.arc) / gap).clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let centre = |p, q, u, v| {
        (2.0 * t3 - 3.0 * t2 + 1.0) * p
            + (t3 - 2.0 * t2 + t) * gap * u
            + (-2.0 * t3 + 3.0 * t2) * q
            + (t3 - t2) * gap * v
    };
    let tangent = |p, q, u, v| {
        (6.0 * t2 - 6.0 * t) * p / gap
            + (3.0 * t2 - 4.0 * t + 1.0) * u
            + (-6.0 * t2 + 6.0 * t) * q / gap
            + (3.0 * t2 - 2.0 * t) * v
    };
    (
        Px::new(
            centre(a.at.x(), b.at.x(), a.along.x(), b.along.x()),
            centre(a.at.y(), b.at.y(), a.along.y(), b.along.y()),
        ),
        Px::new(
            tangent(a.at.x(), b.at.x(), a.along.x(), b.along.x()),
            tangent(a.at.y(), b.at.y(), a.along.y(), b.along.y()),
        ),
        a.width + t * (b.width - a.width),
        a.curvature + t * (b.curvature - a.curvature),
    )
}

fn resample_evidence(
    output: &mut StripEvidence,
    index: usize,
    evidence: &FilmEvidence,
    at: Px,
    along: Px,
) {
    let Some((x, y)) = evidence.valid.image_point(at) else {
        return;
    };
    if bilinear(&evidence.valid, x, y).is_none_or(|v| v < 1.0 - 1e-6) {
        return;
    }
    if let Some(ring) = &evidence.rings
        && let Some(output) = &mut output.ring
    {
        output[index] = bilinear(&ring.response, x, y).unwrap_or(0.0);
    }
    if let Some(output) = &mut output.ridge {
        output[index] = bilinear(&evidence.ridge, x, y).unwrap_or(0.0);
    }
    // Average vectors, not angles: +/-pi directions and +/-pi/2 axes are
    // adjacent, not opposite sides of an angle interval.
    let angle_sample = |strength: &GreyImage<f32>, angles: &GreyImage<f32>, multiplier: f64| {
        let left = x.floor() as u32;
        let top = y.floor() as u32;
        let dx = x - f64::from(left);
        let dy = y - f64::from(top);
        let mut vector = (0.0, 0.0);
        for (column, row, weight) in [
            (left, top, (1.0 - dx) * (1.0 - dy)),
            ((left + 1).min(strength.width() - 1), top, dx * (1.0 - dy)),
            (left, (top + 1).min(strength.height() - 1), (1.0 - dx) * dy),
            (
                (left + 1).min(strength.width() - 1),
                (top + 1).min(strength.height() - 1),
                dx * dy,
            ),
        ] {
            let value = f64::from(*strength.get(column, row).unwrap_or(&0.0)) * weight;
            let angle = f64::from(*angles.get(column, row).unwrap_or(&0.0)) * multiplier;
            vector.0 += value * libm::cos(angle);
            vector.1 += value * libm::sin(angle);
        }
        vector
    };
    if let Some(output) = &mut output.edge_along {
        let vector = angle_sample(&evidence.gradient_edge, &evidence.gradient_direction, 1.0);
        let norm = libm::hypot(along.x(), along.y());
        output[index] = ((vector.0 * along.x() + vector.1 * along.y()) / norm) as f32;
    }
    if let Some(output) = &mut output.ridge_angle {
        let vector = angle_sample(&evidence.ridge, &evidence.ridge_orientation, 2.0);
        if vector.0 != 0.0 || vector.1 != 0.0 {
            let normal = 0.5 * libm::atan2(vector.1, vector.0);
            output[index] = ((normal + std::f64::consts::PI).rem_euclid(std::f64::consts::PI)
                - std::f64::consts::FRAC_PI_2) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{CentreLineEnds, PolarityCheck};
    use crate::strip::{checkpoints, maps, warnings};
    use darkroom::grey_image::Placement;
    use shapes::auto_pass::PassWarningCode;
    use shapes::film::FilmKind;
    use shapes::film_geometry::QuarterTurns;
    use shapes::watch_frames::WatchFrame;

    fn ramp() -> GreyImage<u16> {
        GreyImage::new(
            600,
            600,
            (0..600)
                .flat_map(|y| (0..600).map(move |x| (100 + 3 * x + 7 * y) as u16))
                .collect(),
            Placement::FILM,
        )
        .unwrap()
    }

    fn evidence(image: &GreyImage<u16>) -> FilmEvidence {
        let blank = || image.map(|_| 0.0);
        FilmEvidence {
            film: FilmKind::Front,
            mm_per_px: 0.5,
            scale_assumed: false,
            valid: image.map(|_| 1),
            image: image.map(|v| f32::from(*v) / 6100.0),
            phase_edge: blank(),
            phase_direction: blank(),
            phase_kind: blank(),
            gradient_edge: blank(),
            gradient_direction: blank(),
            ridge: blank(),
            ridge_orientation: blank(),
            valley: blank(),
            valley_orientation: blank(),
            valley_scale: image.map(|_| 0),
            ridge_symmetry: blank(),
            valley_symmetry: blank(),
            rings: None,
            hessian_scales_mm: vec![],
            polarity: PolarityCheck {
                found: Polarity::BoneBright,
                confidence: 1.0,
                needs_confirm: false,
            },
            markers: vec![],
            seam_rows: vec![],
            valid_fraction: 1.0,
            coverage: 1.0,
            warnings: vec![],
        }
    }

    fn line(turns: u8, spacing: (f64, f64), radius: Option<f64>) -> CentreLine {
        let to_film = |x: f64, y: f64| match turns {
            0 => Px::new(x / spacing.0, y / spacing.1),
            1 => Px::new(y / spacing.0, -x / spacing.1),
            2 => Px::new(-x / spacing.0, -y / spacing.1),
            _ => Px::new(-y / spacing.0, x / spacing.1),
        };
        let samples = (0..=120)
            .map(|i| {
                let arc = f64::from(i) * 0.25;
                let (x, y, tx, ty, curvature) = if let Some(r) = radius {
                    (
                        r * (1.0 - libm::cos(arc / r)),
                        r * libm::sin(arc / r),
                        libm::sin(arc / r),
                        libm::cos(arc / r),
                        1.0 / r,
                    )
                } else {
                    (0.0, arc, 0.0, 1.0, 0.0)
                };
                CentreSample {
                    arc,
                    at: add(Px::new(300.0, 300.0), to_film(x, y)),
                    along: to_film(tx, ty),
                    curvature,
                    width: 56.0,
                    width_measured: true,
                    evidence: 1.0,
                }
            })
            .collect();
        CentreLine {
            film: FilmKind::Front,
            quarter_turns: QuarterTurns::new(turns).unwrap(),
            scale_assumed: false,
            sample_step: 0.25,
            samples,
            ends: CentreLineEnds {
                neck_base: None,
                iliac_crest: None,
                top_cropped: true,
                bottom_cropped: true,
                size_factor: 1.0,
            },
            warnings: vec![],
        }
    }

    #[test]
    fn box_sampling_of_a_linear_film_is_exact_in_every_turn_with_unequal_pixels() {
        let image = ramp();
        let valid = image.map(|_| 1u8);
        let spacing = (0.15, 0.18);
        let directions = [
            Px::new(0.5 / 0.15, 0.0),
            Px::new(0.0, -0.5 / 0.18),
            Px::new(-0.5 / 0.15, 0.0),
            Px::new(0.0, 0.5 / 0.18),
        ];
        for turns in 0..4 {
            let line = line(turns, spacing, None);
            let layout = row_layout(&line, spacing);
            // Normalisation introduces a handful of f64 rounding steps.
            assert!((layout[0].across.x() - directions[turns as usize].x()).abs() < 1e-12);
            assert!((layout[0].across.y() - directions[turns as usize].y()).abs() < 1e-12);
            for row in [0, 5, 47, 60] {
                for col in [-20.5, 0.0, 27.25] {
                    let at = add(layout[row].centre, scale(layout[row].across, col));
                    let actual = sample_box(&image, &valid, at, layout[row], 4).unwrap();
                    let expected = 100.0 + 3.0 * at.x() + 7.0 * at.y();
                    // Taps average a linear ramp exactly, apart from the f32
                    // bilinear output: <10^-5 after a 6000-code stretch.
                    assert!((actual - expected).abs() / 6000.0 < 1e-5);
                }
            }
            assert!((layout[60].z + 30.0).abs() < 1e-12);
        }
    }

    #[test]
    fn fold_guard_masks_only_the_local_concave_columns() {
        let image = ramp();
        let evidence = evidence(&image);
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        for radius in [50.0, 25.0] {
            let line = line(0, (0.5, 0.5), Some(radius));
            let strip = build(&image, &evidence, &line, &geometry).unwrap();
            assert_eq!(strip.cols, 201); // Measured 50 mm field of view.
            let last_valid = 100 + (0.8 * radius / STEP).round() as usize;
            for row in 0..strip.rows as usize {
                let first = row * strip.cols as usize;
                assert!(
                    strip.valid[first..=first + last_valid]
                        .iter()
                        .all(|v| *v == 1)
                );
                assert!(
                    strip.valid[first + last_valid + 1..first + 201]
                        .iter()
                        .all(|v| *v == 0)
                );
                assert!(
                    strip.values[first + last_valid + 1..first + 201]
                        .iter()
                        .all(|v| v.is_nan())
                );
            }
            assert_eq!(warnings(&strip)[0].code, PassWarningCode::StripFolded);
            assert_eq!(maps::strip_to_film(&strip, 20.0, 200.0), None);
            // The full convex field stays usable at either curvature.
            assert!(maps::strip_to_film(&strip, 20.0, 0.0).is_some());
            assert!((strip.layout[60].z + radius * libm::sin(30.0 / radius)).abs() < 1e-12);
        }
    }

    #[test]
    fn a_short_bend_does_not_crop_the_straight_rows() {
        let image = ramp();
        let evidence = evidence(&image);
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let mut line = line(0, (0.5, 0.5), Some(25.0));
        let join = line.samples[40]; // A 10 mm circular arc, then its tangent.
        for sample in &mut line.samples[41..] {
            sample.at = add(join.at, scale(join.along, sample.arc - join.arc));
            sample.along = join.along;
            sample.curvature = 0.0;
        }
        let strip = build(&image, &evidence, &line, &geometry).unwrap();
        assert_eq!(strip.cols, 201);
        // The analytic straight segment has Jacobian one across all 100 mm.
        let row = 40;
        let first = row * strip.cols as usize;
        assert!(strip.valid[first..first + 201].iter().all(|v| *v == 1));
        assert!(maps::strip_to_film(&strip, row as f64, 200.0).is_some());
    }

    #[test]
    fn masked_pixels_stay_invalid_and_checkpoints_cover_each_row_once() {
        let image = ramp();
        let mut evidence = evidence(&image);
        evidence.valid = image.map(|v| u8::from(*v < 3400 || *v > 3600));
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let mut line = line(0, (0.5, 0.5), None);
        // More than four 64-row chunks, including a short final chunk.
        for i in 121..=520 {
            line.samples.push(CentreSample {
                arc: f64::from(i) * 0.25,
                at: Px::new(300.0, 300.0 + f64::from(i) * 0.5),
                ..line.samples[0]
            });
        }
        let (strip, barcode) = super::super::build(&image, &evidence, &line, &geometry).unwrap();
        assert!(strip.valid.contains(&0));
        for (&value, &valid) in strip.values.iter().zip(&strip.valid) {
            assert_eq!(value.is_finite(), valid != 0);
        }
        let frames = checkpoints::frames(&strip, &barcode);
        assert!(matches!(frames[0], WatchFrame::StageStarted { .. }));
        assert!(matches!(frames[1], WatchFrame::StripShape { .. }));
        let mut next_row = 0;
        for frame in frames {
            if let WatchFrame::StripRows {
                first_row,
                rows,
                columns,
                image,
                valid,
                ..
            } = frame
            {
                assert_eq!(first_row, next_row);
                for cell in 0..(rows * columns) as usize {
                    assert_eq!(
                        (valid[cell / 8] >> (cell % 8)) & 1,
                        strip.valid[(first_row * columns) as usize + cell]
                    );
                }
                assert_eq!(image.len(), (rows * columns) as usize);
                next_row += rows;
            }
        }
        assert_eq!(next_row, strip.rows);
    }

    #[test]
    fn an_inverted_film_produces_the_same_bone_bright_strip() {
        let image = ramp();
        let negative = image.map(|value| u16::MAX - *value);
        let normal_evidence = evidence(&image);
        let mut negative_evidence = evidence(&negative);
        negative_evidence.polarity.found = Polarity::BoneDark;
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let line = line(0, (0.5, 0.5), None);
        let normal = build(&image, &normal_evidence, &line, &geometry).unwrap();
        let negative = build(&negative, &negative_evidence, &line, &geometry).unwrap();
        assert_eq!(normal.valid, negative.valid);
        assert_eq!(negative.polarity_hint, Some(Polarity::BoneDark));
        // f32 bilinear interpolation of 16-bit codes precedes the stretch;
        // quantisation at that step bounds disagreement well below 10^-5.
        for (&a, &b) in normal.values.iter().zip(&negative.values) {
            assert!((a - b).abs() < 1e-5, "{a} versus {b}");
        }
    }

    #[test]
    fn evidence_directions_cross_the_angle_cut_without_cancelling() {
        let image = GreyImage::new(2, 2, vec![1u16, 2, 3, 4], Placement::FILM).unwrap();
        let mut evidence = evidence(&image);
        evidence.gradient_edge = image.map(|_| 1.0);
        evidence.ridge = image.map(|_| 1.0);
        evidence.gradient_direction = GreyImage::new(
            2,
            2,
            vec![
                std::f32::consts::PI,
                -std::f32::consts::PI,
                std::f32::consts::PI,
                -std::f32::consts::PI,
            ],
            Placement::FILM,
        )
        .unwrap();
        evidence.ridge_orientation = evidence.gradient_direction.map(|angle| angle * 0.5);
        let mut output = StripEvidence {
            edge_along: Some(vec![0.0]),
            ring: None,
            ridge: Some(vec![0.0]),
            ridge_angle: Some(vec![0.0]),
        };
        resample_evidence(
            &mut output,
            0,
            &evidence,
            Px::new(0.5, 0.5),
            Px::new(-0.5, 0.0),
        );
        // Both gradients point left, along the path. A vertical normal
        // describes a horizontal ridge: the outgoing line angle is zero.
        assert!((output.edge_along.unwrap()[0] - 1.0).abs() < 1e-6);
        assert!(output.ridge_angle.unwrap()[0].abs() < 1e-6);
    }

    #[test]
    fn assumed_units_survive_unrolling_and_a_changed_scale_is_refused() {
        let image = ramp();
        let mut evidence = evidence(&image);
        evidence.scale_assumed = true;
        let mut line = line(0, (0.5, 0.5), None);
        line.scale_assumed = true;
        let strip = build(&image, &evidence, &line, &FilmGeometry::UNKNOWN).unwrap();
        assert!(strip.scale_assumed);
        assert_eq!(strip.layout[0].across, Px::new(1.0, 0.0));
        let changed = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        assert_eq!(
            build(&image, &evidence, &line, &changed).unwrap_err().code,
            ErrorCode::EngineOutOfSync
        );
    }
}
