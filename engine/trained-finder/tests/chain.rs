//! The chain against the recorded run on a made-up film, and against the
//! hand-made cases: one peak taken out at a time, and nothing at all.
//!
//! Then the chain against the reference's own way of working it, which
//! prices every pair of peaks (`every_pair`): on every recorded case, on
//! made-up peak lists of many sizes and shapes, odd ones included, and on
//! a lattice of weak peaks like the one a blank film gives. The two must
//! agree exactly, link for link and bit for bit.

mod every_pair;
mod geometry_fixtures;

use geometry_fixtures::*;
use trained_finder::chain::select;
use trained_finder::outline::{Peak, Quad};

#[test]
fn the_second_reading_gives_the_recorded_chain() {
    let run = pipeline();
    let found = select(&peaks(&run["pass2"]["peaks"])).expect("the chain runs");
    let expected = links(&run["chainInCropFractions"]);
    assert_eq!(found.len(), 17);
    let largest = compare_chains(&found, &expected, 1e-12);
    eprintln!("chain on the second reading: largest difference {largest:e}");
}

#[test]
fn each_hand_made_case_gives_its_recorded_chain() {
    let cases = chain_cases();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 4);
    let mut fills = 0;
    for case in cases {
        let found = select(&peaks(&case["peaks"])).expect("the chain runs");
        let expected = links(&case["expected"]);
        let largest = compare_chains(&found, &expected, 1e-12);
        eprintln!("{}: largest difference {largest:e}", case["name"]);
        fills += found.iter().filter(|link| link.filled_in).count();
    }
    // Three cases each lose one picked peak, which the chain fills.
    assert_eq!(fills, 3);
}

/// The chain must be exactly the one the reference's every-pair search
/// gives.
fn same_as_every_pair(label: &str, peaks: &[Peak]) -> usize {
    let found = select(peaks).expect("the chain runs");
    assert_eq!(found, every_pair::select(peaks), "{label}");
    found.len()
}

#[test]
fn every_recorded_case_gives_the_every_pair_chain() {
    let run = pipeline();
    for pass in ["pass1", "pass2"] {
        for list in ["peaks", "outlines"] {
            same_as_every_pair(&format!("{pass} {list}"), &peaks(&run[pass][list]));
        }
    }
    for case in chain_cases().as_array().expect("a list") {
        same_as_every_pair(&case["name"].to_string(), &peaks(&case["peaks"]));
    }
}

/// A small seeded generator, so every run tries the same lists.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// Evenly between `low` and `high`.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * (self.next() as f64 / (1u64 << 53) as f64)
    }
}

/// An outline centred on `centre`, `width` across and `height` down, as
/// fractions of the picture, tilted by `tilt` (down per across).
fn outline(score: f64, centre: [f64; 2], width: f64, height: f64, tilt: f64) -> Peak {
    let [x, y] = centre;
    let (half_w, half_h) = (width / 2.0, height / 2.0);
    let corners: Quad = [
        [x - half_w, y - half_h - tilt * half_w],
        [x + half_w, y - half_h + tilt * half_w],
        [x + half_w, y + half_h + tilt * half_w],
        [x - half_w, y + half_h - tilt * half_w],
    ];
    Peak {
        score,
        centre,
        corners,
        cell: [0, 0],
    }
}

/// `count` outlines strewn about a spine-shaped band, with a spine's own
/// sizes, so the chain has real choices and real gaps to fill.
fn strewn(seeded: &mut Seeded, count: usize) -> Vec<Peak> {
    (0..count)
        .map(|_| {
            let down = seeded.between(-0.05, 1.05);
            outline(
                seeded.between(0.0, 1.0),
                [seeded.between(0.25, 0.75), down],
                seeded.between(0.1, 0.35),
                seeded.between(0.015, 0.07),
                seeded.between(-0.15, 0.15),
            )
        })
        .collect()
}

