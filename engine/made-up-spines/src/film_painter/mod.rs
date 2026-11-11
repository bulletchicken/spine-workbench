//! Paints a film: a camera, the spine, its surroundings and a detector. The
//! painter draws bodies with its own shape, deliberately not the solver's.
//!
//! A [`Canvas`] holds each pixel's line integral: the sum over every solid
//! the pixel's ray crosses of the length inside it times what a millimetre
//! of it attenuates. The detector then turns the integrals into grey
//! levels. What is painted grows with the tier:
//!
//! | Tier | Adds |
//! |---|---|
//! | t0 | the spine alone (bodies, pedicles, laminae, processes), a clean drawing |
//! | t1 | soft tissue, lungs, heart, bowel gas |
//! | t2 | ribs, shoulders, the skull and jaw, the pelvis and femoral heads, the lead letter |
//! | t3 | a real detector (scatter, noise, blur, processing); a ruler and a calibration ball, each on half the films |
//! | t4 | low dose and heavy scatter, collimation, flipped brightness on some films, 8-bit lossy films on some |

pub mod bodies;
pub mod detector;
pub mod markers;
pub mod solids;
pub mod surroundings;

use crate::film_layout::FilmLayout;
use crate::patient::Patient;
use crate::recipes::FilmRecipe;
use crate::seeded::Stream;
use crate::stack_from_l5::StackedSpine;
use darkroom::grey_image::{GreyImage, Placement};
use markers::{Ball, LeadLetter, RulerMarks};
use nalgebra::Vector3;
use shapes::film::FilmKind;
use shapes::levels::{Level, Region};
use shapes::made_up::Tier;
use shapes::pixels::{Px, PxRect};
use solids::{Ray, RayGrid, Solid};

/// What a millimetre of each tissue adds to the line integral, over soft
/// tissue, at an effective beam energy near 60 keV: water 0.0206 /mm and
/// cortical bone 0.0604 /mm (NIST XCOM mass attenuation, ICRU-44 bone at
/// 1.92 g/cm³), trabecular bone a fifth of the way from water to cortex.
pub const CORTEX: f64 = 0.0604 - 0.0206;
pub const TRABECULAR: f64 = 0.2 * CORTEX;
pub const SOFT_TISSUE: f64 = 0.0206;

/// Each pixel's line integral, on one film.
pub struct Canvas<'a> {
    pub layout: &'a FilmLayout,
    /// Row-major, as the film is stored.
    pub path: Vec<f32>,
    pub width: usize,
    pub height: usize,
    grid: RayGrid,
}

impl<'a> Canvas<'a> {
    pub fn new(layout: &'a FilmLayout) -> Self {
        let [width, height] = layout.size.map(|side| side as usize);
        let origin = layout.ray_point(0.0, 0.0);
        Canvas {
            layout,
            path: vec![0.0; width * height],
            width,
            height,
            grid: RayGrid {
                origin,
                across: layout.ray_point(1.0, 0.0) - origin,
                down: layout.ray_point(0.0, 1.0) - origin,
                direction: layout.beam(),
            },
        }
    }

    /// The ray through film pixel `(x, y)`.
    pub fn ray(&self, x: usize, y: usize) -> Ray {
        let (x, y) = (x as f64, y as f64);
        Ray {
            origin: self.grid.origin + self.grid.across * x + self.grid.down * y,
            direction: self.grid.direction,
        }
    }

