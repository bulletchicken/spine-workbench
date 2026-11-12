//! Both networks match the reference on the made-up film in
//! `known-answers/`: each block of the main network, its three heads, the
//! peaks and outlines read from them, and the corner network's moves, all
//! within the tolerances in `known_answer_checks`.
//!
//! The pictures the networks read are made here the way the finder makes
//! them, from the made-up film, and must come out with the fingerprints the
//! manifest records before anything else is compared, so a mismatch in
//! the network is never confused with one in the picture.

mod known_answer_checks;
mod made_up_film;

use known_answer_checks::{
    check_corner_moves, check_reading, expected_sha256, pipeline, sha256_hex,
};
use made_up_film::made_up_front;
use trained_finder::corners::patch::cut;
use trained_finder::framing::{Frame, FrameReason};
use trained_finder::intake::shrink::to_network_size;

/// The picture the main network reads on `pass` (`"pass1"`: the whole
/// film; `"pass2"`: the film cropped to the frame in `pipeline.json`),
/// 384 by 768, row by row: the made-up film, each value shifted right by
/// eight bits, cropped for the second pass, then shrunk to the network's
/// size by the intake.
fn network_picture(pass: &str) -> Vec<u8> {
    let film = made_up_front().top_eight_bits();
    if pass == "pass1" {
        return to_network_size(&film);
    }
    let frame = Frame {
        bounds: [0, 1, 2, 3].map(|k| pipeline()["frame"]["box"][k].as_i64().expect("an edge")),
        size: [film.width(), film.height()],
        reason: FrameReason::SmallSpineCrop,
    };
    to_network_size(&frame.crop(&film).expect("the crop fits in memory"))
}

/// The seventeen patches the corner network reads, 128 by 128 each, one
/// after another: cut from the same 8-bit film around each outline of the
/// chain in `pipeline.json`, by the corners' patch cutter.
fn corner_patches() -> Vec<u8> {
    let film = made_up_front().top_eight_bits();
    let size = [film.width(), film.height()].map(f64::from);
    let mut patches = Vec::new();
    for link in pipeline()["chainInFilmFractions"]
        .as_array()
        .expect("a chain")
    {
        let corners = &link["corners"];
        let quad = [0, 1, 2, 3].map(|corner| {
            [0, 1].map(|axis| corners[corner][axis].as_f64().expect("a number") * size[axis])
        });
        patches.extend(cut(&film, &quad).pixels);
    }
    patches
}

#[test]
fn the_main_network_matches_the_reference_on_both_readings() {
    for pass in ["pass1", "pass2"] {
        let picture = network_picture(pass);
        let name = format!("{pass}_input_u8");
        assert_eq!(
            sha256_hex(&picture),
            expected_sha256(&name),
            "the {pass} picture is the one the reference read"
        );
        let gaps = check_reading(pass, &picture);
        eprintln!("{pass}: {gaps:?}");
    }
}

#[test]
fn the_corner_network_matches_the_reference_on_every_patch() {
    let patches = corner_patches();
    assert_eq!(
        sha256_hex(&patches),
        expected_sha256("refiner_patches_u8"),
        "the patches are the ones the reference read"
    );
    let worst = check_corner_moves(&patches);
    eprintln!("corner moves: largest gap {worst:e} patch units");
}
