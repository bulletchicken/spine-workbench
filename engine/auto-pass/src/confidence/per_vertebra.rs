//! Each vertebra's score, band and reasons, from the chain's posterior, its
//! endplate spreads and what the film shows around it.

use crate::handoffs::{ChainBoundaries, FoundBody, SettledChain, Strip, StripRow};
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason, EndplateSpread, HIGH_FROM};
use shapes::naming::LevelNaming;
use shapes::pixels::{Px, PxRect};
use shapes::watch_frames::{LevelConfidence, WatchFrame};

/// Preserve the solver's posterior and precision separately from naming.
/// Warnings can lower a band, never increase a score. These scores remain
/// uncalibrated until checked on real films.
pub fn settle(boundaries: ChainBoundaries, strip: &Strip, naming: &LevelNaming) -> SettledChain {
    let confidence = boundaries
        .bodies
        .iter()
        .map(|body| score(body, strip, naming, &boundaries.warnings))
        .collect();
    SettledChain {
        boundaries,
        confidence,
    }
}

/// One body's confidence from a computed posterior and local warnings.
pub fn score(
    body: &FoundBody,
    strip: &Strip,
    naming: &LevelNaming,
    warnings: &[PassWarning],
) -> Confidence {
    let certainty = body.certainty;
    let score = if certainty.posterior.is_finite() {
        certainty.posterior.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut band = ConfidenceBand::for_score(score);
    let mut reasons = Vec::new();
    if body.inferred || !certainty.posterior.is_finite() {
        reasons.push(ConfidenceReason::NoEvidence);
        band = band.min(ConfidenceBand::Medium);
    }
    // Competing placements mean the boundaries did not distinguish this
    // body. No missing pedicle or unusual size is inferred from a score.
    if score < HIGH_FROM {
        reasons.push(ConfidenceReason::WeakGap);
    }
    if !naming.flags.is_empty() {
        reasons.push(ConfidenceReason::LevelUncertain);
    }
    if body.sup.row - certainty.sup_spread / strip.row_step < 0.0
        || body.inf.row + certainty.inf_spread / strip.row_step >= f64::from(strip.rows)
    {
        reasons.push(ConfidenceReason::NearFilmEdge);
    }
    for warning in warnings.iter().filter(|warning| {
        warning.film == strip.film
            && warning.level.is_none_or(|level| level == body.level)
            && warning.region.is_none_or(|region| {
                body_region(body, strip).is_some_and(|body| intersects(body, region))
            })
    }) {
        let reason_and_cap = match warning.code {
            PassWarningCode::FilmsDisagree => Some((ConfidenceReason::FilmsDisagree, true)),
            PassWarningCode::TiltInconsistent => Some((ConfidenceReason::TiltInconsistent, true)),
            PassWarningCode::MetalNearby
            | PassWarningCode::PossibleImplant
            | PassWarningCode::MarkersOverSpine => Some((ConfidenceReason::MetalNearby, true)),
            PassWarningCode::PolarityUncertain => Some((ConfidenceReason::PolarityUncertain, true)),
            PassWarningCode::LowEvidence | PassWarningCode::WeakEvidence => {
                Some((ConfidenceReason::WeakGap, false))
            }
            PassWarningCode::NearMaskEdge => Some((ConfidenceReason::NearFilmEdge, false)),
            PassWarningCode::JpegBlocks => Some((ConfidenceReason::LossyFilm, false)),
            _ => None,
        };
        if let Some((reason, capped)) = reason_and_cap {
            if !reasons.contains(&reason) {
                reasons.push(reason);
            }
            if capped {
                band = band.min(ConfidenceBand::Medium);
            }
        }
    }
    let endplate_spread_px = spread_px(strip, body.sup.row, certainty.sup_spread)
        .zip(spread_px(strip, body.inf.row, certainty.inf_spread))
        .map(|(sup, inf)| EndplateSpread { sup, inf });
    Confidence {
        score,
        band,
        reasons,
        calibrated: false,
        endplate_spread_px,
        margin: (certainty.margin.is_finite() && certainty.margin >= 0.0)
            .then_some(certainty.margin),
    }
}

/// The confidence checkpoint follows the same top-to-bottom order as the
/// settled chain and its naming lists.
pub fn frame(chain: &SettledChain) -> WatchFrame {
    WatchFrame::Confidence {
        film: chain.boundaries.film,
        stage: Stage::Confidence,
        levels: chain
            .boundaries
            .bodies
            .iter()
            .zip(&chain.confidence)
            .map(|(body, confidence)| LevelConfidence {
                level: body.level,
                score: confidence.score,
                band: confidence.band,
            })
            .collect(),
    }
}

fn spread_px(strip: &Strip, row: f64, spread: f64) -> Option<f64> {
    if !spread.is_finite() || spread < 0.0 || !strip.row_step.is_finite() || strip.row_step <= 0.0 {
        return None;
    }
    let layout = layout_at(strip, row)?;
    let along = layout.along;
    let pixels = spread * libm::hypot(along.x(), along.y()) / strip.row_step;
    pixels.is_finite().then_some(pixels)
}

fn layout_at(strip: &Strip, row: f64) -> Option<StripRow> {
    if !row.is_finite() || strip.layout.is_empty() {
        return None;
    }
    let last = strip.layout.len() - 1;
    let clipped = row.clamp(0.0, last as f64);
    let lo = libm::floor(clipped) as usize;
    let hi = (lo + 1).min(last);
    let fraction = clipped - lo as f64;
    let a = strip.layout[lo];
    let b = strip.layout[hi];
    let mix = |x: f64, y: f64| x + fraction * (y - x);
    let along = Px::new(mix(a.along.x(), b.along.x()), mix(a.along.y(), b.along.y()));
    let centre = Px::new(
        mix(a.centre.x(), b.centre.x()) + (row - clipped) * along.x(),
        mix(a.centre.y(), b.centre.y()) + (row - clipped) * along.y(),
    );
    Some(StripRow {
        centre,
        along,
        across: Px::new(
            mix(a.across.x(), b.across.x()),
            mix(a.across.y(), b.across.y()),
        ),
        half_width: mix(a.half_width, b.half_width),
        curvature: mix(a.curvature, b.curvature),
        z: mix(a.z, b.z),
    })
}

fn body_region(body: &FoundBody, strip: &Strip) -> Option<PxRect> {
    if strip.row_step <= 0.0 || strip.col_step <= 0.0 {
        return None;
    }
    let mut points = Vec::with_capacity(4);
    for endplate in [body.sup, body.inf] {
        let layout = layout_at(strip, endplate.row)?;
        // The search uses 15 length units when no width was measured.
        let half_width = if layout.half_width > 0.0 {
            layout.half_width
        } else {
            15.0
        };
        for side in [-1.0, 1.0] {
            let offset = side * half_width;
            let row = endplate.row + offset * libm::tan(endplate.tilt) / strip.row_step;
            let layout = layout_at(strip, row)?;
            points.push(Px::new(
                layout.centre.x() + offset / strip.col_step * layout.across.x(),
                layout.centre.y() + offset / strip.col_step * layout.across.y(),
            ));
        }
    }
    if !points.iter().all(|point| point.is_finite()) {
        return None;
    }
    Some(PxRect {
        left: points.iter().map(|p| p.x()).fold(f64::INFINITY, f64::min),
        right: points
            .iter()
            .map(|p| p.x())
            .fold(f64::NEG_INFINITY, f64::max),
        top: points.iter().map(|p| p.y()).fold(f64::INFINITY, f64::min),
        bottom: points
            .iter()
            .map(|p| p.y())
            .fold(f64::NEG_INFINITY, f64::max),
    })
}

fn intersects(a: PxRect, b: PxRect) -> bool {
    b.is_well_formed()
        && a.left <= b.right
        && a.right >= b.left
        && a.top <= b.bottom
        && a.bottom >= b.top
}
