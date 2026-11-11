//! Independent arithmetic and painted-object answers for every scale source.

use darkroom::grey_image::{GreyImage, Placement};
use film_geometry::scale::{
    from_known_object::from_known_object,
    from_ruler::{combine_rulers, from_ruler},
    from_tags::from_tags,
};
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::ErrorCode;
use shapes::film::{
    DisplayWindow, FilmFacts, FilmKind, KeptTags, PixelValues, SourceFormat, SpacingCalibration,
    WindowFunction, WindowSource,
};
use shapes::film_geometry::{FileScaleRule, Ruler, ScalePlane, ScaleSource};
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
use shapes::pixels::Px;

fn tags() -> KeptTags {
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
        patient_orientation: None,
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

fn film(image: GreyImage<u16>) -> DecodedFilm {
    DecodedFilm {
        facts: FilmFacts {
            kind: FilmKind::Front,
            width: image.width(),
            height: image.height(),
            bits_stored: 16,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: Vec::new(),
            exif_quarter_turns: None,
            default_window: DisplayWindow::CentreWidth {
                centre: 32767.5,
                width: 65535.0,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
        },
        image,
    }
}

fn blank(width: u32, height: u32) -> DecodedFilm {
    film(
        GreyImage::new(
            width,
            height,
            vec![0; (width * height) as usize],
            Placement::FILM,
        )
        .unwrap(),
    )
}

#[test]
fn spacing_rules_keep_their_depth_and_preference() {
    // All answers below are direct arithmetic; 1e-12 permits only rounding.
    let cases = [
        (
            KeptTags {
                pixel_spacing: Some([0.15; 2]),
                imager_pixel_spacing: Some([0.15; 2]),
                ..tags()
            },
            FileScaleRule::DetectorUncorrected,
            ScalePlane::DetectorFace,
            0.15,
            1,
        ),
        (
            KeptTags {
                imager_pixel_spacing: Some([0.15; 2]),
                estimated_magnification: Some(1.2),
                ..tags()
            },
            FileScaleRule::DetectorOverMagnification,
            ScalePlane::TableOrBucky,
            0.125,
            2,
        ),
        (
            KeptTags {
                pixel_spacing: Some([0.139; 2]),
                imager_pixel_spacing: Some([0.15; 2]),
                ..tags()
            },
            FileScaleRule::CorrectedUnknownMethod,
            ScalePlane::AssumedDepth,
            0.139,
            2,
        ),
        (
            KeptTags {
                pixel_spacing: Some([0.142; 2]),
                pixel_spacing_calibration: Some(SpacingCalibration::Fiducial),
                ..tags()
            },
            FileScaleRule::Fiducial,
            ScalePlane::FiducialDepth,
            0.142,
            1,
        ),
        (
            KeptTags {
                pixel_spacing: Some([0.142; 2]),
                pixel_spacing_calibration: Some(SpacingCalibration::Geometry),
                ..tags()
            },
            FileScaleRule::Geometry,
            ScalePlane::AssumedDepth,
            0.142,
            1,
        ),
        (
            KeptTags {
                pixel_spacing: Some([0.15; 2]),
                ..tags()
            },
            FileScaleRule::UnknownCalibration,
            ScalePlane::Unknown,
            0.15,
            1,
        ),
        (
            KeptTags {
                imager_pixel_spacing: Some([0.15; 2]),
                distance_source_to_detector: Some(1800.0),
                distance_source_to_patient: Some(1600.0),
                ..tags()
            },
            FileScaleRule::DetectorFromDistances,
            ScalePlane::TableOrBucky,
            0.13333333333333333,
            2,
        ),
        (
            KeptTags {
                imager_pixel_spacing: Some([0.15; 2]),
                estimated_magnification: Some(0.9),
                ..tags()
            },
            FileScaleRule::DetectorUncorrected,
            ScalePlane::DetectorFace,
            0.15,
            1,
        ),
        (
            KeptTags {
                pixel_spacing: Some([0.14; 2]),
                nominal_scanned_pixel_spacing: Some([0.15; 2]),
                ..tags()
            },
            FileScaleRule::CorrectedUnknownMethod,
            ScalePlane::AssumedDepth,
            0.14,
            1,
        ),
    ];
    for (tags, rule, plane, expected, count) in cases {
        let results = from_tags(Some(&tags));
        assert_eq!(results.len(), count);
        assert_eq!(results[0].source, ScaleSource::File { rule });
        assert_eq!(results[0].plane, plane);
        assert!((results[0].mm_per_px - expected).abs() < 1e-12);
    }
    assert!(from_tags(None).is_empty());
    for value in [0.0, 3.5, f64::NAN, f64::INFINITY] {
        assert!(
            from_tags(Some(&KeptTags {
                pixel_spacing: Some([value; 2]),
                ..tags()
            }))
            .is_empty()
        );
    }
    let result = from_tags(Some(&KeptTags {
        pixel_spacing: Some([0.15, 0.16]),
        ..tags()
    }));
    assert_eq!(result[0].mm_per_px, 0.16);
    assert_eq!(result[0].mm_per_px_vertical, Some(0.15));
    let all = from_tags(Some(&KeptTags {
        pixel_spacing: Some([0.142; 2]),
        pixel_spacing_calibration: Some(SpacingCalibration::Fiducial),
        imager_pixel_spacing: Some([0.15; 2]),
        estimated_magnification: Some(1.2),
        distance_source_to_detector: Some(1800.0),
        distance_source_to_patient: Some(1600.0),
        ..tags()
    }));
    assert_eq!(
        all.iter().map(|scale| &scale.source).collect::<Vec<_>>(),
        [
            &ScaleSource::File {
                rule: FileScaleRule::Fiducial
            },
            &ScaleSource::File {
                rule: FileScaleRule::CorrectedUnknownMethod
            },
            &ScaleSource::File {
                rule: FileScaleRule::DetectorOverMagnification
            },
            &ScaleSource::File {
                rule: FileScaleRule::DetectorUncorrected
            },
        ]
    );
}

#[test]
fn rulers_keep_subpixels_pixel_aspect_and_click_uncertainty() {
    let mut film = blank(1300, 1300);
    let result = from_ruler(
        &film,
        Px::new(100.25, 200.5),
        Px::new(100.25, 1200.5),
        150.0,
    )
    .unwrap();
    assert!((result.mm_per_px - 0.15).abs() < 1e-12);
    assert!((result.relative_uncertainty.unwrap() - 0.0014142135623730952).abs() < 1e-12);
    assert_eq!(result.plane, ScalePlane::ObjectDepth);
    assert!(!result.weak);
    film.facts.kept_tags = Some(KeptTags {
        pixel_spacing: Some([0.15, 0.16]),
        ..tags()
    });
    let result = from_ruler(&film, Px::new(100.0, 200.0), Px::new(1100.0, 200.0), 160.0).unwrap();
    assert!((result.mm_per_px - 0.16).abs() < 1e-12);
    assert!((result.mm_per_px_vertical.unwrap() - 0.15).abs() < 1e-12);
    for mm in [0.0, f64::NAN, f64::INFINITY, 2000.1] {
        assert_eq!(
            from_ruler(&film, Px::new(0.0, 0.0), Px::new(0.0, 500.0), mm)
                .unwrap_err()
                .code,
            ErrorCode::ScaleBadLength
        );
    }
    assert_eq!(
        from_ruler(&film, Px::new(0.0, 0.0), Px::new(0.0, 40.0), 10.0)
            .unwrap_err()
            .code,
        ErrorCode::ScaleTooShort
    );
    assert!(
        from_ruler(&film, Px::new(0.0, 0.0), Px::new(0.0, 100.0), 15.0)
            .unwrap()
            .weak
    );
    assert_eq!(
        from_ruler(&film, Px::new(f64::NAN, 0.0), Px::new(0.0, 500.0), 100.0)
            .unwrap_err()
            .code,
        ErrorCode::ToolClickOutsideFilm
    );
}

#[test]
fn independent_rulers_combine_and_conflicting_ones_are_refused() {
    let film = blank(1200, 200);
    let ruler = Ruler {
        a: Px::new(0.0, 20.0),
        b: Px::new(1000.0, 20.0),
        mm: 150.0,
    };
    let combined = combine_rulers(
        &film,
        &[
            ruler,
            Ruler {
                a: Px::new(0.0, 100.0),
                b: Px::new(1000.0, 100.0),
                ..ruler
            },
        ],
    )
    .unwrap();
    assert!((combined.mm_per_px - 0.15).abs() < 1e-12);
    // Two equal independent rulers halve the variance, giving 1/1000 relative SD.
    assert!((combined.relative_uncertainty.unwrap() - 0.001).abs() < 1e-12);
    assert_eq!(
        combine_rulers(&film, &[ruler, Ruler { mm: 152.0, ..ruler }])
            .unwrap_err()
            .code,
        ErrorCode::ScaleDisagree
    );
}

fn disc(seed: u32, covered: bool) -> DecodedFilm {
    let (size, cx, cy, radius) = (240u32, 119.3, 121.7, 250.0 / 3.0);
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut inside = 0;
            // Area coverage by an independently drawn disc, before detector blur.
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = f64::from(x) - cx + (f64::from(sx) + 0.5) / 4.0 - 0.5;
                    let dy = f64::from(y) - cy + (f64::from(sy) + 0.5) / 4.0 - 0.5;
                    inside += usize::from(dx * dx + dy * dy <= radius * radius);
                }
            }
            pixels.push(10000.0 + 20000.0 * inside as f32 / 16.0);
        }
    }
    let image = GreyImage::new(size, size, pixels, Placement::FILM).unwrap();
    let blurred = darkroom::blur::gaussian(&image, 1.5).unwrap();
    let mut state = u64::from(seed) + 1;
    let mut uniform = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 32) as f64 + 0.5) / 4294967296.0
    };
    let pixels = blurred
        .pixels()
        .iter()
        .enumerate()
        .map(|(i, pixel)| {
            // Box-Muller noise: contrast / noise SD = 20.
            let noise = (-2.0 * libm::log(uniform())).sqrt()
                * libm::cos(std::f64::consts::TAU * uniform())
                * 1000.0;
            let mut value = f64::from(*pixel) + noise;
            // A vertical bright rectangle hides 35% of the circumference:
            // acos((edge-cx)/radius) / pi = 0.35.
            if covered
                && (i % size as usize) as f64 > cx + radius * libm::cos(0.35 * std::f64::consts::PI)
            {
                value = 50000.0 + noise;
            }
            value.clamp(0.0, 65535.0).round() as u16
        })
        .collect();
    film(GreyImage::new(size, size, pixels, Placement::FILM).unwrap())
}

