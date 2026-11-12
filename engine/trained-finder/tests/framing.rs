//! The framing against the recorded run on a made-up film, and a blank
//! picture that cannot be framed.

mod geometry_fixtures;

use darkroom::grey_image::{GreyImage, Placement};
use geometry_fixtures::*;
use trained_finder::framing::{Frame, FrameReason, frame_from_reading};
use trained_finder::intake::shrink::to_network_size;
use trained_finder::outline::Reading;

fn recorded_frame() -> Frame {
    let run = pipeline();
    let size = [0, 1].map(|k| run["filmSize"][k].as_u64().unwrap() as u32);
    frame_from_reading(size, &reading(&run["pass1"]))
}

#[test]
fn the_first_reading_gives_the_recorded_box_and_reason() {
    let run = pipeline();
    let frame = recorded_frame();
    let expected: Vec<i64> = run["frame"]["box"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert_eq!(frame.bounds.to_vec(), expected);
    assert_eq!(run["frame"]["reason"], "small-spine crop");
    assert_eq!(frame.reason, FrameReason::SmallSpineCrop);
    assert_eq!(
        frame.whole_film(),
        run["frame"]["unchanged"].as_bool().unwrap()
    );
}

#[test]
fn the_chain_maps_back_to_the_recorded_film_fractions() {
    let run = pipeline();
    let frame = recorded_frame();
    let crop = links(&run["chainInCropFractions"]);
    let film = links(&run["chainInFilmFractions"]);
    assert_eq!(crop.len(), film.len());
    let mut largest: f64 = 0.0;
    for (a, b) in crop.iter().zip(&film) {
        largest = largest
            .max(quad_gap(&a.corners.map(|c| frame.to_film(c)), &b.corners))
            .max(point_gap(frame.to_film(a.centre), b.centre));
    }
    eprintln!("chain mapped to the film: largest difference {largest:e}");
    assert!(largest <= 1e-12, "largest difference {largest:e}");
}

#[test]
fn a_blank_grey_picture_cannot_be_framed() {
    // The reference reads no peak at all on a flat grey 400 by 800 picture;
    // with nothing read, the frame is the whole film.
    let picture = GreyImage::new(400, 800, vec![40u8; 400 * 800], Placement::FILM).unwrap();
    assert!(to_network_size(&picture).iter().all(|&v| v == 40));
    let frame = frame_from_reading(
        [400, 800],
        &Reading {
            peaks: Vec::new(),
            outlines: Vec::new(),
        },
    );
    assert_eq!(frame.reason, FrameReason::InsufficientEvidence);
    assert!(frame.whole_film());
    assert_eq!(frame.bounds, [0, 0, 400, 800]);
}
