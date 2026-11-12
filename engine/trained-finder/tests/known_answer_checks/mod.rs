//! Compares the networks with the reference's answers on the made-up film
//! in `known-answers/` (see the README there for how they were made).
//!
//! The tolerances are the ones the port was planned against: each block's
//! sums, extremes and centre patch within a relative 1e-4; the heads'
//! values within 2e-4 (heat), 1e-3 (corners, in cells) and 1e-4 (the
//! centre in its cell); scores within 1e-5 and outline corners within
//! 1e-5 of the picture's size; the corner network's moves within 1e-5
//! patch units. Peaks must be the same cells in the same order.

#![allow(dead_code)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use trained_finder::grid::Grid;
use trained_finder::outline::Peak;
use trained_finder::reading::read;
use trained_finder::spine_net::HeadMaps;
use trained_finder::weights::Weights;

const MANIFEST: &str = include_str!("../known-answers/manifest.json");
const PIPELINE: &str = include_str!("../known-answers/pipeline.json");
const PASS1_HEAT: &[u8] = include_bytes!("../known-answers/pass1_heat_logits_f32.bin");
const PASS2_HEAT: &[u8] = include_bytes!("../known-answers/pass2_heat_logits_f32.bin");

const BLOCK_TOLERANCE: f64 = 1e-4;
const HEAT_TOLERANCE: f64 = 2e-4;
const CORNERS_TOLERANCE: f64 = 1e-3;
const CENTRE_TOLERANCE: f64 = 1e-4;
const SCORE_TOLERANCE: f64 = 1e-5;
const PLACE_TOLERANCE: f64 = 1e-5;
const MOVE_TOLERANCE: f64 = 1e-5;

pub fn manifest() -> Value {
    serde_json::from_str(MANIFEST).expect("the manifest is JSON")
}

