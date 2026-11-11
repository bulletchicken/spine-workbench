//! Analytic checks of mapping, visibility and the film-worker entry point.

use super::*;
use crate::handoffs::*;
use darkroom::grey_image::{GreyImage, Placement};
use shapes::confidence::{Confidence, ConfidenceBand, EndplateSpread};
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::marks::{MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, Template};
use shapes::pixels::Px;

fn rectangle() -> (Strip, FoundBody) {
    // Independent analytic rectangle: walls at ±14 mm, plates at 8 and
    // 32 mm, blurred by a 0.4 mm Gaussian. The width prior is 12% too big.
    let step = |x: f64| 0.5 * (1.0 + libm::erf(x / (0.4 * libm::sqrt(2.0))));
    let mut values = Vec::new();
    for row in 0..101 {
        for col in 0..101 {
            let x = f64::from(col - 50) * 0.5;
            let y = f64::from(row) * 0.5;
            values.push((step(x + 14.0) * step(14.0 - x) * step(y - 8.0) * step(32.0 - y)) as f32);
        }
    }
    let strip = Strip {
        film: FilmKind::Front,
        rows: 101,
        cols: 101,
        row_step: 0.5,
        col_step: 0.5,
        scale_assumed: false,
        values,
        valid: vec![1; 101 * 101],
        layout: (0..101)
            .map(|row| StripRow {
                centre: Px::new(100.0, 20.0 + f64::from(row)),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                half_width: 14.0 * 1.12,
                z: -f64::from(row) * 0.5,
            })
            .collect(),
        image_right: Some(ImageRight::PatientLeft),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    };
    let body = FoundBody {
        level: Level::C7,
        sup: EndplateOnStrip {
            row: 16.0,
            tilt: 0.0,
        },
        inf: EndplateOnStrip {
            row: 64.0,
            tilt: 0.0,
        },
        inferred: false,
        certainty: BodyCertainty {
            posterior: 1.0,
            sup_spread: 0.0,
            inf_spread: 0.0,
            margin: 10.0,
        },
    };
    (strip, body)
}

#[test]
fn linear_strip_mapping_keeps_fractional_pixel_centres() {
    let (mut strip, _) = rectangle();
    strip.layout[0].centre = Px::new(10.0, 20.0);
    strip.layout[0].across = Px::new(2.0, 1.0);
    strip.layout[1].centre = Px::new(14.0, 25.0);
    strip.layout[1].across = Px::new(4.0, 3.0);
    // At 1/4 row: C=(11,21.25), across=(2.5,1.5); two columns add (5,3).
    assert_eq!(
        back_to_film::strip_to_film(&strip, 0.25, 52.0),
        Some(Px::new(16.0, 24.25))
    );
    assert_eq!(back_to_film::strip_to_film(&strip, -0.1, 50.0), None);
    assert_eq!(back_to_film::strip_to_film(&strip, 0.0, f64::NAN), None);
    strip.layout[0].curvature = 1.0;
    assert_eq!(back_to_film::strip_to_film(&strip, 0.0, 52.0), None);
}

#[test]
fn walls_correct_the_width_prior_and_refine_subpixel_endplates() {
    let (strip, mut body) = rectangle();
    body.sup.row += 0.6;
    body.inf.row -= 0.6;
    let points = front::place(&strip, &body, Polarity::BoneBright).unwrap();
    let expected = [(128.0, 36.0), (72.0, 36.0), (72.0, 84.0), (128.0, 84.0)];
    for (mark, (x, y)) in points.corners().into_iter().zip(expected) {
        // Half a film pixel = 0.25 mm, below the strip's 0.5 mm spacing.
        assert!(
            libm::hypot(mark.at.x() - x, mark.at.y() - y) < 0.5,
            "{mark:?}"
        );
        assert!(!mark.inferred);
    }
}

#[test]
fn weak_masked_and_inferred_rings_never_receive_positions() {
    let (mut strip, mut body) = rectangle();
    let absent = |strip: &Strip, body: &FoundBody| {
        let marks = front::place(strip, body, Polarity::BoneBright).unwrap();
        assert_eq!(marks.ped_l.mark(), None);
        assert_eq!(marks.ped_r.mark(), None);
    };
    let mut ring = vec![0.0; strip.values.len()];
    // This is an enormous robust z score on an otherwise zero map, but
    // below the independent absolute visibility threshold.
    ring[31 * 101 + 70] = 0.19;
    strip.evidence.ring = Some(ring);
    absent(&strip, &body);
    strip.evidence.ring.as_mut().unwrap()[31 * 101 + 70] = 1.0;
    strip.valid[31 * 101 + 70] = 0;
    absent(&strip, &body);
    strip.valid.fill(1);
    body.inferred = true;
    absent(&strip, &body);
}

#[test]
fn missing_image_support_and_cropped_rows_stay_inferred() {
    let (mut strip, mut body) = rectangle();
    strip.valid.fill(0);
    let marks = front::place(&strip, &body, Polarity::BoneBright).unwrap();
    assert!(marks.corners().iter().all(|mark| mark.inferred));
    body.sup.row = -10.0;
    body.inf.row = -2.0;
    body.inferred = true;
    let marks = front::place(&strip, &body, Polarity::BoneBright).unwrap();
    // The frozen along vector is one film pixel per strip row. Extending
    // from C(0)=(100,20) puts these plates at y=10 and y=18 exactly.
    assert_eq!(marks.sup_l.at.y(), 10.0);
    assert_eq!(marks.inf_l.at.y(), 18.0);
    assert!(marks.corners().iter().all(|mark| mark.inferred));
    assert!(matches!(
        marks.ped_l,
        PedicleMark::NotVisible {
            source: MarkSource::Auto
        }
    ));
}

