//! The shared data shapes: every value that crosses a boundary between the
//! engine crates, the browser and the case file on disk.
//!
//! These Rust types are the single source of truth. `cargo chores shapes`
//! writes the TypeScript types (feature `typescript`), the JSON Schema of
//! the case file and the fingerprint from them; nothing on the TypeScript
//! side declares a shared shape by hand.
//!
//! # Rules every shape follows
//!
//! - Field names are camelCase in JSON and TypeScript; enum values are
//!   kebab-case (level names such as `T1` and the mark names such as
//!   `supL` are the two deliberate exceptions).
//! - Unknown fields are refused, and every field is written out, `null`
//!   included, so a value has exactly one spelling.
//! - Only `f64`, `f32`, `i32`, `u32`, `u16`, `u8`, `bool`, `String`, `Vec`,
//!   `Option`, fixed arrays and internally tagged enums are used: no `u64`,
//!   no maps with non-string keys. The JavaScript object, the TypeScript
//!   type and `case.json` are then the same shape.
//! - Every point is in original film pixels (see [`pixels`]). Millimetres
//!   exist only through [`film_geometry::FilmGeometry`].
//!
//! # Map, in the order a film travels
//!
//! | Module | Holds |
//! |---|---|
//! | [`pixels`] | `Px`, `PxRect` and the pixel-space rule |
//! | [`film`] | film kinds, the stored image record and its PNG, kept tags, pixel values, facts read from a file |
//! | [`film_geometry`] | scale, upright, which way the image faces, confirmations, guesses |
//! | [`levels`] | `Level` (C1–C7, T1–T13, L1–L7) and its order |
//! | [`marks`] | `Mark`, pedicle marks, front and side marks, endplate lines and their ends, mark names, the quad order |
//! | [`vertebra`] | `VertebraOnFilm`, `FilmMarks` |
//! | [`landmarks`] | C7, the sacrum top, hip centres |
//! | [`confidence`] | confidence per vertebra, its band and reasons, band thresholds |
//! | [`naming`] | templates with their levels and names, level naming and its flags |
//! | [`auto_pass`] | the auto pass inputs, results, steps and stages |
//! | [`watch_frames`] | the frames the sweep animation plays |
//! | [`toolbox`] | wand and pin inputs, outputs, previews and warnings |
//! | [`checks`] | case check findings |
//! | [`made_up`] | made-up spine recipes, made-up cases, answer keys and comparisons |
//! | [`engine_info`] | `EngineInfo` |
//! | [`errors`] | `EngineError` and every `ErrorCode` |
//! | [`case_file_v1`] | `CaseFileV1`, the documented data file, and a case's contents with its PNGs |
//! | [`reading`] | strict reading and canonical writing of `case.json` |
//! | [`spine_readings`] | fitted angles, curves and version bands |

#![forbid(unsafe_code)]

pub mod auto_pass;
pub mod case_file_v1;
pub mod checks;
pub mod confidence;
pub mod engine_info;
pub mod errors;
pub mod film;
pub mod film_geometry;
pub mod landmarks;
pub mod levels;
pub mod made_up;
pub mod marks;
pub mod naming;
pub mod pixels;
pub mod reading;
pub mod spine_fit;
pub mod spine_readings;
pub mod toolbox;
pub mod vertebra;
pub mod watch_frames;

/// The value of `format` in every case file.
pub const CASE_FORMAT: &str = "spine-case";

/// The case file version this build writes, and the newest it reads.
pub const CASE_VERSION: u32 = 1;

/// SHA-256 of this crate's source files, computed by `build.rs`. The wasm
/// build and the generated TypeScript must carry the same value.
pub const SHAPES_FINGERPRINT: &str = env!("SHAPES_FINGERPRINT");
