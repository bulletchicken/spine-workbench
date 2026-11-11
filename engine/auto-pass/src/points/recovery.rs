//! Missing corners inferred in the film plane, without reusing a folded map.

use crate::handoffs::{FoundBody, Strip};
use shapes::marks::Mark;
use shapes::pixels::Px;

use super::back_to_film::layout_at;

/// Complete a quad ordered upper negative, upper positive, lower positive,
/// lower negative in the upright view. Existing finite marks are preserved,
/// except that a complete crossed or degenerate quad is inferred afresh.
/// Neighbour shapes have that same order and are in original film pixels.
pub(super) fn recover(
    strip: &Strip,
    body: &FoundBody,
    half_width: f64,
    partial: [Option<Mark>; 4],
    neighbour_shapes: &[[Px; 4]],
) -> Option<[Mark; 4]> {
    let partial = partial.map(|mark| mark.filter(|m| m.at.is_finite()));
    let present = partial.iter().filter(|mark| mark.is_some()).count();
    if present == 4 {
        let complete = partial.map(Option::unwrap);
        return if convex(&complete) {
            Some(complete)
        } else {
            recover(strip, body, half_width, [None; 4], neighbour_shapes)
        };
    }
    if present == 3 {
        let missing = partial.iter().position(Option::is_none)?;
        let a = partial[(missing + 1) % 4]?.at;
        let b = partial[(missing + 2) % 4]?.at;
        let c = partial[(missing + 3) % 4]?.at;
        let at = add(sub(a, b), c);
        if at.is_finite() {
            let completed = std::array::from_fn(|i| partial[i].unwrap_or(inferred(at)));
            if convex(&completed) {
                return Some(completed);
            }
        }
    }
    let rows = body.inf.row - body.sup.row;
    if !rows.is_finite()
        || rows <= 0.0
        || !half_width.is_finite()
        || half_width <= 0.0
        || !strip.row_step.is_finite()
        || strip.row_step <= 0.0
        || !strip.col_step.is_finite()
        || strip.col_step <= 0.0
        || !body.sup.tilt.is_finite()
        || !body.inf.tilt.is_finite()
        || body.sup.tilt.abs() >= 1.4
        || body.inf.tilt.abs() >= 1.4
    {
        return None;
    }
    let (centre, column, row) = frame(strip, body, half_width, present > 0, neighbour_shapes)?;
    let across = scaled(column, 1.0 / strip.col_step);
    let down = scaled(row, 1.0 / strip.row_step);
    let determinant = cross(across, down);
    if !centre.is_finite()
        || !across.is_finite()
        || !down.is_finite()
        || !determinant.is_finite()
        || determinant.abs() < 1e-12
    {
        return None;
    }
    // Put neighbour widths into this body's physical frame. Their median
    // may tighten the bounded prior, but never expands it past its cap.
    let mut widths = Vec::new();
    let mut slopes = Vec::new();
    for points in neighbour_shapes {
        for vector in [sub(points[1], points[0]), sub(points[2], points[3])] {
            let width = cross(vector, down) / determinant;
            let rise = cross(across, vector) / determinant;
            if width.is_finite() && width > 0.0 && rise.is_finite() {
                widths.push(0.5 * width);
                slopes.push((rise / width).clamp(-1.0, 1.0));
            }
        }
    }
    let half = median(&mut widths).map_or(half_width, |width| {
        width.clamp(0.75 * half_width, half_width)
    });
    let neighbour_slope = median(&mut slopes);
    let plate_vector = |tilt| {
        let slope = libm::tan(tilt);
        // The chain still supplies the wedge. Nearby bodies only temper
        // the common tilt when neither end of a plate survived mapping.
        let slope = neighbour_slope.map_or(slope, |near| 0.75 * slope + 0.25 * near);
        scaled(add(across, scaled(down, slope)), 2.0 * half)
    };
    let mut upper = plate_vector(body.sup.tilt);
    let mut lower = plate_vector(body.inf.tilt);
    if let (Some(a), Some(b)) = (partial[0], partial[1]) {
        upper = sub(b.at, a.at);
        lower = upper;
    } else if let (Some(a), Some(b)) = (partial[3], partial[2]) {
        lower = sub(b.at, a.at);
        upper = lower;
    }
    let mut height = scaled(row, rows);
    for (top, bottom) in [(0, 3), (1, 2)] {
        if let (Some(a), Some(b)) = (partial[top], partial[bottom]) {
            height = sub(b.at, a.at);
            break;
        }
    }
    let top = sub(centre, scaled(height, 0.5));
    let bottom = add(centre, scaled(height, 0.5));
    let template = [
        sub(top, scaled(upper, 0.5)),
        add(top, scaled(upper, 0.5)),
        add(bottom, scaled(lower, 0.5)),
        sub(bottom, scaled(lower, 0.5)),
    ];
    let mut translation = Px::new(0.0, 0.0);
    for (mark, guess) in partial.iter().zip(template) {
        if let Some(mark) = mark {
            translation = add(translation, sub(mark.at, guess));
        }
    }
    if present > 0 {
        translation = scaled(translation, 1.0 / present as f64);
    }
    let completed = std::array::from_fn(|i| {
        partial[i].unwrap_or_else(|| inferred(add(template[i], translation)))
    });
    (completed.iter().all(|mark| mark.at.is_finite()) && convex(&completed)).then_some(completed)
}

