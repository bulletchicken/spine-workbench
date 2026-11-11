//! Opposed cortical walls: a narrow vertebral body amid broad soft tissue.
//!
//! Thin bright ridges at two plausible body walls distinguish vertebrae
//! from the broad chest shadow. Weak central fill lowers pairs spanning
//! unrelated bones. When anatomical facing is supplied, a comparable pair
//! anterior to a posterior process is preferred. Without supplied facing,
//! a consistent torso displacement can guide that preference; it does not
//! assert anatomical facing. All distances are millimetres (assumed units without
//! calibration), evaluated on the one-unit working grid.

use darkroom::blur::{filter_separable, sampled_gaussian};
use darkroom::grey_image::GreyImage;
use darkroom::hessian::{Hessian, normalised_gradient};
use darkroom::sampling::bilinear;

use super::cost_map::percentile;

pub(crate) struct WallEvidence {
    pub strength: Vec<f32>,
    pub half_width: Vec<f32>,
}

// A wide side-film torso usually lies anterior to the paired cortical
// ridges. Require agreement over the central half of the image and a
// substantial physical displacement; a crop or a symmetric view abstains.
fn tissue_side(image: &GreyImage<f32>, response: &[f32], spacing: f64) -> Option<bool> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let background = percentile(image.pixels(), 0.1);
    let threshold = percentile(response, 0.75).max(1e-6);
    let mut offsets = Vec::new();
    for y in h / 4..3 * h / 4 {
        let row = &response[y * w..(y + 1) * w];
        let x = (0..w)
            .max_by(|&a, &b| row[a].total_cmp(&row[b]))
            .expect("nonempty row");
        if row[x] < threshold {
            continue;
        }
        let mut mass = 0.0;
        let mut moment = 0.0;
        for (j, &value) in image.pixels()[y * w..(y + 1) * w].iter().enumerate() {
            let weight = f64::from((value - background).max(0.0));
            mass += weight;
            moment += weight * j as f64;
        }
        if mass > 1e-6 {
            offsets.push(((moment / mass - x as f64) * spacing) as f32);
        }
    }
    if offsets.len() < (h / 4).max(1) {
        return None;
    }
    let median = percentile(&offsets, 0.5);
    if median.abs() < 10.0 {
        return None;
    }
    let agreement = offsets.iter().filter(|&&v| v * median > 0.0).count();
    (agreement * 5 >= offsets.len() * 4).then_some(median > 0.0)
}

