//! Only sourced values enter the public change catalogue.

use crate::{rods, screws::sizing};
use shapes::implant_layout::{LayoutChoices, RodChoice};

pub(super) fn choices() -> LayoutChoices {
    let diameters = sizing::diameter_grid();
    let lengths = sizing::length_grid();
    let rods = rods::choices();
    let mut sources: Vec<_> = diameters
        .iter()
        .chain(&lengths)
        .flat_map(|g| g.sources.iter().cloned())
        .chain(rods.iter().flat_map(|r| r.sources.iter().cloned()))
        .collect();
    sources.sort();
    sources.dedup();
    let mut rod_diameters_mm: Vec<_> = rods.iter().map(|r| r.diameter_mm).collect();
    rod_diameters_mm.sort_by(f64::total_cmp);
    rod_diameters_mm.dedup();
    let mut rod_materials = Vec::new();
    for rod in &rods {
        if !rod_materials.contains(&rod.material) {
            rod_materials.push(rod.material);
        }
    }
    LayoutChoices {
        screw_diameters_mm: diameters.into_iter().map(|g| g.mm).collect(),
        screw_lengths_mm: lengths.into_iter().map(|g| g.mm).collect(),
        rod_diameters_mm,
        rod_materials,
        rods: rods
            .into_iter()
            .map(|rod| RodChoice {
                material: rod.material,
                diameter_mm: rod.diameter_mm,
            })
            .collect(),
        sources,
    }
}
