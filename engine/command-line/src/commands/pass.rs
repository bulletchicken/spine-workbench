//! `spine pass KIND=PATH...`: runs the whole auto pass on one to four films
//! with no browser, and prints every film's marks as JSON. With
//! `--front-finder trained`, the trained finder reads the front film and
//! the hand-built stages the rest, joined as the app joins them.

use crate::{Acquisition, Outcome, PassArgs, Population};
use film_reader::decoded_film::DecodedFilm;
use scorekeeper::datasets::ScoringFilm;
use shapes::auto_pass::{AutoPassInput, AutoPassResult, ChainOptions, FrontFinder, PassWarning};
use shapes::errors::{EngineError, ErrorCode};
use shapes::film::{Film, FilmImage, FilmKind};
use shapes::film_geometry::FilmGeometry;
use shapes::vertebra::FilmMarks;
use std::collections::BTreeSet;
use std::fs;

pub fn run(args: &PassArgs) -> Outcome {
    if args.films.is_empty() || args.films.len() > 4 {
        return Err("the pass needs one to four films".into());
    }
    let has_side = args.films.iter().any(|film| film.kind == FilmKind::Side);
    if !has_side && (args.side_finder.is_some() || args.side_facing.is_some()) {
        return Err("side-specific flags require side=PATH".into());
    }
    if matches!(args.side_finder, Some(crate::FrontFinder::Trained)) && args.side_facing.is_none() {
        return Err(
            "pass-facing-required: --side-finder trained requires --side-facing left|right".into(),
        );
    }
    let mut kinds = BTreeSet::new();
    let mut decoded = Vec::with_capacity(args.films.len());
    let mut films = Vec::with_capacity(args.films.len());
    for input in &args.films {
        if !kinds.insert(input.kind) {
            return Err("each film kind may appear only once".into());
        }
        let bytes = fs::read(&input.path).map_err(|_| "cannot read an input film")?;
        let pixels = film_reader::read_film(&bytes, input.kind)?;
        let mut geometry = FilmGeometry {
            quarter_turns: pixels.facts.exif_quarter_turns.unwrap_or_default(),
            ..FilmGeometry::UNKNOWN
        };
        if input.kind == FilmKind::Side
            && let Some(facing) = args.side_facing
        {
            geometry.image_right = Some(match facing {
                crate::SideFacing::Left => shapes::film_geometry::ImageRight::Posterior,
                crate::SideFacing::Right => shapes::film_geometry::ImageRight::Anterior,
            });
            geometry.confirmed.facing = true;
        }
        films.push(record(&pixels, geometry));
        decoded.push(pixels);
    }
    let input = AutoPassInput {
        case_version: 0,
        films,
        options: ChainOptions {
            acquisition: match args.acquisition {
                Acquisition::Simultaneous => shapes::auto_pass::Acquisition::Simultaneous,
                Acquisition::Separate => shapes::auto_pass::Acquisition::Separate,
                Acquisition::Unknown => shapes::auto_pass::Acquisition::Unknown,
            },
            population: match args.population {
                Population::Adolescent => shapes::auto_pass::Population::Adolescent,
                Population::Adult => shapes::auto_pass::Population::Adult,
            },
            template: None,
        },
        first_vertebra_id: 1,
        front_finder: match args.front_finder {
            crate::FrontFinder::Trained => FrontFinder::Trained,
            crate::FrontFinder::HandBuilt => FrontFinder::HandBuilt,
        },
        side_finder: match args.side_finder {
            Some(crate::FrontFinder::Trained) => shapes::SideFinder::Trained,
            _ => shapes::SideFinder::HandBuilt,
        },
    };
    let answer = conduct_auto_pass(&input, &decoded)?;
    println!("{}", serde_json::to_string_pretty(&answer)?);
    Ok(())
}

