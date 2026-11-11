//! The checkpoint stream is independent of scheduling and never loses a
//! strip block, sweep, wavefront, named level or final outline.

#[path = "../src/watch/test_films.rs"]
mod test_films;

use auto_pass::handoffs::Strip;
use auto_pass::watch::{ChainPass, FilmPass};
use shapes::auto_pass::{ChainAnswer, ChainOptions, FilmPassResult, Stage};
use shapes::errors::ErrorCode;
use shapes::film::FilmKind;
use shapes::watch_frames::{MapKind, TileEncoding, WatchFrame};
use std::rc::Rc;

fn same_frames(a: &[WatchFrame], b: &[WatchFrame]) {
    assert_eq!(a.len(), b.len());
    for (index, (a, b)) in a.iter().zip(b).enumerate() {
        assert!(a == b, "checkpoint {index} changed with work budget");
    }
}

fn checkpoint_obeys_the_contract(frame: &WatchFrame) {
    let may_thin = matches!(frame, WatchFrame::JointProgress { .. })
        || matches!(frame, WatchFrame::Evidence { map, is_final: false, .. } if *map != MapKind::Wavefront);
    assert_eq!(frame.is_keyframe(), !may_thin);
    match frame {
        WatchFrame::Sweep { heat, .. } => assert_eq!(heat.len(), 256),
        WatchFrame::LevelsNamed { levels, .. } => {
            assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
        }
        WatchFrame::ChainLocked { bodies, .. } => {
            assert!(bodies.windows(2).all(|pair| pair[0].level > pair[1].level));
        }
        _ => {}
    }
}

fn chain_stream(
    result: &FilmPassResult,
    options: &ChainOptions,
    units: u32,
) -> (ChainAnswer, Vec<WatchFrame>) {
    let mut pass = ChainPass::new(vec![result.clone()], options.clone()).unwrap();
    let empty = pass.advance(0).unwrap();
    assert!(!empty.done);
    assert!(empty.frames.is_empty());
    let mut frames = Vec::new();
    loop {
        let step = pass.advance(units).unwrap();
        assert!((0.0..=1.0).contains(&step.fraction));
        frames.extend(step.frames);
        if step.done {
            break;
        }
    }
    assert!(pass.advance(units).unwrap().frames.is_empty());
    for frame in &frames {
        checkpoint_obeys_the_contract(frame);
    }
    (pass.result().unwrap(), frames)
}

fn payloads_obey_the_frame_contract(frames: &[WatchFrame], strip: &Strip) {
    let mut next_row = 0;
    let mut starts = Vec::new();
    let mut barcode_seen = false;
    for frame in frames {
        checkpoint_obeys_the_contract(frame);
        match frame {
            WatchFrame::StageStarted { stage, .. } => starts.push(*stage),
            WatchFrame::Evidence {
                step, steps, tile, ..
            } => {
                assert!(*step >= 1 && step <= steps);
                let bytes_per_cell = if tile.encoding == TileEncoding::SignedPair {
                    2
                } else {
                    1
                };
                assert_eq!(
                    tile.data.len(),
                    (tile.width * tile.height * bytes_per_cell) as usize
                );
                assert!(tile.origin.x().is_finite() && tile.origin.y().is_finite());
            }
            WatchFrame::StripRows {
                first_row,
                rows,
                columns,
                image,
                valid,
                ..
            } => {
                assert_eq!(*first_row, next_row);
                assert_eq!(*columns, strip.cols);
                assert!(*rows > 0 && *rows <= 64);
                let cells = (*rows * *columns) as usize;
                assert_eq!(image.len(), cells);
                assert_eq!(valid.len(), cells.div_ceil(8));
                for cell in 0..cells {
                    assert_eq!(
                        (valid[cell / 8] >> (cell % 8)) & 1,
                        strip.valid[(*first_row * *columns) as usize + cell]
                    );
                }
                next_row += rows;
            }
            WatchFrame::Barcode {
                values, high_pass, ..
            } => {
                assert_eq!(values.len(), strip.rows as usize * 4);
                assert_eq!(high_pass.len(), values.len());
                barcode_seen = true;
            }
            _ => {}
        }
    }
    assert_eq!(next_row, strip.rows);
    assert!(barcode_seen);
    assert_eq!(starts, [Stage::Evidence, Stage::CentreLine, Stage::Strip]);
}

#[test]
fn film_and_chain_checkpoints_are_identical_for_small_and_large_budgets() {
    let (image, film, _) = test_films::film(FilmKind::Front);
    let input = test_films::input(vec![film.clone()]);
    let image = Rc::new(image);
    let mut baseline: Option<(
        FilmPassResult,
        Vec<WatchFrame>,
        ChainAnswer,
        Vec<WatchFrame>,
    )> = None;
    for units in [1, 7, 65_535] {
        let mut pass = FilmPass::new(image.clone(), film.clone()).unwrap();
        let zero = pass.advance(0).unwrap();
        assert!(!zero.done);
        assert!(zero.frames.is_empty());
        let mut frames = Vec::new();
        loop {
            let step = pass.advance(units).unwrap();
            assert!((0.0..=1.0).contains(&step.fraction));
            frames.extend(step.frames);
            if step.done {
                break;
            }
        }
        let result = pass.result().unwrap();
        payloads_obey_the_frame_contract(&frames, &pass.cache().unwrap().strip);
        let (answer, chain_frames) = chain_stream(&result, &input.options, units);
        if let Some((old_result, old_frames, old_answer, old_chain_frames)) = &baseline {
            assert!(
                result == *old_result,
                "film result changed with work budget"
            );
            assert_eq!(answer, *old_answer);
            same_frames(&frames, old_frames);
            same_frames(&chain_frames, old_chain_frames);
        } else {
            baseline = Some((result, frames, answer, chain_frames));
        }
    }
    let (result, _, _, _) = baseline.unwrap();
    let mut cancelled = ChainPass::new(vec![result], input.options).unwrap();
    loop {
        let step = cancelled.advance(1).unwrap();
        if step
            .frames
            .iter()
            .any(|frame| matches!(frame, WatchFrame::Sweep { .. }))
        {
            assert!(!step.done, "cancel must interrupt an unfinished search");
            break;
        }
        assert!(!step.done, "a real sweep checkpoint must be observable");
    }
    cancelled.cancel();
    assert_eq!(
        cancelled.advance(1).unwrap_err().code,
        ErrorCode::PassCancelled
    );
    assert_eq!(
        cancelled.result().unwrap_err().code,
        ErrorCode::PassCancelled
    );
}
