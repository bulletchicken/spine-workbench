//! `pack_case`, `unpack_case` and `FilmDesk.encode_png`.

use crate::{crossing, faults, film_desk::FilmDesk};
use shapes::case_file_v1::OpenedCase;
use shapes::film::Film;
use wasm_bindgen::prelude::*;

/// Checks and packs a case record and its stored PNGs into a fresh byte
/// array. The archive contains only the format's fixed entry names.
#[wasm_bindgen]
pub fn pack_case(opened: JsValue) -> Result<Vec<u8>, JsValue> {
    let opened: OpenedCase = crossing::from_js(opened)?;
    case_file::pack::pack_case(&opened.case, &opened.films).map_err(faults::thrown)
}

/// Reads the whole archive strictly before returning its record and
/// films. PNG byte fields cross as fresh `Uint8Array`s.
#[wasm_bindgen]
pub fn unpack_case(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let opened = case_file::unpack::unpack_case(bytes).map_err(faults::thrown)?;
    crossing::to_js(&opened)
}

#[wasm_bindgen]
impl FilmDesk {
    /// Stores this desk's pixels through the current film record: its
    /// brightness flip and blanked boxes, without any view transform.
    #[wasm_bindgen]
    pub fn encode_png(&self, film: JsValue) -> Result<Vec<u8>, JsValue> {
        let film: Film = crossing::from_js(film)?;
        case_file::film_png::encode_png(self.film.image.pixels(), &self.film.facts, &film)
            .map_err(faults::thrown)
    }
}
