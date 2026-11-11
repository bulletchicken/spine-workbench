//! Point placement on true chains, imperfect strips and complete finder
//! runs. The painter uses rounded solids with posterior elements; the
//! answer key uses projected endplate extrema.

use auto_pass::handoffs::{
    Barcode, BodyCertainty, CentreLine, CentreLineEnds, ChainBoundaries, EndplateOnStrip,
    FilmEvidence, FilmPassCache, FoundBody, Polarity, PolarityCheck, SettledChain, Strip,
    StripEvidence, StripRow,
};
use auto_pass::points::{bending, front, place_points, side};
use darkroom::blur::gaussian;
use darkroom::grey_image::{GreyImage, Placement};
use darkroom::radial_symmetry::{RingSettings, RingShape, Voters, ideal_responses, ring_map};
use darkroom::sampling::bilinear;
use film_reader::decoded_film::DecodedFilm;
use made_up_spines::answer_key::{film_answer, spine_for};
use made_up_spines::film_layout::FilmLayout;
use made_up_spines::film_painter::{PaintedFilm, paint};
use made_up_spines::patient::make_up_patient;
use made_up_spines::recipes::film_recipes;
use shapes::auto_pass::{Acquisition, ChainOptions, Population};
use shapes::confidence::{Confidence, ConfidenceBand};
use shapes::film::{
    DisplayWindow, Film, FilmFacts, FilmImage, FilmKind, PixelValues, SourceFormat, WindowFunction,
    WindowSource,
};
use shapes::film_geometry::{ImageRight, QuarterTurns};
use shapes::levels::Level;
use shapes::made_up::{FilmAnswer, MadeUpRecipe, RecipeName, Tier, TrueMark, TrueMarks};
use shapes::marks::{Mark, PedicleMark};
use shapes::naming::{ChosenBy, LevelNaming, Template};
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
    painted_at_tier(name, kind, image_right, turns, seed, Tier::T0)
}

