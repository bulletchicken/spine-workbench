//! Source keys from the evidence rows used by each reported quantity.

use crate::evidence::{Evidence, TableKind};
use crate::versions::material_key;
use shapes::implant_layout::{AnchorKind, ImplantLayout, RodMaterial};
use shapes::levels::Level;
use shapes::load_bench::{BenchStop, Measure, PartRef};
use std::collections::BTreeSet;

pub(super) fn estimate(
    evidence: &Evidence,
    layout: &ImplantLayout,
    stop: BenchStop,
    measure: Measure,
) -> Vec<String> {
    if stop == BenchStop::Before {
        return Vec::new();
    }
    let mut keys = Keys::new(evidence);
    mechanical(&mut keys, layout);
    if !matches!(
        measure,
        Measure::MainCobb | Measure::Kyphosis | Measure::ApicalTwist
    ) {
        daily(&mut keys, stop);
    }
    match measure {
        Measure::HeaviestAnchorDemand => {
            for anchor in &layout.anchors {
                capacity(&mut keys, anchor.level, anchor.kind);
            }
        }
        Measure::HeaviestRodUse => {
            for rod in &layout.rods {
                strengths(&mut keys, rod.material, false);
            }
        }
        Measure::HeaviestRodSwing => {
            for rod in &layout.rods {
                fatigue(&mut keys, rod.material);
            }
            keys.row(TableKind::DailyLoads, "walking");
        }
        _ => {}
    }
    keys.finish()
}

pub(super) fn cycles(evidence: &Evidence, stop: BenchStop) -> Vec<String> {
    let mut keys = Keys::new(evidence);
    if stop == BenchStop::YearOfWalking {
        keys.terms(TableKind::DailyLoads, "year", &["days"]);
    }
    keys.finish()
}

pub(super) fn part(
    evidence: &Evidence,
    layout: &ImplantLayout,
    stop: BenchStop,
    part: PartRef,
    share_sources: &[String],
) -> Vec<String> {
    if stop == BenchStop::Before {
        return Vec::new();
    }
    let mut keys = Keys::new(evidence);
    match part {
        PartRef::Anchor { level, side } => {
            if let Some(anchor) = layout
                .anchors
                .iter()
                .find(|a| a.level == level && a.side == side)
            {
                capacity(&mut keys, level, anchor.kind);
            }
        }
        PartRef::RodSpan { side, .. } => {
            if let Some(rod) = layout.rods.iter().find(|r| r.side == side) {
                strengths(&mut keys, rod.material, false);
            }
        }
        PartRef::Rod { side } if stop == BenchStop::YearOfWalking => {
            if let Some(rod) = layout.rods.iter().find(|r| r.side == side) {
                fatigue(&mut keys, rod.material);
                keys.row(TableKind::DailyLoads, "walking");
            }
        }
        PartRef::Rod { .. } | PartRef::Junction { .. } => {}
    }
    let mut sources = keys.finish();
    sources.extend_from_slice(share_sources);
    sources.sort();
    sources.dedup();
    sources
}

fn mechanical(keys: &mut Keys<'_>, layout: &ImplantLayout) {
    for row in ["t1-2-to-l4-5-intact", "l5-s1"] {
        keys.row(TableKind::JointSprings, row);
    }
    // Only the scale endpoints are drawn; the historical fitted scales
    // recorded beside them do not enter the calculation.
    keys.terms(
        TableKind::JointSprings,
        "stiffness-scale",
        &["lower", "upper"],
    );
    for row in ["rib-mannen-t1-t4", "rib-mannen-t8-t12"] {
        keys.terms(
            TableKind::JointSprings,
            row,
            &["fe-with-ribs", "fe-without-ribs"],
        );
    }
    for row in ["rib-lateral-bending", "rib-axial-rotation"] {
        keys.terms(TableKind::JointSprings, row, &["lower", "upper"]);
    }
    if !layout.released_levels.is_empty() {
        keys.row(TableKind::JointSprings, "release");
    }
    for rod in &layout.rods {
        keys.terms(
            TableKind::RodMaterials,
            material_key(rod.material),
            &["young-modulus"],
        );
    }
}

fn daily(keys: &mut Keys<'_>, stop: BenchStop) {
    if stop == BenchStop::RodsIn {
        return;
    }
    keys.terms(TableKind::JointSprings, "preload", &["lower", "upper"]);
    for row in ["head-neck", "each-arm-hand", "trunk-slice-endpoints"] {
        keys.row(TableKind::DailyLoads, row);
    }
    match stop {
        BenchStop::BendingForward => keys.row(TableKind::DailyLoads, "ventral-flexion"),
        BenchStop::YearOfWalking => keys.row(TableKind::DailyLoads, "walking"),
        _ => {}
    }
}

fn capacity(keys: &mut Keys<'_>, level: Level, kind: AnchorKind) {
    match kind {
        AnchorKind::None => return,
        AnchorKind::Hook => keys.terms(TableKind::AnchorStrength, "t4-t8-pedicle-hook", &["mean"]),
        AnchorKind::Screw if level <= Level::T8 => {
            keys.terms(
                TableKind::AnchorStrength,
                "thoracic-sampling",
                &["lower", "upper"],
            );
        }
        AnchorKind::Screw if level <= Level::T12 => {
            keys.terms(TableKind::AnchorStrength, "t9-t12-screw", &["mean"]);
        }
        AnchorKind::Screw => keys.terms(TableKind::AnchorStrength, "lumbar-screw", &["normal"]),
    }
    keys.row(TableKind::AnchorStrength, "bone-prior");
}

