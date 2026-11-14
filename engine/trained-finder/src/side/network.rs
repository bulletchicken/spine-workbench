//! The five encoders, three decoders and single landmark head.

use super::reading::SideReading;
use crate::layers::{conv::Conv3x3, pointwise::Conv1x1};
use shapes::errors::EngineError;
use std::rc::Rc;

pub const WIDTH: usize = 384;
pub const HEIGHT: usize = 512;

pub struct SideNetwork {
    pub(crate) convs: Vec<Conv3x3>,
    pub(crate) head: Conv1x1,
}

impl SideNetwork {
    /// Takes the prepared canvas. Float conversion is deferred into bands.
    pub fn start(self: Rc<Self>, picture: Vec<u8>) -> Result<SideReading, EngineError> {
        SideReading::new(self, picture, HEIGHT, WIDTH)
    }
}
