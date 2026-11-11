//! `spine`: the same engine as the browser, on the command line. It runs
//! the whole pipeline with no browser, makes made-up spines, records the
//! watch frames the app replays, packs and checks case files, and scores
//! the engine on real and made-up films.
//!
//! Every subcommand is declared here; each lives in its own file under
//! `commands/`.

// The command line is where the engine meets the disk and the shell: it
// reads films and datasets and writes results, so it may use files and
// the environment. The engine crates it calls never do.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]
#![forbid(unsafe_code)]

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

mod commands {
    pub mod case;
    pub mod frames;
    pub mod make_up;
    pub mod open;
    pub mod pass;
    pub mod score;
}

/// What a command returns: nothing on success, or why it failed.
type Outcome = Result<(), Box<dyn Error>>;

#[derive(Parser)]
#[command(
    name = "spine",
    version,
    about = "The Spine Workbench engine, natively."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Make a made-up spine: four films and their answer key.
    MakeUp(MakeUpArgs),
    /// Open a film and print its size, kept tags and scale.
    Open(OpenArgs),
    /// Run the auto pass on films and print their marks as JSON.
    Pass(PassArgs),
    /// Record the watch frames of an auto pass, or check them for drift.
    Frames(FramesArgs),
    /// Pack, unpack or check a case file.
    Case(CaseArgs),
    /// Run a scoring suite.
    Score(ScoreArgs),
    /// Compare two scoring runs with a paired test.
    Compare(CompareArgs),
}

/// The named made-up spine recipes.
#[derive(Clone, Copy, ValueEnum)]
enum Recipe {
    Random,
    Straight,
    #[value(name = "right-thoracic-40")]
    RightThoracic40,
    #[value(name = "double-major-55-45")]
    DoubleMajor55To45,
    #[value(name = "lumbar-left-30")]
    LumbarLeft30,
    #[value(name = "severe-90")]
    Severe90,
    SixLumbar,
    ElevenRibs,
    UpsideDown,
    MirroredFront,
    SideFacingLeft,
    HeavyShoulders,
}

/// How hard made-up films are to read, from clean (t0) to cluttered (t4).
#[derive(Clone, Copy, ValueEnum)]
enum Tier {
    T0,
    T1,
    T2,
    T3,
    T4,
}

/// The file format made-up films are written in.
#[derive(Clone, Copy, ValueEnum)]
enum FilmFormat {
    Png,
    Dicom,
}

#[derive(Args)]
struct MakeUpArgs {
    recipe: Recipe,
    /// Every random draw comes from this seed.
    #[arg(long)]
    seed: u32,
    #[arg(long, value_enum, default_value_t = Tier::T2)]
    tier: Tier,
    #[arg(long, value_enum, default_value_t = FilmFormat::Png)]
    format: FilmFormat,
    /// Make this many cases, with seeds counting up from `--seed`.
    #[arg(long, default_value_t = 1)]
    count: u32,
    /// Folder the films and answer keys are written to.
    #[arg(long, default_value = ".")]
    out: PathBuf,
}

#[derive(Args)]
struct OpenArgs {
    film: PathBuf,
}

/// A film on the command line: `front=PATH`, `side=PATH`,
/// `bend-left=PATH` or `bend-right=PATH`. The kind is always stated,
/// never guessed from the file name.
#[derive(Clone)]
struct FilmPath {
    kind: shapes::film::FilmKind,
    path: PathBuf,
}

fn film_path(text: &str) -> Result<FilmPath, String> {
    let (kind, path) = text
        .split_once('=')
        .ok_or_else(|| format!("expected KIND=PATH, not `{text}`"))?;
    let kind = shapes::film::FilmKind::ALL
        .into_iter()
        .find(|candidate| candidate.name() == kind)
        .ok_or_else(|| format!("`{kind}` is not front, side, bend-left or bend-right"))?;
    Ok(FilmPath {
        kind,
        path: PathBuf::from(path),
    })
}

/// Whether the front and side films were taken at the same moment.
#[derive(Clone, Copy, ValueEnum)]
enum Acquisition {
    Simultaneous,
    Separate,
    Unknown,
}

/// Whose body sizes the height priors assume.
#[derive(Clone, Copy, ValueEnum)]
enum Population {
    Adolescent,
    Adult,
}