/// Find a local frame without depending on one broken centre-line sample.
/// With an anchor, a sound basis alone is enough: translation is recovered
/// from the surviving corner instead of inventing a centre-line position.
fn frame(
    strip: &Strip,
    body: &FoundBody,
    half_width: f64,
    anchored: bool,
    neighbours: &[[Px; 4]],
) -> Option<(Px, Px, Px)> {
    let middle = body.sup.row + 0.5 * (body.inf.row - body.sup.row);
    let usable = |centre: Px, across: Px, along: Px| {
        let determinant = cross(across, along);
        if !across.is_finite()
            || !along.is_finite()
            || !determinant.is_finite()
            || determinant.abs() < 1e-12
        {
            return None;
        }
        if centre.is_finite() {
            Some((centre, across, along))
        } else if anchored {
            Some((Px::new(0.0, 0.0), across, along))
        } else {
            None
        }
    };
    if let Some(row) = layout_at(strip, middle)
        && let Some(frame) = usable(row.centre, row.across, row.along)
    {
        return Some(frame);
    }
    let closest = strip
        .layout
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let distance = middle - index as f64;
            let centre = add(row.centre, scaled(row.along, distance));
            usable(centre, row.across, row.along).map(|frame| (distance.abs(), frame))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, frame)) = closest {
        return Some(frame);
    }
    if !anchored {
        return None;
    }
    // Without any strip basis, nearby complete bodies still provide a
    // local shape. Component medians keep one distorted neighbour from
    // dominating. An existing corner must locate that shape on the film.
    let mut widths = [Vec::new(), Vec::new()];
    let mut heights = [Vec::new(), Vec::new()];
    for quad in neighbours {
        if !quad.iter().all(|point| point.is_finite()) || !convex(&quad.map(Mark::auto)) {
            continue;
        }
        let width = scaled(add(sub(quad[1], quad[0]), sub(quad[2], quad[3])), 0.5);
        let height = scaled(add(sub(quad[3], quad[0]), sub(quad[2], quad[1])), 0.5);
        widths[0].push(width.x());
        widths[1].push(width.y());
        heights[0].push(height.x());
        heights[1].push(height.y());
    }
    let column = scaled(
        Px::new(median(&mut widths[0])?, median(&mut widths[1])?),
        strip.col_step / (2.0 * half_width),
    );
    let row = scaled(
        Px::new(median(&mut heights[0])?, median(&mut heights[1])?),
        1.0 / (body.inf.row - body.sup.row),
    );
    usable(Px::new(0.0, 0.0), column, row)
}

