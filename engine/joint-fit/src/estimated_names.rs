//! Whether a film's level names are estimates rather than counted.
//!
//! A film's names are estimates when its naming carries the `estimated`
//! flag, which the trained finder sets: it places names from where each
//! vertebra sits and how big it is, not by counting from the first rib or
//! the sacrum. They stop being estimates once L5 is confirmed on a film
//! (`l5ConfirmedOn` is no longer `null`).

use shapes::naming::NamingFlag;
use shapes::vertebra::FilmMarks;

/// Whether the film's names are estimates, by the rule above.
pub fn names_are_estimates(marks: &FilmMarks) -> bool {
    let naming = &marks.naming;
    naming.flags.contains(&NamingFlag::Estimated) && naming.l5_confirmed_on.is_none()
}
