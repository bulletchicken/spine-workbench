//! The frames the sweep animation plays: the contract between the auto
//! pass and the step 3 screen.
//!
//! Frames are real engine output at fixed checkpoints. They never depend on
//! how the work was sliced, so a native run and a browser run produce the
//! same sequence. Every frame says which film and stage it belongs to, and
//! every position is in original film pixels (strip rows also carry their
//! map back to the film). The app does the pacing.
//!
//! A film pass emits, in order: `stage-started` and `evidence` tiles;
//! `stage-started`, `end-zones`, `evidence` tiles of the ridge map and the
//! spreading search, `centre-line` (coarse, then refined) and `width`;
//! `stage-started`, `strip-shape`, `strip-rows` (head first) and
//! `barcode`. The chain pass emits `stage-started`, `sweep` frames down
//! the chain, `template-scored`, `joint-progress` when two films are
//! solved together, and `chain-locked`; then `levels-named`,
//! `confidence`, and `outlines` ending with `final: true`. `warning` frames
//! may come between any two others.
//!
//! Every frame is kept except two kinds, which may be thinned to thirty a
//! second, newest kept: `joint-progress` frames, and `evidence` frames
//! with `final: false` whose `map` is not `wavefront`.

use crate::auto_pass::{PassWarning, Stage};
use crate::confidence::ConfidenceBand;
use crate::film::FilmKind;
use crate::levels::Level;
use crate::naming::{LevelNaming, Template};
use crate::pixels::{Px, PxRect};
use crate::vertebra::VertebraMarks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A small 8-bit picture of a map, placed on the film.
///
/// Tile cell (i, j), i across and j down, has its centre at the film point
/// `origin + i · stepAcross + j · stepDown`. The two steps are vectors in
/// film pixels, so a tile computed on a turned or shrunk copy of the film
/// still lands exactly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewTile {
    pub width: u32,
    pub height: u32,
    pub origin: Px,
    pub step_across: Px,
    pub step_down: Px,
    pub encoding: TileEncoding,
    /// Row-major; one byte per cell, two for `signed-pair`.
    #[serde(with = "serde_bytes")]
    #[schemars(with = "Vec<u8>")]
    #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
    pub data: Vec<u8>,
}

/// How a tile's bytes encode the map.
///
/// Every angle is measured in original film pixel axes, from +x (right)
/// toward +y (down), whatever turned or shrunk grid the map was computed
/// on: a stage working on an upright copy turns its angles back with its
/// positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum TileEncoding {
    /// A 0 to 1 value: `round(255 · v)`.
    Unit,
    /// A direction in (−π, π]: `round(255 · (θ + π) / 2π) mod 256`.
    Direction,
    /// A line angle in [−π/2, π/2): `round(255 · (θ + π/2) / π)`.
    Axial,
    /// 0 or 255.
    Mask,
    /// An index into a list (a scale, a shape).
    Index,
    /// Two bytes per cell: the ridge value, then the valley value.
    SignedPair,
}

/// Which map a tile shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum MapKind {
    // Evidence maps
    Image,
    Validity,
    PhaseEdge,
    PhaseDirection,
    PhaseKind,
    GradientEdge,
    GradientDirection,
    Ridge,
    RidgeOrientation,
    Valley,
    ValleyOrientation,
    ValleyScale,
    RidgeSymmetry,
    ValleySymmetry,
    HessianScale,
    Ring,
    RingShape,
    // Centre line search
    /// The ridge map the centre line follows.
    RidgeHeat,
    /// How far the cheapest-path search has spread.
    Wavefront,
}

/// A straight line on the film.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    pub a: Px,
    pub b: Px,
}

/// The spine's measured edges at one point along the centre line.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WidthSample {
    pub left: Px,
    pub right: Px,
    /// 0 to 1.
    pub confidence: f64,
}

/// A body found by the chain, before its corners are placed: its two
/// endplate lines across the strip.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChainBody {
    pub level: Level,
    pub sup: Segment,
    pub inf: Segment,
    pub inferred: bool,
}

/// A vertebra's outline as a quad, its corners in quad order (see
/// [`crate::marks`]): upper left (or anterior), upper right (or
/// posterior), lower right, lower left. Drawn in order, it never crosses
/// itself. Build it with [`OutlineQuad::new`] so the order cannot drift.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlineQuad {
    pub level: Level,
    pub corners: [Px; 4],
}

