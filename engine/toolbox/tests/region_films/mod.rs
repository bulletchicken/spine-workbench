//! Analytic rectangular bodies painted independently of the wand. Edges are
//! half a pixel outside the last covered centre; answer corners use those
//! exact edges. No segmentation or fitting routine paints these fixtures.

use darkroom::grey_image::{GreyImage, Placement};
use film_reader::decoded_film::DecodedFilm;
use shapes::{
    confidence::{Confidence, ConfidenceBand},
    film::{
        DisplayWindow, FilmFacts, FilmKind, PixelValues, SourceFormat, WindowFunction, WindowSource,
    },
    film_geometry::ImageRight,
    landmarks::Landmarks,
    marks::{Mark, SideMarks},
    naming::{ChosenBy, LevelNaming, Template},
    pixels::Px,
    vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm},
};

pub fn fixture(
    width: u32,
    height: u32,
    body_height: f64,
    body_width: f64,
    angle: f64,
    inverted: bool,
) -> (DecodedFilm, FilmMarks, Vec<[Px; 4]>) {
    let mut pixels = vec![1000u16; (width * height) as usize];
    let mut vertebrae = Vec::new();
    let mut truth = Vec::new();
    let sin = libm::sin(angle);
    let cos = libm::cos(angle);
    let levels = Template::Standard.levels();
    for (i, &level) in levels.iter().enumerate() {
        let cx = f64::from(width) / 2.;
        let cy = (f64::from(height) - (levels.len() - 1) as f64 * body_height * 1.6) / 2.
            + i as f64 * body_height * 1.6;
        let map = |u: f64, v: f64| Px::new(cx + cos * u - sin * v, cy + sin * u + cos * v);
        let q = [
            map(-body_width / 2., -body_height / 2.),
            map(body_width / 2., -body_height / 2.),
            map(body_width / 2., body_height / 2.),
            map(-body_width / 2., body_height / 2.),
        ];
        // Independently rasterise in the body's local frame, with 4x4 coverage.
        let radius = (body_width + body_height) / 2.;
        for y in (cy - radius).floor().max(0.) as u32
            ..=(cy + radius).ceil().min(f64::from(height - 1)) as u32
        {
            for x in (cx - radius).floor().max(0.) as u32
                ..=(cx + radius).ceil().min(f64::from(width - 1)) as u32
            {
                let mut covered = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let dx = f64::from(x) + (f64::from(sx) + 0.5) / 4. - 0.5 - cx;
                        let dy = f64::from(y) + (f64::from(sy) + 0.5) / 4. - 0.5 - cy;
                        let u = cos * dx + sin * dy;
                        let v = -sin * dx + cos * dy;
                        if u.abs() < body_width / 2. && v.abs() < body_height / 2. {
                            covered += 1;
                        }
                    }
                }
                if covered > 0 {
                    pixels[(y * width + x) as usize] = 1000 + covered * 3000;
                }
            }
        }
        truth.push(q);
        vertebrae.push(VertebraOnFilm {
            id: i as u32 + 1,
            level,
            marks: VertebraMarks::Side {
                points: SideMarks {
                    sup_a: Mark::auto(q[0]),
                    sup_p: Mark::auto(q[1]),
                    inf_p: Mark::auto(q[2]),
                    inf_a: Mark::auto(q[3]),
                },
            },
            confidence: Confidence {
                score: 0.95,
                band: ConfidenceBand::High,
                reasons: vec![],
                calibrated: false,
                endplate_spread_px: None,
                margin: None,
            },
            outline: None,
        });
    }
    if inverted {
        for p in &mut pixels {
            *p = u16::MAX - *p;
        }
    }
    let film = DecodedFilm {
        image: GreyImage::new(width, height, pixels, Placement::FILM).unwrap(),
        facts: FilmFacts {
            kind: FilmKind::Side,
            width,
            height,
            bits_stored: 16,
            source: SourceFormat::Png,
            kept_tags: None,
            values: PixelValues::AS_STORED,
            flags: vec![],
            default_window: DisplayWindow::CentreWidth {
                centre: 32768.,
                width: 65535.,
                function: WindowFunction::Linear,
                from: WindowSource::Percentile,
            },
            exif_quarter_turns: None,
        },
    };
    let mut marks = FilmMarks {
        film: FilmKind::Side,
        vertebrae,
        landmarks: Landmarks::NONE,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: vec![],
            from_top: Some(levels.to_vec()),
            from_bottom: Some(levels.to_vec()),
            flags: vec![],
            l5_confirmed_on: None,
        },
        named_for: ImageRight::Anterior,
    };
    marks.landmarks.c7 = Landmarks::c7_of(&marks.vertebrae);
    (film, marks, truth)
}

pub fn perturb(marks: &mut FilmMarks, index: usize, shift: f64, top: bool) {
    let q = marks.vertebrae[index].marks.corners().map(|m| m.at);
    let dx = (q[3].x() - q[0].x()) * shift;
    let dy = (q[3].y() - q[0].y()) * shift;
    if let VertebraMarks::Side { points } = &mut marks.vertebrae[index].marks {
        let ends = if top {
            [&mut points.sup_a, &mut points.sup_p]
        } else {
            [&mut points.inf_a, &mut points.inf_p]
        };
        for mark in ends {
            mark.at = Px::new(mark.at.x() + dx, mark.at.y() + dy);
        }
    }
}
pub fn midpoint(a: Px, b: Px) -> Px {
    Px::new((a.x() + b.x()) * 0.5, (a.y() + b.y()) * 0.5)
}
