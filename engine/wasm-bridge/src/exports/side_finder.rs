//! `FilmDesk.start_trained_side_pass(film, first_vertebra_id)` and the
//! `SidePass` stepper.

use crate::{crossing, faults, film_desk::FilmDesk};
use ::shapes::film::Film;
use ::trained_finder::side;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// A trained pass retains the desk's pixels and leaves its cache alone:
/// the adjusting tools read the film pass's maps, which this pass does not
/// make.
#[wasm_bindgen]
pub struct SidePass {
    inner: side::SidePass,
}

#[wasm_bindgen]
impl FilmDesk {
    /// Starts the trained finder on this side film, read through its
    /// record, without doing any image work. New vertebrae take ids from
    /// `first_vertebra_id` upwards.
    pub fn start_trained_side_pass(
        &self,
        film: JsValue,
        first_vertebra_id: u32,
    ) -> Result<SidePass, JsValue> {
        let film: Film = crossing::from_js(film)?;
        let inner = side::SidePass::new(Rc::clone(&self.film), film, first_vertebra_id)
            .map_err(faults::thrown)?;
        Ok(SidePass { inner })
    }
}

#[wasm_bindgen]
impl SidePass {
    /// Advances a bounded number of engine work units.
    pub fn advance(&mut self, work_units: u32) -> Result<JsValue, JsValue> {
        let step = self.inner.advance(work_units).map_err(faults::thrown)?;
        crossing::to_js(&step)
    }

    /// Consumes the pass; a worker must not free it again after this call.
    pub fn finish(self) -> Result<JsValue, JsValue> {
        let result = self.inner.result().map_err(faults::thrown)?;
        crossing::to_js(&result)
    }
}
