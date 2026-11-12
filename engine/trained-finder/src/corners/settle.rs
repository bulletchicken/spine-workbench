//! A corner moved part of the way to where the network points, never far.
//!
//! Each corner's move is first capped at a quarter of the outline's height,
//! then only `blend` of it is taken (0.375 for the trained network), so no
//! corner moves more than about a tenth of a vertebra's height whatever the
//! network says.

use super::patch::Patch;
use crate::outline::Quad;

/// The cap on one corner's move, as a share of the outline's height.
const CAP: f64 = 0.25;

/// `quad` with each corner moved by its delta (in patch units, from the
/// corner network), capped and blended. Positions are picture pixels as in
/// [`super::patch`].
pub fn settle(
    quad_in_film_pixels: &Quad,
    deltas: &[[f32; 2]; 4],
    patch: &Patch,
    blend: f64,
) -> Quad {
    let q = quad_in_film_pixels;
    let axes = patch.axes;
    let distance = |a: [f64; 2], b: [f64; 2]| {
        let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
        (dx * dx + dy * dy).sqrt()
    };
    // The outline's height before it moves: the mean of its two sides.
    let height = (distance(q[0], q[3]) + distance(q[1], q[2])) / 2.0;
    let mut settled = *q;
    for (corner, delta) in settled.iter_mut().zip(deltas) {
        let delta = delta.map(f64::from);
        let mut shift =
            [0, 1].map(|row| (delta[0] * axes[row][0] + delta[1] * axes[row][1]) * patch.extent);
        let length = (shift[0] * shift[0] + shift[1] * shift[1]).sqrt();
        let keep = 1.0f64.min(CAP * height / length.max(1e-8));
        shift = shift.map(|v| v * keep);
        *corner = [corner[0] + blend * shift[0], corner[1] + blend * shift[1]];
    }
    settled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corners::patch::cut;
    use darkroom::grey_image::{GreyImage, Placement};

    fn square() -> (Quad, Patch) {
        let picture = GreyImage::new(100, 80, vec![0; 8000], Placement::FILM).unwrap();
        let quad = [[40.0, 30.0], [60.0, 30.0], [60.0, 50.0], [40.0, 50.0]];
        let patch = cut(&picture, &quad);
        (quad, patch)
    }

    #[test]
    fn a_small_move_is_blended() {
        // 0.05 patch units across a 34-pixel patch is 1.7 pixels, under the
        // cap of 5 (a quarter of the height 20); a blend of 0.5 takes 0.85.
        let (quad, patch) = square();
        let deltas = [[0.05, 0.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]];
        let settled = settle(&quad, &deltas, &patch, 0.5);
        assert!((settled[0][0] - (40.0 + 0.85)).abs() < 1e-6);
        assert_eq!(settled[0][1], 30.0);
        assert_eq!(settled[1], quad[1]);
    }

    #[test]
    fn a_long_move_is_capped_at_a_quarter_height() {
        // 0.5 patch units down is 17 pixels; capped to 5, then all of it.
        let (quad, patch) = square();
        let deltas = [[0.0, 0.0], [0.0, 0.0], [0.0, 0.5], [0.0, 0.0]];
        let settled = settle(&quad, &deltas, &patch, 1.0);
        assert!((settled[2][1] - 55.0).abs() < 1e-12);
        assert!((settled[2][0] - 60.0).abs() < 1e-12);
    }
}