/// Route selected standing views to their trained finder and only the
/// remaining films to the hand-built chain. Preserve film order and ids.
pub(super) fn conduct_auto_pass(
    input: &AutoPassInput,
    decoded: &[DecodedFilm],
) -> Result<AutoPassResult, EngineError> {
    let trained = |film: &Film| {
        (film.kind == FilmKind::Front && input.front_finder == FrontFinder::Trained)
            || (film.kind == FilmKind::Side && input.side_finder == shapes::SideFinder::Trained)
    };
    if !input.films.iter().any(trained) {
        return auto_pass::run::auto_pass(input, decoded);
    }
    if decoded.len() != input.films.len() {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "a pass needs one decoded film per film record",
        ));
    }
    let mut film_results = Vec::new();
    let mut caches = Vec::new();
    for (image, film) in decoded.iter().zip(&input.films) {
        if !trained(film) {
            let (result, cache) = auto_pass::run::film_pass(image, film)?;
            film_results.push(result);
            caches.push(cache);
        }
    }
    let chain = if film_results.is_empty() {
        None
    } else {
        Some(auto_pass::run::chain_pass(&film_results, &input.options)?)
    };
    let mut next_id = input.first_vertebra_id;
    let mut marks = Vec::with_capacity(input.films.len());
    let mut warnings: Vec<PassWarning> = Vec::new();
    let mut hand_built = film_results.iter().zip(&caches);
    for (image, film) in decoded.iter().zip(&input.films) {
        let placed = if trained(film) {
            let found = if film.kind == FilmKind::Side {
                trained_finder::side::find_on_side_film(image, film, next_id)?
            } else {
                trained_finder::run::find_on_front_film(image, film, next_id)?
            };
            warnings.extend(found.warnings);
            found.marks
        } else {
            let (result, cache) = hand_built
                .next()
                .expect("a film pass ran on every film but the front");
            warnings.extend(result.warnings.iter().cloned());
            let found = chain
                .as_ref()
                .and_then(|chain| chain.films.iter().find(|found| found.film == film.kind))
                .ok_or_else(|| {
                    EngineError::new(
                        ErrorCode::EngineOutOfSync,
                        "the chain pass gave no chain for a film it was given",
                    )
                })?;
            auto_pass::run::place_points(cache, found, next_id)?
        };
        next_id = next_id
            .checked_add(placed.vertebrae.len() as u32)
            .ok_or_else(|| {
                EngineError::new(ErrorCode::EngineOutOfSync, "vertebra ids exhausted")
            })?;
        marks.push(placed);
    }
    if let Some(chain) = &chain {
        warnings.extend(
            chain
                .films
                .iter()
                .flat_map(|found| found.warnings.iter().cloned()),
        );
    }
    let mut once_each = Vec::with_capacity(warnings.len());
    for warning in warnings {
        if !once_each.contains(&warning) {
            once_each.push(warning);
        }
    }
    Ok(AutoPassResult {
        case_version: input.case_version,
        marks,
        two_films: chain.and_then(|chain| chain.two_films),
        warnings: once_each,
    })
}

/// The decoder's non-identifying facts plus the geometry chosen by the caller.
pub(super) fn record(decoded: &DecodedFilm, geometry: FilmGeometry) -> Film {
    let facts = &decoded.facts;
    Film {
        kind: facts.kind,
        image: FilmImage {
            path: facts.kind.png_path().to_owned(),
            width: facts.width,
            height: facts.height,
            bits_stored: facts.bits_stored,
        },
        source: facts.source,
        kept_tags: facts.kept_tags.clone(),
        values: facts.values.clone(),
        flags: facts.flags.clone(),
        blanked: Vec::new(),
        geometry,
    }
}

