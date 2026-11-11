//! Run settings shared by the native and worker entry points.
//!
//! Stage weights remain with the stages that own them. These orchestration
//! choices are experimental defaults, not accuracy-calibrated weights.

/// One algorithm work unit per call when a native caller drains a pass.
/// This changes scheduling only; it never changes a checkpoint or answer.
pub const DEFAULT_WORK_UNITS: u32 = 1;

/// The barcode's smoothing scale at the worker boundary. The frozen strip
/// baton does not carry the centre line's size factor, so both producers
/// and consumers use this same unit factor. No scale is inferred from the
/// returned marks, and assumed physical units remain flagged as assumed.
pub const BARCODE_SIZE_FACTOR: f64 = 1.0;
