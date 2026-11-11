//! Quarter turns and upside-down, from where the skull and the pelvis sit and
//! from the file's orientation tags. A proposal; the user confirms.

use crate::rough_trace::{Profile, RoughTrace};
use film_reader::decoded_film::DecodedFilm;
use shapes::film::{FilmKind, IntakeFlag};
use shapes::film_geometry::{
    Certainty, ImageRight, QuarterTurns, UprightCue, UprightEvidence, UprightGuess,
};

/// Compare the two ends of each possible long axis. The wider, heavier
/// pelvis belongs below the narrower neck. These are experimental cues,
/// deliberately capped at medium certainty without corroborating tags.
pub fn guess(film: &DecodedFilm, trace: &RoughTrace) -> UprightGuess {
    let mut candidates: Vec<_> = (0..4)
        .map(|q| {
            let turns = QuarterTurns::new(q).expect("four quarter turns");
            let profile = trace.profile(turns);
            let (neck, pelvis, elongation) = end_cues(&profile);
            let score = elongation * (0.65 * neck.unwrap_or(0.0) + 0.35 * pelvis.unwrap_or(0.0));
            (turns, score, neck, pelvis, elongation)
        })
        .collect();
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.get().cmp(&b.0.get())));
    let file = film
        .facts
        .kept_tags
        .as_ref()
        .and_then(|tags| tags.patient_orientation.as_ref())
        .and_then(orientation_turns);
    let exif = (!film.facts.flags.contains(&IntakeFlag::ExifMirrored))
        .then_some(film.facts.exif_quarter_turns)
        .flatten();
    let (mut turns, score, neck, pelvis, elongation) = candidates[0];
    let anatomical = score > 0.08 && elongation > 0.4;
    if !anatomical {
        turns = file.or(exif).unwrap_or(QuarterTurns::NONE);
    }
    let agrees = |hint: Option<QuarterTurns>| hint.map(|q| if q == turns { 1.0 } else { -1.0 });
    let file_strength = agrees(file);
    let exif_strength = agrees(exif);
    let conflict = file_strength == Some(-1.0) || exif_strength == Some(-1.0);
    let certainty = if conflict {
        Certainty::Low
    } else if anatomical && (file_strength == Some(1.0) || exif_strength == Some(1.0)) {
        Certainty::High
    } else if anatomical && score > 0.16 {
        Certainty::Medium
    } else if anatomical || file.is_some() || exif.is_some() {
        Certainty::Low
    } else {
        Certainty::None
    };
    UprightGuess {
        quarter_turns: turns,
        certainty,
        evidence: vec![
            UprightEvidence {
                cue: UprightCue::FileOrientation,
                strength: file_strength,
            },
            UprightEvidence {
                cue: UprightCue::ExifOrientation,
                strength: exif_strength,
            },
            UprightEvidence {
                cue: UprightCue::NeckAndHead,
                strength: anatomical.then_some(neck).flatten(),
            },
            UprightEvidence {
                cue: UprightCue::PelvisMass,
                strength: anatomical.then_some(pelvis).flatten(),
            },
        ],
    }
}

/// The file's horizontal patient direction after the proposed display
/// turn. Front and side directions cannot be substituted for each other.
pub fn file_image_right(film: &DecodedFilm, turns: QuarterTurns) -> Option<ImageRight> {
    let axes = film
        .facts
        .kept_tags
        .as_ref()?
        .patient_orientation
        .as_ref()?;
    let right = orientation_right(axes, turns)?;
    match (film.facts.kind, right) {
        (FilmKind::Side, ImageRight::Anterior | ImageRight::Posterior)
        | (
            FilmKind::Front | FilmKind::BendLeft | FilmKind::BendRight,
            ImageRight::PatientLeft | ImageRight::PatientRight,
        ) => Some(right),
        _ => None,
    }
}

fn orientation_turns(axes: &[String; 2]) -> Option<QuarterTurns> {
    let [x, y] = axes.each_ref().map(|axis| axis.as_bytes().first().copied());
    let q = match (x, y) {
        (Some(b'H'), Some(b'A' | b'P' | b'L' | b'R')) => 3,
        (Some(b'F'), Some(b'A' | b'P' | b'L' | b'R')) => 1,
        (Some(b'A' | b'P' | b'L' | b'R'), Some(b'H')) => 2,
        (Some(b'A' | b'P' | b'L' | b'R'), Some(b'F')) => 0,
        _ => return None,
    };
    QuarterTurns::new(q)
}

fn orientation_right(axes: &[String; 2], turns: QuarterTurns) -> Option<ImageRight> {
    let q = turns.get();
    let letter = axes[usize::from(q % 2)].as_bytes().first()?;
    let right = match letter {
        b'L' => ImageRight::PatientLeft,
        b'R' => ImageRight::PatientRight,
        b'A' => ImageRight::Anterior,
        b'P' => ImageRight::Posterior,
        _ => return None,
    };
    Some(if q == 1 || q == 2 {
        right.mirrored()
    } else {
        right
    })
}

fn end_cues(profile: &Profile) -> (Option<f64>, Option<f64>, f64) {
    let max_mass = profile.rows.iter().map(|r| r.mass).fold(0.0_f64, f64::max);
    let occupied: Vec<_> = profile
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.mass > max_mass * 0.04 && row.width >= 2.0)
        .map(|(i, _)| i)
        .collect();
    let (Some(&top), Some(&bottom)) = (occupied.first(), occupied.last()) else {
        return (None, None, 0.0);
    };
    let span = bottom - top + 1;
    if span < 12 {
        return (None, None, 0.0);
    }
    let band = |start: f64, end: f64| {
        &profile.rows[top + (span as f64 * start) as usize
            ..(top + (span as f64 * end) as usize).min(bottom + 1)]
    };
    let mean = |rows: &[crate::rough_trace::TraceRow], mass: bool| {
        let values: Vec<_> = rows
            .iter()
            .filter(|r| r.width >= 2.0)
            .map(|r| if mass { r.mass } else { r.width })
            .collect();
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    let upper = band(0.04, 0.23);
    let lower = band(0.73, 0.96);
    let contrast = |a: Option<f64>, b: Option<f64>| {
        a.zip(b)
            .filter(|(a, b)| *a + *b > 0.0)
            .map(|(a, b)| ((b - a) / (0.5 * (a + b))).clamp(-1.0, 1.0))
    };
    let width = mean(band(0.15, 0.85), false).unwrap_or(profile.width as f64);
    // A long axis must be longer than the body is wide, even when the
    // film itself has broad blank borders or is stored sideways.
    let elongation = ((span as f64 / width.max(1.0) - 1.2) / 1.5).clamp(0.0, 1.0);
    (
        contrast(mean(upper, false), mean(lower, false)),
        contrast(mean(upper, true), mean(lower, true)),
        elongation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_axes_follow_clockwise_pixel_rotations() {
        // The original +x/+y axes of four differently stored upright
        // L/F images. Turning them clockwise must restore right=L.
        for (axes, expected) in [
            (["L", "F"], 0),
            (["F", "R"], 1),
            (["R", "H"], 2),
            (["H", "L"], 3),
        ] {
            let axes = axes.map(String::from);
            let q = orientation_turns(&axes).expect("orthogonal directions");
            assert_eq!(q.get(), expected);
            assert_eq!(orientation_right(&axes, q), Some(ImageRight::PatientLeft));
        }
        assert_eq!(orientation_turns(&["H".into(), "F".into()]), None);
        assert_eq!(orientation_turns(&["".into(), "F".into()]), None);
    }
}
