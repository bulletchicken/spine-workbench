//! Dataset label audits and the host boundary for an injected finder.
//! Real paths and file names are used only for reading, never in reports.

use crate::{CompareArgs, Method, Outcome, ScoreArgs, Split, Suite, not_built};
use scorekeeper::datasets::ScoringFilm;
use scorekeeper::datasets::aasce_2019::{AasceFilm, parse_labels};
use scorekeeper::metrics::{PredictedVertebra, Prediction, score_film};
use scorekeeper::report::{FilmReport, Provenance, Report, json, markdown};
use shapes::errors::EngineError;
use shapes::film::FilmKind;
use shapes::vertebra::FilmMarks;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type Finder<'a> = dyn FnMut(&ScoringFilm<'_>) -> Result<FilmMarks, EngineError> + 'a;

pub fn run(args: &ScoreArgs) -> Outcome {
    crate::commands::pass::score(args, |args, finder| run_with_finder(args, Some(finder)))
}

/// The native automatic pass plugs in here when its entry point is built.
/// A finder receives only film bytes and geometry, never the answer key.
fn run_with_finder(args: &ScoreArgs, mut finder: Option<&mut Finder<'_>>) -> Outcome {
    let run_started_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|t| t.as_secs());
    if !matches!(args.method, Method::Auto) {
        return Err("simulated users and hand-layout methods belong to a later slice".into());
    }
    if args.jobs.is_some_and(|jobs| jobs != 1) {
        return Err("dataset scoring is sequential in this build; use --jobs 1".into());
    }
    if args.count.is_some() || args.tier.is_some() || args.seed != 1 {
        return Err("count, tier and seed options belong to the later made-up suites".into());
    }
    let audit = match args.suite {
        Suite::AasceLabels => true,
        Suite::AutoPassFront | Suite::TrainedFinderFront => {
            if finder.is_none() {
                return Err(
                    "the automatic finder is not built; use aasce-labels for a label audit".into(),
                );
            }
            false
        }
        _ => return Err("this scoring suite belongs to a later slice".into()),
    };
    let split = match args.split {
        Split::Train => "training",
        Split::Test => "test",
    };
    let root = dataset_root(args.data.as_deref());
    let label_dir = root.join("labels").join(split);
    let filenames = read(
        &label_dir.join("filenames.csv"),
        "cannot read filenames.csv",
    )?;
    let landmarks = read(
        &label_dir.join("landmarks.csv"),
        "cannot read landmarks.csv",
    )?;
    let angles = read(&label_dir.join("angles.csv"), "cannot read angles.csv")?;
    let labels = parse_labels(&filenames, &landmarks, &angles)?;
    let image_dir = root.join("data").join(split);
    let image_dir =
        fs::canonicalize(image_dir).map_err(|_| "cannot open the dataset film folder")?;

    // Validate every row and every header before any finder runs. No partial
    // report is written when even one required film cannot be inspected.
    let mut inputs = Vec::with_capacity(labels.len());
    for index in 0..labels.len() {
        let name = labels.filename(index).ok_or("missing filename row")?;
        let path = fs::canonicalize(image_dir.join(name))
            .map_err(|_| format!("cannot open film at row {}", index + 1))?;
        if !path.starts_with(&image_dir) {
            return Err(format!("film at row {} is outside the dataset folder", index + 1).into());
        }
        let bytes = read(&path, "cannot read a dataset film")?;
        let (width, height) = jpeg_dimensions(&bytes)
            .ok_or_else(|| format!("invalid JPEG header at row {}", index + 1))?;
        inputs.push((path, labels.film(index, width, height)?));
    }
    let mut films = Vec::with_capacity(inputs.len());
    for (path, film) in inputs {
        let score = if audit {
            label_score(&film)?
        } else {
            let bytes = read(&path, "cannot reread a dataset film")?;
            let find = finder.as_deref_mut().ok_or("the finder is unavailable")?;
            film.score_with_finder(&bytes, find)?
        };
        films.push(FilmReport {
            row: film.row,
            film: FilmKind::Front,
            width: film.width,
            height: film.height,
            label_flags: film.flags.flags(),
            excluded_levels: film
                .truth_for_scoring()
                .into_iter()
                .filter_map(|(level, _, eligible)| (!eligible).then_some(level))
                .collect(),
            score,
            corner_mm: None,
        });
    }
    let mut label_sha256 = BTreeMap::new();
    for (name, bytes) in [
        ("filenames.csv", &filenames),
        ("landmarks.csv", &landmarks),
        ("angles.csv", &angles),
    ] {
        label_sha256.insert(name.to_owned(), json::sha256(bytes));
    }
    let mode = match args.suite {
        Suite::AasceLabels => "label audit (truth against published angles)",
        Suite::TrainedFinderFront => "finder scoring: trained-finder-front",
        _ => "finder scoring",
    };
    let mut parameters = format!("{split}\n{mode}\niou=0.5\n");
    let mut notes = caveats(audit);
    if matches!(args.suite, Suite::TrainedFinderFront) {
        // The weights decide the answer as much as the code does, so the
        // fingerprint and the report both name them.
        let weights = trained_finder_weights()?;
        parameters.push_str(&format!("weights={weights}\n"));
        notes.push(format!(
            "The trained finder's weights: finder-weights.bin, SHA-256 {weights}."
        ));
    }
    let report = Report::new(
        Provenance {
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            git_revision: revision(),
            run_started_unix_seconds,
            dataset: "AASCE 2019".to_owned(),
            split: Some(split.to_owned()),
            mode: mode.to_owned(),
            listing_sha256: json::sha256(&filenames),
            label_sha256,
            parameters_sha256: json::sha256(parameters.as_bytes()),
        },
        films,
        notes,
    );
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("proving-ground/scoreboards/reports"));
    let machine = json::render(&report)?;
    let human = markdown::render(&report);
    fs::create_dir_all(&out).map_err(|_| "cannot create the report folder")?;
    fs::write(out.join("summary.json"), machine).map_err(|_| "cannot write summary.json")?;
    fs::write(out.join("summary.md"), &human).map_err(|_| "cannot write summary.md")?;
    println!(
        "{} films; {} flagged. Wrote summary.json and summary.md.",
        report.summary.films, report.summary.flagged_films
    );
    if audit {
        println!(
            "Label audit only; JPEG headers inspected, pixels not decoded. Maximum Cobb residual: {:.6} degrees.",
            report
                .summary
                .cobb_all
                .maximum_absolute_error
                .unwrap_or(0.0)
        );
    }
    Ok(())
}

