//! Native wall time for fitted-film proposals and the load bench. Each
//! placement runs on the same ensemble, four versions per engine call.

use crate::{MeasureArgs, Outcome};
use shapes::implant_layout::{ChildFacts, LayoutFilm, LayoutInput, LayoutProposals, NamingDoubt};
use shapes::load_bench::{BenchCompareInput, BenchInput, BenchResult};
use shapes::spine_fit::{FilmToFit, FitLengths};
use shapes::spine_readings::ReadingsInput;
use std::io::Write;
use std::time::{Duration, Instant};

/// This command's scheduling choice, matching the worker's four-version slices.
const BATCH_VERSIONS: u32 = 4;

struct Timings {
    prepare: Duration,
    proposals: Duration,
    placements: Vec<Duration>,
    compare: Duration,
    total: Duration,
}

pub fn run(args: &MeasureArgs) -> Outcome {
    let started = Instant::now();
    let input = if args.recipe.is_some() {
        super::measure::made_up_input(args)?
    } else {
        super::measure::films_input(&args.films)?
    };
    let prepare = started.elapsed();
    let (proposals, result, mut timings) = bench(&input)?;
    timings.prepare = prepare;
    timings.total = started.elapsed();
    let mut out = std::io::stdout().lock();
    if args.json {
        writeln!(out, "{}", json(&proposals, &result)?)?;
    } else {
        print_table(&mut out, input.fit.lengths, &proposals, &result, &timings)?;
    }
    Ok(())
}

fn layout_film(film: &FilmToFit) -> LayoutFilm {
    LayoutFilm {
        marks: film.marks.clone(),
        geometry: film.geometry.clone(),
        size: film.size,
    }
}

fn bench(
    input: &ReadingsInput,
) -> Result<(LayoutProposals, BenchResult, Timings), Box<dyn std::error::Error>> {
    let started = Instant::now();
    // Neither a recipe nor a film supplies these facts. The engine keeps
    // its population assumptions separate from supplied child facts.
    let child = ChildFacts {
        age_years: None,
        weight_kg: None,
        activity: None,
    };
    let proposals = implant_layout::propose_layouts(&LayoutInput {
        fit: input.fit.clone(),
        front: input.front.as_ref().map(layout_film),
        bend_left: input.bend_left.as_ref().map(layout_film),
        bend_right: input.bend_right.as_ref().map(layout_film),
        child,
        naming: if input.front.as_ref().is_some_and(|film| {
            film.marks
                .naming
                .flags
                .contains(&shapes::naming::NamingFlag::Estimated)
        }) {
            NamingDoubt::MayBeOneOff
        } else {
            NamingDoubt::AsRead
        },
        versions: input.versions,
        seed: input.seed,
        pedicle_child_share: None,
    })?;
    let proposal_time = started.elapsed();
    if proposals.layouts.is_empty() {
        return Err("load-bench: no placement could be proposed from these films".into());
    }
    let mut batches = Vec::new();
    let mut placement_times = Vec::with_capacity(proposals.layouts.len());
    for layout in &proposals.layouts {
        let started = Instant::now();
        let mut request = BenchInput {
            fit: input.fit.clone(),
            layouts: vec![layout.clone()],
            child,
            seed: input.seed,
            versions: input.versions,
            version_from: 0,
            version_count: BATCH_VERSIONS.min(input.versions),
        };
        for from in (0..input.versions).step_by(BATCH_VERSIONS as usize) {
            request.version_from = from;
            request.version_count = BATCH_VERSIONS.min(input.versions - from);
            batches.push(load_bench::run_load_bench(&request)?);
        }
        placement_times.push(started.elapsed());
    }
    let started = Instant::now();
    let result = load_bench::compare_layouts(&BenchCompareInput {
        layouts: proposals.layouts.clone(),
        batches,
    })?;
    let timings = Timings {
        prepare: Duration::ZERO,
        proposals: proposal_time,
        placements: placement_times,
        compare: started.elapsed(),
        total: Duration::ZERO,
    };
    Ok((proposals, result, timings))
}

fn json(proposals: &LayoutProposals, result: &BenchResult) -> serde_json::Result<String> {
    // Serialise the contracts themselves, preserving their field names,
    // explicit nulls, reason codes and provenance without a display mapping.
    serde_json::to_string(&serde_json::json!({"proposals": proposals, "result": result}))
}

