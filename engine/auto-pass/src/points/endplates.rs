//! Local endplate refinement on the native strip grid.

use crate::handoffs::{EndplateOnStrip, Polarity, Strip};

use super::back_to_film::{layout_at, sample};

/// Experimental local search: at most 1.5 length units and 2.5 degrees
/// from the solved line. The three-sample peak gives sub-grid placement.
pub(super) fn refine(
    strip: &Strip,
    plate: EndplateOnStrip,
    superior: bool,
    polarity: Polarity,
) -> (EndplateOnStrip, bool) {
    let Some(layout) = layout_at(strip, plate.row) else {
        return (plate, false);
    };
    if layout.half_width <= 0.0 {
        return (plate, false);
    }
    refine_width(strip, plate, superior, polarity, layout.half_width)
}

pub(super) fn refine_width(
    strip: &Strip,
    plate: EndplateOnStrip,
    superior: bool,
    polarity: Polarity,
    half_width: f64,
) -> (EndplateOnStrip, bool) {
    let step = strip.row_step.min(0.5) / strip.row_step;
    let extent = (1.5 / strip.row_step / step)
        .ceil()
        .min(f64::from(strip.rows)) as i32;
    let angle_step = 0.5_f64.to_radians();
    let mut best = plate;
    let mut score = 0.0;
    for i in -extent..=extent {
        for j in -5..=5 {
            let candidate = EndplateOnStrip {
                row: plate.row + f64::from(i) * step,
                tilt: plate.tilt + f64::from(j) * angle_step,
            };
            let value = line_score(strip, candidate, half_width, superior, polarity);
            if value > score {
                best = candidate;
                score = value;
            }
        }
    }
    // A normalised image must show a positive edge, not a rounding residue.
    if score < 0.005 {
        return (plate, false);
    }
    let score_at = |line| line_score(strip, line, half_width, superior, polarity);
    let before = score_at(EndplateOnStrip {
        row: best.row - step,
        ..best
    });
    let after = score_at(EndplateOnStrip {
        row: best.row + step,
        ..best
    });
    best.row = (best.row + step * peak_offset(before, score, after)).clamp(
        plate.row - 1.5 / strip.row_step,
        plate.row + 1.5 / strip.row_step,
    );
    let middle = score_at(best);
    let before = score_at(EndplateOnStrip {
        tilt: best.tilt - angle_step,
        ..best
    });
    let after = score_at(EndplateOnStrip {
        tilt: best.tilt + angle_step,
        ..best
    });
    best.tilt = (best.tilt + angle_step * peak_offset(before, middle, after))
        .clamp(plate.tilt - 5.0 * angle_step, plate.tilt + 5.0 * angle_step);
    (best, true)
}

fn line_score(
    strip: &Strip,
    plate: EndplateOnStrip,
    half_width: f64,
    superior: bool,
    polarity: Polarity,
) -> f64 {
    let sign = if superior { 1.0 } else { -1.0 } * polarity_sign(polarity);
    let slope = libm::tan(plate.tilt) / strip.row_step;
    let mut sum = 0.0;
    let mut count = 0;
    // Stay inside the walls and outside most pedicle rims.
    for c in -12..=12 {
        let offset = f64::from(c) * half_width * 0.05;
        let row = plate.row + offset * slope;
        if let (Some(a), Some(b)) = (
            sample(strip, &strip.values, row - 0.5 / strip.row_step, offset),
            sample(strip, &strip.values, row + 0.5 / strip.row_step, offset),
        ) {
            sum += (sign * (b - a)).max(0.0);
            count += 1;
        }
    }
    if count < 10 {
        0.0
    } else {
        sum / f64::from(count)
    }
}

pub(super) fn peak_offset(before: f64, middle: f64, after: f64) -> f64 {
    let curvature = before - 2.0 * middle + after;
    if curvature < -1e-12 && middle >= before && middle >= after {
        (0.5 * (before - after) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    }
}

pub(super) fn polarity_sign(polarity: Polarity) -> f64 {
    match polarity {
        Polarity::BoneBright => 1.0,
        Polarity::BoneDark => -1.0,
    }
}
