//! One drawing correction on the unchanged fit, separate from every version.

use super::assembly::{self, Placement};
use crate::evidence::Evidence;
use crate::rigid_rod::RigidState;
use crate::springback::SeatedState;
use crate::versions::refusal;
use crate::{BenchError, daily_loads, rigid_rod, springback, versions};
use nalgebra::Vector3;
use shapes::implant_layout::ImplantLayout;
use shapes::load_bench::{MiddleRun, RodLine};
use shapes::spine_fit::SpineFit;

pub(super) fn run(
    layout: &ImplantLayout,
    fit: &SpineFit,
    evidence: &Evidence,
    placement: &Placement,
) -> MiddleRun {
    let mut result = MiddleRun {
        layout_id: layout.id.clone(),
        before: assembly::reference(fit),
        rods_in: None,
        rods: None,
    };
    // Drawing never refuses the slice or discards its version runs.
    let _ = fill(&mut result, layout, fit, evidence, placement);
    result
}

fn fill(
    result: &mut MiddleRun,
    layout: &ImplantLayout,
    fit: &SpineFit,
    evidence: &Evidence,
    placement: &Placement,
) -> Result<(), BenchError> {
    let draw = versions::middle(evidence)?;
    let base = assembly::chain(fit, draw.stiffness_scale, draw.rib_factors, evidence)?;
    let chain = daily_loads::loaded_chain(evidence, &base, 1.0, &layout.released_levels)
        .map_err(|error| refusal(&error.to_string()))?;
    let held = match rigid_rod::solve_held(&chain, &placement.construct, &Default::default(), None)
    {
        Ok(state) if state.settled => state,
        _ => return Ok(()),
    };
    let rigid = match rigid_rod::solve_rigid(&chain, &placement.construct, &held, None) {
        Ok(state) if state.settled => state,
        _ => return Ok(()),
    };
    let seated = match springback::solve_balanced(
        &chain,
        &rigid.poses,
        &rigid.rods,
        &held.poses,
        &rigid.applied_loads,
    ) {
        Ok(state) if state.settled => state,
        _ => return Ok(()),
    };
    result.rods = Some(lines(layout, placement, &rigid, &seated)?);
    result.rods_in = Some(
        seated
            .poses
            .iter()
            .zip(&base.reference)
            .map(|(p, r)| p.to_vertebra(r))
            .collect(),
    );
    Ok(())
}

fn lines(
    layout: &ImplantLayout,
    placement: &Placement,
    rigid: &RigidState,
    seated: &SeatedState,
) -> Result<Vec<RodLine>, BenchError> {
    placement
        .rod_indices
        .iter()
        .enumerate()
        .map(|(r, &proposal)| {
            let stations = &rigid.rod_head_stations_mm[r];
            let deltas: Vec<_> = seated.corrected_rods[r]
                .ties
                .iter()
                .zip(&rigid.rods[r].ties)
                .map(|(p, q)| p.point_mm - q.point_mm)
                .collect();
            // The same first-order contour convention as section statics:
            // interpolate head translations between stations and extrapolate
            // beyond the outer heads, retaining contour and head knots plus
            // occupied straight end continuations. This first-order geometry
            // can differ in length from the supplied head-to-head contour.
            let points_mm = shifted_points(&rigid.rod_points_mm[r], stations, &deltas)?;
            Ok(RodLine {
                side: layout.rods[proposal].side,
                points_mm,
            })
        })
        .collect()
}

fn shifted_points(
    points: &[Vector3<f64>],
    stations: &[f64],
    deltas: &[Vector3<f64>],
) -> Result<Vec<[f64; 3]>, BenchError> {
    if stations.len() < 2
        || stations.len() != deltas.len()
        || stations.iter().any(|s| !s.is_finite())
        || stations.windows(2).any(|s| s[1] <= s[0])
    {
        return Err(refusal("needs-distinct-head-stations"));
    }
    let knots =
        crate::rod_beams::polyline_stations(points).map_err(|error| refusal(&error.to_string()))?;
    points
        .iter()
        .zip(knots)
        .map(|(point, s)| {
            let i = stations
                .partition_point(|x| *x <= s)
                .saturating_sub(1)
                .min(stations.len() - 2);
            let t = (s - stations[i]) / (stations[i + 1] - stations[i]);
            let moved = *point + deltas[i] * (1.0 - t) + deltas[i + 1] * t;
            if moved.iter().any(|x| !x.is_finite()) {
                return Err(refusal("non-finite-rod-line"));
            }
            Ok(moved.into())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_needs_two_distinct_heads_and_a_contour() {
        let points = [Vector3::zeros(), Vector3::new(0.0, 0.0, 30.0)];
        for stations in [&[][..], &[10.0][..], &[10.0, 10.0][..]] {
            let deltas = vec![Vector3::zeros(); stations.len()];
            assert!(shifted_points(&points, stations, &deltas).is_err());
        }
        assert!(shifted_points(&[], &[10.0, 20.0], &[Vector3::zeros(); 2]).is_err());
    }

    #[test]
    fn moving_heads_interpolate_inside_and_extrapolate_at_both_ends() {
        let points = [0.0, 10.0, 15.0, 20.0, 30.0].map(|z| Vector3::new(0.0, 0.0, z));
        let deltas = [Vector3::new(2.0, 0.0, 0.0), Vector3::new(4.0, 0.0, 0.0)];
        assert_eq!(
            shifted_points(&points, &[10.0, 20.0], &deltas).unwrap(),
            vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 10.0],
                [3.0, 0.0, 15.0],
                [4.0, 0.0, 20.0],
                [6.0, 0.0, 30.0]
            ],
        );
    }
}
