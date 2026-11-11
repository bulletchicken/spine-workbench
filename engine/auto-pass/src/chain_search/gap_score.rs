//! Evidence on a padded chain lattice: tilted endplates, disc contrast,
//! pedicle rewards, and the two anatomical anchors.
//!
//! Weights and support widths are experimental. `Strip` carries normalised
//! bone values up to 1.5, not detector intensities, so its frozen validity
//! mask is authoritative: a value near 1 is not treated as saturated metal.
//! The statistics have a peak/6 scale floor when a noiseless piecewise
//! constant strip has zero MAD; otherwise such a strip would lose all edges
//! to a zero winsorisation cap or clip every blurred edge sample equally.

use crate::handoffs::{Barcode, Polarity, Strip};
use shapes::auto_pass::{PassWarning, PassWarningCode};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;

mod anchors;
mod contrast;
mod native_edges;
mod profile;

use anchors::{RangeMaximum, pedicle_tables, rib_onset, sacral_table};
use contrast::gap_table;
use native_edges::{dilated_validity, line_evidence, signed_edges};
use profile::local_period;

const DEG: f64 = std::f64::consts::PI / 180.0;

/// Evidence on lattice boundaries, each between two lattice rows.
#[derive(Clone, Debug)]
pub struct LatticeEvidence {
    pub film: FilmKind,
    pub scale_assumed: bool,
    pub rows: usize,
    pub step: f64,
    pub padding: usize,
    pub min_height: usize,
    pub max_height: usize,
    pub min_gap: usize,
    pub max_gap: usize,
    pub observed: Vec<bool>,
    pub z: Vec<f64>,
    pub half_width: Vec<f64>,
    pub warnings: Vec<PassWarning>,
    pub polarity: Polarity,
    pub tilts: Vec<f64>,
    native_step: f64,
    top: Vec<f64>,
    bottom: Vec<f64>,
    top_native: Vec<f64>,
    bottom_native: Vec<f64>,
    compatible_bottom: Vec<(f64, usize)>,
    gaps: Vec<f64>,
    rib_onset: Vec<f64>,
    sacral: Vec<(f64, f64)>,
    sacral_options: Vec<Vec<(f64, f64)>>,
    pedicles: [RangeMaximum; 2],
}

impl LatticeEvidence {
    /// Builds evidence for one polarity. The survey compares the two
    /// standard-template energies when no polarity hint is available.
    pub fn build(
        strip: &Strip,
        barcode: &Barcode,
        polarity: Polarity,
    ) -> Result<Self, EngineError> {
        Self::with_tilt_limit(strip, barcode, polarity, 20.0)
    }

