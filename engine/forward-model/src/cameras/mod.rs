//! Cameras: how a point in the patient frame lands on a film, in original film
//! pixels. Wave 1 has the parallel camera; the point-source and slot-scanner
//! cameras of Wave 2 sit beside it.
//!
//! The camera formulas are written for the film shown upright, in upright
//! pixels, whose width and height in mm swap with the stored spacings when
//! the film's quarter turns are odd. Each camera turns its answer back to
//! original film pixels before returning it: see `shapes::film_geometry`.

pub mod parallel;
