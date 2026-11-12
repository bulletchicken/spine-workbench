//! The mean of a set of points, each weighted by the softmax of its score.

/// The softmax of `scores` (the largest taken away first, as PyTorch does,
/// so no power overflows) used to weight `points`. The powers are single
/// precision, as in PyTorch; the sums are double precision in a fixed
/// order, rounded once at the end.
///
/// `scores` and `points` go together one for one.
pub fn softmax_mean(scores: &[f32], points: &[[f32; 2]]) -> [f32; 2] {
    debug_assert_eq!(scores.len(), points.len());
    let largest = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut total = 0.0f64;
    let mut across = 0.0f64;
    let mut down = 0.0f64;
    for (&score, point) in scores.iter().zip(points) {
        let weight = f64::from(libm::expf(score - largest));
        total += weight;
        across += weight * f64::from(point[0]);
        down += weight * f64::from(point[1]);
    }
    [(across / total) as f32, (down / total) as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_scores_give_the_plain_mean() {
        let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert_eq!(softmax_mean(&[3.0; 4], &points), [0.5, 0.5]);
    }

    #[test]
    fn one_far_higher_score_takes_all_the_weight() {
        let points = [[0.0, 0.0], [0.25, -0.5], [1.0, 1.0]];
        assert_eq!(softmax_mean(&[-50.0, 200.0, 10.0], &points), [0.25, -0.5]);
    }

    #[test]
    fn two_scores_one_apart_weigh_one_to_e() {
        let points = [[0.0, 0.0], [1.0, 2.0]];
        // The larger score is taken away first, so the weights are e^-1 and 1.
        let lower = f64::from(libm::expf(-1.0));
        let [x, y] = softmax_mean(&[0.0, 1.0], &points);
        assert_eq!(x, (1.0 / (lower + 1.0)) as f32);
        assert_eq!(y, (2.0 / (lower + 1.0)) as f32);
    }
}
