//! Vertebral levels and their order, top to bottom.
//!
//! The measured chain runs T1 to L5. C7 is found with it, as the chain's
//! top anchor, so it is a level too. T13 and L6 exist only on spines with
//! a non-standard count, which the naming always flags.
//!
//! The other names exist so that every chain an edit can make still has a
//! name for each vertebra. A boundary wand split adds a vertebra and a
//! merge removes one, and the names then run on from the film's template
//! (see [`Template::names`](crate::naming::Template::names)): upward
//! through C6 to C1, downward past the last lumbar to L6 and L7. Such a
//! chain is always flagged `non-standard-count` until the user settles
//! it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One vertebral level. Declared top to bottom, so `<` means "above".
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum Level {
    /// C1 to C6: only above C7 on a chain that edits pushed upward.
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10,
    T11,
    T12,
    /// Only on a spine flagged with an extra thoracic vertebra.
    T13,
    L1,
    L2,
    L3,
    L4,
    L5,
    /// Only on a spine flagged with an extra lumbar vertebra, or a chain
    /// that edits pushed downward.
    L6,
    /// Only on a chain that edits pushed downward.
    L7,
}

/// The part of the spine a level belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Region {
    Cervical,
    Thoracic,
    Lumbar,
}

impl Level {
    /// Every level, top to bottom.
    pub const ALL: [Level; 27] = [
        Level::C1,
        Level::C2,
        Level::C3,
        Level::C4,
        Level::C5,
        Level::C6,
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
        Level::T13,
        Level::L1,
        Level::L2,
        Level::L3,
        Level::L4,
        Level::L5,
        Level::L6,
        Level::L7,
    ];

    /// The seventeen levels every case is expected to mark, T1 to L5.
    pub const MEASURED: [Level; 17] = [
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

    /// Position in [`Level::ALL`].
    pub fn order(self) -> usize {
        self as usize
    }

    /// The name used in JSON, TypeScript and on screen.
    pub const fn name(self) -> &'static str {
        match self {
            Level::C1 => "C1",
            Level::C2 => "C2",
            Level::C3 => "C3",
            Level::C4 => "C4",
            Level::C5 => "C5",
            Level::C6 => "C6",
            Level::C7 => "C7",
            Level::T1 => "T1",
            Level::T2 => "T2",
            Level::T3 => "T3",
            Level::T4 => "T4",
            Level::T5 => "T5",
            Level::T6 => "T6",
            Level::T7 => "T7",
            Level::T8 => "T8",
            Level::T9 => "T9",
            Level::T10 => "T10",
            Level::T11 => "T11",
            Level::T12 => "T12",
            Level::T13 => "T13",
            Level::L1 => "L1",
            Level::L2 => "L2",
            Level::L3 => "L3",
            Level::L4 => "L4",
            Level::L5 => "L5",
            Level::L6 => "L6",
            Level::L7 => "L7",
        }
    }

    pub const fn region(self) -> Region {
        match self {
            Level::C1 | Level::C2 | Level::C3 | Level::C4 | Level::C5 | Level::C6 | Level::C7 => {
                Region::Cervical
            }
            Level::L1 | Level::L2 | Level::L3 | Level::L4 | Level::L5 | Level::L6 | Level::L7 => {
                Region::Lumbar
            }
            _ => Region::Thoracic,
        }
    }

    /// T13 and L6: only on spines with a non-standard count.
    pub const fn is_transitional(self) -> bool {
        matches!(self, Level::T13 | Level::L6)
    }

    /// The level directly above in [`Level::ALL`], if any.
    pub fn above(self) -> Option<Level> {
        self.order().checked_sub(1).map(|index| Level::ALL[index])
    }

    /// The level directly below in [`Level::ALL`], if any.
    pub fn below(self) -> Option<Level> {
        Level::ALL.get(self.order() + 1).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_run_top_to_bottom() {
        for pair in Level::ALL.windows(2) {
            assert!(pair[0] < pair[1]);
            assert_eq!(pair[0].below(), Some(pair[1]));
            assert_eq!(pair[1].above(), Some(pair[0]));
        }
        assert_eq!(Level::C1.above(), None);
        assert_eq!(Level::L7.below(), None);
        assert_eq!(Level::C7.below(), Some(Level::T1));
    }

    #[test]
    fn names_match_the_serialised_form() {
        for level in Level::ALL {
            let json = serde_json::to_string(&level).expect("a level serialises");
            assert_eq!(json, format!("\"{}\"", level.name()));
        }
    }

    #[test]
    fn the_measured_chain_is_t1_to_l5() {
        assert_eq!(Level::MEASURED.first(), Some(&Level::T1));
        assert_eq!(Level::MEASURED.last(), Some(&Level::L5));
        assert!(Level::MEASURED.iter().all(|level| !level.is_transitional()));
    }
}
