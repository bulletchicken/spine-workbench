//! Stage 1, evidence maps: from one film to the maps every later stage reads
//! (phase edges, a gradient fallback, ridges and disc-gap valleys, phase
//! symmetry, and pedicle rings on front films), on one working grid, with a
//! validity mask, the polarity check and the noise floor. Hands on a
//! `FilmEvidence`.

pub mod edges;
pub mod pedicle_rings;
pub mod ridges;

mod prepare;

use crate::handoffs::{FilmEvidence, Polarity};
use darkroom::grey_image::GreyImage;
use darkroom::log_gabor::Spectrum;
use darkroom::phase_congruency::{MonogenicRun, NoiseMode, PhaseSettings};
use darkroom::preview;
use film_reader::decoded_film::DecodedFilm;
use shapes::auto_pass::{PassWarning, PassWarningCode, Stage};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmKind, SourceFormat, TransferSyntax};
use shapes::film_geometry::FilmGeometry;
use shapes::pixels::PxRect;
use shapes::watch_frames::{MapKind, PreviewTile, WatchFrame};
use std::f64::consts::PI;

/// Build the frozen evidence hand-off without allocating watch previews.
/// The decoded film is already bone bright. An explicit override describes
/// its pixels, before this stage's private working copy is normalised.
pub fn build(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    polarity_override: Option<Polarity>,
) -> Result<FilmEvidence, EngineError> {
    compute(film, geometry, polarity_override, &[], false).map(|(evidence, _)| evidence)
}

/// Build the same maps and their fixed, ordered watch checkpoints. This is
/// the native stage entry point; the film-pass stepper owns scheduling.
pub fn build_with_checkpoints(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    polarity_override: Option<Polarity>,
) -> Result<(FilmEvidence, Vec<WatchFrame>), EngineError> {
    compute(film, geometry, polarity_override, &[], true)
}

/// Read the case's current brightness, geometry and privacy exclusions.
/// The desk's original decoded pixels remain unchanged. The returned
/// polarity also tells unrolling how to sample those original pixels.
pub fn build_for_film(
    decoded: &DecodedFilm,
    film: &Film,
) -> Result<(FilmEvidence, Vec<WatchFrame>), EngineError> {
    if decoded.facts.kind != film.kind
        || decoded.image.width() != film.image.width
        || decoded.image.height() != film.image.height
    {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "The film record does not match the decoded pixels",
        ));
    }
    let polarity = if decoded.facts.values.inverted != film.values.inverted {
        Some(Polarity::BoneDark)
    } else if film.geometry.confirmed.polarity {
        Some(Polarity::BoneBright)
    } else {
        None
    };
    compute(decoded, &film.geometry, polarity, &film.blanked, true)
}

