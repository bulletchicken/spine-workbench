//! Where each film's camera stands: the film's size, scale, quarter turns and
//! facing, where the patient lands on it, and the collimated field.
//!
//! The film covers the spine from above C2 to below the femoral heads, with
//! the margins its recipe drew, at its recipe's width; the patient's
//! origin lands so the spine sits near the middle across. Everything is
//! worked out on the film shown upright, then stored turned by the
//! recipe's quarter turns, as `docs/conventions/patient-frame.md` and
//! [`ParallelCamera`] define, so the truth and the painting use the very
//! same camera.
//!
//! The true geometry states its scale as a 100 mm span along the patient's
//! long axis through the origin: a parallel film has the same scale at
//! every depth, so any such span is exact.

use crate::recipes::FilmRecipe;
use crate::stack_from_l5::StackedSpine;
use forward_model::cameras::Camera;
use forward_model::cameras::parallel::ParallelCamera;
use forward_model::endplates::Endplates;
use forward_model::patient_frame::{UP, screen_right};
use nalgebra::{Vector2, Vector3};
use shapes::film::{FilmKind, View};
use shapes::film_geometry::{Confirmations, FilmGeometry, Ruler, ScaleSource};
use shapes::pixels::{Px, PxRect};

/// One film's camera and frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmLayout {
    pub kind: FilmKind,
    /// The film's true geometry, every fact confirmed.
    pub geometry: FilmGeometry,
    /// Width and height as stored.
    pub size: [u32; 2],
    /// Width and height shown upright.
    pub upright_size: [u32; 2],
    pub camera: ParallelCamera,
    /// The collimated field in film pixels, when the film is collimated.
    pub field: Option<PxRect>,
}

impl FilmLayout {
    /// The layout of the film `recipe` describes, of `spine`.
    pub fn new(recipe: &FilmRecipe, spine: &StackedSpine) -> Self {
        let mm = recipe.mm_per_px;
        let right = screen_right(recipe.image_right);
        let top = spine
            .vertebrae
            .first()
            .map_or(0.0, |v| Endplates::of(v).top.centre.z);
        let pelvis = &spine.pelvis;
        let bottom = pelvis.hips.iter().map(|h| h.z).fold(f64::MAX, f64::min) - pelvis.hip_radius;
        let z_top = top + recipe.margin_above_mm;
        let z_bottom = bottom - recipe.margin_below_mm;
        let height = libm::ceil((z_top - z_bottom) / mm).max(1.0) as u32;
        let width = libm::round(f64::from(height) * recipe.width_ratio).max(1.0) as u32;
        let across = spine
            .vertebrae
            .iter()
            .map(|v| v.centre.dot(&right))
            .sum::<f64>()
            / spine.vertebrae.len().max(1) as f64
            + recipe.across_offset_mm;
        let origin = Vector2::new(0.5 * f64::from(width - 1) - across / mm, z_top / mm);
        let turned = recipe.quarter_turns.get() % 2 == 1;
        let size = if turned {
            [height, width]
        } else {
            [width, height]
        };
        let front_view = recipe.kind.view() == View::Front;
        let mut geometry = FilmGeometry {
            mm_per_px: Some(mm),
            mm_per_px_vertical: None,
            scale: None,
            quarter_turns: recipe.quarter_turns,
            image_right: Some(recipe.image_right),
            confirmed: Confirmations {
                upright: true,
                left_right: front_view,
                facing: !front_view,
                scale: true,
                polarity: true,
                privacy: true,
            },
        };
        let camera = ParallelCamera::for_film(&geometry, size, origin)
            .expect("a positive scale and a facing always make a camera");
        let span = |p: Vector3<f64>| {
            let at = camera.project(&p);
            Px::new(at.x, at.y)
        };
        geometry.scale = Some(ScaleSource::Ruler {
            rulers: vec![Ruler {
                a: span(Vector3::zeros()),
                b: span(UP * 100.0),
                mm: 100.0,
            }],
        });
        let mut layout = FilmLayout {
            kind: recipe.kind,
            geometry,
            size,
            upright_size: [width, height],
            camera,
            field: None,
        };
        layout.field = recipe.collimation.map(|[left, top, right, bottom]| {
            let (w, h) = (f64::from(width), f64::from(height));
            let corners = [
                layout.upright_to_film(left * w - 0.5, top * h - 0.5),
                layout.upright_to_film(w * (1.0 - right) - 0.5, h * (1.0 - bottom) - 0.5),
            ];
            PxRect {
                left: corners[0].x().min(corners[1].x()),
                top: corners[0].y().min(corners[1].y()),
                right: corners[0].x().max(corners[1].x()),
                bottom: corners[0].y().max(corners[1].y()),
            }
        });
        layout
    }

    /// The patient direction pointing to screen right on the upright film.
    pub fn screen_right(&self) -> Vector3<f64> {
        screen_right(self.camera.image_right)
    }

    /// The direction the rays travel (either way along it: a parallel
    /// film's line integrals do not depend on which).
    pub fn beam(&self) -> Vector3<f64> {
        self.screen_right().cross(&UP)
    }

    /// Upright pixel `(u, v)` to film pixels.
    pub fn upright_to_film(&self, u: f64, v: f64) -> Px {
        let w = f64::from(self.size[0]) - 1.0;
        let h = f64::from(self.size[1]) - 1.0;
        let (x, y) = match self.camera.quarter_turns.get() {
            0 => (u, v),
            1 => (v, h - u),
            2 => (w - u, h - v),
            _ => (w - v, u),
        };
        Px::new(x, y)
    }

