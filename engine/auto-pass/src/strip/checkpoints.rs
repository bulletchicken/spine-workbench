//! Fixed strip checkpoints, independent of how a film worker slices work.

use super::maps::unfolded;
use crate::handoffs::{Barcode, Strip};
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::pixels::Px;
use shapes::watch_frames::WatchFrame;

/// Stage start, geometry, 64-row blocks from head to pelvis, then barcode.
/// Pixel blocks are quantised only for viewing; the returned strip keeps
/// the full floating-point samples.
///
/// The frozen `StripShape` carries only one pixel scale. On a film with
/// unequal pixel spacings that preview uses the head row's scale; consumers
/// needing an exact map must use `Strip.layout`, whose across vector can
/// change length as the physical normal turns.
pub fn frames(strip: &Strip, barcode: &Barcode) -> Vec<WatchFrame> {
    let stage = Stage::Strip;
    let film = strip.film;
    let column_px = strip
        .layout
        .first()
        .map_or(1.0, |row| libm::hypot(row.across.x(), row.across.y()));
    let mut frames = vec![
        WatchFrame::StageStarted { film, stage },
        WatchFrame::StripShape {
            film,
            stage,
            centres: strip.layout.iter().map(|row| row.centre).collect(),
            normals: strip
                .layout
                .iter()
                .map(|row| {
                    let length = libm::hypot(row.across.x(), row.across.y());
                    Px::new(row.across.x() / length, row.across.y() / length)
                })
                .collect(),
            column_px,
            columns: strip.cols,
        },
    ];
    for first_row in (0..strip.rows).step_by(64) {
        let rows = (strip.rows - first_row).min(64);
        let first = first_row as usize * strip.cols as usize;
        let cells = rows as usize * strip.cols as usize;
        let image = strip.values[first..first + cells]
            .iter()
            .zip(&strip.valid[first..first + cells])
            .map(|(&value, &valid)| {
                if valid != 0 && value.is_finite() {
                    (255.0 * value.clamp(0.0, 1.0)).round() as u8
                } else {
                    0
                }
            })
            .collect();
        let mut valid = vec![0; cells.div_ceil(8)];
        for (cell, &usable) in strip.valid[first..first + cells].iter().enumerate() {
            if usable != 0 {
                valid[cell / 8] |= 1 << (cell % 8);
            }
        }
        frames.push(WatchFrame::StripRows {
            film,
            stage,
            first_row,
            rows,
            columns: strip.cols,
            image,
            valid,
        });
    }
    frames.push(WatchFrame::Barcode {
        film,
        stage,
        values: barcode
            .values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect(),
        high_pass: barcode
            .high_pass
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect(),
    });
    if let Some(warning) = fold_warning(strip) {
        frames.push(WatchFrame::Warning {
            film,
            stage,
            warning,
        });
    }
    frames
}

pub(crate) fn fold_warning(strip: &Strip) -> Option<PassWarning> {
    let offset = f64::from(strip.cols.saturating_sub(1)) * 0.5 * strip.col_step;
    strip
        .layout
        .iter()
        .any(|row| !unfolded(row.curvature, offset) || !unfolded(row.curvature, -offset))
        .then_some(PassWarning {
            code: PassWarningCode::StripFolded,
            film: strip.film,
            level: None,
            region: None,
        })
}
