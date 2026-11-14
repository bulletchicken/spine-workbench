use load_bench::evidence::{Basis, Evidence, TableKind, Unit, Usage};

#[test]
fn every_daily_load_ledger_entry_states_its_evidence_usage() {
    let evidence = Evidence::read().unwrap();
    let ledger = include_str!("../../../app/src/words/evidence-ledger.ts");
    for entry in ledger
        .split("    id: '")
        .filter_map(|entry| entry.strip_prefix("place-bench-daily-loads-"))
    {
        let key = entry.split('\'').next().unwrap();
        let row = evidence.row(TableKind::DailyLoads, key).unwrap();
        // Join the ledger's wrapped single-quoted string literals.
        let words = entry.split('\'').step_by(2).collect::<String>();
        let usage = match row.usage {
            Usage::Bench => "bench",
            Usage::Beside => "beside",
            Usage::Input => "input",
        };
        assert!(
            words.contains(&format!("Table use: {usage}.")),
            "{}: ledger must say Table use: {usage}.",
            row.key,
        );
    }
}

#[test]
fn the_bending_interval_is_an_assumption_beside_the_published_activity_group() {
    let evidence = Evidence::read().unwrap();
    let row = evidence
        .row(TableKind::DailyLoads, "ventral-flexion")
        .unwrap();
    assert_eq!(row.usage, Usage::Bench);
    for (term, expected) in [("lower", 1.20), ("upper", 1.28)] {
        let cell = evidence
            .value(TableKind::DailyLoads, &row.key, term)
            .unwrap();
        assert_eq!(cell.value, expected);
        assert_eq!(cell.unit, Unit::Ratio);
        assert_eq!(cell.basis, Basis::OwnInference);
        assert_eq!(cell.source, "project-assumptions");
    }
    for (key, usage) in [
        ("stand-sit-bend-twist", Usage::Beside),
        ("bending-forward", Usage::Beside),
        ("slice-offset", Usage::Input),
    ] {
        assert_eq!(
            evidence.row(TableKind::DailyLoads, key).unwrap().usage,
            usage
        );
    }
}
