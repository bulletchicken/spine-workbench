//! Stage 2: trace, refine and measure the spine's centre line.
//!
//! [`trace`] returns the frozen stage hand-off. [`trace_with_checkpoints`]
//! additionally keeps the fixed, data-derived frames for the watchable pass.
//! The working image is upright, while every returned position and vector
//! remains in original film pixels.

mod body_walls;
pub mod cheapest_path;
pub mod cost_map;
mod refine;
pub mod smooth;

use crate::handoffs::{CentreLine, CentreLineEnds, CentreSample, FilmEvidence};
use darkroom::grey_image::GreyImage;
use shapes::auto_pass::{PassWarningCode, Stage};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::film_geometry::FilmGeometry;
use shapes::pixels::Px;
use shapes::watch_frames::{MapKind, WatchFrame, WidthSample};

use cost_map::{CostMap, warning};
use smooth::Point;

/// Trace a film from its neck zone toward its pelvis, in physical units
/// when the evidence has a scale and assumed units otherwise.
pub fn trace(evidence: &FilmEvidence, geometry: &FilmGeometry) -> Result<CentreLine, EngineError> {
    run(evidence, geometry, false).map(|(line, _)| line)
}

/// Trace and retain fixed checkpoints: end zones, spine-scale ridge,
/// spreading search, coarse and refined lines, then measured widths.
/// Frame generation does not alter the numerical result.
pub fn trace_with_checkpoints(
    evidence: &FilmEvidence,
    geometry: &FilmGeometry,
) -> Result<(CentreLine, Vec<WatchFrame>), EngineError> {
    run(evidence, geometry, true)
}

