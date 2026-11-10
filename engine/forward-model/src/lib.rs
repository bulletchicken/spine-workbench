//! The one model of a vertebra and how it lands on a film. The auto pass,
//! the made-up spines and the Wave 2 fit all use it, so points on a film
//! and a pose in 3D agree by construction.
//!
//! | Module | Holds |
//! |---|---|
//! | [`patient_frame`] | millimetres; x forward, y to the patient's left, z up; origin under L5 |
//! | [`vertebra_params`] | the thirteen numbers of a vertebra and their signs |
//! | [`rotation`] | rotation vector to matrix and back |
//! | [`endplates`] | endplate centres, half-wedge tilts, rim ovals |
//! | [`corners`] | the closed-form corner |
//! | [`pedicles`] | pedicle starting positions |
//! | [`cameras`] | patient millimetres to film pixels |
//! | [`project`] | a vertebra and a camera to front marks, side marks and endplate lines |

#![forbid(unsafe_code)]

pub mod cameras;
pub mod corners;
pub mod endplates;
pub mod patient_frame;
pub mod pedicles;
pub mod project;
pub mod rotation;
pub mod vertebra_params;
