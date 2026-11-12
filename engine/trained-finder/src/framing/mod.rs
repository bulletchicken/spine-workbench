//! A full-length film cropped to its spine for a second look.
//!
//! The network was taught on films cropped close to the spine. On a
//! full-length film the spine is small once the film is shrunk, so the
//! first reading's peaks propose where the spine is, and a box around them
//! (with a margin, and no narrower than the films it was taught on) is read
//! again. A film whose first reading already shows a tall, long spine is
//! left whole. Every number comes from `weights/framing.json`, chosen when
//! the network was trained.
//!
//! Positions here are fractions of the picture, from its outer edges, as
//! [`crate::outline`] says; the box is in whole picture pixels counted the
//! same way, left and top included, right and bottom not.

use crate::outline::{Point, Reading};
use darkroom::grey_image::GreyImage;
use serde::Deserialize;
use shapes::errors::{EngineError, ErrorCode};
use std::sync::OnceLock;

/// The framing settings, compiled in.
const SETTINGS: &str = crate::weights::FRAMING;

/// The numbers the framing was tuned with.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    /// The first reading's outlines leave the film whole when their median
    /// height is at least this share of the picture height...
    gate_height: f64,
    /// ...and together they span at least this share of it.
    gate_span: f64,
    /// Peaks scoring at least this propose where the spine is.
    proposal_threshold: f64,
    /// The margin added above and below, as a share of the proposals' span.
    margin: f64,
    /// The box is at least the median proposal height over this.
    target_height: f64,
    /// The box's height over its width, in pixels, at the narrowest.
    aspect: f64,
    /// The margin added each side, as a share of the picture width.
    xmargin: f64,
}

fn settings() -> &'static Settings {
    static PARSED: OnceLock<Settings> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str(SETTINGS).expect("the compiled-in framing settings are well formed")
    })
}

/// The score a peak needs to propose where the spine is.
pub(crate) fn proposal_threshold() -> f64 {
    settings().proposal_threshold
}

/// Why a frame is what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameReason {
    /// The first reading already shows a tall, long spine: the film is
    /// left whole.
    ConservativeBypass,
    /// Fewer than three peaks were strong enough to say where the spine
    /// is: the film is left whole.
    InsufficientEvidence,
    /// Fewer than three of those lay together down the middle: the film is
    /// left whole.
    InsufficientCentralEvidence,
    /// The film is cropped to the spine and read again.
    SmallSpineCrop,
}

/// Where the second reading looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Left, top, right, bottom in picture pixels; left and top included,
    /// right and bottom not. The right or bottom edge can sit one pixel
    /// past the picture; what lies outside reads as black.
    pub bounds: [i64; 4],
    /// The picture's width and height.
    pub size: [u32; 2],
    pub reason: FrameReason,
}

impl Frame {
    /// The box is the whole picture, so no second reading is needed.
    pub fn whole_film(&self) -> bool {
        self.bounds == [0, 0, i64::from(self.size[0]), i64::from(self.size[1])]
    }

    /// The part of the picture inside the box, black where the box runs
    /// past the picture, placed where it sits on the film. Fails only with
    /// `EngineOutOfMemory`: the crop can be nearly as large as the film.
    pub fn crop(&self, picture: &GreyImage<u8>) -> Result<GreyImage<u8>, EngineError> {
        let [left, top, right, bottom] = self.bounds;
        let width = (right - left).max(0);
        let height = (bottom - top).max(0);
        let (picture_width, picture_height) =
            (i64::from(picture.width()), i64::from(picture.height()));
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact((width * height) as usize)
            .map_err(|_| {
                EngineError::new(
                    ErrorCode::EngineOutOfMemory,
                    "not enough memory to crop the film for the trained finder",
                )
            })?;
        for y in top..top + height {
            for x in left..left + width {
                let inside = (0..picture_width).contains(&x) && (0..picture_height).contains(&y);
                pixels.push(if inside {
                    picture.pixels()[(y * picture_width + x) as usize]
                } else {
                    0
                });
            }
        }
        let placement = darkroom::grey_image::Placement {
            origin: picture.film_point(left as f64, top as f64),
            ..picture.placement()
        };
        let crop = GreyImage::new(width as u32, height as u32, pixels, placement)
            .expect("the crop holds width times height pixels");
        Ok(crop)
    }

    /// A fraction of the cropped picture as a fraction of the whole one.
    /// It multiplies, adds and divides even for a whole-film frame, as the
    /// reference does, so the last bit matches.
    pub fn to_film(&self, point: Point) -> Point {
        let [left, top, right, bottom] = self.bounds.map(|edge| edge as f64);
        [
            (point[0] * (right - left) + left) / f64::from(self.size[0]),
            (point[1] * (bottom - top) + top) / f64::from(self.size[1]),
        ]
    }
}

