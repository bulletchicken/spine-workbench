//! A float map turned into a small 8-bit tile for a watch frame, in one of the
//! tile encodings the app decodes.
//!
//! The byte rules are the frozen ones of `docs/contracts/watch-frames.md`:
//! a unit value is `round(255 · v)`, a direction in (−π, π] is
//! `round(255 · (θ + π) / 2π) mod 256`, an axial angle in [−π/2, π/2) is
//! `round(255 · (θ + π/2) / π)`, a mask is 0 or 255, an index is the byte
//! itself, and a signed pair is two unit bytes per cell, ridge then valley.
//! Each tile carries its grid's placement, so a cell lands on the film where
//! the grid's own pixel sits.
//!
//! A map larger than [`LONGEST_SIDE`] cells on its longest side is first
//! decimated in [`decimation`] × [`decimation`] blocks, each cell placed at
//! its block's centre (see [`crate::pyramid::block_means`]). How a block
//! becomes one cell depends on what the map holds: unit values are
//! averaged, angles are averaged as unit vectors (doubled for line angles)
//! so a block straddling the wrap keeps its direction, and indices take
//! the block's most common value.

use crate::grey_image::GreyImage;
use crate::pyramid::{block_means, block_reduce};
use core::f64::consts::PI;
use shapes::watch_frames::{PreviewTile, TileEncoding};

/// The longest side of a preview tile, in cells.
pub const LONGEST_SIDE: u32 = 1200;

/// The block size, in pixels, that brings a `width` × `height` map down to at
/// most [`LONGEST_SIDE`] cells on its longest side: `⌈max(width, height) / 1200⌉`,
/// and at least 1.
pub fn decimation(width: u32, height: u32) -> u32 {
    width.max(height).div_ceil(LONGEST_SIDE).max(1)
}

/// A unit map (or either half of a signed pair) decimated to its preview
/// size, by block means. Not for angles or indices, which have their own.
pub fn decimated(image: &GreyImage<f32>) -> Option<GreyImage<f32>> {
    block_means(image, decimation(image.width(), image.height()))
}

/// A direction map in (−π, π] decimated to its preview size: each block's
/// angle is that of the mean of its unit vectors, back in (−π, π].
pub fn decimated_direction(image: &GreyImage<f32>) -> Option<GreyImage<f32>> {
    mean_angles(image, decimation(image.width(), image.height()), 1.0)
}

/// A line-angle map in [−π/2, π/2) decimated to its preview size: each
/// block's angle is half that of the mean of its doubled-angle unit vectors,
/// since θ and θ + π are the same line, back in [−π/2, π/2).
pub fn decimated_axial(image: &GreyImage<f32>) -> Option<GreyImage<f32>> {
    mean_angles(image, decimation(image.width(), image.height()), 2.0)
}

/// An index map decimated to its preview size: each block takes its most
/// common index, the smaller on a tie.
pub fn decimated_index(image: &GreyImage<u8>) -> Option<GreyImage<u8>> {
    most_common(image, decimation(image.width(), image.height()))
}

/// Block means of angles as vectors: `turns` is 1 for directions and 2 for
/// line angles. The result is folded into (−π, π] for directions and
/// [−π/2, π/2) for line angles. A block whose vectors cancel exactly
/// reads 0.
fn mean_angles(image: &GreyImage<f32>, block: u32, turns: f64) -> Option<GreyImage<f32>> {
    block_reduce(image, block, |angles| {
        let (mut across, mut up) = (0.0f64, 0.0f64);
        for &theta in angles {
            let turned = turns * f64::from(theta);
            across += libm::cos(turned);
            up += libm::sin(turned);
        }
        let mean = libm::atan2(up, across) / turns;
        let folded = if turns == 1.0 {
            if mean <= -PI { mean + 2.0 * PI } else { mean }
        } else if mean >= PI / 2.0 {
            mean - PI
        } else {
            mean
        };
        folded as f32
    })
}

/// The most common value of each block, the smaller on a tie.
fn most_common(image: &GreyImage<u8>, block: u32) -> Option<GreyImage<u8>> {
    block_reduce(image, block, |values| {
        let mut counts = [0u32; 256];
        for &value in values {
            counts[usize::from(value)] += 1;
        }
        let mut best = 0u8;
        for value in 1..=255u8 {
            if counts[usize::from(value)] > counts[usize::from(best)] {
                best = value;
            }
        }
        best
    })
}

/// A unit value (0 to 1, clamped): `round(255 · v)`.
pub fn unit_tile(image: &GreyImage<f32>) -> PreviewTile {
    tile(
        image,
        TileEncoding::Unit,
        image
            .pixels()
            .iter()
            .map(|&v| unit_byte(f64::from(v)))
            .collect(),
    )
}

