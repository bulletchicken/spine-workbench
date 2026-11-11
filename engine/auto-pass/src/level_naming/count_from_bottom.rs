//! Counting up from the sacrum to L5.

use super::disagreement::matching_names;
use crate::handoffs::ChainBoundaries;
use shapes::levels::Level;

/// Match the caudalmost overlapping body of the sacrum-anchored solve,
/// then count upward. Its lowest name follows the chosen count (L4, L5
/// or L6); it is never silently forced to L5.
pub fn count(chain: &ChainBoundaries, bottom_anchored: &ChainBoundaries) -> Option<Vec<Level>> {
    matching_names(chain, bottom_anchored, true)
}