fn cache(strip: Strip) -> FilmPassCache {
    let image = GreyImage::new(2, 2, vec![0.0f32; 4], Placement::FILM).unwrap();
    let bytes = image.map(|_| 0u8);
    FilmPassCache {
        evidence: FilmEvidence {
            film: strip.film,
            mm_per_px: 0.5,
            scale_assumed: false,
            valid: bytes.clone(),
            image: image.clone(),
            phase_edge: image.clone(),
            phase_direction: image.clone(),
            phase_kind: image.clone(),
            gradient_edge: image.clone(),
            gradient_direction: image.clone(),
            ridge: image.clone(),
            ridge_orientation: image.clone(),
            valley: image.clone(),
            valley_orientation: image.clone(),
            valley_scale: bytes,
            ridge_symmetry: image.clone(),
            valley_symmetry: image,
            rings: None,
            hessian_scales_mm: vec![],
            polarity: PolarityCheck {
                found: Polarity::BoneBright,
                confidence: 1.0,
                needs_confirm: false,
            },
            markers: vec![],
            seam_rows: vec![],
            valid_fraction: 0.0,
            coverage: 0.0,
            warnings: vec![],
        },
        centre_line: CentreLine {
            film: strip.film,
            quarter_turns: QuarterTurns::new(0).unwrap(),
            scale_assumed: false,
            sample_step: 0.5,
            samples: vec![],
            ends: CentreLineEnds {
                neck_base: None,
                iliac_crest: None,
                top_cropped: false,
                bottom_cropped: false,
                size_factor: 1.0,
            },
            warnings: vec![],
        },
        barcode: Barcode {
            film: strip.film,
            values: vec![],
            high_pass: vec![],
            valid: vec![],
        },
        strip,
    }
}

fn naming() -> LevelNaming {
    LevelNaming {
        template: Template::Standard,
        chosen_by: ChosenBy::Engine,
        alternatives: vec![],
        from_top: None,
        from_bottom: None,
        flags: vec![],
        l5_confirmed_on: None,
    }
}

#[test]
fn placing_a_chain_preserves_confidence_and_allocates_ids() {
    let (mut strip, body) = rectangle();
    strip.image_right = None;
    let cache = cache(strip);
    let chain = SettledChain {
        boundaries: ChainBoundaries {
            film: FilmKind::Front,
            template: Template::Standard,
            polarity: Polarity::BoneBright,
            bodies: vec![body],
            sacrum: None,
            energy: 0.0,
            warnings: vec![],
        },
        confidence: vec![Confidence {
            score: 0.7,
            band: ConfidenceBand::Medium,
            reasons: vec![],
            calibrated: false,
            endplate_spread_px: Some(EndplateSpread { sup: 0.5, inf: 0.5 }),
            margin: Some(2.0),
        }],
    };
    let marks = place_points(&cache, &chain, naming(), 42).unwrap();
    assert_eq!(marks.vertebrae[0].id, 42);
    assert_eq!(marks.vertebrae[0].confidence, chain.confidence[0]);
    assert_eq!(marks.named_for, ImageRight::PatientLeft);
    assert_eq!(
        marks.landmarks.c7,
        shapes::landmarks::Landmarks::c7_of(&marks.vertebrae)
    );
    assert!(marks.landmarks.hips.is_empty());
    assert!(place_points(&cache, &chain, naming(), u32::MAX).is_err());
    let mut bad = chain.clone();
    bad.confidence.clear();
    assert!(place_points(&cache, &bad, naming(), 42).is_err());
    bad = chain;
    bad.boundaries.film = FilmKind::Side;
    assert!(place_points(&cache, &bad, naming(), 42).is_err());
}

#[test]
fn malformed_strip_geometry_is_refused_without_sampling() {
    let (strip, body) = rectangle();
    for bad in [
        Strip {
            row_step: 0.0,
            ..strip.clone()
        },
        Strip {
            col_step: f64::NAN,
            ..strip.clone()
        },
        Strip {
            cols: 100,
            ..strip.clone()
        },
        Strip {
            values: vec![],
            ..strip.clone()
        },
        Strip {
            image_right: Some(ImageRight::Anterior),
            ..strip
        },
    ] {
        assert!(front::place(&bad, &body, Polarity::BoneBright).is_err());
    }
}

#[test]
fn extreme_finite_scales_keep_searches_bounded() {
    let (mut strip, body) = rectangle();
    strip.row_step = 1e-300;
    let result = front::place(&strip, &body, Polarity::BoneBright);
    assert!(result.is_err() || result.unwrap().corners().iter().all(|mark| mark.inferred));
    strip.row_step = 0.5;
    for row in &mut strip.layout {
        row.half_width = 1e300;
    }
    let marks = front::place(&strip, &body, Polarity::BoneBright).unwrap();
    assert!(marks.corners().iter().all(|mark| mark.at.is_finite()));
}
