//! Counting down from the first rib to T1.

use super::disagreement::matching_names;
use crate::handoffs::ChainBoundaries;
use shapes::levels::Level;

/// Match the headmost overlapping body of the first-rib solve, then
/// count down its template's names. End-count runs label C7 first, so
/// matching T1 transfers the first-rib anchor without any user point.
pub fn count(chain: &ChainBoundaries, top_anchored: &ChainBoundaries) -> Option<Vec<Level>> {
    matching_names(chain, top_anchored, false)
}