/// The scorekeeper supplies pixels and geometry; its answer key stays outside.
pub(super) fn find_marks(input: &ScoringFilm<'_>) -> Result<FilmMarks, EngineError> {
    let (decoded, film) = scoring_record(input)?;
    let input = AutoPassInput {
        case_version: 0,
        films: vec![film],
        options: ChainOptions {
            acquisition: shapes::auto_pass::Acquisition::Unknown,
            population: shapes::auto_pass::Population::Adolescent,
            template: None,
        },
        first_vertebra_id: 1,
        front_finder: FrontFinder::HandBuilt,
        side_finder: shapes::SideFinder::HandBuilt,
    };
    auto_pass::run::auto_pass(&input, &[decoded])?
        .marks
        .into_iter()
        .next()
        .ok_or_else(|| {
            EngineError::new(
                ErrorCode::PassNoSpineFound,
                "the pass returned no film marks",
            )
        })
}

/// The trained finder on one scoring film, read as `find_marks` reads it.
fn find_marks_trained(input: &ScoringFilm<'_>) -> Result<FilmMarks, EngineError> {
    let (decoded, film) = scoring_record(input)?;
    Ok(trained_finder::run::find_on_front_film(&decoded, &film, 1)?.marks)
}

/// A scoring film decoded, checked against the size the dataset gave, and
/// its record.
fn scoring_record(input: &ScoringFilm<'_>) -> Result<(DecodedFilm, Film), EngineError> {
    let decoded = film_reader::read_film(input.bytes, input.kind)?;
    if decoded.facts.width != input.width || decoded.facts.height != input.height {
        return Err(EngineError::new(
            ErrorCode::EngineOutOfSync,
            "the scoring dimensions do not match the decoded film",
        ));
    }
    let film = record(&decoded, input.geometry.clone());
    Ok((decoded, film))
}

/// The fixed made-up set exercises every film kind through the full pass.
/// Other suites keep the scorekeeper's existing dataset path.
type Finder<'a> = dyn FnMut(&ScoringFilm<'_>) -> Result<FilmMarks, EngineError> + 'a;

