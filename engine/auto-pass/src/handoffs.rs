//! The batons the auto pass hands from stage to stage, and the bytes two of
//! them travel as between workers. A frozen contract: the film stages
//! (evidence, centre line, strip) and the chain stages (chain search,
//! naming, confidence) are built against these types at the same time.
//!
//! ```text
//! film worker                                  spine worker
//! FilmEvidence ─▶ CentreLine ─▶ Strip, Barcode
//!                                 │ Strip::to_bytes (FilmPassResult.strip)
//!                                 └──────────────▶ ChainBoundaries ─▶ SettledChain
//!                                 ┌──────────────── SettledChain::to_bytes (FilmChain.chain)
//! points and landmarks ◀──────────┘
//! ```
//!
//! What a finished film pass leaves on the film's desk, for placing points
//! and for the adjusting tools, is a [`FilmPassCache`].
//!
//! # Units
//!
//! Every position is in original film pixels, and so is every angle: an
//! angle or direction is measured from film +x (right) toward film +y
//! (down), whatever turned or shrunk working grid it was computed on. A
//! stage that works on an upright copy turns its angles back, as it maps
//! its positions back through the grid's `Placement`. Vectors such as
//! "toward image right of the upright film" name a direction seen on the
//! upright film but are written as film-pixel steps.
//!
//! Lengths along the spine (arc length, strip rows and columns, widths)
//! are in the film's **length unit**: millimetres when the film has a
//! scale, and assumed millimetres, taken from the spine's own width, when
//! `scaleAssumed` is set. Nothing measured in assumed millimetres may be
//! reported as a length.

use darkroom::grey_image::GreyImage;
use serde::{Deserialize, Serialize};
use shapes::auto_pass::PassWarning;
use shapes::confidence::Confidence;
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::FilmKind;
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::naming::Template;
use shapes::pixels::{Px, PxRect};

/// Which way bone shows on a film.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Polarity {
    BoneBright,
    BoneDark,
}

// ------------------------------------------------------------ stage 1

/// Stage 1 hands on the evidence maps of one film, all on one working grid.
///
/// The centre line reads the ridge, edge and ring maps; points read the
/// ring map for pedicles; the adjusting tools read the edge maps. Every map
/// is zero where `valid` is zero, orientations included. Every direction
/// and orientation map holds angles in film pixel axes (see the module),
/// even where the grid is turned.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmEvidence {
    pub film: FilmKind,
    /// Millimetres per working-grid pixel (assumed millimetres when
    /// `scale_assumed`).
    pub mm_per_px: f64,
    pub scale_assumed: bool,
    /// 1 where the pixel is usable: not a shutter, marker, seam or border
    /// band, nor within 3 mm of one.
    pub valid: GreyImage<u8>,
    /// Bone bright, stretched to 0 to 1, unusable pixels filled smoothly.
    pub image: GreyImage<f32>,
    /// Monogenic phase congruency, 0 to 1.
    pub phase_edge: GreyImage<f32>,
    /// Across the feature, from its dark side to its bright side, in
    /// (−π, π].
    pub phase_direction: GreyImage<f32>,
    /// +π/2 for a bright line, 0 for a step, −π/2 for a dark line.
    pub phase_kind: GreyImage<f32>,
    /// Gaussian gradient over noise-floored local contrast; an ideal step
    /// scores 1.
    pub gradient_edge: GreyImage<f32>,
    pub gradient_direction: GreyImage<f32>,
    /// Bright-line strength, 0 to 1, and the line normal in [−π/2, π/2).
    pub ridge: GreyImage<f32>,
    pub ridge_orientation: GreyImage<f32>,
    /// Dark-line and dark-band strength (disc gaps), its normal, and the
    /// index of the winning scale in `hessian_scales_mm`.
    pub valley: GreyImage<f32>,
    pub valley_orientation: GreyImage<f32>,
    pub valley_scale: GreyImage<u8>,
    /// Monogenic phase symmetry, bright and dark polarity, 0 to 1.
    pub ridge_symmetry: GreyImage<f32>,
    pub valley_symmetry: GreyImage<f32>,
    /// Front films with a fine enough grid only.
    pub rings: Option<RingMaps>,
    /// The Hessian scales actually used, in millimetres.
    pub hessian_scales_mm: Vec<f64>,
    pub polarity: PolarityCheck,
    /// Burnt-in markers, text and small metal that were masked.
    pub markers: Vec<PxRect>,
    /// Film rows of stitching seams that were masked.
    pub seam_rows: Vec<f64>,
    /// Fraction of working pixels that are usable.
    pub valid_fraction: f64,
    /// Fraction of usable pixels with a phase edge above 0.1.
    pub coverage: f64,
    pub warnings: Vec<PassWarning>,
}