    /// Adds `mu` times each ray's length inside `solid`, over the pixels
    /// its box covers.
    pub fn add(&mut self, solid: &Solid, mu: f64) {
        let corners = solid.corners().map(|corner| self.layout.project(&corner));
        let Some([x0, y0, x1, y1]) = self.pixel_box(&corners) else {
            return;
        };
        let prepared = solid.prepare(&self.grid);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let chord = prepared.chord(x as f64, y as f64);
                if chord > 0.0 {
                    self.path[y * self.width + x] += (mu * chord) as f32;
                }
            }
        }
    }

    pub fn add_all(&mut self, solids: &[(Solid, f64)]) {
        for (solid, mu) in solids {
            self.add(solid, *mu);
        }
    }

    /// Adds, at every pixel, what `along` gives for the pixel's ray start
    /// across the beam (its patient x and y) and its row of the upright
    /// film, whose height `z` is the same all along it: the caller works
    /// out what a row needs once, in `row`.
    pub fn add_by_row<R>(&mut self, row: impl Fn(f64) -> R, along: impl Fn(&R, f64, f64) -> f64) {
        let layout = self.layout;
        let rows: Vec<R> = (0..layout.upright_size[1])
            .map(|v| {
                let [_, sv] = layout.camera.upright_mm_per_px;
                row((layout.camera.origin.y - f64::from(v)) * sv)
            })
            .collect();
        let (o, a, d) = (self.grid.origin, self.grid.across, self.grid.down);
        for y in 0..self.height {
            for x in 0..self.width {
                let (_, v) = layout.film_to_upright(x as f64, y as f64);
                let (fx, fy) = (x as f64, y as f64);
                let px = o.x + fx * a.x + fy * d.x;
                let py = o.y + fx * a.y + fy * d.y;
                let value = along(&rows[v as usize], px, py);
                self.path[y * self.width + x] += value as f32;
            }
        }
    }

    /// Adds something lying flat on the detector: `at(u, v)` in upright mm
    /// from the film's top-left pixel edge, averaged over nine points in
    /// each pixel, over the pixels of the upright box `[left, top, right,
    /// bottom]` (mm).
    pub fn add_flat(&mut self, upright_mm: [f64; 4], at: impl Fn(f64, f64) -> f64) {
        let layout = self.layout;
        let mm = layout.camera.upright_mm_per_px[0];
        let [left, top, right, bottom] = upright_mm;
        let corners = [
            layout.upright_to_film(left / mm - 0.5, top / mm - 0.5),
            layout.upright_to_film(right / mm - 0.5, bottom / mm - 0.5),
        ];
        let Some([x0, y0, x1, y1]) = self.pixel_box(&corners) else {
            return;
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (u, v) = layout.film_to_upright(x as f64, y as f64);
                let mut sum = 0.0;
                for i in 0..3 {
                    for j in 0..3 {
                        let du = (f64::from(i) - 1.0) / 3.0;
                        let dv = (f64::from(j) - 1.0) / 3.0;
                        sum += at((u + 0.5 + du) * mm, (v + 0.5 + dv) * mm);
                    }
                }
                self.path[y * self.width + x] += (sum / 9.0) as f32;
            }
        }
    }

    /// The film pixels covering the box of `points`, one pixel of margin
    /// round it, cut to the film; `None` when it misses the film.
    fn pixel_box(&self, points: &[Px]) -> Option<[usize; 4]> {
        let (mut low_x, mut low_y) = (f64::INFINITY, f64::INFINITY);
        let (mut high_x, mut high_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in points {
            low_x = low_x.min(p.x());
            low_y = low_y.min(p.y());
            high_x = high_x.max(p.x());
            high_y = high_y.max(p.y());
        }
        let (w, h) = (self.width as f64, self.height as f64);
        if !(high_x >= -1.0 && high_y >= -1.0 && low_x <= w && low_y <= h) {
            return None;
        }
        let low = |v: f64| (v.floor() - 1.0).max(0.0) as usize;
        let high = |v: f64, limit: usize| ((v.ceil() + 1.0).max(0.0) as usize).min(limit - 1);
        Some([
            low(low_x),
            low(low_y),
            high(high_x, self.width),
            high(high_y, self.height),
        ])
    }
}

/// One painted film and everything placed on it besides the patient.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintedFilm {
    pub kind: FilmKind,
    /// Grey levels as stored, `0 ..= 2^bits_stored − 1`.
    pub image: GreyImage<u16>,
    pub bits_stored: u8,
    /// Bone dark instead of bright.
    pub inverted: bool,
    pub lead_letter: Option<LeadLetter>,
    pub ruler: Option<RulerMarks>,
    pub ball: Option<Ball>,
    /// The collimated field, when the film is collimated.
    pub field: Option<PxRect>,
}

