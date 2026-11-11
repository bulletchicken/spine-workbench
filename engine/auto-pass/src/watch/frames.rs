//! Chain checkpoints mapped from strip rows into original film pixels.

use crate::chain_search::dynamic_program::SweepCheckpoint;
use crate::chain_search::gap_score::LatticeEvidence;
use crate::handoffs::{FoundBody, Polarity, Strip};
use crate::points;
use shapes::auto_pass::Stage;
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::levels::Level;
use shapes::naming::Template;
use shapes::pixels::Px;
use shapes::vertebra::VertebraMarks;
use shapes::watch_frames::{ChainBody, OutlineQuad, Segment, WatchFrame};

/// The standard count's actual forward message, before the final chain
/// is known. Heat bins always span the unpadded strip.
pub(super) fn sweep(
    strip: &Strip,
    evidence: &LatticeEvidence,
    checkpoint: &SweepCheckpoint,
) -> WatchFrame {
    let levels = Template::Standard.levels();
    let tentative = checkpoint
        .path
        .iter()
        .enumerate()
        .map(|(v, state)| {
            let (_, sup, inf) = evidence.body_edges(state.top, state.height);
            body(
                strip,
                levels[v],
                evidence.native_row(state.top as f64),
                evidence.native_row((state.top + state.height) as f64),
                [evidence.tilts[sup], evidence.tilts[inf]],
                !evidence.observed[state.top] || !evidence.observed[state.top + state.height],
            )
        })
        .collect();
    let energy = &checkpoint.boundary_energy[evidence.padding..evidence.rows - evidence.padding];
    let best = energy.iter().copied().fold(f64::INFINITY, f64::min);
    let mut bins = [f64::INFINITY; 256];
    for (index, &value) in energy.iter().enumerate() {
        let bin = index * 256 / energy.len();
        bins[bin] = bins[bin].min(value);
    }
    let heat = bins
        .into_iter()
        .map(|value| {
            if best.is_finite() && value.is_finite() {
                libm::round(255.0 * libm::exp(-(value - best))) as u8
            } else {
                0
            }
        })
        .collect();
    WatchFrame::Sweep {
        film: strip.film,
        stage: Stage::Chain,
        level: levels[checkpoint.body],
        heat,
        tentative,
    }
}

pub(super) fn locked_body(strip: &Strip, found: &FoundBody) -> ChainBody {
    body(
        strip,
        found.level,
        found.sup.row,
        found.inf.row,
        [found.sup.tilt, found.inf.tilt],
        found.inferred,
    )
}

/// Refine the same corners that point placement returns. Landmark and
/// pedicle confidence do not alter the outline.
pub(super) fn outline(
    strip: &Strip,
    body: &FoundBody,
    polarity: Polarity,
) -> Result<OutlineQuad, EngineError> {
    let marks = match strip.film {
        FilmKind::Front => VertebraMarks::Front {
            points: points::front::place(strip, body, polarity)?,
        },
        FilmKind::Side => VertebraMarks::Side {
            points: points::side::place(strip, body, polarity)?,
        },
        FilmKind::BendLeft | FilmKind::BendRight => VertebraMarks::Bending {
            lines: points::bending::place(strip, body, polarity)?,
        },
    };
    Ok(OutlineQuad::new(body.level, &marks))
}

fn body(
    strip: &Strip,
    level: Level,
    sup: f64,
    inf: f64,
    tilts: [f64; 2],
    inferred: bool,
) -> ChainBody {
    ChainBody {
        level,
        sup: segment(strip, sup, tilts[0]),
        inf: segment(strip, inf, tilts[1]),
        inferred,
    }
}

fn segment(strip: &Strip, row: f64, tilt: f64) -> Segment {
    let width = strip.layout[row.clamp(0.0, f64::from(strip.rows) - 1.0) as usize].half_width;
    let width = if width > 0.0 { width } else { 15.0 };
    let point = |offset: f64| {
        let r = row + offset * libm::tan(tilt) / strip.row_step;
        let index = (libm::floor(r) as isize).clamp(0, strip.rows as isize - 2) as usize;
        let fraction = r - index as f64;
        let a = strip.layout[index];
        let b = strip.layout[index + 1];
        let col = offset / strip.col_step;
        Px::new(
            a.centre.x()
                + fraction * (b.centre.x() - a.centre.x())
                + col * (a.across.x() + fraction * (b.across.x() - a.across.x())),
            a.centre.y()
                + fraction * (b.centre.y() - a.centre.y())
                + col * (a.across.y() + fraction * (b.across.y() - a.across.y())),
        )
    };
    Segment {
        a: point(-width),
        b: point(width),
    }
}