/// The pedicle ring map.
#[derive(Clone, Debug, PartialEq)]
pub struct RingMaps {
    /// Calibrated so an ideal ring scores 1.
    pub response: GreyImage<f32>,
    /// The winning shape: `size · aspects.len() + aspect`.
    pub shape: GreyImage<u8>,
    pub semi_axes_mm: Vec<f64>,
    pub aspects: Vec<f64>,
}

/// Which way bone shows, as the evidence judged it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarityCheck {
    pub found: Polarity,
    /// 0 to 1.
    pub confidence: f64,
    /// The cues were weak or contradictory: the user should confirm.
    pub needs_confirm: bool,
}

// ------------------------------------------------------------ stage 2

/// Stage 2 hands on the spine's centre line, from the neck to below the
/// top of the sacrum, sampled at a fixed step of arc length.
#[derive(Clone, Debug, PartialEq)]
pub struct CentreLine {
    pub film: FilmKind,
    /// The turns that showed the film upright while it was traced.
    pub quarter_turns: QuarterTurns,
    pub scale_assumed: bool,
    /// Arc length between samples, in length units.
    pub sample_step: f64,
    /// Head end first.
    pub samples: Vec<CentreSample>,
    pub ends: CentreLineEnds,
    pub warnings: Vec<PassWarning>,
}

/// One point of the centre line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentreSample {
    /// Arc length from the head end, in length units.
    pub arc: f64,
    pub at: Px,
    /// The film-pixel step for one length unit of arc toward the pelvis.
    pub along: Px,
    /// Signed curvature per length unit, positive when the line bends
    /// toward image right of the upright film.
    pub curvature: f64,
    /// The spine's width here, in length units.
    pub width: f64,
    /// False when the width fell back to an estimate from the scale.
    pub width_measured: bool,
    /// The ridge evidence at this point, 0 to 1.
    pub evidence: f64,
}

/// What the centre line found at its two ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentreLineEnds {
    /// Arc length of the base of the neck, when found.
    pub neck_base: Option<f64>,
    /// Arc length level with the iliac crests, when found.
    pub iliac_crest: Option<f64>,
    pub top_cropped: bool,
    pub bottom_cropped: bool,
    /// The patient's size against an adult, 0.45 to 1.2; 1 when it could
    /// not be judged (and a `size-unknown` warning says so).
    pub size_factor: f64,
}

// ------------------------------------------------------------ stage 3

/// Stage 3 hands on the spine unrolled: a straight strip along the centre
/// line, rows from the head end down, columns across.
///
/// The film point of strip row `i`, column `j` is
/// `layout[i].centre + (j − (cols − 1) / 2) · layout[i].across`, exact at
/// whole rows and linear between them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strip {
    pub film: FilmKind,
    pub rows: u32,
    /// Odd: the centre line runs down column `(cols − 1) / 2`.
    pub cols: u32,
    /// Length units between rows, and between columns.
    pub row_step: f64,
    pub col_step: f64,
    pub scale_assumed: bool,
    /// `rows × cols`, row-major: normalised grey values, bone near 1,
    /// capped at 1.5; NaN where invalid.
    pub values: Vec<f32>,
    /// `rows × cols`: 1 inside the film and its collimation.
    pub valid: Vec<u8>,
    /// One per row.
    pub layout: Vec<StripRow>,
    /// The film's `imageRight` whenever it is known, confirmed or not:
    /// front and bending films name left and right from it, side films
    /// anterior and posterior, and the marks record it as `namedFor`.
    /// `None` only while `imageRight` is unknown; the pass then names for
    /// its own guess and records that.
    pub image_right: Option<ImageRight>,
    /// What the evidence stage judged, when it judged firmly.
    pub polarity_hint: Option<Polarity>,
    pub evidence: StripEvidence,
}

