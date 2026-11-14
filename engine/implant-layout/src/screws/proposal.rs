//! Assemble table-derived screw candidates on supplied levels.

use super::*;

/// Build one pedicle over the complete, already sampled ensemble. Its
/// order need not start at zero, but every global version must occur once.
/// The nominal entry uses the table's central chord and medial angle;
/// convergence includes geometry and per-pedicle medial-angle draws.
/// Width, side-factor, chord and angle coordinates use the same canonical
/// curve-relative identity, keeping corresponding draws under reflection.
/// Nothing here chooses instrumented levels or reads a level rule.
pub fn anchor(
    input: &LayoutInput,
    versions: &[Version],
    readings: &[SpineReading],
    level: Level,
    side: BodySide,
) -> Result<AnchorProposal, LayoutError> {
    validate_ensemble(input, versions, readings)?;
    let nominal_reading = crate::curves::measure(input, &input.fit)?;
    let position = pedicles::position_in_curve(&nominal_reading, level, side);
    let mut version_apical = false;
    let mut version_aorta = false;
    let nominal = input
        .fit
        .vertebrae
        .iter()
        .find(|v| v.level == level)
        .ok_or_else(|| invalid("level-not-fitted"))?;
    let sampler = NamedSampler::new(u64::from(input.seed), input.versions)?;
    let dgrid = sizing::diameter_grid();
    let lgrid = sizing::length_grid();
    let (k, k_source) =
        sizing::default_width_ratio().ok_or_else(|| refused("missing-width-ratio"))?;
    let (share, share_range, mut sources) = chord_shares(input.child.age_years)?;
    sources.push(k_source);
    for (table, key, group) in [
        (Table::ChannelGrades, "inner-diameter-max", "grade-4"),
        (
            Table::DiameterRules,
            "deviation-odds",
            "screw-pedicle-ratio",
        ),
        (
            Table::PlacementChecks,
            "straight-forward-pull-out-gain",
            "straight-forward",
        ),
    ] {
        if let Some(c) = cell(table, key, group) {
            sources.push(c.source.clone());
        }
    }
    let (margin, margin_source) =
        required_scalar(Table::LengthRules, "anterior-breach-threshold", "aorta")?;
    sources.push(margin_source);
    let chord = anatomy::chord(level, position.curve_side, input.child.age_years);
    let angle = anatomy::medial_angle(level, position.curve_side);
    let nominal_trajectory = chord
        .as_ref()
        .zip(angle.as_ref())
        .map(|(c, a)| geometry::trajectory(&nominal.pose, side, a.mean, c.mean))
        .transpose()?;
    let mut widths = Vec::new();
    let mut chords = Vec::new();
    let mut convergence = Vec::new();
    let mut nearest = false;
    let mut missing = chord.is_none() || angle.is_none();
    let mut max_twist = nominal.angles.twist_rad.to_degrees().abs();
    let mut chord_complete = true;
    for (v, r) in versions.iter().zip(readings) {
        let position = pedicles::position_in_curve(r, level, side);
        let latent = pedicles::latent_side(side, position.curve_side);
        version_apical |= position.curve_side == CurveSide::Concave
            && position.apex_distance <= 1.0
            && (Level::T3..=Level::T9).contains(&level);
        version_aorta |= r
            .curves
            .iter()
            .any(|c| c.convex == BodySide::Right && r.vertebrae[c.apex_index].level <= Level::T12);
        let width = pedicles::draw_width(
            level,
            side,
            input.child.age_years,
            position,
            v,
            &sampler,
            input.pedicle_child_share,
        )?;
        if let Some(w) = &width {
            sources.extend(w.sources.iter().cloned());
        }
        missing |= width.is_none();
        widths.push(width);
        let Some(i) = v.fit.vertebrae.iter().position(|v| v.level == level) else {
            return Err(invalid("level-not-in-version"));
        };
        let vertebra = &v.fit.vertebrae[i];
        max_twist = max_twist.max(vertebra.angles.twist_rad.to_degrees().abs());
        if let Some(c) = anatomy::chord(level, position.curve_side, input.child.age_years) {
            sources.extend(c.sources.iter().cloned());
            nearest |= c.nearest_age;
            let z = sampler.normal(&format!("versions-pedicles/{i}/{latent:?}/chord"), v.index)?;
            let drawn = c.mean
                + c.sd.unwrap_or(0.0)
                    * pedicles::shared_normal(
                        v.z_child,
                        z,
                        input
                            .pedicle_child_share
                            .unwrap_or(pedicles::DEFAULT_CHILD_SHARE),
                    );
            chords.push(drawn);
        } else {
            chord_complete = false;
            missing = true;
        }
        if let Some(a) = anatomy::medial_angle(level, position.curve_side) {
            sources.extend(a.sources.iter().cloned());
            let z = sampler.normal(&format!("versions-pedicles/{i}/{latent:?}/angle"), v.index)?;
            let direction =
                geometry::direction(&vertebra.pose, side, a.mean + a.sd.unwrap_or(0.0) * z)?;
            convergence.push(direction.convergence_deg);
        } else {
            missing = true;
        }
    }
    let pedicle = pedicles::estimate(&widths, position.curve_side);
    let outer: Vec<_> = widths.iter().flatten().map(|w| w.outer_mm).collect();
    let diameter = if outer.len() == versions.len() {
        sizing::choose_diameter(&outer, &dgrid, k, sizing::DEFAULT_ALPHA)?
    } else {
        sizing::SizeChoice {
            mm: None,
            also_seen: Vec::new(),
        }
    };
    let length = if chord_complete {
        sizing::choose_length(&chords, &lgrid, share, margin, sizing::DEFAULT_ALPHA)?
    } else {
        sizing::SizeChoice {
            mm: None,
            also_seen: Vec::new(),
        }
    };
    let mut length_also_seen = length.also_seen;
    for f in share_range {
        if chord_complete
            && let Some(mm) =
                sizing::choose_length(&chords, &lgrid, f, margin, sizing::DEFAULT_ALPHA)?.mm
            && Some(mm) != length.mm
        {
            length_also_seen.push(mm);
        }
    }
    length_also_seen.sort_by(f64::total_cmp);
    length_also_seen.dedup();
    for selected in [diameter.mm]
        .into_iter()
        .chain(diameter.also_seen.iter().copied().map(Some))
        .flatten()
    {
        if let Some(g) = dgrid.iter().find(|g| g.mm == selected) {
            sources.extend(g.sources.iter().cloned());
        }
    }
    for selected in [length.mm]
        .into_iter()
        .flatten()
        .chain(length_also_seen.iter().copied())
    {
        if let Some(g) = lgrid.iter().find(|g| g.mm == selected) {
            sources.extend(g.sources.iter().cloned());
        }
    }
    let diameters_beside = diameters_beside(&outer, &dgrid, outer.len() == versions.len())?;
    sources.extend(diameters_beside.iter().map(|rule| rule.source.clone()));
    for rule in &diameters_beside {
        if let Some(mm) = rule.diameter_mm
            && let Some(g) = dgrid.iter().find(|g| g.mm == mm)
        {
            sources.extend(g.sources.iter().cloned());
        }
    }
    let medial_angles_beside = anatomy::medial_angles_beside(level, input.child.age_years);
    sources.extend(medial_angles_beside.iter().map(|a| a.source.clone()));
    let (usual_length_mm, usual_sources) = usual_length(level);
    sources.extend(usual_sources);
    let convergence_deg = band(&convergence).unwrap_or(Band {
        p10: 0.0,
        p50: 0.0,
        p90: 0.0,
    });
    let (mut looks, look_sources) = look_codes(LookInputs {
        level,
        side,
        position,
        right_thoracic_curve: version_aorta
            || nominal_reading.curves.iter().any(|c| {
                c.convex == BodySide::Right
                    && nominal_reading.vertebrae[c.apex_index].level <= Level::T12
            }),
        missing_table: missing,
        no_diameter: diameter.mm.is_none(),
        outer_narrow_share: f64::from(pedicle.versions_too_narrow) / versions.len() as f64,
        inner_narrow_share: f64::from(pedicle.versions_inner_under_1_mm) / versions.len() as f64,
        convergence: convergence_deg,
        max_abs_twist_deg: max_twist,
        shorter_than_grid: chord_complete && length.mm.is_none(),
        nearest_chord_age: nearest,
    })?;
    sources.extend(look_sources);
    if version_apical {
        looks.push(LookCode::ConcaveApexNeedsCt);
    }
    looks.sort();
    looks.dedup();
    let centre =
        pedicle_centres(&VertebraParams::from(&nominal.pose))[usize::from(side == BodySide::Right)];
    let (entry_mm, direction_unit) = nominal_trajectory.as_ref().map_or(
        (centre.into(), geometry::straight_forward(&nominal.pose)?),
        |t| (t.entry_mm, t.direction_unit),
    );
    let has_geometry = nominal_trajectory.is_some();
    let screw_to_pedicle = diameter
        .mm
        .and_then(|d| band(&outer.iter().map(|w| d / w).collect::<Vec<_>>()));
    let kind = if diameter.mm.is_some() && length.mm.is_some() && has_geometry {
        AnchorKind::Screw
    } else {
        AnchorKind::None
    };
    Ok(AnchorProposal {
        level,
        side,
        kind,
        entry_mm,
        entry_status: crate::rods::geometry_status(&input.fit),
        direction_unit,
        direction_status: if has_geometry {
            ValueStatus::FromTables
        } else {
            ValueStatus::Missing
        },
        medial_angle: angle.map(|a| AngleFromTable {
            angle_deg: a.mean,
            source: a.sources[0].clone(),
        }),
        medial_angles_beside,
        convergence_deg,
        diameter_mm: diameter.mm,
        diameter_also_seen: diameter.also_seen,
        diameter_status: if pedicle.outer_width_mm.is_some() {
            ValueStatus::FromTables
        } else {
            ValueStatus::Missing
        },
        width_ratio: Some(k),
        diameters_beside,
        screw_to_pedicle,
        chord_mm: chord.map(|c| c.mean),
        chord_share: Some(share),
        length_mm: length.mm,
        length_also_seen,
        length_status: if chord_complete {
            ValueStatus::FromTables
        } else {
            ValueStatus::Missing
        },
        usual_length_mm,
        pedicle,
        looks,
        sources: unique(sources),
    })
}

