//! Empty or symmetric pixels cannot settle a direction. File hints stay
//! independent and turn with the view, even when anatomy contradicts them.

use darkroom::grey_image::{GreyImage, Placement};
use film_geometry::{rough_trace::RoughTrace, side_facing, upright};
use film_reader::decoded_film::DecodedFilm;
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, KeptTags, PixelValues, SourceFormat, WindowFunction,
    WindowSource,
};
use shapes::film_geometry::{Certainty, FacingCue, ImageRight, QuarterTurns, UprightCue};

fn drawing(kind: FilmKind, value: impl Fn(u32, u32) -> u16) -> DecodedFilm {
    let pixels = (0..320)
        .flat_map(|y| (0..160).map(move |x| (x, y)))
        .map(|(x, y)| value(x, y))
        .collect();
    DecodedFilm {
        image: GreyImage::new(160, 320, pixels, Placement::FILM).expect("one value per pixel"),
        facts: FilmFacts {
            kind,
            width: 160,
            height: 320,
            bits_stored: 16,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            default_window: DisplayWindow::CentreWidth {
                centre: 32767.5,
                width: 65535.0,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        },
    }
}

fn tags(axes: [&str; 2]) -> KeptTags {
    KeptTags {
        sop_class: None,
        modality: None,
        presentation_intent: None,
        manufacturer: None,
        model_name: None,
        photometric: None,
        bits_stored: None,
        pixel_spacing: None,
        imager_pixel_spacing: None,
        nominal_scanned_pixel_spacing: None,
        pixel_spacing_calibration: None,
        estimated_magnification: None,
        distance_source_to_detector: None,
        distance_source_to_patient: None,
        pixel_aspect_ratio: None,
        patient_orientation: Some(axes.map(str::to_owned)),
        view_position: None,
        image_laterality: None,
        rescale_slope: None,
        rescale_intercept: None,
        window_centre: None,
        window_width: None,
        window_function: None,
        shutters: None,
        lossy_compression: None,
        lossy_ratio: None,
        burned_in_annotation: None,
        transfer_syntax: None,
        number_of_frames: None,
    }
}

#[test]
fn blank_and_symmetric_fields_do_not_propose_anatomical_orientation() {
    for shape in 0..2 {
        let film = drawing(FilmKind::Side, |x, _| {
            if shape == 1 && (40..120).contains(&x) {
                40000
            } else {
                0
            }
        });
        let trace = RoughTrace::new(&film).expect("drawing");
        let up = upright::guess(&film, &trace);
        assert_eq!(up.certainty, Certainty::None, "shape {shape}: {up:?}");
        let facing = side_facing::guess(
            &film,
            &trace.profile(QuarterTurns::NONE),
            QuarterTurns::NONE,
        );
        assert_eq!(facing.image_right, None, "shape {shape}: {facing:?}");
        assert_eq!(facing.certainty, Certainty::None);
    }
}

#[test]
fn file_facing_turns_with_the_stored_axes_and_respects_the_view() {
    let mut film = drawing(FilmKind::Front, |_, _| 0);
    for (axes, q) in [
        (["L", "F"], 0),
        (["F", "R"], 1),
        (["R", "H"], 2),
        (["H", "L"], 3),
    ] {
        film.facts.kept_tags = Some(tags(axes));
        assert_eq!(
            upright::file_image_right(&film, QuarterTurns::new(q).expect("turn")),
            Some(ImageRight::PatientLeft)
        );
    }
    film.facts.kind = FilmKind::Side;
    assert_eq!(
        upright::file_image_right(&film, QuarterTurns::new(3).expect("turn")),
        None
    );
}

#[test]
fn conflicting_file_and_exif_hints_are_shown_as_conflicting() {
    let mut film = drawing(FilmKind::Front, |_, _| 0);
    film.facts.kept_tags = Some(tags(["L", "F"]));
    film.facts.exif_quarter_turns = QuarterTurns::new(2);
    let trace = RoughTrace::new(&film).expect("drawing");
    let guess = upright::guess(&film, &trace);
    assert_eq!(guess.certainty, Certainty::Low);
    assert!(
        guess
            .evidence
            .iter()
            .any(|e| e.cue == UprightCue::FileOrientation && e.strength == Some(1.0))
    );
    assert!(
        guess
            .evidence
            .iter()
            .any(|e| e.cue == UprightCue::ExifOrientation && e.strength == Some(-1.0))
    );
    let facing = side_facing::guess(
        &film,
        &trace.profile(QuarterTurns::NONE),
        QuarterTurns::NONE,
    );
    assert_eq!(facing.image_right, None);
    assert!(
        facing
            .evidence
            .iter()
            .any(|e| e.cue == FacingCue::FileOrientation && e.strength.is_none())
    );
}

#[test]
fn anatomy_and_a_contradictory_facing_tag_remain_separate_evidence() {
    // An analytically drawn trunk gets wider towards the pelvis. Its
    // narrow bright ridge stays behind the body centre and bows further
    // back in the thorax. The anterior side is therefore screen right.
    let mut film = drawing(FilmKind::Side, |x, y| {
        let half_width = 30.0 + f64::from(y) / 16.0;
        let spine = 65.0 - 15.0 * libm::sin(core::f64::consts::PI * f64::from(y) / 300.0);
        if (f64::from(x) - spine).abs() < 4.0 {
            65535
        } else if (f64::from(x) - 90.0).abs() < half_width {
            40000
        } else {
            0
        }
    });
    film.facts.kept_tags = Some(tags(["P", "F"]));
    let trace = RoughTrace::new(&film).expect("drawing");
    let facing = side_facing::guess(
        &film,
        &trace.profile(QuarterTurns::NONE),
        QuarterTurns::NONE,
    );
    assert_eq!(facing.image_right, Some(ImageRight::Anterior), "{facing:?}");
    assert_eq!(facing.file_says, Some(ImageRight::Posterior));
    assert_eq!(facing.certainty, Certainty::Low);
    assert!(
        facing
            .evidence
            .iter()
            .any(|e| e.cue == FacingCue::FileOrientation && e.strength == Some(-1.0))
    );
}
