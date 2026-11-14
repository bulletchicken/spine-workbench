//! Scalar cycle answers independent of the mechanics that supplies stress.

use load_bench::evidence::{Evidence, TableKind};
use load_bench::fatigue::{FatigueComparison, WalkingInput, walking_cycle, year_of_walking};
use serde_json::Value;
use shapes::implant_layout::RodMaterial;

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1e-12 + 1e-10 * expected.abs(),
        "{actual} against {expected}"
    );
}

fn input(evidence: &Evidence, material: RodMaterial) -> WalkingInput {
    let strengths = evidence
        .bench_strengths(material)
        .expect("selected strength pair");
    WalkingInput {
        steps_per_day: 1_234.0,
        walking_multiple: 3.0,
        contour_knock_down: evidence
            .value(TableKind::Fatigue, "contour-knock-down", "lower")
            .expect("contour factor")
            .value,
        ultimate_mpa: (strengths.ultimate_lower.value + strengths.ultimate_upper.value) / 2.0,
    }
}

#[test]
fn the_year_counts_steps_rather_than_pairs_of_steps() {
    let evidence = Evidence::read().expect("evidence");
    let material = RodMaterial::TitaniumAlloy;
    let first_input = input(&evidence, material);
    let first = year_of_walking(&evidence, material, first_input, 30.0, 20.0).expect("year");
    // Calendar arithmetic, independently: 1234 * (300 + 60 + 5) = 450410.
    close(first.cycles_per_year, 450_410.0);
    close(first.cycle.amplitude_mpa, 20.0);
    close(first.cycle.mean_mpa, 70.0);
    let doubled = year_of_walking(
        &evidence,
        material,
        WalkingInput {
            steps_per_day: first_input.steps_per_day * 2.0,
            ..first_input
        },
        30.0,
        20.0,
    )
    .expect("twice the steps");
    close(doubled.cycles_per_year, 900_820.0);
    assert_eq!(doubled.cycle, first.cycle);
    assert_eq!(doubled.comparison, first.comparison);
    assert!(
        first
            .sources
            .iter()
            .all(|key| evidence.source(key).is_some())
    );
}

#[test]
fn the_walking_cycle_matches_every_goodman_fixture() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../proving-ground/known-answers/load-bench/goodman.json"
    ))
    .expect("fixture");
    for case in fixture["cases"].as_array().expect("cases") {
        let given = &case["input"];
        let amplitude = given["amplitudeMpa"].as_f64().expect("amplitude");
        let mean = given["meanMpa"].as_f64().expect("mean");
        let ultimate = given["ultimateMpa"].as_f64().expect("ultimate");
        // With a made-up walking multiple of 3, standing=a and locked=m-2a
        // give endpoints m-a and m+a, independently of the conversion.
        let cycle = walking_cycle(mean - 2.0 * amplitude, amplitude, 3.0, ultimate)
            .expect("finite cycle inputs");
        close(cycle.amplitude_mpa, amplitude);
        close(cycle.mean_mpa, mean);
        let expected = case["expect"]["equivalentAmplitudeMpa"].as_f64();
        match expected {
            Some(value) => close(cycle.equivalent_mpa.expect("finite conversion"), value),
            None => assert_eq!(cycle.equivalent_mpa, None),
        }
    }
    // Reversing every signed stress preserves the range but reverses the mean.
    let compressed = walking_cycle(-30.0, -20.0, 3.0, 1000.0).expect("signed cycle");
    close(compressed.amplitude_mpa, 20.0);
    close(compressed.mean_mpa, -70.0);
    close(
        compressed.equivalent_mpa.expect("compressive mean"),
        2000.0 / 107.0,
    );
}

#[test]
fn the_construct_reference_and_contouring_set_the_ratio() {
    let evidence = Evidence::read().expect("evidence");
    let material = RodMaterial::CobaltChrome;
    let given = input(&evidence, material);
    let level = evidence
        .value(TableKind::Fatigue, "cobalt-chrome", "goodman-equivalent")
        .expect("construct level")
        .value;
    let threshold = level * given.contour_knock_down;
    // locked=0 and walk=3 imply a=s, m=2s. Solve a/(1-m/u)=threshold
    // independently: s=threshold*u/(u+2*threshold).
    let standing = threshold * given.ultimate_mpa / (given.ultimate_mpa + 2.0 * threshold);
    let year = year_of_walking(&evidence, material, given, 0.0, standing).expect("year");
    let reference = year.reference.expect("reference");
    close(reference.equivalent_mpa, level);
    close(reference.after_contouring_mpa, threshold);
    let FatigueComparison::Ratio(ratio) = year.comparison else {
        panic!("finite reference comparison");
    };
    close(ratio, 1.0);
    let at_ultimate = year_of_walking(&evidence, material, given, given.ultimate_mpa, 1.0)
        .expect("cycle outside finite conversion");
    assert_eq!(at_ultimate.cycle.equivalent_mpa, None);
    assert_eq!(at_ultimate.comparison, FatigueComparison::NoFiniteGoodman);
}

#[test]
fn stainless_has_no_construct_fatigue_level() {
    let evidence = Evidence::read().expect("evidence");
    let material = RodMaterial::StainlessSteel;
    let year = year_of_walking(&evidence, material, input(&evidence, material), 30.0, 20.0)
        .expect("year without a reference");
    assert_eq!(year.reference, None);
    assert_eq!(year.comparison, FatigueComparison::NoFatigueLevel);
    close(year.cycles_per_year, 450_410.0);
}
