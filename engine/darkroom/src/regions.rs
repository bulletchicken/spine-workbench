//! Flood fill, connected parts and the outline of a region.
//!
//! A region is a mask: a grid of `bool` on the film's own pixels. Two pixels
//! are joined by four neighbours (up, down, left, right) or, when asked, by
//! eight (the diagonals too). Every search is breadth-first from the lowest
//! index, so the same mask always gives the same labels. Nothing is read
//! past the grid's edge: the edge counts as outside the region.

use crate::grey_image::GreyImage;
use std::collections::VecDeque;

/// Connected parts of a mask: pixel labels from 1 for each part, in the order
/// the parts' first pixels appear in reading order, and 0 outside the mask.
#[derive(Clone, Debug, PartialEq)]
pub struct Components {
    pub labels: GreyImage<u32>,
    pub count: u32,
}

/// The pixels reached from `seed` through pixels of `mask`. Empty when the seed
/// is outside the grid or not in the mask.
pub fn flood_fill(
    mask: &GreyImage<bool>,
    seed: (u32, u32),
    eight_connected: bool,
) -> GreyImage<bool> {
    let mut reached = vec![false; mask.pixels().len()];
    let mut seen = vec![false; mask.pixels().len()];
    if mask.get(seed.0, seed.1) == Some(&true) {
        walk(mask, seed, eight_connected, &mut seen, |index| {
            reached[index] = true
        });
    }
    like(mask, reached)
}

/// Labels every connected part of `mask`.
pub fn label_components(mask: &GreyImage<bool>, eight_connected: bool) -> Components {
    let (width, height) = (mask.width(), mask.height());
    let mut labels = vec![0u32; mask.pixels().len()];
    let mut seen = vec![false; mask.pixels().len()];
    let mut count = 0u32;
    for y in 0..height {
        for x in 0..width {
            let index = y as usize * width as usize + x as usize;
            if mask.pixels()[index] && !seen[index] {
                count += 1;
                let label = count;
                walk(mask, (x, y), eight_connected, &mut seen, |found| {
                    labels[found] = label;
                });
            }
        }
    }
    Components {
        labels: like(mask, labels),
        count,
    }
}

/// The pixels of `mask` that have a neighbour (up, down, left or right) outside
/// the mask. The grid's edge counts as outside, so a region touching the edge
/// is outlined there too.
pub fn outline(mask: &GreyImage<bool>) -> GreyImage<bool> {
    let (width, height) = (mask.width(), mask.height());
    let inside = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(width)
            && y < i64::from(height)
            && mask.get(x as u32, y as u32) == Some(&true)
    };
    let mut edge = Vec::with_capacity(mask.pixels().len());
    for y in 0..height {
        for x in 0..width {
            let (x, y) = (i64::from(x), i64::from(y));
            let on_mask = inside(x, y);
            let touches_outside = on_mask
                && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .iter()
                    .any(|&(nx, ny)| !inside(nx, ny));
            edge.push(touches_outside);
        }
    }
    like(mask, edge)
}

