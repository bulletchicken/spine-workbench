//! The lead left/right letter, a ruler and a calibration ball.
//!
//! All three lie flat on the detector, so they are painted in upright film
//! millimetres rather than projected: a parallel film has one scale at every
//! depth, so the ruler's ticks and the ball's width are exact at the
//! film's scale.
//!
//! - **The lead letter** (`L` or `R`, a 5 × 7 block letter) sits in a top
//!   corner on the side of the patient it names. It reads the right way
//!   round when a front or bending film is shown with the patient's left
//!   on screen right, or a side film facing right, and mirrored otherwise.
//!   Practice varies, which is why the app never takes a facing from the
//!   letter alone.
//! - **The ruler** is a lead tick every 10 mm, longer every 50 mm, down
//!   one side of the film.
//! - **The ball** is a steel sphere of 25 or 30 mm.
//!
//! Each is reported with its exact place, so tests of the geometry sensing
//! know the answer.

use super::Canvas;
use crate::film_layout::FilmLayout;
use crate::recipes::{BallPlacement, MarkerPlacement, RulerPlacement};
use crate::seeded::Stream;
use shapes::film::View;
use shapes::film_geometry::{ImageRight, MarkerLetter};
use shapes::pixels::{Px, PxRect};

/// Lead is close to opaque: this much attenuation, through the letter.
const LEAD: f64 = 6.0;
/// The ruler's ticks are thinner lead.
const TICK: f64 = 4.0;
/// Steel, per mm.
const STEEL: f64 = 0.9;

/// The letters as 5 × 7 cells, top row first, `#` filled.
const LETTER_L: [&str; 7] = [
    "#....", "#....", "#....", "#....", "#....", "#....", "#####",
];
const LETTER_R: [&str; 7] = [
    "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
];

/// Where the lead letter was painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeadLetter {
    pub letter: MarkerLetter,
    /// Its box on the film, in film pixels.
    pub bounds: PxRect,
    /// Painted mirrored, as on a film shown the other way round.
    pub mirrored: bool,
}

/// Where the ruler was painted: the centres of its first and last ticks,
/// and how far apart they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerMarks {
    pub first: Px,
    pub last: Px,
    pub mm: f64,
}

/// Where the ball was painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub centre: Px,
    pub diameter_mm: f64,
    pub radius_px: f64,
}

/// Upright edges of the collimated field from the recipe's insets (left,
/// top, right, bottom as shares of the film), in mm; the whole film when
/// there are none.
fn field_mm(layout: &FilmLayout, insets: Option<[f64; 4]>) -> [f64; 4] {
    let mm = layout.camera.upright_mm_per_px[0];
    let [w, h] = layout.upright_size.map(|side| f64::from(side) * mm);
    let [left, top, right, bottom] = insets.unwrap_or([0.0; 4]);
    [left * w, top * h, w * (1.0 - right), h * (1.0 - bottom)]
}

/// Paints the lead letter inside the collimated field (`insets`, as the
/// recipe gives them): which letter, and which top corner on a side film,
/// come from `stream`.
pub fn paint_letter(
    canvas: &mut Canvas,
    placement: &MarkerPlacement,
    insets: Option<[f64; 4]>,
    stream: &mut Stream,
) -> LeadLetter {
    let layout = canvas.layout;
    let right = layout.camera.image_right;
    let front_view = layout.kind.view() == View::Front;
    let letter = if stream.chance(if front_view { 0.5 } else { 0.85 }) {
        MarkerLetter::L
    } else {
        MarkerLetter::R
    };
    let toward_screen_right = if front_view {
        // The letter sits on the side of the patient it names.
        (letter == MarkerLetter::L) == (right == ImageRight::PatientLeft)
    } else {
        stream.chance(0.5)
    };
    let mirrored = matches!(right, ImageRight::PatientRight | ImageRight::Posterior);
    let mm = layout.camera.upright_mm_per_px[0];
    let [width_px, height_px] = layout.upright_size.map(f64::from);
    let cell = placement.height_mm / 7.0;
    let (letter_w, letter_h) = (5.0 * cell, 7.0 * cell);
    let [field_left, field_top, field_right, _] = field_mm(layout, insets);
    let margin = 0.04 * width_px * mm;
    let left = if toward_screen_right {
        field_right - margin - letter_w
    } else {
        field_left + margin
    };
    let top = (placement.down * height_px * mm).max(field_top + 5.0);
    let rows = if letter == MarkerLetter::L {
        LETTER_L
    } else {
        LETTER_R
    };
    canvas.add_flat([left, top, left + letter_w, top + letter_h], |u, v| {
        let column = ((u - left) / cell).floor();
        let row = ((v - top) / cell).floor();
        if !(0.0..5.0).contains(&column) || !(0.0..7.0).contains(&row) {
            return 0.0;
        }
        let column = if mirrored {
            4 - column as usize
        } else {
            column as usize
        };
        if rows[row as usize].as_bytes()[column] == b'#' {
            LEAD
        } else {
            0.0
        }
    });
    LeadLetter {
        letter,
        bounds: film_rect(layout, [left, top, left + letter_w, top + letter_h]),
        mirrored,
    }
}

