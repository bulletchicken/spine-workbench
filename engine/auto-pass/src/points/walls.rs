//! Bright-inside wall fits and their intersections with the endplates.

use crate::handoffs::{EndplateOnStrip, FoundBody, Polarity, Strip};
use shapes::errors::EngineError;
use shapes::marks::Mark;
use shapes::pixels::Px;

use super::back_to_film::{layout_at, mapped, refused, sample, validate};
use super::endplates::{peak_offset, polarity_sign, refine, refine_width};

/// Ends ordered toward negative then positive upright image right.
pub(super) struct BodyCorners {
    pub sup: [Mark; 2],
    pub inf: [Mark; 2],
    pub top: EndplateOnStrip,
    pub bottom: EndplateOnStrip,
    pub half_width: f64,
}

#[derive(Clone, Copy)]
pub(super) struct Wall {
    pub offset: f64,
    pub slope: f64,
    pub middle: f64,
    pub observed: bool,
}

impl Wall {
    pub(super) fn intersection(self, strip: &Strip, plate: EndplateOnStrip) -> Option<(f64, f64)> {
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

/// A local fit may have holes where the strip ends or folds.
#[derive(Clone)]
pub(super) struct FittedBody {
    pub partial: [Option<Mark>; 4],
    pub top: EndplateOnStrip,
    pub bottom: EndplateOnStrip,
    pub half_width: f64,
}

pub(super) fn body_corners(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<BodyCorners, EngineError> {
    validate(strip)?;
    let width =
        super::widths::estimate(strip, body).ok_or_else(|| refused("invalid body height"))?;
    let fitted = fit_body(strip, body, polarity, width)?;
    complete(strip, body, &fitted, &[]).ok_or_else(|| refused("no finite body frame is available"))
}

pub(super) fn fit_body(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
    half_width: f64,
) -> Result<FittedBody, EngineError> {
    if !valid_plate(body.sup)
        || !valid_plate(body.inf)
        || body.inf.row <= body.sup.row
        || !half_width.is_finite()
        || half_width <= 0.0
    {
        return Err(refused("the body's endplates or width are invalid"));
    }
    // A bounded wall guide must not hide a supported lateral part of an
    // endplate. Retry an unseen line over the column band, still capped by
    // the body's height envelope so trunk-wide bands cannot steer its tilt.
    let refine_plate = |plate: EndplateOnStrip, superior| {
        let fitted = refine_width(strip, plate, superior, polarity, half_width);
        if fitted.1 {
            return fitted;
        }
        let wider = layout_at(strip, plate.row)
            .map(|layout| layout.half_width)
            .filter(|width| width.is_finite() && *width > half_width)
            .map(|width| {
                width.min(
                    super::widths::maximum_ratio(strip.film)
                        * (body.inf.row - body.sup.row)
                        * strip.row_step
                        * 0.5,
                )
            });
        wider.map_or(fitted, |width| {
            refine_width(strip, plate, superior, polarity, width)
        })
    };
    let (mut top, mut top_seen) = if body.inferred {
        (body.sup, false)
    } else {
        refine_plate(body.sup, true)
    };
    let (mut bottom, mut bottom_seen) = if body.inferred {
        (body.inf, false)
    } else {
        refine_plate(body.inf, false)
    };
    if bottom.row <= top.row {
        (top, bottom, top_seen, bottom_seen) = (body.sup, body.inf, false, false);
    }
    let height = (bottom.row - top.row) * strip.row_step;
    let half_width = half_width.min(super::widths::maximum_ratio(strip.film) * height * 0.5);
    let walls =
        super::wall_pairs::choose(strip, [top, bottom], half_width, polarity, !body.inferred);
    Ok(FittedBody {
        partial: [
            mark_at(strip, walls[0], top, top_seen),
            mark_at(strip, walls[1], top, top_seen),
            mark_at(strip, walls[1], bottom, bottom_seen),
            mark_at(strip, walls[0], bottom, bottom_seen),
        ],
        top,
        bottom,
        half_width: (walls[1].offset - walls[0].offset) * 0.5,
    })
}

pub(super) fn complete(
    strip: &Strip,
    body: &FoundBody,
    fitted: &FittedBody,
    neighbours: &[[Px; 4]],
) -> Option<BodyCorners> {
    let refined = FoundBody {
        sup: fitted.top,
        inf: fitted.bottom,
        ..*body
    };
    let marks = super::recovery::recover(
        strip,
        &refined,
        fitted.half_width,
        fitted.partial,
        neighbours,
    )?;
    Some(BodyCorners {
        sup: [marks[0], marks[1]],
        inf: [marks[3], marks[2]],
        top: fitted.top,
        bottom: fitted.bottom,
        half_width: fitted.half_width,
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

pub(super) fn wall_score(
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