pub fn pipeline() -> Value {
    serde_json::from_str(PIPELINE).expect("the pipeline record is JSON")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The fingerprint the manifest records for one of its pictures.
pub fn expected_sha256(blob: &str) -> String {
    manifest()["blobs"][blob]["sha256"]
        .as_str()
        .expect("a recorded fingerprint")
        .to_owned()
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

/// The largest gap seen at each stage, kept to be printed.
#[derive(Debug, Default)]
pub struct Gaps {
    pub block_stats_relative: f64,
    pub block_patch_relative: f64,
    pub heat: f64,
    pub corners_patch: f64,
    pub centre_patch: f64,
    pub head_stats_relative: f64,
    pub score: f64,
    pub place: f64,
}

/// Relative gap, measured against at least 1 so values near zero are
/// held to the same absolute tolerance.
fn relative(got: f64, want: f64) -> f64 {
    (got - want).abs() / want.abs().max(1.0)
}

/// The statistics the manifest keeps for one map stack, over its values
/// without the border.
struct Stats {
    sum: f64,
    abs_sum: f64,
    square_sum: f64,
    min: f64,
    max: f64,
}

fn stats(grid: &Grid) -> Stats {
    let mut stats = Stats {
        sum: 0.0,
        abs_sum: 0.0,
        square_sum: 0.0,
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
    };
    for channel in 0..grid.channels() {
        for row in 0..grid.rows() {
            for &value in grid.row(channel, row) {
                let value = f64::from(value);
                stats.sum += value;
                stats.abs_sum += value.abs();
                stats.square_sum += value * value;
                stats.min = stats.min.min(value);
                stats.max = stats.max.max(value);
            }
        }
    }
    stats
}

/// Checks one map stack against the manifest's record of it: its shape,
/// statistics (relative 1e-4) and centre patch (relative 1e-4 for a
/// block, the head's own tolerance for a head). Returns the largest
/// relative statistic gap and the largest patch gap.
fn check_stack(pass: &str, name: &str, grid: &Grid, patch_tolerance: Option<f64>) -> (f64, f64) {
    let record = &manifest()[format!("{pass}Stats")][name];
    let shape: Vec<usize> = record["shape"]
        .as_array()
        .expect("a shape")
        .iter()
        .map(|side| side.as_u64().expect("a side") as usize)
        .collect();
    assert_eq!(
        shape,
        vec![grid.channels(), grid.rows(), grid.columns()],
        "{pass} {name}"
    );
    let got = stats(grid);
    let mut worst_stat = 0.0f64;
    for (label, value, key) in [
        ("sum", got.sum, "sum"),
        ("absolute sum", got.abs_sum, "absSum"),
        ("square sum", got.square_sum, "squareSum"),
        ("least", got.min, "min"),
        ("most", got.max, "max"),
    ] {
        let gap = relative(value, number(&record[key]));
        assert!(
            gap <= BLOCK_TOLERANCE,
            "{pass} {name} {label}: {value} against {}",
            record[key]
        );
        worst_stat = worst_stat.max(gap);
    }
    let at = &record["patchAt"];
    let (top, left) = (number(&at[0]) as usize, number(&at[1]) as usize);
    let mut worst_patch = 0.0f64;
    for (channel, rows) in record["patch"]
        .as_array()
        .expect("a patch")
        .iter()
        .enumerate()
    {
        for (r, columns) in rows.as_array().expect("patch rows").iter().enumerate() {
            for (c, want) in columns
                .as_array()
                .expect("patch columns")
                .iter()
                .enumerate()
            {
                let got = f64::from(grid.at(channel, top + r, left + c));
                let want = number(want);
                let (gap, tolerance) = match patch_tolerance {
                    Some(tolerance) => ((got - want).abs(), tolerance),
                    None => (relative(got, want), BLOCK_TOLERANCE),
                };
                assert!(
                    gap <= tolerance,
                    "{pass} {name} patch channel {channel} at ({}, {}): {got} against {want}",
                    top + r,
                    left + c
                );
                worst_patch = worst_patch.max(gap);
            }
        }
    }
    (worst_stat, worst_patch)
}

/// Runs one reading of `picture` a unit at a time, checks every block as
/// it finishes and the heads at the end, then reads the maps and checks
/// the peaks and outlines against the pipeline record. `pass` is
/// `"pass1"` or `"pass2"`.
pub fn check_reading(pass: &str, picture: &[u8]) -> Gaps {
    let weights = Weights::baked().expect("the compiled-in weights read");
    let net = weights.spine_net();
    let mut reading = net.start(picture).expect("a picture of the right size");
    let mut gaps = Gaps::default();
    let mut blocks_seen = Vec::new();
    while !reading.is_done() {
        reading.advance(1).expect("memory enough");
        if let Some((name, grid)) = reading.just_finished() {
            let (stat, patch) = check_stack(pass, name, grid, None);
            gaps.block_stats_relative = gaps.block_stats_relative.max(stat);
            gaps.block_patch_relative = gaps.block_patch_relative.max(patch);
            blocks_seen.push(name);
        }
    }
    assert_eq!(reading.units_done(), net.units());
    assert_eq!(
        blocks_seen,
        ["e1", "e2", "e3", "e4", "e5", "d4", "d3", "d2"]
    );
    let maps = reading.into_maps().expect("the reading is done");
    gaps.merge_heads(&check_heads(pass, &maps));
    check_read_maps(pass, &maps, weights.threshold(), &mut gaps);
    gaps
}

impl Gaps {
    fn merge_heads(&mut self, heads: &Gaps) {
        self.heat = self.heat.max(heads.heat);
        self.corners_patch = self.corners_patch.max(heads.corners_patch);
        self.centre_patch = self.centre_patch.max(heads.centre_patch);
        self.head_stats_relative = self.head_stats_relative.max(heads.head_stats_relative);
    }
}

/// Checks the three head maps: the whole heat map value by value, and each
/// head's statistics and centre patch.
pub fn check_heads(pass: &str, maps: &HeadMaps) -> Gaps {
    let mut gaps = Gaps::default();
    let heat_file = if pass == "pass1" {
        PASS1_HEAT
    } else {
        PASS2_HEAT
    };
    let mut worst = 0.0f64;
    for (index, (&four, &got)) in heat_file
        .as_chunks::<4>()
        .0
        .iter()
        .zip(maps.heat.plane(0))
        .enumerate()
    {
        let want = f32::from_le_bytes(four);
        let gap = f64::from((got - want).abs());
        assert!(
            gap <= HEAT_TOLERANCE,
            "{pass} heat logit at cell {index}: {got} against {want}"
        );
        worst = worst.max(gap);
    }
    assert_eq!(heat_file.len(), 4 * maps.heat.plane(0).len());
    gaps.heat = worst;
    for (name, grid, tolerance) in [
        ("heat", &maps.heat, HEAT_TOLERANCE),
        ("corners", &maps.corners, CORNERS_TOLERANCE),
        ("sub", &maps.centre_in_cell, CENTRE_TOLERANCE),
    ] {
        let (stat, patch) = check_stack(pass, name, grid, Some(tolerance));
        gaps.head_stats_relative = gaps.head_stats_relative.max(stat);
        match name {
            "heat" => gaps.heat = gaps.heat.max(patch),
            "corners" => gaps.corners_patch = patch,
            _ => gaps.centre_patch = patch,
        }
    }
    gaps
}

fn check_peak(pass: &str, list: &str, index: usize, got: &Peak, want: &Value, gaps: &mut Gaps) {
    let cell = [
        number(&want["cell"][0]) as u32,
        number(&want["cell"][1]) as u32,
    ];
    assert_eq!(got.cell, cell, "{pass} {list} {index}: cell");
    let score_gap = (got.score - number(&want["score"])).abs();
    assert!(
        score_gap <= SCORE_TOLERANCE,
        "{pass} {list} {index}: score {} against {}",
        got.score,
        want["score"]
    );
    gaps.score = gaps.score.max(score_gap);
    let mut places = vec![(got.centre, &want["centre"])];
    for (corner, place) in got.corners.iter().enumerate() {
        places.push((*place, &want["corners"][corner]));
    }
    for (got_place, want_place) in places {
        for axis in 0..2 {
            let gap = (got_place[axis] - number(&want_place[axis])).abs();
            assert!(
                gap <= PLACE_TOLERANCE,
                "{pass} {list} {index}: {got_place:?} against {want_place}"
            );
            gaps.place = gaps.place.max(gap);
        }
    }
}

/// Reads the maps and checks both lists against the pipeline record: the
/// same cells in the same order, each score and place within tolerance.
pub fn check_read_maps(pass: &str, maps: &HeadMaps, threshold: f64, gaps: &mut Gaps) {
    let reading = read(maps, threshold);
    let record = &pipeline()[pass];
    for (list, got) in [("peaks", &reading.peaks), ("outlines", &reading.outlines)] {
        let want = record[list].as_array().expect("a list");
        let got_cells: Vec<[u32; 2]> = got.iter().map(|peak| peak.cell).collect();
        let want_cells: Vec<[u32; 2]> = want
            .iter()
            .map(|peak| {
                [
                    number(&peak["cell"][0]) as u32,
                    number(&peak["cell"][1]) as u32,
                ]
            })
            .collect();
        assert_eq!(
            got_cells, want_cells,
            "{pass} {list}: the same cells in the same order"
        );
        for (index, (got, want)) in got.iter().zip(want).enumerate() {
            check_peak(pass, list, index, got, want, gaps);
        }
    }
}

/// Runs the corner network on each 128 by 128 patch in `patches` (one
/// after another) from the recorded starting corners, and checks its moves
/// against the record. Returns the largest gap.
pub fn check_corner_moves(patches: &[u8]) -> f64 {
    let net = Weights::baked()
        .expect("the compiled-in weights read")
        .corner_net();
    let record = &pipeline()["corners"];
    let initial = record["initialInPatchUnits"].as_array().expect("a list");
    let moves = record["deltasInPatchUnits"].as_array().expect("a list");
    assert_eq!(patches.len(), initial.len() * 128 * 128);
    let mut worst = 0.0f64;
    for (index, pixels) in patches.as_chunks::<{ 128 * 128 }>().0.iter().enumerate() {
        let start: [[f32; 2]; 4] = std::array::from_fn(|corner| {
            std::array::from_fn(|axis| number(&initial[index][corner][axis]) as f32)
        });
        let got = net.deltas_for(pixels, &start);
        for (corner, delta) in got.iter().enumerate() {
            for axis in 0..2 {
                let want = number(&moves[index][corner][axis]);
                let gap = (f64::from(delta[axis]) - want).abs();
                assert!(
                    gap <= MOVE_TOLERANCE,
                    "patch {index} corner {corner}: {delta:?} against {}",
                    moves[index][corner]
                );
                worst = worst.max(gap);
            }
        }
    }
    worst
}
