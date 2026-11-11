//! Bright-inside wall fits and their intersections with the endplates.

use crate::handoffs::{EndplateOnStrip, FoundBody, Polarity, Strip};
use shapes::errors::EngineError;
use shapes::marks::Mark;

use super::back_to_film::{layout_at, mapped, refused, sample, validate};
use super::endplates::{peak_offset, polarity_sign, refine};

/// Ends ordered toward negative then positive upright image right.
pub(super) struct BodyCorners {
    pub sup: [Mark; 2],
    pub inf: [Mark; 2],
    pub top: EndplateOnStrip,
    pub bottom: EndplateOnStrip,
    pub half_width: f64,
}

#[derive(Clone, Copy)]
struct Wall {
    offset: f64,
    slope: f64,
    middle: f64,
    observed: bool,
}

impl Wall {
    fn intersection(self, strip: &Strip, plate: EndplateOnStrip) -> Option<(f64, f64)> {
        let tilt = libm::tan(plate.tilt);
        let denominator = 1.0 - self.slope * tilt;
        if denominator.abs() < 0.2 {
            return None;
        }
        let along = (plate.row * strip.row_step + (self.offset - self.slope * self.middle) * tilt)
            / denominator;
        let offset = self.offset + self.slope * (along - self.middle);
        Some((along / strip.row_step, offset))
    }
}

