//! Single-input sensitivity in the input's own marginal standard deviation.
//! Candidate transitions come from the measured values and comparison
//! boundaries; bisection retains the side which actually gives a new level.

use super::{FusionRegion, LevelReadings, RuleContext, RuleReading};
use crate::LayoutError;
use crate::curves::{self, Reading, SpineReading, lumbar::PedicleOutlines};
use forward_model::pedicles::pedicle_centres;
use forward_model::rotation::{
    matrix_from_rotation_vector, rot_x, rot_y, rot_z, rotation_vector_from_matrix,
};
use forward_model::vertebra_params::VertebraParams;
use shapes::implant_layout::{LayoutInput, LevelRule, Measurement, RuleFlip, RuleInput};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;

/// This project's numerical separation from an exact decision boundary,
/// in the measurement's own unit. It is not a clinical angular tolerance.
const BOUNDARY_STEP: f64 = 1e-8;
/// This project's bisection work bound; 48 halvings resolve far below the
/// precision of a film mark for the finite measured search intervals.
const BISECTIONS: usize = 48;
const UNREACHABLE_INPUT: &str = "flip-input-outside-readable-branch";

/// Replay a reported single-input change. Positions retain the original
/// film registration: when L5 moves, its inferior film marks follow the
/// changed projection, keeping the marked sacral line fixed in patient y.
/// The input is cloned and no stored case or film is written.
pub fn apply_flip(input: &LayoutInput, flip: &RuleFlip) -> Result<LayoutInput, LayoutError> {
    apply_delta(
        input,
        flip.measurement,
        flip.level,
        flip.region,
        flip.change_value,
    )
}

/// Read one rule's nominal sensitivity without running unrelated rules'
/// sensitivity searches. Geometry, table draws and replay are identical
/// to the complete ensemble reader, with the same seed and version count.
pub fn nominal_flip(
    input: &LayoutInput,
    rule: LevelRule,
    region: FusionRegion,
) -> Result<Option<RuleFlip>, LayoutError> {
    let versions = crate::readings::versions(
        &input.fit,
        u64::from(input.seed),
        input.versions,
        0..input.versions,
    )?;
    let sampler = crate::readings::NamedSampler::new(u64::from(input.seed), input.versions)?;
    let (reading, outlines, _) =
        super::ensemble::nominal_snapshot(input, &versions.versions, &sampler)?;
    let context = RuleContext {
        input,
        fit: &input.fit,
        reading: &reading,
        outlines: &outlines,
        region,
    };
    let rules = super::evaluate(&context)?;
    let rule = rules
        .iter()
        .find(|candidate| candidate.rule == rule)
        .ok_or_else(|| LayoutError::Refused("rule-does-not-propose-an-end".into()))?;
    search(
        input,
        NominalGeometry {
            reading: &reading,
            outlines: &outlines,
            versions: &versions.versions,
        },
        rule,
        region,
    )
}

#[derive(Clone, Copy)]
struct NominalGeometry<'a> {
    reading: &'a SpineReading,
    outlines: &'a [PedicleOutlines],
    versions: &'a [crate::readings::Version],
}

pub(crate) fn find(
    input: &LayoutInput,
    all: &LevelReadings,
    rule: &RuleReading,
    region: FusionRegion,
) -> Result<Option<RuleFlip>, LayoutError> {
    search(
        input,
        NominalGeometry {
            reading: &all.nominal,
            outlines: &all.nominal_outlines,
            versions: &all.versions.versions,
        },
        rule,
        region,
    )
}