/// Where one strip row lies on the film.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StripRow {
    /// The row's centre-line point.
    pub centre: Px,
    /// The film-pixel step for one column toward image right of the
    /// upright film.
    pub across: Px,
    /// The film-pixel step for one row toward the pelvis.
    pub along: Px,
    /// Signed curvature per length unit, positive toward `across`.
    pub curvature: f64,
    /// The spine's half-width here, in length units; 0 when unknown.
    pub half_width: f64,
    /// Height in length units, up positive, 0 at row 0.
    pub z: f64,
}

/// Evidence maps resampled onto the strip, each `rows × cols`, row-major,
/// where the evidence stage made them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StripEvidence {
    /// Signed edge strength along the strip, positive where it brightens
    /// toward the pelvis.
    pub edge_along: Option<Vec<f32>>,
    /// The pedicle ring map (front and bending films).
    pub ring: Option<Vec<f32>>,
    /// The ridge map.
    pub ridge: Option<Vec<f32>>,
    /// The ridge's line angle in [−π/2, π/2), in film pixel axes: radians
    /// from film +x toward film +y.
    pub ridge_angle: Option<Vec<f32>>,
}

/// Stage 3 also hands on the bone and gap profile along the strip, one
/// value per strip row.
#[derive(Clone, Debug, PartialEq)]
pub struct Barcode {
    pub film: FilmKind,
    /// Mean grey value across the middle of the strip; NaN where invalid.
    pub values: Vec<f32>,
    /// High-passed and scaled: bone positive, disc gaps negative; NaN where
    /// invalid.
    pub high_pass: Vec<f32>,
    /// 1 where the row is valid.
    pub valid: Vec<u8>,
}

/// What a finished film pass leaves on the film's desk.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmPassCache {
    pub evidence: FilmEvidence,
    pub centre_line: CentreLine,
    pub strip: Strip,
    pub barcode: Barcode,
}

// ------------------------------------------------------- stages 4 to 6

/// Stage 4 hands on one film's solved chain on its strip: every body from
/// C7 down, with its endplates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainBoundaries {
    pub film: FilmKind,
    /// The vertebra count the chain was solved with.
    pub template: Template,
    /// The polarity the chain was solved with.
    pub polarity: Polarity,
    /// Top to bottom, C7 first, levels in order.
    pub bodies: Vec<FoundBody>,
    /// The sacral plateau, when found.
    pub sacrum: Option<EndplateOnStrip>,
    /// The chain's total energy; lower is better.
    pub energy: f64,
    pub warnings: Vec<PassWarning>,
}

/// One vertebral body found on the strip.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundBody {
    pub level: Level,
    pub sup: EndplateOnStrip,
    pub inf: EndplateOnStrip,
    /// Placed where the film shows nothing (cropped off), from the chain
    /// alone.
    pub inferred: bool,
    pub certainty: BodyCertainty,
}

/// An endplate line on the strip. It crosses the centre column at `row`
/// (fractional), and `k` columns from the centre it passes through row
/// `row + k · colStep · tan(tilt) / rowStep`: positive tilts descend toward
/// image right of the upright film.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndplateOnStrip {
    pub row: f64,
    /// Radians.
    pub tilt: f64,
}

