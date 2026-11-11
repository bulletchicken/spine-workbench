//! Stage 3, the spine unrolled: a straight strip along the centre line, with
//! its map back to film pixels, and the bone and gap profile along it. Hands
//! on a `Strip` and a `Barcode`.

pub mod barcode;
pub mod checkpoints;
pub mod maps;
pub mod unroll;

use crate::handoffs::{Barcode, CentreLine, FilmEvidence, Strip};
use darkroom::grey_image::GreyImage;
use shapes::auto_pass::PassWarning;
use shapes::errors::EngineError;
use shapes::film_geometry::FilmGeometry;

/// Unroll the original film and form its bone-and-gap profile. The working
/// evidence controls validity and polarity; its smaller image is never
/// substituted for the original pixels.
pub fn build(
    image: &GreyImage<u16>,
    evidence: &FilmEvidence,
    line: &CentreLine,
    geometry: &FilmGeometry,
) -> Result<(Strip, Barcode), EngineError> {
    let strip = unroll::build(image, evidence, line, geometry)?;
    let barcode = barcode::build(&strip, line.ends.size_factor)?;
    Ok((strip, barcode))
}

/// Warnings belonging to this stage. The frozen strip baton has no warning
/// field, so the film pass keeps these alongside its other stage warnings.
pub fn warnings(strip: &Strip) -> Vec<PassWarning> {
    checkpoints::fold_warning(strip).into_iter().collect()
}