pub(super) fn score(
    args: &crate::ScoreArgs,
    dataset: impl FnOnce(&crate::ScoreArgs, &mut Finder<'_>) -> Outcome,
) -> Outcome {
    if !matches!(args.suite, crate::Suite::Ci) {
        return score_dataset(args, dataset);
    }
    use scorekeeper::datasets::made_up::{score_marks, true_corners};
    use scorekeeper::metrics::{Quad, cobb_from_corners::endplate_cobb};
    use scorekeeper::report::Distribution;
    use serde_json::json;
    use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
    use std::time::Instant;

    if !matches!(args.method, crate::Method::Auto) || args.jobs.is_some_and(|jobs| jobs != 1) {
        return Err("the made-up pass runs sequentially with --method auto".into());
    }
    if args.data.is_some() {
        return Err("the made-up set generates its own films; omit --data".into());
    }
    let count = args.count.unwrap_or(1);
    if count == 0 || args.seed.checked_add(count - 1).is_none() {
        return Err("the made-up seed range is empty or exceeds u32".into());
    }
    let tier = match args.tier.unwrap_or(crate::Tier::T1) {
        crate::Tier::T0 => Tier::T0,
        crate::Tier::T1 => Tier::T1,
        crate::Tier::T2 => Tier::T2,
        crate::Tier::T3 => Tier::T3,
        crate::Tier::T4 => Tier::T4,
    };
    let names = [
        RecipeName::Straight,
        RecipeName::RightThoracic40,
        RecipeName::DoubleMajor55To45,
        RecipeName::LumbarLeft30,
        RecipeName::Severe90,
        RecipeName::SixLumbar,
        RecipeName::ElevenRibs,
        RecipeName::UpsideDown,
        RecipeName::MirroredFront,
        RecipeName::SideFacingLeft,
        RecipeName::HeavyShoulders,
    ];
    let mut totals = std::collections::BTreeMap::<shapes::film::FilmKind, MadeUpTotals>::new();
    let mut rows = Vec::new();
    for name in names {
        for offset in 0..count {
            let recipe = MadeUpRecipe {
                name,
                seed: args.seed + offset,
                tier,
            };
            let case = made_up_spines::make_up_case(&recipe)?;
            for (painted, answer) in case.films.iter().zip(&case.answer_key.films) {
                let bytes = made_up_spines::film_png::encode(painted);
                let input = ScoringFilm {
                    kind: painted.kind,
                    bytes: &bytes,
                    width: painted.image.width(),
                    height: painted.image.height(),
                    geometry: answer.geometry.clone(),
                };
                let started = Instant::now();
                let found = find_marks(&input);
                let seconds = started.elapsed().as_secs_f64();
                let total = totals.entry(painted.kind).or_default();
                total.films += 1;
                total.eligible += answer.vertebrae.len();
                total.seconds.push(seconds);
                let marks = match found {
                    Ok(marks) => marks,
                    Err(error) => {
                        total.failed += 1;
                        rows.push(json!({"recipe": recipe, "film": painted.kind,
                            "error": error.code, "eligible": answer.vertebrae.len(),
                            "found": 0, "seconds": seconds}));
                        eprintln!("{:?} {:?}: failed {:?}", name, painted.kind, error.code);
                        continue;
                    }
                };
                let measured = score_marks(answer, input.height, &marks)?;
                let truth: Vec<_> = answer
                    .vertebrae
                    .iter()
                    .map(|v| {
                        let [tl, tr, bl, br] = true_corners(&v.marks).map(|p| p.at.0);
                        Quad { tl, tr, bl, br }
                    })
                    .collect();
                let prediction: Vec<_> = marks
                    .vertebrae
                    .iter()
                    .map(|v| Quad::from_marks(&v.marks))
                    .collect();
                let true_cobb = endplate_cobb(&truth);
                let predicted_cobb = endplate_cobb(&prediction);
                let cobb_error = (predicted_cobb - true_cobb).abs();
                total.found += measured.score.found;
                total.emitted += marks.vertebrae.len();
                total.corner_px.extend_from_slice(&measured.score.corner_px);
                total
                    .corner_norm
                    .extend_from_slice(&measured.score.corner_norm);
                total.corner_mm.extend_from_slice(&measured.corner_mm);
                total.cobb_error.push(cobb_error);
                eprintln!(
                    "{:?} {:?}: {}/{} found, {:.3} s",
                    name, painted.kind, measured.score.found, measured.score.eligible, seconds
                );
                rows.push(json!({"recipe": recipe, "film": painted.kind,
                    "emitted": marks.vertebrae.len(), "score": measured,
                    "endplateCobbTrue": true_cobb, "endplateCobbPredicted": predicted_cobb,
                    "endplateCobbAbsoluteError": cobb_error, "seconds": seconds}));
            }
        }
    }
    let mut summary = Vec::new();
    let mut markdown = String::from(
        "# Made-up automatic pass\n\nFound means one-to-one convex-hull IoU >= 0.5. Corner error matches by level, excludes invisible corners, and uses original pixels. Cobb is the maximum ordered endplate angle, not the three-column dataset benchmark. Failed films count as zero found; errors are unavailable, not zero. Timing includes decoding and the entire single-film pass, excluding generation and scoring. Accuracy on made-up films is not clinical accuracy.\n\n| Film | Films | Failed | Found / eligible | Mean corner px | Mean corner / true vertebral height | Mean corner mm | Cobb MAE degrees | Mean seconds |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for (kind, total) in totals {
        let px = Distribution::from_values(&total.corner_px);
        let norm = Distribution::from_values(&total.corner_norm);
        let mm = Distribution::from_values(&total.corner_mm);
        let cobb = Distribution::from_values(&total.cobb_error);
        let seconds = Distribution::from_values(&total.seconds);
        markdown.push_str(&format!(
            "| {} | {} | {} | {} / {} | {} | {} | {} | {} | {} |\n",
            kind.name(),
            total.films,
            total.failed,
            total.found,
            total.eligible,
            statistic(px.mean),
            statistic(norm.mean),
            statistic(mm.mean),
            statistic(cobb.mean),
            statistic(seconds.mean)
        ));
        summary.push(
            json!({"film": kind, "films": total.films, "failed": total.failed,
            "found": total.found, "eligible": total.eligible, "emitted": total.emitted,
            "cornerPx": px, "cornerNormalised": norm, "cornerMm": mm,
            "endplateCobbErrorDegrees": cobb, "seconds": seconds}),
        );
    }
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| "proving-ground/scoreboards/reports".into());
    fs::create_dir_all(&out).map_err(|_| "cannot create the report folder")?;
    let report = json!({"mode": "made-up whole-pass measurement; no accuracy gate",
        "engineVersion": env!("CARGO_PKG_VERSION"), "firstSeed": args.seed,
        "seedsPerRecipe": count, "tier": tier, "summary": summary, "films": rows});
    fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&report)?,
    )
    .map_err(|_| "cannot write summary.json")?;
    fs::write(out.join("summary.md"), markdown).map_err(|_| "cannot write summary.md")?;
    println!("{} films. Wrote summary.json and summary.md.", rows.len());
    Ok(())
}

