//! The answer as the marks the rest of the engine reads.
//!
//! The trained finder hands back the same `FilmMarks` the auto pass does,
//! so the case, its checks, the drawing and saving take them unchanged:
//! four body corners per vertebra in original film pixels, both pedicles
//! not visible (the network does not look for them), the network's own
//! score as the confidence, and nothing calibrated. A vertebra the chain
//! filled in was never seen, so all four of its corners are inferred, its
//! score is 0 and it says why. The level names are estimates from where
//! each vertebra sits and how big it is, and the naming says so.

use crate::intake::as_shown;
use crate::watch::FoundVertebra;
use darkroom::grey_image::Placement;
use shapes::confidence::{Confidence, ConfidenceBand, ConfidenceReason};
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::landmarks::Landmarks;
use shapes::marks::{FrontMarks, Mark, MarkSource, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template};
use shapes::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};

/// The front film's marks for `vertebrae`, found on a picture that sits on
/// the film at `placement`. `named_for` is the film's facing (patient left
/// while unknown); ids count up from `first_vertebra_id`, which the caller
/// has checked leaves room for every vertebra.
pub fn film_marks(
    vertebrae: &[FoundVertebra],
    placement: Placement,
    named_for: ImageRight,
    first_vertebra_id: u32,
) -> FilmMarks {
    // The reference orders by height and the marks must run in level order;
    // the two agree on every film seen, and level order is what the case
    // can hold should they ever not.
    let mut in_order: Vec<&FoundVertebra> = vertebrae.iter().collect();
    in_order.sort_by_key(|vertebra| vertebra.level_index);
    let vertebrae: Vec<VertebraOnFilm> = in_order
        .into_iter()
        .enumerate()
        .map(|(index, found)| VertebraOnFilm {
            id: first_vertebra_id + index as u32,
            level: found.level(),
            marks: VertebraMarks::Front {
                points: front_marks(found, placement, named_for),
            },
            confidence: confidence(found),
            outline: None,
        })
        .collect();
    FilmMarks {
        film: FilmKind::Front,
        landmarks: Landmarks {
            c7: Landmarks::c7_of(&vertebrae),
            sacrum_top: None,
            hips: Vec::new(),
        },
        vertebrae,
        naming: estimated_naming(),
        named_for,
    }
}

/// The naming every trained answer carries: the standard count, chosen by
/// the engine, with the names flagged as estimates.
pub fn estimated_naming() -> LevelNaming {
    LevelNaming {
        template: Template::Standard,
        chosen_by: ChosenBy::Engine,
        alternatives: Vec::new(),
        from_top: None,
        from_bottom: None,
        flags: vec![NamingFlag::Estimated],
        l5_confirmed_on: None,
    }
}

/// The four corners named for the patient. The picture's corners go round
/// upper left, upper right, lower right, lower left as the upright film
/// shows them; with patient left at screen right, the patient's left
/// corners are the picture's right ones, as the auto pass names them
/// (`engine/auto-pass/src/points/front.rs`).
fn front_marks(found: &FoundVertebra, placement: Placement, named_for: ImageRight) -> FrontMarks {
    let [upper_left, upper_right, lower_right, lower_left] = found.corners.map(|corner| Mark {
        at: as_shown::to_film_pixels(placement, corner),
        source: MarkSource::Auto,
        pinned: false,
        inferred: found.filled_in,
    });
    let not_seen = PedicleMark::NotVisible {
        source: MarkSource::Auto,
    };
    let ([sup_l, sup_r], [inf_l, inf_r]) = if named_for == ImageRight::PatientLeft {
        ([upper_right, upper_left], [lower_right, lower_left])
    } else {
        ([upper_left, upper_right], [lower_left, lower_right])
    };
    FrontMarks {
        sup_l,
        sup_r,
        inf_l,
        inf_r,
        ped_l: not_seen,
        ped_r: not_seen,
    }
}