fn print_table(
    out: &mut impl Write,
    lengths: FitLengths,
    proposals: &LayoutProposals,
    result: &BenchResult,
    timings: &Timings,
) -> Outcome {
    writeln!(
        out,
        "tier: {}; lengths: {}",
        serde_json::to_value(proposals.tier)?
            .as_str()
            .ok_or("tier is not a name")?,
        match lengths {
            FitLengths::Measured => "measured",
            FitLengths::TypicalSize => "typical-size",
        }
    )?;
    let placements = proposals.layouts.len();
    let versions = proposals.versions;
    let runs = placements * versions as usize;
    writeln!(
        out,
        "placements: {placements}; versions per placement: {versions}; placement-version runs: {runs}; batch: {BATCH_VERSIONS} versions"
    )?;
    writeln!(out, "first placement: {}", result.first_id)?;
    writeln!(out, "level side kind diameter_mm length_mm look_codes")?;
    for anchor in &proposals.layouts[0].anchors {
        // Display names come from the wire, so look codes remain suitable
        // for looking up the same evidence as the JSON output.
        let anchor = serde_json::to_value(anchor)?;
        writeln!(
            out,
            "{} {} {} {} {} {}",
            anchor["level"].as_str().ok_or("level is not a name")?,
            anchor["side"].as_str().ok_or("side is not a name")?,
            anchor["kind"].as_str().ok_or("anchor kind is not a name")?,
            anchor["diameterMm"],
            anchor["lengthMm"],
            anchor["looks"],
        )?;
    }
    writeln!(out, "wall time (seconds; bench includes all stops):")?;
    writeln!(
        out,
        "placement span versions total_s per_version_s unsettled"
    )?;
    for ((layout, duration), compared) in proposals
        .layouts
        .iter()
        .zip(&timings.placements)
        .zip(&result.layouts)
    {
        writeln!(
            out,
            "{} {:?}-{:?} {} {:.6} {:.6} {}",
            layout.id,
            layout.upper,
            layout.lower,
            versions,
            duration.as_secs_f64(),
            duration.as_secs_f64() / f64::from(versions),
            compared.versions_unsettled,
        )?;
    }
    print_totals(out, timings, placements, versions)
}