/// Paints one film of `patient`, whose spine on this film is `spine`.
pub fn paint(
    recipe: &FilmRecipe,
    layout: &FilmLayout,
    patient: &Patient,
    spine: &StackedSpine,
    seed: u64,
) -> PaintedFilm {
    let kind = recipe.kind.name();
    let mut canvas = Canvas::new(layout);
    let k = spine.scale;
    for (index, vertebra) in spine.vertebrae.iter().enumerate() {
        canvas.add_all(&bodies::vertebra_solids(
            vertebra,
            spine.levels[index],
            &spine.sizes[index],
            k,
        ));
    }
    let tier = recipe.tier;
    if tier >= Tier::T1 {
        let body = surroundings::Body::new(spine, &patient.recipe);
        surroundings::paint_soft_tissue(
            &mut canvas,
            &body,
            spine,
            &mut Stream::new(&format!("soft-tissue/{kind}"), seed),
        );
        if tier >= Tier::T2 {
            canvas.add_all(&surroundings::rib_solids(spine, &patient.recipe));
            canvas.add_all(&surroundings::shoulder_solids(
                spine,
                &body,
                &patient.recipe,
                recipe.kind == FilmKind::Side,
            ));
            canvas.add_all(&surroundings::skull_solids(spine));
            canvas.add_all(&surroundings::pelvis_solids(spine));
        }
    }
    let mut marks = Stream::new(&format!("markers/{kind}"), seed);
    let lead_letter = recipe.lead_marker.map(|placement| {
        markers::paint_letter(&mut canvas, &placement, recipe.collimation, &mut marks)
    });
    let ruler = recipe
        .ruler
        .map(|placement| markers::paint_ruler(&mut canvas, &placement, recipe.collimation));
    let ball = recipe
        .ball
        .map(|placement| markers::paint_ball(&mut canvas, &placement, recipe.collimation));
    let roi = layout.project(&thoracolumbar_junction(spine));
    let exposure = detector::expose(
        &canvas,
        recipe,
        roi,
        &mut Stream::new(&format!("noise/{kind}"), seed),
    );
    let [width, height] = layout.size;
    PaintedFilm {
        kind: recipe.kind,
        image: GreyImage::new(width, height, exposure.values, Placement::FILM)
            .expect("one grey level per pixel"),
        bits_stored: exposure.bits,
        inverted: recipe.inverted,
        lead_letter,
        ruler,
        ball,
        field: layout.field,
    }
}