fn strengths(keys: &mut Keys<'_>, material: RodMaterial, ultimate: bool) {
    if let Some(pair) = keys.evidence.bench_strengths(material) {
        let cells = if ultimate {
            [pair.ultimate_lower, pair.ultimate_upper]
        } else {
            [pair.yield_lower, pair.yield_upper]
        };
        for cell in cells {
            keys.values.insert(&cell.source);
        }
    }
}

fn fatigue(keys: &mut Keys<'_>, material: RodMaterial) {
    strengths(keys, material, true);
    keys.row(TableKind::Fatigue, "contour-knock-down");
    if material != RodMaterial::StainlessSteel {
        // The row carries both the published run-out test and its stress
        // conversion, whose fixture arm and equal sharing are assumptions.
        keys.row(TableKind::Fatigue, material_key(material));
    }
}

struct Keys<'a> {
    evidence: &'a Evidence,
    values: BTreeSet<&'a str>,
}

impl<'a> Keys<'a> {
    fn new(evidence: &'a Evidence) -> Self {
        Self {
            evidence,
            values: BTreeSet::new(),
        }
    }

    fn row(&mut self, table: TableKind, row: &str) {
        if let Some(row) = self.evidence.row(table, row) {
            for cell in &row.values {
                self.values.insert(&cell.source);
            }
        }
    }

    fn terms(&mut self, table: TableKind, row: &str, terms: &[&str]) {
        for term in terms {
            if let Some(cell) = self.evidence.value(table, row, term) {
                self.values.insert(&cell.source);
            }
        }
    }

    fn finish(self) -> Vec<String> {
        self.values.into_iter().map(str::to_owned).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::implant_layout::LayoutChange;

    #[test]
    fn each_capacity_keeps_its_own_region_and_kind() {
        let evidence = Evidence::read().unwrap();
        for (level, kind, publication) in [
            (Level::T8, AnchorKind::Screw, "lehman-2003"),
            (Level::T9, AnchorKind::Screw, "liljenqvist-2001"),
            (Level::L1, AnchorKind::Screw, "soshi-1991"),
            (Level::L1, AnchorKind::Hook, "liljenqvist-2001"),
        ] {
            let mut keys = Keys::new(&evidence);
            capacity(&mut keys, level, kind);
            let mut expected = vec![publication.to_owned(), "project-assumptions".to_owned()];
            expected.sort();
            assert_eq!(keys.finish(), expected);
        }
        let mut keys = Keys::new(&evidence);
        capacity(&mut keys, Level::T8, AnchorKind::None);
        assert!(keys.finish().is_empty());
    }

    #[test]
    fn fatigue_sources_keep_the_construct_test_and_contouring_evidence() {
        let evidence = Evidence::read().unwrap();
        for material in [RodMaterial::TitaniumAlloy, RodMaterial::CobaltChrome] {
            let mut keys = Keys::new(&evidence);
            fatigue(&mut keys, material);
            let sources = keys.finish();
            assert_eq!(
                sources,
                [
                    "prasad-2017",
                    "project-assumptions",
                    "slivka-2013",
                    "vieweg-2023"
                ]
            );
            for source in sources {
                assert!(evidence.source(&source).is_some());
            }
        }
        let mut keys = Keys::new(&evidence);
        fatigue(&mut keys, RodMaterial::StainlessSteel);
        assert!(!keys.finish().iter().any(|source| source == "vieweg-2023"));
    }

    #[test]
    fn film_only_angles_have_no_material_or_load_citations() {
        let evidence = Evidence::read().unwrap();
        let layout = ImplantLayout {
            id: "first".into(),
            change: LayoutChange::First {},
            upper: Level::T4,
            lower: Level::L1,
            anchors: Vec::new(),
            rods: Vec::new(),
            released_levels: Vec::new(),
        };
        for measure in [Measure::MainCobb, Measure::Kyphosis, Measure::ApicalTwist] {
            assert!(estimate(&evidence, &layout, BenchStop::Before, measure).is_empty());
        }
    }

    #[test]
    fn every_source_selected_from_the_rows_resolves() {
        let evidence = Evidence::read().unwrap();
        let layout = ImplantLayout {
            id: "released".into(),
            change: LayoutChange::First {},
            upper: Level::T4,
            lower: Level::L1,
            anchors: Vec::new(),
            rods: Vec::new(),
            released_levels: vec![Level::T8],
        };
        let mut keys = Keys::new(&evidence);
        mechanical(&mut keys, &layout);
        for stop in [
            BenchStop::RodsIn,
            BenchStop::Standing,
            BenchStop::BendingForward,
            BenchStop::YearOfWalking,
        ] {
            daily(&mut keys, stop);
        }
        for level in [
            Level::T1,
            Level::T8,
            Level::T9,
            Level::T12,
            Level::L1,
            Level::L5,
        ] {
            for kind in [AnchorKind::Screw, AnchorKind::Hook, AnchorKind::None] {
                capacity(&mut keys, level, kind);
            }
        }
        for material in [
            RodMaterial::TitaniumAlloy,
            RodMaterial::CobaltChrome,
            RodMaterial::StainlessSteel,
        ] {
            strengths(&mut keys, material, false);
            fatigue(&mut keys, material);
        }
        for source in keys.finish() {
            assert!(evidence.source(&source).is_some(), "{source}");
        }
        assert!(cycles(&evidence, BenchStop::Standing).is_empty());
        assert_eq!(
            cycles(&evidence, BenchStop::YearOfWalking),
            ["project-assumptions"]
        );
    }
}
