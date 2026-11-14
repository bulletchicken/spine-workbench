//! Real polynomial roots for projected body-contact events. Coefficients
//! are in ascending powers. Derivative roots partition monotone intervals;
//! sign-changing intervals are bisected and stationary tangencies retained.
//! This is numerical geometry, with no clinical tolerance or table value.

/// This project's bound on interval refinement. Logarithmic midpoints
/// first reduce very wide positive/negative magnitude intervals; ordinary
/// bisection then reaches adjacent representable values.
const ROOT_STEPS: usize = 80;
/// This project's round-off allowance for a stationary tangency, relative
/// to the absolute Horner sum. It does not merge distinct root positions.
const TANGENCY_ROUND_OFF: f64 = 8.0 * f64::EPSILON;

/// Distinct finite real roots, in ascending order, for degree at most four.
/// Exact leading zeros are removed; a constant, all-zero, non-finite or
/// higher-degree polynomial returns no events. Stationary roots whose
/// residual is within Horner round-off are contact candidates: the caller
/// must check the original, unsquared silhouette equation as well.
pub(super) fn real_roots(coefficients: &[f64]) -> Vec<f64> {
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Vec::new();
    }
    let Some(degree) = coefficients.iter().rposition(|&value| value != 0.0) else {
        return Vec::new();
    };
    if degree == 0 || degree > 4 {
        return Vec::new();
    }
    let first = coefficients[..=degree]
        .iter()
        .position(|&value| value != 0.0)
        .expect("nonzero polynomial");
    if first > 0 {
        let mut roots = real_roots(&coefficients[first..=degree]);
        roots.push(0.0);
        roots.sort_by(f64::total_cmp);
        roots.dedup_by(|a, b| *a == *b);
        return roots;
    }
    let magnitude = coefficients[..=degree]
        .iter()
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    // A power of two preserves exact coefficient relations, including a
    // repeated binomial root. Dividing by the arbitrary largest cell can
    // itself split a triple root through coefficient rounding.
    let exponent = magnitude.to_bits() & (0x7ff_u64 << 52);
    let scale = if exponent != 0 {
        f64::from_bits(exponent)
    } else {
        f64::from_bits(1_u64 << (63 - magnitude.to_bits().leading_zeros()))
    };
    let coefficients: Vec<_> = coefficients[..=degree]
        .iter()
        .map(|value| value / scale)
        .collect();
    solve(&coefficients)
}

fn solve(coefficients: &[f64]) -> Vec<f64> {
    let degree = coefficients.len() - 1;
    if degree == 1 {
        let root = -coefficients[0] / coefficients[1];
        return if root.is_finite() {
            vec![root]
        } else {
            Vec::new()
        };
    }
    // Scaling keeps derivative formation finite even when the original
    // coefficients were close to the largest representable number.
    let derivative: Vec<_> = coefficients
        .iter()
        .enumerate()
        .skip(1)
        .map(|(power, coefficient)| *coefficient * power as f64)
        .collect();
    let bound = root_bound(coefficients);
    let mut critical = real_roots(&derivative);
    critical.retain(|&root| root > -bound && root < bound);
    let mut points = Vec::with_capacity(critical.len() + 2);
    points.push(-bound);
    points.extend(critical);
    points.push(bound);
    let values: Vec<_> = points
        .iter()
        .map(|&point| evaluate(coefficients, point))
        .collect();
    let mut roots = Vec::new();
    let mut crosses = vec![false; points.len().saturating_sub(1)];
    for index in 0..points.len() - 1 {
        if opposite(values[index].0, values[index + 1].0) {
            crosses[index] = true;
            roots.push(bisect(
                coefficients,
                points[index],
                points[index + 1],
                values[index].0,
            ));
        }
    }
    for (index, &point) in points.iter().enumerate() {
        let (value, magnitude) = values[index];
        let adjacent_crossing = index.checked_sub(1).is_some_and(|i| crosses[i])
            || crosses.get(index).copied().unwrap_or(false);
        // A small but nonzero stationary value bracketed by crossing
        // intervals describes two close roots, not one repeated root.
        // Keep both interval roots and omit the stationary candidate.
        if value == 0.0 || (!adjacent_crossing && value.abs() <= TANGENCY_ROUND_OFF * magnitude) {
            roots.push(point);
        }
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| *a == *b);
    roots
}