fn search(
    input: &LayoutInput,
    nominal: NominalGeometry<'_>,
    rule: &RuleReading,
    region: FusionRegion,
) -> Result<Option<RuleFlip>, LayoutError> {
    let Ok(original) = rule.level else {
        return Ok(None);
    };
    let mut nearest: Option<RuleFlip> = None;
    let mut cache = ProbeCache::new(input, nominal);
    for (index, measurement) in rule.inputs.iter().enumerate() {
        let (Some(value), Some(sd)) = (measurement.value, measurement.sd) else {
            continue;
        };
        if sd <= 0.0
            || !supported(measurement.measurement)
            || rule.inputs[..index]
                .iter()
                .any(|other| same_input(other, measurement))
        {
            continue;
        }
        let maximum = nearest.map(|flip| flip.change_in_sd.abs() * sd);
        let boundaries = events(
            input,
            nominal.reading,
            nominal.outlines,
            measurement,
            maximum,
            rule.rule,
        );
        let samples = interval_samples(&boundaries);
        for direction in [-1.0, 1.0] {
            let maximum = nearest.map(|flip| flip.change_in_sd.abs() * sd);
            let transition = first_changed(&samples, maximum, |distance| {
                let delta = direction * distance;
                if !(value + delta).is_finite() {
                    return Ok(None);
                }
                Ok(probe(
                    input,
                    nominal,
                    rule.rule,
                    measurement,
                    delta,
                    region,
                    &mut cache,
                )?
                .ok()
                .filter(|&level| level != original))
            })?;
            let Some((distance, _)) = transition else {
                continue;
            };
            let delta = direction * distance;
            // Replay against the actual complete nominal reader. The
            // bracket probes use the same cached latent draws, including
            // changes of curve side and apex in the pedicle model.
            let moved = apply_delta(
                input,
                measurement.measurement,
                measurement.level,
                measurement.region,
                delta,
            )?;
            let replay = super::ensemble::nominal(&moved)?;
            let replay_context = RuleContext {
                input: &moved,
                fit: &moved.fit,
                reading: &replay.reading,
                outlines: &replay.outlines,
                region,
            };
            let changed = evaluate_one(&replay_context, rule.rule)?.level.ok();
            if let Some(moves_to) = changed.filter(|&level| level != original) {
                let flip = RuleFlip {
                    measurement: measurement.measurement,
                    level: measurement.level,
                    region: measurement.region,
                    change_value: delta,
                    change_in_sd: delta / sd,
                    moves_to,
                };
                if nearest
                    .is_none_or(|previous| flip.change_in_sd.abs() < previous.change_in_sd.abs())
                {
                    nearest = Some(flip);
                }
            }
        }
    }
    Ok(nearest)
}

// Probe each known boundary, the open interval beside it, and its
// midpoint in ascending distance. Pruning uses the last tested point,
// never an untested outer bracket: a wide bracket may hold a close flip.
fn interval_samples(boundaries: &[f64]) -> Vec<f64> {
    let mut samples = Vec::new();
    let mut previous = 0.0;
    for &distance in boundaries {
        if distance > previous {
            samples.push((previous + distance) * 0.5);
            samples.push(distance);
            previous = distance;
        }
    }
    samples
}

fn first_changed(
    samples: &[f64],
    maximum: Option<f64>,
    mut changed: impl FnMut(f64) -> Result<Option<Level>, LayoutError>,
) -> Result<Option<(f64, Level)>, LayoutError> {
    let mut previous = 0.0;
    for &outer in samples {
        if maximum.is_some_and(|limit| previous >= limit) {
            break;
        }
        let distance = maximum.map_or(outer, |limit| outer.min(limit));
        if distance <= previous {
            continue;
        }
        if let Some(mut level) = changed(distance)? {
            let mut low = previous;
            let mut high = distance;
            for _ in 0..BISECTIONS {
                let middle = (low + high) * 0.5;
                if middle == low || middle == high {
                    break;
                }
                if let Some(at_middle) = changed(middle)? {
                    high = middle;
                    level = at_middle;
                } else {
                    low = middle;
                }
            }
            return Ok(Some((high, level)));
        }
        previous = distance;
    }
    Ok(None)
}

