//! Finds isolated lead letters by local contrast and parametric shape
//! correlation. Letters are evidence only; no confirmation is set here.

mod glyphs;

use darkroom::grey_image::GreyImage;
use darkroom::{blur, pyramid, regions};
use film_reader::decoded_film::DecodedFilm;
use shapes::film_geometry::{LeadMarker, MarkerLetter, MarkerMaterial};
use shapes::pixels::PxRect;

#[derive(Clone)]
struct Part {
    label: u32,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    area: usize,
    contrast: f64,
    peak: f32,
}

impl Part {
    fn width(&self) -> usize {
        self.right - self.left + 1
    }
    fn height(&self) -> usize {
        self.bottom - self.top + 1
    }
    fn longest(&self) -> usize {
        self.width().max(self.height())
    }
    fn centre(&self) -> (f64, f64) {
        (
            (self.left + self.right) as f64 * 0.5,
            (self.top + self.bottom) as f64 * 0.5,
        )
    }
    fn bounds(&self, image: &GreyImage<f32>) -> PxRect {
        let a = image.film_point(self.left as f64 - 0.5, self.top as f64 - 0.5);
        let b = image.film_point(self.right as f64 + 0.5, self.bottom as f64 + 0.5);
        PxRect {
            left: a.x(),
            top: a.y(),
            right: b.x(),
            bottom: b.y(),
        }
    }
}