fn root_bound(coefficients: &[f64]) -> f64 {
    let degree = coefficients.len() - 1;
    let leading = coefficients[degree].abs();
    let ratio = coefficients[..degree]
        .iter()
        .map(|value| value.abs() / leading)
        .fold(0.0, f64::max);
    let cauchy = (1.0 + ratio).min(f64::MAX);
    // The degree-aware bound prevents, for example, x²-1e300 from
    // needing hundreds of halvings of the much looser Cauchy interval.
    let leading_log = libm::log(leading);
    let exponent = coefficients[..degree]
        .iter()
        .enumerate()
        .filter(|(_, value)| **value != 0.0)
        .map(|(power, value)| (libm::log(value.abs()) - leading_log) / (degree - power) as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let degree_bound = (2.0 * libm::exp(exponent)).min(f64::MAX);
    let bound = cauchy.min(degree_bound);
    if bound < f64::MAX {
        bound.next_up()
    } else {
        bound
    }
}

// For |x|>1 evaluate p(x)/x^degree in the inverse coordinate. Its sign
// differs only for a negative x and odd degree; no intermediate power or
// Horner product can overflow. The absolute sum gives a local error scale.
fn evaluate(coefficients: &[f64], x: f64) -> (f64, f64) {
    let mut value = 0.0;
    let mut magnitude = 0.0;
    if x.abs() <= 1.0 {
        for &coefficient in coefficients.iter().rev() {
            value = libm::fma(value, x, coefficient);
            magnitude = libm::fma(magnitude, x.abs(), coefficient.abs());
        }
    } else {
        let inverse = x.recip();
        for &coefficient in coefficients {
            value = libm::fma(value, inverse, coefficient);
            magnitude = libm::fma(magnitude, inverse.abs(), coefficient.abs());
        }
        if x < 0.0 && (coefficients.len() - 1) % 2 == 1 {
            value = -value;
        }
    }
    (value, magnitude)
}

fn opposite(a: f64, b: f64) -> bool {
    (a < 0.0 && b > 0.0) || (a > 0.0 && b < 0.0)
}

fn midpoint(left: f64, right: f64) -> f64 {
    if left < 0.0 && right > 0.0 {
        return 0.0;
    }
    let tiny = f64::from_bits(1);
    let (near, far, sign) = if left >= 0.0 {
        (left.max(tiny), right, 1.0)
    } else {
        ((-right).max(tiny), -left, -1.0)
    };
    if far > near * 4.0 {
        sign * libm::exp((libm::log(near) + libm::log(far)) * 0.5)
    } else {
        left * 0.5 + right * 0.5
    }
}

fn bisect(coefficients: &[f64], mut left: f64, mut right: f64, mut left_value: f64) -> f64 {
    for _ in 0..ROOT_STEPS {
        let middle = midpoint(left, right);
        if middle <= left || middle >= right {
            break;
        }
        let value = evaluate(coefficients, middle).0;
        if value == 0.0 {
            return middle;
        }
        if opposite(left_value, value) {
            right = middle;
        } else {
            left = middle;
            left_value = value;
        }
    }
    if evaluate(coefficients, left).0.abs() <= evaluate(coefficients, right).0.abs() {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polynomial(roots: &[f64]) -> Vec<f64> {
        let mut coefficients = vec![1.0];
        for &root in roots {
            let mut next = vec![0.0; coefficients.len() + 1];
            for (power, &coefficient) in coefficients.iter().enumerate() {
                next[power] -= root * coefficient;
                next[power + 1] += coefficient;
            }
            coefficients = next;
        }
        coefficients
    }

    fn assert_roots(coefficients: &[f64], expected: &[f64], tolerance: f64) {
        let roots = real_roots(coefficients);
        assert_eq!(roots.len(), expected.len(), "{coefficients:?}: {roots:?}");
        for (&actual, &expected) in roots.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
                "{actual} versus {expected}"
            );
        }
    }

    #[test]
    fn the_four_degrees_keep_their_factored_answers() {
        assert_roots(&[6.0, 2.0], &[-3.0], 1e-14);
        assert_roots(&polynomial(&[-2.0, 3.0]), &[-2.0, 3.0], 1e-14);
        assert_roots(&polynomial(&[-2.0, 0.0, 3.0]), &[-2.0, 0.0, 3.0], 1e-14);
        assert_roots(
            &polynomial(&[-2.0, -1.0, 1.0, 3.0]),
            &[-2.0, -1.0, 1.0, 3.0],
            1e-13,
        );
    }

    #[test]
    fn repeated_roots_are_single_contact_candidates() {
        assert_roots(&polynomial(&[1.0, 1.0]), &[1.0], 1e-14);
        assert_roots(&polynomial(&[-2.0, 1.0, 1.0]), &[-2.0, 1.0], 1e-13);
        assert_roots(&polynomial(&[-1.0, -1.0, 2.0, 2.0]), &[-1.0, 2.0], 1e-13);
        assert_roots(&polynomial(&[1.0, 1.0, 1.0, 1.0]), &[1.0], 1e-13);
        for degree in 1..=4 {
            let mut coefficients = vec![0.0; degree + 1];
            coefficients[degree] = 1.0;
            assert_roots(&coefficients, &[0.0], 0.0);
        }
    }

    #[test]
    fn nearby_distinct_roots_are_not_merged_by_an_angular_tolerance() {
        let expected = [-1.0, 0.0001, 0.0002, 2.0];
        assert_roots(&polynomial(&expected), &expected, 1e-13);
    }

    #[test]
    fn multiplying_every_coefficient_leaves_the_roots_unchanged() {
        let expected = [-1.0, 0.0001, 0.0002, 2.0];
        let coefficients = polynomial(&expected);
        for scale in [-1e200, 1e-200, 7.0] {
            let scaled: Vec<_> = coefficients.iter().map(|value| value * scale).collect();
            assert_roots(&scaled, &expected, 1e-12);
        }
    }

    #[test]
    fn empty_constant_and_nonreal_polynomials_supply_no_events() {
        for coefficients in [&[][..], &[0.0, 0.0][..], &[2.0][..], &[1.0, 0.0, 1.0][..]] {
            assert!(real_roots(coefficients).is_empty());
        }
        assert_roots(&[-2.0, 1.0, 0.0, 0.0], &[2.0], 1e-14);
    }

    #[test]
    fn wide_root_magnitudes_do_not_overflow_horner_or_the_interval_midpoint() {
        assert_roots(&[-1.0, 0.0, 1e-300], &[-1e150, 1e150], 1e-12);
        assert_roots(&[1.0, -1e150, 1.0], &[1e-150, 1e150], 1e-12);
    }
}
