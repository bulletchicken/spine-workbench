//! Shutters and straight collimation bands, used only for statistics.

use super::percentile;
use darkroom::grey_image::GreyImage;
use shapes::film::{KeptTags, Shutter};
use shapes::pixels::{Px, PxRect};

pub(crate) fn shutter_mask(image: &GreyImage<f32>, tags: Option<&KeptTags>) -> Vec<bool> {
    let shutters = tags.and_then(|t| t.shutters.as_deref()).unwrap_or(&[]);
    mask_shutters(image, shutters)
}

fn mask_shutters(image: &GreyImage<f32>, shutters: &[Shutter]) -> Vec<bool> {
    if shutters.is_empty() {
        return vec![true; image.pixels().len()];
    }
    let placement = image.placement();
    let half_x = (placement.step_across.x().abs() + placement.step_down.x().abs()) * 0.5;
    let half_y = (placement.step_across.y().abs() + placement.step_down.y().abs()) * 0.5;
    (0..image.height())
        .flat_map(|y| {
            (0..image.width()).map(move |x| {
                let at = image.film_point(f64::from(x), f64::from(y));
                // A coarse cell is usable only if every source pixel that
                // contributed to it is inside every shutter. This prevents
                // bright excluded pixels leaking across a downsampled edge.
                let left = libm::floor(at.x() - half_x + 0.5) as i64;
                let right = libm::ceil(at.x() + half_x + 0.5) as i64;
                let top = libm::floor(at.y() - half_y + 0.5) as i64;
                let bottom = libm::ceil(at.y() + half_y + 0.5) as i64;
                (top..bottom).all(|y| {
                    (left..right).all(|x| {
                        shutters
                            .iter()
                            .all(|s| inside(s, Px::new(x as f64, y as f64)))
                    })
                })
            })
        })
        .collect()
}

fn inside(shutter: &Shutter, p: Px) -> bool {
    // DICOM shutter coordinates are one-based row and column numbers.
    let (x, y) = (p.x() + 1.0, p.y() + 1.0);
    match shutter {
        Shutter::Rectangle {
            left,
            right,
            upper,
            lower,
        } => {
            x >= f64::from(*left)
                && x <= f64::from(*right)
                && y >= f64::from(*upper)
                && y <= f64::from(*lower)
        }
        Shutter::Circle {
            centre_row,
            centre_column,
            radius,
        } => {
            radius.is_finite()
                && *radius >= 0.0
                && (x - centre_column) * (x - centre_column) + (y - centre_row) * (y - centre_row)
                    <= radius * radius
        }
        Shutter::Polygon { vertices } => {
            let mut in_polygon = false;
            if vertices.len() < 3 {
                return false;
            }
            for i in 0..vertices.len() {
                let [ay, ax] = vertices[i];
                let [by, bx] = vertices[(i + 1) % vertices.len()];
                let cross = (x - ax) * (by - ay) - (y - ay) * (bx - ax);
                if cross.abs() < 1e-9
                    && x >= ax.min(bx)
                    && x <= ax.max(bx)
                    && y >= ay.min(by)
                    && y <= ay.max(by)
                {
                    return true;
                }
                if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
                    in_polygon = !in_polygon;
                }
            }
            in_polygon
        }
    }
}

