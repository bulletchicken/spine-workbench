//! Marks: the points placed on a vertebra.
//!
//! Front and bending films are front views; side films are side views.
//!
//! | Film | Marks |
//! |---|---|
//! | front | `supL`, `supR`, `infL`, `infR` (corners) and `pedL`, `pedR` (pedicles) |
//! | side | `supA`, `supP`, `infA`, `infP` (corners) |
//! | bending | the superior and inferior endplate lines, ends ordered patient left, then right |
//!
//! `sup` and `inf` are the upper and lower endplates. `L` and `R` are the
//! **patient's** left and right, `A` and `P` anterior and posterior; which
//! image direction that is comes from the film's
//! [`ImageRight`](crate::film_geometry::ImageRight).
//!
//! # Quad order
//!
//! Wherever a vertebra's four corners are listed together, they go round
//! the outline, so drawing them in order never crosses itself: upper left
//! (or anterior), upper right (or posterior), lower right (or posterior),
//! lower left (or anterior). That is `supL, supR, infR, infL` on a front
//! film, `supA, supP, infP, infA` on a side film, and on a bending film
//! the upper line's two ends followed by the lower line's ends reversed.
//! [`FrontMarks::corners`], [`SideMarks::corners`] and
//! [`EndplateLines::corners`] give them in this order.

use crate::pixels::Px;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Who or what last placed a mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MarkSource {
    /// The auto pass.
    Auto,
    /// A region wand edit refitted it.
    RegionWand,
    /// A boundary wand split or merge placed it.
    BoundaryWand,
    /// The user dragged it with the pin tool.
    Pin,
    /// The user placed it by hand.
    Hand,
}

/// One placed point.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mark {
    pub at: Px,
    pub source: MarkSource,
    /// Pinned marks stay exactly where the user put them; every tool and
    /// re-solve respects them.
    pub pinned: bool,
    /// Placed without image evidence at that spot: from the chain's
    /// padding beyond a cropped film, or a default pedicle position.
    pub inferred: bool,
}

impl Mark {
    /// A mark the auto pass placed on evidence.
    pub const fn auto(at: Px) -> Self {
        Mark {
            at,
            source: MarkSource::Auto,
            pinned: false,
            inferred: false,
        }
    }
}

/// A pedicle: placed, or not visible. Only pedicles can be "not visible";
/// the types allow it nowhere else.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "seen",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PedicleMark {
    Visible {
        at: Px,
        source: MarkSource,
        pinned: bool,
        inferred: bool,
        /// Strength of the ring evidence where the auto pass found it;
        /// `null` when a person placed it.
        ring_score: Option<f64>,
    },
    NotVisible {
        /// Who decided it cannot be seen.
        source: MarkSource,
    },
}

impl PedicleMark {
    /// The placed mark, if the pedicle is visible.
    pub const fn mark(&self) -> Option<Mark> {
        match *self {
            PedicleMark::Visible {
                at,
                source,
                pinned,
                inferred,
                ..
            } => Some(Mark {
                at,
                source,
                pinned,
                inferred,
            }),
            PedicleMark::NotVisible { .. } => None,
        }
    }
}

/// The six marks of a vertebra on a front film.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrontMarks {
    pub sup_l: Mark,
    pub sup_r: Mark,
    pub inf_l: Mark,
    pub inf_r: Mark,
    pub ped_l: PedicleMark,
    pub ped_r: PedicleMark,
}

/// The four corners of a vertebra on a side film.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SideMarks {
    pub sup_a: Mark,
    pub sup_p: Mark,
    pub inf_a: Mark,
    pub inf_p: Mark,
}

/// The two endplate lines of a vertebra on a bending film. Each line's
/// ends are ordered patient left, then patient right.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndplateLines {
    pub sup: [Mark; 2],
    pub inf: [Mark; 2],
}

/// One of a bending film's two endplate lines.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum EndplateLine {
    /// The upper endplate.
    Sup,
    /// The lower endplate.
    Inf,
}

/// One end of an endplate line: the patient's left or right.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LineEnd {
    PatientLeft,
    PatientRight,
}

impl EndplateLines {
    /// One end of one line.
    pub const fn end(&self, line: EndplateLine, end: LineEnd) -> Mark {
        let ends = match line {
            EndplateLine::Sup => self.sup,
            EndplateLine::Inf => self.inf,
        };
        match end {
            LineEnd::PatientLeft => ends[0],
            LineEnd::PatientRight => ends[1],
        }
    }

