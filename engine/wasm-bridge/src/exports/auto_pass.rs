//! `FilmDesk.start_film_pass(film)`, the `FilmPass` and `ChainPass`
//! steppers, and `FilmDesk.place_points`.

use crate::{crossing, faults, film_desk::DeskCache, film_desk::FilmDesk};
use ::auto_pass::watch::{chain_pass, film_pass};
use ::shapes::auto_pass::{ChainOptions, FilmChain, FilmPassResult};
use ::shapes::errors::{EngineError, ErrorCode};
use ::shapes::film::Film;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// A film pass retains the desk's pixels and fills its cache only on finish.
#[wasm_bindgen]
pub struct FilmPass {
    inner: film_pass::FilmPass,
    cache: Rc<RefCell<DeskCache>>,
}

#[wasm_bindgen]
impl FilmDesk {
    /// Starts the film stages using the current brightness and geometry.
    pub fn start_film_pass(&self, film: JsValue) -> Result<FilmPass, JsValue> {
        let film: Film = crossing::from_js(film)?;
        let inner =
            film_pass::FilmPass::new(Rc::clone(&self.film), film).map_err(faults::thrown)?;
        self.cache.borrow_mut().film_pass = None;
        Ok(FilmPass {
            inner,
            cache: Rc::clone(&self.cache),
        })
    }

    /// Places a solved chain using the finished pass retained on this desk.
    pub fn place_points(&self, chain: JsValue, first_vertebra_id: u32) -> Result<JsValue, JsValue> {
        let chain: FilmChain = crossing::from_js(chain)?;
        let cache = self.cache.borrow();
        let cache = cache.film_pass.as_ref().ok_or_else(|| {
            faults::thrown(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the film pass must finish before points can be placed",
            ))
        })?;
        let marks = ::auto_pass::run::place_points(cache, &chain, first_vertebra_id)
            .map_err(faults::thrown)?;
        crossing::to_js(&marks)
    }
}

#[wasm_bindgen]
impl FilmPass {
    /// Advances a bounded number of engine work units.
    pub fn advance(&mut self, work_units: u32) -> Result<JsValue, JsValue> {
        let step = self.inner.advance(work_units).map_err(faults::thrown)?;
        crossing::to_js(&step)
    }

    /// Consumes the pass, leaving its maps on the originating film desk.
    pub fn finish(mut self) -> Result<JsValue, JsValue> {
        let result = self.inner.result().map_err(faults::thrown)?;
        let result = crossing::to_js(&result)?;
        self.cache.borrow_mut().film_pass = Some(self.inner.take_cache().map_err(faults::thrown)?);
        Ok(result)
    }
}

/// The chain stages over the film workers' strips.
#[wasm_bindgen]
pub struct ChainPass {
    inner: chain_pass::ChainPass,
}

#[wasm_bindgen]
impl ChainPass {
    /// Starts without doing the search, so the worker can yield immediately.
    pub fn start(results: JsValue, options: JsValue) -> Result<ChainPass, JsValue> {
        let results: Vec<FilmPassResult> = crossing::from_js(results)?;
        let options: ChainOptions = crossing::from_js(options)?;
        let inner = chain_pass::ChainPass::new(results, options).map_err(faults::thrown)?;
        Ok(ChainPass { inner })
    }

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