    /// Film pixel `(x, y)` to upright pixels.
    pub fn film_to_upright(&self, x: f64, y: f64) -> (f64, f64) {
        let w = f64::from(self.size[0]) - 1.0;
        let h = f64::from(self.size[1]) - 1.0;
        match self.camera.quarter_turns.get() {
            0 => (x, y),
            1 => (h - y, x),
            2 => (w - x, h - y),
            _ => (y, w - x),
        }
    }

    /// The point on the plane through the patient origin, across the beam,
    /// that lands on film pixel `(x, y)`.
    pub fn ray_point(&self, x: f64, y: f64) -> Vector3<f64> {
        let (u, v) = self.film_to_upright(x, y);
        let [su, sv] = self.camera.upright_mm_per_px;
        let origin = self.camera.origin;
        self.screen_right() * ((u - origin.x) * su) + UP * ((origin.y - v) * sv)
    }

    /// Whether a film point is on the film and inside its collimated field.
    pub fn shows(&self, at: Px) -> bool {
        let on_film = at.is_finite()
            && at.x() >= -0.5
            && at.y() >= -0.5
            && at.x() <= f64::from(self.size[0]) - 0.5
            && at.y() <= f64::from(self.size[1]) - 0.5;
        on_film
            && self.field.is_none_or(|field| {
                at.x() >= field.left
                    && at.x() <= field.right
                    && at.y() >= field.top
                    && at.y() <= field.bottom
            })
    }

    /// Where a patient point lands on this film.
    pub fn project(&self, point: &Vector3<f64>) -> Px {
        let at = self.camera.project(point);
        Px::new(at.x, at.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patient::make_up_patient;
    use crate::recipes::film_recipes;
    use shapes::film_geometry::{ImageRight, QuarterTurns};
    use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};

    fn layouts(name: RecipeName, tier: Tier) -> Vec<(FilmLayout, StackedSpine)> {
        let recipe = MadeUpRecipe {
            name,
            seed: 3,
            tier,
        };
        let patient = make_up_patient(name, 3).expect("stacks");
        film_recipes(&recipe, u64::from(recipe.seed))
            .iter()
            .map(|film| {
                (
                    FilmLayout::new(film, &patient.standing),
                    patient.standing.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn the_whole_spine_and_both_hips_land_on_every_film() {
        for (layout, spine) in layouts(RecipeName::Random, Tier::T2) {
            for v in &spine.vertebrae {
                assert!(layout.shows(layout.project(&v.centre)), "{:?}", layout.kind);
            }
            for hip in &spine.pelvis.hips {
                assert!(layout.shows(layout.project(hip)));
            }
        }
    }

    #[test]
    fn a_ray_point_lands_back_on_its_pixel_and_the_beam_does_not_move_it() {
        for name in [RecipeName::Straight, RecipeName::UpsideDown] {
            for (layout, _) in layouts(name, Tier::T0) {
                let p = layout.ray_point(123.0, 456.0);
                let back = layout.project(&(p + layout.beam() * 77.0));
                assert!((back.x() - 123.0).abs() < 1e-9 && (back.y() - 456.0).abs() < 1e-9);
                let (u, v) = layout.film_to_upright(123.0, 456.0);
                let again = layout.upright_to_film(u, v);
                assert!((again.x() - 123.0).abs() < 1e-12 && (again.y() - 456.0).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn the_true_scale_is_a_hundred_millimetre_span() {
        for (layout, _) in layouts(RecipeName::UpsideDown, Tier::T0) {
            let Some(ScaleSource::Ruler { rulers }) = &layout.geometry.scale else {
                panic!("a ruler scale");
            };
            let length = libm::hypot(
                rulers[0].a.x() - rulers[0].b.x(),
                rulers[0].a.y() - rulers[0].b.y(),
            );
            let mm = layout.geometry.mm_per_px.expect("a scale");
            assert!((length * mm - 100.0).abs() < 1e-9);
            assert_eq!(
                layout.geometry.quarter_turns,
                QuarterTurns::new(2).expect("two")
            );
        }
    }

    #[test]
    fn upright_films_put_the_head_at_the_top_and_turned_ones_swap_their_sides() {
        let (layout, spine) = &layouts(RecipeName::Straight, Tier::T0)[0];
        let head = layout.project(&spine.vertebrae[0].centre);
        let hip = layout.project(&spine.pelvis.hips[0]);
        assert!(head.y() < hip.y());
        assert_eq!(layout.geometry.image_right, Some(ImageRight::PatientLeft));
        // The patient's left hip is on screen right.
        assert!(spine.pelvis.hips[0].y > 0.0 && hip.x() > head.x());
    }

    #[test]
    fn a_collimated_field_hides_what_lies_outside_it() {
        let collimated = (0..20)
            .flat_map(|seed| {
                let recipe = MadeUpRecipe {
                    name: RecipeName::Straight,
                    seed,
                    tier: Tier::T4,
                };
                let patient = make_up_patient(RecipeName::Straight, seed).expect("stacks");
                film_recipes(&recipe, u64::from(recipe.seed))
                    .into_iter()
                    .map(move |film| FilmLayout::new(&film, &patient.standing))
            })
            .find(|layout| layout.field.is_some())
            .expect("some t4 film is collimated");
        let field = collimated.field.expect("a field");
        assert!(collimated.shows(Px::new(
            0.5 * (field.left + field.right),
            0.5 * (field.top + field.bottom)
        )));
        if field.left > 1.0 {
            assert!(!collimated.shows(Px::new(0.0, 0.5 * (field.top + field.bottom))));
        }
        assert!(!collimated.shows(Px::new(-3.0, 10.0)));
    }
}