/// The weight file's SHA-256 as the trained finder's compiled-in table
/// records it.
fn trained_finder_weights() -> Result<String, Box<dyn std::error::Error>> {
    let table: serde_json::Value = serde_json::from_str(trained_finder::weights::WEIGHT_TABLE)?;
    table["sha256"]["finder-weights.bin"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the trained finder's weight table names no weight file".into())
}

fn label_score(
    film: &AasceFilm,
) -> Result<scorekeeper::metrics::FilmScore, scorekeeper::metrics::ScoreError> {
    let truth = film.truth_for_scoring();
    let prediction = Prediction {
        vertebrae: truth
            .iter()
            .map(|&(level, quad, _)| PredictedVertebra {
                level,
                quad,
                confidence: 1.0,
            })
            .collect(),
    };
    score_film(&truth, Some(film.cobb), &prediction, f64::from(film.height))
}

fn dataset_root(data: Option<&Path>) -> PathBuf {
    let root = data.map_or_else(
        || PathBuf::from("local-only/real-films/aasce-2019"),
        Path::to_path_buf,
    );
    if root.join("boostnet_labeldata").is_dir() {
        root.join("boostnet_labeldata")
    } else {
        root
    }
}

fn read(path: &Path, message: &'static str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    fs::read(path).map_err(|_| message.into())
}

fn revision() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit())).then(|| text.to_owned())
}

