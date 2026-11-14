//! Recompute the assumed prior from first placements on six made-up spines.

#[path = "../../tests/realistic_cases/mod.rs"]
mod realistic_cases;

use super::assembly;
use crate::evidence::{Basis, Evidence, TableKind, Unit};
use crate::{daily_loads, rigid_rod, versions};
use shapes::implant_layout::RodMaterial;

// Scale-one R peaks, N, in realistic_cases::CASES order. These are solver
// calibration answers, not measured tissue properties.
const REFERENCE_FORCES_N: [f64; 6] = [
    317.36381454846503,
    194.37452927254435,
    543.4172057237764,
    320.98172625607543,
    160.2604429831296,
    200.1558126092878,
];

fn bounds(e: &Evidence, forces: &[f64; 6]) -> (f64, f64, f64) {
    let force = |term| {
        versions::cell(e, TableKind::JointSprings, "stiffness-scale", term, Unit::N).unwrap()
    };
    let low = force("reference-force-lower");
    let high = force("reference-force-upper");
    let mean = libm::exp(forces.iter().map(|f| libm::log(*f)).sum::<f64>() / 6.0);
    let centre = libm::sqrt(low * high) / mean;
    let half_width = libm::sqrt(high / low);
    (centre, centre / half_width, centre * half_width)
}

// Round to the nearest hundredth (two decimal places), halfway values up.
fn rounded(value: f64) -> f64 {
    libm::round(value * 100.0) / 100.0
}

#[test]
fn the_stored_range_centres_the_geometric_means_and_uses_the_published_factor() {
    let evidence = Evidence::read().unwrap();
    let row = evidence
        .row(TableKind::JointSprings, "stiffness-scale")
        .unwrap();
    for (term, value, unit, source, basis) in [
        (
            "reference-force-lower",
            192.0,
            Unit::N,
            "salmingo-2013",
            Basis::Published,
        ),
        (
            "reference-force-upper",
            439.0,
            Unit::N,
            "salmingo-2013",
            Basis::Published,
        ),
        (
            "reference-rod-diameter",
            6.0,
            Unit::Mm,
            "salmingo-2012",
            Basis::OwnInference,
        ),
    ] {
        let cell = evidence
            .value(TableKind::JointSprings, "stiffness-scale", term)
            .unwrap();
        assert_eq!(
            (cell.value, cell.unit, cell.source.as_str(), cell.basis),
            (value, unit, source, basis)
        );
    }
    let (centre, lower, upper) = bounds(&evidence, &REFERENCE_FORCES_N);
    assert!((centre - 1.096180421360762).abs() < 1e-11);
    assert!((upper / lower - 439.0 / 192.0).abs() < 1e-12);
    assert_eq!(stored(&evidence), (0.72, 1.66));
    assert_eq!(stored(&evidence), (rounded(lower), rounded(upper)));
    assert!(row.notes.contains("Assumed"));
    assert!(row.notes.contains("not a measurement of the child"));
    for qualification in [
        "the largest screw force, 192 to 439 N across six patients",
        "earlier three-patient report (Salmingo 2012, PMID 22321374) and is assumed for the six",
        "as if all of it came from stiffness; six patients are few",
        "summed forces spread by a factor of 3.1, so the true spread could be wider",
        "rods held rigid, before springback, so the rod chosen does not change them",
        "published forces come from rods after they flexed",
        "factor of 3.4 at scale 1, more than the published 2.29",
        "one spine's forces at the stored range can still pass 439 N",
    ] {
        assert!(row.notes.contains(qualification), "{qualification}");
    }
    assert!(!row.notes.contains("widest"));
    assert!(!row.notes.contains("per patient"));
    for force in REFERENCE_FORCES_N {
        assert!(row.notes.contains(&format!("{force:.6}")));
    }
}

fn stored(e: &Evidence) -> (f64, f64) {
    let endpoint = |term| {
        versions::cell(
            e,
            TableKind::JointSprings,
            "stiffness-scale",
            term,
            Unit::Ratio,
        )
        .unwrap()
    };
    (endpoint("lower"), endpoint("upper"))
}

#[test]
#[ignore = "six full first placements; run with --release --ignored --nocapture"]
fn six_first_placements_recompute_the_stored_stiffness_range() {
    let evidence = Evidence::read().unwrap();
    let mut forces = [0.0; 6];
    for (i, (recipe, seed)) in realistic_cases::CASES.into_iter().enumerate() {
        let input = realistic_cases::build(recipe, seed).unwrap().input;
        let proposals = implant_layout::propose_layouts(&input).unwrap();
        let mut layout = proposals.layouts[0].clone();
        let diameter = versions::cell(
            &evidence,
            TableKind::JointSprings,
            "stiffness-scale",
            "reference-rod-diameter",
            Unit::Mm,
        )
        .unwrap();
        let choice = implant_layout::rods::choices()
            .into_iter()
            .find(|r| r.material == RodMaterial::TitaniumAlloy && r.diameter_mm == diameter)
            .expect("sourced 6 mm titanium-alloy rod");
        for rod in &mut layout.rods {
            rod.material = choice.material;
            rod.diameter_mm = choice.diameter_mm;
        }
        let placement = assembly::construct(&input.fit, &layout, &evidence).unwrap();
        let peak = |scale| {
            let draw = versions::middle(&evidence).unwrap();
            let base = assembly::chain(&input.fit, scale, draw.rib_factors, &evidence).unwrap();
            let chain =
                daily_loads::loaded_chain(&evidence, &base, 1.0, &layout.released_levels).unwrap();
            let held =
                rigid_rod::solve_held(&chain, &placement.construct, &Default::default(), None)
                    .unwrap();
            assert!(held.settled, "{recipe:?}/{seed} H: {:?}", held.reason);
            let rigid = rigid_rod::solve_rigid(&chain, &placement.construct, &held, None).unwrap();
            assert!(rigid.settled, "{recipe:?}/{seed} R: {:?}", rigid.reason);
            rigid
                .head_wrenches_on_rod
                .iter()
                .flatten()
                .map(|w| w.fixed_rows::<3>(0).norm())
                .fold(0.0, f64::max)
        };
        forces[i] = peak(1.0);
        assert!(
            (forces[i] / REFERENCE_FORCES_N[i] - 1.0).abs() < 1e-8,
            "{recipe:?}/{seed}: reference force changed to {} N",
            forces[i]
        );
        println!(
            "{recipe:?}/{seed} {:?}-{:?}: {:.12} N",
            layout.upper, layout.lower, forces[i]
        );
        if i == 0 || i == 3 {
            for scale in [stored(&evidence).0, stored(&evidence).1] {
                let actual = peak(scale);
                let expected = forces[i] * scale;
                println!(
                    "linearity {recipe:?}/{seed} scale {scale}: {actual:.12} N; linear {expected:.12} N; relative error {:.3e}",
                    (actual / expected - 1.0).abs()
                );
                assert!((actual / expected - 1.0).abs() < 1e-8);
            }
        }
    }
    let (centre, lower, upper) = bounds(&evidence, &forces);
    println!(
        "forces {forces:?}; centre {centre:.12}; bounds {lower:.12}, {upper:.12}; rounded {}, {}",
        rounded(lower),
        rounded(upper)
    );
    assert_eq!(stored(&evidence), (rounded(lower), rounded(upper)));
}
