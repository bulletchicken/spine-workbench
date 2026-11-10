//! Image filters every step shares. The darkroom knows nothing about
//! spines: it processes pictures, and the auto pass and the tools decide
//! what the results mean.
//!
//! Every image is a [`grey_image::GreyImage`], which carries where it sits
//! on the original film, so a result found on a crop or a smaller working
//! copy is always reported in original film pixels. Every transcendental
//! function comes from `libm`, so native and browser results match.
//!
//! | Module | Holds |
//! |---|---|
//! | [`grey_image`] | `GreyImage<T>` and its placement on the film |
//! | [`normalise`] | percentile windows; 16-bit values to floats |
//! | [`blur`] | separable Gaussian and box blurs |
//! | [`gradients`] | Scharr gradients |
//! | [`hessian`] | second derivatives; ridge and valley measures |
//! | [`fft`] | the 2D FFT with padding |
//! | [`pyramid`] | working-resolution copies with an exact map back |
//! | [`sampling`] | bilinear sampling, along curves and their normals |
//! | [`regions`] | flood fill, connected parts, outlines |
//! | [`preview`] | float maps to small 8-bit tiles for watch frames |
//! | [`log_gabor`] | the oriented log-Gabor filter bank |
//! | [`phase_congruency`] | contrast-invariant phase-based edges and lines |
//! | [`radial_symmetry`] | the ring detector |

#![forbid(unsafe_code)]

pub mod blur;
pub mod fft;
pub mod gradients;
pub mod grey_image;
pub mod hessian;
pub mod log_gabor;
pub mod normalise;
pub mod phase_congruency;
pub mod preview;
pub mod pyramid;
pub mod radial_symmetry;
pub mod regions;
pub mod sampling;
