//! Closed-form transverse-segment projection, with widths read from the
//! same published model rather than repeated numerical table cells.

use super::ensemble::model_outlines;
use super::test_spine;
use crate::{curves, pedicles, readings};
use forward_model::pedicles::pedicle_centres;
use forward_model::vertebra_params::VertebraParams;
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

#[test]
fn a_sixty_degree_roll_halves_the_transverse_outline_in_patient_y() {
    // A transverse segment of length W rotated 60 degrees about patient
    // x projects to W*cos(60)=W/2 in patient y. This is the project's
    // explicit line-outline model, independent of any table's width.
    let mut input = test_spine::input();
    for v in &mut input.fit.vertebrae {
        v.pose.rotation = [60_f64.to_radians(), 0., 0.];
        v.angles.side_tilt_rad = 60_f64.to_radians();
        v.sd.centre_mm = [0.; 3];
        v.sd.angles_rad = [0.; 3];
    }
    let sampler = readings::NamedSampler::new(u64::from(input.seed), input.versions).unwrap();
    let version = readings::version(&input.fit, u64::from(input.seed), input.versions, 0).unwrap();
    let reading = curves::measure(&input, &version.fit).unwrap();
    let (outlines, sources) = model_outlines(&input, &version, &reading, &sampler).unwrap();
    let outline = outlines.iter().find(|o| o.level == Level::T5).unwrap();
    let centres = pedicle_centres(&VertebraParams::from(&version.fit.vertebrae[4].pose));
    for (side, interval, centre) in [
        (BodySide::Left, outline.left_y_mm, centres[0].y),
        (BodySide::Right, outline.right_y_mm, centres[1].y),
    ] {
        let position = pedicles::position_in_curve(&reading, Level::T5, side);
        let width = pedicles::drawn_outer_width(
            Level::T5,
            side,
            input.child.age_years,
            position,
            &version,
            &sampler,
            input.pedicle_child_share,
        )
        .unwrap()
        .unwrap();
        assert!((interval[1] - interval[0] - width * 0.5).abs() < 1e-12);
        assert!(((interval[0] + interval[1]) * 0.5 - centre).abs() < 1e-12);
    }
    assert!(
        sources
            .iter()
            .all(|source| crate::evidence::tables().source(source).is_some())
    );
}
