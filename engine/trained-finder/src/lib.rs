//! A small trained network that finds the vertebrae on a front film.
//!
//! The film is shrunk to the size the network was taught on and read
//! once; a full-length film is then cropped to its spine and read again.
//! The network's maps become candidate outlines, a chain picks the
//! seventeen from T1 to L5 in order, and a second, smaller network settles
//! each corner on the full-size film. Every result leaves the crate in
//! original film pixels, as the same marks the auto pass gives.
//!
//! Nothing here is learned at run time: the numbers in `weights/` were
//! trained elsewhere and are compiled in. `README.md` says where they came
//! from, how well they do, and where they fail.
//!
//! | Module | Holds |
//! |---|---|
//! | [`outline`] | the plain shapes the stages hand on: a peak, a reading, a chain link |
//! | [`grid`] | a stack of float maps: channels by rows by columns |
//! | [`layers`] | convolution, upsampling, pooling, group norm, activations |
//! | [`weights`] | the trained numbers, compiled in and read strictly |
//! | [`spine_net`] | the main network, run in small steps |
//! | [`reading`] | the network's maps read as peaks and outlines |
//! | [`overlap`] | how much two outlines overlap |
//! | [`corners`] | each corner settled on the full-size film |

#![forbid(unsafe_code)]

pub mod corners;
pub mod grid;
pub mod layers;
pub mod outline;
pub mod overlap;
pub mod reading;
pub mod spine_net;
pub mod weights;