/// Midway between the last thoracic vertebra and the first lumbar one.
fn thoracolumbar_junction(spine: &StackedSpine) -> Vector3<f64> {
    let last_thoracic = spine
        .levels
        .iter()
        .rposition(|l| l.region() == Region::Thoracic);
    let first_lumbar = spine.levels.iter().position(|l| *l == Level::L1);
    match (last_thoracic, first_lumbar) {
        (Some(a), Some(b)) => 0.5 * (spine.vertebrae[a].centre + spine.vertebrae[b].centre),
        _ => spine
            .vertebrae
            .last()
            .map_or(Vector3::zeros(), |v| v.centre),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer_key::{corners_of, film_answer, spine_for};
    use crate::patient::make_up_patient;
    use crate::recipes::{BallPlacement, MarkerPlacement, RulerPlacement, film_recipes};
    use shapes::film_geometry::{ImageRight, MarkerLetter};
    use shapes::made_up::{MadeUpRecipe, RecipeName, TrueMarks};

    /// One painted film of a recipe, with its layout and the patient.
    fn painted(
        name: RecipeName,
        seed: u32,
        tier: Tier,
        kind: FilmKind,
    ) -> (PaintedFilm, FilmLayout, Patient) {
        let recipe = MadeUpRecipe { name, seed, tier };
        let patient = make_up_patient(name, seed).expect("stacks");
        let film = film_recipes(&recipe, u64::from(seed))
            .into_iter()
            .find(|f| f.kind == kind)
            .expect("the film");
        let spine = spine_for(&patient, kind).clone();
        let layout = FilmLayout::new(&film, &spine);
        let painted = paint(&film, &layout, &patient, &spine, u64::from(seed));
        (painted, layout, patient)
    }

    fn value(film: &PaintedFilm, at: Px) -> f64 {
        let (x, y) = (libm::round(at.x()) as u32, libm::round(at.y()) as u32);
        f64::from(*film.image.get(x, y).expect("on the film"))
    }

    #[test]
    fn a_clean_film_shows_each_body_where_its_true_corners_put_it() {
        let (film, layout, patient) = painted(RecipeName::Straight, 1, Tier::T0, FilmKind::Front);
        assert_eq!((film.bits_stored, film.inverted), (16, false));
        let levels = patient.recipe.template.levels();
        let truth = film_answer(&layout, &patient.standing, &levels);
        let mm = layout.geometry.mm_per_px.expect("a scale");
        for level in [Level::T6, Level::L1, Level::L3] {
            let vertebra = truth
                .vertebrae
                .iter()
                .find(|v| v.level == level)
                .expect("level");
            let TrueMarks::Front {
                sup_l,
                sup_r,
                inf_l,
                inf_r,
                ..
            } = vertebra.marks
            else {
                panic!("front marks");
            };
            // A quarter of the way up the body, clear of the pedicles and
            // processes: the body's edges lie on the lines through its
            // true corners.
            let along = |a: Px, b: Px| {
                Px::new(
                    a.x() + 0.25 * (b.x() - a.x()),
                    a.y() + 0.25 * (b.y() - a.y()),
                )
            };
            let (left, right) = (along(inf_l.at, sup_l.at), along(inf_r.at, sup_r.at));
            let row = libm::round(0.5 * (left.y() + right.y()));
            // Patient left on screen right: the left corner is the larger x.
            // Two millimetres inside an edge is bone, three outside is not.
            // A rounded body's chord falls to nothing at its outline like a
            // square root, so the outline is where the value has all but
            // reached the outside's: within 2% of the step.
            let edge = |expected: f64, outward: f64| {
                let inside = value(&film, Px::new(expected - outward * 2.0 / mm, row));
                let outside = value(&film, Px::new(expected + outward * 3.0 / mm, row));
                assert!(inside > outside + 5000.0, "{level:?}: {inside} {outside}");
                let outline_level = outside + 0.02 * (inside - outside);
                let mut x = expected - outward * 2.0 / mm;
                while value(&film, Px::new(x, row)) > outline_level {
                    x += outward;
                }
                x - 0.5 * outward
            };
            let tolerance = 0.5 / mm;
            let (found_left, found_right) = (edge(left.x(), 1.0), edge(right.x(), -1.0));
            assert!(
                (found_left - left.x()).abs() < tolerance,
                "{level:?} left {found_left} {}",
                left.x()
            );
            assert!(
                (found_right - right.x()).abs() < tolerance,
                "{level:?} right {found_right} {}",
                right.x()
            );
        }
        // The bodies are what is brightest: a vertebra's centre outshines
        // the space 60 mm to its side.
        let l3 = patient.standing.vertebra(Level::L3).expect("L3").centre;
        let beside = l3 + Vector3::new(0.0, 60.0, 0.0);
        assert!(
            value(&film, layout.project(&l3)) > value(&film, layout.project(&beside)) + 10_000.0
        );
    }

    /// A blank canvas on a front film of the straight recipe.
    fn blank(image_right: ImageRight) -> (FilmLayout, crate::recipes::FilmRecipe) {
        let recipe = MadeUpRecipe {
            name: RecipeName::Straight,
            seed: 2,
            tier: Tier::T2,
        };
        let patient = make_up_patient(recipe.name, recipe.seed).expect("stacks");
        let mut film = film_recipes(&recipe, 2).remove(0);
        film.image_right = image_right;
        (FilmLayout::new(&film, &patient.standing), film)
    }

    #[test]
    fn the_lead_letter_sits_on_its_side_and_reads_mirrored_when_the_film_is() {
        for image_right in [ImageRight::PatientLeft, ImageRight::PatientRight] {
            for seed in 0..6 {
                let (layout, _) = blank(image_right);
                let mut canvas = Canvas::new(&layout);
                let placement = MarkerPlacement {
                    height_mm: 20.0,
                    down: 0.05,
                };
                let letter = markers::paint_letter(
                    &mut canvas,
                    &placement,
                    None,
                    &mut Stream::new("letter", seed),
                );
                let centre_x = 0.5 * (letter.bounds.left + letter.bounds.right);
                let on_right = centre_x > 0.5 * f64::from(layout.size[0]);
                // An L names the patient's left, which is on screen right
                // exactly when the film faces patient left.
                let left_side_on_right = image_right == ImageRight::PatientLeft;
                assert_eq!(
                    on_right,
                    (letter.letter == MarkerLetter::L) == left_side_on_right
                );
                assert_eq!(letter.mirrored, image_right == ImageRight::PatientRight);
                // The letter's height in pixels is its 20 mm.
                let mm = layout.geometry.mm_per_px.expect("a scale");
                assert!((letter.bounds.bottom - letter.bounds.top - 20.0 / mm).abs() < 1e-6);
                // Lead fills the letter's bottom-left (an L's foot, an R's
                // leg) unless mirrored, where it is the bottom-right.
                let foot_x = if letter.mirrored {
                    letter.bounds.right - 2.0
                } else {
                    letter.bounds.left + 2.0
                };
                let at = |x: f64, y: f64| {
                    canvas.path[libm::round(y) as usize * canvas.width + libm::round(x) as usize]
                };
                assert!(at(foot_x, letter.bounds.bottom - 2.0) > 3.0);
                // The top of an L's right half is empty.
                if letter.letter == MarkerLetter::L {
                    let far_x = if letter.mirrored {
                        letter.bounds.left + 2.0
                    } else {
                        letter.bounds.right - 2.0
                    };
                    assert_eq!(at(far_x, letter.bounds.top + 2.0), 0.0);
                }
            }
        }
    }

    #[test]
    fn the_rulers_ticks_are_ten_millimetres_apart() {
        let (layout, _) = blank(ImageRight::PatientLeft);
        let mut canvas = Canvas::new(&layout);
        let ruler = markers::paint_ruler(
            &mut canvas,
            &RulerPlacement {
                length_mm: 200.0,
                on_left: true,
                down: 0.2,
            },
            None,
        );
        assert_eq!(ruler.mm, 200.0);
        let mm = layout.geometry.mm_per_px.expect("a scale");
        let length_px = libm::hypot(
            ruler.last.x() - ruler.first.x(),
            ruler.last.y() - ruler.first.y(),
        );
        assert!((length_px * mm - 200.0).abs() < 1e-6);
        // Read the ticks off the painting: centres of the runs of lead down
        // the column through the ticks.
        let x = libm::round(ruler.first.x()) as usize;
        let column: Vec<f32> = (0..canvas.height)
            .map(|y| canvas.path[y * canvas.width + x])
            .collect();
        let mut centres = Vec::new();
        let mut run: Vec<f64> = Vec::new();
        for (y, value) in column.iter().enumerate() {
            if *value > 0.0 {
                run.push(y as f64);
            } else if !run.is_empty() {
                centres.push(run.iter().sum::<f64>() / run.len() as f64);
                run.clear();
            }
        }
        assert_eq!(centres.len(), 21);
        for pair in centres.windows(2) {
            assert!(
                ((pair[1] - pair[0]) * mm - 10.0).abs() < 0.6 * mm + 0.05,
                "{pair:?}"
            );
        }
    }

    #[test]
    fn the_ball_is_as_wide_as_its_diameter() {
        let (layout, _) = blank(ImageRight::PatientLeft);
        let mut canvas = Canvas::new(&layout);
        let ball = markers::paint_ball(
            &mut canvas,
            &BallPlacement {
                diameter_mm: 30.0,
                across: 0.3,
                down: 0.5,
            },
            None,
        );
        let mm = layout.geometry.mm_per_px.expect("a scale");
        assert!((ball.radius_px - 15.0 / mm).abs() < 1e-12);
        // The painted disc's area, counting each pixel by how much steel it
        // holds against the centre's, is π·r².
        let full = 0.9 * 30.0;
        let area: f64 = canvas
            .path
            .iter()
            .map(|&p| f64::from(p) / full)
            .filter(|&p| p > 0.0)
            .count() as f64;
        let expected = core::f64::consts::PI * ball.radius_px * ball.radius_px;
        assert!(
            (area - expected).abs() < 0.02 * expected,
            "{area} {expected}"
        );
    }

    #[test]
    fn a_hard_film_is_noisy_low_in_contrast_and_white_outside_its_field() {
        // The first seed whose front film is collimated, bone bright and
        // not lossy.
        let (seed, film) = (0..40)
            .find_map(|seed| {
                let recipe = MadeUpRecipe { name: RecipeName::RightThoracic40, seed, tier: Tier::T4 };
                let film = film_recipes(&recipe, u64::from(seed)).remove(0);
                let lossy = matches!(film.detector, crate::recipes::DetectorRecipe::Physical(p) if p.lossy_quality.is_some());
                (film.collimation.is_some() && !film.inverted && !lossy).then_some((seed, film))
            })
            .expect("some seed");
        let (painted, layout, _) =
            painted(RecipeName::RightThoracic40, seed, Tier::T4, FilmKind::Front);
        let top = f64::from((1u32 << painted.bits_stored) - 1);
        let field = layout.field.expect("collimated");
        assert!(film.collimation.is_some());
        // Outside the field the film was never exposed: white.
        if field.left >= 2.0 {
            assert_eq!(
                value(&painted, Px::new(0.0, 0.5 * (field.top + field.bottom))),
                top
            );
        }
        // Inside, neighbouring pixels in the soft tissue beside the spine
        // differ: the noise.
        let patient = make_up_patient(RecipeName::RightThoracic40, seed).expect("stacks");
        let l2 = patient.standing.vertebra(Level::L2).expect("L2").centre;
        let patch = layout.project(&(l2 + Vector3::new(0.0, 70.0, 0.0)));
        let values: Vec<f64> = (0..20)
            .map(|i| value(&painted, Px::new(patch.x() + f64::from(i), patch.y())))
            .collect();
        let steps: f64 = values.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / 19.0;
        assert!(steps > 0.002 * top, "{steps}");
    }

    #[test]
    fn the_same_seed_paints_the_same_film() {
        let (first, _, _) = painted(RecipeName::Random, 9, Tier::T1, FilmKind::Side);
        let (again, _, _) = painted(RecipeName::Random, 9, Tier::T1, FilmKind::Side);
        assert_eq!(first, again);
        let (other, _, _) = painted(RecipeName::Random, 10, Tier::T1, FilmKind::Side);
        assert_ne!(first.image, other.image);
    }

    #[test]
    fn corners_stay_named_through_a_turned_film() {
        let (film, layout, patient) = painted(RecipeName::UpsideDown, 4, Tier::T0, FilmKind::Front);
        let truth = film_answer(
            &layout,
            &patient.standing,
            &patient.recipe.template.levels(),
        );
        let l1 = truth
            .vertebrae
            .iter()
            .find(|v| v.level == Level::L1)
            .expect("L1");
        // Every corner of L1 sits on bone or at its very edge, upside down
        // or not.
        let centre = corners_of(&l1.marks).iter().fold((0.0, 0.0), |acc, c| {
            (acc.0 + 0.25 * c.at.x(), acc.1 + 0.25 * c.at.y())
        });
        let background = value(&film, Px::new(2.0, 2.0));
        assert!(value(&film, Px::new(centre.0, centre.1)) > background + 10_000.0);
    }
}
