//! Measurements in the forward model's right-handed patient frame: x is
//! anterior, y points to the patient's left, z points cranially; lengths
//! are millimetres; the origin is the underside of L5. Positive coronal
//! tilt is atan2(-normal.y, normal.z).
//! A positive signed Cobb (lower tilt minus upper tilt) is right-convex;
//! its apex has negative lateral deviation. Positive kyphosis is upper
//! sagittal tilt minus lower sagittal tilt, atan2(normal.x, normal.z).
//! Named twist is positive when the body's front turns to the left.
//!
//! Every nonzero curve between alternating tilt extremes is retained:
//! the floor applies only to the contract’s `SpineReadings.curves`, never
//! this per-version Rust API consumed by level rules. Fit marginal errors
//! are independent in the versions;
//! neighbouring joint-angle noise can therefore be overstated.

pub mod geometry;
pub use geometry::measure;
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::spine_readings::BodySide;

/// Why a geometric quantity cannot be read from the supplied films.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadReason {
    NoSacrum,
    NoSideFilm,
    NoBendingFilms,
    TwistNotMeasured,
    NotFitted,
}
/// A value or the missing observation that prevents it.
pub type Reading<T> = Result<T, ReadReason>;

/// Endplate and named-angle measurements for one vertebra, in degrees.
#[derive(Clone, Debug, PartialEq)]
pub struct VertebraReading {
    pub level: Level,
    pub centre_mm: [f64; 3],
    pub upper_tilt_deg: f64,
    pub lower_tilt_deg: f64,
    pub twist_deg: Reading<f64>,
    /// The disc below, positive when opening to the patient's left.
    pub disc_below_deg: Reading<f64>,
    pub centre_line_offset_mm: Reading<f64>,
}

/// One standing curve. Indices refer to the fit's top-to-bottom order,
/// independent of its level names. Endpoints use the upper endplate of
/// the upper end and the lower endplate of the lower end.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub upper_index: usize,
    pub lower_index: usize,
    pub apex_index: usize,
    /// None when the apex lies outside the published classification regions.
    pub region: Option<CurveRegion>,
    pub convex: BodySide,
    pub cobb_deg: f64,
    pub signed_cobb_deg: f64,
    pub apical_deviation_mm: f64,
    pub bending_cobb_deg: Reading<f64>,
    /// Which bend supplies the smaller residual, for later disc rules.
    pub straightening_bend: Reading<shapes::film::FilmKind>,
    pub bending_disc_below_deg: Reading<f64>,
}

/// Geometry of one version, before classification or level rules.
#[derive(Clone, Debug, PartialEq)]
pub struct SpineReading {
    pub vertebrae: Vec<VertebraReading>,
    pub curves: Vec<Curve>,
    pub major: Option<usize>,
    pub csvl_y_mm: Reading<f64>,
    pub kyphosis_t2_t5_deg: Reading<f64>,
    pub kyphosis_t5_t12_deg: Reading<f64>,
    pub kyphosis_t10_l2_deg: Reading<f64>,
    pub l4_tilt_deg: Reading<f64>,
    pub minor_curve_minimum_deg: Option<f64>,
}
