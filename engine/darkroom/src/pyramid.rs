//! Working-resolution copies of a film, each with an exact map back to
//! original film pixels, so nothing found on a smaller copy is ever reported
//! in its pixels.
//!
//! A copy shrunk by a factor `f` along an axis has pixel `i` covering the
//! native footprint `[i·f, (i + 1)·f)` in edge coordinates, that is, the
//! native centres from `i·f − 1/2` to `(i + 1)·f − 1/2`. Its value is the
//! mean of the native pixels under that footprint, with the pixels at the
//! two ends weighted by how much of them lies inside. Its centre is at
//! native position `(i + 1/2)·f − 1/2`, and the placement records exactly
//! that, so the map back is a single affine step rather than an
//! approximation. The two axes may have different factors, so a film with
//! pixels taller than wide gets a working copy with square pixels.
//!
//! The working factor follows the evidence-map spec's scale rule: the copy's
//! spacing is the largest of the native spacing, a finest spacing, and the
//! spacing that keeps the copy within a pixel budget.

use crate::grey_image::{GreyImage, Placement};
use shapes::pixels::Px;

/// The factor `f ≥ 1` of a working copy of a `width` × `height` film at
/// `native_mm` millimetres a pixel: the working spacing is `f · native_mm`,
/// the largest of `native_mm`, `finest_mm` and the spacing at which the copy
/// has at most `max_pixels` pixels. The evidence-map spec uses a finest
/// spacing of 0.35 mm and a budget of 2.4 million pixels.
pub fn working_factor(
    native_mm: f64,
    finest_mm: f64,
    width: u32,
    height: u32,
    max_pixels: f64,
) -> f64 {
    let budget = (f64::from(width) * f64::from(height) / max_pixels).sqrt();
    1.0f64.max(finest_mm / native_mm).max(budget)
}

/// The area shrink of `image` by `factor_across` along its rows and
/// `factor_down` down its columns. The copy has `⌊width / factor_across⌋`
/// by `⌊height / factor_down⌋` pixels, each the area average of its native
/// footprint, in f32. Any grid whose values widen to f64 can be shrunk, so a
/// decoded 16-bit film needs no float copy first. Native rows are read in
/// order and each is averaged across once; only that row and the copy row
/// being summed (both in f64) are held, besides the copy itself. `None`
/// when a factor is below 1 or not a number.
pub fn shrink<T: Copy + Into<f64>>(
    image: &GreyImage<T>,
    factor_across: f64,
    factor_down: f64,
) -> Option<GreyImage<f32>> {
    let is_shrink = |factor: f64| factor.is_finite() && factor >= 1.0;
    if !is_shrink(factor_across) || !is_shrink(factor_down) {
        return None;
    }
    let (width, height) = (image.width() as usize, image.height() as usize);
    let out_width = (width as f64 / factor_across).floor() as usize;
    let out_height = (height as f64 / factor_down).floor() as usize;
    let columns = footprint_weights(width, factor_across, out_width);
    let rows = footprint_weights(height, factor_down, out_height);

    let average_across = |y: usize| -> Vec<f64> {
        let native_row = &image.pixels()[y * width..][..width];
        columns
            .iter()
            .map(|weights| weights.iter().map(|&(j, w)| w * native_row[j].into()).sum())
            .collect()
    };

    let mut pixels = Vec::with_capacity(out_width * out_height);
    let mut across: Option<(usize, Vec<f64>)> = None;
    for weights in &rows {
        let mut sums = vec![0.0f64; out_width];
        for &(y, w) in weights {
            let row = match across.take() {
                Some((kept, row)) if kept == y => row,
                _ => average_across(y),
            };
            for (sum, value) in sums.iter_mut().zip(&row) {
                *sum += w * value;
            }
            across = Some((y, row));
        }
        pixels.extend(sums.into_iter().map(|v| v as f32));
    }

    let source = image.placement();
    let placement = Placement {
        origin: source.film_point(0.5 * factor_across - 0.5, 0.5 * factor_down - 0.5),
        step_across: scaled(source.step_across, factor_across),
        step_down: scaled(source.step_down, factor_down),
    };
    GreyImage::new(out_width as u32, out_height as u32, pixels, placement)
}

/// The means of `block` × `block` squares of the image, the copy at a whole
/// factor. A block at the right or bottom edge may hold fewer pixels, and is
/// then averaged over the pixels it holds. Each copy pixel is placed at the
/// centre of a full block, so the map back is exact for every full block; a
/// partial block sits up to `(block − 1) / 2` native pixels from its own
/// centre, which the caller must allow for. `None` when `block` is 0.
pub fn block_means(image: &GreyImage<f32>, block: u32) -> Option<GreyImage<f32>> {
    block_reduce(image, block, |values| {
        let sum: f64 = values.iter().map(|&v| f64::from(v)).sum();
        (sum / values.len() as f64) as f32
    })
}

