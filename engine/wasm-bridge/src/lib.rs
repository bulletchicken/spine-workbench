//! The only crate that knows about the browser. Every export is thin: it
//! converts shapes, calls one engine function and converts back. One
//! `.wasm` file holds them all, and every worker runs the same module.
//!
//! | Module | Holds |
//! |---|---|
//! | [`film_desk`] | `FilmDesk`: one decoded film and its cached results |
//! | [`crossing`] | how values cross into JavaScript |
//! | [`faults`] | errors and panics as JavaScript values |
//! | [`exports`] | one file per engine crate |

pub mod crossing;
pub mod exports;
pub mod faults;
pub mod film_desk;
