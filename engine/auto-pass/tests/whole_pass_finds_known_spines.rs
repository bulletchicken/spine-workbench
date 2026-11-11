//! Run every kind of painted film without feeding the key to the finder.
//! Accuracy is reported rather than imposed as an orchestration gate.

#[path = "../src/watch/test_films.rs"]
mod test_films;

use auto_pass::run;
use shapes::film::FilmKind;

#[test]
fn the_whole_pass_runs_all_four_film_kinds_and_allocates_case_wide_ids() {
    let fixtures: Vec<_> = FilmKind::ALL.into_iter().map(test_films::film).collect();
    let input = test_films::input(fixtures.iter().map(|(_, film, _)| film.clone()).collect());
    let images: Vec<_> = fixtures.iter().map(|(image, _, _)| image.clone()).collect();
    let answer = run::auto_pass(&input, &images).expect("the painted films complete the pass");
    assert_eq!(answer.case_version, 42);
    assert_eq!(answer.marks.len(), 4);
    assert!(answer.two_films.is_some());
    let mut next_id = 1000;
    for (marks, (_, film, key)) in answer.marks.iter().zip(&fixtures) {
        assert_eq!(marks.film, film.kind);
        assert_eq!(Some(marks.named_for), film.geometry.image_right);
        assert!(
            marks
                .vertebrae
                .windows(2)
                .all(|pair| pair[0].level < pair[1].level)
        );
        // The straight recipe independently stacks C7, twelve thoracic
        // and five lumbar bodies. This is a count of truth, not a target
        // forced onto the finder, whose template option is unset.
        assert_eq!(key.vertebrae.len(), 18);
        println!(
            "{}: {} vertebrae placed; {} in the independent answer key",
            marks.film.name(),
            marks.vertebrae.len(),
            key.vertebrae.len()
        );
        for body in &marks.vertebrae {
            assert_eq!(body.id, next_id);
            next_id += 1;
            assert!(
                body.marks
                    .corners()
                    .iter()
                    .all(|mark| mark.at.x().is_finite() && mark.at.y().is_finite())
            );
        }
    }
}