fn compute(
    film: &DecodedFilm,
    geometry: &FilmGeometry,
    polarity_override: Option<Polarity>,
    blanked: &[PxRect],
    watch: bool,
) -> Result<(FilmEvidence, Vec<WatchFrame>), EngineError> {
    let prepared = if blanked.is_empty() {
        prepare::prepare(film, geometry, polarity_override)?
    } else {
        prepare::prepare_with_exclusions(film, geometry, polarity_override, blanked)?
    };
    let spacing = prepared.mm_per_px;
    let assumed = prepared.scale_assumed;
    let ring_sizes = pedicle_rings::sizes(assumed);
    let rings_on = film.facts.kind != FilmKind::Side && spacing <= 0.6;
    let mut scales: Vec<f64> = vec![0.75, 1.25, 2.0, 3.0, 4.5];
    if assumed {
        scales.insert(0, 0.75 / 1.6);
        scales.push(4.5 * 1.6);
    }
    // The metadata records the actual physical scales after the 1 px floor.
    for mm in &mut scales {
        *mm = (*mm).max(spacing);
    }
    let steps = 8
        + scales.len() as u32
        + 2
        + if rings_on {
            ring_sizes.len() as u32 + 1
        } else {
            0
        };
    let mut frames = Frames::new(film.facts.kind, steps, watch);
    frames.unit(MapKind::Image, true, None, &prepared.image);
    if watch {
        let valid = darkroom::pyramid::block_reduce(
            &prepared.valid,
            preview::decimation(prepared.valid.width(), prepared.valid.height()),
            |values| values.iter().map(|&v| usize::from(v)).sum::<usize>() * 2 > values.len(),
        )
        .expect("nonempty working grid");
        frames.push(MapKind::Validity, true, None, preview::mask_tile(&valid));
    }
    let spectrum = Spectrum::of(&prepared.image).ok_or_else(|| {
        EngineError::new(
            ErrorCode::PassFlatFilm,
            "The working film has no finite spectrum",
        )
    })?;
    let mut shortest: f64 = 1.5 / if assumed { 1.6 } else { 1.0 };
    let mut warnings = prepared.warnings;
    let native_x = geometry
        .mm_per_px
        .unwrap_or(spacing / prepared.image.placement().step_across.x());
    let native_spacing = native_x.max(geometry.mm_per_px_vertical.unwrap_or(native_x));
    let jpeg = film.facts.source == SourceFormat::Jpeg
        || film
            .facts
            .kept_tags
            .as_ref()
            .and_then(|tags| tags.transfer_syntax)
            .is_some_and(|syntax| {
                matches!(
                    syntax,
                    TransferSyntax::JpegBaseline | TransferSyntax::JpegExtended
                )
            });
    if jpeg && spacing / native_spacing < 2.0 {
        shortest = shortest.max(16.0 * native_spacing);
        warnings.push(warning(film.facts.kind, PassWarningCode::JpegBlocks));
    }
    if spacing > 0.6 {
        warnings.push(warning(film.facts.kind, PassWarningCode::CoarseFilm));
    }
    let settings = PhaseSettings {
        shortest_wavelength: (shortest / spacing).max(3.0),
        congruency_scales: 4 + usize::from(assumed),
        symmetry_scales: 5 + usize::from(assumed),
        noise: NoiseMode::Tiles {
            side: (32.0 / spacing).round().max(1.0) as u32,
        },
        ..PhaseSettings::EVIDENCE
    };
    let mut run =
        MonogenicRun::new(&spectrum, &settings, Some(&prepared.valid)).ok_or_else(|| {
            EngineError::new(
                ErrorCode::PassNoSpineFound,
                "The phase filters could not be prepared",
            )
        })?;
    while run.step() {
        if watch
            && matches!(run.scales_done(), 2 | 3)
            && let Some(mut edge) = run.provisional_edge()
        {
            mask(&mut edge, &prepared.valid);
            frames.unit(MapKind::PhaseEdge, false, None, &edge);
        }
    }
    let mut phase = run.finish();
    drop(phase.noise_level);
    for map in [
        &mut phase.edge,
        &mut phase.direction,
        &mut phase.kind,
        &mut phase.ridge_symmetry,
        &mut phase.valley_symmetry,
    ] {
        mask(map, &prepared.valid);
    }
    frames.unit(MapKind::PhaseEdge, true, None, &phase.edge);
    frames.unit(MapKind::RidgeSymmetry, true, None, &phase.ridge_symmetry);
    frames.unit(MapKind::ValleySymmetry, true, None, &phase.valley_symmetry);
    // Release the ring voting scratch before retaining the Hessian maps.
    // Only small preview tiles wait here; their displayed order stays fixed.
    let mut ring_tiles = Vec::new();
    let rings = if rings_on {
        Some(pedicle_rings::compute(
            &spectrum,
            &prepared.contrast,
            &prepared.valid,
            spacing,
            ring_sizes.clone(),
            geometry.quarter_turns.get(),
            |size, response| {
                if watch {
                    let small = preview::decimated(response).expect("nonempty working grid");
                    ring_tiles.push((size, preview::unit_tile(&small)));
                }
            },
        )?)
    } else {
        None
    };
    let mut hessian_tiles = Vec::new();
    let mut hessian = ridges::compute(
        &spectrum,
        &prepared.contrast,
        &prepared.valid,
        spacing,
        &scales,
        |index, ridge, valley| {
            if watch {
                let ridge = preview::decimated(ridge).expect("nonempty working grid");
                let valley = preview::decimated(valley).expect("nonempty working grid");
                hessian_tiles.push((
                    index,
                    preview::signed_pair_tile(&ridge, &valley).expect("same grids"),
                ));
            }
        },
    );
    let (gradient_edge, mut gradient_direction) = edges::gradient_maps(
        edges::voters(
            &spectrum,
            &prepared.contrast,
            &prepared.valid,
            (1.0 / spacing).max(1.0),
        ),
        &prepared.valid,
    );
    frames.unit(MapKind::GradientEdge, true, None, &gradient_edge);
    for (index, tile) in hessian_tiles {
        frames.push(MapKind::HessianScale, false, Some(scales[index]), tile);
    }
    frames.unit(MapKind::Ridge, true, None, &hessian.ridge);
    frames.unit(MapKind::Valley, true, None, &hessian.valley);
    for (size, tile) in ring_tiles {
        frames.push(MapKind::Ring, false, Some(ring_sizes[size]), tile);
    }
    if let Some(rings) = &rings {
        frames.unit(MapKind::Ring, true, None, &rings.response);
    }
    // Gradient and line normals are covectors: inverse-transpose of the
    // working placement, then a half-turn fold for an unoriented normal.
    for (map, axial) in [
        (&mut phase.direction, false),
        (&mut gradient_direction, false),
        (&mut hessian.ridge_orientation, true),
        (&mut hessian.valley_orientation, true),
    ] {
        film_angles(map, &prepared.valid, axial);
    }
    let usable = prepared.valid.pixels().iter().filter(|&&v| v != 0).count();
    let coverage = phase
        .edge
        .pixels()
        .iter()
        .zip(prepared.valid.pixels())
        .filter(|(e, v)| **v != 0 && **e > 0.1)
        .count() as f64
        / usable.max(1) as f64;
    if coverage < 0.01 {
        warnings.push(warning(film.facts.kind, PassWarningCode::LowEvidence));
    }
    if watch {
        frames
            .frames
            .extend(warnings.iter().cloned().map(|warning| WatchFrame::Warning {
                film: film.facts.kind,
                stage: Stage::Evidence,
                warning,
            }));
        debug_assert_eq!(frames.step, frames.steps);
    }
    Ok((
        FilmEvidence {
            film: film.facts.kind,
            mm_per_px: spacing,
            scale_assumed: assumed,
            valid: prepared.valid,
            image: prepared.image,
            phase_edge: phase.edge,
            phase_direction: phase.direction,
            phase_kind: phase.kind,
            gradient_edge,
            gradient_direction,
            ridge: hessian.ridge,
            ridge_orientation: hessian.ridge_orientation,
            valley: hessian.valley,
            valley_orientation: hessian.valley_orientation,
            valley_scale: hessian.valley_scale,
            ridge_symmetry: phase.ridge_symmetry,
            valley_symmetry: phase.valley_symmetry,
            rings,
            hessian_scales_mm: scales,
            polarity: prepared.polarity,
            markers: prepared.markers,
            seam_rows: prepared.seam_rows,
            valid_fraction: prepared.valid_fraction,
            coverage,
            warnings,
        },
        frames.frames,
    ))
}

