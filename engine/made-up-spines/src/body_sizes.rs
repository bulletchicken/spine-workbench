//! How big each vertebra, disc and pedicle is.
//!
//! The base table is an adult spine, T1 to L5, in millimetres:
//!
//! - central body heights and disc heights from Busscher et al.,
//!   "Comparative anatomical dimensions of the complete human and porcine
//!   spine", Eur Spine J 2010, Table 3, human columns. The
//!   paper does not say which disc each row is; it is taken as the disc
//!   below the level, so the last row is L5–S1, which fits the jump to a
//!   lumbar-sized disc under T12;
//! - endplate widths and depths rising evenly across the 27.7–54.0 mm and
//!   16.5–36.7 mm ranges Busscher reports; the lower endplate is 4% larger
//!   than the upper. Made-up values in a published range, not
//!   measurements;
//! - pedicle width and height, made up in the usual adult proportions.
//!
//! The table is taken as a spine 450 mm from the top of T1 to the top of
//! S1, and every length is multiplied by `k = L / 450` for a spine of
//! length `L` before anything is stacked, so a large curve shortens the
//! spine rather than being enlarged to a height. C7 and the cervical
//! vertebrae above it are T1 scaled by 0.95 for each level up. A thirteenth
//! thoracic vertebra copies T12 and a sixth lumbar copies L5; the last
//! lumbar vertebra always sits on the L5–S1 disc.

use crate::seeded::Stream;
use shapes::levels::{Level, Region};

/// The length the base table describes, from the top of T1 to the top of
/// S1, in mm.
pub const TABLE_LENGTH_MM: f64 = 450.0;

/// Central body heights, T1 to L5.
const HEIGHT: [f64; 17] = [
    16.1, 16.7, 17.5, 17.9, 17.6, 18.1, 19.3, 19.8, 21.3, 22.2, 24.2, 25.5, 24.5, 24.7, 25.5, 24.1,
    25.3,
];

/// The disc below each level, T1–T2 to L5–S1.
const DISC_BELOW: [f64; 17] = [
    5.0, 4.5, 4.7, 4.7, 4.8, 5.3, 5.2, 4.9, 4.8, 5.9, 6.1, 9.3, 10.3, 11.5, 11.8, 12.7, 8.8,
];

/// Upper endplate full width.
const TOP_WIDTH: [f64; 17] = [
    27.7, 27.7, 27.7, 28.8, 30.0, 31.1, 32.3, 33.4, 34.6, 36.8, 40.3, 42.6, 44.9, 47.1, 49.4, 51.7,
    54.0,
];

/// Upper endplate full depth.
const TOP_DEPTH: [f64; 17] = [
    16.5, 17.6, 18.6, 19.7, 21.8, 23.9, 26.1, 27.1, 28.2, 29.3, 30.3, 31.4, 32.4, 34.6, 35.6, 36.7,
    36.7,
];

/// The lower endplate is this much larger than the upper, both ways.
const BOTTOM_SCALE: f64 = 1.04;

/// Each cervical level up from T1 is this much smaller.
const CERVICAL_STEP: f64 = 0.95;

/// One vertebra's sizes, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertebraSize {
    pub height: f64,
    /// The disc under this vertebra.
    pub disc_below: f64,
    pub top_half_width: f64,
    pub top_half_depth: f64,
    pub bottom_half_width: f64,
    pub bottom_half_depth: f64,
    pub pedicle_width: f64,
    pub pedicle_height: f64,
}

/// Pedicle width and height, adult.
fn pedicle(row: usize) -> (f64, f64) {
    match row {
        0..=2 => (6.0, 9.0),
        3..=7 => (4.5, 10.0),
        8..=11 => (6.5, 13.0),
        12..=14 => (7.5, 14.0),
        15 => (9.0, 14.0),
        _ => (11.0, 14.0),
    }
}

/// The base sizes of a table row, with `disc_row` giving the disc below.
fn row_size(row: usize, disc_row: usize) -> VertebraSize {
    let (pedicle_width, pedicle_height) = pedicle(row);
    VertebraSize {
        height: HEIGHT[row],
        disc_below: DISC_BELOW[disc_row],
        top_half_width: 0.5 * TOP_WIDTH[row],
        top_half_depth: 0.5 * TOP_DEPTH[row],
        bottom_half_width: 0.5 * TOP_WIDTH[row] * BOTTOM_SCALE,
        bottom_half_depth: 0.5 * TOP_DEPTH[row] * BOTTOM_SCALE,
        pedicle_width,
        pedicle_height,
    }
}

impl VertebraSize {
    /// Every length times `factor`.
    fn scaled(&self, factor: f64) -> Self {
        VertebraSize {
            height: self.height * factor,
            disc_below: self.disc_below * factor,
            top_half_width: self.top_half_width * factor,
            top_half_depth: self.top_half_depth * factor,
            bottom_half_width: self.bottom_half_width * factor,
            bottom_half_depth: self.bottom_half_depth * factor,
            pedicle_width: self.pedicle_width * factor,
            pedicle_height: self.pedicle_height * factor,
        }
    }
}

