//! Parametric L and R shapes, packed for exact binary correlation.

use shapes::film_geometry::MarkerLetter;

pub(super) const SIDE: usize = 32;
pub(super) type Bits = [u64; 16];

pub(super) struct Template {
    pub bits: Bits,
    pub letter: MarkerLetter,
    pub mirrored: bool,
    pub rotation: u32,
}

pub(super) fn templates() -> &'static [Template] {
    static TEMPLATES: std::sync::OnceLock<Vec<Template>> = std::sync::OnceLock::new();
    TEMPLATES.get_or_init(build_templates)
}

fn build_templates() -> Vec<Template> {
    let mut result = Vec::with_capacity(1728);
    for letter in [MarkerLetter::L, MarkerLetter::R] {
        for stroke in [0.12, 0.18, 0.24] {
            for serif in [false, true] {
                for mirrored in [false, true] {
                    for rotation in (0..360).step_by(5) {
                        result.push(Template {
                            bits: draw(letter, stroke, serif, mirrored, rotation),
                            letter,
                            mirrored,
                            rotation,
                        });
                    }
                }
            }
        }
    }
    result
}

fn ink(letter: MarkerLetter, x: f64, y: f64, stroke: f64, serif: bool) -> bool {
    if !(0.0..=0.72).contains(&x) || !(0.0..=1.0).contains(&y) {
        return false;
    }
    let upright = x <= stroke;
    let slabs = serif
        && ((y < stroke * 0.55 && x < stroke * 2.0)
            || (y > 1.0 - stroke * 0.55 && x < stroke * 2.0));
    match letter {
        MarkerLetter::L => upright || y >= 1.0 - stroke || slabs,
        MarkerLetter::R => {
            // A rectangular left half and a curved right half of the bowl.
            let ring = if x <= 0.42 {
                y <= stroke || (y - 0.48).abs() <= stroke * 0.5
            } else {
                let d = ((x - 0.42) * (x - 0.42) + (y - 0.24) * (y - 0.24)).sqrt();
                (d - 0.24).abs() <= stroke * 0.5
            };
            let leg = y >= 0.46 && (x - (stroke + (y - 0.46) * 0.95)).abs() < stroke * 0.60;
            upright || (y <= 0.56 && ring) || leg || slabs
        }
    }
}

fn draw(letter: MarkerLetter, stroke: f64, serif: bool, mirrored: bool, rotation: u32) -> Bits {
    // Render first, then put the rotated ink's bounding box into exactly
    // the same square frame used for a measured component.
    const GRID: usize = 80;
    let angle = f64::from(rotation) * core::f64::consts::PI / 180.0;
    let (sin, cos) = (libm::sin(angle), libm::cos(angle));
    let mut raster = [false; GRID * GRID];
    let (mut left, mut top, mut right, mut bottom) = (GRID, GRID, 0, 0);
    for y in 0..GRID {
        for x in 0..GRID {
            let (u, v) = ((x as f64 + 0.5) / 40.0 - 1.0, (y as f64 + 0.5) / 40.0 - 1.0);
            let mut a = cos * u + sin * v;
            let b = -sin * u + cos * v;
            if mirrored {
                a = -a;
            }
            let across = if letter == MarkerLetter::R {
                (a + 0.36) / 0.92
            } else {
                a + 0.36
            };
            if ink(letter, across, b + 0.5, stroke, serif) {
                raster[y * GRID + x] = true;
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }
    let side = 1.25 * ((right - left + 1).max(bottom - top + 1)) as f64;
    let cx = (left + right + 1) as f64 * 0.5;
    let cy = (top + bottom + 1) as f64 * 0.5;
    area_bits(
        |x, y| {
            x >= 0
                && y >= 0
                && x < GRID as i32
                && y < GRID as i32
                && raster[y as usize * GRID + x as usize]
        },
        cx - side * 0.5,
        cy - side * 0.5,
        side,
    )
}

/// Exact pixel-area coverage in each cell of the square comparison grid.
pub(super) fn area_bits(inside: impl Fn(i32, i32) -> bool, left: f64, top: f64, side: f64) -> Bits {
    let mut bits = [0; 16];
    let step = side / SIDE as f64;
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (x0, y0) = (left + x as f64 * step, top + y as f64 * step);
            let (x1, y1) = (x0 + step, y0 + step);
            let mut covered = 0.0;
            for row in libm::floor(y0) as i32..libm::ceil(y1) as i32 {
                let height = (y1.min(f64::from(row + 1)) - y0.max(f64::from(row))).max(0.0);
                for col in libm::floor(x0) as i32..libm::ceil(x1) as i32 {
                    if inside(col, row) {
                        let width = (x1.min(f64::from(col + 1)) - x0.max(f64::from(col))).max(0.0);
                        covered += width * height;
                    }
                }
            }
            if covered >= 0.5 * step * step {
                let index = y * SIDE + x;
                bits[index / 64] |= 1 << (index % 64);
            }
        }
    }
    bits
}

pub(super) fn score(a: &Bits, b: &Bits) -> f64 {
    let different: u32 = a.iter().zip(b).map(|(a, b)| (a ^ b).count_ones()).sum();
    1.0 - 2.0 * f64::from(different) / (SIDE * SIDE) as f64
}