/// Finds letters and possible text, in original film pixels. A missing
/// letter means no proposal; a letter never confirms a patient's side.
pub fn find(film: &DecodedFilm) -> (Vec<LeadMarker>, Vec<PxRect>) {
    let Some(image) = pyramid::shrink(&film.image, 4.0, 4.0) else {
        return (vec![], vec![]);
    };
    if image.width() < 3 || image.height() < 3 {
        return (vec![], vec![]);
    }
    let spacing = film
        .facts
        .kept_tags
        .as_ref()
        .and_then(|tags| tags.pixel_spacing.or(tags.imager_pixel_spacing))
        .map(|s| s[1])
        .filter(|s| s.is_finite() && (0.01..=2.0).contains(s));
    let (hmin, hmax) = spacing.map_or((10.0, 100.0), |s| {
        (8.0 / (s * 4.0) * 0.8, 40.0 / (s * 4.0) * 1.2)
    });
    let shutter =
        crate::rough_trace::exclusions::shutter_mask(&image, film.facts.kept_tags.as_ref());
    let bands = crate::rough_trace::exclusions::collimation(&image, &shutter);
    let points: Vec<_> = (0..image.pixels().len())
        .map(|i| {
            image.film_point(
                (i % image.width() as usize) as f64,
                (i / image.width() as usize) as f64,
            )
        })
        .collect();
    let valid: Vec<bool> = points
        .iter()
        .zip(&shutter)
        .map(|(p, keep)| {
            *keep
                && !bands.iter().any(|b| {
                    p.x() >= b.left && p.x() <= b.right && p.y() >= b.top && p.y() <= b.bottom
                })
        })
        .collect();
    // A dark air margin and an unexposed border have the same line
    // statistics. Do not discard a high-contrast letter in that ambiguity;
    // only bright collimation restricts the marker search to its inner edge.
    let overall_mid = percentile(image.pixels(), 0.5);
    let bright_bands: Vec<_> = bands
        .iter()
        .filter(|b| {
            let samples: Vec<_> = image
                .pixels()
                .iter()
                .zip(&points)
                .filter_map(|(value, p)| {
                    (p.x() >= b.left && p.x() <= b.right && p.y() >= b.top && p.y() <= b.bottom)
                        .then_some(*value)
                })
                .collect();
            percentile(&samples, 0.5) > overall_mid
        })
        .collect();
    let search_valid: Vec<bool> = points
        .iter()
        .zip(&shutter)
        .map(|(p, keep)| {
            *keep
                && !bright_bands.iter().any(|b| {
                    if !(p.x() >= b.left && p.x() <= b.right && p.y() >= b.top && p.y() <= b.bottom)
                    {
                        return false;
                    }
                    if b.right - b.left < b.bottom - b.top {
                        let edge = if b.left < 0.0 { b.right } else { b.left };
                        (p.x() - edge).abs() > 0.03 * f64::from(film.image.width())
                    } else {
                        let edge = if b.top < 0.0 { b.bottom } else { b.top };
                        (p.y() - edge).abs() > 0.03 * f64::from(film.image.height())
                    }
                })
        })
        .collect();
    let values: Vec<f32> = image
        .pixels()
        .iter()
        .zip(&valid)
        .filter_map(|(p, keep)| keep.then_some(*p))
        .collect();
    if values.is_empty() || values.iter().all(|p| *p == values[0]) {
        return (vec![], vec![]);
    }
    let masked = GreyImage::new(
        image.width(),
        image.height(),
        image
            .pixels()
            .iter()
            .zip(&shutter)
            .map(|(p, keep)| if *keep { *p } else { 0.0 })
            .collect(),
        image.placement(),
    )
    .expect("mask keeps dimensions");
    let weights = GreyImage::new(
        image.width(),
        image.height(),
        shutter
            .iter()
            .map(|keep| if *keep { 1.0 } else { 0.0 })
            .collect(),
        image.placement(),
    )
    .expect("mask keeps dimensions");
    let mean = blur::box_blur(&masked, hmax.ceil() as u32);
    let weights = blur::box_blur(&weights, hmax.ceil() as u32);
    let contrast: Vec<f32> = image
        .pixels()
        .iter()
        .zip(mean.pixels())
        .zip(weights.pixels())
        .map(|((p, m), n)| if *n > 0.0 { p - m / n } else { 0.0 })
        .collect();
    let contrast_values: Vec<f32> = contrast
        .iter()
        .zip(&valid)
        .filter_map(|(p, keep)| keep.then_some(*p))
        .collect();
    let range = percentile(&values, 0.99) - percentile(&values, 0.01);
    let threshold = percentile(&contrast_values, 0.995)
        .max(0.15 * range)
        .max(f32::EPSILON);
    // Retain the high-percentile requirement as a seed, then follow the
    // connected stroke down to half that contrast (never below the
    // specified range floor). This preserves strokes across gradients.
    let edge_threshold = (0.15 * range).max(0.5 * threshold).max(f32::EPSILON);
    let bright_pixels: Vec<bool> = contrast
        .iter()
        .zip(&search_valid)
        .map(|(c, keep)| *keep && *c >= edge_threshold)
        .collect();
    let bright = close(&image.map(|_| false), &bright_pixels);
    let bright_parts = regions::label_components(&bright, true);
    let mut markers = vec![];
    let mut text = vec![];
    let templates = glyphs::templates();
    search(
        film,
        &image,
        &contrast,
        &bright_parts,
        (hmin, hmax),
        &bright_pixels,
        false,
        (threshold, edge_threshold),
        templates,
        &mut markers,
        &mut text,
    );

    // A dark letter must be enclosed by a bright plate. Background reached
    // from the film's edge is never a cut-out candidate. Contrast here is
    // relative to the enclosing plate: a wide background mean can be dark
    // even when the letter is a deep hole in a small bright plate.
    let plate_threshold = (0.15 * range).max(f32::EPSILON);
    let plate = close(
        &bright,
        &contrast
            .iter()
            .zip(&search_valid)
            .map(|(c, keep)| *keep && *c >= plate_threshold)
            .collect::<Vec<_>>(),
    );
    let enclosed = enclosed_background(&plate);
    let holes: Vec<bool> = contrast
        .iter()
        .zip(enclosed)
        .zip(&search_valid)
        .map(|((c, enclosed), keep)| *keep && enclosed && *c < plate_threshold)
        .collect();
    let hole_mask = close(&bright, &holes);
    let hole_parts = regions::label_components(&hole_mask, true);
    search(
        film,
        &image,
        &contrast,
        &hole_parts,
        (hmin, hmax),
        &holes,
        true,
        (plate_threshold, plate_threshold),
        templates,
        &mut markers,
        &mut text,
    );
    markers.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.bounds.top.total_cmp(&b.bounds.top))
            .then_with(|| a.bounds.left.total_cmp(&b.bounds.left))
    });
    text.sort_by(|a, b| {
        a.top
            .total_cmp(&b.top)
            .then_with(|| a.left.total_cmp(&b.left))
    });
    text.dedup();
    (markers, text)
}