/// Bilateral candidate anchors in the supplied instrumented order. Apply
/// a pattern afterwards; no level selection is hidden in this function.
pub fn anchors(
    input: &LayoutInput,
    versions: &[Version],
    readings: &[SpineReading],
    levels: &[Level],
) -> Result<Vec<AnchorProposal>, LayoutError> {
    if levels.is_empty() || levels.windows(2).any(|w| w[0] >= w[1]) {
        return Err(invalid("instrumented-level-order"));
    }
    let mut result = Vec::new();
    for &level in levels {
        for side in [BodySide::Left, BodySide::Right] {
            result.push(anchor(input, versions, readings, level, side)?);
        }
    }
    Ok(result)
}

/// Published regional practice range, for display alone. It never enters
/// diameter or length selection. No regional summary creates a chord.
pub fn usual_length(level: Level) -> (Option<[f64; 2]>, Vec<String>) {
    let suffix = if (Level::T1..=Level::T5).contains(&level) {
        "t1-t5"
    } else if (Level::T6..=Level::T12).contains(&level) {
        "t6-t12"
    } else if (Level::L1..=Level::L5).contains(&level) {
        "l1-l5"
    } else {
        return (None, Vec::new());
    };
    let mut sources = Vec::new();
    let mut range = None;
    for prefix in ["balikci-2019", "kuraishi-2013-full-paper"] {
        if let Some(c) = cell(
            Table::LengthRules,
            "usual-length",
            &format!("{prefix}-{suffix}"),
        ) {
            sources.push(c.source.clone());
            let bounds = match &c.value {
                PublishedValue::Range { lower, upper } => Some([*lower, *upper]),
                PublishedValue::Grid { values } => {
                    Some([*values.first().unwrap(), *values.last().unwrap()])
                }
                _ => None,
            };
            if range.is_none() {
                range = bounds;
            }
        }
    }
    (range, unique(sources))
}

