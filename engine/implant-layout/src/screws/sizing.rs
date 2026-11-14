//! Discrete size rules over a complete ensemble. Published ranges describe
//! series, not a complete catalogue; expanding them is a project inference.

use crate::LayoutError;
use crate::evidence::{PublishedValue, Table, Usage, tables};
use crate::pedicles::{cell, central, unique};

/// This project's maximum fraction of versions below a size's limit.
pub const DEFAULT_ALPHA: f64 = 0.10;
/// This project's minimum fraction for an also-seen discrete size.
pub const ALSO_SEEN_SHARE: f64 = 0.10;

/// A grid size, in mm, retaining the publications that support it.
#[derive(Clone, Debug, PartialEq)]
pub struct GridSize {
    pub mm: f64,
    pub sources: Vec<String>,
}

/// Ensemble choice and the other per-version choices occurring often
/// enough to display. No qualifying grid size remains `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct SizeChoice {
    pub mm: Option<f64>,
    pub also_seen: Vec<f64>,
}

/// This project's diameter grid inference: expand the ranges of the four
/// series named in the placement rule using the smallest step enumerated
/// in their explicit grid. A value keeps every covering source key; the
/// step's source is retained for inferred intermediate values as well.
pub fn diameter_grid() -> Vec<GridSize> {
    let cells: Vec<_> = tables()
        .cells(Table::ScrewGrids)
        .iter()
        .filter(|c| {
            c.usage == Usage::Usable
                && c.coordinate.key == "diameter"
                && c.coordinate.group.as_deref() != Some("balikci")
        })
        .collect();
    let step_cell = cells
        .iter()
        .find(|c| matches!(c.value, PublishedValue::Grid { .. }));
    let Some(step_cell) = step_cell else {
        return Vec::new();
    };
    let PublishedValue::Grid { values } = &step_cell.value else {
        return Vec::new();
    };
    let Some(step) = values
        .windows(2)
        .map(|w| w[1] - w[0])
        .min_by(f64::total_cmp)
    else {
        return Vec::new();
    };
    let mut grid = Vec::new();
    for c in &cells {
        match &c.value {
            PublishedValue::Range { lower, upper } => {
                let n = libm::floor((upper - lower) / step + 1e-9) as usize;
                for i in 0..=n {
                    add(
                        &mut grid,
                        lower + i as f64 * step,
                        &[c.source.clone(), step_cell.source.clone()],
                    );
                }
            }
            PublishedValue::Grid { values } => {
                for &mm in values {
                    add(&mut grid, mm, std::slice::from_ref(&c.source));
                }
            }
            PublishedValue::Scalar { value } => {
                add(&mut grid, *value, std::slice::from_ref(&c.source))
            }
            _ => {}
        }
    }
    grid.sort_by(|a, b| a.mm.total_cmp(&b.mm));
    grid
}

/// This project's length grid inference: retain the catalogue entries in
/// the chosen span, including the intermediate lengths. These
/// selected coordinates read their sizes from the source, never from the
/// coordinate spelling. Longer catalogue entries remain evidence only.
pub fn length_grid() -> Vec<GridSize> {
    let mut grid = Vec::new();
    for group in [
        "balikci-standard-25",
        "balikci-mid-27.5",
        "balikci-standard-30",
        "balikci-mid-32.5",
        "balikci-standard-35",
        "balikci-mid-37.5",
        "balikci-standard-40",
        "balikci-mid-42.5",
        "balikci-standard-45",
    ] {
        if let Some(c) = cell(Table::ScrewGrids, "length", group)
            && let Some(mm) = central(c)
        {
            add(&mut grid, mm, std::slice::from_ref(&c.source));
        }
    }
    grid.sort_by(|a, b| a.mm.total_cmp(&b.mm));
    grid
}

/// Published default expansion rule with its source key.
pub fn default_width_ratio() -> Option<(f64, String)> {
    let c = cell(
        Table::DiameterRules,
        "outer-width-ratio-max",
        "expansion-25",
    )?;
    Some((central(c)?, c.source.clone()))
}