fn percentile(values: &[f32], fraction: f64) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    sorted[((sorted.len() - 1) as f64 * fraction) as usize]
}

fn close(shape: &GreyImage<bool>, mask: &[bool]) -> GreyImage<bool> {
    let (w, h) = (shape.width() as usize, shape.height() as usize);
    let mut dilated = vec![false; mask.len()];
    for y in 0..h {
        for x in 0..w {
            dilated[y * w + x] = (y.saturating_sub(1)..=(y + 1).min(h - 1))
                .any(|yy| (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|xx| mask[yy * w + xx]));
        }
    }
    let mut closed = vec![false; mask.len()];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            closed[y * w + x] =
                (y - 1..=y + 1).all(|yy| (x - 1..=x + 1).all(|xx| dilated[yy * w + xx]));
        }
    }
    GreyImage::new(shape.width(), shape.height(), closed, shape.placement())
        .expect("mask keeps its dimensions")
}

fn enclosed_background(bright: &GreyImage<bool>) -> Vec<bool> {
    let negative = bright.map(|v| !v);
    let parts = regions::label_components(&negative, false);
    let (w, h) = (bright.width() as usize, bright.height() as usize);
    let mut outside = vec![false; parts.count as usize + 1];
    outside[0] = true;
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                outside[parts.labels.pixels()[y * w + x] as usize] = true;
            }
        }
    }
    parts
        .labels
        .pixels()
        .iter()
        .map(|v| !outside[*v as usize])
        .collect()
}

fn parts(labels: &regions::Components, contrast: &[f32], original: &[bool]) -> Vec<Part> {
    let w = labels.labels.width() as usize;
    let mut parts: Vec<Part> = (0..labels.count)
        .map(|n| Part {
            label: n + 1,
            left: usize::MAX,
            top: usize::MAX,
            right: 0,
            bottom: 0,
            area: 0,
            contrast: 0.0,
            peak: 0.0,
        })
        .collect();
    for (i, label) in labels.labels.pixels().iter().enumerate() {
        if *label == 0 {
            continue;
        }
        let part = &mut parts[*label as usize - 1];
        let (x, y) = (i % w, i / w);
        part.left = part.left.min(x);
        part.top = part.top.min(y);
        part.right = part.right.max(x);
        part.bottom = part.bottom.max(y);
        part.area += usize::from(original[i]);
        part.contrast += f64::from(contrast[i].abs());
        part.peak = part.peak.max(contrast[i]);
    }
    parts
}