    /// Rebuild with a wider limit after too many endplates touch the first
    /// tilt range. The normal range is ±20°, the fallback is ±35°.
    pub fn with_tilt_limit(
        strip: &Strip,
        barcode: &Barcode,
        polarity: Polarity,
        tilt_limit_degrees: f64,
    ) -> Result<Self, EngineError> {
        validate(strip, barcode)?;
        if !matches!(tilt_limit_degrees, 20.0 | 35.0) {
            return Err(bad_input(
                "the endplate tilt limit must be 20 or 35 degrees",
            ));
        }
        let period = local_period(barcode, strip.row_step);
        let (period, guessed) = period
            .map_or((f64::from(strip.rows) * strip.row_step / 19.0, true), |p| {
                (p, false)
            });
        let (step, min_height, max_height, min_gap, max_gap) = if strip.scale_assumed {
            (period / 25.0, 8, 35, 1, 14)
        } else if !guessed && period < 18.0 {
            (0.5, 8, 50, 1, 16)
        } else {
            (1.0, 5, 40, 1, 16)
        };
        if !step.is_finite() || step <= 0.0 {
            return Err(bad_input("the lattice spacing is not positive"));
        }
        let pad_length = if strip.scale_assumed {
            4.0 * period
        } else {
            100.0
        };
        let padded_rows = (pad_length / step).ceil();
        let inside = (f64::from(strip.rows) * strip.row_step / step).ceil();
        if !inside.is_finite()
            || !padded_rows.is_finite()
            || inside + 2.0 * padded_rows + 1.0 > 16384.0
        {
            return Err(EngineError::new(
                ErrorCode::PassFilmTooSmall,
                "the strip is too long or too finely sampled for a chain lattice",
            ));
        }
        let padding = padded_rows as usize;
        let inside_rows = inside as usize;
        let rows = inside_rows + 2 * padding + 1;
        let tilt_steps = (2.0 * tilt_limit_degrees / 2.5) as usize;
        let tilts: Vec<_> = (0..=tilt_steps)
            .map(|i| (-tilt_limit_degrees + 2.5 * i as f64) * DEG)
            .collect();
        let mut warnings = Vec::new();
        let warning = |code| PassWarning {
            code,
            film: strip.film,
            level: None,
            region: None,
        };
        if strip.scale_assumed {
            warnings.push(warning(PassWarningCode::ScaleAssumed));
            if guessed {
                warnings.push(warning(PassWarningCode::ScaleGuess));
            }
        }
        if strip.film == FilmKind::Side && strip.image_right.is_none() {
            warnings.push(warning(PassWarningCode::FacingUnknown));
        }
        let usable = dilated_validity(strip);
        let signed_edges = signed_edges(strip, &usable, polarity);
        let mut observed = vec![false; rows];
        let mut half_width = vec![15.0; rows];
        let mut z = vec![0.0; rows];
        for i in 0..rows {
            let native = (i as f64 - padding as f64) * step / strip.row_step;
            let nearest = native.round().clamp(0.0, f64::from(strip.rows - 1)) as usize;
            let width = strip.layout[nearest].half_width;
            half_width[i] = if width > 0.0 { width } else { 15.0 };
            z[i] = interpolate_z(strip, native - 0.5);
            if !z[i].is_finite() {
                return Err(bad_input("the strip's height map overflows in padding"));
            }
            if native < 0.0 || native >= f64::from(strip.rows) {
                continue;
            }
            let start = native.floor() as usize;
            let end = ((native + step / strip.row_step).ceil() as usize)
                .max(start + 1)
                .min(strip.rows as usize);
            let radius = (0.4 * half_width[i] / strip.col_step)
                .floor()
                .min(f64::from(strip.cols)) as usize;
            let centre = strip.cols as usize / 2;
            let left = centre.saturating_sub(radius);
            let right = (centre + radius + 1).min(strip.cols as usize);
            let mut count = 0;
            for r in start..end {
                for c in left..right {
                    count += usize::from(usable[r * strip.cols as usize + c]);
                }
            }
            observed[i] = count * 2 >= (end - start) * (right - left);
        }
        if observed[padding..padding + inside_rows]
            .iter()
            .filter(|v| !**v)
            .count()
            * 5
            > inside_rows
        {
            warnings.push(warning(PassWarningCode::ManyUnobservedRows));
        }
        let count = rows * tilts.len();
        let mut inner_top = vec![0.0; count];
        let mut inner_bottom = vec![0.0; count];
        let mut outer_top = vec![0.0; count];
        let mut outer_bottom = vec![0.0; count];
        let native_positions: Vec<_> = (0..count)
            .map(|index| (index / tilts.len()) as f64 - padding as f64)
            .map(|row| row * step / strip.row_step - 0.5)
            .collect();
        let mut top_native = native_positions.clone();
        let mut bottom_native = native_positions;
        for i in padding..padding + inside_rows {
            if !observed[i] {
                continue;
            }
            let native = (i - padding) as f64 * step / strip.row_step;
            // Each cell is centred on its lattice boundary. Keep the
            // native row of each signed maximum for endplate refinement.
            // A sub-native lattice cell samples its centre bilinearly.
            let reach = 0.5 * step / strip.row_step;
            let first = (native - reach).ceil().max(1.0) as usize;
            let last = (native + reach)
                .floor()
                .min(f64::from(strip.rows - 1))
                .max(0.0) as usize;
            let samples: Vec<_> = if first <= last {
                (first..=last).map(|row| row as f64).collect()
            } else {
                vec![native]
            };
            for (t, &tilt) in tilts.iter().enumerate() {
                let index = i * tilts.len() + t;
                for &sample in &samples {
                    let inner = line_evidence(
                        strip,
                        &usable,
                        &signed_edges,
                        sample,
                        tilt,
                        0.0,
                        0.6 * half_width[i],
                    );
                    let outer = line_evidence(
                        strip,
                        &usable,
                        &signed_edges,
                        sample,
                        tilt,
                        1.3 * half_width[i],
                        2.2 * half_width[i],
                    );
                    // Samples ascend, so strict comparisons keep the
                    // smallest native row on a positive-evidence tie.
                    if inner.0 > inner_top[index] {
                        inner_top[index] = inner.0;
                        outer_top[index] = outer.0;
                        top_native[index] = sample - 0.5;
                    }
                    if inner.1 > inner_bottom[index] {
                        inner_bottom[index] = inner.1;
                        outer_bottom[index] = outer.1;
                        bottom_native[index] = sample - 0.5;
                    }
                }
            }
        }
        let top_stats = Statistics::new(
            inner_top
                .iter()
                .enumerate()
                .filter(|(i, _)| observed[i / tilts.len()])
                .map(|(_, x)| *x),
        );
        let bottom_stats = Statistics::new(
            inner_bottom
                .iter()
                .enumerate()
                .filter(|(i, _)| observed[i / tilts.len()])
                .map(|(_, x)| *x),
        );
        let outer_top_stats = Statistics::new(outer_top.iter().copied());
        let outer_bottom_stats = Statistics::new(outer_bottom.iter().copied());
        for index in 0..count {
            if observed[index / tilts.len()] {
                inner_top[index] = top_stats.z(inner_top[index]).clamp(-4.0, 6.0)
                    - 0.5 * outer_top_stats.z(outer_top[index]).clamp(0.0, 6.0);
                inner_bottom[index] = bottom_stats.z(inner_bottom[index]).clamp(-4.0, 6.0)
                    - 0.5 * outer_bottom_stats.z(outer_bottom[index]).clamp(0.0, 6.0);
            }
        }
        let mut compatible_bottom = vec![(0.0, 0); count];
        for i in 0..rows {
            for (a, &angle) in tilts.iter().enumerate() {
                let mut best = (f64::NEG_INFINITY, 0);
                for (b, &other) in tilts.iter().enumerate() {
                    let value = inner_bottom[i * tilts.len() + b];
                    if (angle - other).abs() <= 20.0 * DEG + 1e-12 && value > best.0 {
                        best = (value, b);
                    }
                }
                compatible_bottom[i * tilts.len() + a] = best;
            }
        }
        let gaps = gap_table(
            strip,
            &usable,
            polarity,
            &observed,
            &half_width,
            step,
            padding,
            min_gap,
            max_gap,
        );
        let rib_onset = rib_onset(strip, &usable, &observed, step, padding, &half_width);
        let sacral = sacral_table(
            strip,
            &usable,
            &signed_edges,
            &observed,
            step,
            padding,
            &half_width,
        );
        let pedicles = pedicle_tables(strip, &usable, &observed, step, padding, &half_width);
        Ok(Self {
            film: strip.film,
            scale_assumed: strip.scale_assumed,
            rows,
            step,
            padding,
            min_height,
            max_height,
            min_gap,
            max_gap,
            observed,
            z,
            half_width,
            warnings,
            polarity,
            tilts,
            native_step: strip.row_step,
            top: inner_top,
            bottom: inner_bottom,
            top_native,
            bottom_native,
            compatible_bottom,
            gaps,
            rib_onset,
            sacral: sacral.best,
            sacral_options: sacral.options,
            pedicles,
        })
    }