#[expect(dead_code, reason = "a command reads its arguments once it is built")]
#[derive(Args)]
struct PassArgs {
    /// One to four films, each as KIND=PATH.
    #[arg(required = true, value_parser = film_path)]
    films: Vec<FilmPath>,
    #[arg(long, value_enum, default_value_t = Acquisition::Unknown)]
    acquisition: Acquisition,
    #[arg(long, value_enum, default_value_t = Population::Adolescent)]
    population: Population,
}

#[expect(dead_code, reason = "a command reads its arguments once it is built")]
#[derive(Args)]
struct FramesArgs {
    /// Compare with the recorded frames instead of writing them, and fail
    /// on any difference.
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct CaseArgs {
    #[command(subcommand)]
    action: CaseAction,
}

#[expect(dead_code, reason = "a command reads its arguments once it is built")]
#[derive(Subcommand)]
enum CaseAction {
    /// Pack `case.json` and its films into a case file.
    Pack {
        /// The case's `case.json`.
        case: PathBuf,
        /// Each film as KIND=PATH.
        #[arg(long = "film", value_parser = film_path)]
        films: Vec<FilmPath>,
        /// The case file to write.
        #[arg(long)]
        out: PathBuf,
    },
    /// Unpack a case file into a folder.
    Unpack {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Read a case file strictly and run the case checks.
    Check { file: PathBuf },
}

/// The scoring suites.
#[derive(Clone, Copy, ValueEnum)]
enum Suite {
    /// The made-up-spine suite CI gates on.
    Ci,
    AutoPassFront,
    AutoPassSide,
    TwoFilmsVoting,
    RegionWand,
    /// How close made-up films come to real ones.
    Realism,
    /// Counts, flags and Cobb reproduction on the AASCE labels.
    AasceLabels,
}

/// What lays the spine out in a suite.
#[derive(Clone, Copy, ValueEnum)]
enum Method {
    Auto,
    /// The auto pass, then simulated pins.
    AutoWithPins,
    /// Laid out by hand from two, four or six clicks.
    #[value(name = "hand-2")]
    Hand2,
    #[value(name = "hand-4")]
    Hand4,
    #[value(name = "hand-6")]
    Hand6,
}

/// Which part of a dataset to score.
#[derive(Clone, Copy, ValueEnum)]
enum Split {
    Train,
    Test,
}

#[derive(Args)]
struct ScoreArgs {
    suite: Suite,
    /// The dataset folder; by default the suite's own folder under
    /// `local-only/real-films/`.
    #[arg(long)]
    data: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Split::Test)]
    split: Split,
    #[arg(long, value_enum, default_value_t = Method::Auto)]
    method: Method,
    /// Made-up cases to score, for suites that use them.
    #[arg(long)]
    count: Option<u32>,
    #[arg(long, default_value_t = 1)]
    seed: u32,
    #[arg(long, value_enum)]
    tier: Option<Tier>,
    /// Films scored at once.
    #[arg(long)]
    jobs: Option<usize>,
    /// Folder the report is written to; by default
    /// `proving-ground/scoreboards/reports/`.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// The measure two runs are compared on.
#[derive(Clone, Copy, ValueEnum)]
enum Metric {
    CornerError,
    ClicksToGoodOutline,
    Naming,
}

/// The paired test used to compare two runs.
#[derive(Clone, Copy, ValueEnum)]
enum PairedTest {
    Bootstrap,
    Wilcoxon,
    Mcnemar,
}

#[expect(dead_code, reason = "a command reads its arguments once it is built")]
#[derive(Args)]
struct CompareArgs {
    /// The JSON report of the first run.
    first: PathBuf,
    /// The JSON report of the second run.
    second: PathBuf,
    #[arg(long, value_enum)]
    metric: Metric,
    #[arg(long, value_enum)]
    test: PairedTest,
}

fn main() -> ExitCode {
    let outcome = match Cli::parse().command {
        Command::MakeUp(args) => commands::make_up::run(&args),
        Command::Open(args) => commands::open::run(&args),
        Command::Pass(args) => commands::pass::run(&args),
        Command::Frames(args) => commands::frames::run(&args),
        Command::Case(args) => commands::case::run(&args),
        Command::Score(args) => commands::score::run(&args),
        Command::Compare(args) => commands::score::compare(&args),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("spine: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The answer of a command whose engine part is not in this build yet.
fn not_built(command: &str) -> Outcome {
    Err(format!("`spine {command}` is not built yet in this version").into())
}
