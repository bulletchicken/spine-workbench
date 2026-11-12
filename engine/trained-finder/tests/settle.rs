//! Where each patch sits, and the settled corners, against the recorded
//! run on a made-up film. The patches' pixels are checked against the film
//! itself in `shrink_and_patch_on_a_made_up_film.rs`.

mod geometry_fixtures;

use darkroom::grey_image::{GreyImage, Placement};
use geometry_fixtures::*;
use trained_finder::corners::patch::cut;
use trained_finder::corners::settle::settle;
use trained_finder::outline::Quad;

/// The chain's outlines in film pixels, measured from the film's outer
/// corner, as the reference hands them to the patch cut.
fn outlines_in_pixels() -> (Vec<Quad>, [f64; 2]) {
    let run = pipeline();
    let size = [0, 1].map(|k| number(&run["filmSize"][k]));
    let quads = links(&run["chainInFilmFractions"])
        .iter()
        .map(|link| link.corners.map(|[x, y]| [x * size[0], y * size[1]]))
        .collect();
    (quads, size)
}

/// Where a patch sits depends only on its outline, so a black film of the
/// right size serves.
fn black_film(size: [f64; 2]) -> GreyImage<u8> {
    let (width, height) = (size[0] as u32, size[1] as u32);
    GreyImage::new(
        width,
        height,
        vec![0; width as usize * height as usize],
        Placement::FILM,
    )
    .unwrap()
}

#[test]
fn each_patch_sits_where_the_reference_put_it() {
    let run = pipeline();
    let (quads, size) = outlines_in_pixels();
    let film = black_film(size);
    let geometry = run["corners"]["patchGeometry"].as_array().unwrap();
    let initial = run["corners"]["initialInPatchUnits"].as_array().unwrap();
    assert_eq!(quads.len(), geometry.len());
    let mut largest: f64 = 0.0;
    for (k, quad) in quads.iter().enumerate() {
        let patch = cut(&film, quad);
        let recorded = &geometry[k];
        largest = largest
            .max(point_gap(patch.centre, point(&recorded["centre"])))
            .max(point_gap(patch.axes[0], point(&recorded["axes"][0])))
            .max(point_gap(patch.axes[1], point(&recorded["axes"][1])))
            .max((patch.extent - number(&recorded["extent"])).abs());
        // The starting corners are single precision, so they must match
        // exactly.
        let recorded_initial = [0, 1, 2, 3].map(|c| point(&initial[k][c]));
        assert_eq!(
            patch.initial.map(|c| c.map(f64::from)),
            recorded_initial,
            "patch {k}"
        );
    }
    eprintln!("patch geometry: largest difference {largest:e}");
    assert!(largest <= 1e-9, "largest difference {largest:e}");
}

#[test]
fn the_recorded_moves_settle_the_corners_where_the_reference_did() {
    let run = pipeline();
    let (quads, size) = outlines_in_pixels();
    let film = black_film(size);
    let blend = number(&run["corners"]["blend"]);
    let deltas = run["corners"]["deltasInPatchUnits"].as_array().unwrap();
    let settled = run["corners"]["settledInFilmPixels"].as_array().unwrap();
    let mut largest: f64 = 0.0;
    for (k, quad) in quads.iter().enumerate() {
        let patch = cut(&film, quad);
        let moves = [0, 1, 2, 3].map(|c| [0, 1].map(|axis| number(&deltas[k][c][axis]) as f32));
        let found = settle(quad, &moves, &patch, blend);
        largest = largest.max(quad_gap(&found, &geometry_fixtures::quad(&settled[k])));
    }
    eprintln!("settled corners: largest difference {largest:e} px");
    assert!(largest <= 0.01, "largest difference {largest:e} px");
}