#[derive(Default)]
struct MadeUpTotals {
    films: usize,
    failed: usize,
    eligible: usize,
    emitted: usize,
    found: usize,
    corner_px: Vec<f64>,
    corner_norm: Vec<f64>,
    corner_mm: Vec<f64>,
    cobb_error: Vec<f64>,
    seconds: Vec<f64>,
}

fn statistic(value: Option<f64>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |number| format!("{number:.4}"))
}

/// A failed search contributes an empty prediction to scoring, never a
/// fabricated layout. This adapter is used only by the dataset harness;
/// the native and browser pass entry points retain their error result.
///
/// The suite chooses the finder. The auto pass's searches fail in several
/// ways that mean "nothing to find here"; the trained finder's are finding
/// no vertebra at all and a film too small to search. Any other error stops
/// the run.
fn score_dataset(
    args: &crate::ScoreArgs,
    dataset: impl FnOnce(&crate::ScoreArgs, &mut Finder<'_>) -> Outcome,
) -> Outcome {
    use serde_json::json;
    use std::time::Instant;
    let trained = matches!(args.suite, crate::Suite::TrainedFinderFront);
    let mut runs = Vec::new();
    let mut finder = |input: &ScoringFilm<'_>| {
        let started = Instant::now();
        let outcome = if trained {
            find_marks_trained(input)
        } else {
            find_marks(input)
        };
        let seconds = started.elapsed().as_secs_f64();
        let row = runs.len() + 1;
        let error = outcome.as_ref().err().map(|error| error.code);
        runs.push(json!({"row": row, "seconds": seconds, "error": error,
            "emitted": outcome.as_ref().map_or(0, |marks| marks.vertebrae.len())}));
        match outcome {
            Ok(marks) => Ok(marks),
            Err(error)
                if matches!(
                    error.code,
                    ErrorCode::PassNoSpineFound | ErrorCode::PassFilmTooSmall
                ) || (!trained
                    && matches!(
                        error.code,
                        ErrorCode::PassFlatFilm
                            | ErrorCode::PassTooLittleUsable
                            | ErrorCode::PassStripTooShort
                    )) =>
            {
                eprintln!("film row {row}: search failed with {:?}", error.code);
                Ok(empty_prediction(input))
            }
            Err(error) => Err(error),
        }
    };
    dataset(args, &mut finder)?;
    if runs.is_empty() {
        return Ok(());
    }
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| "proving-ground/scoreboards/reports".into());
    let path = out.join("summary.json");
    let mut report: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    let failed = runs.iter().filter(|run| !run["error"].is_null()).count();
    let seconds: Vec<_> = runs
        .iter()
        .filter_map(|run| run["seconds"].as_f64())
        .collect();
    let timing = scorekeeper::report::Distribution::from_values(&seconds);
    report["finderRuns"] = json!(runs);
    report["summary"]["failedSearches"] = json!(failed);
    report["summary"]["secondsPerFilm"] = json!(timing);
    if let Some(caveats) = report["caveats"].as_array_mut() {
        caveats.retain(|value| {
            !value
                .as_str()
                .is_some_and(|text| text.contains("Full pixel decoding is unavailable"))
        });
        caveats.push(json!("Every film was fully decoded before its finder ran. Failed searches produce no detections and remain in the denominator; finderRuns records their row numbers and error codes. Cobb with insufficient points uses the scorekeeper's explicit zero-angle/100% SMAPE penalty. Timing includes decoding and the whole single-film pass."));
    }
    fs::write(path, serde_json::to_string_pretty(&report)?)?;
    let markdown_path = out.join("summary.md");
    let markdown = fs::read_to_string(&markdown_path)?;
    let mut markdown = markdown
        .lines()
        .filter(|line| !line.contains("Full pixel decoding is unavailable"))
        .collect::<Vec<_>>()
        .join("\n");
    markdown.push_str(&format!("\n\n## Finder execution\n\nEvery film was fully decoded before its finder ran. {failed} of {} searches failed and contributed zero detections. Their anonymous row numbers and error codes are in summary.json. Failed Cobb predictions use the scorekeeper's zero-angle/100% SMAPE penalty; corner errors are unavailable rather than zero. Mean time per film: {} seconds (decode and full pass, excluding scoring).\n", runs.len(), statistic(timing.mean)));
    fs::write(markdown_path, markdown)?;
    Ok(())
}