pub(crate) fn detect(
    image: &GreyImage<f32>,
    spacing: f64,
    size: f64,
    anterior_right: Option<bool>,
) -> WallEvidence {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let gradient = normalised_gradient(image, 1.2 / spacing).expect("bounded wall scale");
    let hessian = Hessian::at_scale(image, 1.2 / spacing).expect("bounded wall scale");
    let rim_values = hessian
        .xx
        .pixels()
        .iter()
        .zip(gradient.x.pixels())
        .map(|(&xx, &gx)| {
            let line = (-xx).max(0.0);
            if line > 1e-6 {
                line * (1.0 - gx.abs() / (2.0 * line)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect();
    let rims = GreyImage::new(image.width(), image.height(), rim_values, image.placement())
        .expect("same grid");
    let pooled_rims = filter_separable(
        &rims,
        &[1.0],
        &sampled_gaussian(2.0 / spacing).expect("bounded scale"),
    )
    .expect("odd kernels");
    let intensity = filter_separable(
        image,
        &[1.0],
        &sampled_gaussian(2.0 / spacing).expect("bounded scale"),
    )
    .expect("odd kernels");
    let mut strength = vec![0.0f32; w * h];
    let mut half_width = vec![12.0f32; w * h];
    // Broad body-depth support, with soft tails outside 16–35 mm.
    let widths = (4..=22).map(|i| 2.0 * f64::from(i)).collect::<Vec<_>>();
    let body_scale = size.clamp(0.9, 1.15);
    for y in 0..h {
        for x in 0..w {
            let sample = |map: &GreyImage<f32>, offset: f64| {
                f64::from(bilinear(map, x as f64 + offset / spacing, y as f64).unwrap_or(0.0))
            };
            for &width in &widths {
                let width = width * body_scale;
                let half = 0.5 * width;
                let left = sample(&pooled_rims, -half).max(0.0);
                let right = sample(&pooled_rims, half).max(0.0);
                if left <= 0.0 || right <= 0.0 {
                    continue;
                }
                let background =
                    0.5 * (sample(&intensity, -half - 3.0) + sample(&intensity, half + 3.0));
                let rim = 0.5 * (sample(&intensity, -half) + sample(&intensity, half)) - background;
                let middle = (sample(&intensity, -0.2 * width)
                    + sample(&intensity, 0.0)
                    + sample(&intensity, 0.2 * width))
                    / 3.0
                    - background;
                let fill = if rim > 1e-5 {
                    (middle / rim).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let width_prior =
                    (width / (16.0 * body_scale)).min(1.0) * (35.0 * body_scale / width).min(1.0);
                let score = libm::sqrt(left * right) * (0.5 + 0.5 * fill) * width_prior;
                if score > f64::from(strength[y * w + x]) {
                    strength[y * w + x] = score as f32;
                    half_width[y * w + x] = half as f32;
                }
            }
        }
    }
    let response = GreyImage::new(image.width(), image.height(), strength, image.placement())
        .expect("same grid");
    let pooled = filter_separable(
        &response,
        &sampled_gaussian(1.0 / spacing).expect("bounded scale"),
        &sampled_gaussian(5.0 / spacing).expect("bounded scale"),
    )
    .expect("odd kernels");
    let mut strength = pooled.pixels().to_vec();
    // A posterior process and the posterior body wall can form a second
    // plausible pair. Supplied facing wins; a consistent visible torso can
    // supply an image-only preference when the geometry is unconfirmed.
    if let Some(right) = anterior_right.or_else(|| tissue_side(image, &strength, spacing)) {
        let original = strength.clone();
        let near = (8.0 / spacing).ceil() as usize;
        let far = (40.0 / spacing).ceil() as usize;
        for y in 0..h {
            for x in 0..w {
                let mut ahead = 0.0f32;
                for offset in near..=far {
                    let candidate = if right {
                        x.checked_add(offset).filter(|&j| j < w)
                    } else {
                        x.checked_sub(offset)
                    };
                    if let Some(j) = candidate {
                        ahead = ahead.max(original[y * w + j]);
                    }
                }
                strength[y * w + x] = (original[y * w + x] - 0.9 * ahead).max(0.0);
            }
        }
    }
    // Soft saturation preserves the position of a strong response peak.
    // Hard clipping otherwise turns the best body centres into plateaux.
    let scale = 0.25 * percentile(&strength, 0.99).max(1e-6);
    for value in &mut strength {
        *value /= *value + scale;
    }
    WallEvidence {
        strength,
        half_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;

    fn cortical_bodies() -> GreyImage<f32> {
        let mut pixels = Vec::new();
        for y in 0..180 {
            for x in 0..180 {
                let x = f64::from(x);
                let broad = 0.5 * libm::exp(-0.5 * (x - 125.0) * (x - 125.0) / 1600.0);
                let bone = if (4..26).contains(&(y % 30)) {
                    let left = libm::exp(-0.5 * (x - 55.0) * (x - 55.0));
                    let right = libm::exp(-0.5 * (x - 85.0) * (x - 85.0));
                    let fill = if (55.0..=85.0).contains(&x) { 0.1 } else { 0.0 };
                    // A posterior process is brighter than either body wall.
                    let process = 0.7 * libm::exp(-0.5 * (x - 35.0) * (x - 35.0) / 4.0);
                    fill + 0.3 * (left + right) + process
                } else {
                    0.0
                };
                pixels.push((broad + bone) as f32);
            }
        }
        GreyImage::new(180, 180, pixels, Placement::FILM).expect("analytic image")
    }

    #[test]
    fn paired_cortices_locate_bodies_despite_broad_tissue_and_posterior_processes() {
        let image = cortical_bodies();
        let walls = detect(&image, 1.0, 1.0, Some(true));
        for y in [15, 45, 75, 105, 135, 165] {
            let x = (20..150)
                .max_by(|&a, &b| {
                    walls.strength[y * 180 + a].total_cmp(&walls.strength[y * 180 + b])
                })
                .expect("candidate centres");
            // Cortices at 55 and 85 independently define the body centre 70.
            // A 1 mm grid and smoothed pair response allow 2 mm localisation.
            assert!((x as f64 - 70.0).abs() <= 2.0, "centre at {x}");
        }
    }

    #[test]
    fn reversing_anterior_direction_mirrors_the_measured_wall_evidence() {
        let image = cortical_bodies();
        let mirrored = GreyImage::new(
            180,
            180,
            image
                .pixels()
                .as_chunks::<180>()
                .0
                .iter()
                .flat_map(|row| row.iter().rev().copied())
                .collect(),
            Placement::FILM,
        )
        .expect("mirrored image");
        for facing in [Some(true), None] {
            let right = detect(&image, 1.0, 1.0, facing);
            let left = detect(&mirrored, 1.0, 1.0, facing.map(|value| !value));
            for y in 0..180 {
                for x in 0..180 {
                    // Only floating summation order changes under reflection.
                    assert!(
                        (right.strength[y * 180 + x] - left.strength[y * 180 + 179 - x]).abs()
                            < 1e-5
                    );
                }
            }
        }
    }
    #[test]
    fn a_symmetric_crop_abstains_from_a_torso_side_preference() {
        let image = GreyImage::new(180, 180, vec![0.5; 180 * 180], Placement::FILM).expect("crop");
        let mut response = vec![0.0; 180 * 180];
        for y in 0..180 {
            response[y * 180 + 90] = 1.0;
        }
        assert_eq!(tissue_side(&image, &response, 1.0), None);
    }

    #[test]
    fn inconsistent_torso_sides_do_not_override_unknown_facing() {
        let mut pixels = Vec::new();
        let mut response = vec![0.0; 180 * 180];
        for y in 0..180 {
            let centre = if y < 90 { 45.0 } else { 135.0 };
            for x in 0..180 {
                let d = f64::from(x) - centre;
                pixels.push(libm::exp(-0.5 * d * d / 400.0) as f32);
            }
            response[y * 180 + 90] = 1.0;
        }
        let image =
            GreyImage::new(180, 180, pixels, Placement::FILM).expect("opposing tissue masses");
        assert_eq!(tissue_side(&image, &response, 1.0), None);
    }
}
