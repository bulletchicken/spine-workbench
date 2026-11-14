//! Table-based screw proposals over versions, with explicit missing sizes.

pub mod anatomy;
pub mod geometry;
pub mod sizing;

use crate::LayoutError;
use crate::curves::{SpineReading, summary::band};
use crate::evidence::{PublishedValue, Table};
use crate::pedicles::{self, CurvePosition, cell, central, unique};
use crate::readings::{NamedSampler, Version};
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{
    AnchorKind, AnchorProposal, AngleFromTable, CannotTell, CannotTellCode, CurveSide,
    DiameterCount, DiameterRule, LayoutInput, LookCode, LookSources, NamingDoubt, ScrewHere,
    ValueStatus,
};
use shapes::levels::Level;
use shapes::spine_readings::{Band, BodySide};

mod looks;
mod proposal;

pub use looks::refresh_look_sources;
pub use looks::{ENDS_HOLDING_SHARE, LookInputs, context_looks, look_codes};
pub(crate) use proposal::ratio_for_diameter;
pub use proposal::{anchor, anchors, usual_length};

fn required_scalar(table: Table, key: &str, group: &str) -> Result<(f64, String), LayoutError> {
    let c = cell(table, key, group).ok_or_else(|| refused("missing-evidence"))?;
    Ok((
        central(c).ok_or_else(|| refused("non-scalar-evidence"))?,
        c.source.clone(),
    ))
}
fn invalid(reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: "screws".into(),
        reason: reason.into(),
    }
}
fn refused(reason: &str) -> LayoutError {
    LayoutError::Refused(reason.into())
}

#[cfg(test)]
mod tests;