fn same_input(a: &RuleInput, b: &RuleInput) -> bool {
    a.measurement == b.measurement && a.level == b.level && a.region == b.region
}
fn supported(measurement: Measurement) -> bool {
    matches!(
        measurement,
        Measurement::CoronalTilt
            | Measurement::Twist
            | Measurement::CentreLineOffset
            | Measurement::Cobb
            | Measurement::KyphosisT2T5
            | Measurement::KyphosisT5T12
            | Measurement::KyphosisT10L2
    )
}

fn evaluate_one(ctx: &RuleContext<'_>, rule: LevelRule) -> Result<RuleReading, LayoutError> {
    match rule {
        LevelRule::EndPlusOne => super::end_plus_one::evaluate(ctx),
        LevelRule::SukNeutral => super::suk_neutral::evaluate(ctx),
        LevelRule::LastTouched => super::last_touched::evaluate(ctx),
        LevelRule::StableMinusOne => super::stable_minus_one::evaluate(ctx),
        LevelRule::SubstantiallyTouched => super::substantially_touched::evaluate(ctx),
        LevelRule::LenkeL3L4 => super::lenke_l3_l4::evaluate(ctx),
        LevelRule::LevPlusOne5C => super::lev_plus_one_5c::evaluate(ctx),
        LevelRule::DeviatedVertebra => super::deviated_vertebra::evaluate(ctx),
        LevelRule::UpperEndVertebra => super::upper_end_vertebra::evaluate(ctx),
        LevelRule::Uev5C => super::uev_5c::evaluate(ctx),
        LevelRule::LenkeShoulder => super::lenke_shoulder::evaluate(ctx),
        _ => Err(LayoutError::Refused("rule-does-not-propose-an-end".into())),
    }
}

struct WidthSnapshot {
    input: LayoutInput,
    positions: Vec<crate::pedicles::CurvePosition>,
    outlines: Vec<PedicleOutlines>,
}

struct ProbeCache {
    widths: Vec<WidthSnapshot>,
}

impl ProbeCache {
    fn new(input: &LayoutInput, nominal: NominalGeometry<'_>) -> Self {
        Self {
            widths: vec![WidthSnapshot {
                input: input.clone(),
                positions: model_positions(nominal.reading),
                outlines: nominal.outlines.to_vec(),
            }],
        }
    }
}

fn model_positions(reading: &SpineReading) -> Vec<crate::pedicles::CurvePosition> {
    reading
        .vertebrae
        .iter()
        .flat_map(|v| {
            [
                shapes::spine_readings::BodySide::Left,
                shapes::spine_readings::BodySide::Right,
            ]
            .map(|side| crate::pedicles::position_in_curve(reading, v.level, side))
        })
        .collect()
}

fn projections_transport(before: &LayoutInput, after: &LayoutInput) -> bool {
    before
        .fit
        .vertebrae
        .iter()
        .zip(&after.fit.vertebrae)
        .all(|(a, b)| {
            let old = matrix_from_rotation_vector(&a.pose.rotation.into())[(1, 1)].abs();
            let new = matrix_from_rotation_vector(&b.pose.rotation.into())[(1, 1)].abs();
            !(old <= f64::EPSILON && new > f64::EPSILON)
        })
}

