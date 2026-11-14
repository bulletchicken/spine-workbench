//! Section statics with head couples, then the sourced strength comparisons.
//! Correction contours follow the computed head translations, linearly
//! interpolated between heads. This is a first-order drawing of the elastic
//! contour, not an additional beam equilibrium solve. Daily increments keep
//! dP × F0 at heads and cuts; no daily displacement changes the shown spine.

use super::*;
use crate::fatigue::{self, FatigueComparison, WalkingInput};
use crate::rod_beams::{polyline_at, polyline_stations};
use crate::rod_statics::{self, HeadIncrement, HeadLoad, SectionIncrement, SectionLoad};
use shapes::load_bench::RodSwing;

pub(super) struct StressValues {
    pub uses: Vec<f64>,
    pub swing: Option<Vec<RodSwing>>,
    pub cycles: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn at_stop(
    layout: &ImplantLayout,
    draw: &Draw,
    e: &Evidence,
    placement: &Placement,
    rigid: &RigidState,
    seated: &SeatedState,
    response: Option<&DailyResponse>,
    daily: &DailyCases,
    year: bool,
) -> Result<StressValues, BenchError> {
    let mut result = StressValues {
        uses: Vec::new(),
        swing: year.then(Vec::new),
        cycles: None,
    };
    for (r, &proposal) in placement.rod_indices.iter().enumerate() {
        let rod = &layout.rods[proposal];
        let (yield_mpa, ultimate_mpa) =
            crate::versions::strengths(e, rod.material, draw.strength_quantile)?;
        let stations = &rigid.rod_head_stations_mm[r];
        let head_deltas: Vec<_> = seated.corrected_rods[r]
            .ties
            .iter()
            .zip(&rigid.rods[r].ties)
            .map(|(p, q)| p.point_mm - q.point_mm)
            .collect();
        let old_knots =
            polyline_stations(&rigid.rod_points_mm[r]).map_err(|e| refusal(&e.to_string()))?;
        let points: Vec<_> = rigid.rod_points_mm[r]
            .iter()
            .zip(&old_knots)
            .map(|(p, s)| *p + interpolate(stations, &head_deltas, *s))
            .collect();
        let new_knots = polyline_stations(&points).map_err(|e| refusal(&e.to_string()))?;
        let current_stations: Vec<_> = stations
            .iter()
            .map(|s| mapped(&old_knots, &new_knots, *s))
            .collect();
        let heads: Vec<_> = seated.corrected_rods[r]
            .ties
            .iter()
            .enumerate()
            .map(|(h, tie)| {
                let load = seated.head_wrenches_on_rod[r][h];
                HeadLoad {
                    station_mm: current_stations[h],
                    position_mm: tie.point_mm,
                    force_n: load.fixed_rows::<3>(0).into_owned(),
                    couple_nmm: load.fixed_rows::<3>(3).into_owned(),
                }
            })
            .collect();
        let increments = |daily: &DailyResponse| -> Vec<HeadIncrement> {
            daily.head_wrenches_on_rod[r]
                .iter()
                .enumerate()
                .map(|(h, w)| HeadIncrement {
                    point_mm: daily.head_displacements_mm[r][h],
                    force_n: w.fixed_rows::<3>(0).into_owned(),
                    couple_nmm: w.fixed_rows::<3>(3).into_owned(),
                })
                .collect()
        };
        let delta = response.map(increments);
        let standing_delta = increments(&daily.standing);
        let mut rod_ratio: Option<f64> = Some(0.0);
        let mut finite_goodman = true;
        for pair in current_stations.windows(2) {
            let mut cuts = vec![pair[0], pair[1]];
            cuts.extend(
                new_knots
                    .iter()
                    .copied()
                    .filter(|s| *s > pair[0] && *s < pair[1]),
            );
            // Both faces of every head/corner, including a locked head's
            // couple jump. No distributed loads occur between these cuts.
            let boundaries = cuts.clone();
            for s in boundaries {
                for neighbour in [s.next_down(), s.next_up()] {
                    if neighbour >= pair[0] && neighbour <= pair[1] {
                        cuts.push(neighbour);
                    }
                }
            }
            let mut peak: f64 = 0.0;
            for s in cuts {
                let locked = rod_statics::section_at(&points, &heads, s)
                    .map_err(|e| refusal(&e.to_string()))?;
                let total_section = if let Some(delta) = &delta {
                    section(&points, &heads, &current_stations, delta, s)?
                } else {
                    locked
                };
                let stress = rod_statics::surface_stress(&total_section, rod.diameter_mm)
                    .map_err(|e| refusal(&e.to_string()))?
                    .von_mises_mpa;
                peak = peak.max(stress / yield_mpa);
                if year {
                    let standing = section(&points, &heads, &current_stations, &standing_delta, s)?;
                    let turn = daily.standing.rod_displacements[r]
                        .fixed_rows::<3>(3)
                        .into_owned();
                    let increment = increment_in_rod_frame(&locked, &standing, &turn);
                    let locked_stress = rod_statics::surface_stress(&locked, rod.diameter_mm)
                        .map_err(|e| refusal(&e.to_string()))?
                        .von_mises_mpa;
                    let standing_stress = rod_statics::surface_stress(&increment, rod.diameter_mm)
                        .map_err(|e| refusal(&e.to_string()))?
                        .von_mises_mpa;
                    let walking = fatigue::year_of_walking(
                        e,
                        rod.material,
                        WalkingInput {
                            steps_per_day: draw.values.steps_per_day,
                            walking_multiple: draw.values.walking_multiple,
                            contour_knock_down: draw.values.contour_knock_down,
                            ultimate_mpa,
                        },
                        locked_stress,
                        standing_stress,
                    )
                    .map_err(|e| refusal(&e.to_string()))?;
                    result.cycles = Some(walking.cycles_per_year);
                    match walking.comparison {
                        FatigueComparison::Ratio(ratio) => {
                            rod_ratio = rod_ratio.map(|v| v.max(ratio));
                        }
                        FatigueComparison::NoFatigueLevel => rod_ratio = None,
                        FatigueComparison::NoFiniteGoodman => finite_goodman = false,
                    }
                }
            }
            if !peak.is_finite() {
                return Err(refusal("non-finite-rod-stress"));
            }
            result.uses.push(peak);
        }
        if let Some(swing) = &mut result.swing {
            swing.push(RodSwing {
                side: rod.side,
                ratio: if finite_goodman { rod_ratio } else { None },
            });
        }
    }
    Ok(result)
}

/// First variation in the solver's free, turning rod frame. Spatial
/// subtraction alone includes omega cross the locked-in section load,
/// even for a rigid turn with no rod strain. Keep the reference tangent,
/// as for the scalar section envelope; no material fibre is followed.
fn increment_in_rod_frame(
    locked: &SectionLoad,
    loaded: &SectionLoad,
    turn: &Vector3<f64>,
) -> SectionLoad {
    SectionLoad {
        force_n: loaded.force_n - locked.force_n - turn.cross(&locked.force_n),
        moment_nmm: loaded.moment_nmm - locked.moment_nmm - turn.cross(&locked.moment_nmm),
        ..*locked
    }
}

fn section(
    points: &[Vector3<f64>],
    heads: &[HeadLoad],
    stations: &[f64],
    delta: &[HeadIncrement],
    s: f64,
) -> Result<SectionLoad, BenchError> {
    let motions: Vec<_> = delta.iter().map(|d| d.point_mm).collect();
    let (_, tangent) = polyline_at(points, s).map_err(|e| refusal(&e.to_string()))?;
    let i = interval(stations, s);
    let tangent_change = (motions[i + 1] - motions[i]) / (stations[i + 1] - stations[i]);
    let current_tangent =
        (tangent + tangent_change - tangent * tangent.dot(&tangent_change)).normalize();
    rod_statics::linearised_section_at(
        points,
        heads,
        delta,
        s,
        SectionIncrement {
            point_mm: interpolate(stations, &motions, s),
            tangent_unit: current_tangent,
        },
    )
    .map_err(|e| refusal(&e.to_string()))
}
fn interval(stations: &[f64], s: f64) -> usize {
    stations
        .partition_point(|x| *x <= s)
        .saturating_sub(1)
        .min(stations.len() - 2)
}
fn interpolate(stations: &[f64], values: &[Vector3<f64>], s: f64) -> Vector3<f64> {
    let i = interval(stations, s);
    let t = (s - stations[i]) / (stations[i + 1] - stations[i]);
    values[i] * (1.0 - t) + values[i + 1] * t
}
fn mapped(old: &[f64], new: &[f64], s: f64) -> f64 {
    let i = interval(old, s);
    new[i] + (s - old[i]) / (old[i + 1] - old[i]) * (new[i + 1] - new[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rigid_rod_turn_adds_no_walking_stress_swing() {
        let locked = SectionLoad {
            position_mm: Vector3::zeros(),
            tangent_unit: Vector3::z(),
            force_n: Vector3::new(0.0, 0.0, -10.0),
            moment_nmm: Vector3::new(100.0, 0.0, 0.0),
        };
        // Turning .05 rad about z carries 100*x to 100*x+5*y to first
        // order, with no material change in the rod's 100 Nmm moment.
        let turned = SectionLoad {
            moment_nmm: Vector3::new(100.0, 5.0, 0.0),
            ..locked
        };
        let increment = increment_in_rod_frame(&locked, &turned, &Vector3::new(0.0, 0.0, 0.05));
        let stress = rod_statics::surface_stress(&increment, 2.0)
            .unwrap()
            .von_mises_mpa;
        assert_eq!(stress, 0.0);
        let cycle = fatigue::walking_cycle(100.0, stress, 1.5, 1000.0).unwrap();
        assert_eq!(cycle.amplitude_mpa, 0.0);
        assert_eq!(cycle.equivalent_mpa, Some(0.0));

        // A genuine added 2 Nmm along x remains after the same rigid turn.
        let loaded = SectionLoad {
            moment_nmm: Vector3::new(102.0, 5.0, 0.0),
            ..locked
        };
        let increment = increment_in_rod_frame(&locked, &loaded, &Vector3::new(0.0, 0.0, 0.05));
        let stress = rod_statics::surface_stress(&increment, 2.0)
            .unwrap()
            .von_mises_mpa;
        // d=2, Z=pi*d^3/32=pi/4: stress=2/Z=8/pi MPa.
        assert!((stress - 8.0 / core::f64::consts::PI).abs() < 1e-12);
    }
}
