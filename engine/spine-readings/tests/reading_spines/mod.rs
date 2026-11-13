//! Small spines with independent, hand-stated angles.

use shapes::film::FilmKind;
use shapes::film_geometry::{FilmGeometry, ImageRight};
use shapes::landmarks::{Landmarks, SacrumTop};
use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::marks::Mark;
use shapes::naming::{ChosenBy, LevelNaming, Template};
use shapes::pixels::Px;
use shapes::spine_fit::{
    FilmPlacement, FilmToFit, FitAngles, FitLengths, FitRmsMm, FitStandardDeviations,
    FitStopReason, FittedVertebra, InferredFit, MarkSpread, SpineFit,
};
use shapes::spine_readings::{Band, Reading, ReadingsInput};
use shapes::vertebra::FilmMarks;

pub fn near(got: f64, expected: f64, tolerance: f64) {
    assert!(
        (got - expected).abs() <= tolerance,
        "{got} against {expected}, tolerance {tolerance}"
    );
}

pub fn measured(reading: Reading) -> (f64, Band) {
    match reading {
        Reading::Measured { value_deg, band } => (value_deg, band),
        value => panic!("a measured angle was expected: {value:?}"),
    }
}

pub fn vertebra(level: Level, tilt: f64, lean: f64, y: f64, z: f64) -> FittedVertebra {
    FittedVertebra {
        level,
        pose: VertebraPose {
            level,
            centre: [0.0, y, z],
            // The front measurement is carried independently in corner_tilt_rad.
            // A pure Ry(lean) gives the exact sagittal normal angle lean.
            rotation: [0.0, lean.to_radians(), 0.0],
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        },
        angles: FitAngles {
            side_tilt_rad: 0.0,
            forward_lean_rad: lean.to_radians(),
            twist_rad: 0.0,
        },
        sd: FitStandardDeviations {
            centre_mm: [0.0; 3],
            angles_rad: [0.0; 3],
        },
        inferred: InferredFit {
            depth: false,
            width: false,
            twist: false,
        },
        rms_mm: FitRmsMm {
            front: Some(0.0),
            side: Some(0.0),
        },
        corner_tilt_rad: Some(tilt.to_radians()),
    }
}

pub fn input(vertebrae: Vec<FittedVertebra>) -> ReadingsInput {
    let placement = FilmPlacement {
        mm_per_px: [2.0, 3.0],
        origin_px: [300.0, 400.0],
        facing_assumed: false,
    };
    ReadingsInput {
        fit: SpineFit {
            origin_level: vertebrae.last().unwrap().level,
            vertebrae,
            side_scale: 1.0,
            side_offset_mm: 0.0,
            iterations: 0,
            converged: true,
            stop_reason: FitStopReason::SmallStep,
            rms_mm: 0.0,
            lengths: FitLengths::Measured,
            typical_curve_deg: None,
            front_film: placement,
            side_film: Some(placement),
        },
        front: None,
        side: Some(sacrum_film()),
        bend_left: None,
        bend_right: None,
        versions: 32,
        seed: 3,
    }
}

fn sacrum_film() -> FilmToFit {
    // Under the fit's placement, dx=80 mm and dz=-80*tan(35°) mm;
    // the sacrum's upward normal therefore leans 35°. The deliberately
    // different film geometry scale proves that the fit's placement wins.
    let rise_px = 40.0 * libm::tan(35.0_f64.to_radians()) / 3.0;
    FilmToFit {
        marks: FilmMarks {
            film: FilmKind::Side,
            vertebrae: Vec::new(),
            landmarks: Landmarks {
                sacrum_top: Some(SacrumTop::Side {
                    sac_a: Mark::auto(Px::new(320.0, 400.0 + rise_px)),
                    sac_p: Mark::auto(Px::new(280.0, 400.0 - rise_px)),
                }),
                ..Landmarks::NONE
            },
            naming: LevelNaming {
                template: Template::Standard,
                chosen_by: ChosenBy::User,
                alternatives: Vec::new(),
                from_top: None,
                from_bottom: None,
                flags: Vec::new(),
                l5_confirmed_on: None,
            },
            named_for: ImageRight::Anterior,
        },
        geometry: FilmGeometry {
            mm_per_px: Some(0.5),
            image_right: Some(ImageRight::Anterior),
            ..FilmGeometry::UNKNOWN
        },
        size: [1000, 1000],
        mark_spread: MarkSpread::Millimetres {
            sd_mm: 1.0,
            shared_sd_mm: 0.0,
        },
        facing_assumed: false,
    }
}

pub fn hand_spine() -> ReadingsInput {
    // Pure side rotations make the named sagittal answers simple differences:
    // T2-T5=8°, T5-T12=22°, T10-L2=5°, L1-L5 lordosis=40°,
    // L1-sacrum lordosis=50°. Front measured tilts form a right curve
    // from T2(-15°) to L1(+15°), with the apex at T10(y=-12).
    let rows = [
        (Level::T2, -15.0, 20.0, 0.0),
        (Level::T5, -8.0, 12.0, -6.0),
        (Level::T10, 0.0, 0.0, -12.0),
        (Level::T12, 8.0, -10.0, -6.0),
        (Level::L1, 15.0, -15.0, 0.0),
        (Level::L2, 8.0, -5.0, 3.0),
        (Level::L5, 0.0, 25.0, 0.0),
    ];
    input(
        rows.into_iter()
            .enumerate()
            .map(|(i, (level, tilt, lean, y))| {
                vertebra(level, tilt, lean, y, 200.0 - i as f64 * 25.0)
            })
            .collect(),
    )
}