/// A spine's seventeen, top to bottom, a little out of place, among
/// `others` strewn outlines.
fn spine_among(seeded: &mut Seeded, others: usize) -> Vec<Peak> {
    let mut peaks: Vec<Peak> = (0..17)
        .map(|level| {
            let down = 0.08 + 0.05 * level as f64 + seeded.between(-0.01, 0.01);
            outline(
                seeded.between(0.3, 1.0),
                [0.5 + seeded.between(-0.03, 0.03), down],
                0.16 + 0.006 * level as f64,
                0.03 + 0.001 * level as f64,
                seeded.between(-0.05, 0.05),
            )
        })
        .collect();
    peaks.extend(strewn(seeded, others));
    peaks
}

#[test]
fn made_up_peak_lists_give_the_every_pair_chain() {
    let mut seeded = Seeded(17);
    let mut links = 0;
    for count in [0, 1, 2, 3, 5, 17, 40, 120, 300] {
        for _ in 0..6 {
            links += same_as_every_pair(&format!("{count} strewn"), &strewn(&mut seeded, count));
        }
    }
    for others in [0, 10, 100] {
        for _ in 0..4 {
            let mut peaks = spine_among(&mut seeded, others);
            links += same_as_every_pair(&format!("spine among {others}"), &peaks);
            // Every other spine peak gone, so the chain must fill gaps or
            // break.
            let mut level = 0;
            peaks.retain(|_| {
                level += 1;
                level > 17 || level % 3 != 0
            });
            links += same_as_every_pair(&format!("broken spine among {others}"), &peaks);
        }
    }
    // The chains are real chains, not all empty.
    assert!(links > 200, "{links} links in all");
}

#[test]
fn odd_peak_lists_give_the_every_pair_chain() {
    let mut seeded = Seeded(29);
    let spine = spine_among(&mut seeded, 0);
    // One peak, and two.
    same_as_every_pair("one peak", &spine[8..9]);
    same_as_every_pair("two peaks", &spine[8..10]);
    same_as_every_pair(
        "two peaks a level apart",
        &[spine[3].clone(), spine[5].clone()],
    );
    // Every peak at one height: no step can be taken.
    let level: Vec<Peak> = strewn(&mut seeded, 60)
        .into_iter()
        .map(|peak| outline(peak.score, [peak.centre[0], 0.4], 0.2, 0.03, 0.0))
        .collect();
    same_as_every_pair("all at one height", &level);
    // Every peak twice, and the whole spine three times: equal costs
    // everywhere, so only the order of the ties decides.
    let mut doubled = strewn(&mut seeded, 50);
    doubled.extend(doubled.clone());
    same_as_every_pair("each twice", &doubled);
    let mut thrice = spine.clone();
    thrice.extend(spine.clone());
    thrice.extend(spine.clone());
    same_as_every_pair("the spine three times", &thrice);
    // Outlines with no size, and outlines far off the picture.
    let mut odd = spine.clone();
    odd.push(outline(0.9, [0.5, 0.33], 0.0, 0.0, 0.0));
    odd.push(outline(0.9, [0.5, 0.34], 0.0, 0.0, 0.0));
    odd.push(outline(0.7, [-3.0, 9.0], 0.2, 0.03, 0.0));
    odd.push(outline(0.7, [4.0, -6.0], 5.0, 2.0, 1.0));
    same_as_every_pair("no size and far off", &odd);
    // Absurd corners, whose costs overflow to infinity or not a number,
    // and one with no score, which the chain leaves out.
    let mut absurd = spine;
    absurd.push(outline(0.8, [0.5, 1e300], 1e300, 1e300, 0.0));
    absurd.push(outline(0.8, [1e300, 0.45], 1e300, 0.03, 0.0));
    absurd.push(outline(0.8, [0.5, -1e300], 0.2, 1e300, 0.0));
    absurd.push(outline(0.0, [0.5, 0.5], 0.2, 0.03, 0.0));
    same_as_every_pair("absurd", &absurd);
}

