//! Side-plane endplates and the sacrum top in the fit's film placement.

use crate::curves::geometry::{sagittal_tilt_deg, upright};
use forward_model::{endplates::Endplates, vertebra_params::VertebraParams};
use shapes::film::FilmKind;
use shapes::film_geometry::ImageRight;
use shapes::landmarks::SacrumTop;
use shapes::levels::Level;
use shapes::spine_fit::SpineFit;
use shapes::spine_readings::ReadingsInput;

pub(crate) fn plates(fit: &SpineFit) -> Vec<[f64; 2]> {
    fit.vertebrae
        .iter()
        .map(|v| {
            let p = Endplates::of(&VertebraParams::from(&v.pose));
            [
                sagittal_tilt_deg(&p.top.normal),
                sagittal_tilt_deg(&p.bottom.normal),
            ]
        })
        .collect()
}

/// No sacrum angle is supplied by a typical fitted vertebra. Only actual
/// marks on a side film, transformed through its fitted spacing, give it.
pub(crate) fn sacrum_tilt(input: &ReadingsInput) -> Option<f64> {
    let film = input.side.as_ref()?;
    let placement = input.fit.side_film?;
    if film.marks.film != FilmKind::Side {
        return None;
    }
    let Some(SacrumTop::Side { sac_a, sac_p }) = film.marks.landmarks.sacrum_top else {
        return None;
    };
    let sign = match film.geometry.image_right? {
        ImageRight::Anterior => 1.0,
        ImageRight::Posterior => -1.0,
        _ => return None,
    };
    let spacing = placement.mm_per_px;
    if spacing.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return None;
    }
    let [su, sv] = if film.geometry.quarter_turns.get().is_multiple_of(2) {
        spacing
    } else {
        [spacing[1], spacing[0]]
    };
    // origin_px is subtracted explicitly: the two points live in the same
    // patient frame as the fit. It cancels in their direction, as it must.
    let point = |mark: shapes::marks::Mark| {
        let [u, v] = upright(film, mark.at);
        [
            sign * (u - placement.origin_px[0]) * su,
            -(v - placement.origin_px[1]) * sv,
        ]
    };
    let a = point(sac_a);
    let p = point(sac_p);
    let dx = a[0] - p[0];
    let dz = a[1] - p[1];
    if !dx.is_finite() || !dz.is_finite() || (dx == 0.0 && dz == 0.0) {
        return None;
    }
    // The upward normal to the undirected sacrum line has angle -atan(dz/dx).
    Some(-0.5 * libm::atan2(2.0 * dx * dz, dx * dx - dz * dz).to_degrees())
}

pub(crate) fn level_index(fit: &SpineFit, level: Level) -> Option<usize> {
    fit.vertebrae.iter().position(|v| v.level == level)
}
