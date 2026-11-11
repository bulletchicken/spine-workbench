//! The maths behind the adjusting tools. Each tool answers a hover with
//! what a click would do before it acts, and nothing happens on a click
//! that the hover did not show. An edit on one film never touches another,
//! and the vertebra count changes only as the declared result of a split
//! or merge the user asked for.
//!
//! | Module | Holds |
//! |---|---|
//! | [`region_wand`] | add to or take from an outline, then refit the corners |
//! | [`boundary_wand`] | split one vertebra or merge two, and renumber below |
//! | [`pins`] | marks that stay put while their neighbours re-solve |

#![forbid(unsafe_code)]

pub mod boundary_wand;
pub mod pins;
pub mod region_wand;

use auto_pass::handoffs::FilmPassCache;
use film_reader::decoded_film::DecodedFilm;
use shapes::{
    errors::EngineError,
    film_geometry::FilmGeometry,
    toolbox::{ToolPreview, ToolPreviewInput, WandKind},
};

/// Preview the same decision the corresponding commit validates. Both calls
/// read the supplied marks and echo their case version without storing edits.
pub fn preview(
    film: &DecodedFilm,
    geometry: Option<&FilmGeometry>,
    cache: Option<&FilmPassCache>,
    input: &ToolPreviewInput,
) -> Result<ToolPreview, EngineError> {
    match input.tool {
        WandKind::RegionWand => {
            region_wand::hover::preview(film, &input.marks, input.at, input.case_version)
        }
        WandKind::BoundaryWand => boundary_wand::preview(film, geometry, cache, input),
    }
}