/// A few hundred weak, overlapping outlines on the heat map's 96 by 192
/// grid, as a blank film gives them: the network's background heat is a
/// plateau, so cells all over it make peaks, each a wide, shallow outline
/// scoring from 0.05 up to `best`, most of them near 0.05. Every
/// `columns`th column and `rows`th row is tried, and each cell tried is
/// kept with the chance `kept`.
fn weak_lattice(
    seeded: &mut Seeded,
    columns: usize,
    rows: usize,
    kept: f64,
    best: f64,
) -> Vec<Peak> {
    let mut peaks = Vec::new();
    for column in (4..96).step_by(columns) {
        for row in (0..192).step_by(rows) {
            if seeded.between(0.0, 1.0) >= kept {
                continue;
            }
            let centre = [(column as f64 + 0.5) / 96.0, (row as f64 + 0.5) / 192.0];
            let weak = seeded.between(0.0, 1.0);
            peaks.push(outline(
                0.05 + (best - 0.05) * weak * weak,
                centre,
                seeded.between(0.25, 0.32),
                seeded.between(0.024, 0.06),
                seeded.between(-0.02, 0.02),
            ));
        }
    }
    peaks
}

#[test]
fn a_lattice_of_weak_peaks_gives_the_every_pair_chain() {
    let mut seeded = Seeded(41);
    // Runs of cells down a few columns, as on the blank films tried, as
    // weak as theirs and a little stronger; and a coarser lattice, whose
    // spacing suits a chain of all seventeen.
    for (columns, rows, kept, best) in [
        (11, 1, 0.3, 0.15),
        (11, 1, 0.3, 0.2),
        (11, 1, 0.3, 0.3),
        (7, 6, 1.0, 0.2),
    ] {
        let lattice = weak_lattice(&mut seeded, columns, rows, kept, best);
        assert!(lattice.len() > 400);
        let links = same_as_every_pair("weak lattice", &lattice);
        eprintln!("a weak lattice of {} peaks: {links} links", lattice.len());
    }
}

/// The engine has no clock, so the chain is timed here, natively, by hand,
/// against the every-pair search: on a blank film's peak list, one per
/// grey level, and on 3,000 strewn peaks. `cargo test --release -p
/// trained-finder --test chain -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run by hand in release"]
#[allow(clippy::disallowed_types)]
fn the_chain_timed_natively() {
    use std::time::Instant;
    use trained_finder::weights::Weights;
    // The middle of five runs, in milliseconds, and what the run gave.
    fn timed<T>(mut run: impl FnMut() -> T) -> (f64, T) {
        let mut times = Vec::new();
        let mut last = None;
        for _ in 0..5 {
            let started = Instant::now();
            last = Some(run());
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        (times[2], last.expect("five runs"))
    }
    let weights = Weights::baked().expect("weights");
    let mut lists = Vec::new();
    for value in [0u8, 128, 255] {
        let mut reading = weights
            .spine_net()
            .start(&vec![value; 384 * 768])
            .expect("a reading");
        while !reading.is_done() {
            reading.advance(1000).expect("a reading");
        }
        let maps = reading.into_maps().expect("maps");
        let peaks = trained_finder::reading::read(&maps, weights.threshold()).peaks;
        lists.push((format!("blank film at {value}"), peaks));
    }
    lists.push(("3,000 strewn".to_owned(), strewn(&mut Seeded(3), 3000)));
    for (label, peaks) in &lists {
        let (now, found) = timed(|| select(peaks).expect("the chain runs"));
        let (before, every) = timed(|| every_pair::select(peaks));
        assert_eq!(found, every, "{label}");
        eprintln!(
            "{label}: {} peaks, {} links; {now:.1} ms, every pair {before:.1} ms",
            peaks.len(),
            found.len()
        );
    }
}
