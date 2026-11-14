//! Angular body-contact boundaries from the two fitted endplate ellipses.
//! These are geometric equalities, with no paper threshold or angular grid.

use super::RuleContext;
use super::contact_roots::real_roots;
use forward_model::endplates::Endplates;
use forward_model::rotation::{rot_x, rot_y};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector3;
use shapes::implant_layout::Measurement;

/// Every change, in degrees, at which an endplate rim meets the CSVL
/// while one named angle turns. A body contact can begin or end only at
/// one of these equalities. Internal endplate equalities may add harmless
/// brackets; the rule's actual body union settles whether its level moves.
pub(super) fn angular_changes(
    context: &RuleContext<'_>,
    index: usize,
    measurement: Measurement,
) -> Vec<f64> {
    let Ok(line) = context.reading.csvl_y_mm else {
        return Vec::new();
    };
    let vertebra = &context.fit.vertebrae[index];
    let axis = match measurement {
        Measurement::CoronalTilt => Vector3::x(),
        Measurement::Twist => {
            // R=Rx(tilt) Ry(lean) Rz(twist): a twist increment is a
            // rigid turn around Rx Ry z in the patient's frame.
            rot_x(vertebra.angles.side_tilt_rad)
                * rot_y(vertebra.angles.forward_lean_rad)
                * Vector3::z()
        }
        _ => return Vec::new(),
    };
    let params = VertebraParams::from(&vertebra.pose);
    let plates = Endplates::of(&params);
    let mut changes = Vec::new();
    for plate in [plates.top, plates.bottom] {
        // Rodrigues gives every rotated y component as k+a*cos+b*sin.
        // Contact is (line-centre_y)^2 = (w*width_y)^2+(d*depth_y)^2.
        let centre = components(plate.centre - params.centre, axis);
        let centre = [line - params.centre.y - centre[0], -centre[1], -centre[2]];
        let width = components(plate.width_axis * plate.half_width, axis);
        let depth = components(plate.depth_axis * plate.half_depth, axis);
        let centre = half_tangent_numerator(centre);
        let width = half_tangent_numerator(width);
        let depth = half_tangent_numerator(depth);
        let mut polynomial = [0.; 5];
        for i in 0..3 {
            for j in 0..3 {
                polynomial[i + j] +=
                    centre[i] * centre[j] - width[i] * width[j] - depth[i] * depth[j];
            }
        }
        changes.extend(
            real_roots(&polynomial)
                .into_iter()
                .map(|root| (2. * libm::atan(root)).to_degrees()),
        );
    }
    // tan(delta/2) omits the half-turn itself.
    changes.extend([-180., 180.]);
    changes.sort_by(f64::total_cmp);
    changes.dedup_by(|a, b| a.to_bits() == b.to_bits());
    changes
}

fn components(vector: Vector3<f64>, axis: Vector3<f64>) -> [f64; 3] {
    let fixed = axis * axis.dot(&vector);
    [fixed.y, (vector - fixed).y, axis.cross(&vector).y]
}

// For t=tan(delta/2), multiply k+a*cos(delta)+b*sin(delta)
// by (1+t*t): (k+a)+2*b*t+(k-a)*t*t.
fn half_tangent_numerator([k, a, b]: [f64; 3]) -> [f64; 3] {
    [k + a, 2. * b, k - a]
}
