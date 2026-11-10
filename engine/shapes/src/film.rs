//! A film: which one it is, how its pixels are stored in the case file,
//! the short list of facts kept from the original file, and how the stored
//! pixel values relate to the values in that file.
//!
//! Nothing here can hold a name, an identifier, a date, an institution or
//! the original file name: the types have no field for them, so they cannot
//! be stored.

use crate::film_geometry::{FilmGeometry, QuarterTurns};
use crate::pixels::PxRect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The four films of a case. Bending films are front views.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum FilmKind {
    Front,
    Side,
    BendLeft,
    BendRight,
}

/// Which way the beam saw the patient.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum View {
    /// Front to back: the front film and both bending films.
    Front,
    /// Side to side.
    Side,
}

impl FilmKind {
    pub const ALL: [FilmKind; 4] = [
        FilmKind::Front,
        FilmKind::Side,
        FilmKind::BendLeft,
        FilmKind::BendRight,
    ];

    /// The film's name in JSON, TypeScript and zip paths.
    pub const fn name(self) -> &'static str {
        match self {
            FilmKind::Front => "front",
            FilmKind::Side => "side",
            FilmKind::BendLeft => "bend-left",
            FilmKind::BendRight => "bend-right",
        }
    }

    /// Where the film's pixels live inside the case file.
    pub const fn png_path(self) -> &'static str {
        match self {
            FilmKind::Front => "films/front.png",
            FilmKind::Side => "films/side.png",
            FilmKind::BendLeft => "films/bend-left.png",
            FilmKind::BendRight => "films/bend-right.png",
        }
    }

    pub const fn view(self) -> View {
        match self {
            FilmKind::Side => View::Side,
            FilmKind::Front | FilmKind::BendLeft | FilmKind::BendRight => View::Front,
        }
    }

    pub const fn is_bending(self) -> bool {
        matches!(self, FilmKind::BendLeft | FilmKind::BendRight)
    }
}

/// The largest film side accepted anywhere, in pixels.
pub const MAX_FILM_SIDE: u32 = 16_384;

/// The stored image of one film inside the case file: a lossless grayscale
/// PNG at the film's own bit depth, re-encoded from the decoded pixels, so
/// no tag or EXIF from the original can travel with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmImage {
    /// Always [`FilmKind::png_path`] of the film it belongs to.
    #[cfg_attr(
        feature = "typescript",
        ts(
            type = "'films/front.png' | 'films/side.png' | 'films/bend-left.png' | 'films/bend-right.png'"
        )
    )]
    pub path: String,
    pub width: u32,
    pub height: u32,
    /// Significant bits per pixel, 1 to 16. Values are stored left-aligned
    /// in an 8- or 16-bit PNG with an sBIT chunk, so `value >> (16 − bits)`
    /// (or `8 − bits`) gives back the exact stored value.
    pub bits_stored: u8,
}

/// One film's stored image as it travels with a case: the bytes of
/// [`FilmImage`]'s PNG, for the film of that kind. Unpacking a case gives
/// these; packing one takes them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmPng {
    pub kind: FilmKind,
    #[serde(with = "serde_bytes")]
    #[schemars(with = "Vec<u8>")]
    #[cfg_attr(feature = "typescript", ts(type = "Uint8Array"))]
    pub png: Vec<u8>,
}

/// What the original file was. Not identifying, and it matters: a JPEG
/// source is lossy, and only a DICOM source has kept tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SourceFormat {
    Png,
    Jpeg,
    Dicom,
}

