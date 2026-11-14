//! Join the shared versions to correction, statics and daily load cases.
//! Rod contours and body-local heads describe one supplied placement; they
//! are held fixed while the same geometry draws are used for every layout.

mod assembly;
mod middle;
mod values;

#[cfg(test)]
mod stiffness_calibration;

use crate::BenchError;
use crate::daily_loads::{self, DailyInputs};
use crate::evidence::Evidence;
use crate::{rigid_rod, springback, versions};
use shapes::implant_layout::ImplantLayout;
use shapes::load_bench::{BenchBatch, BenchInput, BenchRun};

/// Run one slice. Global indices, seed and total size alone identify draws.
pub fn run(input: &BenchInput) -> Result<BenchBatch, BenchError> {
    if input.fit.vertebrae.is_empty() {
        return Err(BenchError::NoSpine);
    }
    let end = input
        .version_from
        .checked_add(input.version_count)
        .filter(|end| *end <= input.versions && input.versions > 0)
        .ok_or_else(|| BenchError::InvalidValue {
            field: "versionCount".into(),
            reason: "outside-ensemble".into(),
        })?;
    if input.layouts.is_empty() {
        return Err(versions::refusal("no-layouts"));
    }
    for (i, layout) in input.layouts.iter().enumerate() {
        if layout.id.is_empty() || input.layouts[..i].iter().any(|l| l.id == layout.id) {
            return Err(versions::refusal("duplicate-or-empty-layout-id"));
        }
    }
    let evidence = Evidence::read().map_err(|error| versions::refusal(&error.to_string()))?;
    let draws = (input.version_from..end)
        .map(|k| {
            versions::draw(
                &input.fit,
                &input.child,
                input.seed,
                input.versions,
                k,
                &evidence,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut main_curve = None;
    let mut middle = Vec::new();
    let mut runs = Vec::new();
    for layout in &input.layouts {
        let construct = assembly::construct(&input.fit, layout, &evidence)?;
        let readings = values::Readings::new(&input.fit, layout)?;
        main_curve = readings.main_curve;
        if input.version_from == 0 {
            middle.push(middle::run(layout, &input.fit, &evidence, &construct));
        }
        for draw in &draws {
            runs.push(run_one(layout, draw, &evidence, &construct, &readings)?);
        }
    }
    Ok(BenchBatch {
        middle,
        main_curve,
        version_from: input.version_from,
        version_count: input.version_count,
        runs,
    })
}

fn run_one(
    layout: &ImplantLayout,
    draw: &versions::Draw,
    evidence: &Evidence,
    construct: &assembly::Placement,
    readings: &values::Readings,
) -> Result<BenchRun, BenchError> {
    let base_chain = assembly::chain(
        &draw.geometry.fit,
        draw.values.stiffness_scale,
        draw.values.rib_factors,
        evidence,
    )?;
    let chain = daily_loads::loaded_chain(evidence, &base_chain, 1.0, &layout.released_levels)
        .map_err(|error| versions::refusal(&error.to_string()))?;
    let mut draws = draw.values;
    draws.rod_yield_mpa =
        versions::strengths(evidence, layout.rods[0].material, draw.strength_quantile)?.0;
    let before = values::before(&draw.geometry.fit, &chain, layout, readings)?;
    let mut run = BenchRun {
        layout_id: layout.id.clone(),
        version: draw.geometry.index,
        settled: false,
        stiff_rod_share: 0.0,
        draws,
        stops: vec![before],
    };
    // Numerical caps remain visible as unsettled versions. Invalid supplied
    // constructs are refused during assembly, before entering the ensemble.
    let held = match rigid_rod::solve_held(&chain, &construct.construct, &Default::default(), None)
    {
        Ok(state) if state.settled => state,
        _ => return Ok(run),
    };
    let rigid = match rigid_rod::solve_rigid(&chain, &construct.construct, &held, None) {
        Ok(state) if state.settled => state,
        _ => return Ok(run),
    };
    let seated = match springback::solve(&chain, &rigid.poses, &rigid.rods, &held.poses) {
        Ok(state) if state.settled => state,
        _ => return Ok(run),
    };
    run.stiff_rod_share = seated.stiff_rod_share;
    let daily = match daily_loads::solve(
        evidence,
        &base_chain,
        &seated,
        &layout.released_levels,
        DailyInputs {
            weight_kg: draws.weight_kg,
            slice_offset_mm: draws.slice_offset_mm,
            preload_factor: draws.preload_factor,
            walking_multiple: draws.walking_multiple,
            bending_multiple: draw.bending_multiple,
        },
    ) {
        Ok(value) => value,
        Err(_) => return Ok(run),
    };
    run.stops.extend(values::loaded(
        layout, draw, evidence, construct, readings, &chain, &rigid, &seated, &daily,
    )?);
    run.settled = true;
    Ok(run)
}
