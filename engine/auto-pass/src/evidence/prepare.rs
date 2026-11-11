//! A bounded working copy, its usable field, and noise-floored contrast.

#[path = "conditioning.rs"]
mod conditioning;
#[path = "masking.rs"]
mod masking;

use crate::handoffs::{Polarity, PolarityCheck};
use darkroom::grey_image::GreyImage;
use darkroom::pyramid;
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{PassWarning, PassWarningCode};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{FilmKind, MAX_FILM_SIDE, View};
use shapes::film_geometry::{FilmGeometry, QuarterTurns};
use shapes::pixels::PxRect;

pub(super) struct PreparedEvidence {
    pub image: GreyImage<f32>,
    pub valid: GreyImage<u8>,
    pub contrast: GreyImage<f32>,
    pub mm_per_px: f64,
    pub scale_assumed: bool,
    pub polarity: PolarityCheck,
    pub markers: Vec<PxRect>,
    pub seam_rows: Vec<f64>,
    pub valid_fraction: f64,
    pub warnings: Vec<PassWarning>,
}

pub(super) fn local_contrast(
    image: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    spacing: f64,
) -> GreyImage<f32> {
    conditioning::contrast(image, valid, spacing)
}

/// The decoder has already made bone bright. An explicit override acts on
/// those stored values, never on the original file's photometric convention.
pub(super) fn prepare(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    polarity_override: Option<Polarity>,
) -> Result<PreparedEvidence, EngineError> {
    prepare_with_exclusions(film, geometry, polarity_override, &[])
}

/// Case-file blanked boxes exclude every working pixel whose averaging
/// footprint touched the box, so no excluded value reaches a filter.
pub(super) fn prepare_with_exclusions(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    polarity_override: Option<Polarity>,
    blanked: &[PxRect],
) -> Result<PreparedEvidence, EngineError> {
    let (width, height) = (film.image.width(), film.image.height());
    if !(64..=MAX_FILM_SIDE).contains(&width)
        || !(64..=MAX_FILM_SIDE).contains(&height)
        || width != film.facts.width
        || height != film.facts.height
        || !(1..=16).contains(&film.facts.bits_stored)
    {
        return Err(EngineError::new(
            ErrorCode::PassFilmTooSmall,
            "the film dimensions or stored precision cannot be searched",
        ));
    }
    if blanked.iter().any(|area| !area.is_on_film(width, height)) {
        return Err(EngineError::new(
            ErrorCode::CaseInvalidValue,
            "a blanked region lies outside its film",
        ));
    }
    let scale_assumed = geometry.mm_per_px.is_none();
    let sx = geometry
        .mm_per_px
        .unwrap_or_else(|| assumed_spacing(film, geometry, blanked));
    let sy = geometry.mm_per_px_vertical.unwrap_or(sx);
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 || sx > 2.0 || sy > 2.0 {
        return Err(EngineError::new(
            ErrorCode::PassFilmTooSmall,
            "the film spacing cannot support the evidence search",
        ));
    }
    let budget_spacing = (f64::from(width) * f64::from(height) * sx * sy / 2.4e6).sqrt();
    let mm_per_px = sx.max(sy).max(0.35).max(budget_spacing);
    let work = pyramid::shrink(&film.image, mm_per_px / sx, mm_per_px / sy).ok_or_else(|| {
        EngineError::new(ErrorCode::PassFilmTooSmall, "the working grid is empty")
    })?;
    if work.width() < 16 || work.height() < 16 {
        return Err(EngineError::new(
            ErrorCode::PassFilmTooSmall,
            "the working grid is too small",
        ));
    }
    let usable = exclude_footprints(&work, blanked);
    if usable.iter().filter(|&&v| v != 0).count() * 5 < usable.len() {
        return Err(EngineError::new(
            ErrorCode::PassTooLittleUsable,
            "blanked regions leave too little usable film",
        ));
    }
    let window = conditioning::window(work.pixels(), Some(&usable), 0.001, 0.999);
    let span = window.1 - window.0;
    let observed_max = work
        .pixels()
        .iter()
        .zip(&usable)
        .filter_map(|(&v, &on)| (on != 0).then_some(v))
        .fold(0.0f32, f32::max);
    if span < (observed_max * 1e-6).max(1e-6) {
        return Err(EngineError::new(
            ErrorCode::PassFlatFilm,
            "the film has no usable contrast",
        ));
    }
    let preliminary = conditioning::like(
        &work,
        work.pixels()
            .iter()
            .zip(&usable)
            .map(|(&value, &on)| {
                if on == 0 {
                    0.0
                } else {
                    ((value - window.0) / span).clamp(0.0, 1.0)
                }
            })
            .collect(),
    );
    let mut masked = masking::find(&preliminary, mm_per_px, geometry.quarter_turns, usable);
    let mut warnings = Vec::new();
    let kind = film.facts.kind;
    if scale_assumed {
        warnings.push(warning(kind, PassWarningCode::ScaleAssumed));
        warnings.push(warning(kind, PassWarningCode::ScaleGuess));
    }
    if budget_spacing > sx.max(sy).max(0.35) {
        warnings.push(warning(kind, PassWarningCode::BudgetReduced));
    }
    if !masked.seam_rows.is_empty() {
        warnings.push(warning(kind, PassWarningCode::SeamsFound));
    }
    if masked.central_marker {
        warnings.push(warning(kind, PassWarningCode::MarkersOverSpine));
    }
    let polarity = polarity_check(
        &preliminary,
        &masked.valid,
        kind,
        geometry.quarter_turns,
        polarity_override,
    );
    if polarity.needs_confirm {
        warnings.push(warning(kind, PassWarningCode::PolarityUncertain));
    }
    // The fill sees the original mask; the extra three-millimetre guard
    // removes the residual filter responses from the final evidence maps.
    let image =
        conditioning::stretch_and_fill(&preliminary, &masked.valid, mm_per_px, polarity.found);
    masked.valid = masking::guard(&masked.valid, mm_per_px);
    let valid_fraction = masked.valid.pixels().iter().filter(|&&v| v != 0).count() as f64
        / masked.valid.pixels().len() as f64;
    if valid_fraction < 0.2 {
        return Err(EngineError::new(
            ErrorCode::PassTooLittleUsable,
            "less than a fifth of the film is usable",
        ));
    }
    let contrast = conditioning::contrast(&image, &masked.valid, mm_per_px);
    Ok(PreparedEvidence {
        image,
        valid: masked.valid,
        contrast,
        mm_per_px,
        scale_assumed,
        polarity,
        markers: masked.markers,
        seam_rows: masked.seam_rows,
        valid_fraction,
        warnings,
    })
}

