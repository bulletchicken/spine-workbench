//! Marker positions relative to the body, never a left/right decision.

use crate::rough_trace::Profile;
use crate::upright::file_image_right;
use film_reader::decoded_film::DecodedFilm;
use shapes::film::IntakeFlag;
use shapes::film_geometry::{
    Certainty, FacingCue, FacingEvidence, ImageRight, ImageRightGuess, LeadMarker, MarkerLetter,
    MarkerMaterial, QuarterTurns,
};
use shapes::pixels::Px;

pub(crate) fn guess(
    film: &DecodedFilm,
    profile: &Profile,
    turns: QuarterTurns,
    markers: &[LeadMarker],
) -> ImageRightGuess {
    let file_says = file_image_right(film, turns);
    let mut candidates = Vec::new();
    if let Some((midline, body_width)) = profile.midline() {
        for marker in markers {
            if marker.material != MarkerMaterial::Lead {
                continue;
            }
            let centre = Px::new(
                (marker.bounds.left + marker.bounds.right) * 0.5,
                (marker.bounds.top + marker.bounds.bottom) * 0.5,
            );
            let Some((x, _)) = profile.placement.image_point(centre) else {
                continue;
            };
            if (x - midline).abs() < 0.15 * body_width {
                continue;
            }
            let right = match (marker.letter, x > midline) {
                (MarkerLetter::L, true) | (MarkerLetter::R, false) => ImageRight::PatientLeft,
                _ => ImageRight::PatientRight,
            };
            candidates.push((right, x > midline, marker.score));
        }
    }
    let image_right = candidates
        .first()
        .map(|c| c.0)
        .filter(|right| candidates.iter().all(|c| c.0 == *right));
    let both_sides = candidates.iter().any(|c| c.1) && candidates.iter().any(|c| !c.1);
    let agreement = file_says
        .zip(image_right)
        .map(|(file, proposal)| if file == proposal { 1.0 } else { -1.0 });
    let best = candidates.iter().map(|c| c.2).fold(0.0, f64::max);
    let certainty = match image_right {
        None if markers.is_empty() => Certainty::None,
        None => Certainty::Low,
        Some(_) if both_sides || (best >= 0.75 && agreement == Some(1.0)) => Certainty::High,
        Some(_) => Certainty::Medium,
    };
    ImageRightGuess {
        image_right,
        certainty,
        file_says,
        evidence: vec![
            FacingEvidence {
                cue: FacingCue::FileOrientation,
                strength: agreement,
            },
            FacingEvidence {
                cue: FacingCue::LeadMarker,
                strength: image_right.map(|_| best),
            },
        ],
        looks_mirrored: markers.iter().any(|m| m.mirrored)
            || film.facts.flags.contains(&IntakeFlag::ExifMirrored),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rough_trace::RoughTrace;
    use darkroom::grey_image::{GreyImage, Placement};
    use shapes::film::{
        DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
    };
    use shapes::film_geometry::PolarityGuess;
    use shapes::pixels::PxRect;

    fn scene() -> (DecodedFilm, RoughTrace) {
        // A centred, straight silhouette, width 40, on a 100-pixel film.
        // Marker positions below are independent of this body mask.
        let image = GreyImage::new(
            100,
            200,
            (0..20_000)
                .map(|i| {
                    if (30..70).contains(&(i % 100)) {
                        50_000u16
                    } else {
                        1000
                    }
                })
                .collect(),
            Placement::FILM,
        )
        .unwrap();
        let trace = RoughTrace {
            image: image.map(|v| f32::from(*v) / 65_535.0),
            valid: vec![true; 20_000],
            collimation: Vec::new(),
            polarity: PolarityGuess::AsStored,
        };
        let film = DecodedFilm {
            image,
            facts: FilmFacts {
                kind: FilmKind::Front,
                width: 100,
                height: 200,
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
        };
        (film, trace)
    }

    fn marker(letter: MarkerLetter, x: f64) -> LeadMarker {
        LeadMarker {
            letter,
            bounds: PxRect {
                left: x - 3.0,
                top: 20.0,
                right: x + 3.0,
                bottom: 32.0,
            },
            score: 0.9,
            margin: 0.3,
            mirrored: false,
            rotation_deg: 0,
            cut_out: false,
            material: MarkerMaterial::Lead,
        }
    }

    #[test]
    fn position_supplies_laterality_and_mirror_supplies_its_own_evidence() {
        let (film, trace) = scene();
        let profile = trace.profile(QuarterTurns::NONE);
        let right = marker(MarkerLetter::L, 85.0);
        let guess = guess(&film, &profile, QuarterTurns::NONE, &[right]);
        assert_eq!(guess.image_right, Some(ImageRight::PatientLeft));
        assert_eq!(guess.certainty, Certainty::Medium);
        let left = LeadMarker {
            bounds: PxRect {
                left: 12.0,
                right: 18.0,
                ..right.bounds
            },
            mirrored: true,
            ..right
        };
        let flipped = super::guess(&film, &profile, QuarterTurns::NONE, &[left]);
        assert_eq!(flipped.image_right, Some(ImageRight::PatientRight));
        assert!(flipped.looks_mirrored);
        // The stored L is left of the body after a half turn; the glyph's
        // mirror flag is unchanged, independently of its upright position.
        let q = QuarterTurns::new(2).unwrap();
        let turned = super::guess(&film, &trace.profile(q), q, &[right]);
        assert_eq!(turned.image_right, Some(ImageRight::PatientRight));
        assert!(!turned.looks_mirrored);
    }

    #[test]
    fn missing_central_digital_and_contradictory_markers_never_choose_a_side() {
        let (film, trace) = scene();
        let profile = trace.profile(QuarterTurns::NONE);
        for markers in [
            Vec::new(),
            vec![marker(MarkerLetter::L, 50.0)],
            vec![LeadMarker {
                material: MarkerMaterial::Digital,
                ..marker(MarkerLetter::L, 85.0)
            }],
            vec![marker(MarkerLetter::L, 85.0), marker(MarkerLetter::R, 85.0)],
        ] {
            assert_eq!(
                guess(&film, &profile, QuarterTurns::NONE, &markers).image_right,
                None
            );
        }
        let agreement = guess(
            &film,
            &profile,
            QuarterTurns::NONE,
            &[marker(MarkerLetter::L, 85.0), marker(MarkerLetter::R, 15.0)],
        );
        assert_eq!(agreement.image_right, Some(ImageRight::PatientLeft));
        assert_eq!(agreement.certainty, Certainty::High);
    }
}