/// What the chain search knows about how sure one body is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyCertainty {
    /// Posterior that both endplates lie within a third of a body height
    /// of where they were placed, 0 to 1.
    pub posterior: f64,
    /// One standard deviation of each endplate's position, in length units.
    pub sup_spread: f64,
    pub inf_spread: f64,
    /// Energy gap to the best placement outside that box.
    pub margin: f64,
}

/// Stages 5 and 6 settle the chain: its bodies carry their final level
/// names, and each has its confidence. This is what the film's worker
/// turns into points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettledChain {
    pub boundaries: ChainBoundaries,
    /// One per body, in the same order.
    pub confidence: Vec<Confidence>,
}

// ------------------------------------------------------------- bytes

// Each encoding starts with four magic bytes and a format number, both
// checked on reading, so a baton from a different engine build is refused
// instead of misread. Numbers are little-endian; float arrays keep their
// exact bits, NaN included.
const STRIP_MAGIC: &[u8; 4] = b"SWst";
const CHAIN_MAGIC: &[u8; 4] = b"SWch";
const FORMAT: u32 = 1;

/// The strip's fields other than its pixel arrays.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StripHeader {
    film: FilmKind,
    rows: u32,
    cols: u32,
    row_step: f64,
    col_step: f64,
    scale_assumed: bool,
    layout: Vec<StripRow>,
    image_right: Option<ImageRight>,
    polarity_hint: Option<Polarity>,
    /// Which of edge along, ring, ridge and ridge angle follow.
    evidence: [bool; 4],
}

impl Strip {
    /// The strip as bytes, for `FilmPassResult.strip`: magic, format, the
    /// length of a JSON header, the header, then `values`, `valid` and
    /// each evidence map present.
    pub fn to_bytes(&self) -> Vec<u8> {
        let maps = [
            &self.evidence.edge_along,
            &self.evidence.ring,
            &self.evidence.ridge,
            &self.evidence.ridge_angle,
        ];
        let header = StripHeader {
            film: self.film,
            rows: self.rows,
            cols: self.cols,
            row_step: self.row_step,
            col_step: self.col_step,
            scale_assumed: self.scale_assumed,
            layout: self.layout.clone(),
            image_right: self.image_right,
            polarity_hint: self.polarity_hint,
            evidence: maps.map(Option::is_some),
        };
        let mut bytes = start(STRIP_MAGIC);
        put_json(&mut bytes, &header);
        put_floats(&mut bytes, &self.values);
        bytes.extend_from_slice(&self.valid);
        for map in maps.into_iter().flatten() {
            put_floats(&mut bytes, map);
        }
        bytes
    }

    /// Reads [`Strip::to_bytes`], refusing anything that does not match
    /// exactly: a wrong magic or format, a damaged header, arrays of the
    /// wrong length, or bytes left over.
    pub fn from_bytes(bytes: &[u8]) -> Result<Strip, EngineError> {
        let mut reader = Reader::open(bytes, STRIP_MAGIC, "strip")?;
        let header: StripHeader = reader.json()?;
        let cells = usize::try_from(u64::from(header.rows) * u64::from(header.cols))
            .map_err(|_| refused("strip", "the strip is too large"))?;
        if header.cols.is_multiple_of(2) {
            return Err(refused("strip", "the column count is even"));
        }
        if header.layout.len() != header.rows as usize {
            return Err(refused(
                "strip",
                "the layout does not have one entry per row",
            ));
        }
        let values = reader.floats(cells)?;
        let valid = reader.take(cells)?.to_vec();
        let [edge_along, ring, ridge, ridge_angle] = header.evidence;
        let mut map = |present: bool| present.then(|| reader.floats(cells)).transpose();
        let evidence = StripEvidence {
            edge_along: map(edge_along)?,
            ring: map(ring)?,
            ridge: map(ridge)?,
            ridge_angle: map(ridge_angle)?,
        };
        reader.finish()?;
        Ok(Strip {
            film: header.film,
            rows: header.rows,
            cols: header.cols,
            row_step: header.row_step,
            col_step: header.col_step,
            scale_assumed: header.scale_assumed,
            values,
            valid,
            layout: header.layout,
            image_right: header.image_right,
            polarity_hint: header.polarity_hint,
            evidence,
        })
    }
}

