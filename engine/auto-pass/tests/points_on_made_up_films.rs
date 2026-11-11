//! Point placement starts with the known chain, independently of tracing
//! and naming. The painter uses rounded solids with posterior elements;
//! the answer key uses projected endplate extrema.

use auto_pass::handoffs::{
    BodyCertainty, EndplateOnStrip, FoundBody, Polarity, Strip, StripEvidence, StripRow,
};
use auto_pass::points::{bending, front, side};
use darkroom::blur::gaussian;
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::radial_symmetry::{RingSettings, RingShape, Voters, ideal_responses, ring_map};
use darkroom::sampling::bilinear;
use made_up_spines::answer_key::{film_answer, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::film_painter::{PaintedFilm, paint};
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::film::FilmKind;
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{Mark, PedicleMark};
use shapes::pixels::Px;

const STEP_MM: f64 = 0.5;

fn difference(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}

fn dot(a: Px, b: Px) -> f64 {
    a.x() * b.x() + a.y() * b.y()
}

fn distance(a: Px, b: Px) -> f64 {
    let d = difference(a, b);
    libm::sqrt(dot(d, d))
}

fn midpoint(a: Px, b: Px) -> Px {
    Px::new((a.x() + b.x()) * 0.5, (a.y() + b.y()) * 0.5)
}

fn true_corners(marks: TrueMarks) -> [TrueMark; 4] {
    match marks {
        TrueMarks::Front {
            sup_l,
            sup_r,
            inf_l,
            inf_r,
            ..
        } => [sup_l, sup_r, inf_r, inf_l],
        TrueMarks::Side {
            sup_a,
            sup_p,
            inf_a,
            inf_p,
        } => [sup_a, sup_p, inf_p, inf_a],
        TrueMarks::Bending { sup, inf } => [sup[0], sup[1], inf[1], inf[0]],
    }
}

fn painted(
    name: RecipeName,
    kind: FilmKind,
    image_right: ImageRight,
    turns: u8,
    seed: u32,
) -> (PaintedFilm, FilmLayout, FilmAnswer) {
    let recipe = MadeUpRecipe {
        name,
        seed,
        tier: Tier::T0,
    };
    let patient = make_up_patient(recipe.name, recipe.seed).expect("the fixed recipe stacks");
    let spine = spine_for(&patient, kind);
    let mut film_recipe = film_recipes(&recipe, u64::from(recipe.seed))
        .into_iter()
        .find(|film| film.kind == kind)
        .expect("all four films");
    // Half-millimetre detector pixels keep this known-answer check small.
    // The painter and the independent answer key use this same camera.
    film_recipe.mm_per_px = STEP_MM;
    film_recipe.image_right = image_right;
    film_recipe.quarter_turns = QuarterTurns::new(turns).expect("zero to three turns");
    let layout = FilmLayout::new(&film_recipe, spine);
    let answer = film_answer(&layout, spine, &patient.recipe.template.levels());
    let film = paint(
        &film_recipe,
        &layout,
        &patient,
        spine,
        u64::from(recipe.seed),
    );
    (film, layout, answer)
}

/// A straight local section of the true chain. Its centre and endplate
/// tilts come from the answer key, but its wall prior is deliberately 12%
/// too wide: placement must find the film's walls to recover the corners.
fn strip_for_body(
    image: &GreyImage<u16>,
    layout: &FilmLayout,
    truth: &shapes::made_up::TrueVertebra,
) -> (Strip, FoundBody) {
    let corners = true_corners(truth.marks).map(|mark| mark.at);
    let sup = midpoint(corners[0], corners[1]);
    let inf = midpoint(corners[2], corners[3]);
    let height_px = distance(sup, inf);
    let along = Px::new(
        (inf.x() - sup.x()) / height_px,
        (inf.y() - sup.y()) / height_px,
    );
    let mut across = Px::new(along.y(), -along.x());
    let upright_right = difference(
        layout.upright_to_film(1.0, 0.0),
        layout.upright_to_film(0.0, 0.0),
    );
    if dot(across, upright_right) < 0.0 {
        across = Px::new(-across.x(), -across.y());
    }
    let mm_per_px = layout.geometry.mm_per_px.expect("known scale");
    let height = height_px * mm_per_px;
    let margin = 10.0;
    let rows = ((height + 2.0 * margin) / STEP_MM).ceil() as u32 + 1;
    let cols = 161;
    let pixels_per_sample = STEP_MM / mm_per_px;
    let half_width = 1.12
        * 0.25
        * mm_per_px
        * (distance(corners[0], corners[1]) + distance(corners[2], corners[3]));
    let layouts: Vec<StripRow> = (0..rows)
        .map(|row| {
            let offset_px = (f64::from(row) * STEP_MM - margin) / mm_per_px;
            StripRow {
                centre: Px::new(
                    sup.x() + along.x() * offset_px,
                    sup.y() + along.y() * offset_px,
                ),
                across: Px::new(
                    across.x() * pixels_per_sample,
                    across.y() * pixels_per_sample,
                ),
                along: Px::new(along.x() * pixels_per_sample, along.y() * pixels_per_sample),
                curvature: 0.0,
                half_width,
                z: -f64::from(row) * STEP_MM,
            }
        })
        .collect();
    let mut values = Vec::with_capacity((rows * cols) as usize);
    let mut valid = Vec::with_capacity(values.capacity());
    for row in &layouts {
        for column in 0..cols {
            let offset = f64::from(column) - f64::from(cols - 1) * 0.5;
            let point = Px::new(
                row.centre.x() + offset * row.across.x(),
                row.centre.y() + offset * row.across.y(),
            );
            let value = bilinear(image, point.x(), point.y()).filter(|_| layout.shows(point));
            valid.push(u8::from(value.is_some()));
            values.push(value.map_or(f32::NAN, |value| value / 65535.0));
        }
    }
    let plate = |a: Px, b: Px, row: f64| {
        let line = difference(a, b);
        // atan(down/across) keeps the line's tilt independent of which
        // endpoint the patient-facing names put first.
        EndplateOnStrip {
            row,
            tilt: libm::atan(dot(line, along) / dot(line, across)),
        }
    };
    let body = FoundBody {
        level: truth.level,
        sup: plate(corners[0], corners[1], margin / STEP_MM),
        inf: plate(corners[3], corners[2], (margin + height) / STEP_MM),
        inferred: false,
        certainty: BodyCertainty {
            posterior: 1.0,
            sup_spread: 0.0,
            inf_spread: 0.0,
            margin: 10.0,
        },
    };
    let strip = Strip {
        film: layout.kind,
        rows,
        cols,
        row_step: STEP_MM,
        col_step: STEP_MM,
        scale_assumed: false,
        values,
        valid,
        layout: layouts,
        image_right: layout.geometry.image_right,
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    };
    (strip, body)
}

fn placed_corners(strip: &Strip, body: &FoundBody) -> [Mark; 4] {
    match strip.film {
        FilmKind::Front => front::place(strip, body, Polarity::BoneBright)
            .expect("valid body")
            .corners(),
        FilmKind::Side => side::place(strip, body, Polarity::BoneBright)
            .expect("valid body")
            .corners(),
        FilmKind::BendLeft | FilmKind::BendRight => {
            bending::place(strip, body, Polarity::BoneBright)
                .expect("valid body")
                .corners()
        }
    }
}

#[test]
fn true_chains_place_corners_on_painted_films() {
    let configurations = [
        (FilmKind::Front, ImageRight::PatientLeft, 0),
        (FilmKind::Side, ImageRight::Posterior, 3),
        (FilmKind::BendLeft, ImageRight::PatientLeft, 2),
        (FilmKind::BendRight, ImageRight::PatientRight, 1),
    ];
    for (seed, (kind, facing, turns)) in [3, 7]
        .into_iter()
        .flat_map(|seed| configurations.map(|configuration| (seed, configuration)))
    {
        let (film, layout, answer) =
            painted(RecipeName::RightThoracic40, kind, facing, turns, seed);
        let mut squared_error = 0.0;
        let mut maximum_error = 0.0f64;
        let mut count = 0;
        let mut measured = 0;
        for truth in &answer.vertebrae {
            let (strip, body) = strip_for_body(&film.image, &layout, truth);
            let mut body_measured = 0;
            for (placed, wanted) in placed_corners(&strip, &body)
                .into_iter()
                .zip(true_corners(truth.marks))
            {
                if !wanted.visible {
                    continue;
                }
                if !placed.inferred {
                    measured += 1;
                    body_measured += 1;
                }
                let error = distance(placed.at, wanted.at) * STEP_MM;
                squared_error += error * error;
                maximum_error = maximum_error.max(error);
                count += 1;
                assert!(
                    error <= 4.0,
                    "seed {seed}, {kind:?} {:?}: corner error {error:.3} mm",
                    truth.level
                );
            }
            assert!(
                body_measured >= 2,
                "seed {seed}, {kind:?} {:?}: the body needs measured corners",
                truth.level
            );
        }
        // The half-mm grid contributes at most one pixel to localization;
        // the painter rounds its corners by about 1 mm and includes
        // overlapping posterior elements. These bounds permit that model
        // mismatch while still requiring wall recovery from a biased prior.
        let rms = libm::sqrt(squared_error / f64::from(count));
        assert_eq!(count, 72, "all four corners of C7 through L5 were checked");
        // The clean 3D painter can still obscure one edge with posterior
        // structures. Keep those marks honestly inferred, but require at
        // least eight ninths to be measured so the biased prior cannot pass.
        assert!(
            measured >= 64,
            "seed {seed}, {kind:?}: only {measured} measured corners"
        );
        assert!(
            rms <= 2.0,
            "seed {seed}, {kind:?}: RMS {rms:.3}, maximum {maximum_error:.3} mm"
        );
    }
}

#[test]
fn no_ring_channel_never_invents_pedicle_positions() {
    let (film, layout, answer) = painted(
        RecipeName::Severe90,
        FilmKind::Front,
        ImageRight::PatientRight,
        1,
        3,
    );
    let mut hidden = 0;
    for truth in &answer.vertebrae {
        let (strip, body) = strip_for_body(&film.image, &layout, truth);
        let marks = front::place(&strip, &body, Polarity::BoneBright).expect("valid body");
        let TrueMarks::Front { ped_l, ped_r, .. } = truth.marks else {
            unreachable!()
        };
        for (placed, wanted) in [marks.ped_l, marks.ped_r].into_iter().zip([ped_l, ped_r]) {
            assert!(matches!(placed, PedicleMark::NotVisible { .. }));
            assert!(placed.mark().is_none(), "not visible carries no position");
            hidden += usize::from(!wanted.visible);
        }
    }
    assert!(hidden > 0, "the recipe contains genuinely hidden pedicles");
}

/// The same physical ring bank and local contrast as the evidence maps,
/// computed without depending on the tracing stage under development.
fn ring_voters(image: &GreyImage<f32>) -> Option<Voters> {
    let mean = gaussian(image, 8.0 / STEP_MM)?;
    let squares = gaussian(&image.map(|value| value * value), 8.0 / STEP_MM)?;
    let contrast = mean
        .pixels()
        .iter()
        .zip(squares.pixels())
        .map(|(&mean, &square)| {
            let variance = (f64::from(square) - f64::from(mean) * f64::from(mean)).max(0.0);
            libm::sqrt(variance).max(1e-4) as f32
        })
        .collect();
    let contrast = GreyImage::new(image.width(), image.height(), contrast, image.placement())?;
    Voters::from_image(image, 0.6 / STEP_MM, &contrast)
}

fn ring_bank() -> (Vec<RingShape>, Vec<f64>) {
    let mut shapes = Vec::new();
    for width in [2.5f64, 3.5, 5.0, 7.0] {
        for aspect in [0.8, 1.25, 2.0] {
            shapes.push(
                RingShape::new(
                    width / STEP_MM,
                    width * aspect / STEP_MM,
                    (0.3 * width).clamp(1.0, 2.0) / STEP_MM,
                )
                .expect("ring with a hole"),
            );
        }
    }
    let ideals = ideal_responses(
        &shapes,
        &RingSettings::evidence(),
        0.5 / STEP_MM,
        ring_voters,
    )
    .expect("every ring calibrates");
    (shapes, ideals)
}

fn add_rings(strip: &mut Strip, shapes: &[RingShape], ideals: &[f64]) {
    let image = GreyImage::new(
        strip.cols,
        strip.rows,
        strip
            .values
            .iter()
            .map(|&value| if value.is_finite() { value } else { 0.0 })
            .collect(),
        Placement::FILM,
    )
    .expect("one value per cell");
    let voters = ring_voters(&image).expect("finite image");
    let rings =
        ring_map(&voters, shapes, ideals, &RingSettings::evidence()).expect("calibrated bank");
    strip.evidence.ring = Some(rings.response.pixels().to_vec());
}

#[test]
fn hidden_pedicles_on_a_painted_film_stay_without_positions() {
    let (film, layout, answer) = painted(
        RecipeName::Severe90,
        FilmKind::Front,
        ImageRight::PatientLeft,
        0,
        3,
    );
    let (shapes, ideals) = ring_bank();
    let mut hidden = 0;
    for truth in &answer.vertebrae {
        let TrueMarks::Front { ped_l, ped_r, .. } = truth.marks else {
            unreachable!()
        };
        if ped_l.visible && ped_r.visible {
            continue;
        }
        let (mut strip, body) = strip_for_body(&film.image, &layout, truth);
        add_rings(&mut strip, &shapes, &ideals);
        let marks = front::place(&strip, &body, Polarity::BoneBright).expect("valid body");
        for (placed, wanted) in [marks.ped_l, marks.ped_r].into_iter().zip([ped_l, ped_r]) {
            if wanted.visible {
                continue;
            }
            hidden += 1;
            assert!(
                placed.mark().is_none(),
                "{:?}: hidden pedicle must have no position",
                truth.level
            );
        }
    }
    assert!(hidden > 0, "the rendered chain has hidden pedicles");
}

#[test]
fn visible_rings_are_measured_and_follow_patient_names() {
    let (rows, cols) = (121, 161);
    let centre_col = f64::from(cols - 1) * 0.5;
    let ring_centres = [(centre_col - 20.0, 53.0), (centre_col + 20.0, 53.0)];
    let values = (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let (x, y) = (f64::from(col), f64::from(row));
            let body = (x - centre_col).abs() <= 30.0 && (40.0..=80.0).contains(&y);
            let ring = ring_centres.iter().any(|&(cx, cy)| {
                let (dx, dy) = ((x - cx) * STEP_MM, (y - cy) * STEP_MM);
                let outer = dx * dx / (3.5 * 3.5) + dy * dy / (4.375 * 4.375);
                let inner = dx * dx / (2.45 * 2.45) + dy * dy / (3.325 * 3.325);
                outer <= 1.0 && inner >= 1.0
            });
            0.05 + 0.25 * f32::from(u8::from(body)) + 0.6 * f32::from(u8::from(ring))
        })
        .collect();
    let image = GreyImage::new(cols, rows, values, Placement::FILM).expect("grid");
    let blurred = gaussian(&image, 0.5 / STEP_MM).expect("one-pixel blur");
    let mut strip = Strip {
        film: FilmKind::Front,
        rows,
        cols,
        row_step: STEP_MM,
        col_step: STEP_MM,
        scale_assumed: false,
        values: blurred.pixels().to_vec(),
        valid: vec![1; (rows * cols) as usize],
        layout: (0..rows)
            .map(|row| StripRow {
                centre: Px::new(centre_col, f64::from(row)),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                half_width: 15.0,
                z: -f64::from(row) * STEP_MM,
            })
            .collect(),
        image_right: Some(ImageRight::PatientLeft),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    };
    let body = FoundBody {
        level: Level::L3,
        sup: EndplateOnStrip {
            row: 40.0,
            tilt: 0.0,
        },
        inf: EndplateOnStrip {
            row: 80.0,
            tilt: 0.0,
        },
        inferred: false,
        certainty: BodyCertainty {
            posterior: 1.0,
            sup_spread: 0.0,
            inf_spread: 0.0,
            margin: 10.0,
        },
    };
    let (shapes, ideals) = ring_bank();
    add_rings(&mut strip, &shapes, &ideals);
    let marks = front::place(&strip, &body, Polarity::BoneBright).expect("valid body");
    for (placed, (x, y)) in [marks.ped_r, marks.ped_l].into_iter().zip(ring_centres) {
        let placed = placed.mark().expect("a complete bright ring is visible");
        // One mm permits detector-grid quantisation and half-maximum
        // centroiding; the drawn centres are fixed independently above.
        assert!(distance(placed.at, Px::new(x, y)) * STEP_MM <= 1.0);
        assert!(!placed.inferred);
    }
    strip.image_right = Some(ImageRight::PatientRight);
    let swapped = front::place(&strip, &body, Polarity::BoneBright).expect("valid body");
    assert_eq!(swapped.ped_l, marks.ped_r);
    assert_eq!(swapped.ped_r, marks.ped_l);
    assert_eq!(swapped.sup_l, marks.sup_r);
    assert_eq!(swapped.inf_l, marks.inf_r);

    // Brightness inversion changes the intensity sign, while the ring
    // channel still describes bone-bright evidence. The chosen chain
    // polarity must preserve the point positions.
    strip
        .values
        .iter_mut()
        .for_each(|value| *value = 1.0 - *value);
    let inverted = front::place(&strip, &body, Polarity::BoneDark).expect("valid inverted body");
    for (a, b) in inverted.corners().into_iter().zip(swapped.corners()) {
        // Subtracting f32 values from one changes their last bits; this
        // tolerance is one thousandth of a detector pixel, not a new
        // localization allowance.
        assert!(distance(a.at, b.at) <= 0.001);
    }
    assert_eq!(inverted.ped_l, swapped.ped_l);
    assert_eq!(inverted.ped_r, swapped.ped_r);
}
