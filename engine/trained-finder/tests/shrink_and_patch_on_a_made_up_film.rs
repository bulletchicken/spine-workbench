//! The two stages that must match the reference to the byte, the shrink to
//! the network's size and the patch cut, on the made-up film the known
//! answers were recorded on.
//!
//! The film is not kept in git: it is painted again here from its recipe
//! and checked against the recorded PNG's SHA-256 first. The pictures
//! are then compared by their SHA-256 alone.

mod geometry_fixtures;
mod made_up_film;

use geometry_fixtures::*;
use made_up_film::{made_up_front, sha256};
use trained_finder::corners::patch::cut;
use trained_finder::framing::{Frame, FrameReason};
use trained_finder::intake::shrink::to_network_size;

fn recorded_sha(name: &str) -> String {
    manifest()["blobs"][name]["sha256"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn the_shrunk_pictures_and_the_patches_match_the_reference_to_the_byte() {
    let run = pipeline();
    let film = made_up_front().top_eight_bits();
    let size = [film.width(), film.height()];
    assert_eq!(size, [1409, 3128]);

    // The first reading's picture: the whole film.
    assert_eq!(
        sha256(&to_network_size(&film)),
        recorded_sha("pass1_input_u8")
    );

    // The second reading's picture: the recorded frame's crop.
    let bounds = [0, 1, 2, 3].map(|k| run["frame"]["box"][k].as_i64().unwrap());
    let frame = Frame {
        bounds,
        size,
        reason: FrameReason::SmallSpineCrop,
    };
    assert_eq!(
        sha256(&to_network_size(
            &frame.crop(&film).expect("the crop fits in memory")
        )),
        recorded_sha("pass2_input_u8")
    );

    // The seventeen patches, cut from the recorded chain on the film.
    let scale = size.map(f64::from);
    let mut joined = Vec::new();
    for link in links(&run["chainInFilmFractions"]) {
        let quad = link.corners.map(|[x, y]| [x * scale[0], y * scale[1]]);
        joined.extend(cut(&film, &quad).pixels);
    }
    assert_eq!(joined.len(), 17 * 128 * 128);
    assert_eq!(sha256(&joined), recorded_sha("refiner_patches_u8"));
}