fn probe(
    input: &LayoutInput,
    nominal: NominalGeometry<'_>,
    rule: LevelRule,
    measurement: &RuleInput,
    delta: f64,
    region: FusionRegion,
    cache: &mut ProbeCache,
) -> Result<Reading<Level>, LayoutError> {
    let moved = match apply_delta(
        input,
        measurement.measurement,
        measurement.level,
        measurement.region,
        delta,
    ) {
        Ok(moved) => moved,
        Err(LayoutError::Refused(reason)) if reason == UNREACHABLE_INPUT => {
            return Ok(Err(
                shapes::implant_layout::CannotTellCode::NoLevelMeetsTheRule,
            ));
        }
        Err(error) => return Err(error),
    };
    if rule == LevelRule::LastTouched {
        // Only the moved body silhouettes and sacral line enter this
        // rule. Curve classification and table widths cannot affect it.
        let mut reading = nominal.reading.clone();
        reading.csvl_y_mm = curves::geometry::csvl(&moved, &moved.fit);
        for (read, fit) in reading.vertebrae.iter_mut().zip(&moved.fit.vertebrae) {
            read.centre_mm = fit.pose.centre;
            read.centre_line_offset_mm = reading.csvl_y_mm.map(|line| fit.pose.centre[1] - line);
        }
        let context = RuleContext {
            input: &moved,
            fit: &moved.fit,
            reading: &reading,
            outlines: &[],
            region,
        };
        return Ok(
            super::last_touched::landmark(&context).map(|index| reading.vertebrae[index].level)
        );
    }
    let mut raw = curves::measure(&moved, &moved.fit)?;
    let positions = model_positions(&raw);
    let table_dependent = matches!(
        rule,
        LevelRule::StableMinusOne
            | LevelRule::SubstantiallyTouched
            | LevelRule::Uev5C
            | LevelRule::LevPlusOne5C
    );
    let outlines = if !table_dependent {
        move_outlines(input, &moved, nominal.outlines)
    } else if let Some(snapshot) = cache.widths.iter().find(|snapshot| {
        snapshot.positions == positions && projections_transport(&snapshot.input, &moved)
    }) {
        // Identical curve positions have identical table and latent width
        // draws. The cached widths transport exactly under a new pose.
        move_outlines(&snapshot.input, &moved, &snapshot.outlines)
    } else {
        let sampler = crate::readings::NamedSampler::new(u64::from(input.seed), input.versions)?;
        let (_, outlines, _) =
            super::ensemble::nominal_snapshot(&moved, nominal.versions, &sampler)?;
        cache.widths.push(WidthSnapshot {
            input: moved.clone(),
            positions,
            outlines: outlines.clone(),
        });
        outlines
    };
    super::ensemble::classify_lumbar_against_table_widths(&mut raw, &moved.fit, &outlines)?;
    let ctx = RuleContext {
        input: &moved,
        fit: &moved.fit,
        reading: &raw,
        outlines: &outlines,
        region,
    };
    Ok(evaluate_one(&ctx, rule)?.level)
}

fn move_outlines(
    before: &LayoutInput,
    after: &LayoutInput,
    outlines: &[PedicleOutlines],
) -> Vec<PedicleOutlines> {
    outlines
        .iter()
        .filter_map(|outline| {
            let a = before
                .fit
                .vertebrae
                .iter()
                .find(|v| v.level == outline.level)?;
            let b = after
                .fit
                .vertebrae
                .iter()
                .find(|v| v.level == outline.level)?;
            let old = matrix_from_rotation_vector(&a.pose.rotation.into())[(1, 1)].abs();
            let new = matrix_from_rotation_vector(&b.pose.rotation.into())[(1, 1)].abs();
            if old <= f64::EPSILON || new <= f64::EPSILON {
                return None;
            }
            let centres = pedicle_centres(&VertebraParams::from(&b.pose));
            let left = (outline.left_y_mm[1] - outline.left_y_mm[0]) * 0.5 * new / old;
            let right = (outline.right_y_mm[1] - outline.right_y_mm[0]) * 0.5 * new / old;
            Some(PedicleOutlines {
                level: outline.level,
                left_y_mm: [centres[0].y - left, centres[0].y + left],
                right_y_mm: [centres[1].y - right, centres[1].y + right],
            })
        })
        .collect()
}

