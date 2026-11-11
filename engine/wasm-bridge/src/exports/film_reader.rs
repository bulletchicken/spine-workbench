//! `FilmDesk.open`, `FilmDesk.open_saved`, `facts`, `thumbnail` and
//! `display_pixels`.
//!
//! Display pixels are IEEE 754 binary16 bit patterns for
//! `p / (2^bits_stored - 1)` in the inclusive range `[0, 1]`. They carry
//! the bone-bright stored values; the viewer applies window and level.

use crate::{crossing, faults, film_desk::FilmDesk};
use ::shapes::film::{Film, FilmKind};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl FilmDesk {
    /// Reads PNG, JPEG or DICOM bytes into a fresh desk. The kind names
    /// the film slot; the format is recognised from the bytes alone.
    pub fn open(bytes: &[u8], kind: JsValue) -> Result<FilmDesk, JsValue> {
        let kind: FilmKind = crossing::from_js(kind)?;
        let film = ::film_reader::read_film(bytes, kind).map_err(faults::thrown)?;
        Ok(FilmDesk::new(film))
    }

    /// Opens a case PNG against its saved record. Its stored values stay
    /// exact; the record restores the original source facts and value map.
    pub fn open_saved(png: &[u8], film: JsValue) -> Result<FilmDesk, JsValue> {
        let film: Film = crossing::from_js(film)?;
        let decoded = ::film_reader::read_saved_film(png, &film).map_err(faults::thrown)?;
        Ok(FilmDesk::new(decoded))
    }

    /// The film's dimensions, value map, flags and kept file facts.
    pub fn facts(&self) -> Result<JsValue, JsValue> {
        crossing::to_js(&self.film.facts)
    }

    /// A metadata-free, 8-bit greyscale PNG, bounded by `max_side`.
    pub fn thumbnail(&self, max_side: u32) -> Result<Vec<u8>, JsValue> {
        ::film_reader::thumbnail::thumbnail(&self.film, max_side).map_err(faults::thrown)
    }

    /// A fresh half-float buffer in `[0, 1]`, before window and level.
    /// wasm-bindgen copies this owned vector into a new `Uint16Array`.
    pub fn display_pixels(&self) -> Result<Vec<u16>, JsValue> {
        ::film_reader::display_pixels::display_pixels(&self.film).map_err(faults::thrown)
    }
}