fn run(
    evidence: &FilmEvidence,
    geometry: &FilmGeometry,
    checkpoints: bool,
) -> Result<(CentreLine, Vec<WatchFrame>), EngineError> {
    let mut map = CostMap::build(evidence, geometry)?;
    let mut path = cheapest_path::search(&map, true, checkpoints).ok_or_else(|| {
        EngineError::new(
            ErrorCode::PassNoSpineFound,
            "no path joins the spine end zones",
        )
    })?;
    let knee_length = path
        .points
        .windows(2)
        .filter(|pair| {
            let dx = (pair[1].x - pair[0].x).abs();
            let dy = (pair[1].y - pair[0].y).abs();
            libm::atan2(dx, dy).to_degrees() > 55.0
        })
        .map(|pair| pair[0].distance(pair[1]))
        .sum::<f64>();
    if knee_length > 40.0 {
        if let Some(alternative) = cheapest_path::search(&map, false, false) {
            let strength = |points: &[Point]| {
                points
                    .iter()
                    .map(|&p| map.sample(&map.ridge, p))
                    .sum::<f64>()
                    / points.len() as f64
            };
            if strength(&alternative.points) > strength(&path.points) {
                path.points = alternative.points;
            }
        }
        map.warnings
            .push(warning(PassWarningCode::SevereCurve, evidence.film));
    }
    let points = smooth::resample(&path.points);
    if points.len() < 12 {
        return Err(EngineError::new(
            ErrorCode::PassNoSpineFound,
            "the traced spine is too short",
        ));
    }
    let guide = smooth::fit(&points, &vec![1.0; points.len()], 100.0, 1.0);
    let front = evidence.film != FilmKind::Side;
    let (refined, weights) = refine::refine(&map, &guide, front);
    let fine_curve = smooth::fit(&refined, &weights, 50.0, 0.25);
    let width_curve: Vec<_> = fine_curve.iter().step_by(4).copied().collect();
    let widths = refine::widths(&map, &width_curve, front);
    let placement = map.image.placement();
    let to_vector = |p: Point| {
        Px::new(
            (p.x * placement.step_across.x() + p.y * placement.step_down.x()) / map.spacing,
            (p.x * placement.step_across.y() + p.y * placement.step_down.y()) / map.spacing,
        )
    };
    let samples: Vec<_> = fine_curve
        .iter()
        .map(|s| {
            let i = (s.arc.floor() as usize).min(widths.values.len() - 1);
            let next = (i + 1).min(widths.values.len() - 1);
            let fraction = s.arc - s.arc.floor();
            CentreSample {
                arc: s.arc,
                at: map.film_point(s.point),
                along: to_vector(s.tangent),
                curvature: s.curvature,
                width: widths.values[i] + fraction * (widths.values[next] - widths.values[i]),
                width_measured: widths.measured[i],
                evidence: map.sample(&map.ridge, s.point),
            }
        })
        .collect();
    let nearest_arc = |y: f64| {
        fine_curve
            .iter()
            .min_by(|a, b| (a.point.y - y).abs().total_cmp(&(b.point.y - y).abs()))
            .map(|p| p.arc)
    };
    let ends = CentreLineEnds {
        neck_base: map.neck_y.and_then(nearest_arc),
        iliac_crest: map.crest_y.and_then(nearest_arc),
        top_cropped: map.top_cropped,
        bottom_cropped: map.bottom_cropped,
        size_factor: map.size_factor,
    };
    if widths.measured.iter().any(|&measured| !measured) {
        map.warnings
            .push(warning(PassWarningCode::WidthFallback, evidence.film));
    }
    if samples.iter().map(|s| s.evidence).sum::<f64>() / (samples.len() as f64) < 0.35 {
        map.warnings
            .push(warning(PassWarningCode::WeakEvidence, evidence.film));
    }
    let first = fine_curve.first().expect("nonempty curve");
    let last = fine_curve.last().expect("nonempty curve");
    if last.arc / first.point.distance(last.point).max(1e-6) > 1.6 {
        map.warnings
            .push(warning(PassWarningCode::Tortuous, evidence.film));
    }
    if fine_curve
        .iter()
        .any(|s| s.curvature.abs() > 1.0 / (40.0 * map.size_factor))
    {
        map.warnings
            .push(warning(PassWarningCode::TightCurve, evidence.film));
    }
    let near_edge = fine_curve
        .iter()
        .filter(|s| {
            let w = f64::from(map.image.width() - 1) * map.spacing;
            let h = f64::from(map.image.height() - 1) * map.spacing;
            s.point
                .x
                .min(w - s.point.x)
                .min(s.point.y)
                .min(h - s.point.y)
                < 10.0
        })
        .count() as f64
        * 0.25;
    if near_edge > 20.0 {
        map.warnings
            .push(warning(PassWarningCode::NearMaskEdge, evidence.film));
    }
    let line = CentreLine {
        film: evidence.film,
        quarter_turns: geometry.quarter_turns,
        scale_assumed: evidence.scale_assumed,
        sample_step: 0.25,
        samples,
        ends,
        warnings: map.warnings.clone(),
    };
    let mut frames = Vec::new();
    if checkpoints {
        let film = evidence.film;
        let stage = Stage::CentreLine;
        frames.push(WatchFrame::StageStarted { film, stage });
        frames.push(WatchFrame::EndZones {
            film,
            stage,
            top: map.zone_bounds(&map.top),
            bottom: map.zone_bounds(&map.bottom),
            top_cropped: map.top_cropped,
            bottom_cropped: map.bottom_cropped,
        });
        let total = 1 + path.wavefronts.len() as u32;
        let image = GreyImage::new(
            map.image.width(),
            map.image.height(),
            map.ridge.clone(),
            placement,
        )
        .expect("same grid");
        frames.push(WatchFrame::Evidence {
            film,
            stage,
            map: MapKind::RidgeHeat,
            step: 1,
            steps: total,
            is_final: true,
            scale_mm: None,
            tile: darkroom::preview::unit_tile(
                &darkroom::preview::decimated(&image).expect("nonempty grid"),
            ),
        });
        for (i, values) in path.wavefronts.into_iter().enumerate() {
            let image = GreyImage::new(map.image.width(), map.image.height(), values, placement)
                .expect("same grid");
            frames.push(WatchFrame::Evidence {
                film,
                stage,
                map: MapKind::Wavefront,
                step: i as u32 + 2,
                steps: total,
                is_final: i as u32 + 2 == total,
                scale_mm: None,
                tile: darkroom::preview::unit_tile(
                    &darkroom::preview::decimated(&image).expect("nonempty grid"),
                ),
            });
        }
        frames.push(WatchFrame::CentreLine {
            film,
            stage,
            points: path.points.iter().map(|&p| map.film_point(p)).collect(),
            refined: false,
        });
        frames.push(WatchFrame::CentreLine {
            film,
            stage,
            points: line.samples.iter().map(|s| s.at).collect(),
            refined: true,
        });
        let width_samples = width_curve
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let half = 0.5 * widths.values[i];
                WidthSample {
                    left: map.film_point(Point {
                        x: s.point.x - half * s.tangent.y,
                        y: s.point.y + half * s.tangent.x,
                    }),
                    right: map.film_point(Point {
                        x: s.point.x + half * s.tangent.y,
                        y: s.point.y - half * s.tangent.x,
                    }),
                    confidence: if widths.measured[i] {
                        widths.confidence[i]
                    } else {
                        0.0
                    },
                }
            })
            .collect();
        frames.push(WatchFrame::Width {
            film,
            stage,
            samples: width_samples,
        });
    }
    Ok((line, frames))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{Polarity, PolarityCheck};
    use darkroom::grey_image::Placement;

    fn bar(curved: bool) -> FilmEvidence {
        let width = 181;
        let height = 341;
        let centre = |y: f64| {
            if curved {
                90.0 + 20.0 * libm::sin(y / 75.0)
            } else {
                90.0
            }
        };
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let distance = (f64::from(x) - centre(f64::from(y))).abs();
                let body = 1.0 / (1.0 + libm::exp((distance - 15.0) / 0.7));
                let gap = (y % 32 < 4) as u8;
                (0.05 + body * (0.75 - 0.3 * f64::from(gap))) as f32
            })
            .collect();
        let image = GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes");
        let zero = image.map(|_| 0.0);
        FilmEvidence {
            film: FilmKind::Front,
            mm_per_px: 1.0,
            scale_assumed: false,
            valid: image.map(|_| 1u8),
            image,
            phase_edge: zero.clone(),
            phase_direction: zero.clone(),
            phase_kind: zero.clone(),
            gradient_edge: zero.clone(),
            gradient_direction: zero.clone(),
            ridge: zero.clone(),
            ridge_orientation: zero.clone(),
            valley: zero.clone(),
            valley_orientation: zero.clone(),
            valley_scale: zero.map(|_| 0u8),
            ridge_symmetry: zero.clone(),
            valley_symmetry: zero,
            rings: None,
            hessian_scales_mm: vec![1.0],
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
        }
    }

    #[test]
    fn a_straight_bar_stays_centred_and_has_the_known_width() {
        let evidence = bar(false);
        let geometry = FilmGeometry {
            mm_per_px: Some(1.0),
            ..FilmGeometry::UNKNOWN
        };
        let line = trace(&evidence, &geometry).expect("the bright bar is traceable");
        for sample in line
            .samples
            .iter()
            .filter(|s| s.at.y() > 40.0 && s.at.y() < 300.0)
        {
            // One-unit cut-line sampling and a two-unit search grid permit
            // a one-unit centre error; the analytic full bar width is 30.
            assert!(
                (sample.at.x() - 90.0).abs() < 1.0,
                "centre {}",
                sample.at.x()
            );
            assert!((sample.width - 30.0).abs() < 3.0, "width {}", sample.width);
        }
    }

    #[test]
    fn a_curved_bar_stays_within_the_path_distance_budget() {
        let evidence = bar(true);
        let geometry = FilmGeometry {
            mm_per_px: Some(1.0),
            ..FilmGeometry::UNKNOWN
        };
        let line = trace(&evidence, &geometry).expect("the curved bar is traceable");
        let errors: Vec<_> = line
            .samples
            .iter()
            .filter(|s| s.at.y() > 40.0 && s.at.y() < 300.0)
            .map(|s| (s.at.x() - 90.0 - 20.0 * libm::sin(s.at.y() / 75.0)).abs())
            .collect();
        // The synthetic ridge is defined directly in film coordinates,
        // independently of the spline and path implementation.
        assert!(errors.iter().sum::<f64>() / errors.len() as f64 <= 1.0);
        assert!(errors.into_iter().fold(0.0f64, f64::max) <= 3.0);
    }

    #[test]
    fn malformed_evidence_is_refused_before_searching() {
        let geometry = FilmGeometry {
            mm_per_px: Some(1.0),
            ..FilmGeometry::UNKNOWN
        };
        let mut evidence = bar(false);
        evidence.valid = GreyImage::new(2, 2, vec![1; 4], Placement::FILM).expect("sizes");
        assert_eq!(
            trace(&evidence, &geometry)
                .expect_err("mismatched mask")
                .code,
            ErrorCode::EngineOutOfSync
        );
        let mut evidence = bar(false);
        evidence.image = evidence.image.map(|_| f32::NAN);
        assert_eq!(
            trace(&evidence, &geometry)
                .expect_err("non-finite image")
                .code,
            ErrorCode::EngineOutOfSync
        );
        let mut evidence = bar(false);
        evidence.valid = evidence.valid.map(|_| 2u8);
        assert_eq!(
            trace(&evidence, &geometry)
                .expect_err("non-binary mask")
                .code,
            ErrorCode::EngineOutOfSync
        );
    }
    #[test]
    fn checkpoints_preserve_the_answer_and_finish_with_real_widths() {
        let evidence = bar(false);
        let geometry = FilmGeometry {
            mm_per_px: Some(1.0),
            ..FilmGeometry::UNKNOWN
        };
        let direct = trace(&evidence, &geometry).expect("trace");
        let (line, frames) = trace_with_checkpoints(&evidence, &geometry).expect("watchable trace");
        assert_eq!(line, direct);
        assert!(matches!(
            frames.first(),
            Some(WatchFrame::StageStarted {
                stage: Stage::CentreLine,
                ..
            })
        ));
        assert!(matches!(frames.get(1), Some(WatchFrame::EndZones { .. })));
        let mut wavefronts = 0;
        for frame in &frames {
            match frame {
                WatchFrame::Evidence { map, tile, .. } => {
                    if *map == MapKind::Wavefront {
                        wavefronts += 1;
                    }
                    assert!(tile.data.len() <= 2_000_000);
                }
                WatchFrame::CentreLine {
                    points,
                    refined: true,
                    ..
                } => {
                    assert_eq!(
                        *points,
                        line.samples
                            .iter()
                            .map(|sample| sample.at)
                            .collect::<Vec<_>>()
                    );
                }
                _ => {}
            }
        }
        assert!(wavefronts <= 16);
        assert!(
            matches!(frames.last(), Some(WatchFrame::Width { samples, .. }) if !samples.is_empty())
        );
    }
}
