//! `FilmDesk.sense_geometry`, `scale_from_ruler` and `scale_from_known_object`.

use crate::{crossing, faults, film_desk::FilmDesk};
use ::shapes::pixels::Px;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl FilmDesk {
    /// The film's geometry proposals, kept for repeat calls on this desk.
    /// Nothing here confirms or applies a proposal.
    pub fn sense_geometry(&self) -> Result<JsValue, JsValue> {
        if let Some(guess) = self.cache.borrow().geometry.as_ref() {
            return crossing::to_js(guess);
        }
        let guess = ::film_geometry::sense_geometry(&self.film).map_err(faults::thrown)?;
        let result = crossing::to_js(&guess)?;
        self.cache.borrow_mut().geometry = Some(guess);
        Ok(result)
    }

    /// A scale proposal from two original-film points and their distance in mm.
    pub fn scale_from_ruler(&self, a: JsValue, b: JsValue, mm: f64) -> Result<JsValue, JsValue> {
        let a: Px = crossing::from_js(a)?;
        let b: Px = crossing::from_js(b)?;
        let scale =
            ::film_geometry::scale_from_ruler(&self.film, a, b, mm).map_err(faults::thrown)?;
        crossing::to_js(&scale)
    }

    /// A fitted circle's scale, starting from a centre-to-rim drag and diameter.
    pub fn scale_from_known_object(
        &self,
        near: JsValue,
        edge: JsValue,
        diameter_mm: f64,
    ) -> Result<JsValue, JsValue> {
        let near: Px = crossing::from_js(near)?;
        let edge: Px = crossing::from_js(edge)?;
        let scale = ::film_geometry::scale_from_known_object(&self.film, near, edge, diameter_mm)
            .map_err(faults::thrown)?;
        crossing::to_js(&scale)
    }
}