/// Each `block` × `block` square of the image reduced to one value by
/// `reduce`, which receives the square's pixels in reading order (fewer at
/// the right and bottom edges, never none). The copy is placed as
/// [`block_means`] places it. `None` when `block` is 0.
pub fn block_reduce<T: Copy, U>(
    image: &GreyImage<T>,
    block: u32,
    mut reduce: impl FnMut(&[T]) -> U,
) -> Option<GreyImage<U>> {
    if block == 0 {
        return None;
    }
    let (width, height) = (image.width(), image.height());
    let out_width = width.div_ceil(block);
    let out_height = height.div_ceil(block);
    let mut pixels = Vec::with_capacity(out_width as usize * out_height as usize);
    let mut square = Vec::with_capacity(block as usize * block as usize);
    for by in 0..out_height {
        for bx in 0..out_width {
            square.clear();
            for y in (by * block)..((by + 1) * block).min(height) {
                for x in (bx * block)..((bx + 1) * block).min(width) {
                    square.push(*image.get(x, y).expect("inside the image"));
                }
            }
            pixels.push(reduce(&square));
        }
    }
    let source = image.placement();
    let centre = f64::from(block - 1) / 2.0;
    let factor = f64::from(block);
    let placement = Placement {
        origin: source.film_point(centre, centre),
        step_across: scaled(source.step_across, factor),
        step_down: scaled(source.step_down, factor),
    };
    GreyImage::new(out_width, out_height, pixels, placement)
}

/// The native pixels under each copy pixel's footprint, with the share of each
/// pixel that lies inside it. Each share list sums to 1 when the footprints
/// lie inside the native image.
fn footprint_weights(native_len: usize, factor: f64, out_len: usize) -> Vec<Vec<(usize, f64)>> {
    (0..out_len)
        .map(|i| {
            let start = i as f64 * factor;
            let end = (i + 1) as f64 * factor;
            let first = start.floor() as usize;
            let last = (end.ceil() as usize).min(native_len);
            (first..last)
                .filter_map(|j| {
                    let overlap = end.min((j + 1) as f64) - start.max(j as f64);
                    (overlap > 0.0).then_some((j, overlap / factor))
                })
                .collect()
        })
        .collect()
}