/// A direction in (−π, π]: `round(255 · (θ + π) / 2π) mod 256`.
pub fn direction_tile(image: &GreyImage<f32>) -> PreviewTile {
    tile(
        image,
        TileEncoding::Direction,
        image
            .pixels()
            .iter()
            .map(|&theta| direction_byte(f64::from(theta)))
            .collect(),
    )
}

/// A line angle in [−π/2, π/2): `round(255 · (θ + π/2) / π)`.
pub fn axial_tile(image: &GreyImage<f32>) -> PreviewTile {
    tile(
        image,
        TileEncoding::Axial,
        image
            .pixels()
            .iter()
            .map(|&theta| axial_byte(f64::from(theta)))
            .collect(),
    )
}

/// A mask: 255 where it is set and 0 where it is not.
pub fn mask_tile(image: &GreyImage<bool>) -> PreviewTile {
    tile(
        image,
        TileEncoding::Mask,
        image
            .pixels()
            .iter()
            .map(|&on| if on { 255 } else { 0 })
            .collect(),
    )
}

/// An index into a list, one byte per cell, stored as it is.
pub fn index_tile(image: &GreyImage<u8>) -> PreviewTile {
    tile(image, TileEncoding::Index, image.pixels().to_vec())
}

/// A ridge map and a valley map on the same grid, two bytes per cell: the
/// ridge's unit byte, then the valley's. `None` when the two grids differ in
/// size or placement.
pub fn signed_pair_tile(ridge: &GreyImage<f32>, valley: &GreyImage<f32>) -> Option<PreviewTile> {
    if ridge.width() != valley.width()
        || ridge.height() != valley.height()
        || ridge.placement() != valley.placement()
    {
        return None;
    }
    let data = ridge
        .pixels()
        .iter()
        .zip(valley.pixels())
        .flat_map(|(&r, &v)| [unit_byte(f64::from(r)), unit_byte(f64::from(v))])
        .collect();
    Some(tile(ridge, TileEncoding::SignedPair, data))
}

fn unit_byte(value: f64) -> u8 {
    (255.0 * value.clamp(0.0, 1.0)).round() as u8
}

fn direction_byte(theta: f64) -> u8 {
    let scaled = (255.0 * (theta + PI) / (2.0 * PI)).round() as i64;
    scaled.rem_euclid(256) as u8
}

fn axial_byte(theta: f64) -> u8 {
    (255.0 * (theta + PI / 2.0) / PI).round().clamp(0.0, 255.0) as u8
}