    /// Convert a lattice boundary into the frozen strip's pixel-centre rows.
    pub fn native_row(&self, lattice_row: f64) -> f64 {
        (lattice_row - self.padding as f64) * self.step / self.native_step - 0.5
    }

    pub fn top_score(&self, row: usize, tilt: usize) -> f64 {
        self.top[row * self.tilts.len() + tilt]
    }
    pub fn bottom_score(&self, row: usize, tilt: usize) -> f64 {
        self.bottom[row * self.tilts.len() + tilt]
    }

    /// Native pixel-centre row of the pooled superior-edge maximum.
    /// With no positive evidence, this is the unrefined lattice boundary.
    pub fn top_native_row(&self, row: usize, tilt: usize) -> f64 {
        self.top_native[row * self.tilts.len() + tilt]
    }

    /// Native pixel-centre row of the pooled inferior-edge maximum.
    pub fn bottom_native_row(&self, row: usize, tilt: usize) -> f64 {
        self.bottom_native[row * self.tilts.len() + tilt]
    }

    /// Maximum compatible top/bottom edge reward, and its two tilt indices.
    pub fn body_edges(&self, top: usize, height: usize) -> (f64, usize, usize) {
        if top + height >= self.rows {
            return (f64::NEG_INFINITY, 0, 0);
        }
        let mut best = (f64::NEG_INFINITY, 0, 0);
        for a in 0..self.tilts.len() {
            let (bottom, b) = self.compatible_bottom[(top + height) * self.tilts.len() + a];
            let score = self.top_score(top, a) + bottom;
            if score > best.0 {
                best = (score, a, b);
            }
        }
        best
    }

