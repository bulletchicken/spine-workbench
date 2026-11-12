//! `FilmDesk.start_trained_pass(film, first_vertebra_id)` and the
//! `TrainedPass` stepper.

use crate::{crossing, faults, film_desk::FilmDesk};
use ::shapes::film::Film;
use ::trained_finder::watch;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// A trained pass retains the desk's pixels and leaves its cache alone:
/// the adjusting tools read the film pass's maps, which this pass does not
/// make.
#[wasm_bindgen]
pub struct TrainedPass {
    inner: watch::TrainedPass,
}

#[wasm_bindgen]
impl FilmDesk {
    /// Starts the trained finder on this front film, read through its
    /// record, without doing any image work. New vertebrae take ids from
    /// `first_vertebra_id` upwards.
    pub fn start_trained_pass(
        &self,
        film: JsValue,
        first_vertebra_id: u32,
    ) -> Result<TrainedPass, JsValue> {
        let film: Film = crossing::from_js(film)?;
        let inner = watch::TrainedPass::new(Rc::clone(&self.film), film, first_vertebra_id)
            .map_err(faults::thrown)?;
        Ok(TrainedPass { inner })
    }
}

#[wasm_bindgen]
impl TrainedPass {
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