/// This project's combination rule: the younger published share range
/// supplies its midpoint and both alternatives; older chords take the
/// cited lower bound. Percentages become dimensionless fractions here.
fn chord_shares(age: Option<f64>) -> Result<(f64, Vec<f64>, Vec<String>), LayoutError> {
    let young = age.is_some_and(|a| a < 12.0);
    let c = cell(
        Table::LengthRules,
        if young {
            "ap-penetration"
        } else {
            "chord-penetration"
        },
        if young { "children" } else { "krag-cited" },
    )
    .ok_or_else(|| refused("missing-chord-share"))?;
    let PublishedValue::Range { lower, upper } = c.value else {
        return Err(refused("chord-share-not-range"));
    };
    // Project inference: children take the midpoint, older the lower end.
    Ok((
        if young {
            (lower + upper) * 0.5 / 100.0
        } else {
            lower / 100.0
        },
        if young {
            vec![lower / 100.0, upper / 100.0]
        } else {
            Vec::new()
        },
        vec![c.source.clone()],
    ))
}
fn diameters_beside(
    widths: &[f64],
    grid: &[sizing::GridSize],
    complete: bool,
) -> Result<Vec<DiameterRule>, LayoutError> {
    let mut output = Vec::new();
    for (key, group) in [
        ("outer-width-ratio-max", "adult-cadaver"),
        ("outer-width-ratio", "upendra-cited"),
    ] {
        let Some(c) = cell(Table::DiameterRules, key, group) else {
            continue;
        };
        let k = match c.value {
            PublishedValue::Range { upper, .. } => upper,
            _ => central(c).ok_or_else(|| refused("width-rule-not-number"))?,
        };
        output.push(DiameterRule {
            width_ratio: k,
            diameter_mm: if complete {
                sizing::choose_diameter(widths, grid, k, sizing::DEFAULT_ALPHA)?.mm
            } else {
                None
            },
            source: c.source.clone(),
        });
    }
    Ok(output)
}
fn validate_ensemble(
    input: &LayoutInput,
    versions: &[Version],
    readings: &[SpineReading],
) -> Result<(), LayoutError> {
    if versions.len() != input.versions as usize
        || versions.is_empty()
        || readings.len() != versions.len()
        || versions.iter().enumerate().any(|(i, v)| {
            v.index >= input.versions || versions[..i].iter().any(|p| p.index == v.index)
        })
        || input
            .child
            .age_years
            .is_some_and(|a| !a.is_finite() || a < 0.0)
        || input
            .pedicle_child_share
            .is_some_and(|s| !s.is_finite() || !(0.0..=1.0).contains(&s))
    {
        return Err(invalid("complete-ensemble-required"));
    }
    Ok(())
}