    /// The four ends in quad order: upper left, upper right, lower right,
    /// lower left (patient left and right).
    pub const fn corners(&self) -> [Mark; 4] {
        [self.sup[0], self.sup[1], self.inf[1], self.inf[0]]
    }
}

/// The name of one front or side vertebra mark, exactly as it is written
/// everywhere. A bending film's points are named by line and end instead
/// ([`EndplateLine`], [`LineEnd`]).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum MarkName {
    SupL,
    SupR,
    InfL,
    InfR,
    PedL,
    PedR,
    SupA,
    SupP,
    InfA,
    InfP,
}

impl MarkName {
    /// The front film's marks, in the order corners then pedicles.
    pub const FRONT: [MarkName; 6] = [
        MarkName::SupL,
        MarkName::SupR,
        MarkName::InfL,
        MarkName::InfR,
        MarkName::PedL,
        MarkName::PedR,
    ];

    /// The side film's marks.
    pub const SIDE: [MarkName; 4] = [
        MarkName::SupA,
        MarkName::SupP,
        MarkName::InfA,
        MarkName::InfP,
    ];

    /// The name as written in JSON, TypeScript and on screen.
    pub const fn name(self) -> &'static str {
        match self {
            MarkName::SupL => "supL",
            MarkName::SupR => "supR",
            MarkName::InfL => "infL",
            MarkName::InfR => "infR",
            MarkName::PedL => "pedL",
            MarkName::PedR => "pedR",
            MarkName::SupA => "supA",
            MarkName::SupP => "supP",
            MarkName::InfA => "infA",
            MarkName::InfP => "infP",
        }
    }
}

impl FrontMarks {
    /// The four corners in quad order (see the module): `supL, supR, infR,
    /// infL`.
    pub const fn corners(&self) -> [Mark; 4] {
        [self.sup_l, self.sup_r, self.inf_r, self.inf_l]
    }
}

impl SideMarks {
    /// The four corners in quad order (see the module): `supA, supP, infP,
    /// infA`.
    pub const fn corners(&self) -> [Mark; 4] {
        [self.sup_a, self.sup_p, self.inf_p, self.inf_a]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64) -> Mark {
        Mark::auto(Px::new(x, y))
    }

    /// Twice the signed area of a closed polygon: its magnitude is the
    /// area only when the polygon does not cross itself.
    fn doubled_area(points: [Mark; 4]) -> f64 {
        (0..4)
            .map(|i| {
                let (a, b) = (points[i].at, points[(i + 1) % 4].at);
                a.x() * b.y() - b.x() * a.y()
            })
            .sum()
    }

    #[test]
    fn every_kind_of_corners_goes_round_the_outline() {
        // A 4 by 2 rectangle, patient left (and anterior) on image left.
        let (top_left, top_right, bottom_left, bottom_right) =
            (at(0.0, 0.0), at(4.0, 0.0), at(0.0, 2.0), at(4.0, 2.0));
        let pedicle = PedicleMark::NotVisible {
            source: MarkSource::Auto,
        };
        let front = FrontMarks {
            sup_l: top_left,
            sup_r: top_right,
            inf_l: bottom_left,
            inf_r: bottom_right,
            ped_l: pedicle,
            ped_r: pedicle,
        };
        let side = SideMarks {
            sup_a: top_left,
            sup_p: top_right,
            inf_a: bottom_left,
            inf_p: bottom_right,
        };
        let lines = EndplateLines {
            sup: [top_left, top_right],
            inf: [bottom_left, bottom_right],
        };
        for corners in [front.corners(), side.corners(), lines.corners()] {
            assert_eq!(doubled_area(corners).abs(), 16.0);
            assert_eq!(corners, [top_left, top_right, bottom_right, bottom_left]);
        }
    }

    #[test]
    fn line_ends_are_patient_left_then_right() {
        let lines = EndplateLines {
            sup: [at(0.0, 0.0), at(4.0, 0.0)],
            inf: [at(0.0, 2.0), at(4.0, 2.0)],
        };
        assert_eq!(
            lines.end(EndplateLine::Sup, LineEnd::PatientLeft),
            lines.sup[0]
        );
        assert_eq!(
            lines.end(EndplateLine::Inf, LineEnd::PatientRight),
            lines.inf[1]
        );
    }
}
