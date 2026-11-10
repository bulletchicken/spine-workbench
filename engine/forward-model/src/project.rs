//! A vertebra and a camera to the marks the app shows: front marks, side marks
//! and bending-film endplate lines.
//!
//! The names follow the patient, never the screen: `L` is the patient's
//! left and `A` anterior whichever way the film faces, and the camera
//! decides where on the film each lands. Corners are placed as the auto
//! pass places them (`auto`, not pinned, not inferred). Pedicles come from
//! the starting fractions in [`pedicles`](crate::pedicles), so they are
//! always marked inferred.
//!
//! Each call turns the vertebra once and makes no allocation.
//!
//! # What each call needs
//!
//! [`front_marks`] and [`endplate_lines`] need a camera facing the
//! patient's left or right (a front or bending film); [`side_marks`] needs
//! one facing anterior or posterior (a side film). Nothing checks the
//! pairing, since a [`Camera`] need not have one facing. Paired wrongly,
//! an unturned vertebra fails with [`AxisAlongBeam`], because the axis
//! that names its corners then points along the beam, but a turned one
//! returns marks that mean nothing.
//!
//! The vertebra must hold the values
//! [`VertebraParams`](crate::vertebra_params::VertebraParams#allowed-values)
//! allows. Outside them a call returns `Ok` with meaningless marks rather
//! than an error.

use crate::cameras::Camera;
use crate::corners::{AxisAlongBeam, CornerAxis};
use crate::endplates::Endplates;
use crate::pedicles::pedicle_centres_turned;
use crate::vertebra_params::VertebraParams;
use nalgebra::Vector2;
use shapes::marks::{EndplateLines, FrontMarks, Mark, MarkSource, PedicleMark, SideMarks};
use shapes::pixels::Px;

/// The six marks of `vertebra` on a front film: its four body corners and
/// its two pedicle centres.
pub fn front_marks<C: Camera + ?Sized>(
    vertebra: &VertebraParams,
    camera: &C,
) -> Result<FrontMarks, AxisAlongBeam> {
    let rotation = vertebra.rotation_matrix();
    let plates = Endplates::turned(vertebra, &rotation);
    let [sup_l, sup_r] = camera.corners(&plates.top, CornerAxis::LeftRight)?;
    let [inf_l, inf_r] = camera.corners(&plates.bottom, CornerAxis::LeftRight)?;
    let [ped_l, ped_r] = pedicle_centres_turned(vertebra, &rotation);
    Ok(FrontMarks {
        sup_l: corner(sup_l),
        sup_r: corner(sup_r),
        inf_l: corner(inf_l),
        inf_r: corner(inf_r),
        ped_l: pedicle(camera.project(&ped_l)),
        ped_r: pedicle(camera.project(&ped_r)),
    })
}

/// The four body corners of `vertebra` on a side film.
pub fn side_marks<C: Camera + ?Sized>(
    vertebra: &VertebraParams,
    camera: &C,
) -> Result<SideMarks, AxisAlongBeam> {
    let plates = Endplates::of(vertebra);
    let [sup_a, sup_p] = camera.corners(&plates.top, CornerAxis::FrontBack)?;
    let [inf_a, inf_p] = camera.corners(&plates.bottom, CornerAxis::FrontBack)?;
    Ok(SideMarks {
        sup_a: corner(sup_a),
        sup_p: corner(sup_p),
        inf_a: corner(inf_a),
        inf_p: corner(inf_p),
    })
}

/// The two endplate lines of `vertebra` on a bending film, each with its
/// ends patient left, then patient right.
pub fn endplate_lines<C: Camera + ?Sized>(
    vertebra: &VertebraParams,
    camera: &C,
) -> Result<EndplateLines, AxisAlongBeam> {
    let plates = Endplates::of(vertebra);
    let sup = camera.corners(&plates.top, CornerAxis::LeftRight)?;
    let inf = camera.corners(&plates.bottom, CornerAxis::LeftRight)?;
    Ok(EndplateLines {
        sup: sup.map(corner),
        inf: inf.map(corner),
    })
}

fn px(at: Vector2<f64>) -> Px {
    Px::new(at.x, at.y)
}

fn corner(at: Vector2<f64>) -> Mark {
    Mark::auto(px(at))
}