/// The network's score, and why a person should look: a filled-in
/// vertebra was not seen, and every name is an estimate.
fn confidence(found: &FoundVertebra) -> Confidence {
    let mut reasons = Vec::new();
    if found.filled_in {
        reasons.push(ConfidenceReason::NoEvidence);
    }
    reasons.push(ConfidenceReason::LevelUncertain);
    Confidence {
        score: found.score,
        band: ConfidenceBand::for_score(found.score),
        reasons,
        calibrated: false,
        endplate_spread_px: None,
        margin: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::levels::Level;
    use shapes::pixels::Px;

    /// A vertebra 20 pixels wide and 10 high, its upper left corner at
    /// (`left`, `top`) on the picture, measured from the outer corner.
    fn found(level_index: usize, left: f64, top: f64, filled_in: bool) -> FoundVertebra {
        FoundVertebra {
            level_index,
            filled_in,
            score: if filled_in { 0.0 } else { 0.75 },
            corners: [
                [left, top],
                [left + 20.0, top],
                [left + 20.0, top + 10.0],
                [left, top + 10.0],
            ],
        }
    }

    fn corners(marks: &FilmMarks, index: usize) -> FrontMarks {
        match marks.vertebrae[index].marks {
            VertebraMarks::Front { points } => points,
            _ => panic!("front marks"),
        }
    }

    #[test]
    fn patient_left_at_screen_right_names_the_picture_right_corners_left() {
        let vertebrae = [found(0, 10.5, 20.5, false)];
        let left = film_marks(&vertebrae, Placement::FILM, ImageRight::PatientLeft, 7);
        let points = corners(&left, 0);
        // Measured from the outer corner, so half a pixel comes off.
        assert_eq!(points.sup_l.at, Px::new(30.0, 20.0));
        assert_eq!(points.sup_r.at, Px::new(10.0, 20.0));
        assert_eq!(points.inf_l.at, Px::new(30.0, 30.0));
        assert_eq!(points.inf_r.at, Px::new(10.0, 30.0));
        let right = film_marks(&vertebrae, Placement::FILM, ImageRight::PatientRight, 7);
        let points = corners(&right, 0);
        assert_eq!(points.sup_l.at, Px::new(10.0, 20.0));
        assert_eq!(points.inf_r.at, Px::new(30.0, 30.0));
        assert_eq!(right.named_for, ImageRight::PatientRight);
        // Either way the outline goes round the same four places.
        assert_eq!(
            left.vertebrae[0].marks.corners().map(|mark| mark.at)[0],
            Px::new(30.0, 20.0)
        );
    }

    #[test]
    fn marks_run_in_level_order_with_ids_from_the_first() {
        // Handed over out of order, they come back T1, T2, T3.
        let vertebrae = [
            found(1, 10.0, 40.0, false),
            found(0, 10.0, 20.0, false),
            found(2, 10.0, 60.0, true),
        ];
        let marks = film_marks(&vertebrae, Placement::FILM, ImageRight::PatientLeft, 100);
        let levels: Vec<Level> = marks.vertebrae.iter().map(|v| v.level).collect();
        assert_eq!(levels, [Level::T1, Level::T2, Level::T3]);
        let ids: Vec<u32> = marks.vertebrae.iter().map(|v| v.id).collect();
        assert_eq!(ids, [100, 101, 102]);
        assert_eq!(marks.landmarks.c7, None);
        assert_eq!(marks.naming.flags, [NamingFlag::Estimated]);
        let seen = &marks.vertebrae[0].confidence;
        assert_eq!(seen.reasons, [ConfidenceReason::LevelUncertain]);
        assert_eq!(seen.band, ConfidenceBand::Medium);
        assert!(!seen.calibrated);
        assert!(
            corners(&marks, 0)
                .corners()
                .iter()
                .all(|mark| !mark.inferred && mark.source == MarkSource::Auto)
        );
    }
}
