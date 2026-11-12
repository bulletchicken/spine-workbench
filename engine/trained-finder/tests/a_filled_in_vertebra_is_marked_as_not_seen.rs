//! A vertebra the chain filled in between two neighbours was never seen by
//! the network: all four of its corners are inferred, its score is 0, its
//! reasons say it was placed from the chain alone, and the marks still
//! pass the strict reader. The cases are the recorded chain's picks with
//! one peak taken away (`known-answers/chain-cases.json`).

mod geometry_fixtures;

use darkroom::grey_image::Placement;
use geometry_fixtures::{chain_cases, peaks};
use shapes::case_file_v1::CaseFileV1;
use shapes::confidence::{ConfidenceBand, ConfidenceReason};
use shapes::film::{Film, FilmImage, FilmKind, PixelValues, SourceFormat};
use shapes::film_geometry::{FilmGeometry, ImageRight};
use trained_finder::chain::select;
use trained_finder::to_marks::film_marks;
use trained_finder::watch::FoundVertebra;

/// A front film record of `width` by `height`, stored in 16 bits.
fn record(width: u32, height: u32) -> Film {
    Film {
        kind: FilmKind::Front,
        image: FilmImage {
            path: FilmKind::Front.png_path().to_owned(),
            width,
            height,
            bits_stored: 16,
        },
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues::AS_STORED,
        flags: Vec::new(),
        blanked: Vec::new(),
        geometry: FilmGeometry::UNKNOWN,
    }
}

#[test]
fn a_filled_in_vertebra_is_inferred_scored_zero_and_says_why() {
    let (width, height) = (1409, 3128);
    let cases = chain_cases();
    let mut checked = 0;
    for case in cases.as_array().expect("a list") {
        let chain = select(&peaks(&case["peaks"])).expect("the chain runs");
        if !chain.iter().any(|link| link.filled_in) {
            continue;
        }
        let found: Vec<FoundVertebra> = chain
            .iter()
            .map(|link| FoundVertebra {
                level_index: link.level_index,
                filled_in: link.filled_in,
                score: link.score,
                corners: link
                    .corners
                    .map(|[x, y]| [x * f64::from(width), y * f64::from(height)]),
            })
            .collect();
        let marks = film_marks(&found, Placement::FILM, ImageRight::PatientLeft, 1);
        assert_eq!(marks.vertebrae.len(), 17, "{}", case["name"]);
        for (vertebra, link) in marks.vertebrae.iter().zip(&chain) {
            let corners = vertebra.marks.corners();
            let confidence = &vertebra.confidence;
            if link.filled_in {
                assert!(corners.iter().all(|mark| mark.inferred));
                assert_eq!(confidence.score, 0.0);
                assert_eq!(confidence.band, ConfidenceBand::Low);
                assert_eq!(
                    confidence.reasons,
                    [
                        ConfidenceReason::NoEvidence,
                        ConfidenceReason::LevelUncertain
                    ]
                );
            } else {
                assert!(corners.iter().all(|mark| !mark.inferred));
                assert_eq!(confidence.reasons, [ConfidenceReason::LevelUncertain]);
            }
        }
        let mut case_file = CaseFileV1::new("0.1.0");
        case_file.films = vec![record(width, height)];
        case_file.marks = vec![marks];
        case_file.next_vertebra_id = 18;
        shapes::reading::check_case_file(&case_file).expect("the strict reader takes the marks");
        checked += 1;
    }
    assert_eq!(
        checked, 3,
        "three cases each lose one peak, which is filled"
    );
}