pub(super) fn body_corners(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<BodyCorners, EngineError> {
    validate(strip)?;
    if !valid_plate(body.sup) || !valid_plate(body.inf) || body.inf.row <= body.sup.row {
        return Err(refused("the body's endplates are invalid or reversed"));
    }
    let half_width = layout_at(strip, (body.sup.row + body.inf.row) * 0.5)
        .map(|r| r.half_width)
        .filter(|w| *w > 0.0)
        .ok_or_else(|| refused("the strip has no body width"))?;
    let (top, top_seen) = if body.inferred {
        (body.sup, false)
    } else {
        refine(strip, body.sup, true, polarity)
    };
    let (bottom, bottom_seen) = if body.inferred {
        (body.inf, false)
    } else {
        refine(strip, body.inf, false, polarity)
    };
    if bottom.row <= top.row {
        return Err(refused("refinement crossed the endplates"));
    }
    let walls = [-1.0, 1.0].map(|side| {
        fit(
            strip,
            [top, bottom],
            side * half_width,
            polarity,
            1.6,
            !body.inferred,
        )
    });
    let mark = |wall: Wall, plate, seen| {
        mark_at(strip, wall, plate, seen && !body.inferred)
            .ok_or_else(|| refused("a corner lies outside the strip or in a fold"))
    };
    Ok(BodyCorners {
        sup: [
            mark(walls[0], top, top_seen)?,
            mark(walls[1], top, top_seen)?,
        ],
        inf: [
            mark(walls[0], bottom, bottom_seen)?,
            mark(walls[1], bottom, bottom_seen)?,
        ],
        top,
        bottom,
        half_width: (walls[1].offset - walls[0].offset) * 0.5,
    })
}

fn valid_plate(plate: EndplateOnStrip) -> bool {
    plate.row.is_finite() && plate.tilt.is_finite() && plate.tilt.abs() < 1.4
}

fn mark_at(strip: &Strip, wall: Wall, plate: EndplateOnStrip, seen: bool) -> Option<Mark> {
    let (row, offset) = wall.intersection(strip, plate)?;
    let col = f64::from(strip.cols - 1) * 0.5 + offset / strip.col_step;
    let at = mapped(strip, row, col)?;
    Some(Mark {
        inferred: !seen || !wall.observed || sample(strip, &strip.values, row, offset).is_none(),
        ..Mark::auto(at)
    })
}

fn fit(
    strip: &Strip,
    plates: [EndplateOnStrip; 2],
    expected: f64,
    polarity: Polarity,
    maximum: f64,
    observed: bool,
) -> Wall {
    let [top, bottom] = plates;
    let middle = 0.5 * (top.row + bottom.row) * strip.row_step;
    let height = (bottom.row - top.row) * strip.row_step;
    let mut best = Wall {
        offset: expected,
        slope: 0.0,
        middle,
        observed: false,
    };
    if !observed || height <= 0.0 {
        return best;
    }
    // Experimental range: 0.5..1.6 times the measured half width. The
    // wall may taper by at most 3 length units and ten degrees.
    let side = expected.signum();
    let low = 0.5 * expected.abs();
    let high = (maximum * expected.abs()).min(f64::from(strip.cols - 1) * strip.col_step * 0.5);
    if !high.is_finite() || high <= low {
        return best;
    }
    let steps = ((high - low) / 0.5)
        .ceil()
        .max(1.0)
        .min(f64::from(strip.cols)) as usize;
    let spacing = (high - low) / steps as f64;
    let max_slope = (3.0 / height).min(libm::tan(10.0_f64.to_radians()));
    let mut score = 0.0;
    for i in 0..=steps {
        for j in -6..=6 {
            let candidate = Wall {
                offset: side * (low + i as f64 * spacing),
                slope: f64::from(j) * max_slope / 6.0,
                ..best
            };
            let value = wall_score(strip, plates, candidate, side, polarity);
            // Bilinear edges can give a flat maximum across several
            // slopes. Ignore reduction roundoff and prefer the straighter
            // wall, rather than a different taper at each end.
            if value > score + 1e-10
                || ((value - score).abs() <= 1e-10 && candidate.slope.abs() < best.slope.abs())
            {
                best = candidate;
                score = value;
            }
        }
    }
    if score < 0.005 {
        return Wall {
            offset: expected,
            slope: 0.0,
            middle,
            observed: false,
        };
    }
    let score_at = |offset| wall_score(strip, plates, Wall { offset, ..best }, side, polarity);
    let shift = spacing
        * peak_offset(
            score_at(best.offset - spacing),
            score,
            score_at(best.offset + spacing),
        );
    best.offset = side * (side * (best.offset + shift)).clamp(low, high);
    let slope_step = max_slope / 6.0;
    let at_slope = |slope| wall_score(strip, plates, Wall { slope, ..best }, side, polarity);
    best.slope = (best.slope
        + slope_step
            * peak_offset(
                at_slope(best.slope - slope_step),
                at_slope(best.slope),
                at_slope(best.slope + slope_step),
            ))
    .clamp(-max_slope, max_slope);
    best.observed = true;
    best
}

fn wall_score(
    strip: &Strip,
    plates: [EndplateOnStrip; 2],
    wall: Wall,
    side: f64,
    polarity: Polarity,
) -> f64 {
    let Some((top, _)) = wall.intersection(strip, plates[0]) else {
        return 0.0;
    };
    let Some((bottom, _)) = wall.intersection(strip, plates[1]) else {
        return 0.0;
    };
    if bottom <= top {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut count = 0;
    for i in 0..17 {
        // Exclude rounded rim corners, while still constraining both ends.
        let fraction = 0.12 + f64::from(i) * 0.76 / 16.0;
        let row = top + fraction * (bottom - top);
        let offset = wall.offset + wall.slope * (row * strip.row_step - wall.middle);
        if let (Some(inside), Some(outside)) = (
            sample(strip, &strip.values, row, offset - side * 0.5),
            sample(strip, &strip.values, row, offset + side * 0.5),
        ) {
            sum += (polarity_sign(polarity) * (inside - outside)).max(0.0);
            count += 1;
        }
    }
    if count < 7 {
        0.0
    } else {
        sum / f64::from(count)
    }
}

pub(super) fn plateau_ends(
    strip: &Strip,
    plateau: EndplateOnStrip,
    polarity: Polarity,
) -> Option<[Mark; 2]> {
    if !valid_plate(plateau) {
        return None;
    }
    let half = layout_at(strip, plateau.row)?.half_width;
    if half <= 0.0 {
        return None;
    }
    let (top, seen) = refine(strip, plateau, true, polarity);
    if !seen {
        return None;
    }
    let bottom = EndplateOnStrip {
        row: top.row + half * 0.6 / strip.row_step,
        ..top
    };
    let walls = [-half, half].map(|offset| fit(strip, [top, bottom], offset, polarity, 1.5, true));
    let marks = [
        mark_at(strip, walls[0], top, true)?,
        mark_at(strip, walls[1], top, true)?,
    ];
    marks.iter().all(|mark| !mark.inferred).then_some(marks)
}