/// The base sizes of each of `levels` (top to bottom, any cervical levels
/// first, ending with the last lumbar vertebra), scaled by `k`.
pub fn base_sizes(levels: &[Level], k: f64) -> Vec<VertebraSize> {
    let last = levels.len().saturating_sub(1);
    let mut thoracic_seen = 0usize;
    let mut lumbar_seen = 0usize;
    let rows: Vec<Option<usize>> = levels
        .iter()
        .map(|level| match level.region() {
            Region::Cervical => None,
            Region::Thoracic => {
                let row = thoracic_seen.min(11);
                thoracic_seen += 1;
                Some(row)
            }
            Region::Lumbar => {
                let row = 12 + lumbar_seen.min(4);
                lumbar_seen += 1;
                Some(row)
            }
        })
        .collect();
    let t1 = row_size(0, 0);
    levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            let size = match rows[index] {
                Some(row) => {
                    let disc_row = if index == last {
                        16
                    } else if row == 16 {
                        15
                    } else {
                        row
                    };
                    row_size(row, disc_row)
                }
                None => {
                    let steps_up = Level::T1.order() - level.order();
                    t1.scaled(libm::pow(CERVICAL_STEP, steps_up as f64))
                }
            };
            size.scaled(k)
        })
        .collect()
}

/// The sizes with each width and depth multiplied by a normal factor
/// (mean 1, sd 5%, within two sd) and each height and disc by another
/// (sd 3%, within two sd). One width factor and one depth factor per
/// vertebra keep the lower endplate 4% larger than the upper.
pub fn jittered(sizes: &[VertebraSize], stream: &mut Stream) -> Vec<VertebraSize> {
    sizes
        .iter()
        .map(|size| {
            let width = stream.truncated_normal(1.0, 0.05, 0.9, 1.1);
            let depth = stream.truncated_normal(1.0, 0.05, 0.9, 1.1);
            let height = stream.truncated_normal(1.0, 0.03, 0.94, 1.06);
            let disc = stream.truncated_normal(1.0, 0.03, 0.94, 1.06);
            VertebraSize {
                height: size.height * height,
                disc_below: size.disc_below * disc,
                top_half_width: size.top_half_width * width,
                top_half_depth: size.top_half_depth * depth,
                bottom_half_width: size.bottom_half_width * width,
                bottom_half_depth: size.bottom_half_depth * depth,
                pedicle_width: size.pedicle_width * width,
                pedicle_height: size.pedicle_height * height,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::naming::Template;

    #[test]
    fn the_table_straightens_to_its_published_sums() {
        // Busscher's heights sum to 360.3 mm and the discs to 120.3 mm.
        let sizes = base_sizes(&Level::MEASURED, 1.0);
        let heights: f64 = sizes.iter().map(|s| s.height).sum();
        let discs: f64 = sizes.iter().map(|s| s.disc_below).sum();
        assert!((heights - 360.3).abs() < 1e-9, "{heights}");
        assert!((discs - 120.3).abs() < 1e-9, "{discs}");
    }

    #[test]
    fn everything_scales_with_k() {
        let one = base_sizes(&Level::MEASURED, 1.0);
        let small = base_sizes(&Level::MEASURED, 0.8);
        for (a, b) in one.iter().zip(&small) {
            assert!((b.height - 0.8 * a.height).abs() < 1e-12);
            assert!((b.bottom_half_depth - 0.8 * a.bottom_half_depth).abs() < 1e-12);
        }
    }

    #[test]
    fn odd_counts_copy_their_neighbours_and_keep_the_lumbosacral_disc() {
        let extra_lumbar = base_sizes(&Template::ExtraLumbar.levels(), 1.0);
        let last = extra_lumbar.last().expect("a level");
        let l5 = extra_lumbar[extra_lumbar.len() - 2];
        assert_eq!(last.height, 25.3);
        assert_eq!(last.disc_below, 8.8);
        assert_eq!(l5.disc_below, 12.7);
        let missing_lumbar = base_sizes(&Template::MissingLumbar.levels(), 1.0);
        assert_eq!(missing_lumbar.last().expect("a level").disc_below, 8.8);
        let extra_thoracic = base_sizes(&Template::ExtraThoracic.levels(), 1.0);
        // C7, T1 … T12, T13: T13 copies T12.
        assert_eq!(extra_thoracic[13].height, extra_thoracic[12].height);
    }

    #[test]
    fn cervical_vertebrae_shrink_upward_from_t1() {
        let sizes = base_sizes(&[Level::C6, Level::C7, Level::T1], 1.0);
        assert!((sizes[1].height - 16.1 * 0.95).abs() < 1e-12);
        assert!((sizes[0].height - 16.1 * 0.95 * 0.95).abs() < 1e-12);
        assert!((sizes[0].top_half_width - 13.85 * 0.9025).abs() < 1e-12);
    }

    #[test]
    fn jitter_keeps_every_size_within_two_standard_deviations() {
        let base = base_sizes(&Level::MEASURED, 1.0);
        let jittered = jittered(&base, &mut Stream::new("sizes", 3));
        for (a, b) in base.iter().zip(&jittered) {
            let width = b.top_half_width / a.top_half_width;
            assert!((0.9..=1.1).contains(&width));
            assert!((b.bottom_half_width / a.bottom_half_width - width).abs() < 1e-12);
            assert!((0.94..=1.06).contains(&(b.height / a.height)));
        }
    }
}