fn scaled(step: Px, factor: f64) -> Px {
    Px::new(step.x() * factor, step.y() * factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(width: u32, height: u32, value: impl Fn(u32, u32) -> f32) -> GreyImage<f32> {
        let mut pixels = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push(value(x, y));
            }
        }
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    #[test]
    fn the_working_factor_gives_the_spec_grids() {
        // Adult film, 3000 × 8000 at 0.15 mm: f = √10 = 3.1623, a 948 × 2529 grid,
        // working spacing 0.4743 mm.
        let budget = 2.4e6;
        // 3000 · 8000 / 2.4e6 is exactly 10 in f64 (every operand is a whole
        // number held exactly), so the factor is the correctly rounded √10
        // and equals 10f64.sqrt() exactly. The spacing √10 · 0.15 =
        // 0.4743416... is checked against its five-place rounding, 0.47434,
        // so to 1e-5: twice the most that rounding can move it.
        let adult = working_factor(0.15, 0.35, 3000, 8000, budget);
        assert_eq!(adult, 10f64.sqrt());
        assert_eq!((3000f64 / adult).floor() as u32, 948);
        assert_eq!((8000f64 / adult).floor() as u32, 2529);
        assert!((adult * 0.15 - 0.47434).abs() < 1e-5);
        // Child, 1500 × 3600 at 0.15 mm: the finest spacing 0.35 mm wins,
        // f = 7/3, a 642 × 1542 grid. Neither 0.35 nor 0.15 is exact in
        // f64, so 0.35 / 0.15 carries three roundings, under 1e-15 in all;
        // 1e-12 sits far above that and far below the budget's 1.5.
        let child = working_factor(0.15, 0.35, 1500, 3600, budget);
        assert!((child - 7.0 / 3.0).abs() < 1e-12);
        assert_eq!((1500f64 / child).floor() as u32, 642);
        assert_eq!((3600f64 / child).floor() as u32, 1542);
    }

    #[test]
    fn a_constant_film_stays_constant_at_any_factor() {
        // The shares (0.4 and 0.2 of a pixel at factor 2.5) are not exact in
        // binary, so the f64 mean of 7s is off by a few f64 ulps; storing it
        // in f32 rounds by at most half an ulp of 7 (4.8e-7). So 1e-6.
        let flat = grid(10, 10, |_, _| 7.0);
        let copy = shrink(&flat, 2.5, 2.5).expect("factor is valid");
        assert_eq!((copy.width(), copy.height()), (4, 4));
        for value in copy.pixels() {
            assert!((value - 7.0).abs() < 1e-6, "value {value}");
        }
    }

    #[test]
    fn the_shrink_keeps_the_total_when_the_footprints_fill_the_film() {
        // 10 pixels at factor 2.5 make exactly 4 copy pixels, so every native
        // pixel is shared out and the total is the same before and after.
        // Each copy pixel covers f × f native pixels, so the area-weighted total
        // (copy value × f²) equals the native total. Each of the 16 copy values
        // is below 10 and stored in f32 (within 6e-7), and is multiplied by
        // 6.25, so the totals agree to 16 · 6.25 · 6e-7 = 6e-5; 1e-4.
        let factor = 2.5;
        let pattern = grid(10, 10, |x, y| ((x * 7 + y * 3) % 11) as f32);
        let copy = shrink(&pattern, factor, factor).expect("factor is valid");
        let before: f64 = pattern.pixels().iter().map(|&v| f64::from(v)).sum();
        let after: f64 = copy.pixels().iter().map(|&v| f64::from(v)).sum::<f64>() * factor * factor;
        assert!((before - after).abs() < 1e-4, "{before} against {after}");
    }

    #[test]
    fn a_ramp_shrunk_by_a_whole_factor_maps_back_exactly() {
        // Native value = column index. A factor of 2 averages columns (2i, 2i+1)
        // to 2i + 1/2, which is exactly the film position of copy pixel i.
        let ramp = grid(8, 2, |x, _| x as f32);
        let copy = shrink(&ramp, 2.0, 2.0).expect("factor is valid");
        assert_eq!((copy.width(), copy.height()), (4, 1));
        for i in 0..4u32 {
            let value = f64::from(copy.get(i, 0).copied().expect("inside"));
            let back = copy.film_point(f64::from(i), 0.0).x();
            assert_eq!(value, back, "copy pixel {i}");
        }
    }

    #[test]
    fn a_crop_shrinks_onto_its_own_film_points() {
        // A crop starting at film column 2 keeps its film placement. The first
        // copy pixel averages film columns 2 and 3, so its centre is at film x = 2.5.
        let wide = grid(8, 2, |x, _| x as f32);
        let crop = wide.crop(2, 0, 4, 2).expect("the crop lies inside");
        let copy = shrink(&crop, 2.0, 2.0).expect("factor is valid");
        assert_eq!(copy.film_point(0.0, 0.0).x(), 2.5);
        assert_eq!(copy.pixels(), &[2.5, 4.5]);
    }

    #[test]
    fn each_axis_shrinks_by_its_own_factor_and_maps_back_exactly() {
        // A 16-bit film with value x + 10y, shrunk by 2 across and 3 down.
        // Copy pixel (i, k) averages columns 2i, 2i + 1 and rows 3k to 3k + 2,
        // so its value is (2i + 1/2) + 10(3k + 1), and its centre is at film
        // (2i + 1/2, 3k + 1): value = x + 10y at its own film point. All of
        // it is exact in binary.
        let film = GreyImage::new(
            8,
            9,
            (0..9u16)
                .flat_map(|y| (0..8u16).map(move |x| x + 10 * y))
                .collect(),
            Placement::FILM,
        )
        .expect("sizes match");
        let copy = shrink(&film, 2.0, 3.0).expect("factors are valid");
        assert_eq!((copy.width(), copy.height()), (4, 3));
        for k in 0..3u32 {
            for i in 0..4u32 {
                let point = copy.film_point(f64::from(i), f64::from(k));
                let value = f64::from(*copy.get(i, k).expect("inside"));
                assert_eq!(
                    point,
                    Px::new(2.0 * f64::from(i) + 0.5, 3.0 * f64::from(k) + 1.0)
                );
                assert_eq!(value, point.x() + 10.0 * point.y(), "copy pixel {i},{k}");
            }
        }
    }

    #[test]
    fn block_means_place_each_block_at_its_centre() {
        // 4 × 2 with blocks of 2 × 2: the means are 2.5 and 4.5, and copy pixel
        // i sits at film x = 0.5 + 2i, the centre of its block.
        let image = grid(4, 2, |x, y| (y * 4 + x) as f32);
        let copy = block_means(&image, 2).expect("block is valid");
        assert_eq!(copy.pixels(), &[2.5, 4.5]);
        assert_eq!(copy.film_point(1.0, 0.0), Px::new(2.5, 0.5));
    }

    #[test]
    fn a_partial_block_is_averaged_over_the_pixels_it_holds() {
        let image = grid(3, 1, |x, _| x as f32);
        let copy = block_means(&image, 2).expect("block is valid");
        assert_eq!((copy.width(), copy.height()), (2, 1));
        assert_eq!(copy.pixels(), &[0.5, 2.0]);
    }

    #[test]
    fn invalid_factors_give_no_copy() {
        let image = grid(4, 4, |_, _| 1.0);
        assert!(shrink(&image, 0.0, 1.0).is_none());
        assert!(shrink(&image, f64::NAN, 1.0).is_none());
        assert!(shrink(&image, 2.0, f64::INFINITY).is_none());
        // Below 1 would enlarge, which is not what a shrink is for, and a tiny
        // factor would ask for an enormous copy.
        assert!(shrink(&image, 1e-12, 1e-12).is_none());
        assert!(shrink(&image, 2.0, 0.5).is_none());
        assert!(block_means(&image, 0).is_none());
    }
}