/// The tile for `image`'s grid with the given encoding and bytes.
fn tile<T>(image: &GreyImage<T>, encoding: TileEncoding, data: Vec<u8>) -> PreviewTile {
    let placement = image.placement();
    PreviewTile {
        width: image.width(),
        height: image.height(),
        origin: placement.origin,
        step_across: placement.step_across,
        step_down: placement.step_down,
        encoding,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;
    use shapes::pixels::Px;

    fn row<T>(values: Vec<T>) -> GreyImage<T> {
        let width = values.len() as u32;
        GreyImage::new(width, 1, values, Placement::FILM).expect("one row")
    }

    #[test]
    fn unit_values_are_scaled_and_clamped() {
        // 0.25 · 255 = 63.75 and 0.75 · 255 = 191.25; values past the ends clamp.
        let tile = unit_tile(&row(vec![0.0f32, 0.25, 0.75, 1.0, 2.0, -1.0]));
        assert_eq!(tile.data, vec![0, 64, 191, 255, 255, 0]);
        assert_eq!(tile.encoding, TileEncoding::Unit);
    }

    #[test]
    fn directions_wrap_into_one_byte() {
        // θ = π/4 is 255 · 5/8 = 159.375; θ = ±π/2 are 191.25 and 63.75. θ = π
        // is 255, and −π would be 0 (the range excludes it).
        let theta = [
            PI as f32 / 4.0,
            PI as f32 / 2.0,
            -(PI as f32) / 2.0,
            PI as f32,
        ];
        let tile = direction_tile(&row(theta.to_vec()));
        assert_eq!(tile.data, vec![159, 191, 64, 255]);
    }

    #[test]
    fn axial_angles_run_from_zero_to_255() {
        // −π/2 is 0, −π/4 is 255 · 1/4 = 63.75 and π/4 is 255 · 3/4 = 191.25.
        let theta = [-(PI as f32) / 2.0, -(PI as f32) / 4.0, PI as f32 / 4.0];
        let tile = axial_tile(&row(theta.to_vec()));
        assert_eq!(tile.data, vec![0, 64, 191]);
    }

    #[test]
    fn masks_are_black_or_white() {
        let tile = mask_tile(&row(vec![true, false, true]));
        assert_eq!(tile.data, vec![255, 0, 255]);
        assert_eq!(tile.encoding, TileEncoding::Mask);
    }

    #[test]
    fn indices_are_stored_as_they_are() {
        let tile = index_tile(&row(vec![0u8, 3, 255]));
        assert_eq!(tile.data, vec![0, 3, 255]);
        assert_eq!(tile.encoding, TileEncoding::Index);
    }

    #[test]
    fn a_signed_pair_is_ridge_then_valley_per_cell() {
        let ridge = row(vec![1.0f32, 0.0]);
        let valley = row(vec![0.0f32, 1.0]);
        let tile = signed_pair_tile(&ridge, &valley).expect("the grids match");
        assert_eq!(tile.data, vec![255, 0, 0, 255]);
        assert_eq!(tile.encoding, TileEncoding::SignedPair);
        assert!(signed_pair_tile(&ridge, &row(vec![0.0f32])).is_none());
    }

    #[test]
    fn a_tile_carries_the_placement_of_its_grid() {
        let half = Placement {
            origin: Px::new(0.5, 0.5),
            step_across: Px::new(2.0, 0.0),
            step_down: Px::new(0.0, 2.0),
        };
        let grid = GreyImage::new(2, 1, vec![0.0f32, 1.0], half).expect("sizes match");
        let tile = unit_tile(&grid);
        assert_eq!((tile.width, tile.height), (2, 1));
        assert_eq!(tile.origin, Px::new(0.5, 0.5));
        assert_eq!(tile.step_across, Px::new(2.0, 0.0));
        assert_eq!(tile.step_down, Px::new(0.0, 2.0));
    }

    #[test]
    fn maps_longer_than_1200_cells_are_decimated() {
        assert_eq!(decimation(1200, 1), 1);
        assert_eq!(decimation(1201, 1), 2);
        assert_eq!(decimation(3000, 8000), 7);
        assert_eq!(decimation(0, 0), 1);
        let long = GreyImage::new(4, 2, vec![0.0f32; 8], Placement::FILM).expect("sizes match");
        assert_eq!(decimated(&long).map(|g| g.width()), Some(4));
    }

    #[test]
    fn directions_across_the_wrap_keep_their_direction() {
        // π − 0.01 and −π + 0.03 both point almost along −x; their unit
        // vectors' mean points half way between them, at π + 0.01, which is
        // −π + 0.01 in (−π, π]. Averaging the numbers would give 0.01, the
        // opposite direction (byte 128). The answer is stored in f32, within
        // 2.4e-7 of the true angle, so 1e-6. Its byte is
        // round(255 · 0.01 / 2π) = 0.
        let pair = row(vec![(PI - 0.01) as f32, (-PI + 0.03) as f32]);
        let mean = mean_angles(&pair, 2, 1.0).expect("block is valid");
        let angle = f64::from(mean.pixels()[0]);
        assert!((angle - (-PI + 0.01)).abs() < 1e-6, "angle {angle}");
        assert_eq!(direction_tile(&mean).data, vec![0]);
        // The block's cell sits at the block centre, as a block mean's does.
        assert_eq!(mean.film_point(0.0, 0.0), Px::new(0.5, 0.5));
    }

    #[test]
    fn line_angles_across_the_wrap_keep_their_line() {
        // π/2 − 0.01 and −π/2 + 0.03 are both nearly vertical lines. Doubled,
        // they are π − 0.02 and −π + 0.06, whose mean is π + 0.02, that is
        // −π + 0.02; halved, −π/2 + 0.01. Averaging the numbers would give
        // 0.01, a horizontal line. Tolerance as for directions.
        let pair = row(vec![(PI / 2.0 - 0.01) as f32, (-PI / 2.0 + 0.03) as f32]);
        let mean = mean_angles(&pair, 2, 2.0).expect("block is valid");
        let angle = f64::from(mean.pixels()[0]);
        assert!((angle - (-PI / 2.0 + 0.01)).abs() < 1e-6, "angle {angle}");
        // π/2 − 0.01 and −π/2 + 0.01 are mirror images about vertical: their
        // doubled vectors cancel up and down and point along −x, angle π,
        // which halves to π/2 and folds to −π/2, the range's bottom end.
        let mirrored = row(vec![(PI / 2.0 - 0.01) as f32, (-PI / 2.0 + 0.01) as f32]);
        let folded = f64::from(mean_angles(&mirrored, 2, 2.0).expect("valid").pixels()[0]);
        assert!((folded + PI / 2.0).abs() < 1e-6, "angle {folded}");
    }

    #[test]
    fn an_index_block_takes_its_most_common_value() {
        let indices = row(vec![5u8, 2, 2, 9, 3, 3, 7, 7]);
        let decimated = most_common(&indices, 4).expect("block is valid");
        // 2 is the most common of 5, 2, 2, 9; 3 and 7 tie, and 3 is smaller.
        assert_eq!(decimated.pixels(), &[2, 3]);
        assert_eq!(index_tile(&decimated).data, vec![2, 3]);
    }
}