fn print_totals(
    out: &mut impl Write,
    timings: &Timings,
    placements: usize,
    versions: u32,
) -> Outcome {
    let bench: Duration = timings.placements.iter().sum();
    writeln!(out, "prepare_s: {:.6}", timings.prepare.as_secs_f64())?;
    writeln!(out, "proposals_s: {:.6}", timings.proposals.as_secs_f64())?;
    writeln!(out, "bench_s: {:.6}", bench.as_secs_f64())?;
    writeln!(out, "compare_s: {:.6}", timings.compare.as_secs_f64())?;
    writeln!(out, "total_s: {:.6}", timings.total.as_secs_f64())?;
    writeln!(
        out,
        "bench_mean_per_placement_s: {:.6}",
        bench.as_secs_f64() / placements as f64
    )?;
    writeln!(
        out,
        "bench_mean_per_placement_version_s: {:.6}",
        bench.as_secs_f64() / (placements as f64 * f64::from(versions))
    )?;
    writeln!(
        out,
        "bench_mean_per_version_all_placements_s: {:.6}",
        bench.as_secs_f64() / f64::from(versions)
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn recipe_args(front_only: bool) -> MeasureArgs {
        MeasureArgs {
            recipe: Some(crate::Recipe::RightThoracic40),
            seed: Some(1),
            films: Vec::new(),
            front_only,
            json: false,
        }
    }

    #[test]
    fn the_bench_accepts_recipes_and_real_films_with_the_same_view_switches() {
        for args in [
            vec![
                "--recipe",
                "right-thoracic-40",
                "--seed",
                "1",
                "--front-only",
                "--json",
            ],
            vec!["--film", "front=x", "--film", "side=y", "--json"],
        ] {
            let parsed =
                crate::Cli::try_parse_from(["spine", "load-bench"].into_iter().chain(args))
                    .expect("bench arguments");
            let crate::Command::LoadBench(args) = parsed.command else {
                panic!("load-bench arguments");
            };
            assert!(args.json);
        }
        for args in [
            vec![],
            vec!["--recipe", "straight"],
            vec!["--film", "front=x", "--recipe", "straight", "--seed", "1"],
            vec!["--film", "front=x", "--front-only"],
        ] {
            assert!(
                crate::Cli::try_parse_from(["spine", "load-bench"].into_iter().chain(args),)
                    .is_err()
            );
        }
    }

    #[test]
    fn placement_and_version_times_have_their_distinct_denominators() {
        // Two placements taking 8 and 12 seconds across four versions:
        // 20/2 = 10 per placement, 20/8 = 2.5 per placement-version,
        // and 20/4 = 5 per version across the complete placement set.
        let timings = Timings {
            prepare: Duration::from_secs(1),
            proposals: Duration::from_secs(2),
            placements: vec![Duration::from_secs(8), Duration::from_secs(12)],
            compare: Duration::from_secs(3),
            total: Duration::from_secs(26),
        };
        let mut out = Vec::new();
        print_totals(&mut out, &timings, 2, 4).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            concat!(
                "prepare_s: 1.000000\nproposals_s: 2.000000\nbench_s: 20.000000\n",
                "compare_s: 3.000000\ntotal_s: 26.000000\n",
                "bench_mean_per_placement_s: 10.000000\n",
                "bench_mean_per_placement_version_s: 2.500000\n",
                "bench_mean_per_version_all_placements_s: 5.000000\n"
            )
        );
    }

    fn check_bench(input: &ReadingsInput) {
        let (proposals, result, timings) = bench(input).expect("proposals and bench comparison");
        assert!(!proposals.layouts.is_empty());
        assert_eq!(result.versions, input.versions);
        assert_eq!(result.first_id, "first");
        assert_eq!(result.layouts.len(), proposals.layouts.len());
        assert!(result.layouts[0].versions_unsettled < input.versions);
        assert!(result.layouts[0].stops.iter().any(|stop| {
            stop.stop == shapes::load_bench::BenchStop::RodsIn && !stop.estimates.is_empty()
        }));
        for layout in &result.layouts {
            assert!(layout.versions_unsettled <= input.versions);
            for count in &layout.counts {
                assert!(count.lower + count.higher + count.ties <= input.versions);
            }
        }
        let text = json(&proposals, &result).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 2);
        assert_eq!(
            serde_json::from_value::<LayoutProposals>(value["proposals"].clone()).unwrap(),
            proposals
        );
        assert_eq!(
            serde_json::from_value::<BenchResult>(value["result"].clone()).unwrap(),
            result
        );
        let mut table = Vec::new();
        print_table(&mut table, input.fit.lengths, &proposals, &result, &timings).unwrap();
        let table = String::from_utf8(table).unwrap();
        assert!(table.contains("diameter_mm length_mm look_codes"));
        assert!(table.contains("bench_mean_per_placement_version_s:"));
    }

    #[test]
    #[ignore = "fits films and runs every proposed placement; run explicitly in release"]
    fn an_unscaled_front_film_runs_through_the_typical_fit_and_the_bench() {
        use shapes::spine_fit::{FitFromFilms, TypicalSource};
        let mut input = super::super::measure::made_up_input(&recipe_args(true)).unwrap();
        let front = input.front.as_mut().unwrap();
        front.geometry.mm_per_px = None;
        front.geometry.mm_per_px_vertical = None;
        front.geometry.scale = None;
        front.geometry.confirmed.scale = false;
        input.fit = joint_fit::fit_from_films(&FitFromFilms {
            front: front.clone(),
            side: None,
            typical: TypicalSource::BuiltIn,
            twist_readings: Vec::new(),
        })
        .unwrap();
        assert_eq!(input.fit.lengths, FitLengths::TypicalSize);
        assert!(input.fit.side_film.is_none());
        // Five versions exercise a complete four-version slice and its remainder.
        input.versions = 5;
        check_bench(&input);
    }

    #[test]
    #[ignore = "fits films and runs every proposed placement; run explicitly in release"]
    fn a_front_and_side_fit_run_through_proposals_and_the_bench() {
        let mut input = super::super::measure::made_up_input(&recipe_args(false)).unwrap();
        assert!(
            input.fit.side_film.is_some(),
            "the side film must actually be fitted"
        );
        assert_eq!(input.fit.lengths, FitLengths::Measured);
        input.versions = 4;
        check_bench(&input);
    }
}