#[allow(clippy::too_many_arguments)]
fn search(
    film: &DecodedFilm,
    image: &GreyImage<f32>,
    contrast: &[f32],
    labels: &regions::Components,
    (hmin, hmax): (f64, f64),
    original: &[bool],
    cut_out: bool,
    (threshold, edge_threshold): (f32, f32),
    templates: &[glyphs::Template],
    markers: &mut Vec<LeadMarker>,
    hints: &mut Vec<PxRect>,
) {
    let parts = parts(labels, contrast, original);
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut candidates = vec![];
    for part in &parts {
        let longest = part.longest() as f64;
        let fill = part.area as f64 / (part.width() * part.height()) as f64;
        if longest < hmin
            || longest > hmax
            || part.width().min(part.height()) as f64 / longest < 0.35
            || !(0.15..=0.75).contains(&fill)
        {
            continue;
        }
        let (cx, cy) = part.centre();
        let neighbours = parts
            .iter()
            .filter(|other| {
                if other.label == part.label {
                    return false;
                }
                let (ox, oy) = other.centre();
                let similar = (0.5..=2.0).contains(&(other.longest() as f64 / longest));
                similar
                    && (((oy - cy).abs() < 0.5 * longest && (ox - cx).abs() < 2.0 * longest)
                        || ((ox - cx).abs() < 0.5 * longest && (oy - cy).abs() < 2.0 * longest))
            })
            .count();
        if neighbours > 0 {
            hints.push(part.bounds(image));
            continue;
        }
        if cx < 0.12 * w as f64
            || cx > 0.88 * w as f64
            || cy < 0.12 * h as f64
            || cy > 0.88 * h as f64
        {
            hints.push(part.bounds(image));
        }
        if cut_out || part.peak >= threshold {
            candidates.push(part);
        }
    }
    candidates.sort_by(|a, b| {
        b.contrast
            .total_cmp(&a.contrast)
            .then(a.label.cmp(&b.label))
    });
    candidates.truncate(40);
    for part in candidates {
        let (cx, cy) = part.centre();
        let side = 1.25 * part.longest() as f64;
        // Components are found cheaply on the coarse mask; read the
        // untouched pixels for correlation so a narrow R aperture or leg
        // does not disappear under the four-pixel averaging or closing.
        let bits = glyphs::area_bits(
            |x, y| {
                if x < 0
                    || y < 0
                    || x >= film.image.width() as i32
                    || y >= film.image.height() as i32
                {
                    return false;
                }
                let i = (y as usize / 4) * w + x as usize / 4;
                if x as usize / 4 >= w
                    || y as usize / 4 >= h
                    || labels.labels.pixels()[i] != part.label
                {
                    return false;
                }
                let local_mean = image.pixels()[i] - contrast[i];
                let c = f32::from(
                    film.image.pixels()[y as usize * film.image.width() as usize + x as usize],
                ) - local_mean;
                if cut_out {
                    c < edge_threshold
                } else {
                    c >= edge_threshold
                }
            },
            4.0 * (cx + 0.5 - side * 0.5),
            4.0 * (cy + 0.5 - side * 0.5),
            4.0 * side,
        );
        let mut best = [(-1.0, 0usize); 2];
        for (index, template) in templates.iter().enumerate() {
            let score = glyphs::score(&bits, &template.bits);
            let letter = usize::from(template.letter == MarkerLetter::R);
            if score > best[letter].0 {
                best[letter] = (score, index);
            }
        }
        let winner = usize::from(best[1].0 > best[0].0);
        let (score, index) = best[winner];
        let margin = score - best[1 - winner].0;
        if score < 0.60 || margin < 0.15 {
            continue;
        }
        let template = &templates[index];
        markers.push(LeadMarker {
            letter: template.letter,
            bounds: part.bounds(image),
            score,
            margin,
            mirrored: template.mirrored,
            rotation_deg: template.rotation,
            cut_out,
            material: if cut_out {
                MarkerMaterial::Lead
            } else {
                material(film, image, contrast, edge_threshold, labels, part)
            },
        });
    }
}

fn material(
    film: &DecodedFilm,
    image: &GreyImage<f32>,
    contrast: &[f32],
    threshold: f32,
    labels: &regions::Components,
    part: &Part,
) -> MarkerMaterial {
    let maximum = film.image.pixels().iter().copied().max().unwrap_or(0);
    for y in part.top * 4..((part.bottom + 1) * 4).min(film.image.height() as usize) {
        for x in part.left * 4..((part.right + 1) * 4).min(film.image.width() as usize) {
            let i = (y / 4) * labels.labels.width() as usize + x / 4;
            let native = film.image.pixels()[y * film.image.width() as usize + x];
            let local_mean = image.pixels()[i] - contrast[i];
            if labels.labels.pixels()[i] == part.label
                && f32::from(native) - local_mean >= threshold
                && native != maximum
            {
                return MarkerMaterial::Lead;
            }
        }
    }
    MarkerMaterial::Digital
}
