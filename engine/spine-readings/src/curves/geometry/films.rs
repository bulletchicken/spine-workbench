//! Film registration, bending endplate lines and physical pixel spacing.

use crate::curves::ReadReason;
use crate::curves::Reading;
use forward_model::endplates::Endplates;
use forward_model::vertebra_params::VertebraParams;
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::pixels::Px;
use shapes::spine_fit::FilmToFit;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::ReadingsInput;
use shapes::vertebra::VertebraMarks;

/// Film pixels to upright pixels, with no display mirroring.
pub fn upright(film: &FilmToFit, point: Px) -> [f64; 2] {
    let w = f64::from(film.size[0]) - 1.0;
    let h = f64::from(film.size[1]) - 1.0;
    let (x, y) = (point.x(), point.y());
    match film.geometry.quarter_turns.get() {
        0 => [x, y],
        1 => [h - y, x],
        2 => [w - x, h - y],
        _ => [y, w - x],
    }
}

fn spacing(film: &FilmToFit) -> Option<[f64; 2]> {
    let x = film.geometry.mm_per_px?;
    let y = film.geometry.mm_per_px_vertical.unwrap_or(x);
    if !x.is_finite() || !y.is_finite() || x <= 0.0 || y <= 0.0 {
        return None;
    }
    Some(if film.geometry.quarter_turns.get().is_multiple_of(2) {
        [x, y]
    } else {
        [y, x]
    })
}

/// Read the CSVL through marked sacrum ends. The nominal L5 inferior
/// corner midpoint registers its fitted lower-endplate centre to pixels;
/// the version's L5 is projected in that fixed registration. Subtracting
/// that projection and adding its y back leaves a fixed patient-frame line.
pub fn csvl(input: &ReadingsInput, fit: &SpineFit) -> Reading<f64> {
    let film = input.front.as_ref().ok_or(ReadReason::NoSacrum)?;
    let Some(SacrumTop::Front { sac_l, sac_r }) = film.marks.landmarks.sacrum_top else {
        return Err(ReadReason::NoSacrum);
    };
    if film.marks.film != FilmKind::Front {
        return Err(ReadReason::NoSacrum);
    }
    let sign = match film.geometry.image_right {
        Some(ImageRight::PatientLeft) => 1.0,
        Some(ImageRight::PatientRight) => -1.0,
        _ => return Err(ReadReason::NoSacrum),
    };
    let [su, _] = spacing(film).ok_or(ReadReason::NoSacrum)?;
    let nominal = input
        .fit
        .vertebrae
        .iter()
        .find(|v| v.level == Level::L5)
        .ok_or(ReadReason::NotFitted)?;
    let sampled = fit
        .vertebrae
        .iter()
        .find(|v| v.level == Level::L5)
        .ok_or(ReadReason::NotFitted)?;
    let marked = film
        .marks
        .vertebrae
        .iter()
        .find(|v| v.level == Level::L5)
        .ok_or(ReadReason::NotFitted)?;
    let VertebraMarks::Front { points } = marked.marks else {
        return Err(ReadReason::NoSacrum);
    };
    let bottom = Endplates::of(&VertebraParams::from(&nominal.pose))
        .bottom
        .centre
        .y;
    let version_bottom = Endplates::of(&VertebraParams::from(&sampled.pose))
        .bottom
        .centre
        .y;
    let observed = (upright(film, points.inf_l.at)[0] + upright(film, points.inf_r.at)[0]) * 0.5;
    let sacrum = (upright(film, sac_l.at)[0] + upright(film, sac_r.at)[0]) * 0.5;
    let projected = observed + sign * (version_bottom - bottom) / su;
    let line = version_bottom + sign * (sacrum - projected) * su;
    if line.is_finite() {
        Ok(line)
    } else {
        Err(ReadReason::NoSacrum)
    }
}

fn line_tilt(film: &FilmToFit, line: [shapes::marks::Mark; 2]) -> Reading<f64> {
    let [a, b] = line.map(|m| upright(film, m.at));
    let [su, sv] = spacing(film)
        .or_else(|| {
            // Angles remain readable without a physical scale only for square
            // pixels, which the geometry represents by an absent vertical spacing.
            (film.geometry.mm_per_px.is_none() && film.geometry.mm_per_px_vertical.is_none())
                .then_some([1.0, 1.0])
        })
        .ok_or(ReadReason::NoBendingFilms)?;
    let dx = (b[0] - a[0]) * su;
    let dz = -(b[1] - a[1]) * sv;
    let scale = dx.abs().max(dz.abs());
    if !dx.is_finite() || !dz.is_finite() || scale == 0.0 {
        return Err(ReadReason::NoBendingFilms);
    }
    let (dx, dz) = (dx / scale, dz / scale);
    // A line is undirected; doubling before atan2 removes its endpoint order.
    Ok(0.5 * libm::atan2(2.0 * dx * dz, dx * dx - dz * dz).to_degrees())
}

fn bending_lines(film: &FilmToFit, level: Level) -> Reading<shapes::marks::EndplateLines> {
    if !film.marks.film.is_bending() {
        return Err(ReadReason::NoBendingFilms);
    }
    let v = film
        .marks
        .vertebrae
        .iter()
        .find(|v| v.level == level)
        .ok_or(ReadReason::NoBendingFilms)?;
    match v.marks {
        VertebraMarks::Bending { lines } => Ok(lines),
        _ => Err(ReadReason::NoBendingFilms),
    }
}

fn line_difference(a: f64, b: f64) -> f64 {
    // Each line has already been assigned its upward-normal tilt. Their
    // difference can be obtuse, just like the standing endplate Cobb.
    a - b
}

/// Cobb on one bend, using the standing curve's end vertebrae.
pub fn bending_angle(film: &FilmToFit, upper: Level, lower: Level) -> Reading<f64> {
    let a = bending_lines(film, upper)?;
    let b = bending_lines(film, lower)?;
    Ok(line_difference(line_tilt(film, a.sup)?, line_tilt(film, b.inf)?).abs())
}

/// Smaller residual of both bends. A single film cannot establish the
/// smaller of two residuals, so both are required.
pub fn bending_cobb(input: &ReadingsInput, upper: Level, lower: Level) -> Reading<(f64, FilmKind)> {
    let left = input.bend_left.as_ref().ok_or(ReadReason::NoBendingFilms)?;
    let right = input
        .bend_right
        .as_ref()
        .ok_or(ReadReason::NoBendingFilms)?;
    if left.marks.film != FilmKind::BendLeft || right.marks.film != FilmKind::BendRight {
        return Err(ReadReason::NoBendingFilms);
    }
    let a = bending_angle(left, upper, lower)?;
    let b = bending_angle(right, upper, lower)?;
    Ok(if a <= b {
        (a, FilmKind::BendLeft)
    } else {
        (b, FilmKind::BendRight)
    })
}

/// Signed disc angle on one bend: upper vertebra bottom minus lower top,
/// positive when the disc opens to the patient's left.
pub fn bending_disc(film: &FilmToFit, upper: Level, lower: Level) -> Reading<f64> {
    let a = bending_lines(film, upper)?;
    let b = bending_lines(film, lower)?;
    let sign = match film.geometry.image_right {
        Some(ImageRight::PatientLeft) => 1.0,
        Some(ImageRight::PatientRight) => -1.0,
        _ => return Err(ReadReason::NoBendingFilms),
    };
    Ok(sign * line_difference(line_tilt(film, a.inf)?, line_tilt(film, b.sup)?))
}
