//! `DecodedFilm`: what reading a film produces, and what every later step
//! starts from. A frozen contract.

use darkroom::grey_image::GreyImage;
use shapes::film::FilmFacts;

/// One film, read.
///
/// `image` holds the stored values: exact, bone bright, from 0 to
/// 2^bitsStored − 1, placed at `Placement::FILM`. They relate to the file's
/// own values through `facts.values`, exactly and reversibly.
///
/// The pixels are never turned, flipped, resized or masked. Upright and
/// mirroring are view transforms; shutters and padding are left to the
/// stages that need them, which read the shutters from `facts.keptTags`.
/// The case file's PNG stores exactly these values, so a film read again
/// from a saved case gives the same pixels.
///
/// `image` is `facts.width` by `facts.height`.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedFilm {
    pub image: GreyImage<u16>,
    pub facts: FilmFacts,
}
