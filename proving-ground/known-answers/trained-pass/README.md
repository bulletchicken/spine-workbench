# The trained finder's recorded answer

`front.json` is the native engine's answer for the made-up front film
`straight`, seed 7, tier t2: the film the trained finder's own known
answers were recorded on (`engine/trained-finder/tests/known-answers/`).
It holds the recipe, the first vertebra id (1000), the film's record as
`spine pass` makes it for a film it opens (nothing confirmed, nothing
blanked), and the `TrainedPassResult` the trained pass gave.

Two tests hold the two builds to it:

- `engine/trained-finder/tests/finds_the_vertebrae_on_a_made_up_film.rs`
  (`the_product_path_gives_the_recorded_answer`) runs the finder natively
  on the film's 16-bit PNG and needs its answer, written as pretty JSON,
  to equal this file character for character;
- `proving-ground/parity/trained-pass.test.ts` runs the release browser
  build in Node, through `FilmDesk.open`, `start_trained_pass`, `advance`
  and `finish`, and needs exactly the same answer.

So the browser build and the native build give the same bits. These are
observations of the finder on a made-up film, not an accuracy target: the
network was taught on real films and never saw a made-up one.

When the finder changes on purpose, run
`cargo test -p trained-finder --test finds_the_vertebrae_on_a_made_up_film -- --nocapture the_product_path_gives_the_recorded_answer`:
the native test prints the new answer when it differs, and that text
becomes this file.
