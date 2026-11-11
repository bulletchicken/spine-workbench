//! The human-readable report.

use super::{CobbSummary, Distribution, Report};
use std::fmt::Write;

pub fn render(report: &Report) -> String {
    let s = &report.summary;
    let mut text = format!(
        "# {}: {}\n\n",
        report.provenance.dataset, report.provenance.mode
    );
    let _ = writeln!(
        text,
        "Engine: {}; revision: {}. Split: {}.\n",
        report.provenance.engine_version,
        report
            .provenance
            .git_revision
            .as_deref()
            .unwrap_or("unavailable"),
        report.provenance.split.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        text,
        "Films: {}; flagged: {}. Eligible vertebrae: {}. Found: {}; missed: {}; false positives in the spine strip: {}.",
        s.films, s.flagged_films, s.eligible, s.found, s.missed, s.false_positives
    );
    let _ = writeln!(
        text,
        "Found fraction: {}. Naming fraction among matches: {} ({}/{}).\n",
        number(s.found_fraction),
        number(s.naming_fraction),
        s.levels_right,
        s.found
    );
    distribution(&mut text, "Corner error (px)", &s.corner_px);
    distribution(
        &mut text,
        "Corner error / true height",
        &s.corner_normalised,
    );
    distribution(&mut text, "Corner error (mm)", &s.corner_mm);
    text.push_str("\nPer-level corner errors (pixels; normalised by true height):\n\n");
    for level in &s.per_level {
        distribution(
            &mut text,
            &format!("{:?} px", level.level),
            &level.corner_px,
        );
        distribution(
            &mut text,
            &format!("{:?} / height", level.level),
            &level.corner_normalised,
        );
    }
    text.push('\n');
    let _ = writeln!(
        text,
        "Fractions strictly under 0.10 / 0.25 true height: {} / {}.\n",
        number(s.under_010_height),
        number(s.under_025_height)
    );
    cobb(&mut text, "All films", &s.cobb_all);
    cobb(&mut text, "Unflagged films", &s.cobb_unflagged);
    text.push_str("\n## Row results\n\nRow numbers refer to the CSV order, starting at 1; original file names are omitted.\n\n");
    text.push_str("| Row | Film | Found / eligible | Naming right | Chain shift | Mean corner px | Flags |\n|---|---|---|---|---|---|---|\n");
    for film in &report.films {
        let score = &film.score;
        let _ = writeln!(
            text,
            "| {} | {} | {} / {} | {} | {} | {} | {:?}; {:?} |",
            film.row,
            film.film.name(),
            score.found,
            score.eligible,
            score.naming_ok,
            number(score.chain_shift),
            number(Distribution::from_values(&score.corner_px).mean),
            film.label_flags,
            score.flags
        );
    }
    text.push_str("\n## Interpretation\n\n");
    for caveat in &report.caveats {
        let _ = writeln!(text, "- {caveat}");
    }
    text.push_str("\nExact provenance hashes and per-level corner samples are in summary.json.\n");
    text
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |x| format!("{x:.6}"))
}

fn distribution(text: &mut String, label: &str, d: &Distribution) {
    let _ = writeln!(
        text,
        "{label}: n={}, mean={}, median={}, P90={}.",
        d.count,
        number(d.mean),
        number(d.median),
        number(d.p90)
    );
}

fn cobb(text: &mut String, label: &str, c: &CobbSummary) {
    let _ = writeln!(
        text,
        "{label}: Cobb n={}, SMAPE={}%; column MAE=[{}, {}, {}] degrees; all columns within 5 / 10 degrees={} / {}.",
        c.films,
        number(c.smape_percent),
        number(c.mae_degrees[0]),
        number(c.mae_degrees[1]),
        number(c.mae_degrees[2]),
        number(c.within_5_degrees),
        number(c.within_10_degrees)
    );
    let _ = writeln!(
        text,
        "Maximum absolute residual={} degrees; median film maximum={} degrees; fraction within 0.01 degrees={}.",
        number(c.maximum_absolute_error),
        number(c.median_film_maximum_error),
        number(c.within_001_degrees)
    );
}