    pub fn gap(&self, bottom: usize, gap: usize) -> f64 {
        if bottom >= self.rows || !(self.min_gap..=self.max_gap).contains(&gap) {
            return f64::INFINITY;
        }
        self.gaps[bottom * (self.max_gap - self.min_gap + 1) + gap - self.min_gap]
    }

    /// Reward only: absent or obscured pedicles cannot penalise a body.
    pub fn pedicle(&self, top: usize, height: usize) -> f64 {
        let first = (top as f64 + 0.20 * height as f64).ceil() as usize;
        let last = (top as f64 + 0.44 * height as f64).floor() as usize;
        if first > last || last >= self.rows {
            return 0.0;
        }
        -0.5 * self
            .pedicles
            .iter()
            .map(|table| (table.maximum(first, last + 1) - 1.0).clamp(0.0, 3.0))
            .sum::<f64>()
    }

    pub fn rib(&self, row: usize) -> f64 {
        -0.25 * self.rib_strength(row).clamp(0.0, 4.0)
    }
    pub fn rib_strength(&self, row: usize) -> f64 {
        self.rib_onset[row]
    }
    pub fn sacrum(&self, row: usize) -> (f64, f64) {
        (-self.sacral[row].0, self.sacral[row].1)
    }
    /// All plateau candidates as `(cost, tilt)`, with the existing best
    /// first. Unobserved rows carry zero costs and no image evidence.
    pub fn sacral_options(&self, row: usize) -> &[(f64, f64)] {
        &self.sacral_options[row]
    }

    pub fn sacrum_strength(&self, row: usize) -> f64 {
        self.sacral[row].0
    }
}

