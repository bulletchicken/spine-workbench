//! The layers the two networks are built from, written by hand.
//!
//! Every sum is added up in one fixed order that does not depend on the
//! target, and no step uses a fused multiply-add, so a map comes out with
//! the same bits natively and in WebAssembly. Powers come from `libm` for
//! the same reason.
//!
//! | Module | Holds |
//! |---|---|
//! | [`conv`] | 3x3 convolution, stride 1 or 2, bias, optional ReLU |
//! | [`pointwise`] | 1x1 convolution with bias |
//! | [`resample`] | bilinear upsampling by two, 2x2 average pooling |
//! | [`group_norm`] | group norm with its trained scale and shift |
//! | [`activation`] | sigmoid and SiLU |
//! | [`softmax_mean`] | the mean of points weighted by a softmax |

pub mod activation;
pub mod conv;
pub mod group_norm;
pub mod pointwise;
pub mod resample;
pub mod softmax_mean;

#[cfg(test)]
pub(crate) mod seeded;