// Every equality that can change an extremum or a zero crossing supplies
// a bracket. Offsets also include both sides of the finite silhouettes.
fn events(
    input: &LayoutInput,
    reading: &SpineReading,
    outlines: &[PedicleOutlines],
    measurement: &RuleInput,
    maximum: Option<f64>,
    rule: LevelRule,
) -> Vec<f64> {
    let value = measurement.value.expect("measured input");
    let mut targets = vec![0.];
    if let Some(threshold) = measurement.threshold {
        targets.extend([threshold, -threshold]);
    }
    match measurement.measurement {
        Measurement::CoronalTilt | Measurement::Cobb => {
            if rule != LevelRule::LastTouched {
                targets.extend(tilt_targets(reading, measurement));
                targets.extend([value - 180., value + 180.]);
            }
        }
        Measurement::Twist => {
            if rule != LevelRule::LastTouched {
                for v in &reading.vertebrae {
                    if let Ok(twist) = v.twist_deg {
                        targets.extend([twist, -twist]);
                    }
                }
            }
        }
        Measurement::CentreLineOffset => {
            if let Some(index) = measurement
                .level
                .and_then(|l| reading.vertebrae.iter().position(|v| v.level == l))
            {
                if rule != LevelRule::LastTouched {
                    targets.extend(apex_targets(reading, index, value));
                }
                let ctx = RuleContext {
                    input,
                    fit: &input.fit,
                    reading,
                    outlines,
                    region: FusionRegion::Structural,
                };
                let interval = super::last_touched::body_interval(&ctx, index);
                targets.extend([-interval[0], -interval[1]]);
                let params = VertebraParams::from(&input.fit.vertebrae[index].pose);
                let centres = pedicle_centres(&params);
                targets.extend(centres.iter().map(|c| params.centre.y - c.y));
                if let Some(outline) = outlines.iter().find(|o| Some(o.level) == measurement.level)
                {
                    targets.extend(
                        outline
                            .left_y_mm
                            .into_iter()
                            .chain(outline.right_y_mm)
                            .map(|edge| params.centre.y - edge),
                    );
                }
            }
            let span = reading
                .vertebrae
                .iter()
                .map(|v| v.centre_mm[1].abs())
                .fold(0., f64::max)
                + input
                    .fit
                    .vertebrae
                    .iter()
                    .map(|v| v.pose.top_half_width + v.pose.bottom_half_width)
                    .fold(0., f64::max);
            targets.extend([value - span * 2., value + span * 2.]);
        }
        _ => {
            targets.extend([value - 180., value + 180.]);
        }
    }
    let mut distances = Vec::new();
    for target in targets {
        let distance = (target - value).abs();
        if distance.is_finite() {
            distances.extend([distance, distance + BOUNDARY_STEP]);
        }
    }
    if matches!(
        measurement.measurement,
        Measurement::CoronalTilt | Measurement::Twist
    ) && let Some(index) = measurement
        .level
        .and_then(|level| input.fit.vertebrae.iter().position(|v| v.level == level))
    {
        let context = RuleContext {
            input,
            fit: &input.fit,
            reading,
            outlines,
            region: FusionRegion::Structural,
        };
        for change in
            super::contact_events::angular_changes(&context, index, measurement.measurement)
        {
            let distance = change.abs();
            // Sample both open sides as well as the equality. This also
            // brackets contact intervals much narrower than a degree.
            distances.extend([
                distance,
                distance + BOUNDARY_STEP,
                (distance - BOUNDARY_STEP).max(0.),
            ]);
        }
    }
    if let Some(limit) = maximum {
        distances.push(limit);
    }
    distances.retain(|&d| d > 0. && d.is_finite() && maximum.is_none_or(|limit| d <= limit));
    distances.sort_by(f64::total_cmp);
    distances.dedup_by(|a, b| a.to_bits() == b.to_bits());
    distances
}

