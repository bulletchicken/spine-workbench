//! C7: the mean of the C7 vertebra's four corners, as
//! `shapes::landmarks::Landmarks::c7_of` defines it.

use shapes::landmarks::Landmarks;
use shapes::marks::Mark;
use shapes::vertebra::VertebraOnFilm;

/// Recompute the C7 centre from the current chain and its current marks.
pub fn from_vertebrae(vertebrae: &[VertebraOnFilm]) -> Option<Mark> {
    Landmarks::c7_of(vertebrae)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::confidence::{Confidence, ConfidenceBand};
    use shapes::levels::Level;
    use shapes::marks::{EndplateLines, MarkSource};
    use shapes::pixels::Px;
    use shapes::vertebra::VertebraMarks;

    #[test]
    fn the_centre_follows_the_current_corners_and_level() {
        let mut last = Mark::auto(Px::new(8.0, 12.0));
        last.inferred = true;
        let mut vertebra = VertebraOnFilm {
            id: 7,
            level: Level::C7,
            marks: VertebraMarks::Bending {
                lines: EndplateLines {
                    sup: [Mark::auto(Px::new(2.0, 4.0)), Mark::auto(Px::new(8.0, 4.0))],
                    inf: [Mark::auto(Px::new(2.0, 12.0)), last],
                },
            },
            confidence: Confidence {
                score: 1.0,
                band: ConfidenceBand::High,
                reasons: Vec::new(),
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        };
        // A 6 by 8 rectangle has centre (5, 8), exactly in binary.
        let centre = from_vertebrae(&[vertebra.clone()]).expect("C7 present");
        assert_eq!(centre.at, Px::new(5.0, 8.0));
        assert!(centre.inferred);
        assert!(!centre.pinned);
        assert_eq!(centre.source, MarkSource::Auto);
        vertebra.level = Level::T1;
        assert_eq!(from_vertebrae(&[vertebra]), None);
        assert_eq!(from_vertebrae(&[]), None);
    }
}
