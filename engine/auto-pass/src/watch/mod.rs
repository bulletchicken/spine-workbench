//! The part you watch: the auto pass as two state machines that do a bounded
//! slice of work per call and report frames at fixed checkpoints.

pub mod chain_pass;
pub mod film_pass;
pub mod frames;

pub use chain_pass::ChainPass;
pub use film_pass::FilmPass;