/// Visits every pixel of the part of `mask` joined to `start`, calling `found`
/// with each pixel's index once. `seen` marks the pixels already visited, so
/// several walks can share it.
fn walk(
    mask: &GreyImage<bool>,
    start: (u32, u32),
    eight_connected: bool,
    seen: &mut [bool],
    mut found: impl FnMut(usize),
) {
    let (width, height) = (mask.width(), mask.height());
    let index_of = |x: u32, y: u32| y as usize * width as usize + x as usize;
    let mut queue = VecDeque::new();
    seen[index_of(start.0, start.1)] = true;
    queue.push_back(start);
    while let Some((x, y)) = queue.pop_front() {
        found(index_of(x, y));
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let diagonal = dx != 0 && dy != 0;
                if (dx, dy) == (0, 0) || (diagonal && !eight_connected) {
                    continue;
                }
                let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                if nx < 0 || ny < 0 || nx >= i64::from(width) || ny >= i64::from(height) {
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let next = index_of(nx, ny);
                if mask.pixels()[next] && !seen[next] {
                    seen[next] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
}

/// A grid of `values` on the placement and size of `template`.
fn like<U>(template: &GreyImage<bool>, values: Vec<U>) -> GreyImage<U> {
    GreyImage::new(
        template.width(),
        template.height(),
        values,
        template.placement(),
    )
    .expect("one value per pixel of the template")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grey_image::Placement;

    /// A mask of the given size with the listed pixels set.
    fn mask(width: u32, height: u32, on: &[(u32, u32)]) -> GreyImage<bool> {
        let mut pixels = vec![false; width as usize * height as usize];
        for &(x, y) in on {
            pixels[y as usize * width as usize + x as usize] = true;
        }
        GreyImage::new(width, height, pixels, Placement::FILM).expect("sizes match")
    }

    fn count(image: &GreyImage<bool>) -> usize {
        image.pixels().iter().filter(|&&on| on).count()
    }

    fn square(x0: u32, y0: u32, side: u32) -> Vec<(u32, u32)> {
        (y0..y0 + side)
            .flat_map(|y| (x0..x0 + side).map(move |x| (x, y)))
            .collect()
    }

    #[test]
    fn a_flood_fill_takes_the_whole_connected_square() {
        let block = mask(5, 5, &square(1, 1, 3));
        assert_eq!(count(&flood_fill(&block, (1, 1), false)), 9);
        assert_eq!(count(&flood_fill(&block, (2, 2), true)), 9);
        // A seed outside the mask fills nothing.
        assert_eq!(count(&flood_fill(&block, (0, 0), false)), 0);
    }

    #[test]
    fn a_flood_fill_stops_at_a_gap() {
        // Every pixel except column 2 of a 5 × 5 grid: the seed on the left reaches
        // columns 0 and 1 only, 10 pixels.
        let on: Vec<(u32, u32)> = (0..5)
            .flat_map(|y| (0..5).filter(|&x| x != 2).map(move |x| (x, y)))
            .collect();
        let split = mask(5, 5, &on);
        let left = flood_fill(&split, (0, 0), true);
        assert_eq!(count(&left), 10);
        assert_eq!(left.get(3, 0), Some(&false));
    }

    #[test]
    fn diagonal_pixels_join_only_with_eight_connection() {
        let diagonal = mask(3, 3, &[(0, 0), (1, 1)]);
        assert_eq!(label_components(&diagonal, false).count, 2);
        assert_eq!(label_components(&diagonal, true).count, 1);
        assert_eq!(count(&flood_fill(&diagonal, (0, 0), false)), 1);
        assert_eq!(count(&flood_fill(&diagonal, (0, 0), true)), 2);
    }

    #[test]
    fn components_are_labelled_in_reading_order() {
        // A 2 × 2 block at the top left and a lone pixel at the bottom right.
        let two = mask(5, 5, &[(0, 0), (1, 0), (0, 1), (1, 1), (4, 4)]);
        let parts = label_components(&two, false);
        assert_eq!(parts.count, 2);
        assert_eq!(parts.labels.get(0, 0), Some(&1));
        assert_eq!(parts.labels.get(4, 4), Some(&2));
        assert_eq!(parts.labels.get(2, 2), Some(&0));
        let sizes: Vec<usize> = (1..=2)
            .map(|label| {
                parts
                    .labels
                    .pixels()
                    .iter()
                    .filter(|&&l| l == label)
                    .count()
            })
            .collect();
        assert_eq!(sizes, vec![4, 1]);
    }

    #[test]
    fn the_outline_of_a_square_is_its_ring() {
        // A 3 × 3 square in a 5 × 5 grid: its outline is the 8 border pixels, and
        // the centre is not on it.
        let block = mask(5, 5, &square(1, 1, 3));
        let edge = outline(&block);
        assert_eq!(count(&edge), 8);
        assert_eq!(edge.get(2, 2), Some(&false));
        assert_eq!(edge.get(1, 1), Some(&true));
    }

    #[test]
    fn a_pixel_open_only_at_a_corner_is_not_on_the_outline() {
        // A 4 × 4 grid set everywhere except (0, 0). Pixel (1, 1) has the gap
        // only on its diagonal; its four neighbours are all set, so it is
        // inside. Pixels (1, 0) and (0, 1) touch the gap directly.
        let on: Vec<(u32, u32)> = square(0, 0, 4)
            .into_iter()
            .filter(|&p| p != (0, 0))
            .collect();
        let edge = outline(&mask(4, 4, &on));
        assert_eq!(edge.get(1, 1), Some(&false));
        assert_eq!(edge.get(1, 0), Some(&true));
        assert_eq!(edge.get(0, 1), Some(&true));
    }

    #[test]
    fn a_region_on_the_grid_edge_is_outlined_at_the_edge() {
        // A full 2 × 2 grid is all outline: every pixel touches the outside.
        let full = mask(2, 2, &square(0, 0, 2));
        assert_eq!(count(&outline(&full)), 4);
    }
}