impl OutlineQuad {
    /// The outline of one vertebra's marks.
    pub fn new(level: Level, marks: &VertebraMarks) -> Self {
        OutlineQuad {
            level,
            corners: marks.corners().map(|mark| mark.at),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelConfidence {
    pub level: Level,
    pub score: f64,
    pub band: ConfidenceBand,
}

/// One frame of the sweep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WatchFrame {
    StageStarted {
        film: FilmKind,
        stage: Stage,
    },
    /// A preview of a map as it builds.
    Evidence {
        film: FilmKind,
        stage: Stage,
        map: MapKind,
        /// 1-based position in this film's sequence of map previews.
        step: u32,
        steps: u32,
        /// The finished map, not an intermediate one.
        #[serde(rename = "final")]
        is_final: bool,
        /// The filter scale the preview shows, in mm, when it has one.
        scale_mm: Option<f64>,
        tile: PreviewTile,
    },
    /// Where the centre line will start and end.
    EndZones {
        film: FilmKind,
        stage: Stage,
        top: PxRect,
        bottom: PxRect,
        top_cropped: bool,
        bottom_cropped: bool,
    },
    /// The centre line, from the neck down.
    CentreLine {
        film: FilmKind,
        stage: Stage,
        points: Vec<Px>,
        /// The smooth, final line rather than the coarse path.
        refined: bool,
    },
    /// The spine's width along the centre line.
    Width {
        film: FilmKind,
        stage: Stage,
        samples: Vec<WidthSample>,
    },
    /// How each strip row maps back to the film, for the curved-to-straight
    /// morph: row r is the line through `centres[r]` along `normals[r]`.
    StripShape {
        film: FilmKind,
        stage: Stage,
        centres: Vec<Px>,
        /// Unit vectors in film pixels, pointing to strip column +1.
        normals: Vec<Px>,
        /// Film pixels per strip column.
        column_px: f64,
        columns: u32,
    },
    /// A block of the unrolled strip, head first.
    StripRows {
        film: FilmKind,
        stage: Stage,
        first_row: u32,
        rows: u32,
        columns: u32,
        /// One byte per cell, row-major.
        #[serde(with = "serde_bytes")]
        #[schemars(with = "Vec<u8>")]
        #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
        image: Vec<u8>,
        /// One bit per cell, row-major, lowest bit first: 1 = usable.
        #[serde(with = "serde_bytes")]
        #[schemars(with = "Vec<u8>")]
        #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
        valid: Vec<u8>,
    },
    /// The bone and gap profile down the strip: bone bright, gaps dark.
    Barcode {
        film: FilmKind,
        stage: Stage,
        /// Little-endian `f32` per strip row.
        #[serde(with = "serde_bytes")]
        #[schemars(with = "Vec<u8>")]
        #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
        values: Vec<u8>,
        /// The same with slow trends removed, little-endian `f32`.
        #[serde(with = "serde_bytes")]
        #[schemars(with = "Vec<u8>")]
        #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
        high_pass: Vec<u8>,
    },
    /// The search moving down the chain.
    Sweep {
        film: FilmKind,
        stage: Stage,
        /// The level the search has reached.
        level: Level,
        /// 256 bins from the top of the strip to the bottom: how cheaply
        /// this level could end there (255 = best).
        #[serde(with = "serde_bytes")]
        #[schemars(with = "Vec<u8>")]
        #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
        heat: Vec<u8>,
        /// The best chain so far ending at this level.
        tentative: Vec<ChainBody>,
    },
    /// One vertebra count scored.
    TemplateScored {
        film: FilmKind,
        stage: Stage,
        template: Template,
        score: f64,
        chance: f64,
    },
    /// One round of the joint front and side solve, on the film just
    /// re-solved.
    JointProgress {
        film: FilmKind,
        stage: Stage,
        start: u32,
        round: u32,
        upper: f64,
        lower: f64,
    },
    /// The chain is settled; bodies listed from the sacrum upward, the
    /// order they lock in.
    ChainLocked {
        film: FilmKind,
        stage: Stage,
        bodies: Vec<ChainBody>,
    },
    LevelsNamed {
        film: FilmKind,
        stage: Stage,
        naming: LevelNaming,
        /// The level of each body, top to bottom, as `FilmMarks.vertebrae`
        /// and the two naming lists run.
        levels: Vec<Level>,
    },
    Confidence {
        film: FilmKind,
        stage: Stage,
        levels: Vec<LevelConfidence>,
    },
    /// Outlines on the film; the last one has `final: true`.
    Outlines {
        film: FilmKind,
        stage: Stage,
        quads: Vec<OutlineQuad>,
        #[serde(rename = "final")]
        is_final: bool,
    },
    Warning {
        film: FilmKind,
        stage: Stage,
        warning: PassWarning,
    },
}

impl WatchFrame {
    /// Whether the frame must be kept. Every frame is kept except two
    /// kinds, which may be thinned to thirty a second, newest kept:
    /// `joint-progress` frames, and `evidence` frames with `final: false`
    /// whose `map` is not `wavefront`. Every kind is named here, so a new
    /// one does not compile until it is decided.
    pub fn is_keyframe(&self) -> bool {
        match self {
            WatchFrame::JointProgress { .. } => false,
            WatchFrame::Evidence { map, is_final, .. } => *is_final || *map == MapKind::Wavefront,
            WatchFrame::StageStarted { .. }
            | WatchFrame::EndZones { .. }
            | WatchFrame::CentreLine { .. }
            | WatchFrame::Width { .. }
            | WatchFrame::StripShape { .. }
            | WatchFrame::StripRows { .. }
            | WatchFrame::Barcode { .. }
            | WatchFrame::Sweep { .. }
            | WatchFrame::TemplateScored { .. }
            | WatchFrame::ChainLocked { .. }
            | WatchFrame::LevelsNamed { .. }
            | WatchFrame::Confidence { .. }
            | WatchFrame::Outlines { .. }
            | WatchFrame::Warning { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::{EndplateLines, Mark};

    #[test]
    fn an_outline_quad_goes_round_the_vertebra() {
        let at = |x, y| Mark::auto(Px::new(x, y));
        let lines = EndplateLines {
            sup: [at(10.0, 20.0), at(30.0, 21.0)],
            inf: [at(11.0, 40.0), at(31.0, 41.0)],
        };
        let quad = OutlineQuad::new(Level::L1, &VertebraMarks::Bending { lines });
        assert_eq!(
            quad.corners,
            [
                Px::new(10.0, 20.0),
                Px::new(30.0, 21.0),
                Px::new(31.0, 41.0),
                Px::new(11.0, 40.0)
            ]
        );
    }
}