/// The frame for a picture of `size` (width, height), from its first
/// reading.
pub fn frame_from_reading(size: [u32; 2], first: &Reading) -> Frame {
    let settings = settings();
    let (width, height) = (f64::from(size[0]), f64::from(size[1]));
    let whole = |reason| Frame {
        bounds: [0, 0, i64::from(size[0]), i64::from(size[1])],
        size,
        reason,
    };

    if !first.outlines.is_empty() {
        let mut heights: Vec<f64> = first
            .outlines
            .iter()
            .map(|outline| spread(outline.corners.iter().map(|c| c[1])))
            .collect();
        let tall = median(&mut heights);
        let span = spread(
            first
                .outlines
                .iter()
                .flat_map(|o| o.corners.iter().map(|c| c[1])),
        );
        if tall >= settings.gate_height && span >= settings.gate_span {
            return whole(FrameReason::ConservativeBypass);
        }
    }

    let proposals: Vec<_> = first
        .peaks
        .iter()
        .filter(|peak| peak.score >= settings.proposal_threshold)
        .map(|peak| peak.corners)
        .collect();
    if proposals.len() < 3 {
        return whole(FrameReason::InsufficientEvidence);
    }
    let centres: Vec<f64> = proposals.iter().map(mean_across).collect();
    let widths: Vec<f64> = proposals
        .iter()
        .map(|q| spread(q.iter().map(|c| c[0])))
        .collect();
    let heights: Vec<f64> = proposals
        .iter()
        .map(|q| spread(q.iter().map(|c| c[1])))
        .collect();
    let middle = median(&mut centres.clone());
    let middle_width = median(&mut widths.clone());
    let middle_height = median(&mut heights.clone());
    let reach = 0.15f64.max(1.5 * middle_width);
    let kept: Vec<_> = (0..proposals.len())
        .filter(|&i| {
            (centres[i] - middle).abs() < reach
                && widths[i] < 2.0 * middle_width
                && heights[i] < 2.0 * middle_height
        })
        .map(|i| proposals[i])
        .collect();
    if kept.len() < 3 {
        return whole(FrameReason::InsufficientCentralEvidence);
    }

    let downs = || kept.iter().flat_map(|q| q.iter().map(|c| c[1]));
    let low = downs().fold(f64::INFINITY, f64::min);
    let high = downs().fold(f64::NEG_INFINITY, f64::max);
    let span = high - low;
    let mut kept_heights: Vec<f64> = kept
        .iter()
        .map(|q| spread(q.iter().map(|c| c[1])))
        .collect();
    let box_height = (span * (1.0 + 2.0 * settings.margin))
        .max(median(&mut kept_heights) / settings.target_height)
        .min(1.0);
    let middle_down = (low + high) / 2.0;
    let y0 = 0.0f64.max((1.0 - box_height).min(middle_down - box_height / 2.0));
    let y1 = y0 + box_height;

    let across_spread = spread(kept.iter().flat_map(|q| q.iter().map(|c| c[0])));
    let box_width = (box_height * height / width / settings.aspect)
        .max(across_spread + 2.0 * settings.xmargin)
        .min(1.0);
    let mut kept_centres: Vec<f64> = kept.iter().map(mean_across).collect();
    let middle_across = median(&mut kept_centres);
    let x0 = 0.0f64.max((1.0 - box_width).min(middle_across - box_width / 2.0));
    let x1 = x0 + box_width;

    Frame {
        bounds: [
            (x0 * width).floor() as i64,
            (y0 * height).floor() as i64,
            (x1 * width).ceil() as i64,
            (y1 * height).ceil() as i64,
        ],
        size,
        reason: FrameReason::SmallSpineCrop,
    }
}

/// The largest value less the smallest.
fn spread(values: impl Iterator<Item = f64>) -> f64 {
    let (low, high) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    high - low
}

/// The mean of a quad's four across values, summed in order.
fn mean_across(quad: &[Point; 4]) -> f64 {
    (((quad[0][0] + quad[1][0]) + quad[2][0]) + quad[3][0]) / 4.0
}

/// The middle value, or the mean of the two middle values for an even
/// count. Reorders `values`.
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::Peak;
    use darkroom::grey_image::Placement;

    fn peak(score: f64, left: f64, top: f64, size: f64) -> Peak {
        let corners = [
            [left, top],
            [left + size, top],
            [left + size, top + size / 2.0],
            [left, top + size / 2.0],
        ];
        Peak {
            score,
            centre: [left + size / 2.0, top + size / 4.0],
            corners,
            cell: [0, 0],
        }
    }

    #[test]
    fn the_compiled_in_settings_read() {
        let settings = settings();
        assert_eq!(settings.proposal_threshold, 0.25);
        assert_eq!(settings.xmargin, 0.05);
    }

    #[test]
    fn nothing_found_leaves_the_film_whole() {
        let frame = frame_from_reading(
            [400, 800],
            &Reading {
                peaks: Vec::new(),
                outlines: Vec::new(),
            },
        );
        assert_eq!(frame.reason, FrameReason::InsufficientEvidence);
        assert!(frame.whole_film());
    }

    #[test]
    fn scattered_peaks_leave_the_film_whole() {
        // Three strong peaks, but each far across from the others: none
        // sits within 0.15 of the middle one except itself.
        let peaks = vec![
            peak(0.9, 0.05, 0.2, 0.04),
            peak(0.9, 0.45, 0.4, 0.04),
            peak(0.9, 0.85, 0.6, 0.04),
        ];
        let frame = frame_from_reading(
            [400, 800],
            &Reading {
                peaks,
                outlines: Vec::new(),
            },
        );
        assert_eq!(frame.reason, FrameReason::InsufficientCentralEvidence);
        assert!(frame.whole_film());
    }

    #[test]
    fn a_crop_past_the_picture_reads_black_and_keeps_its_place() {
        let picture = GreyImage::new(2, 2, vec![1, 2, 3, 4], Placement::FILM).unwrap();
        let frame = Frame {
            bounds: [1, 0, 3, 2],
            size: [2, 2],
            reason: FrameReason::SmallSpineCrop,
        };
        let crop = frame.crop(&picture).unwrap();
        assert_eq!(crop.pixels(), &[2, 0, 4, 0]);
        assert_eq!(crop.film_point(0.0, 0.0), picture.film_point(1.0, 0.0));
        assert_eq!(frame.to_film([0.5, 0.5]), [1.0, 0.5]);
    }
}
