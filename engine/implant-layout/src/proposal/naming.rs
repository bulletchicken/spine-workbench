//! Renaming scenarios retain geometry and draw identities. They are not
//! sampled probabilities, and never rerun the choice of fusion ends.

use crate::LayoutError;
use crate::curves;
use crate::level_rules::LevelReadings;
use crate::patterns::Pattern;
use crate::screws;
use shapes::implant_layout::{
    AnchorChange, ImplantLayout, LayoutInput, NamingDoubt, NamingScenario,
};
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;

fn renamed(names: &[Level], level: Level, shift: i32) -> Result<Level, LayoutError> {
    names
        .iter()
        .position(|&l| l == level)
        .and_then(|i| i.checked_add_signed(shift as isize))
        .and_then(|i| names.get(i))
        .copied()
        .ok_or_else(|| LayoutError::Refused("name-outside-anatomical-count".into()))
}

fn rename_fit(fit: &mut SpineFit, names: &[Level], shift: i32) -> Result<(), LayoutError> {
    for v in &mut fit.vertebrae {
        v.level = renamed(names, v.level, shift)?;
        v.pose.level = v.level;
    }
    fit.origin_level = renamed(names, fit.origin_level, shift)?;
    Ok(())
}

pub(super) fn scenarios(
    input: &LayoutInput,
    readings: &LevelReadings,
    first: &ImplantLayout,
) -> Result<Vec<NamingScenario>, LayoutError> {
    if input.naming == NamingDoubt::AsRead {
        return Ok(Vec::new());
    }
    let names = input
        .front
        .as_ref()
        .map_or_else(
            || crate::level_rules::support::fit_template(&input.fit),
            |f| f.marks.naming.template,
        )
        .names();
    let original_levels: Vec<_> = first
        .anchors
        .as_chunks::<2>()
        .0
        .iter()
        .map(|a| a[0].level)
        .collect();
    let mut scenarios = Vec::new();
    for shift in [-1, 1] {
        let mut moved = input.clone();
        rename_fit(&mut moved.fit, &names, shift)?;
        for film in [
            &mut moved.front,
            &mut moved.bend_left,
            &mut moved.bend_right,
        ]
        .into_iter()
        .flatten()
        {
            for v in &mut film.marks.vertebrae {
                v.level = renamed(&names, v.level, shift)?;
            }
        }
        let mut versions = readings.versions.versions.clone();
        for v in &mut versions {
            rename_fit(&mut v.fit, &names, shift)?;
            for p in &mut v.pedicles {
                p.level = renamed(&names, p.level, shift)?;
            }
        }
        let measured = versions
            .iter()
            .map(|v| curves::measure(&moved, &v.fit))
            .collect::<Result<Vec<_>, _>>()?;
        let levels = original_levels
            .iter()
            .map(|&level| renamed(&names, level, shift))
            .collect::<Result<Vec<_>, _>>()?;
        let nominal = curves::measure(&moved, &moved.fit)?;
        let apex = nominal
            .major
            .map(|i| nominal.vertebrae[nominal.curves[i].apex_index].level);
        let mut anchors = screws::anchors(&moved, &versions, &measured, &levels)?;
        super::anchors::apply(
            &mut anchors,
            &levels,
            apex,
            nominal
                .major
                .and_then(|i| nominal.curves.get(i))
                .map(|c| c.convex),
            Pattern::AllPedicles,
        )?;
        let mut changes = Vec::new();
        for (from, to) in first.anchors.iter().zip(anchors) {
            if from.kind == to.kind
                && from.diameter_mm == to.diameter_mm
                && from.length_mm == to.length_mm
                && from.looks == to.looks
            {
                continue;
            }
            changes.push(AnchorChange {
                level: from.level,
                side: from.side,
                renamed_as: to.level,
                kind_from: from.kind,
                kind_to: to.kind,
                diameter_from_mm: from.diameter_mm,
                diameter_to_mm: to.diameter_mm,
                length_from_mm: from.length_mm,
                length_to_mm: to.length_mm,
                looks_added: to
                    .looks
                    .iter()
                    .filter(|c| !from.looks.contains(c))
                    .copied()
                    .collect(),
                looks_removed: from
                    .looks
                    .iter()
                    .filter(|c| !to.looks.contains(c))
                    .copied()
                    .collect(),
            });
        }
        scenarios.push(NamingScenario { shift, changes });
    }
    Ok(scenarios)
}