/// The keep-list: the only DICOM values ever copied out of a file. Every
/// other tag (names, identifiers, dates, times, institutions, operators,
/// stations, instance identifiers, private tags) is dropped while the file
/// is parsed and has no field here. All are `null` when the file did not
/// carry them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeptTags {
    /// (0008,0016) the class of image, a fixed standard identifier (never
    /// the instance identifier). It decides what the spacing tags mean.
    pub sop_class: Option<String>,
    /// (0008,0060), for example `DX` or `CR`.
    pub modality: Option<String>,
    /// (0008,0068).
    pub presentation_intent: Option<PresentationIntent>,
    /// (0008,0070), cut to 64 characters. Tells slot scanners apart.
    pub manufacturer: Option<String>,
    /// (0008,1090), cut to 64 characters.
    pub model_name: Option<String>,
    /// (0028,0004).
    pub photometric: Option<Photometric>,
    /// (0028,0101) as declared in the file.
    pub bits_stored: Option<u8>,
    /// (0028,0030) in mm, row spacing (vertical) then column spacing.
    pub pixel_spacing: Option<[f64; 2]>,
    /// (0018,1164) in mm, row then column.
    pub imager_pixel_spacing: Option<[f64; 2]>,
    /// (0018,2010) in mm, row then column.
    pub nominal_scanned_pixel_spacing: Option<[f64; 2]>,
    /// (0028,0A02).
    pub pixel_spacing_calibration: Option<SpacingCalibration>,
    /// (0018,1114).
    pub estimated_magnification: Option<f64>,
    /// (0018,1110) in mm. Kept for the cone and slot cameras of Wave 2.
    pub distance_source_to_detector: Option<f64>,
    /// (0018,1111) in mm.
    pub distance_source_to_patient: Option<f64>,
    /// (0028,0034) vertical then horizontal.
    pub pixel_aspect_ratio: Option<[u32; 2]>,
    /// (0020,0020): the patient directions of +x and +y, letters A, P, L,
    /// R, H and F only.
    pub patient_orientation: Option<[String; 2]>,
    /// (0018,5101).
    pub view_position: Option<ViewPosition>,
    /// (0020,0062).
    pub image_laterality: Option<Laterality>,
    /// (0028,1053).
    pub rescale_slope: Option<f64>,
    /// (0028,1052).
    pub rescale_intercept: Option<f64>,
    /// (0028,1050), the first value, in modality units.
    pub window_centre: Option<f64>,
    /// (0028,1051), the first value.
    pub window_width: Option<f64>,
    /// (0028,1056).
    pub window_function: Option<WindowFunction>,
    /// (0018,1600 to 1620), geometry only. Several shapes together leave
    /// the smallest visible area.
    pub shutters: Option<Vec<Shutter>>,
    /// (0028,2110).
    pub lossy_compression: Option<bool>,
    /// (0028,2112), the first value.
    pub lossy_ratio: Option<f64>,
    /// (0028,0301).
    pub burned_in_annotation: Option<bool>,
    /// The transfer syntax, named from our own table.
    pub transfer_syntax: Option<TransferSyntax>,
    /// (0028,0008). Only the first frame is read.
    pub number_of_frames: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum PresentationIntent {
    ForPresentation,
    ForProcessing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Photometric {
    Monochrome1,
    Monochrome2,
    Rgb,
    YbrFull,
    YbrFull422,
    YbrRct,
    YbrIct,
    PaletteColour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum SpacingCalibration {
    Geometry,
    Fiducial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ViewPosition {
    Ap,
    Pa,
    Ll,
    Rl,
    Rld,
    Lld,
    Rlo,
    Llo,
    Lateral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Laterality {
    Left,
    Right,
    Both,
    Unpaired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum WindowFunction {
    Linear,
    LinearExact,
    Sigmoid,
}

/// A display shutter as the file describes it, in the file's own 1-based
/// row and column numbers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "shape",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Shutter {
    Rectangle {
        left: u32,
        right: u32,
        upper: u32,
        lower: u32,
    },
    Circle {
        centre_row: f64,
        centre_column: f64,
        radius: f64,
    },
    Polygon {
        /// Row, column pairs; the polygon closes itself.
        vertices: Vec<[f64; 2]>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum TransferSyntax {
    ImplicitLittleEndian,
    ExplicitLittleEndian,
    ExplicitBigEndian,
    DeflatedLittleEndian,
    EncapsulatedUncompressed,
    RleLossless,
    JpegBaseline,
    JpegExtended,
    JpegLossless,
    JpegLosslessFirstOrder,
    JpegLsLossless,
    JpegLsNearLossless,
    Jpeg2000Lossless,
    Jpeg2000,
}

/// How a stored pixel value `p` (bone bright, `0 ..= 2^bits − 1`) relates
/// to the value `s` in the original file, and from there to modality
/// values: `s = offset + (inverted ? 2^bits − 1 − p : p)`, then `m` is
/// `modality` applied to `s`. Both steps are exact, so windows from the
/// file can be applied to the stored pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelValues {
    /// The smallest value the file could hold (0, or −2^(bits−1) for
    /// signed data).
    pub offset: i32,
    /// True when the stored values run opposite to the file's values, so
    /// that bone is bright. Includes the user's own flip.
    pub inverted: bool,
    /// True when the user flipped the brightness by hand.
    pub inverted_by_user: bool,
    pub modality: ModalityMap,
}

impl PixelValues {
    /// Values exactly as stored: what a PNG or JPEG film has.
    pub const AS_STORED: PixelValues = PixelValues {
        offset: 0,
        inverted: false,
        inverted_by_user: false,
        modality: ModalityMap::Linear {
            slope: 1.0,
            intercept: 0.0,
        },
    };
}

/// From file values `s` to modality values `m`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ModalityMap {
    /// `m = slope · s + intercept`.
    Linear { slope: f64, intercept: f64 },
    /// `m = table[clamp(s − firstMapped, 0, table.length − 1)]`.
    Lookup {
        first_mapped: i32,
        bits: u8,
        table: Vec<u16>,
    },
}

/// Something noticed while reading a film that the stored pixels alone no
/// longer show.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum IntakeFlag {
    MultiFrame,
    ForProcessing,
    UnusualSopClass,
    NoFileMeta,
    NonstandardDeflate,
    BitsStoredMismatch,
    J2kNotExact,
    Jp2Wrapper,
    ColourConverted,
    AlphaIgnored,
    ExifMirrored,
    PolarityConflict,
    NonMonotoneModalityLut,
    DxRescaleNotIdentity,
    AnisotropicPixels,
    SodIsTableDistance,
    SlotGeometry,
    Lossy,
    BurnedInAnnotation,
    InvalidSpacing,
    ObliqueOrientation,
}

/// One film as saved in the case file.
///
/// There is no field for the original file name, so one cannot be stored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Film {
    pub kind: FilmKind,
    pub image: FilmImage,
    pub source: SourceFormat,
    /// `null` unless the source was DICOM.
    pub kept_tags: Option<KeptTags>,
    pub values: PixelValues,
    pub flags: Vec<IntakeFlag>,
    /// Boxes the user blanked out (names or IDs burnt into the image),
    /// each on the film. Every pixel a box covers (see
    /// [`PxRect::covers_pixel`]) is zero in the stored PNG and left out of
    /// every analysis.
    pub blanked: Vec<PxRect>,
    pub geometry: FilmGeometry,
}

/// The window the film opens with, before the user changes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DisplayWindow {
    /// Centre and width in modality units.
    CentreWidth {
        centre: f64,
        width: f64,
        function: WindowFunction,
        from: WindowSource,
    },
    /// A lookup table from modality values to display values.
    Lookup {
        first: i32,
        bits: u8,
        table: Vec<u16>,
        from: WindowSource,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum WindowSource {
    /// The first window centre and width in the file.
    File,
    /// The first VOI lookup table in the file.
    FileLookup,
    /// The 1st and 99th percentiles of the film's own values.
    Percentile,
}

/// What reading a film found out, before anything is decided. Never saved
/// as such: the case file keeps the parts that cannot be read again from
/// the stored PNG.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilmFacts {
    pub kind: FilmKind,
    pub width: u32,
    pub height: u32,
    pub bits_stored: u8,
    pub source: SourceFormat,
    pub kept_tags: Option<KeptTags>,
    pub values: PixelValues,
    pub flags: Vec<IntakeFlag>,
    pub default_window: DisplayWindow,
    /// Quarter turns clockwise that the JPEG's EXIF orientation asks for.
    /// A proposal only; pixels are never turned.
    pub exif_quarter_turns: Option<QuarterTurns>,
}
