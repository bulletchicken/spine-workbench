//! When the two counts disagree: flag it, offer the readings, and ask which
//! vertebra is L5.

use crate::handoffs::{ChainBoundaries, FoundBody};
use shapes::levels::Level;

/// The independent counts give two names to the same physical body.
/// A match covers at least half of each body's interval, preventing a
/// tiny interval contained inside a much larger one from naming it.
pub fn ends_disagree(top: &ChainBoundaries, bottom: &ChainBoundaries) -> bool {
    top.film == bottom.film
        && top.bodies.iter().any(|a| {
            bottom
                .bodies
                .iter()
                .any(|b| overlap(a, b).is_some() && a.level != b.level)
        })
}

fn overlap(a: &FoundBody, b: &FoundBody) -> Option<f64> {
    let ha = a.inf.row - a.sup.row;
    let hb = b.inf.row - b.sup.row;
    let shared = a.inf.row.min(b.inf.row) - a.sup.row.max(b.sup.row);
    (ha.is_finite() && hb.is_finite() && ha > 0.0 && hb > 0.0 && shared >= 0.5 * ha.max(hb))
        .then_some(shared / ha.max(hb))
}

pub(super) fn matching_names(
    chain: &ChainBoundaries,
    anchored: &ChainBoundaries,
    from_bottom: bool,
) -> Option<Vec<Level>> {
    if chain.film != anchored.film || chain.template != anchored.template || chain.bodies.is_empty()
    {
        return None;
    }
    let names = chain.template.names();
    let count = chain.bodies.len();
    for offset in 0..count {
        let i = if from_bottom {
            count - 1 - offset
        } else {
            offset
        };
        let body = &chain.bodies[i];
        let mut best: Option<(&FoundBody, f64)> = None;
        for candidate in &anchored.bodies {
            if let Some(shared) = overlap(body, candidate)
                && best.is_none_or(|(_, old)| shared > old)
            {
                best = Some((candidate, shared));
            }
        }
        if let Some((matched, _)) = best {
            let position = names.iter().position(|level| *level == matched.level)?;
            let start = position.checked_sub(i)?;
            return names
                .get(start..start.checked_add(count)?)
                .map(<[_]>::to_vec);
        }
    }
    None
}
