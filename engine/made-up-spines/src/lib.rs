//! Spines and films with known answers, for tests, scoring and "Try a
//! made-up spine" in the app: a recipe and a seed give four films and an
//! answer key, the same natively and in the browser.
//!
//! The films are painted with their own body shape and a second curve
//! generator, so tests built on them cannot simply agree with the solver.
//! Accuracy on made-up films is never clinical accuracy.
//!
//! | Module | Holds |
//! |---|---|
//! | [`recipes`] | the named recipes and the ranges drawn from |
//! | [`seeded`] | the seeded random streams every draw comes from |
//! | [`curves`] | two independent curve generators |
//! | [`stack_from_l5`] | stacking vertebrae upward from L5 |
//! | [`film_painter`] | camera, spine, surroundings and detector |
//! | [`click_noise`] | noise on true marks, standing in for clicks |
//! | [`answer_key`] | the true marks, levels, geometry, landmarks and angles |

#![forbid(unsafe_code)]

pub mod answer_key;
pub mod click_noise;
pub mod curves;
pub mod film_painter;
pub mod recipes;
pub mod seeded;
pub mod stack_from_l5;