pub(crate) fn collimation(image: &GreyImage<f32>, valid: &[bool]) -> Vec<PxRect> {
    let mut sorted: Vec<f32> = image
        .pixels()
        .iter()
        .zip(valid)
        .filter_map(|(&v, &keep)| keep.then_some(v))
        .collect();
    sorted.sort_by(f32::total_cmp);
    let range = percentile(&sorted, 0.99) - percentile(&sorted, 0.01);
    if range <= 0.0 {
        return Vec::new();
    }
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut bands = Vec::new();
    for side in 0..4 {
        let vertical = side % 2 == 0;
        let length = if vertical { w } else { h };
        let across = if vertical { h } else { w };
        let mut lines = Vec::new();
        for i in 0..length {
            let inward = if side < 2 { i } else { length - 1 - i };
            let mut line = Vec::new();
            for j in across / 4..3 * across / 4 {
                let index = if vertical {
                    j * w + inward
                } else {
                    inward * w + j
                };
                if valid[index] {
                    line.push(image.pixels()[index]);
                }
            }
            // A shutter already excludes this side: do not infer another
            // border from pixels outside its field.
            if line.len() < (across / 4).max(1) {
                break;
            }
            line.sort_by(f32::total_cmp);
            let centre = percentile(&line, 0.5);
            let mut deviations: Vec<f32> = line.iter().map(|v| (v - centre).abs()).collect();
            deviations.sort_by(f32::total_cmp);
            lines.push((centre, percentile(&deviations, 0.5)));
        }
        let Some(&(edge, _)) = lines.first() else {
            continue;
        };
        let n = lines
            .iter()
            .take_while(|&&(v, mad)| (v - edge).abs() < 0.03 * range && mad < 0.02 * range)
            .count();
        if n < 2 || n as f64 > 0.30 * length as f64 {
            continue;
        }
        if !lines
            .iter()
            .skip(n)
            .take(3)
            .any(|&(v, _)| (v - edge).abs() > 0.10 * range)
        {
            continue;
        }
        let (x0, y0, x1, y1) = match side {
            0 => (-0.5, -0.5, n as f64 - 0.5, h as f64 - 0.5),
            1 => (-0.5, -0.5, w as f64 - 0.5, n as f64 - 0.5),
            2 => ((w - n) as f64 - 0.5, -0.5, w as f64 - 0.5, h as f64 - 0.5),
            _ => (-0.5, (h - n) as f64 - 0.5, w as f64 - 0.5, h as f64 - 0.5),
        };
        let a = image.film_point(x0, y0);
        let b = image.film_point(x1, y1);
        bands.push(PxRect {
            left: a.x(),
            top: a.y(),
            right: b.x(),
            bottom: b.y(),
        });
    }
    bands
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkroom::grey_image::Placement;

    #[test]
    fn shutter_numbers_are_one_based_and_shapes_intersect() {
        let rectangle = Shutter::Rectangle {
            left: 2,
            right: 4,
            upper: 3,
            lower: 5,
        };
        assert!(inside(&rectangle, Px::new(1.0, 2.0)));
        assert!(!inside(&rectangle, Px::new(0.0, 2.0)));
        let circle = Shutter::Circle {
            centre_row: 4.0,
            centre_column: 3.0,
            radius: 1.0,
        };
        assert!(inside(&circle, Px::new(2.0, 4.0)));
        assert!(!inside(&circle, Px::new(3.0, 4.0)));
        let polygon = Shutter::Polygon {
            vertices: vec![[1.0, 1.0], [1.0, 5.0], [5.0, 1.0]],
        };
        assert!(inside(&polygon, Px::new(2.0, 2.0)));
        assert!(!inside(&polygon, Px::new(3.0, 3.0)));
    }

    #[test]
    fn a_shrunk_cell_must_not_include_any_pixel_outside_the_shutter() {
        let image = GreyImage::new(
            2,
            1,
            vec![100.0, 1.0],
            Placement {
                origin: Px::new(3.5, 3.5),
                step_across: Px::new(8.0, 0.0),
                step_down: Px::new(0.0, 8.0),
            },
        )
        .unwrap();
        // The first box holds original x=0..7. Its centre is inside the
        // shutter (x>=2), but pixels x=0 and x=1 contaminate its average.
        let shutters = [Shutter::Rectangle {
            left: 3,
            right: 16,
            upper: 1,
            lower: 8,
        }];
        assert_eq!(mask_shutters(&image, &shutters), vec![false, true]);
    }

    #[test]
    fn a_flat_edge_with_an_abrupt_step_is_excluded_without_changing_pixels() {
        let image = GreyImage::new(
            40,
            80,
            (0..3200)
                .map(|i| {
                    if i % 40 < 4 {
                        0.0
                    } else {
                        0.5 + 0.25 * ((i / 40) % 3) as f32
                    }
                })
                .collect(),
            Placement::FILM,
        )
        .unwrap();
        let bands = collimation(&image, &vec![true; 3200]);
        assert_eq!(
            bands,
            vec![PxRect {
                left: -0.5,
                top: -0.5,
                right: 3.5,
                bottom: 79.5
            }]
        );
    }
}