#[test]
fn a_noisy_disc_and_a_partly_hidden_rim_meet_the_spec_over_200_seeds() {
    let centre_guess = Px::new(124.3, 121.7);
    let edge_guess = Px::new(199.3, 121.7);
    let mut clean_passes = 0;
    let mut covered_passes = 0;
    for seed in 0..200 {
        for covered in [false, true] {
            let film = disc(seed, covered);
            let Ok(result) = from_known_object(&film, centre_guess, edge_guess, 25.0) else {
                continue;
            };
            let ScaleSource::KnownObject { radius_px, .. } = result.source else {
                panic!()
            };
            let error = (result.mm_per_px / 0.15 - 1.0).abs();
            if covered {
                covered_passes += usize::from(error <= 0.006);
            } else {
                clean_passes += usize::from(
                    error <= 0.004 && (radius_px - 250.0 / 3.0).abs() <= 0.3 && !result.weak,
                );
            }
        }
    }
    assert!(clean_passes >= 196, "clean passes: {clean_passes}/200");
    assert!(
        covered_passes >= 190,
        "covered passes: {covered_passes}/200"
    );
}

#[test]
fn a_blank_crop_and_invalid_object_clicks_are_refused() {
    let film = blank(240, 240);
    let (centre, edge) = (Px::new(100.0, 100.0), Px::new(180.0, 100.0));
    assert_eq!(
        from_known_object(&film, centre, edge, 25.0)
            .unwrap_err()
            .code,
        ErrorCode::ScaleFitFailed
    );
    assert_eq!(
        from_known_object(&film, centre, edge, 101.0)
            .unwrap_err()
            .code,
        ErrorCode::ScaleBadLength
    );
    assert_eq!(
        from_known_object(&film, centre, Px::new(-1.0, 0.0), 25.0)
            .unwrap_err()
            .code,
        ErrorCode::ToolClickOutsideFilm
    );
    assert_eq!(
        from_known_object(&film, centre, centre, 25.0)
            .unwrap_err()
            .code,
        ErrorCode::ScaleFitFailed
    );
}