fn painted_at_tier(
    name: RecipeName,
    kind: FilmKind,
    image_right: ImageRight,
    turns: u8,
    seed: u32,
    tier: Tier,
) -> (PaintedFilm, FilmLayout, FilmAnswer) {
    let recipe = MadeUpRecipe { name, seed, tier };
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

fn decoded_record(painted: PaintedFilm, layout: &FilmLayout) -> (DecodedFilm, Film) {
    let facts = FilmFacts {
        kind: painted.kind,
        width: painted.image.width(),
        height: painted.image.height(),
        bits_stored: painted.bits_stored,
        source: SourceFormat::Png,
        kept_tags: None,
        values: PixelValues::AS_STORED,
        flags: vec![],
        default_window: DisplayWindow::CentreWidth {
            centre: 32768.0,
            width: 65536.0,
            function: WindowFunction::Linear,
            from: WindowSource::Percentile,
        },
        exif_quarter_turns: None,
    };
    let record = Film {
        kind: painted.kind,
        image: FilmImage {
            path: painted.kind.png_path().to_owned(),
            width: facts.width,
            height: facts.height,
            bits_stored: facts.bits_stored,
        },
        source: facts.source,
        kept_tags: None,
        values: facts.values.clone(),
        flags: vec![],
        blanked: vec![],
        geometry: layout.geometry.clone(),
    };
    (
        DecodedFilm {
            image: painted.image,
            facts,
        },
        record,
    )
}

#[test]
fn complete_finder_keeps_body_outlines_inside_the_trunk() {
    let (painted, layout, truth) = painted_at_tier(
        RecipeName::RightThoracic40,
        FilmKind::Front,
        ImageRight::PatientLeft,
        0,
        7,
        Tier::T1,
    );
    let (decoded, record) = decoded_record(painted, &layout);
    let (film_result, cache) =
        auto_pass::run::film_pass(&decoded, &record).expect("the film traces");
    let answer = auto_pass::run::chain_pass(
        &[film_result],
        &ChainOptions {
            acquisition: Acquisition::Unknown,
            population: Population::Adolescent,
            template: None,
        },
    )
    .expect("the measured strip has a chain");
    let chain = &answer.films[0];
    let settled = SettledChain::from_bytes(&chain.chain).unwrap();
    let marks = auto_pass::run::place_points(&cache, chain, 100)
        .expect("one corner cannot discard the film");
    assert_eq!(marks.vertebrae.len(), settled.boundaries.bodies.len());
    assert!(!marks.vertebrae.is_empty());
    for body in &marks.vertebrae {
        let c = body.marks.corners().map(|mark| mark.at);
        let width = 0.5 * (distance(c[0], c[1]) + distance(c[2], c[3]));
        let height = distance(midpoint(c[0], c[1]), midpoint(c[2], c[3]));
        // 2.8 is the experimental front-view envelope derived from the
        // independent body-size table and its dimensional jitter. A little
        // room remains for tilted lines and the curved strip's mapping.
        assert!(
            width <= 3.0 * height,
            "{:?}: width {width}, height {height}",
            body.level
        );
        let centre = midpoint(midpoint(c[0], c[1]), midpoint(c[2], c[3]));
        let nearest = truth
            .vertebrae
            .iter()
            .min_by(|a, b| {
                let at = |body: &shapes::made_up::TrueVertebra| {
                    let c = true_corners(body.marks).map(|mark| mark.at);
                    distance(centre, midpoint(midpoint(c[0], c[1]), midpoint(c[2], c[3])))
                };
                at(a).total_cmp(&at(b))
            })
            .unwrap();
        let t = true_corners(nearest.marks).map(|mark| mark.at);
        let true_width = 0.5 * (distance(t[0], t[1]) + distance(t[2], t[3]));
        // This regression constrains the body rather than requiring the
        // imperfect chain to have its final heights or level names right.
        assert!(
            width < 1.75 * true_width,
            "{:?}: outline width {width}, true local width {true_width}",
            body.level
        );
    }
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
                    "seed {seed}, {kind:?} {:?}: corner error {error:.3} mm; placed {:?}, wanted {:?}, half width {}, height {}",
                    truth.level,
                    placed,
                    wanted,
                    strip.layout[0].half_width,
                    (body.inf.row - body.sup.row) * strip.row_step,
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

/// Three independent rectangles inside brighter, wider vertical bands.
/// The inner bodies are 30 mm wide and 20 mm tall. The 48 mm and 70 mm
/// bands model stronger distracting walls; their edges are known by
/// construction rather than read from a point-placement result.
fn crowded_strip() -> (Strip, Vec<FoundBody>) {
    crowded_strip_with_top_tilt(0.0)
}

fn crowded_strip_with_top_tilt(top_slope: f64) -> (Strip, Vec<FoundBody>) {
    let (rows, cols) = (251, 201);
    let step = |x: f64| 0.5 * (1.0 + libm::erf(x / (0.4 * libm::sqrt(2.0))));
    let mut values = Vec::with_capacity((rows * cols) as usize);
    for row in 0..rows {
        let y = f64::from(row) * STEP_MM;
        for col in 0..cols {
            let x = (f64::from(col) - 100.0) * STEP_MM;
            let bodies: f64 = [20.0, 55.0, 90.0]
                .into_iter()
                .enumerate()
                .map(|(index, top)| {
                    let slope = if index == 0 { top_slope } else { 0.0 };
                    step(y - top - slope * x) * step(top + 20.0 + slope * x - y)
                })
                .sum();
            let band = |half| step(x + half) * step(half - x);
            values.push(
                (0.05 + 0.35 * band(35.0) + 0.30 * band(24.0) + 0.25 * band(15.0) * bodies) as f32,
            );
        }
    }
    let strip = Strip {
        film: FilmKind::Front,
        rows,
        cols,
        row_step: STEP_MM,
        col_step: STEP_MM,
        scale_assumed: false,
        values,
        valid: vec![1; (rows * cols) as usize],
        layout: (0..rows)
            .map(|row| StripRow {
                centre: Px::new(100.0, f64::from(row)),
                across: Px::new(1.0, 0.0),
                along: Px::new(0.0, 1.0),
                curvature: 0.0,
                // One biased centre-line width must not drag the whole column
                // out to the outer band; adjacent bodies vote for its width.
                half_width: if (110..=150).contains(&row) {
                    30.0
                } else {
                    16.8
                },
                z: -f64::from(row) * STEP_MM,
            })
            .collect(),
        image_right: Some(ImageRight::PatientLeft),
        polarity_hint: Some(Polarity::BoneBright),
        evidence: StripEvidence::default(),
    };
    let bodies = [Level::C7, Level::T1, Level::T2]
        .into_iter()
        .zip([20.0, 55.0, 90.0])
        .map(|(level, top)| FoundBody {
            level,
            sup: EndplateOnStrip {
                row: top / STEP_MM,
                tilt: if level == Level::C7 {
                    libm::atan(top_slope)
                } else {
                    0.0
                },
            },
            inf: EndplateOnStrip {
                row: (top + 20.0) / STEP_MM,
                tilt: if level == Level::C7 {
                    libm::atan(top_slope)
                } else {
                    0.0
                },
            },
            inferred: false,
            certainty: BodyCertainty {
                posterior: 0.99,
                sup_spread: 0.2,
                inf_spread: 0.2,
                margin: 20.0,
            },
        })
        .collect();
    (strip, bodies)
}

fn cache_for_strip(strip: Strip) -> FilmPassCache {
    let image = GreyImage::new(
        strip.cols,
        strip.rows,
        strip.values.clone(),
        Placement::FILM,
    )
    .unwrap();
    let valid =
        GreyImage::new(strip.cols, strip.rows, strip.valid.clone(), Placement::FILM).unwrap();
    let zero = image.map(|_| 0.0f32);
    let samples = strip
        .layout
        .iter()
        .enumerate()
        .map(|(row, layout)| auto_pass::handoffs::CentreSample {
            arc: row as f64 * strip.row_step,
            at: layout.centre,
            along: Px::new(
                layout.along.x() / strip.row_step,
                layout.along.y() / strip.row_step,
            ),
            curvature: layout.curvature,
            width: 2.0 * layout.half_width,
            width_measured: true,
            evidence: 1.0,
        })
        .collect();
    FilmPassCache {
        evidence: FilmEvidence {
            film: strip.film,
            mm_per_px: STEP_MM,
            scale_assumed: false,
            valid: valid.clone(),
            image,
            phase_edge: zero.clone(),
            phase_direction: zero.clone(),
            phase_kind: zero.clone(),
            gradient_edge: zero.clone(),
            gradient_direction: zero.clone(),
            ridge: zero.clone(),
            ridge_orientation: zero.clone(),
            valley: zero.clone(),
            valley_orientation: zero.clone(),
            valley_scale: valid.map(|_| 0),
            ridge_symmetry: zero.clone(),
            valley_symmetry: zero,
            rings: None,
            hessian_scales_mm: vec![],
            polarity: PolarityCheck {
                found: Polarity::BoneBright,
                confidence: 1.0,
                needs_confirm: false,
            },
            markers: vec![],
            seam_rows: vec![],
            valid_fraction: 1.0,
            coverage: 1.0,
            warnings: vec![],
        },
        centre_line: CentreLine {
            film: strip.film,
            quarter_turns: QuarterTurns::default(),
            scale_assumed: false,
            sample_step: strip.row_step,
            samples,
            ends: CentreLineEnds {
                neck_base: None,
                iliac_crest: None,
                top_cropped: false,
                bottom_cropped: false,
                size_factor: 1.0,
            },
            warnings: vec![],
        },
        barcode: Barcode {
            film: strip.film,
            values: vec![],
            high_pass: vec![],
            valid: vec![],
        },
        strip,
    }
}

fn settled(bodies: Vec<FoundBody>) -> SettledChain {
    let confidence = bodies
        .iter()
        .map(|_| Confidence {
            score: 0.99,
            band: ConfidenceBand::High,
            reasons: vec![],
            calibrated: false,
            endplate_spread_px: None,
            margin: Some(20.0),
        })
        .collect();
    SettledChain {
        boundaries: ChainBoundaries {
            film: FilmKind::Front,
            template: Template::Standard,
            polarity: Polarity::BoneBright,
            bodies,
            sacrum: None,
            energy: 0.0,
            warnings: vec![],
        },
        confidence,
    }
}

fn names_for(chain: &SettledChain) -> LevelNaming {
    let names: Vec<_> = chain
        .boundaries
        .bodies
        .iter()
        .map(|body| body.level)
        .collect();
    LevelNaming {
        template: chain.boundaries.template,
        chosen_by: ChosenBy::Engine,
        alternatives: vec![],
        from_top: Some(names.clone()),
        from_bottom: Some(names),
        flags: vec![],
        l5_confirmed_on: None,
    }
}

#[test]
fn neighbouring_bodies_keep_walls_inside_stronger_outer_bands() {
    let (strip, bodies) = crowded_strip();
    let standalone = front::place(&strip, &bodies[1], Polarity::BoneBright).unwrap();
    // This fixture needs neighbouring bodies: its biased local prior
    // alone selects the stronger 48 mm band, over 25% beyond the true
    // 30 mm width. A height cap or a local inner-pair preference alone
    // therefore cannot satisfy the independently known corners below.
    assert!(distance(standalone.sup_l.at, standalone.sup_r.at) * STEP_MM > 1.25 * 30.0);
    let chain = settled(bodies);
    let marks = place_points(&cache_for_strip(strip), &chain, names_for(&chain), 100).unwrap();
    assert_eq!(marks.vertebrae.len(), 3);
    for body in &marks.vertebrae {
        let c = body.marks.corners();
        // 1 mm is two detector pixels: enough for the independently
        // blurred edge while excluding either distracting outer wall.
        for (mark, x) in c.into_iter().zip([130.0, 70.0, 70.0, 130.0]) {
            assert!(
                (mark.at.x() - x).abs() * STEP_MM <= 1.0,
                "{:?}: {:?}",
                body.level,
                mark.at
            );
        }
    }
}

#[test]
fn tilted_wall_intersections_obey_the_body_width_envelope() {
    let (mut strip, mut bodies) = crowded_strip();
    bodies.truncate(1);
    let tilt = 35.0_f64.to_radians();
    bodies[0].sup = EndplateOnStrip { row: 110.0, tilt };
    bodies[0].inf = EndplateOnStrip { row: 150.0, tilt };
    let edge = |x: f64| 0.5 * (1.0 + libm::erf(x / (0.4 * libm::sqrt(2.0))));
    for row in 0..strip.rows {
        let y = f64::from(row) * STEP_MM;
        strip.layout[row as usize].half_width = 28.0;
        for col in 0..strip.cols {
            let x = (f64::from(col) - 100.0) * STEP_MM;
            let drift = 0.15 * (y - 65.0);
            let walls = edge(x + 26.5 - drift) * edge(26.5 + drift - x);
            let plates =
                edge(y - 55.0 - x * libm::tan(tilt)) * edge(75.0 + x * libm::tan(tilt) - y);
            strip.values[(row * strip.cols + col) as usize] = (0.05 + 0.9 * walls * plates) as f32;
        }
    }
    // The distractor's parallel walls are 53 mm apart at the middle,
    // below 2.8 * 20 mm. Their intersection with tilted plates is wider:
    // 53 / (1 - 0.15*tan(35°)) laterally, or about 72.3 mm along a plate.
    // Bounding only the middle offsets therefore still admits a wide quad.
    let chain = settled(bodies);
    let marks = place_points(&cache_for_strip(strip), &chain, names_for(&chain), 100).unwrap();
    let c = marks.vertebrae[0].marks.corners().map(|mark| mark.at);
    let width = distance(c[0], c[1]).max(distance(c[2], c[3]));
    let height = distance(midpoint(c[0], c[1]), midpoint(c[2], c[3]));
    assert!(width <= 2.8 * height, "width {width}, height {height}");
}

#[test]
fn a_folded_corner_is_inferred_without_losing_the_chain() {
    let (strip, bodies) = crowded_strip();
    let chain = settled(bodies);
    let normal = place_points(
        &cache_for_strip(strip.clone()),
        &chain,
        names_for(&chain),
        100,
    )
    .unwrap();
    let mut folded = strip;
    // Only the superior patient-left corner of the middle body enters a
    // fold: 1 - curvature * lateral_distance < 0.2 at x=+15 mm.
    for row in 108..=112 {
        folded.layout[row].curvature = 0.08;
    }
    let repaired = place_points(&cache_for_strip(folded), &chain, names_for(&chain), 100)
        .expect("one corner cannot refuse a film");
    assert_eq!(repaired.vertebrae.len(), 3);
    let middle = &repaired.vertebrae[1];
    let corners = middle.marks.corners();
    assert!(corners[0].inferred);
    assert!(middle.confidence.score < normal.vertebrae[1].confidence.score);
    assert!(middle.confidence.band < ConfidenceBand::High);
    assert!(distance(corners[0].at, Px::new(130.0, 110.0)) * STEP_MM <= 1.0);
    for (a, b) in corners[1..]
        .iter()
        .zip(&normal.vertebrae[1].marks.corners()[1..])
    {
        assert!(distance(a.at, b.at) * STEP_MM <= 0.25);
    }
    for index in [0, 2] {
        assert_eq!(
            repaired.vertebrae[index].confidence,
            normal.vertebrae[index].confidence
        );
        for (a, b) in repaired.vertebrae[index]
            .marks
            .corners()
            .iter()
            .zip(normal.vertebrae[index].marks.corners())
        {
            assert!(distance(a.at, b.at) * STEP_MM <= 0.25);
        }
    }
}

#[test]
fn a_corner_above_the_strip_is_inferred_in_the_original_film() {
    let (strip, mut bodies) = crowded_strip_with_top_tilt(0.2);
    let whole = settled(bodies.clone());
    let mut cache = cache_for_strip(strip);
    let normal = place_points(&cache, &whole, names_for(&whole), 100).unwrap();
    // The first body's upper corners are at (70, 34) and (130, 46)
    // original pixels. Starting the strip at row 36 hides only the first;
    // its remaining three corners and both neighbouring bodies survive.
    let removed = 36;
    let strip = &mut cache.strip;
    strip.values.drain(..removed * strip.cols as usize);
    strip.valid.drain(..removed * strip.cols as usize);
    strip.layout.drain(..removed);
    strip.rows -= removed as u32;
    for body in &mut bodies {
        body.sup.row -= removed as f64;
        body.inf.row -= removed as f64;
    }
    let cropped = settled(bodies);
    let marks = place_points(&cache, &cropped, names_for(&cropped), 100)
        .expect("one corner outside the strip cannot refuse a film");
    assert_eq!(marks.vertebrae.len(), 3);
    let first = &marks.vertebrae[0];
    let corners = first.marks.corners();
    assert!(corners[1].inferred);
    assert!(first.confidence.score < normal.vertebrae[0].confidence.score);
    // One millimetre permits the independently blurred detector edge;
    // the tilted parallelogram gives the missing corner without a fit.
    assert!(distance(corners[1].at, Px::new(70.0, 34.0)) * STEP_MM <= 1.0);
    for index in [0, 2, 3] {
        assert!(
            distance(
                corners[index].at,
                normal.vertebrae[0].marks.corners()[index].at
            ) * STEP_MM
                <= 0.5
        );
    }
}

#[test]
fn unplaceable_bodies_do_not_discard_the_placeable_ones() {
    let (strip, mut bodies) = crowded_strip();
    bodies[0].inf.row = bodies[0].sup.row - 1.0;
    let mut chain = settled(bodies);
    let cache = cache_for_strip(strip);
    let kept =
        place_points(&cache, &chain, names_for(&chain), 100).expect("two bodies remain placeable");
    assert_eq!(
        kept.vertebrae
            .iter()
            .map(|body| body.level)
            .collect::<Vec<_>>(),
        vec![Level::T1, Level::T2]
    );
    assert_eq!(
        kept.vertebrae
            .iter()
            .map(|body| body.id)
            .collect::<Vec<_>>(),
        vec![100, 101]
    );
    assert_eq!(kept.naming.from_top, Some(vec![Level::T1, Level::T2]));
    assert_eq!(kept.naming.from_bottom, kept.naming.from_top);
    assert!(kept.landmarks.c7.is_none());
    for body in &mut chain.boundaries.bodies {
        body.inf.row = body.sup.row - 1.0;
    }
    assert!(place_points(&cache, &chain, names_for(&chain), 100).is_err());
}
