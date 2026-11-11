//! Exact squared Euclidean distance to a mask on a sampled grid.
//! Separable lower envelopes of parabolas (Felzenszwalb and Huttenlocher,
//! Theory of Computing 8, 2012). Empty masks have infinite distance.

pub(super) fn squared(mask: &[bool], width: usize, height: usize) -> Vec<f64> {
    let mut result = vec![f64::INFINITY; mask.len()];
    for x in 0..width {
        let column: Vec<_> = (0..height)
            .map(|y| {
                if mask[y * width + x] {
                    0.
                } else {
                    f64::INFINITY
                }
            })
            .collect();
        let transformed = line(&column);
        for y in 0..height {
            result[y * width + x] = transformed[y];
        }
    }
    for row in result.chunks_mut(width) {
        row.copy_from_slice(&line(row));
    }
    result
}
fn line(values: &[f64]) -> Vec<f64> {
    let mut sites: Vec<usize> = Vec::new();
    let mut breaks: Vec<f64> = Vec::new();
    for (q, &value) in values.iter().enumerate().filter(|(_, v)| v.is_finite()) {
        let mut s = f64::NEG_INFINITY;
        while let Some(&p) = sites.last() {
            s = ((value + (q * q) as f64) - (values[p] + (p * p) as f64)) / (2. * (q - p) as f64);
            if s > *breaks.last().expect("one breakpoint per site") {
                break;
            }
            sites.pop();
            breaks.pop();
        }
        if sites.is_empty() {
            s = f64::NEG_INFINITY;
        }
        sites.push(q);
        breaks.push(s);
    }
    let mut result = vec![f64::INFINITY; values.len()];
    if sites.is_empty() {
        return result;
    }
    let mut k = 0;
    for (q, value) in result.iter_mut().enumerate() {
        while k + 1 < sites.len() && breaks[k + 1] < (q as f64) {
            k += 1;
        }
        let d = q as f64 - sites[k] as f64;
        *value = d * d + values[sites[k]];
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distance_to_a_single_pixel_is_pythagoras() {
        let mut mask = vec![false; 35];
        mask[9] = true;
        let d = squared(&mask, 7, 5);
        for y in 0..5 {
            for x in 0..7 {
                let dx = x as f64 - 2.;
                let dy = y as f64 - 1.;
                assert_eq!(d[y * 7 + x], dx * dx + dy * dy);
            }
        }
        assert!(squared(&[false; 4], 2, 2).iter().all(|v| v.is_infinite()));
    }
}