impl SettledChain {
    /// The settled chain as bytes, for `FilmChain.chain`: magic, format,
    /// the length of the JSON body, the body.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = start(CHAIN_MAGIC);
        put_json(&mut bytes, self);
        bytes
    }

    /// Reads [`SettledChain::to_bytes`], refusing a wrong magic or format,
    /// a damaged body, a confidence count that does not match the bodies,
    /// or bytes left over.
    pub fn from_bytes(bytes: &[u8]) -> Result<SettledChain, EngineError> {
        let mut reader = Reader::open(bytes, CHAIN_MAGIC, "chain")?;
        let chain: SettledChain = reader.json()?;
        reader.finish()?;
        if chain.confidence.len() != chain.boundaries.bodies.len() {
            return Err(refused("chain", "one confidence per body is required"));
        }
        Ok(chain)
    }
}

fn start(magic: &[u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&FORMAT.to_le_bytes());
    bytes
}

fn put_json<T: Serialize>(bytes: &mut Vec<u8>, value: &T) {
    let json = serde_json::to_vec(value).expect("baton types always serialise");
    let length = u32::try_from(json.len()).expect("a baton header is under 4 GiB");
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&json);
}

fn put_floats(bytes: &mut Vec<u8>, values: &[f32]) {
    bytes.reserve(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
}

fn refused(what: &str, why: &str) -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfSync,
        format!("the {what} handed between workers could not be read"),
    )
    .with_detail(why)
}