fn pedicle(at: Vector2<f64>) -> PedicleMark {
    PedicleMark::Visible {
        at: px(at),
        source: MarkSource::Auto,
        pinned: false,
        inferred: true,
        ring_score: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cameras::parallel::ParallelCamera;
    use nalgebra::Vector3;
    use shapes::film_geometry::{FilmGeometry, ImageRight};

    fn camera(image_right: ImageRight) -> ParallelCamera {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            image_right: Some(image_right),
            ..FilmGeometry::UNKNOWN
        };
        ParallelCamera::for_film(&geometry, [2000, 4000], Vector2::new(1000.0, 3000.0))
            .expect("the geometry is complete")
    }

    fn straight() -> VertebraParams {
        VertebraParams {
            centre: Vector3::new(0.0, 10.0, 100.0),
            rotation: Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        }
    }

    #[test]
    fn corners_are_placed_by_the_auto_pass_and_pedicles_are_inferred() {
        let marks = front_marks(&straight(), &camera(ImageRight::PatientLeft))
            .expect("a straight vertebra has corners");
        // Worked by hand: the top endplate is at z = 110, so v = 3000 − 220;
        // its left end at y = 25, so u = 1000 + 50.
        assert_eq!(marks.sup_l, Mark::auto(Px::new(1050.0, 2780.0)));
        let PedicleMark::Visible {
            source,
            pinned,
            inferred,
            ring_score,
            ..
        } = marks.ped_l
        else {
            panic!("a projected pedicle is visible");
        };
        assert_eq!(
            (source, pinned, inferred, ring_score),
            (MarkSource::Auto, false, true, None)
        );
    }

    #[test]
    fn bending_lines_are_the_front_corners() {
        let v = VertebraParams {
            rotation: Vector3::new(0.2, -0.1, 0.3),
            wedge_front: 0.1,
            ..straight()
        };
        let cam = camera(ImageRight::PatientRight);
        let front = front_marks(&v, &cam).expect("corners");
        let lines = endplate_lines(&v, &cam).expect("lines");
        assert_eq!(lines.sup, [front.sup_l, front.sup_r]);
        assert_eq!(lines.inf, [front.inf_l, front.inf_r]);
    }

    #[test]
    fn a_vertebra_seen_end_on_has_no_corners() {
        // A quarter turn about its own up axis puts its front-to-back axis
        // along y, the beam of a side film. Its width axis then lies along
        // x, the beam of a front film.
        let v = VertebraParams {
            rotation: Vector3::new(0.0, 0.0, core::f64::consts::FRAC_PI_2),
            ..straight()
        };
        assert_eq!(
            side_marks(&v, &camera(ImageRight::Anterior)),
            Err(AxisAlongBeam)
        );
        assert_eq!(
            front_marks(&v, &camera(ImageRight::PatientLeft)),
            Err(AxisAlongBeam)
        );
        assert!(side_marks(&straight(), &camera(ImageRight::Anterior)).is_ok());
    }

    #[test]
    fn a_vertebra_just_short_of_end_on_is_answered() {
        // Worked by hand: turned θ = π/2 − 1e-6 about its own up axis, the
        // width axis ŷ goes to (−sin θ, cos θ, 0) and the depth axis −x̂ to
        // (−cos θ, −sin θ, 0). On the front film (across is y) the width
        // axis shows only cos θ = sin 1e-6 of its length, and the rim
        // reaches √((15 cos θ)² + (10 sin θ)²) = √(100 + 125 cos² θ) mm
        // across, 10 mm to within 1e-11. So the top corners land at
        // y = 10 ± 10: u = 1040 (left) and 1000, v = 2780. On the side
        // film (across is x) the rim reaches √((15 sin θ)² + (10 cos θ)²),
        // 15 mm to within 1e-11, so the anterior corner is at
        // u = 1000 + 30 and the posterior one at 970.
        let v = VertebraParams {
            rotation: Vector3::new(0.0, 0.0, core::f64::consts::FRAC_PI_2 - 1e-6),
            ..straight()
        };
        let near =
            |mark: Mark, u: f64, v: f64| libm::hypot(mark.at.x() - u, mark.at.y() - v) <= 1e-9;
        let front = front_marks(&v, &camera(ImageRight::PatientLeft))
            .expect("a front film answers a vertebra just short of end-on");
        assert!(near(front.sup_l, 1040.0, 2780.0), "{:?}", front.sup_l);
        assert!(near(front.sup_r, 1000.0, 2780.0), "{:?}", front.sup_r);
        let side = side_marks(&v, &camera(ImageRight::Anterior))
            .expect("a side film answers the same vertebra");
        assert!(near(side.sup_a, 1030.0, 2780.0), "{:?}", side.sup_a);
        assert!(near(side.sup_p, 970.0, 2780.0), "{:?}", side.sup_p);
    }
}