/// Only the baseline JPEG header is inspected here. The absent film decoder
/// must later validate the compressed pixel stream before finder scoring.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) || !bytes.ends_with(&[0xff, 0xd9]) {
        return None;
    }
    let mut offset = 2;
    let mut dimensions = None;
    while offset < bytes.len() {
        if *bytes.get(offset)? != 0xff {
            return None;
        }
        while *bytes.get(offset)? == 0xff {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if matches!(marker, 0xd8 | 0xd9 | 0x00 | 0xd0..=0xd7) {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([
            *bytes.get(offset)?,
            *bytes.get(offset + 1)?,
        ]));
        if length < 2 {
            return None;
        }
        let segment = bytes.get(offset + 2..offset.checked_add(length)?)?;
        if marker == 0xc0 {
            if dimensions.is_some() || segment.len() < 6 || segment[0] != 8 {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([segment[1], segment[2]]));
            let width = u32::from(u16::from_be_bytes([segment[3], segment[4]]));
            if width == 0 || height == 0 || segment.len() != 6 + 3 * usize::from(segment[5]) {
                return None;
            }
            dimensions = Some((width, height));
        }
        if marker == 0xda {
            return dimensions;
        }
        offset += length;
    }
    None
}

fn caveats(audit: bool) -> Vec<String> {
    let mut notes = vec![
        "Films are pre-cropped to roughly T1–L5: upright, 8-bit JPEG, no marker or millimetre scale. Orientation, left/right, skull, pelvis and full-length behaviour cannot be tested here.",
        "Errors are in original pixels and true vertebral-height units only. The public test split is not the challenge's hidden test; no leaderboard comparison is valid.",
        "Flagged vertebrae are excluded from corner, found and naming metrics. All films remain in published-angle Cobb scoring; the unflagged-film result is also shown.",
        "Roughly five percent of label rows have corrupt vertebrae; flagged rows are listed above by row number only.",
        "Accuracy on made-up films is not clinical accuracy.",
        "JPEG signatures, dimensions and terminal markers were inspected. Full pixel decoding is unavailable in this build; a header audit cannot establish that compressed pixels are valid.",
    ].into_iter().map(str::to_owned).collect::<Vec<_>>();
    if audit {
        notes.push("This is a label audit, not a finder accuracy result. Labels serve as predictions solely to check geometry and reproduce the published Cobb columns.".to_owned());
    }
    notes
}

