//! Reading the known answers in `tests/known-answers/` for the geometry
//! tests: the framing, the chain, the patches and the settled corners.
//! The answers come from the reference pipeline run on a made-up film.

#![allow(dead_code)]

use serde_json::Value;
use trained_finder::outline::{ChainLink, Peak, Point, Quad, Reading};

/// The recorded run of the whole pipeline.
pub fn pipeline() -> Value {
    serde_json::from_str(include_str!("../known-answers/pipeline.json"))
        .expect("pipeline.json reads")
}

/// The chain alone, on hand-made inputs.
pub fn chain_cases() -> Value {
    serde_json::from_str(include_str!("../known-answers/chain-cases.json"))
        .expect("chain-cases.json reads")
}

/// SHA-256 sums and how the film was made.
pub fn manifest() -> Value {
    serde_json::from_str(include_str!("../known-answers/manifest.json"))
        .expect("manifest.json reads")
}

pub fn number(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

pub fn point(value: &Value) -> Point {
    [number(&value[0]), number(&value[1])]
}

pub fn quad(value: &Value) -> Quad {
    [0, 1, 2, 3].map(|k| point(&value[k]))
}

pub fn peak(value: &Value) -> Peak {
    Peak {
        score: number(&value["score"]),
        centre: point(&value["centre"]),
        corners: quad(&value["corners"]),
        cell: [0, 1].map(|k| value["cell"][k].as_u64().expect("a cell") as u32),
    }
}

pub fn peaks(value: &Value) -> Vec<Peak> {
    value.as_array().expect("a list").iter().map(peak).collect()
}

pub fn reading(value: &Value) -> Reading {
    Reading {
        peaks: peaks(&value["peaks"]),
        outlines: peaks(&value["outlines"]),
    }
}

/// A recorded chain link. A filled link has no peak, score 0 and no cell.
pub fn link(value: &Value) -> ChainLink {
    ChainLink {
        level_index: value["level_index"].as_u64().expect("a level") as usize,
        filled_in: value["filled_in"].as_bool().expect("a flag"),
        corners: quad(&value["corners"]),
        centre: point(&value["centre"]),
        score: number(&value["score"]),
        peak_index: value["peak_index"].as_u64().map(|i| i as usize),
    }
}

pub fn links(value: &Value) -> Vec<ChainLink> {
    value.as_array().expect("a list").iter().map(link).collect()
}

/// The largest difference between two points' coordinates.
pub fn point_gap(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

/// The largest difference between two quads' coordinates.
pub fn quad_gap(a: &Quad, b: &Quad) -> f64 {
    (0..4).map(|k| point_gap(a[k], b[k])).fold(0.0, f64::max)
}

/// Checks two chains link by link: levels, fills and peaks exactly,
/// numbers within `tolerance`. Returns the largest difference seen.
pub fn compare_chains(found: &[ChainLink], expected: &[ChainLink], tolerance: f64) -> f64 {
    assert_eq!(found.len(), expected.len(), "chain lengths");
    let mut largest: f64 = 0.0;
    for (k, (a, b)) in found.iter().zip(expected).enumerate() {
        assert_eq!(a.level_index, b.level_index, "link {k} level");
        assert_eq!(a.filled_in, b.filled_in, "link {k} fill");
        assert_eq!(a.peak_index, b.peak_index, "link {k} peak");
        largest = largest
            .max(quad_gap(&a.corners, &b.corners))
            .max(point_gap(a.centre, b.centre))
            .max((a.score - b.score).abs());
    }
    assert!(largest <= tolerance, "largest difference {largest:e}");
    largest
}