// With one coronal tilt moved, both its endplates turn by that amount.
// Extrema change at a pairwise endplate equality. The major curve can
// also change when a moving endpoint Cobb equals another standing curve's
// Cobb. Those boundaries accompany the endplate-extremum boundaries.
fn tilt_targets(reading: &SpineReading, measurement: &RuleInput) -> Vec<f64> {
    let value = measurement.value.expect("measured angle");
    let (own, direction) = if measurement.measurement == Measurement::Cobb {
        let curve = reading
            .curves
            .iter()
            .filter(|c| {
                c.region == measurement.region
                    && measurement
                        .level
                        .is_none_or(|level| reading.vertebrae[c.apex_index].level == level)
            })
            .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg));
        let Some(curve) = curve else {
            return Vec::new();
        };
        (curve.lower_index, curve.signed_cobb_deg.signum())
    } else {
        let Some(index) = measurement
            .level
            .and_then(|level| reading.vertebrae.iter().position(|v| v.level == level))
        else {
            return Vec::new();
        };
        (index, 1.0)
    };
    let plate = &reading.vertebrae[own];
    let shift = plate.lower_tilt_deg - plate.upper_tilt_deg;
    let mut fixed = vec![0.0];
    fixed.extend(reading.curves.iter().map(|curve| curve.cobb_deg));
    fixed.sort_by(f64::total_cmp);
    fixed.dedup_by(|a, b| a.to_bits() == b.to_bits());
    let mut targets = Vec::new();
    for other in &reading.vertebrae {
        for tilt in [other.upper_tilt_deg, other.lower_tilt_deg] {
            for &cobb in &fixed {
                for sign in [-1.0, 1.0] {
                    for offset in [0.0, shift] {
                        targets.push(
                            value
                                + (tilt + sign * cobb - offset - plate.upper_tilt_deg) / direction,
                        );
                    }
                }
            }
        }
    }
    targets
}

// Centre motion leaves the angular end vertebrae fixed. Each candidate
// apex's deviation from their chord is affine in that one centre change.
// Pairwise equal absolute deviations are every possible apex tie.
fn apex_targets(reading: &SpineReading, moving: usize, value: f64) -> Vec<f64> {
    let mut spans: Vec<_> = reading
        .curves
        .iter()
        .map(|c| (c.upper_index, c.lower_index))
        .collect();
    let lumbar: Vec<_> = reading
        .vertebrae
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (v.level.region() == shapes::levels::Region::Lumbar).then_some(i))
        .collect();
    if let (Some(&first), Some(&last)) = (lumbar.first(), lumbar.last()) {
        spans.push((first, last));
    }
    let mut targets = Vec::new();
    for (upper, lower) in spans {
        if lower <= upper || moving < upper || moving > lower {
            continue;
        }
        let a = reading.vertebrae[upper].centre_mm;
        let b = reading.vertebrae[lower].centre_mm;
        let affine: Vec<_> = (upper..=lower)
            .map(|i| {
                let centre = reading.vertebrae[i].centre_mm;
                let t = if a[2] != b[2] {
                    (centre[2] - a[2]) / (b[2] - a[2])
                } else {
                    (i - upper) as f64 / (lower - upper) as f64
                };
                let deviation = centre[1] - (a[1] + t * (b[1] - a[1]));
                let slope = f64::from(i == moving)
                    - (1.0 - t) * f64::from(upper == moving)
                    - t * f64::from(lower == moving);
                (deviation, slope)
            })
            .collect();
        for (i, &(a, da)) in affine.iter().enumerate() {
            for &(b, db) in affine.iter().skip(i + 1) {
                for sign in [-1.0, 1.0] {
                    let denominator = da - sign * db;
                    if denominator != 0.0 {
                        targets.push(value + (sign * b - a) / denominator);
                    }
                }
            }
        }
    }
    targets
}