/// Reads a baton front to back.
struct Reader<'a> {
    rest: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    fn open(bytes: &'a [u8], magic: &[u8; 4], what: &'static str) -> Result<Self, EngineError> {
        let mut reader = Reader { rest: bytes, what };
        if reader.take(4)? != magic {
            return Err(refused(what, "wrong magic bytes"));
        }
        if reader.u32()? != FORMAT {
            return Err(refused(what, "unknown format number"));
        }
        Ok(reader)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], EngineError> {
        if self.rest.len() < count {
            return Err(refused(self.what, "the bytes end early"));
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        let mut word = [0u8; 4];
        word.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(word))
    }

    fn json<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, EngineError> {
        let length = self.u32()? as usize;
        let json = self.take(length)?;
        serde_json::from_slice(json).map_err(|error| refused(self.what, &error.to_string()))
    }

    fn floats(&mut self, count: usize) -> Result<Vec<f32>, EngineError> {
        let length = count
            .checked_mul(4)
            .ok_or_else(|| refused(self.what, "the array is too large"))?;
        Ok(self
            .take(length)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| f32::from_bits(u32::from_le_bytes(*word)))
            .collect())
    }

    fn finish(self) -> Result<(), EngineError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(refused(self.what, "bytes are left over"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::auto_pass::PassWarningCode;
    use shapes::confidence::{ConfidenceBand, ConfidenceReason};

    fn strip() -> Strip {
        let row = |i: u32| StripRow {
            centre: Px::new(100.0, 40.0 + f64::from(i) * 2.5),
            across: Px::new(3.3, 0.0),
            along: Px::new(0.0, 3.3),
            curvature: 0.001,
            half_width: if i == 0 { 0.0 } else { 14.0 },
            z: -0.5 * f64::from(i),
        };
        Strip {
            film: FilmKind::Front,
            rows: 2,
            cols: 3,
            row_step: 0.5,
            col_step: 0.5,
            scale_assumed: false,
            values: vec![0.1, f32::NAN, 1.5, 0.0, -0.0, 0.75],
            valid: vec![1, 0, 1, 1, 1, 1],
            layout: vec![row(0), row(1)],
            image_right: Some(ImageRight::PatientLeft),
            polarity_hint: None,
            evidence: StripEvidence {
                edge_along: None,
                ring: Some(vec![0.0, 0.25, 1.0, 0.5, 0.0, f32::INFINITY]),
                ridge: None,
                ridge_angle: Some(vec![-1.5, 0.0, 1.5, 0.1, 0.2, 0.3]),
            },
        }
    }

    fn same_bits(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
    }

    #[test]
    fn a_strip_crosses_workers_unchanged() {
        let sent = strip();
        let received = Strip::from_bytes(&sent.to_bytes()).expect("the strip reads back");
        assert!(same_bits(&received.values, &sent.values));
        assert!(same_bits(
            received.evidence.ring.as_deref().unwrap_or_default(),
            sent.evidence.ring.as_deref().unwrap_or_default()
        ));
        assert_eq!(received.evidence.edge_along, None);
        assert_eq!(received.layout, sent.layout);
        assert_eq!(received.valid, sent.valid);
        assert_eq!(received.image_right, sent.image_right);
        assert_eq!(received.evidence.ridge_angle, sent.evidence.ridge_angle);
    }

    #[test]
    fn a_damaged_strip_is_refused() {
        let bytes = strip().to_bytes();
        let refusals = [
            bytes[..bytes.len() - 1].to_vec(),
            [bytes.as_slice(), &[0]].concat(),
            [b"SWch".as_slice(), &bytes[4..]].concat(),
            [&bytes[..4], &2u32.to_le_bytes(), &bytes[8..]].concat(),
        ];
        for damaged in refusals {
            let error = Strip::from_bytes(&damaged).expect_err("damaged bytes are refused");
            assert_eq!(error.code, ErrorCode::EngineOutOfSync);
        }
    }

    #[test]
    fn a_layout_must_have_one_entry_per_row() {
        let mut short = strip();
        short.layout.pop();
        assert!(Strip::from_bytes(&short.to_bytes()).is_err());
    }

    fn settled() -> SettledChain {
        let body = |level, row: f64| FoundBody {
            level,
            sup: EndplateOnStrip { row, tilt: 0.05 },
            inf: EndplateOnStrip {
                row: row + 40.0,
                tilt: -0.02,
            },
            inferred: false,
            certainty: BodyCertainty {
                posterior: 0.97,
                sup_spread: 0.6,
                inf_spread: 0.8,
                margin: 12.5,
            },
        };
        let confidence = Confidence {
            score: 0.97,
            band: ConfidenceBand::High,
            reasons: vec![ConfidenceReason::RibOverlap],
            calibrated: false,
            endplate_spread_px: None,
            margin: Some(12.5),
        };
        SettledChain {
            boundaries: ChainBoundaries {
                film: FilmKind::Side,
                template: Template::Standard,
                polarity: Polarity::BoneBright,
                bodies: vec![body(Level::C7, 10.0), body(Level::T1, 60.0)],
                sacrum: Some(EndplateOnStrip {
                    row: 900.25,
                    tilt: 0.7,
                }),
                energy: -153.0,
                warnings: vec![PassWarning {
                    code: PassWarningCode::TiltInconsistent,
                    film: FilmKind::Side,
                    level: Some(Level::T1),
                    region: None,
                }],
            },
            confidence: vec![confidence.clone(), confidence],
        }
    }

    #[test]
    fn a_settled_chain_crosses_workers_unchanged() {
        let sent = settled();
        let received = SettledChain::from_bytes(&sent.to_bytes()).expect("the chain reads back");
        assert_eq!(received, sent);
    }

    #[test]
    fn a_chain_needs_one_confidence_per_body() {
        let mut chain = settled();
        chain.confidence.pop();
        let error = SettledChain::from_bytes(&chain.to_bytes()).expect_err("refused");
        assert_eq!(error.code, ErrorCode::EngineOutOfSync);
        assert!(SettledChain::from_bytes(&strip().to_bytes()).is_err());
    }
}
