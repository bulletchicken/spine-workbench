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
//! | [`intake`] | a film as the network was taught to see one |
//! | [`spine_net`] | the main network, run in small steps |
//! | [`reading`] | the network's maps read as peaks and outlines |
//! | [`overlap`] | how much two outlines overlap |
//! | [`framing`] | a full-length film cropped to its spine for a second look |
//! | [`chain`] | the seventeen picked in order, a one-vertebra gap filled |
//! | [`corners`] | each corner settled on the full-size film |
//! | [`to_marks`] | the answer as the marks the rest of the engine reads |
//! | [`watch`] | the stepper and its frames |
//! | [`run`] | one-call entry points for native use |

#![forbid(unsafe_code)]

pub mod chain;
pub mod corners;
pub mod framing;
pub mod grid;
pub mod intake;
pub mod layers;
pub mod outline;
pub mod overlap;
pub mod reading;
pub mod run;
pub mod side;
pub mod spine_net;
pub mod to_marks;
pub mod watch;
pub mod weights;