fn apply_delta(
    input: &LayoutInput,
    measurement: Measurement,
    level: Option<Level>,
    region: Option<CurveRegion>,
    delta: f64,
) -> Result<LayoutInput, LayoutError> {
    if !delta.is_finite() {
        return Err(LayoutError::Refused("non-finite-flip".into()));
    }
    let mut moved = input.clone();
    let original_line = curves::geometry::csvl(input, &input.fit).ok();
    let index = level.and_then(|l| input.fit.vertebrae.iter().position(|v| v.level == l));
    match measurement {
        Measurement::CoronalTilt | Measurement::Twist | Measurement::CentreLineOffset => {
            let i = index.ok_or_else(|| LayoutError::Refused("flip-level-absent".into()))?;
            let v = &mut moved.fit.vertebrae[i];
            match measurement {
                Measurement::CoronalTilt => shift_coronal(v, delta.to_radians()),
                Measurement::Twist => v.angles.twist_rad += delta.to_radians(),
                _ => v.pose.centre[1] += delta,
            }
            if measurement != Measurement::CentreLineOffset {
                rotate(v);
            }
        }
        Measurement::Cobb => {
            let raw = curves::measure(input, &input.fit)?;
            let curve = raw
                .curves
                .iter()
                .filter(|c| {
                    c.region == region
                        && level.is_none_or(|l| raw.vertebrae[c.apex_index].level == l)
                })
                .max_by(|a, b| a.cobb_deg.total_cmp(&b.cobb_deg))
                .ok_or_else(|| LayoutError::Refused("flip-curve-absent".into()))?;
            if curve.cobb_deg + delta < 0.0 {
                return Err(LayoutError::Refused(UNREACHABLE_INPUT.into()));
            }
            let v = &mut moved.fit.vertebrae[curve.lower_index];
            shift_coronal(v, delta.to_radians() * curve.signed_cobb_deg.signum());
            rotate(v);
        }
        Measurement::KyphosisT2T5 | Measurement::KyphosisT5T12 | Measurement::KyphosisT10L2 => {
            let (upper, lower) = match measurement {
                Measurement::KyphosisT2T5 => (Level::T2, Level::T5),
                Measurement::KyphosisT5T12 => (Level::T5, Level::T12),
                _ => (Level::T10, Level::L2),
            };
            let old = curves::geometry::regional_kyphosis(&input.fit, upper, lower)
                .map_err(|_| LayoutError::Refused("flip-side-reading-absent".into()))?;
            let i = moved
                .fit
                .vertebrae
                .iter()
                .position(|v| v.level == upper)
                .ok_or_else(|| LayoutError::Refused("flip-level-absent".into()))?;
            // Solve for the changed measured sagittal angle rather than
            // treating a rotation-vector component as an endplate tilt.
            for _ in 0..12 {
                let now = curves::geometry::regional_kyphosis(&moved.fit, upper, lower)
                    .map_err(|_| LayoutError::Refused("flip-side-reading-absent".into()))?;
                let residual = old + delta - now;
                if residual.abs() < BOUNDARY_STEP {
                    break;
                }
                moved.fit.vertebrae[i].angles.forward_lean_rad += residual.to_radians();
                rotate(&mut moved.fit.vertebrae[i]);
            }
            let actual = curves::geometry::regional_kyphosis(&moved.fit, upper, lower)
                .map_err(|_| LayoutError::Refused("flip-side-reading-absent".into()))?;
            if (actual - old - delta).abs() >= BOUNDARY_STEP {
                return Err(LayoutError::Refused(UNREACHABLE_INPUT.into()));
            }
        }
        _ => return Err(LayoutError::Refused("flip-input-has-no-marginal-sd".into())),
    }
    if let (Some(old), Ok(new)) = (original_line, curves::geometry::csvl(&moved, &moved.fit)) {
        keep_registration(&mut moved, new - old)?;
    }
    Ok(moved)
}

// A front-film tilt remains authoritative after a replayed input change.
fn shift_coronal(v: &mut shapes::spine_fit::FittedVertebra, delta_rad: f64) {
    v.angles.side_tilt_rad += delta_rad;
    v.corner_tilt_rad = v.corner_tilt_rad.map(|tilt| tilt + delta_rad);
}

fn rotate(v: &mut shapes::spine_fit::FittedVertebra) {
    let matrix = rot_x(v.angles.side_tilt_rad)
        * rot_y(v.angles.forward_lean_rad)
        * rot_z(v.angles.twist_rad);
    v.pose.rotation = rotation_vector_from_matrix(&matrix).into();
}

