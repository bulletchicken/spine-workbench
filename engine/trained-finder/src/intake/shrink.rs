//! A picture shrunk (or stretched) to the 384 by 768 the network reads.
//!
//! The network was taught on pictures shrunk by the Python imaging
//! library's bilinear resize, so this is that resize to the last bit: a
//! triangle filter that widens when shrinking, weights rounded to 22-bit
//! fixed point, whole-number sums, and an 8-bit picture between the two
//! passes. The order of the two passes changes about one value in seven by
//! one level, so it follows the library's own rule as well.

use darkroom::grey_image::GreyImage;

/// The width of the picture the network reads.
pub const NETWORK_WIDTH: usize = 384;
/// The height of the picture the network reads.
pub const NETWORK_HEIGHT: usize = 768;

/// Bits after the binary point in a fixed-point weight: 32, less 8 for the
/// value, less 2 to spare.
const PRECISION_BITS: u32 = 22;

/// The picture at 384 by 768, row-major. The aspect is not kept: every
/// picture is stretched to fill. An empty picture gives black.
pub fn to_network_size(picture: &GreyImage<u8>) -> Vec<u8> {
    resize(
        picture.pixels(),
        picture.width() as usize,
        picture.height() as usize,
        NETWORK_WIDTH,
        NETWORK_HEIGHT,
    )
}

/// The resize itself, from `width` by `height` to `out_width` by
/// `out_height`.
fn resize(
    pixels: &[u8],
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<u8> {
    if width == 0 || height == 0 {
        return vec![0; out_width * out_height];
    }
    let across = width != out_width;
    let down = height != out_height;
    // The library shrinks across first unless going down first is the
    // smaller job, which for a tall film it is.
    let across_first = out_width * height <= width * out_height;
    match (across, down) {
        (false, false) => pixels.to_vec(),
        (true, false) => resize_across(pixels, width, height, out_width),
        (false, true) => resize_down(pixels, width, height, out_height),
        (true, true) if across_first => {
            let narrowed = resize_across(pixels, width, height, out_width);
            resize_down(&narrowed, out_width, height, out_height)
        }
        (true, true) => {
            let shortened = resize_down(pixels, width, height, out_height);
            resize_across(&shortened, width, out_height, out_width)
        }
    }
}

/// For one axis, each output sample's first input sample and its whole-
/// number weights.
struct Weights {
    first: usize,
    weights: Vec<i32>,
}

fn weights(in_size: usize, out_size: usize) -> Vec<Weights> {
    let scale = in_size as f64 / out_size as f64;
    let filter_scale = scale.max(1.0);
    let support = filter_scale;
    let step = 1.0 / filter_scale;
    (0..out_size)
        .map(|index| {
            let centre = (index as f64 + 0.5) * scale;
            // `as` truncates toward zero, as the library's C casts do.
            let first = ((centre - support + 0.5) as i64).max(0);
            let last = ((centre + support + 0.5) as i64).min(in_size as i64);
            let raw: Vec<f64> = (first..last)
                .map(|at| triangle((at as f64 - centre + 0.5) * step))
                .collect();
            let total: f64 = raw.iter().fold(0.0, |sum, &w| sum + w);
            let fixed = raw
                .iter()
                .map(|&w| {
                    let w = if total != 0.0 { w / total } else { w };
                    let scaled = w * f64::from(1u32 << PRECISION_BITS);
                    if w < 0.0 {
                        (-0.5 + scaled) as i32
                    } else {
                        (0.5 + scaled) as i32
                    }
                })
                .collect();
            Weights {
                first: first as usize,
                weights: fixed,
            }
        })
        .collect()
}

fn triangle(t: f64) -> f64 {
    let t = t.abs();
    if t < 1.0 { 1.0 - t } else { 0.0 }
}

/// One weighted sum, rounded and clamped to a byte.
fn sum(samples: impl Iterator<Item = u8>, weights: &[i32]) -> u8 {
    let mut total: i32 = 1 << (PRECISION_BITS - 1);
    for (&weight, sample) in weights.iter().zip(samples) {
        total += i32::from(sample) * weight;
    }
    (total >> PRECISION_BITS).clamp(0, 255) as u8
}

fn resize_across(pixels: &[u8], width: usize, height: usize, out_width: usize) -> Vec<u8> {
    let columns = weights(width, out_width);
    let mut out = Vec::with_capacity(out_width * height);
    for row in pixels.chunks_exact(width).take(height) {
        for column in &columns {
            out.push(sum(row[column.first..].iter().copied(), &column.weights));
        }
    }
    out
}

fn resize_down(pixels: &[u8], width: usize, height: usize, out_height: usize) -> Vec<u8> {
    let rows = weights(height, out_height);
    let mut out = Vec::with_capacity(width * out_height);
    for row in &rows {
        for x in 0..width {
            let column = (row.first..).map(|y| pixels[y * width + x]);
            out.push(sum(column, &row.weights));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_already_the_right_size_is_copied() {
        let pixels: Vec<u8> = (0..NETWORK_WIDTH * NETWORK_HEIGHT)
            .map(|i| (i % 251) as u8)
            .collect();
        let picture = GreyImage::new(
            NETWORK_WIDTH as u32,
            NETWORK_HEIGHT as u32,
            pixels.clone(),
            darkroom::grey_image::Placement::FILM,
        )
        .unwrap();
        assert_eq!(to_network_size(&picture), pixels);
    }

    #[test]
    fn halving_averages_neighbours_with_rounding() {
        // Shrinking 4 to 2, the first output is centred on the edge between
        // inputs 0 and 1 and its triangle, two inputs wide each side, gives
        // inputs 0, 1 and 2 the weights 3, 3 and 1 (input 3 lies beyond
        // it): (3 * 10 + 3 * 20 + 30) / 7 = 17.14, so 17. The
        // second gives inputs 1, 2 and 3 the weights 1, 3 and 3: (20 + 3 *
        // 30 + 3 * 40) / 7 = 32.86, so 33.
        assert_eq!(resize(&[10, 20, 30, 40], 4, 1, 2, 1), [17, 33]);
        assert_eq!(resize(&[10, 20, 30, 40], 1, 4, 1, 2), [17, 33]);
    }

    #[test]
    fn a_flat_picture_stays_flat_whichever_pass_goes_first() {
        let wide = vec![77u8; 900 * 100];
        assert!(resize(&wide, 900, 100, 384, 768).iter().all(|&v| v == 77));
        let tall = vec![200u8; 50 * 3000];
        assert!(resize(&tall, 50, 3000, 384, 768).iter().all(|&v| v == 200));
    }
}