fn empty_prediction(input: &ScoringFilm<'_>) -> FilmMarks {
    use shapes::film_geometry::ImageRight;
    use shapes::naming::{ChosenBy, LevelNaming, NamingFlag, Template};
    FilmMarks {
        film: input.kind,
        vertebrae: Vec::new(),
        landmarks: shapes::landmarks::Landmarks::NONE,
        naming: LevelNaming {
            template: Template::Standard,
            chosen_by: ChosenBy::Engine,
            alternatives: Vec::new(),
            from_top: None,
            from_bottom: None,
            flags: vec![NamingFlag::NamesUnknown],
            l5_confirmed_on: None,
        },
        named_for: input.geometry.image_right.unwrap_or(
            if input.kind == shapes::film::FilmKind::Side {
                ImageRight::Anterior
            } else {
                ImageRight::PatientLeft
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorekeeper::metrics::{Prediction, Quad, ScoreFlag, score_film};
    use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
    use shapes::naming::NamingFlag;

    #[test]
    fn a_failed_search_counts_all_truth_as_missed_without_zero_corner_errors() {
        let input = ScoringFilm {
            kind: shapes::film::FilmKind::Front,
            bytes: &[],
            width: 100,
            height: 500,
            geometry: FilmGeometry::UNKNOWN,
        };
        let truth: Vec<_> = shapes::levels::Level::MEASURED
            .into_iter()
            .enumerate()
            .map(|(index, level)| {
                let y = index as f64 * 25.0;
                (
                    level,
                    Quad {
                        tl: [20.0, y],
                        tr: [40.0, y],
                        bl: [20.0, y + 20.0],
                        br: [40.0, y + 20.0],
                    },
                    true,
                )
            })
            .collect();
        let score = score_film(
            &truth,
            Some([30.0, 10.0, 5.0]),
            &Prediction::from(&empty_prediction(&input)),
            500.0,
        )
        .unwrap();
        assert_eq!(score.eligible, 17);
        assert_eq!(score.found, 0);
        assert!(score.corner_px.is_empty() && score.corner_norm.is_empty());
        assert_eq!(score.smape_term, Some(100.0));
        assert!(score.flags.contains(&ScoreFlag::InsufficientForCobb));
    }

    /// A small made-up film of a straight spine, one millimetre a pixel,
    /// as its PNG decodes: the recipe the auto pass's own tests read.
    fn made_up_film(kind: FilmKind) -> DecodedFilm {
        use made_up_spines::{answer_key, film_layout::FilmLayout, film_painter, patient, recipes};
        let recipe = MadeUpRecipe {
            name: RecipeName::Straight,
            seed: 17,
            tier: Tier::T0,
        };
        let seed = u64::from(recipe.seed);
        let patient = patient::make_up_patient(recipe.name, recipe.seed).unwrap();
        let spine = answer_key::spine_for(&patient, kind);
        let mut drawing = recipes::film_recipes(&recipe, seed)
            .into_iter()
            .find(|drawing| drawing.kind == kind)
            .unwrap();
        drawing.mm_per_px = 1.0;
        let layout = FilmLayout::new(&drawing, spine);
        let painted = film_painter::paint(&drawing, &layout, &patient, spine, seed);
        film_reader::read_film(&made_up_spines::film_png::encode(&painted), kind).unwrap()
    }

    /// The trained finder on the front film, ids from 5.
    fn trained_input(decoded: &[DecodedFilm]) -> AutoPassInput {
        AutoPassInput {
            case_version: 3,
            films: decoded
                .iter()
                .map(|film| record(film, FilmGeometry::UNKNOWN))
                .collect(),
            options: ChainOptions {
                acquisition: shapes::auto_pass::Acquisition::Unknown,
                population: shapes::auto_pass::Population::Adolescent,
                template: None,
            },
            first_vertebra_id: 5,
            front_finder: FrontFinder::Trained,
            side_finder: shapes::SideFinder::HandBuilt,
        }
    }

    #[test]
    fn a_front_film_alone_goes_to_the_trained_finder_and_no_chain_runs() {
        let decoded = [made_up_film(FilmKind::Front)];
        let answer = conduct_auto_pass(&trained_input(&decoded), &decoded).unwrap();
        assert_eq!(answer.case_version, 3);
        let [front] = answer.marks.as_slice() else {
            panic!("one film's marks per film");
        };
        assert_eq!(front.film, FilmKind::Front);
        assert!(!front.vertebrae.is_empty());
        // Only the trained finder flags its names as estimates.
        assert_eq!(front.naming.flags, [NamingFlag::Estimated]);
        let ids: Vec<u32> = front.vertebrae.iter().map(|vertebra| vertebra.id).collect();
        assert_eq!(ids, (5..).take(ids.len()).collect::<Vec<u32>>());
        assert!(answer.two_films.is_none());
    }

    /// Side first and then front, so the front's ids must count on from
    /// the side's. Ignored because the hand-built pass on even a small film
    /// takes about fifteen seconds unoptimised; run it with `--ignored`.
    #[test]
    #[ignore = "the hand-built pass takes about fifteen seconds in a test build"]
    fn a_trained_front_film_joins_the_hand_built_films_in_film_order() {
        let decoded = [made_up_film(FilmKind::Side), made_up_film(FilmKind::Front)];
        let answer = conduct_auto_pass(&trained_input(&decoded), &decoded).unwrap();
        let [side, front] = answer.marks.as_slice() else {
            panic!("one film's marks per film");
        };
        assert_eq!((side.film, front.film), (FilmKind::Side, FilmKind::Front));
        assert!(!side.vertebrae.is_empty() && !front.vertebrae.is_empty());
        assert_eq!(front.naming.flags, [NamingFlag::Estimated]);
        assert!(!side.naming.flags.contains(&NamingFlag::Estimated));
        let ids: Vec<u32> = side
            .vertebrae
            .iter()
            .chain(&front.vertebrae)
            .map(|vertebra| vertebra.id)
            .collect();
        assert_eq!(ids, (5..).take(ids.len()).collect::<Vec<u32>>());
        // The hand-built chain saw no front film to check the side against.
        assert!(answer.two_films.is_none());
        for (index, warning) in answer.warnings.iter().enumerate() {
            assert!(!answer.warnings[..index].contains(warning));
        }
    }
}
