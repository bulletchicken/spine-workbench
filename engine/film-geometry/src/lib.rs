//! The two facts every film needs before work starts: how big a pixel is,
//! and which way the film is turned and faces. Every answer here is a
//! proposal with its evidence; the user confirms, and only confirmed facts
//! are saved.
//!
//! | Module | Holds |
//! |---|---|
//! | [`rough_trace`] | a cheap trace on a coarse copy; collimation; the brightness check |
//! | [`scale`] | scale from tags, a ruler or a known object |
//! | [`upright`] | quarter turns and upside-down |
//! | [`side_facing`] | which way a side film faces |
//! | [`lead_marker`] | the lead left/right letter and possible burnt-in text |

#![forbid(unsafe_code)]

pub mod lead_marker;
pub mod rough_trace;
pub mod scale;
pub mod side_facing;
pub mod upright;

mod front_facing;

use film_reader::decoded_film::DecodedFilm;
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::film_geometry::GeometryGuess;

pub use scale::from_known_object::from_known_object as scale_from_known_object;
pub use scale::from_ruler::from_ruler as scale_from_ruler;

/// Propose each geometry fact independently and retain its evidence.
/// Empty anatomy can still have a usable scale or file hint, so a blank
/// film returns uncertain proposals. Nothing here sets a confirmation.
pub fn sense_geometry(film: &DecodedFilm) -> Result<GeometryGuess, EngineError> {
    let trace = rough_trace::RoughTrace::new(film)?;
    let upright = upright::guess(film, &trace);
    let profile = trace.profile(upright.quarter_turns);
    let (lead_markers, text_hints) = lead_marker::find(film);
    let image_right = if film.facts.kind == FilmKind::Side {
        let mut guess = side_facing::guess(film, &profile, upright.quarter_turns);
        guess.looks_mirrored |= lead_markers.iter().any(|marker| marker.mirrored);
        guess
    } else {
        front_facing::guess(film, &profile, upright.quarter_turns, &lead_markers)
    };
    Ok(GeometryGuess {
        upright,
        image_right,
        polarity: trace.polarity,
        scales: scale::from_tags::from_tags(film.facts.kept_tags.as_ref()),
        lead_markers,
        text_hints,
        collimation: trace.collimation,
    })
}
