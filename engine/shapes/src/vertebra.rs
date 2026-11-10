//! A vertebra as marked on one film, and everything marked on one film.
//!
//! Each film's chain is its own: an edit on the front film changes only
//! the front film. The case checks report when the films' levels stop
//! matching.

use crate::confidence::Confidence;
use crate::film::FilmKind;
use crate::film_geometry::ImageRight;
use crate::landmarks::{HipSide, Landmarks, SacrumTop};
use crate::levels::Level;
use crate::marks::{EndplateLines, FrontMarks, Mark, SideMarks};
use crate::naming::LevelNaming;
use crate::pixels::Px;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The marks of one vertebra; which kind depends on the film.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum VertebraMarks {
    /// Front films.
    Front { points: FrontMarks },
    /// Side films.
    Side { points: SideMarks },
    /// Bending films.
    Bending { lines: EndplateLines },
}

impl VertebraMarks {
    /// True when this kind of marks belongs on `film`.
    pub const fn suits(&self, film: FilmKind) -> bool {
        matches!(
            (self, film),
            (VertebraMarks::Front { .. }, FilmKind::Front)
                | (VertebraMarks::Side { .. }, FilmKind::Side)
                | (
                    VertebraMarks::Bending { .. },
                    FilmKind::BendLeft | FilmKind::BendRight
                )
        )
    }

    /// The four corners (a bending film's line ends) in quad order, round
    /// the outline: see [`crate::marks`].
    pub const fn corners(&self) -> [Mark; 4] {
        match self {
            VertebraMarks::Front { points } => points.corners(),
            VertebraMarks::Side { points } => points.corners(),
            VertebraMarks::Bending { lines } => lines.corners(),
        }
    }
}

/// One vertebra on one film.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VertebraOnFilm {
    /// Stable within a case and never reused, even after the vertebra is
    /// removed (see `CaseFileV1::next_vertebra_id`). Undo, selection and
    /// the renumbering animation follow it.
    pub id: u32,
    pub level: Level,
    pub marks: VertebraMarks,
    pub confidence: Confidence,
    /// The region wand's outline, when it made one.
    pub outline: Option<Vec<Px>>,
}

/// Everything marked on one film.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmMarks {
    pub film: FilmKind,
    /// Top to bottom, levels strictly in order.
    pub vertebrae: Vec<VertebraOnFilm>,
    pub landmarks: Landmarks,
    pub naming: LevelNaming,
    /// The facing the left/right (or anterior/posterior) names were given
    /// for: the film's `imageRight` at its `quarterTurns` at the moment
    /// the names were given, whether or not it was confirmed (the
    /// engine's own guess while `imageRight` is still unknown).
    ///
    /// A `set-geometry` edit is judged against the film's geometry before
    /// it. One that changes the turns drops the marks, because the upright
    /// direction decides which endplate is upper and the order of the
    /// levels. One that keeps the turns and changes `imageRight` to a
    /// facing (not to unknown) renames the marks for it
    /// ([`FilmMarks::renamed_for`], which swaps the names only if this
    /// differs). Any other change (the scale, the confirmations, or
    /// `imageRight` back to unknown) leaves the marks as they are, even
    /// when this differs from `imageRight`.
    pub named_for: ImageRight,
}

impl FilmMarks {
    /// These marks with their names given for `facing`: unchanged when
    /// `facing` is already `named_for`, otherwise every left/right (on a
    /// side film, anterior/posterior) pair swapped and `named_for` set to
    /// `facing`.
    ///
    /// The pairs are each vertebra's corners (`supL`/`supR` and
    /// `infL`/`infR`, or `supA`/`supP` and `infA`/`infP`), a front film's
    /// pedicles (`pedL`/`pedR`, each with its seen state and ring score),
    /// the two ends of each of a bending film's endplate lines, the
    /// sacrum top's two ends, and the `left` and `right` hip centres.
    /// Every point keeps its position and its flags (source, pinned,
    /// inferred); only which name holds it changes. Levels, ids,
    /// confidence, outlines and naming are untouched, and `c7` is worked
    /// out again ([`Landmarks::c7_of`]): the mean of the same four
    /// corners, so the same point up to rounding, well inside
    /// [`C7_TOLERANCE_PX`](crate::landmarks::C7_TOLERANCE_PX), because the
    /// corners are added in another order.
    ///
    /// `facing` suits the film's view, as the reader ensures for
    /// `imageRight`. This is the one definition: the app's case reducer
    /// mirrors it in TypeScript and must reproduce
    /// `proving-ground/known-answers/side-swap.json`.
    pub fn renamed_for(&self, facing: ImageRight) -> FilmMarks {
        debug_assert_eq!(
            facing.suits_front_view(),
            self.named_for.suits_front_view(),
            "a facing for the other view"
        );
        let mut marks = self.clone();
        if facing == self.named_for {
            return marks;
        }
        for vertebra in &mut marks.vertebrae {
            match &mut vertebra.marks {
                VertebraMarks::Front { points } => {
                    std::mem::swap(&mut points.sup_l, &mut points.sup_r);
                    std::mem::swap(&mut points.inf_l, &mut points.inf_r);
                    std::mem::swap(&mut points.ped_l, &mut points.ped_r);
                }
                VertebraMarks::Side { points } => {
                    std::mem::swap(&mut points.sup_a, &mut points.sup_p);
                    std::mem::swap(&mut points.inf_a, &mut points.inf_p);
                }
                VertebraMarks::Bending { lines } => {
                    lines.sup.swap(0, 1);
                    lines.inf.swap(0, 1);
                }
            }
        }
        match &mut marks.landmarks.sacrum_top {
            Some(SacrumTop::Front { sac_l, sac_r }) => std::mem::swap(sac_l, sac_r),
            Some(SacrumTop::Side { sac_a, sac_p }) => std::mem::swap(sac_a, sac_p),
            None => {}
        }
        for hip in &mut marks.landmarks.hips {
            hip.side = match hip.side {
                HipSide::Left => HipSide::Right,
                HipSide::Right => HipSide::Left,
                unpaired_or_merged => unpaired_or_merged,
            };
        }
        marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
        marks.named_for = facing;
        marks
    }
}
