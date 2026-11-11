//! `FilmDesk.tool_preview`, `region_wand`, `boundary_wand` and `resolve_pins`.
//!
//! Requests carry their current marks; the desk lends pixels and cached
//! film-pass maps without storing any edit state. The frozen request and
//! desk types do not carry the user's current `FilmGeometry`, so these
//! calls use relative pixel tolerances. Geometry proposals are not a
//! substitute for the user's saved geometry.

use crate::{crossing, faults, film_desk::FilmDesk};
use ::shapes::errors::{EngineError, ErrorCode};
use ::shapes::film::FilmKind;
use ::shapes::toolbox::{BoundaryWandInput, PinInput, RegionWandInput, ToolPreviewInput};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl FilmDesk {
    /// What the selected wand would do to these marks at the pointer.
    pub fn tool_preview(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ToolPreviewInput = crossing::from_js(input)?;
        same_film(self, input.marks.film)?;
        let cache = self.cache.borrow();
        let output = ::toolbox::preview(&self.film, None, cache.film_pass.as_ref(), &input)
            .map_err(faults::thrown)?;
        crossing::to_js(&output)
    }

    /// Refit one outline, returning its growth tiles as fresh byte arrays.
    pub fn region_wand(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: RegionWandInput = crossing::from_js(input)?;
        same_film(self, input.marks.film)?;
        let cache = self.cache.borrow();
        let output =
            ::toolbox::region_wand::apply(&self.film, None, cache.film_pass.as_ref(), &input)
                .map_err(faults::thrown)?;
        crossing::to_js(&output)
    }

    /// Split or merge exactly the boundary shown by the hover preview.
    pub fn boundary_wand(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: BoundaryWandInput = crossing::from_js(input)?;
        same_film(self, input.marks.film)?;
        let cache = self.cache.borrow();
        let output =
            ::toolbox::boundary_wand::apply(&self.film, None, cache.film_pass.as_ref(), &input)
                .map_err(faults::thrown)?;
        crossing::to_js(&output)
    }

    /// Move or release a pin and solve the affected marks around it.
    pub fn resolve_pins(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: PinInput = crossing::from_js(input)?;
        same_film(self, input.marks.film)?;
        let cache = self.cache.borrow();
        let output = ::toolbox::pins::apply(&self.film, None, cache.film_pass.as_ref(), &input)
            .map_err(faults::thrown)?;
        crossing::to_js(&output)
    }
}

fn same_film(desk: &FilmDesk, film: FilmKind) -> Result<(), JsValue> {
    if film == desk.film.facts.kind {
        Ok(())
    } else {
        Err(faults::thrown(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "the tool's marks belong to another film",
        )))
    }
}
