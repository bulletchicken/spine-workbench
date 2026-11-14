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
//! the evidence gives no minimum minor-curve size. No scoliosis cutoff is
//! supplied here. Upper and lower plates are drawn independently in the
//! versions, at sqrt(2) times the fit's reported tilt spread; actual finder
//! errors may be correlated.

pub mod classification;
pub mod geometry;
pub mod lumbar;
pub mod summary;

use shapes::implant_layout::{CannotTellCode, CurveType, LumbarModifier, SagittalModifier};
use shapes::levels::Level;
use shapes::made_up::CurveRegion;
use shapes::spine_readings::BodySide;

/// A measured value or the code explaining why the films cannot give it.
pub type Reading<T> = Result<T, CannotTellCode>;

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
    pub structural: Reading<bool>,
}

/// The whole spine's classification in one version. Missing components
/// retain their reason; an unreadable structural test never holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Classification {
    pub curve_type: Reading<CurveType>,
    pub lumbar_modifier: Reading<LumbarModifier>,
    pub sagittal_modifier: Reading<SagittalModifier>,
}

/// All the readings of one version, with the film-dependent quantities
/// kept distinct from the geometry that can still be measured.
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
    pub classification: Classification,
    /// No source supplies this cutoff. Curves are not filtered by size.
    pub minor_curve_minimum_deg: Option<f64>,
    /// Evidence keys for the regional boundaries and classification.
    pub sources: Vec<String>,
}

pub use geometry::{measure, measure_with_pedicle_outlines};