fn validate(strip: &Strip, barcode: &Barcode) -> Result<(), EngineError> {
    let cells = (strip.rows as usize)
        .checked_mul(strip.cols as usize)
        .ok_or_else(|| bad_input("strip dimensions overflow"))?;
    if strip.rows < 3
        || strip.cols == 0
        || strip.cols.is_multiple_of(2)
        || strip.layout.len() != strip.rows as usize
        || strip.values.len() != cells
        || strip.valid.len() != cells
    {
        return Err(bad_input("strip dimensions or arrays do not match"));
    }
    if barcode.film != strip.film
        || barcode.values.len() != strip.rows as usize
        || barcode.high_pass.len() != strip.rows as usize
        || barcode.valid.len() != strip.rows as usize
    {
        return Err(bad_input("the barcode does not match its strip"));
    }
    if !strip.row_step.is_finite()
        || strip.row_step <= 0.0
        || !strip.col_step.is_finite()
        || strip.col_step <= 0.0
        || strip.layout.iter().any(|r| {
            !r.z.is_finite()
                || !r.half_width.is_finite()
                || r.half_width < 0.0
                || !r.centre.is_finite()
                || !r.across.is_finite()
                || !r.along.is_finite()
        })
    {
        return Err(bad_input(
            "strip geometry must be finite, with positive sample spacings",
        ));
    }
    let max_sigma = f64::from(darkroom::blur::MAX_KERNEL_RADIUS) / 3.0;
    if 0.75 / strip.row_step > max_sigma || 0.5 / strip.col_step > max_sigma {
        return Err(bad_input(
            "strip sample spacings are too small for the edge filter",
        ));
    }
    if !(f64::from(strip.cols) * strip.col_step).is_finite() {
        return Err(bad_input("strip width overflows its length unit"));
    }
    if [
        &strip.evidence.edge_along,
        &strip.evidence.ring,
        &strip.evidence.ridge,
        &strip.evidence.ridge_angle,
    ]
    .iter()
    .any(|map| map.as_ref().is_some_and(|values| values.len() != cells))
    {
        return Err(bad_input("an evidence map does not match the strip"));
    }
    Ok(())
}

fn bad_input(message: &str) -> EngineError {
    EngineError::new(ErrorCode::EngineOutOfSync, message)
}

