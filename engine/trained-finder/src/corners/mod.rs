//! Each corner settled on the full-size film.
//!
//! The chain's outlines come from a picture shrunk to 384 by 768, so a
//! corner can sit a few film pixels off. For each outline a small square
//! of the full-size film is cut out around it ([`patch`]), a second,
//! smaller network says where each corner looks to be ([`corner_net`]),
//! and the corner moves part of the way there, never far ([`settle`]).

pub mod corner_net;
pub mod patch;
pub mod settle;