/// Paints the ruler down one side of the film, inside the collimated
/// field.
pub fn paint_ruler(
    canvas: &mut Canvas,
    placement: &RulerPlacement,
    insets: Option<[f64; 4]>,
) -> RulerMarks {
    let layout = canvas.layout;
    let mm = layout.camera.upright_mm_per_px[0];
    let film_h = f64::from(layout.upright_size[1]) * mm;
    let [field_left, field_top, field_right, field_bottom] = field_mm(layout, insets);
    let top = (placement.down * film_h).max(field_top + 5.0);
    let room = (field_bottom - 5.0 - top).max(10.0);
    let ticks = (placement.length_mm.min(room) / 10.0).floor().max(1.0) as u32 + 1;
    let edge = if placement.on_left {
        field_left + 6.0
    } else {
        field_right - 6.0
    };
    let outward = if placement.on_left { 1.0 } else { -1.0 };
    let length = 10.0 * f64::from(ticks - 1);
    let (low, high) = if placement.on_left {
        (edge, edge + 9.0)
    } else {
        (edge - 9.0, edge)
    };
    canvas.add_flat([low, top - 1.0, high, top + length + 1.0], |u, v| {
        let nearest = ((v - top) / 10.0).round();
        if !(0.0..f64::from(ticks)).contains(&nearest) || (v - top - 10.0 * nearest).abs() > 0.5 {
            return 0.0;
        }
        let long = if (nearest as u32).is_multiple_of(5) {
            9.0
        } else {
            5.0
        };
        let into = (u - edge) * outward;
        if (0.0..=long).contains(&into) {
            TICK
        } else {
            0.0
        }
    });
    let tick_centre = |n: f64| {
        let u = edge + outward * 2.5;
        upright_mm_to_film(layout, u, top + 10.0 * n)
    };
    RulerMarks {
        first: tick_centre(0.0),
        last: tick_centre(f64::from(ticks - 1)),
        mm: length,
    }
}

/// Paints the calibration ball, inside the collimated field.
pub fn paint_ball(
    canvas: &mut Canvas,
    placement: &BallPlacement,
    insets: Option<[f64; 4]>,
) -> Ball {
    let layout = canvas.layout;
    let mm = layout.camera.upright_mm_per_px[0];
    let film_w = f64::from(layout.upright_size[0]) * mm;
    let film_h = f64::from(layout.upright_size[1]) * mm;
    let radius = 0.5 * placement.diameter_mm;
    let [field_left, _, _, field_bottom] = field_mm(layout, insets);
    let cu = (placement.across * film_w).max(field_left + radius + 5.0);
    let cv = (placement.down * film_h).min(field_bottom - radius - 5.0);
    canvas.add_flat(
        [cu - radius, cv - radius, cu + radius, cv + radius],
        |u, v| {
            let rho_sq = (u - cu) * (u - cu) + (v - cv) * (v - cv);
            if rho_sq < radius * radius {
                STEEL * 2.0 * (radius * radius - rho_sq).sqrt()
            } else {
                0.0
            }
        },
    );
    Ball {
        centre: upright_mm_to_film(layout, cu, cv),
        diameter_mm: placement.diameter_mm,
        radius_px: radius / mm,
    }
}

/// An upright position in mm from the film's top-left pixel edge, as a
/// film pixel.
fn upright_mm_to_film(layout: &FilmLayout, u: f64, v: f64) -> Px {
    let mm = layout.camera.upright_mm_per_px[0];
    layout.upright_to_film(u / mm - 0.5, v / mm - 0.5)
}

/// An upright box in mm as a film box.
fn film_rect(layout: &FilmLayout, [left, top, right, bottom]: [f64; 4]) -> PxRect {
    let a = upright_mm_to_film(layout, left, top);
    let b = upright_mm_to_film(layout, right, bottom);
    PxRect {
        left: a.x().min(b.x()),
        top: a.y().min(b.y()),
        right: a.x().max(b.x()),
        bottom: a.y().max(b.y()),
    }
}
