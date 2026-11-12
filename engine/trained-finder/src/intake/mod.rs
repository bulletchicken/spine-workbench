//! A film as the network was taught to see one.
//!
//! [`as_shown`] reads the film through its record into an upright 8-bit
//! picture and maps positions on it back to the film; [`shrink`] brings a
//! picture to the network's 384 by 768.

pub mod as_shown;
pub mod shrink;