fn exclude_footprints(image: &GreyImage<f32>, blanked: &[PxRect]) -> Vec<u8> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut usable = vec![1; w * h];
    for area in blanked {
        let (left, top) = image
            .image_point(shapes::pixels::Px::new(area.left, area.top))
            .expect("working placement is invertible");
        let (right, bottom) = image
            .image_point(shapes::pixels::Px::new(area.right, area.bottom))
            .expect("working placement is invertible");
        let x0 = (left + 0.5).floor().clamp(0.0, w as f64) as usize;
        let x1 = (right + 0.5).ceil().clamp(0.0, w as f64) as usize;
        let y0 = (top + 0.5).floor().clamp(0.0, h as f64) as usize;
        let y1 = (bottom + 0.5).ceil().clamp(0.0, h as f64) as usize;
        for row in y0..y1 {
            usable[row * w + x0..row * w + x1].fill(0)
        }
    }
    usable
}

fn warning(film: FilmKind, code: PassWarningCode) -> PassWarning {
    PassWarning {
        code,
        film,
        level: None,
        region: None,
    }
}

fn polarity_check(
    image: &GreyImage<f32>,
    valid: &GreyImage<u8>,
    kind: FilmKind,
    turns: QuarterTurns,
    override_value: Option<Polarity>,
) -> PolarityCheck {
    if let Some(found) = override_value {
        return PolarityCheck {
            found,
            confidence: 1.0,
            needs_confirm: false,
        };
    }
    let (stored_w, stored_h) = (image.width() as usize, image.height() as usize);
    let (w, h) = upright_size(stored_w, stored_h, turns);
    let mut inner = Vec::new();
    let mut outer = Vec::new();
    let mut border = Vec::new();
    for y in h / 5..h / 2 {
        let index = |x| upright_index(stored_w, stored_h, turns, x, y);
        let Some(left) = (0..w).find(|&x| valid.pixels()[index(x)] != 0) else {
            continue;
        };
        let right = (0..w)
            .rfind(|&x| valid.pixels()[index(x)] != 0)
            .unwrap_or(left);
        for x in left..=right {
            if valid.pixels()[index(x)] == 0 {
                continue;
            }
            let position = (x - left) as f64 / (right - left).max(1) as f64;
            let value = image.pixels()[index(x)];
            if (0.35..=0.65).contains(&position) {
                inner.push(value)
            }
            if !(0.15..=0.85).contains(&position) {
                outer.push(value)
            }
            if !(0.03..=0.97).contains(&position) {
                border.push(value)
            }
        }
    }
    let mut contrary = false;
    if border.len() > 20 {
        let (low, high) = conditioning::window(&border, None, 0.1, 0.9);
        contrary = low > 0.8 && high - low < 0.02;
    }
    if kind.view() == View::Front && !inner.is_empty() && !outer.is_empty() {
        let d = conditioning::median(&mut inner) - conditioning::median(&mut outer);
        let (lo, hi) = conditioning::window(image.pixels(), Some(valid.pixels()), 0.25, 0.75);
        contrary |= d < -0.15 * (hi - lo).max(0.05);
    }
    PolarityCheck {
        found: Polarity::BoneBright,
        confidence: 1.0,
        needs_confirm: contrary,
    }
}

