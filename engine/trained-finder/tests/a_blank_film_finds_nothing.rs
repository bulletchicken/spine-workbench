//! A film with no spine on it. The first reading proposes nothing, so the
//! frame is left whole with a warning that the evidence is low. On a flat
//! film the chain then holds a few weak guesses at most, and the pass finds
//! nothing rather than hand them back (the one place the engine departs
//! from the reference pipeline); every later call gives the same error.
//! However the work is sliced, the same frames come out before the error.
//!
//! Every grey gives a flat 384 by 768 picture to the network, so the films
//! are the smallest the pass will search, which keeps the intake cheap.

use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{
    DisplayWindow, Film, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat, WindowFunction,
    WindowSource,
};
use shapes::film_geometry::FilmGeometry;
use shapes::watch_frames::WatchFrame;
use std::rc::Rc;
use trained_finder::watch::TrainedPass;

/// A flat front film of one grey, 64 by 128, stored in 8 bits.
fn blank_film(grey: u8) -> (DecodedFilm, Film) {
    let (width, height) = (64, 128);
    let facts = FilmFacts {
        kind: FilmKind::Front,
        width,
        height,
        bits_stored: 8,
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues::AS_STORED,
        flags: Vec::new(),
        default_window: DisplayWindow::CentreWidth {
            centre: 127.5,
            width: 255.0,
            function: WindowFunction::LinearExact,
            from: WindowSource::Percentile,
        },
        exif_quarter_turns: None,
    };
    let film = Film {
        kind: FilmKind::Front,
        image: FilmImage {
            path: FilmKind::Front.png_path().to_owned(),
            width,
            height,
            bits_stored: 8,
        },
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues::AS_STORED,
        flags: Vec::new(),
        blanked: Vec::new(),
        geometry: FilmGeometry::UNKNOWN,
    };
    let pixels = vec![u16::from(grey); (width * height) as usize];
    let image = GreyImage::new(width, height, pixels, Placement::FILM).unwrap();
    (DecodedFilm { image, facts }, film)
}

/// Runs the pass on a blank film, offering `work_units` a call, until it
/// fails; checks that it then keeps failing the same way, and gives the
/// frames it sent and the error.
fn run_to_failure(grey: u8, work_units: u32) -> (Vec<WatchFrame>, EngineError) {
    let (decoded, film) = blank_film(grey);
    let mut pass = TrainedPass::new(Rc::new(decoded), film, 1).expect("the record suits the film");
    let mut frames = Vec::new();
    let error = loop {
        match pass.advance(work_units) {
            Ok(step) => {
                assert!(!step.done, "nothing to find, so never done");
                frames.extend(step.frames);
            }
            Err(error) => break error,
        }
    };
    for units in [0, 1, 1000] {
        assert_eq!(pass.advance(units).err(), Some(error.clone()));
    }
    assert_eq!(pass.result().err(), Some(error.clone()));
    (frames, error)
}

/// What a blank film sends before it fails: the start of the evidence
/// stage, then the warning the framing gave. The chain's own start goes
/// with the error of the unit that sent it.
fn sent_before_failing() -> Vec<WatchFrame> {
    vec![
        WatchFrame::StageStarted {
            film: FilmKind::Front,
            stage: Stage::Evidence,
        },
        WatchFrame::Warning {
            film: FilmKind::Front,
            stage: Stage::Evidence,
            warning: PassWarning {
                code: PassWarningCode::LowEvidence,
                film: FilmKind::Front,
                level: None,
                region: None,
            },
        },
    ]
}

/// Each grey with the slice size beside it, a different one in turn.
fn finds_nothing(greys: &[u8]) {
    for (grey, work_units) in greys.iter().zip([1, 8, 100_000].into_iter().cycle()) {
        let (frames, error) = run_to_failure(*grey, work_units);
        assert_eq!(error.code, ErrorCode::PassNoSpineFound, "grey {grey}");
        assert_eq!(frames, sent_before_failing(), "grey {grey}");
    }
}

#[test]
fn a_blank_film_sends_the_same_frames_and_error_whatever_the_slice_size() {
    let outcomes = [1, 8, 100_000].map(|work_units| run_to_failure(40, work_units));
    assert_eq!(outcomes[0].1.code, ErrorCode::PassNoSpineFound);
    assert_eq!(outcomes[0].0, sent_before_failing());
    assert_eq!(outcomes[1], outcomes[0], "eight units a call");
    assert_eq!(outcomes[2], outcomes[0], "every unit in one call");
}

// Left to the reference's rule, the chain picks one vertebra on the films
// at 0, 20, 200, 230 and 255, and four at 160, every one scoring under the
// framing's 0.25.

#[test]
fn dark_blank_films_find_nothing() {
    finds_nothing(&[0, 20, 128]);
}

#[test]
fn bright_blank_films_find_nothing() {
    finds_nothing(&[160, 200, 230, 255]);
}
