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