pub fn compare(_args: &CompareArgs) -> Outcome {
    not_built("compare")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_the_dimensions_of_a_hand_written_jpeg_header() {
        // SOF0 says 8-bit, height 20, width 10, one component. This is
        // deliberately not a decodable image: the check promises headers only.
        let bytes = [
            0xff, 0xd8, 0xff, 0xc0, 0, 11, 8, 0, 20, 0, 10, 1, 1, 0x11, 0, 0xff, 0xda, 0, 2, 0xff,
            0xd9,
        ];
        assert_eq!(jpeg_dimensions(&bytes), Some((10, 20)));
        assert_eq!(jpeg_dimensions(&bytes[..bytes.len() - 1]), None);
        let mut bad = bytes;
        bad[5] = 255;
        assert_eq!(jpeg_dimensions(&bad), None);
    }

    /// A one-film dataset in the AASCE layout under the system's temporary
    /// folder: seventeen flat, evenly spaced labels and `film` as its one
    /// JPEG, named `fixture.jpg`.
    fn tiny_dataset(folder: &str, film: &[u8]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("{folder}-{}", std::process::id()));
        fs::create_dir_all(root.join("labels/test")).unwrap();
        fs::create_dir_all(root.join("data/test")).unwrap();
        let mut columns = Vec::new();
        for _ in 0..17 {
            columns.extend([0.2, 0.8, 0.2, 0.8]);
        }
        for j in 0..17 {
            let y = (j as f64 + 1.0) / 20.0;
            columns.extend([y, y, y + 0.02, y + 0.02]);
        }
        let csv = columns
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        fs::write(root.join("labels/test/filenames.csv"), b"fixture.jpg\n").unwrap();
        fs::write(root.join("labels/test/landmarks.csv"), csv).unwrap();
        fs::write(root.join("labels/test/angles.csv"), b"0,0,0\n").unwrap();
        fs::write(root.join("data/test/fixture.jpg"), film).unwrap();
        root
    }

    fn score_args(suite: Suite, root: &Path) -> ScoreArgs {
        ScoreArgs {
            suite,
            data: Some(root.to_path_buf()),
            split: Split::Test,
            method: Method::Auto,
            count: None,
            seed: 1,
            tier: None,
            jobs: Some(1),
            out: Some(root.join("report")),
        }
    }

    #[test]
    fn label_audit_writes_both_reports_and_refuses_missing_films() {
        let root = tiny_dataset(
            "spine-score-fake",
            &[
                0xff, 0xd8, 0xff, 0xc0, 0, 11, 8, 0, 200, 0, 100, 1, 1, 0x11, 0, 0xff, 0xda, 0, 2,
                0xff, 0xd9,
            ],
        );
        let args = score_args(Suite::AasceLabels, &root);
        run(&args).expect("the complete header fixture can be audited");
        let json = fs::read_to_string(root.join("report/summary.json")).unwrap();
        let report: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(report["summary"]["found"], 17);
        assert_eq!(
            report["summary"]["cobbAll"]["maximumAbsoluteError"].as_f64(),
            Some(0.0)
        );
        assert!(!json.contains("fixture.jpg"));
        assert!(root.join("report/summary.md").is_file());
        fs::remove_dir_all(root.join("report")).unwrap();
        fs::remove_file(root.join("data/test/fixture.jpg")).unwrap();
        let error = run(&args).expect_err("every film is required").to_string();
        assert!(error.contains("row 1"));
        assert!(!error.contains("fixture.jpg"));
        assert!(!root.join("report").exists());
        fs::remove_dir_all(root).unwrap();
    }

    /// A baseline JPEG of one flat mid-grey channel, `width` by `height`,
    /// both multiples of 32: every 8 by 8 block holds only a zero DC
    /// difference and an end of block, each a one-bit code, so the scan is
    /// all zero bits.
    fn flat_grey_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        // Every quantiser 1.
        bytes.extend([0xff, 0xdb, 0, 67, 0]);
        bytes.extend([1; 64]);
        // Baseline, 8 bits, one channel sampled 1 by 1, quantiser 0.
        bytes.extend([0xff, 0xc0, 0, 11, 8]);
        bytes.extend(height.to_be_bytes());
        bytes.extend(width.to_be_bytes());
        bytes.extend([1, 1, 0x11, 0]);
        // DC and AC tables, each one symbol (0) with the one-bit code 0.
        for table in [0x00, 0x10] {
            bytes.extend([0xff, 0xc4, 0, 20, table, 1]);
            bytes.extend([0; 15]);
            bytes.push(0);
        }
        bytes.extend([0xff, 0xda, 0, 8, 1, 1, 0x00, 0, 63, 0]);
        let blocks = usize::from(width / 8) * usize::from(height / 8);
        bytes.extend(vec![0; blocks * 2 / 8]);
        bytes.extend([0xff, 0xd9]);
        bytes
    }

    /// The tiny dataset is made here, as the label audit's is, but with a
    /// film that decodes: flat grey, on which the trained finder finds no
    /// vertebra. That must count as an empty prediction, not stop the run.
    #[test]
    fn the_trained_finder_suite_counts_a_film_with_nothing_found_as_missed() {
        let root = tiny_dataset("spine-score-trained", &flat_grey_jpeg(64, 128));
        let args = score_args(Suite::TrainedFinderFront, &root);
        run(&args).expect("a film with nothing found is scored, not refused");
        let text = fs::read_to_string(root.join("report/summary.json")).unwrap();
        let markdown = fs::read_to_string(root.join("report/summary.md")).unwrap();
        fs::remove_dir_all(&root).unwrap();
        let report: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["summary"]["eligible"], 17);
        assert_eq!(report["summary"]["found"], 0);
        assert_eq!(report["summary"]["failedSearches"], 1);
        assert_eq!(report["finderRuns"][0]["error"], "pass-no-spine-found");
        let mode = "finder scoring: trained-finder-front";
        assert_eq!(report["provenance"]["mode"], mode);
        assert!(markdown.starts_with(&format!("# AASCE 2019: {mode}\n")));
        let weights = trained_finder_weights().unwrap();
        assert_eq!(weights.len(), 64);
        assert_eq!(
            report["provenance"]["parametersSha256"],
            json::sha256(format!("test\n{mode}\niou=0.5\nweights={weights}\n").as_bytes())
        );
        assert!(markdown.contains(&weights));
        assert!(!text.contains("fixture.jpg") && !markdown.contains("fixture.jpg"));
    }
}