#[test]
fn a_shuttered_rim_cannot_calibrate_the_film() {
    let mut film = disc(8, false);
    film.facts.kept_tags = Some(KeptTags {
        shutters: Some(vec![shapes::film::Shutter::Circle {
            centre_column: 120.3,
            centre_row: 122.7,
            radius: 50.0,
        }]),
        ..tags()
    });
    let result = from_known_object(&film, Px::new(119.3, 121.7), Px::new(194.3, 121.7), 25.0);
    assert_eq!(result.unwrap_err().code, ErrorCode::ScaleFitFailed);
}

#[test]
fn a_round_object_in_anisotropic_pixels_reports_horizontal_radius() {
    // A 24 mm disc at 0.24 mm across and 0.15 mm down is an ellipse:
    // horizontal radius 50 pixels, vertical radius 80 pixels.
    let pixels = (0..240u32 * 240)
        .map(|index| {
            let x = (f64::from(index % 240) - 120.0) / 50.0;
            let y = (f64::from(index / 240) - 120.0) / 80.0;
            if x * x + y * y <= 1.0 { 50000 } else { 10000 }
        })
        .collect();
    let mut film = film(GreyImage::new(240, 240, pixels, Placement::FILM).unwrap());
    film.facts.kept_tags = Some(KeptTags {
        pixel_spacing: Some([0.15, 0.24]),
        ..tags()
    });
    let result =
        from_known_object(&film, Px::new(120.0, 120.0), Px::new(170.0, 120.0), 24.0).unwrap();
    // At this smaller radius the binary pixel boundary is uncertain by
    // half a pixel; allow 1%, without changing the 83.333 px conformance test.
    assert!((result.mm_per_px / 0.24 - 1.0).abs() <= 0.01);
    assert!((result.mm_per_px_vertical.unwrap() / 0.15 - 1.0).abs() <= 0.01);
    let ScaleSource::KnownObject {
        centre, radius_px, ..
    } = result.source
    else {
        panic!()
    };
    assert!((radius_px - 50.0).abs() <= 0.5);
    assert!(libm::hypot(centre.x() - 120.0, centre.y() - 120.0) <= 0.5);
}

#[test]
fn made_up_rulers_recover_the_answer_keys_scale() {
    let recipe = MadeUpRecipe {
        name: RecipeName::Straight,
        seed: 5,
        tier: Tier::T3,
    };
    let made = made_up_spines::make_up_case(&recipe).unwrap();
    let mut rulers = 0;
    for (painted, layout) in made.films.iter().zip(&made.layouts) {
        let film = film(painted.image.clone());
        let expected = layout.camera.upright_mm_per_px[0];
        if let Some(ruler) = painted.ruler {
            let result = from_ruler(&film, ruler.first, ruler.last, ruler.mm).unwrap();
            // Endpoints come from the independent painter, so only floating
            // point rounding separates the result from the camera's spacing.
            assert!((result.mm_per_px - expected).abs() < 1e-12);
            rulers += 1;
        }
    }
    assert!(rulers > 0, "the recipe must exercise a ruler");
}