/// The median half-height width of the central bright ridge is assigned
/// 24 anatomical units. It is a scale guess, never a physical measurement.
fn assumed_spacing(film: &DecodedFilm, geometry: &FilmGeometry, blanked: &[PxRect]) -> f64 {
    let (stored_w, stored_h) = (film.image.width() as usize, film.image.height() as usize);
    let (w, h) = upright_size(stored_w, stored_h, geometry.quarter_turns);
    let (w, h) = (w as u32, h as u32);
    let stride = (w / 256).max(1);
    let n = (w / stride) as usize;
    let mut widths = Vec::new();
    for row in 0..24u32 {
        let y = h / 3 + row * (h / 3) / 24;
        let profile: Vec<f32> = (0..n)
            .map(|x| {
                let x = (x as u32 * stride).min(w - 1);
                let index = upright_index(
                    stored_w,
                    stored_h,
                    geometry.quarter_turns,
                    x as usize,
                    y as usize,
                );
                if blanked.iter().any(|area| {
                    area.covers_pixel((index % stored_w) as u32, (index / stored_w) as u32)
                }) {
                    0.0
                } else {
                    f32::from(film.image.pixels()[index])
                }
            })
            .collect();
        let (lo, hi) = conditioning::window(&profile, None, 0.1, 0.95);
        if hi <= lo {
            continue;
        }
        let peak = (n / 4..3 * n / 4).max_by(|&a, &b| profile[a].total_cmp(&profile[b]));
        let Some(peak) = peak else { continue };
        let threshold = lo + 0.5 * (profile[peak] - lo);
        let mut left = peak;
        let mut right = peak;
        while left > 0 && profile[left - 1] > threshold {
            left -= 1
        }
        while right + 1 < n && profile[right + 1] > threshold {
            right += 1
        }
        let width = (right - left + 1) as f32 * stride as f32;
        if width >= w as f32 / 150.0 && width <= w as f32 / 3.0 {
            widths.push(width)
        }
    }
    if widths.len() >= 6 {
        (24.0 / f64::from(conditioning::median(&mut widths))).clamp(0.01, 2.0)
    } else {
        450.0 / f64::from(w.min(h))
    }
}

fn upright_size(width: usize, height: usize, turns: QuarterTurns) -> (usize, usize) {
    if turns.get().is_multiple_of(2) {
        (width, height)
    } else {
        (height, width)
    }
}

