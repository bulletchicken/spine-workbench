//! The typical spine compiled into the engine, read strictly once: each
//! body's width, depth over width and side wedge, side profiles by curve
//! size, how closely bodies stack, and twist at a curve's apex, every value
//! with its spread. Values are looked up by level name.

use crate::toward::Spread;
use serde::Deserialize;
use shapes::levels::Level;
use std::sync::OnceLock;

const TEXT: &str = include_str!("../typical-spine/typical-spine.json");

/// The levels the file describes, top to bottom.
const LEVELS: [Level; 18] = [
    Level::C7,
    Level::T1,
    Level::T2,
    Level::T3,
    Level::T4,
    Level::T5,
    Level::T6,
    Level::T7,
    Level::T8,
    Level::T9,
    Level::T10,
    Level::T11,
    Level::T12,
    Level::L1,
    Level::L2,
    Level::L3,
    Level::L4,
    Level::L5,
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TypicalSpine {
    about: String,
    sources: Vec<Source>,
    bodies: Vec<Body>,
    side_profiles: Vec<SideProfile>,
    stacking_spread: f64,
    twist_at_apex: Vec<TwistAtApex>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    key: String,
    citation: String,
    gives: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeanSd {
    mean: f64,
    sd: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Body {
    level: Level,
    half_width_mm: f64,
    depth_to_width: MeanSd,
    side_wedge_deg: MeanSd,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SideProfile {
    curve_deg: f64,
    forward_lean_deg: [f64; 18],
    sd_deg: [f64; 18],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TwistAtApex {
    curve_deg: f64,
    twist_deg: f64,
    sd_deg: f64,
}

/// What the typical spine says about each fitted vertebra, in the solver's
/// units, and how closely neighbouring bodies stack.
pub struct Typical {
    /// One per observation, in the problem's order.
    pub levels: Vec<TypicalLevel>,
    /// Spread of the forward step between centres, as a share of the rise.
    pub stacking_spread: f64,
}

pub struct TypicalLevel {
    pub lean_rad: Spread,
    /// Logarithm of half-depth over half-width.
    pub log_depth_to_width: Spread,
    pub side_wedge_rad: Spread,
    pub twist_rad: Spread,
}

/// The built-in typical spine. Its text is part of the engine and a test
/// reads it, so a failure here is a build that should never have passed.
pub fn built_in() -> &'static TypicalSpine {
    static SPINE: OnceLock<TypicalSpine> = OnceLock::new();
    SPINE.get_or_init(|| read(TEXT).expect("the built-in typical spine is well formed"))
}

fn read(text: &str) -> Option<TypicalSpine> {
    let spine: TypicalSpine = serde_json::from_str(text).ok()?;
    spine.well_formed().then_some(spine)
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

impl TypicalSpine {
    fn well_formed(&self) -> bool {
        let words = |text: &String| !text.trim().is_empty();
        words(&self.about)
            && !self.sources.is_empty()
            && self
                .sources
                .iter()
                .all(|s| words(&s.key) && words(&s.citation) && words(&s.gives))
            && self.bodies.len() == LEVELS.len()
            && self.bodies.iter().zip(LEVELS).all(|(body, level)| {
                body.level == level
                    && positive(body.half_width_mm)
                    && positive(body.depth_to_width.mean)
                    && positive(body.depth_to_width.sd)
                    && body.side_wedge_deg.mean.is_finite()
                    && positive(body.side_wedge_deg.sd)
            })
            && !self.side_profiles.is_empty()
            && self
                .side_profiles
                .windows(2)
                .all(|p| p[0].curve_deg < p[1].curve_deg)
            && self.side_profiles.iter().all(|p| {
                p.curve_deg.is_finite()
                    && p.forward_lean_deg.iter().all(|x| x.is_finite())
                    && p.sd_deg.iter().all(|&x| positive(x))
            })
            && positive(self.stacking_spread)
            && !self.twist_at_apex.is_empty()
            && self
                .twist_at_apex
                .windows(2)
                .all(|t| t[0].curve_deg < t[1].curve_deg)
            && self
                .twist_at_apex
                .iter()
                .all(|t| t.curve_deg.is_finite() && t.twist_deg.is_finite() && positive(t.sd_deg))
    }

    /// The typical values for each level, with the side profile of a curve
    /// of `curve_deg` degrees. Twist is zero give or take the spread the
    /// file gives at the apex of its smallest curve, until the curves set
    /// it (`curve_twist`).
    ///
    /// When the names are estimates, a name may be a level off, so each
    /// value looked up by name (forward lean, at the side-profile row the
    /// curve chose; depth over width; side wedge) takes a wider spread: the
    /// root of its own spread squared plus the largest difference between
    /// its value and either neighbouring level's, squared, both in the
    /// file's own units. A one-level shift then moves each value by less
    /// than its spread.
    pub fn for_levels(
        &self,
        levels: impl Iterator<Item = Level>,
        curve_deg: f64,
        names_are_estimates: bool,
    ) -> Typical {
        let twist_rad = Spread {
            mean: 0.0,
            sd: self.twist_at_apex[0].sd_deg.to_radians(),
        };
        let levels = levels
            .map(|level| {
                let row = row(level);
                let mut lean = self.side_profile(row, curve_deg);
                let mut depth = self.bodies[row].depth_to_width;
                let mut wedge = self.bodies[row].side_wedge_deg;
                if names_are_estimates {
                    let lean_at = |r| self.side_profile(r, curve_deg).mean;
                    lean.sd = widened(lean.mean, lean.sd, neighbours(row).map(lean_at));
                    let depth_at = |r: usize| self.bodies[r].depth_to_width.mean;
                    depth.sd = widened(depth.mean, depth.sd, neighbours(row).map(depth_at));
                    let wedge_at = |r: usize| self.bodies[r].side_wedge_deg.mean;
                    wedge.sd = widened(wedge.mean, wedge.sd, neighbours(row).map(wedge_at));
                }
                TypicalLevel {
                    lean_rad: Spread {
                        mean: lean.mean.to_radians(),
                        sd: lean.sd.to_radians(),
                    },
                    log_depth_to_width: Spread {
                        mean: libm::log(depth.mean),
                        sd: depth.sd / depth.mean,
                    },
                    side_wedge_rad: Spread {
                        mean: wedge.mean.to_radians(),
                        sd: wedge.sd.to_radians(),
                    },
                    twist_rad,
                }
            })
            .collect();
        Typical {
            levels,
            stacking_spread: self.stacking_spread,
        }
    }

    /// The size of the twist at the apex of a curve of `curve_deg` degrees,
    /// in radians, interpolated between the rows either side and held at
    /// the end rows.
    pub fn apex_twist_rad(&self, curve_deg: f64) -> Spread {
        let rows = &self.twist_at_apex;
        let (mean, sd) = match rows.iter().position(|t| t.curve_deg > curve_deg) {
            Some(0) => (rows[0].twist_deg, rows[0].sd_deg),
            None => (rows[rows.len() - 1].twist_deg, rows[rows.len() - 1].sd_deg),
            Some(i) => {
                let (a, b) = (&rows[i - 1], &rows[i]);
                let t = (curve_deg - a.curve_deg) / (b.curve_deg - a.curve_deg);
                (
                    a.twist_deg + t * (b.twist_deg - a.twist_deg),
                    a.sd_deg + t * (b.sd_deg - a.sd_deg),
                )
            }
        };
        Spread {
            mean: mean.to_radians(),
            sd: sd.to_radians(),
        }
    }

    /// A body's typical half-width in millimetres.
    pub fn half_width_mm(&self, level: Level) -> f64 {
        self.bodies[row(level)].half_width_mm
    }

    /// Forward lean in degrees at one row, interpolated between the side
    /// profiles either side of `curve_deg` and held at the end rows.
    fn side_profile(&self, row: usize, curve_deg: f64) -> Spread {
        let profiles = &self.side_profiles;
        let above = profiles
            .iter()
            .position(|p| p.curve_deg > curve_deg)
            .unwrap_or(profiles.len());
        let (a, b) = match above {
            0 => (&profiles[0], &profiles[0]),
            i if i == profiles.len() => (&profiles[i - 1], &profiles[i - 1]),
            i => (&profiles[i - 1], &profiles[i]),
        };
        let t = if b.curve_deg > a.curve_deg {
            (curve_deg - a.curve_deg) / (b.curve_deg - a.curve_deg)
        } else {
            0.0
        };
        Spread {
            mean: a.forward_lean_deg[row] + t * (b.forward_lean_deg[row] - a.forward_lean_deg[row]),
            sd: a.sd_deg[row] + t * (b.sd_deg[row] - a.sd_deg[row]),
        }
    }
}

/// The rows either side of `row` that the file holds.
fn neighbours(row: usize) -> impl Iterator<Item = usize> {
    [row.checked_sub(1), Some(row + 1)]
        .into_iter()
        .flatten()
        .filter(|&r| r < LEVELS.len())
}

/// `sd` widened by the largest difference between `mean` and a
/// neighbouring level's value.
fn widened(mean: f64, sd: f64, neighbours: impl Iterator<Item = f64>) -> f64 {
    let step = neighbours
        .map(|value| (value - mean).abs())
        .fold(0.0, f64::max);
    libm::sqrt(sd * sd + step * step)
}

/// The file's row for a level: levels above C7 take C7's, T13 takes T12's,
/// and levels below L5 take L5's.
fn row(level: Level) -> usize {
    LEVELS
        .iter()
        .position(|&l| l == level)
        .unwrap_or(match level {
            l if l < Level::C7 => 0,
            Level::T13 => 12,
            _ => LEVELS.len() - 1,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_typical_spine_is_well_formed() {
        assert!(read(TEXT).is_some());
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let mut value: serde_json::Value = serde_json::from_str(TEXT).expect("json");
        value["bodies"][0]
            .as_object_mut()
            .expect("an object")
            .insert("extra".to_string(), serde_json::json!(1));
        assert!(read(&value.to_string()).is_none());
    }

    #[test]
    fn levels_beyond_the_file_take_the_nearest_row() {
        assert_eq!(row(Level::C5), 0);
        assert_eq!(row(Level::T12), 12);
        assert_eq!(row(Level::T13), 12);
        assert_eq!(row(Level::L1), 13);
        assert_eq!(row(Level::L6), 17);
    }

    #[test]
    fn names_that_are_estimates_widen_each_spread_by_the_step_to_a_neighbour() {
        // L2 at no curve, from the file: forward lean -8.6 (sd 7.1) between
        // L1's -18.7 and L3's 7.2, so the largest step is 15.8 and the
        // spread becomes √(7.1² + 15.8²) = 17.322 degrees. Depth over width
        // 0.836 (sd 0.081) between 0.839 and 0.847: √(0.081² + 0.011²) =
        // 0.081743, 0.097779 of the mean as a spread of its logarithm. Side
        // wedge -8.5 (sd 4.9) between -2.5 and -5.7: √(4.9² + 6.0²) = 7.7466
        // degrees. Counted names keep the file's own spreads.
        let spine = built_in();
        let at = |estimates| {
            let typical = spine.for_levels([Level::L2].into_iter(), 0.0, estimates);
            let level = &typical.levels[0];
            [
                level.lean_rad.sd.to_degrees(),
                level.log_depth_to_width.sd,
                level.side_wedge_rad.sd.to_degrees(),
            ]
        };
        let expected = [[17.322, 0.097779, 7.7466], [7.1, 0.081 / 0.836, 4.9]];
        for (got, expected) in [at(true), at(false)].into_iter().zip(expected) {
            for (got, expected) in got.into_iter().zip(expected) {
                assert!(
                    (got / expected - 1.0).abs() < 1e-4,
                    "{got} against {expected}"
                );
            }
        }
    }
}
