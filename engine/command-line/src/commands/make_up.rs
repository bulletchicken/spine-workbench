//! `spine make-up <recipe> --seed N [--tier t0..t4] [--format png|dicom]
//! [--count N] [--out DIR]`: four films and their answer key per case,
//! written to a folder. The same seed gives the same films as "Try a
//! made-up spine" in the app. DICOM films are written through
//! `film_reader::test_support`.

use crate::{MakeUpArgs, Outcome, not_built};

pub fn run(_args: &MakeUpArgs) -> Outcome {
    not_built("make-up")
}