fn keep_registration(input: &mut LayoutInput, dy: f64) -> Result<(), LayoutError> {
    if dy == 0. {
        return Ok(());
    }
    let Some(film) = input.front.as_mut() else {
        return Ok(());
    };
    let sign = match film.geometry.image_right {
        Some(shapes::film_geometry::ImageRight::PatientLeft) => 1.,
        Some(shapes::film_geometry::ImageRight::PatientRight) => -1.,
        _ => return Ok(()),
    };
    let scale = if film.geometry.quarter_turns.get().is_multiple_of(2) {
        film.geometry.mm_per_px
    } else {
        film.geometry.mm_per_px_vertical.or(film.geometry.mm_per_px)
    }
    .ok_or_else(|| LayoutError::Refused("flip-registration-has-no-scale".into()))?;
    let dx = sign * dy / scale;
    if let Some(v) = film
        .marks
        .vertebrae
        .iter_mut()
        .find(|v| v.level == Level::L5)
        && let shapes::vertebra::VertebraMarks::Front { points } = &mut v.marks
    {
        for mark in [&mut points.inf_l, &mut points.inf_r] {
            let (x, y) = (mark.at.x(), mark.at.y());
            mark.at = match film.geometry.quarter_turns.get() {
                0 => shapes::pixels::Px::new(x + dx, y),
                1 => shapes::pixels::Px::new(x, y - dx),
                2 => shapes::pixels::Px::new(x - dx, y),
                _ => shapes::pixels::Px::new(x, y + dx),
            };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreachable_angle_is_not_reported_as_the_requested_input_change() {
        // Two projected normal angles each lie in [-180,180], so a
        // regional change of 1000 degrees cannot be a measured change.
        let input = super::super::test_spine::input();
        assert_eq!(
            apply_delta(&input, Measurement::KyphosisT2T5, None, None, 1000.),
            Err(LayoutError::Refused(UNREACHABLE_INPUT.into()))
        );
        let reading = curves::measure(&input, &input.fit).unwrap();
        let curve = reading
            .curves
            .iter()
            .find(|c| c.region == Some(CurveRegion::MainThoracic))
            .unwrap();
        assert_eq!(
            apply_delta(
                &input,
                Measurement::Cobb,
                None,
                curve.region,
                -curve.cobb_deg - 1.
            ),
            Err(LayoutError::Refused(UNREACHABLE_INPUT.into()))
        );
    }

    #[test]
    fn a_wide_outer_bracket_cannot_hide_a_closer_transition_than_the_current_flip() {
        // The first input already moved at three SD. A second input's
        // first outer bracket is five SD, but its actual step is at one
        // SD. Pruning by the untested five would lose the smaller flip.
        let result = first_changed(&[5.0], Some(3.0), |distance| {
            Ok((distance > 1.0).then_some(Level::L1))
        })
        .unwrap()
        .unwrap();
        assert!(result.0 > 1.0 && result.0 - 1.0 < 1e-12);
        assert_eq!(result.1, Level::L1);
    }

    #[test]
    fn the_first_changed_interval_is_searched_before_a_later_return_to_the_original_level() {
        // Two transitions at one and two units form a narrow interval;
        // a third at four gives another level. The first interval's
        // independent thresholds must win over the later large change.
        let samples = interval_samples(&[1.0, 1.0 + BOUNDARY_STEP, 2.0, 4.0, 4.0 + BOUNDARY_STEP]);
        let result = first_changed(&samples, None, |distance| {
            Ok(if distance > 1.0 && distance < 2.0 {
                Some(Level::L1)
            } else if distance > 4.0 {
                Some(Level::L2)
            } else {
                None
            })
        })
        .unwrap()
        .unwrap();
        assert!(result.0 > 1.0 && result.0 - 1.0 < 1e-12);
        assert_eq!(result.1, Level::L1);
    }
}