/// Largest diameter with at most `alpha` of widths strictly below d/k.
/// This project's ensemble count is unweighted; all versions are required.
pub fn choose_diameter(
    widths: &[f64],
    grid: &[GridSize],
    k: f64,
    alpha: f64,
) -> Result<SizeChoice, LayoutError> {
    validate(widths, grid, alpha)?;
    if !k.is_finite() || k <= 0.0 || widths.iter().any(|w| *w <= 0.0) {
        return Err(invalid("width-ratio"));
    }
    choose(widths, grid, alpha, |w, d| w < d / k, |w| k * w)
}

/// This project's length cap, applied to each drawn chord before grid
/// rounding: min(share*chord, chord-margin). The longest size whose limit
/// is met in at least (1-alpha) of versions is retained. No sanity-band
/// clamping occurs. A negative limit qualifies no catalogue length.
pub fn choose_length(
    chords: &[f64],
    grid: &[GridSize],
    share: f64,
    margin_mm: f64,
    alpha: f64,
) -> Result<SizeChoice, LayoutError> {
    validate(chords, grid, alpha)?;
    if !share.is_finite()
        || !(0.0..=1.0).contains(&share)
        || !margin_mm.is_finite()
        || margin_mm < 0.0
    {
        return Err(invalid("chord-rule"));
    }
    let limits: Vec<_> = chords
        .iter()
        .map(|&c| length_limit(c, share, margin_mm))
        .collect();
    choose(&limits, grid, alpha, |limit, d| limit < d, |limit| limit)
}

/// This project's cap combines a sourced chord share and anterior margin.
pub fn length_limit(chord: f64, share: f64, margin_mm: f64) -> f64 {
    (share * chord).min(chord - margin_mm)
}

fn choose(
    values: &[f64],
    grid: &[GridSize],
    alpha: f64,
    too_small: impl Fn(f64, f64) -> bool,
    limit: impl Fn(f64) -> f64,
) -> Result<SizeChoice, LayoutError> {
    let mm = grid
        .iter()
        .rev()
        .find(|g| {
            values.iter().filter(|&&v| too_small(v, g.mm)).count() as f64 / values.len() as f64
                <= alpha
        })
        .map(|g| g.mm);
    let mut also_seen = Vec::new();
    for g in grid {
        let count = values
            .iter()
            .filter(|&&v| {
                grid.iter()
                    .rev()
                    .find(|size| size.mm <= limit(v))
                    .is_some_and(|size| size.mm == g.mm)
            })
            .count();
        if Some(g.mm) != mm && count as f64 / values.len() as f64 >= ALSO_SEEN_SHARE {
            also_seen.push(g.mm);
        }
    }
    Ok(SizeChoice { mm, also_seen })
}
fn validate(values: &[f64], grid: &[GridSize], alpha: f64) -> Result<(), LayoutError> {
    if values.is_empty()
        || values.iter().any(|v| !v.is_finite())
        || !alpha.is_finite()
        || !(0.0..1.0).contains(&alpha)
        || grid.is_empty()
        || grid
            .iter()
            .any(|s| !s.mm.is_finite() || s.mm <= 0.0 || s.sources.is_empty())
        || grid.windows(2).any(|w| w[0].mm >= w[1].mm)
    {
        return Err(invalid("sizes-or-ensemble"));
    }
    Ok(())
}
fn add(grid: &mut Vec<GridSize>, mm: f64, sources: &[String]) {
    if let Some(g) = grid.iter_mut().find(|g| g.mm == mm) {
        g.sources = unique(g.sources.iter().cloned().chain(sources.iter().cloned()));
    } else {
        grid.push(GridSize {
            mm,
            sources: unique(sources.iter().cloned()),
        });
    }
}
fn invalid(reason: &str) -> LayoutError {
    LayoutError::InvalidValue {
        field: "sizing".into(),
        reason: reason.into(),
    }
}