fn inferred(at: Px) -> Mark {
    Mark {
        inferred: true,
        ..Mark::auto(at)
    }
}
fn add(a: Px, b: Px) -> Px {
    Px::new(a.x() + b.x(), a.y() + b.y())
}
fn sub(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}
fn scaled(a: Px, scale: f64) -> Px {
    Px::new(a.x() * scale, a.y() * scale)
}
fn cross(a: Px, b: Px) -> f64 {
    a.x() * b.y() - a.y() * b.x()
}
fn convex(quad: &[Mark; 4]) -> bool {
    let turns: [f64; 4] = std::array::from_fn(|i| {
        cross(
            sub(quad[(i + 1) % 4].at, quad[i].at),
            sub(quad[(i + 2) % 4].at, quad[(i + 1) % 4].at),
        )
    });
    turns.iter().all(|turn| turn.is_finite())
        && (turns.iter().all(|turn| *turn > 1e-9) || turns.iter().all(|turn| *turn < -1e-9))
}
fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(f64::total_cmp);
    Some((values[(values.len() - 1) / 2] + values[values.len() / 2]) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{BodyCertainty, EndplateOnStrip, StripEvidence, StripRow};
    use shapes::film::FilmKind;
    use shapes::levels::Level;

    fn fixture() -> (Strip, FoundBody) {
        let strip = Strip {
            film: FilmKind::Front,
            rows: 41,
            cols: 9,
            row_step: 1.0,
            col_step: 1.0,
            scale_assumed: false,
            values: vec![0.0; 41 * 9],
            valid: vec![1; 41 * 9],
            layout: (0..41)
                .map(|row| StripRow {
                    centre: Px::new(100.0, 20.0 + f64::from(row)),
                    across: Px::new(1.0, 0.0),
                    along: Px::new(0.0, 1.0),
                    curvature: 1.0,
                    half_width: 10.0,
                    z: -f64::from(row),
                })
                .collect(),
            image_right: None,
            polarity_hint: None,
            evidence: StripEvidence::default(),
        };
        let body = FoundBody {
            level: Level::T1,
            sup: EndplateOnStrip {
                row: 10.0,
                tilt: 0.0,
            },
            inf: EndplateOnStrip {
                row: 30.0,
                tilt: 0.0,
            },
            inferred: false,
            certainty: BodyCertainty {
                posterior: 1.0,
                sup_spread: 0.0,
                inf_spread: 0.0,
                margin: 1.0,
            },
        };
        (strip, body)
    }

    #[test]
    fn three_corners_recover_the_fourth_without_moving_any_mark() {
        let (strip, body) = fixture();
        // A sloping parallelogram, independently specified in film pixels.
        let quad = [
            Px::new(90.0, 28.0),
            Px::new(110.0, 32.0),
            Px::new(112.0, 52.0),
            Px::new(92.0, 48.0),
        ];
        for missing in 0..4 {
            let mut partial = quad.map(|at| Some(Mark::auto(at)));
            partial[missing] = None;
            let completed = recover(&strip, &body, 10.0, partial, &[]).unwrap();
            for i in 0..4 {
                assert_eq!(completed[i].at, quad[i]);
                assert_eq!(completed[i].inferred, i == missing);
                if i != missing {
                    assert_eq!(Some(completed[i]), partial[i]);
                }
            }
        }
    }

    #[test]
    fn crossed_finite_corners_are_replaced_by_an_inferred_convex_shape() {
        let (strip, body) = fixture();
        let crossed = [
            Px::new(90.0, 30.0),
            Px::new(110.0, 50.0),
            Px::new(110.0, 30.0),
            Px::new(90.0, 50.0),
        ]
        .map(|at| Some(Mark::auto(at)));
        let result = recover(&strip, &body, 10.0, crossed, &[]).unwrap();
        assert!(convex(&result));
        assert!(result.iter().all(|mark| mark.inferred && !mark.pinned));
        assert_eq!(result[0].at, Px::new(90.0, 30.0));
        assert_eq!(result[2].at, Px::new(110.0, 50.0));
    }

    #[test]
    fn a_narrow_folded_strip_uses_the_body_frame_and_neighbour_width() {
        let (strip, body) = fixture();
        let neighbour = [
            Px::new(92.0, 0.0),
            Px::new(108.0, 0.0),
            Px::new(108.0, 20.0),
            Px::new(92.0, 20.0),
        ];
        let result = recover(&strip, &body, 10.0, [None; 4], &[neighbour]).unwrap();
        // The strip's half extent is 4 pixels and its normals fold beyond
        // one pixel, yet the independent neighbour gives a 16-pixel body.
        let expected = [
            Px::new(92.0, 30.0),
            Px::new(108.0, 30.0),
            Px::new(108.0, 50.0),
            Px::new(92.0, 50.0),
        ];
        assert_eq!(result.map(|mark| mark.at), expected);
        assert!(result.iter().all(|mark| mark.inferred && !mark.pinned));
    }

    #[test]
    fn one_surviving_corner_anchors_the_inferred_shape() {
        let (strip, body) = fixture();
        let anchor = Mark::auto(Px::new(93.0, 34.0));
        let result = recover(&strip, &body, 10.0, [Some(anchor), None, None, None], &[]).unwrap();
        assert_eq!(result[0], anchor);
        assert_eq!(result[2].at, Px::new(113.0, 54.0));
        assert!(result[1..].iter().all(|mark| mark.inferred));
        let mut invalid = body;
        invalid.inf.row = invalid.sup.row;
        assert!(recover(&strip, &invalid, 10.0, [None; 4], &[]).is_none());
    }

    #[test]
    fn an_invalid_midpoint_uses_the_nearest_sound_frame() {
        let (mut strip, body) = fixture();
        strip.layout[20].centre = Px::new(f64::NAN, f64::NAN);
        strip.layout[20].across = Px::new(0.0, 0.0);
        let result = recover(&strip, &body, 10.0, [None; 4], &[]).unwrap();
        // Row 19 is at (100,39), with a one-pixel down step. Its affine
        // extension therefore puts row 20 at (100,40), exactly.
        assert_eq!(result[0].at, Px::new(90.0, 30.0));
        assert_eq!(result[2].at, Px::new(110.0, 50.0));
        assert!(result.iter().all(|mark| mark.inferred));

        // Both stored frames can be finite and invertible while their
        // interpolated normal cancels. This still passes strip validation.
        let (mut strip, mut body) = fixture();
        strip.layout[21].across = Px::new(-1.0, 0.0);
        body.sup.row += 0.5;
        body.inf.row += 0.5;
        let result = recover(&strip, &body, 10.0, [None; 4], &[]).unwrap();
        assert_eq!(result[0].at, Px::new(90.0, 30.5));
        assert_eq!(result[2].at, Px::new(110.0, 50.5));
    }

    #[test]
    fn a_neighbour_shape_needs_an_anchor_when_every_strip_frame_is_broken() {
        let (mut strip, body) = fixture();
        for row in &mut strip.layout {
            row.centre = Px::new(f64::NAN, f64::NAN);
            row.across = Px::new(0.0, 0.0);
        }
        let neighbour = [
            Px::new(0.0, 0.0),
            Px::new(16.0, 0.0),
            Px::new(16.0, 20.0),
            Px::new(0.0, 20.0),
        ];
        let anchor = Mark::auto(Px::new(93.0, 34.0));
        let result = recover(
            &strip,
            &body,
            10.0,
            [Some(anchor), None, None, None],
            &[neighbour],
        )
        .unwrap();
        assert_eq!(result[0], anchor);
        assert_eq!(result[2].at, Px::new(109.0, 54.0));
        assert!(result[1..].iter().all(|mark| mark.inferred));
        assert!(recover(&strip, &body, 10.0, [None; 4], &[neighbour]).is_none());
    }

    #[test]
    fn a_body_beyond_the_strip_uses_the_endpoint_tangent() {
        let (strip, mut body) = fixture();
        body.sup.row = 50.0;
        body.inf.row = 70.0;
        let result = recover(&strip, &body, 10.0, [None; 4], &[]).unwrap();
        // The final row 40 is centred at y=60, and its tangent is (0,1).
        assert_eq!(result[0].at, Px::new(90.0, 70.0));
        assert_eq!(result[2].at, Px::new(110.0, 90.0));
        assert!(result.iter().all(|mark| mark.inferred));
    }
}
