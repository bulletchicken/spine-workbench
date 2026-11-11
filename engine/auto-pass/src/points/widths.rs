//! Width estimates shared by the two walls and neighbouring vertebrae.

use crate::handoffs::{FoundBody, Strip};
use shapes::film::FilmKind;
use shapes::levels::Region;

use super::back_to_film::layout_at;

/// Experimental projection envelope, not a diagnostic anatomical limit.
/// The body-size table used by made-up spines derives from Busscher et al.
/// (2010, PMC2900026): its largest lower-width/height ratio, including
/// size jitter, is 2.61; depth/height is below 1.9. These rounded bounds
/// leave projection room while excluding a trunk-sized pair of walls.
pub(super) fn maximum_ratio(film: FilmKind) -> f64 {
    if film == FilmKind::Side { 2.2 } else { 2.8 }
}

pub(super) fn height(strip: &Strip, body: &FoundBody) -> Option<f64> {
    let height = (body.inf.row - body.sup.row) * strip.row_step;
    (height.is_finite() && height > 0.0).then_some(height)
}

pub(super) fn estimate(strip: &Strip, body: &FoundBody) -> Option<f64> {
    let height = height(strip, body)?;
    // Broad regional centres from the same table. They bound how much an
    // imperfect column-width measurement can steer a local wall search.
    let ratio = match (strip.film, body.level.region()) {
        (FilmKind::Side, _) => 1.35,
        (_, Region::Lumbar) => 2.0,
        _ => 1.75,
    };
    let nominal = 0.5 * height * ratio;
    let mut samples: Vec<f64> = (0..5)
        .filter_map(|i| {
            let row = body.sup.row + (0.2 + f64::from(i) * 0.15) * (body.inf.row - body.sup.row);
            layout_at(strip, row)
                .map(|row| row.half_width)
                .filter(|width| width.is_finite() && *width > 0.0)
        })
        .collect();
    Some(
        median(&mut samples)
            .unwrap_or(nominal)
            .clamp(0.65 * nominal, 1.25 * nominal),
    )
}

pub(super) fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable_by(f64::total_cmp);
    Some((values[(values.len() - 1) / 2] + values[values.len() / 2]) * 0.5)
}
