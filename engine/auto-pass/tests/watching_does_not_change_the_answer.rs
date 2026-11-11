//! Watching observes checkpoints without changing a stage or final answer.

#[path = "../src/watch/test_films.rs"]
mod test_films;

use auto_pass::{run, watch::FilmPass};
use shapes::film::FilmKind;
use shapes::watch_frames::{OutlineQuad, WatchFrame};
use std::rc::Rc;

#[test]
fn watched_and_direct_passes_return_the_same_marks_and_final_outlines() {
    let (image, film, _) = test_films::film(FilmKind::Front);
    let input = test_films::input(vec![film]);
    let direct = run::auto_pass(&input, std::slice::from_ref(&image)).unwrap();
    let (watched, frames) = run::watch_auto_pass(&input, &[image], 7).unwrap();
    assert_eq!(watched, direct);
    let marks = &direct.marks[0];
    let expected: Vec<_> = marks
        .vertebrae
        .iter()
        .map(|body| OutlineQuad::new(body.level, &body.marks))
        .collect();
    let last = frames
        .iter()
        .rev()
        .find_map(|frame| match frame {
            WatchFrame::Outlines {
                film: FilmKind::Front,
                quads,
                is_final: true,
                ..
            } => Some(quads),
            _ => None,
        })
        .expect("a final outline frame");
    assert_eq!(*last, expected);
    let named = frames
        .iter()
        .find_map(|frame| match frame {
            WatchFrame::LevelsNamed { levels, .. } => Some(levels),
            _ => None,
        })
        .expect("named levels");
    assert_eq!(
        *named,
        marks
            .vertebrae
            .iter()
            .map(|body| body.level)
            .collect::<Vec<_>>()
    );
    assert!(named.windows(2).all(|pair| pair[0] < pair[1]));
    let locked = frames
        .iter()
        .find_map(|frame| match frame {
            WatchFrame::ChainLocked { bodies, .. } => Some(bodies),
            _ => None,
        })
        .expect("locked chain");
    assert!(locked.windows(2).all(|pair| pair[0].level > pair[1].level));
}

#[test]
fn watching_keeps_the_exact_stage_maps_and_float_bits() {
    let (image, film, _) = test_films::film(FilmKind::Front);
    let (direct_result, direct) = run::film_pass(&image, &film).unwrap();
    let mut watched = FilmPass::new(Rc::new(image), film).unwrap();
    while !watched.advance(1).unwrap().done {}
    assert_eq!(watched.result().unwrap(), direct_result);
    let cache = watched.cache().unwrap();
    assert_eq!(cache.evidence, direct.evidence);
    assert_eq!(cache.centre_line, direct.centre_line);
    // Strip invalid samples and invalid barcode rows are NaNs, for which
    // floating-point equality is false. Compare the stored bits instead.
    assert_eq!(cache.strip.to_bytes(), direct.strip.to_bytes());
    for (a, b) in [
        (&cache.barcode.values, &direct.barcode.values),
        (&cache.barcode.high_pass, &direct.barcode.high_pass),
    ] {
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits()));
    }
    assert_eq!(cache.barcode.valid, direct.barcode.valid);
    assert!(watched.advance(1).unwrap().frames.is_empty());
    watched.take_cache().unwrap();
    assert_eq!(watched.result().unwrap(), direct_result);
    assert!(watched.take_cache().is_err());
}