pub(super) fn like<T, U>(template: &GreyImage<T>, pixels: Vec<U>) -> GreyImage<U> {
    GreyImage::new(
        template.width(),
        template.height(),
        pixels,
        template.placement(),
    )
    .expect("one value per grid cell")
}

pub(super) fn mask(map: &mut GreyImage<f32>, valid: &GreyImage<u8>) {
    *map = like(
        map,
        map.pixels()
            .iter()
            .zip(valid.pixels())
            .map(|(&value, &valid)| if valid == 0 { 0.0 } else { value })
            .collect(),
    );
}

fn film_angles(map: &mut GreyImage<f32>, valid: &GreyImage<u8>, axial: bool) {
    let p = map.placement();
    let a = p.step_across;
    let b = p.step_down;
    if a.y() == 0.0 && b.x() == 0.0 && a.x() == b.y() && a.x() > 0.0 {
        return;
    }
    let determinant = a.x() * b.y() - a.y() * b.x();
    *map = like(
        map,
        map.pixels()
            .iter()
            .zip(valid.pixels())
            .map(|(&theta, &v)| {
                if v == 0 {
                    return 0.0;
                }
                let (x, y) = (libm::cos(f64::from(theta)), libm::sin(f64::from(theta)));
                let mut angle = libm::atan2(
                    (a.x() * y - b.x() * x) / determinant,
                    (b.y() * x - a.y() * y) / determinant,
                );
                if axial {
                    angle -= PI * ((angle + PI / 2.0) / PI).floor();
                } else if angle <= -PI {
                    angle = PI;
                }
                angle as f32
            })
            .collect(),
    );
}

fn warning(film: FilmKind, code: PassWarningCode) -> PassWarning {
    PassWarning {
        code,
        film,
        level: None,
        region: None,
    }
}

struct Frames {
    film: FilmKind,
    steps: u32,
    step: u32,
    enabled: bool,
    frames: Vec<WatchFrame>,
}

impl Frames {
    fn new(film: FilmKind, steps: u32, enabled: bool) -> Self {
        Self {
            film,
            steps,
            step: 0,
            enabled,
            frames: if enabled {
                vec![WatchFrame::StageStarted {
                    film,
                    stage: Stage::Evidence,
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn unit(&mut self, map: MapKind, is_final: bool, scale: Option<f64>, image: &GreyImage<f32>) {
        if self.enabled {
            let small = preview::decimated(image).expect("nonempty working grid");
            self.push(map, is_final, scale, preview::unit_tile(&small));
        }
    }

    fn push(&mut self, map: MapKind, is_final: bool, scale_mm: Option<f64>, tile: PreviewTile) {
        self.step += 1;
        self.frames.push(WatchFrame::Evidence {
            film: self.film,
            stage: Stage::Evidence,
            map,
            step: self.step,
            steps: self.steps,
            is_final,
            scale_mm,
            tile,
        });
    }
}