#[derive(Clone, Copy)]
struct Statistics {
    centre: f64,
    scale: f64,
}
impl Statistics {
    fn new(values: impl Iterator<Item = f64>) -> Self {
        let mut values: Vec<_> = values.filter(|v| v.is_finite()).collect();
        let centre = median(&mut values);
        let peak = values
            .iter()
            .map(|v| (v - centre).abs())
            .fold(0.0, f64::max);
        let mut deviations: Vec<_> = values.iter().map(|v| (v - centre).abs()).collect();
        let scale = (1.4826 * median(&mut deviations) + 1e-6).max(peak / 6.0);
        Self { centre, scale }
    }
    fn z(self, value: f64) -> f64 {
        (value - self.centre) / self.scale
    }
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn interpolate_z(strip: &Strip, row: f64) -> f64 {
    let low = row.floor().clamp(0.0, f64::from(strip.rows - 2)) as usize;
    let fraction = row - low as f64;
    strip.layout[low].z + fraction * (strip.layout[low + 1].z - strip.layout[low].z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{StripEvidence, StripRow};
    use shapes::film_geometry::ImageRight;
    use shapes::pixels::Px;

    fn boxes() -> (Strip, Barcode) {
        let rows = 520;
        let cols = 81;
        // Eighteen analytic, flat-topped rectangles, each 20 rows high, with
        // six-row discs. They are unrelated to the anatomical prior table.
        let profile: Vec<f32> = (0..rows)
            .map(|r| {
                if (15..483).contains(&r) && (r - 15) % 26 < 20 {
                    0.9
                } else {
                    0.2
                }
            })
            .collect();
        let values = (0..rows)
            .flat_map(|r| {
                let value = profile[r];
                (0..cols).map(move |c| if (20..=60).contains(&c) { value } else { 0.2 })
            })
            .collect();
        let strip = Strip {
            film: FilmKind::Front,
            rows: rows as u32,
            cols: cols as u32,
            row_step: 1.0,
            col_step: 1.0,
            scale_assumed: false,
            values,
            valid: vec![1; rows * cols],
            layout: (0..rows)
                .map(|r| StripRow {
                    centre: Px::new(40.0, r as f64),
                    across: Px::new(1.0, 0.0),
                    along: Px::new(0.0, 1.0),
                    curvature: 0.0,
                    half_width: 20.0,
                    z: -(r as f64),
                })
                .collect(),
            image_right: Some(ImageRight::PatientLeft),
            polarity_hint: Some(Polarity::BoneBright),
            evidence: StripEvidence::default(),
        };
        let barcode = Barcode {
            film: FilmKind::Front,
            values: profile.clone(),
            high_pass: profile,
            valid: vec![1; rows],
        };
        (strip, barcode)
    }

    #[test]
    fn flat_box_edges_peak_at_the_known_boundaries() {
        let (strip, barcode) = boxes();
        let evidence = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        let top = evidence.padding + 15;
        let bottom = top + 20;
        let zero_tilt = evidence.tilts.iter().position(|v| *v == 0.0).unwrap();
        assert!(
            evidence.top_score(top, zero_tilt) > evidence.top_score(top - 1, zero_tilt),
            "step {}, top {}, scores {:?}",
            evidence.step,
            top,
            (top - 3..top + 4)
                .map(|r| evidence.top_score(r, zero_tilt))
                .collect::<Vec<_>>()
        );
        assert!(evidence.top_score(top, zero_tilt) > evidence.top_score(top + 1, zero_tilt));
        assert!(
            evidence.bottom_score(bottom, zero_tilt) > evidence.bottom_score(bottom - 1, zero_tilt)
        );
        assert!(
            evidence.bottom_score(bottom, zero_tilt) > evidence.bottom_score(bottom + 1, zero_tilt)
        );
        assert!(evidence.gap(bottom, 6) < evidence.gap(top + 8, 6));
        assert_eq!(evidence.native_row(top as f64), 14.5);
        let (score, a, b) = evidence.body_edges(top, 20);
        assert!(score > 10.0);
        assert_eq!((evidence.tilts[a], evidence.tilts[b]), (0.0, 0.0));
    }

    #[test]
    fn pooled_edges_keep_the_native_boundary_between_lattice_rows() {
        let (original, _) = boxes();
        let cols = original.cols as usize;
        let mut values = vec![0.2; cols];
        for row in original.values.chunks_exact(cols) {
            values.extend_from_slice(row);
            values.extend_from_slice(row);
        }
        let rows = values.len() / cols;
        let profile: Vec<_> = values.chunks_exact(cols).map(|r| r[cols / 2]).collect();
        let mut strip = original;
        strip.rows = rows as u32;
        strip.row_step = 0.5;
        strip.values = values;
        strip.valid = vec![1; rows * cols];
        strip.layout = (0..rows)
            .map(|r| StripRow {
                centre: Px::new(40.0, r as f64 * 0.5),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 0.5),
                curvature: 0.0,
                half_width: 20.0,
                z: -(r as f64) * 0.5,
            })
            .collect();
        let barcode = Barcode {
            film: strip.film,
            values: profile.clone(),
            high_pass: profile,
            valid: vec![1; rows],
        };
        let evidence = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        assert_eq!(evidence.step, 1.0);
        let zero = evidence.tilts.iter().position(|tilt| *tilt == 0.0).unwrap();
        let top = evidence.padding + 15;
        let bottom = evidence.padding + 35;
        // The one-row lead-in places the exact box edges at native
        // boundaries 31 and 71, hence pixel-centre rows 30.5 and 70.5.
        assert_eq!(evidence.native_row(top as f64), 29.5);
        assert_eq!(evidence.top_native_row(top, zero), 30.5);
        assert_eq!(evidence.bottom_native_row(bottom, zero), 70.5);
        // Both adjacent cells include a halfway boundary; their argmax
        // coordinates agree even if the height prior chooses either cell.
        assert_eq!(evidence.top_native_row(top + 1, zero), 30.5);
    }

    #[test]
    fn side_sacral_evidence_is_unchanged_by_stored_quarter_turns() {
        let (mut strip, mut barcode) = boxes();
        strip.film = FilmKind::Side;
        strip.image_right = Some(ImageRight::Anterior);
        barcode.film = FilmKind::Side;
        for row in &mut strip.layout {
            // A local normal that is not horizontal checks that only the
            // stored cardinal rotation is removed, not local curvature.
            row.across = Px::new(0.96, 0.28);
            row.along = Px::new(-0.28, 0.96);
        }
        let original = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        assert!(original.sacral.iter().any(|(score, _)| *score > 0.0));
        for _ in 0..3 {
            for row in &mut strip.layout {
                let rotate = |p: Px| Px::new(-p.y(), p.x());
                row.centre = rotate(row.centre);
                row.across = rotate(row.across);
                row.along = rotate(row.along);
            }
            let turned = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
            // A quarter turn is only coordinate swaps and sign changes;
            // the strip pixels, physical prior and local tilts are identical.
            assert_eq!(turned.sacral, original.sacral);
            assert_eq!(turned.sacral_options, original.sacral_options);
        }
    }

    #[test]
    fn sacral_candidates_retain_the_same_best_cost_and_tilt() {
        let (strip, barcode) = boxes();
        let evidence = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        for row in 0..evidence.rows {
            let options = evidence.sacral_options(row);
            assert_eq!(options[0], evidence.sacrum(row));
            assert!(options.iter().all(|candidate| candidate.0 >= options[0].0));
            if !evidence.observed[row] {
                assert!(options.iter().all(|candidate| candidate.0 == 0.0));
            }
        }
        assert!(
            evidence
                .sacral_options
                .iter()
                .any(|options| options.iter().any(|candidate| candidate.0 > options[0].0))
        );
    }

    #[test]
    fn inverted_evidence_matches_the_original() {
        let (mut strip, mut barcode) = boxes();
        let bright = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        for v in &mut strip.values {
            *v = 1.1 - *v;
        }
        for v in &mut barcode.values {
            *v = 1.1 - *v;
        }
        let dark = LatticeEvidence::build(&strip, &barcode, Polarity::BoneDark).unwrap();
        // Float32 inversion changes contrasts by a few ULPs; scores are
        // scale-normalised. 1e-5 covers this quantisation, not row errors.
        for (a, b) in bright.top.iter().zip(&dark.top) {
            assert!((a - b).abs() < 1e-5);
        }
        for (a, b) in bright.gaps.iter().zip(&dark.gaps) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn missing_rows_and_padding_have_exactly_zero_evidence() {
        let (mut strip, barcode) = boxes();
        let cols = strip.cols as usize;
        for r in 60..90 {
            strip.valid[r * cols..(r + 1) * cols].fill(0);
        }
        let evidence = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        for row in [0, 20, evidence.padding + 75, evidence.rows - 1] {
            assert!(
                !evidence.observed[row],
                "row {row}, step {}, padding {}",
                evidence.step, evidence.padding
            );
            for tilt in 0..evidence.tilts.len() {
                assert_eq!(evidence.top_score(row, tilt), 0.0);
                assert_eq!(evidence.bottom_score(row, tilt), 0.0);
            }
            assert_eq!(evidence.gap(row, 6), 0.0);
            assert_eq!(evidence.rib(row), 0.0);
            assert_eq!(evidence.sacrum(row).0, 0.0);
        }
    }

    #[test]
    fn a_dark_patch_in_a_minority_of_columns_cannot_invent_a_gap() {
        let (mut strip, barcode) = boxes();
        let clean = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        for r in 48..54 {
            for c in 39..42 {
                strip.values[r * strip.cols as usize + c] = 0.1;
            }
        }
        let patched = LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).unwrap();
        // Three damaged columns leave ten equal centre-band columns. Their
        // median therefore has the exact same analytically known contrast.
        assert_eq!(clean.gaps, patched.gaps);
    }

    #[test]
    fn malformed_handoffs_are_refused_without_indexing_them() {
        let (mut strip, barcode) = boxes();
        strip.valid.pop();
        assert!(LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).is_err());
        let (mut strip, barcode) = boxes();
        strip.row_step = f64::MAX;
        assert!(LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).is_err());
        strip.row_step = f64::MIN_POSITIVE;
        assert!(LatticeEvidence::build(&strip, &barcode, Polarity::BoneBright).is_err());
    }

    #[test]
    fn sparse_table_maxima_match_independent_intervals() {
        let table = RangeMaximum::new(vec![2.0, 8.0, 1.0, 5.0, 4.0, 9.0, 3.0]);
        assert_eq!(table.maximum(0, 7), 9.0);
        assert_eq!(table.maximum(2, 5), 5.0);
        assert_eq!(table.maximum(2, 3), 1.0);
    }
}