fn upright_index(width: usize, height: usize, turns: QuarterTurns, x: usize, y: usize) -> usize {
    let (u, v) = match turns.get() {
        0 => (x, y),
        1 => (y, height - 1 - x),
        2 => (width - 1 - x, height - 1 - y),
        _ => (width - 1 - y, x),
    };
    v * width + u
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;
    use shapes::film::{
        DisplayWindow, FilmFacts, PixelValues, SourceFormat, WindowFunction, WindowSource,
    };

    fn film(bits: u8, multiple: u16, constant: bool) -> DecodedFilm {
        let (width, height) = (160, 320);
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let value = if constant {
                    1500
                } else {
                    500 + (x + 2 * y) as u16
                        + if (65..95).contains(&x) && y % 35 < 27 {
                            1800
                        } else {
                            0
                        }
                };
                pixels.push(value * multiple);
            }
        }
        DecodedFilm {
            image: GreyImage::new(width, height, pixels, Placement::FILM).unwrap(),
            facts: FilmFacts {
                kind: FilmKind::Front,
                width,
                height,
                bits_stored: bits,
                source: SourceFormat::Png,
                kept_tags: None,
                values: PixelValues::AS_STORED,
                flags: Vec::new(),
                default_window: DisplayWindow::CentreWidth {
                    centre: 2000.0,
                    width: 4000.0,
                    function: WindowFunction::Linear,
                    from: WindowSource::Percentile,
                },
                exif_quarter_turns: None,
            },
        }
    }

    #[test]
    fn stored_bit_count_and_unused_code_range_do_not_change_preparation() {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let low = prepare(&film(12, 1, false), &geometry, None).unwrap();
        let container = prepare(&film(16, 1, false), &geometry, None).unwrap();
        let scaled = prepare(&film(16, 16, false), &geometry, None).unwrap();
        assert_eq!(low.valid, container.valid);
        assert_eq!(low.valid, scaled.valid);
        assert_eq!(low.image, container.image);
        // Multiplication by 16 is exact in binary, including histogram
        // coordinates and interpolated percentile endpoints.
        assert_eq!(low.image, scaled.image);
        assert_eq!(low.contrast, scaled.contrast);
        assert_eq!(low.polarity, scaled.polarity);
    }

    #[test]
    fn a_flat_film_is_refused_before_feature_filters() {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let error = match prepare(&film(12, 1, true), &geometry, None) {
            Ok(_) => panic!("a constant has no evidence"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::PassFlatFilm);
    }

    #[test]
    fn unequal_native_pixels_make_an_isotropic_exactly_placed_grid() {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.25),
            mm_per_px_vertical: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        let prepared = prepare(&film(12, 1, false), &geometry, None).unwrap();
        assert_eq!((prepared.image.width(), prepared.image.height()), (80, 320));
        let placement = prepared.image.placement();
        assert_eq!(placement.origin, shapes::pixels::Px::new(0.5, 0.0));
        assert_eq!(placement.step_across, shapes::pixels::Px::new(2.0, 0.0));
        assert_eq!(placement.step_down, shapes::pixels::Px::new(0.0, 1.0));
        assert_eq!(prepared.mm_per_px, 0.5);
    }

    #[test]
    fn upright_profiles_read_the_hand_worked_quarter_turns() {
        // Stored rows [0,1], [2,3], [4,5]. A clockwise turn puts
        // [4,2,0] above [5,3,1]. These are original linear indices.
        let expected = [
            vec![0, 1, 2, 3, 4, 5],
            vec![4, 2, 0, 5, 3, 1],
            vec![5, 4, 3, 2, 1, 0],
            vec![1, 3, 5, 0, 2, 4],
        ];
        for (q, wanted) in expected.iter().enumerate() {
            let turns = QuarterTurns::new(q as u8).unwrap();
            let (w, h) = upright_size(2, 3, turns);
            let mut found = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    found.push(upright_index(2, 3, turns, x, y));
                }
            }
            assert_eq!(&found, wanted);
        }
    }

    #[test]
    fn blanked_boxes_remove_whole_resampling_footprints_and_the_guard() {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.25),
            mm_per_px_vertical: Some(0.5),
            ..FilmGeometry::UNKNOWN
        };
        // This box is over 60 mm high: it is deliberately too large to
        // qualify as a small marker. Its left edge cuts through the
        // working cell whose native footprint is [39.5,41.5].
        let blanked = [PxRect {
            left: 40.25,
            top: 60.25,
            right: 62.25,
            bottom: 220.25,
        }];
        let prepared =
            prepare_with_exclusions(&film(12, 1, false), &geometry, None, &blanked).unwrap();
        assert_eq!(prepared.valid.get(20, 100), Some(&0));
        assert_eq!(prepared.valid.get(14, 100), Some(&0));
        assert_eq!(prepared.valid.get(13, 100), Some(&1));
        assert_eq!(prepared.valid.get(32, 100), Some(&0));
        assert!(prepared.image.pixels().iter().all(|v| v.is_finite()));
    }
}
