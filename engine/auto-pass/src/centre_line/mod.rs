//! Stage 2, the spine's centre line: from the neck to below the top of the
//! sacrum, as the cheapest path through a ridge map, then smoothed, with the
//! spine's width along it. Hands on a `CentreLine`.

pub mod cheapest_path;
pub mod cost_map;
pub mod smooth;
