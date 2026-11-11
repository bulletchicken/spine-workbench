//! Stacks the vertebrae upward from L5, endplate on disc on endplate, and
//! checks the balance the recipe asked for.
//!
//! Each vertebra's endplates take the profiles' values a little either
//! side of its centre `t`: the share `ν` of the bend between two vertebrae
//! that the vertebrae take as wedge (the disc takes the rest) places them
//! at `t ∓ ν/2`. So
//!
//! ```text
//! front wedge  ωf = tilt(t − ν_c/2)    − tilt(t + ν_c/2)
//! side wedge   ωs = heading(t − ν_s/2) − heading(t + ν_s/2)
//! ```
//!
//! and the vertebra's rotation is `R = rotX(tilt) · rotY(heading) ·
//! rotZ(twist(t))`, with the tilt and heading midway between its
//! endplates'. In a curve convex to the right (tilt falling going up) the
//! right side is then taller, and in kyphosis (heading rising going up) the
//! front is shorter. Because every endplate's tilt is a value the profile
//! takes, no endplate tilts further than the profile's own extremes, the
//! end vertebrae, and each curve measures no more than it was built with.
//! The disc takes the rest of the bend through the stacking:
//!
//! 1. the last lumbar vertebra has its lower endplate centre at the origin;
//! 2. going up, the vertebra above sits on a disc of its own height, along
//!    the bisector of the two facing endplate normals;
//! 3. the sacrum sits under the last lumbar vertebra the same way, its top
//!    tilted forward by the sacral slope and to the side by the pelvic
//!    obliquity, the femoral heads in front of and below it so that the
//!    pelvic tilt is `PI − SS`;
//! 4. to bring C7 over the sacrum from side to side as the recipe asked,
//!    the whole body is turned about the forward axis through the origin,
//!    by at most 4°: a pelvis tipped further would be a different patient,
//!    so beyond that the trunk is left shifted to one side, as many
//!    scoliotic trunks are. A turn about that axis adds the same angle to
//!    every frontal tilt, so no Cobb angle changes. (Balance from front to
//!    back is the lordosis's job: see [`crate::patient`].)
//!
//! [`check_discs`] then makes sure no disc closes: every point of an upper
//! vertebra's lower rim stays at least `0.5·k` mm above the endplate below
//! it. A spine that fails it is refused and the recipe drawn again.

use crate::body_sizes::{TABLE_LENGTH_MM, VertebraSize};
use crate::curves::{CoronalProfile, SagittalProfile};
use crate::recipes::VertebralShare;
use forward_model::endplates::{Endplate, Endplates};
use forward_model::rotation::{rot_x, rot_y, rot_z};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::{Matrix3, Vector3};
use shapes::levels::{Level, Region};

/// The most the whole body is turned to balance it from side to side.
pub const MOST_BALANCE_DEG: f64 = 4.0;

/// How a stacked spine was asked to be built.
pub struct StackPlan<'a> {
    /// Top to bottom, from the highest cervical vertebra painted to the
    /// last lumbar.
    pub levels: &'a [Level],
    pub sizes: &'a [VertebraSize],
    pub coronal: &'a CoronalProfile,
    pub sagittal: &'a SagittalProfile,
    pub share: VertebralShare,
    /// A lean of the trunk toward one side, for bending films: an extra
    /// tilt (radians, positive toward the patient's right) that is zero at
    /// the last lumbar vertebra and grows evenly to its full size ten
    /// vertebrae above.
    pub trunk_lean: f64,
    pub pelvic_incidence_deg: f64,
    pub pelvic_obliquity_deg: f64,
    /// From the top of T1 to the top of S1, straightened.
    pub length_mm: f64,
    /// Where C7 should sit to the left of the centre of the top of the
    /// sacrum, mm; `None` leaves the body as stacked.
    pub coronal_balance: Option<f64>,
}

/// The pelvis under the spine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pelvis {
    /// The top of S1.
    pub sacrum_top: Endplate,
    /// The sacrum's axes in the patient frame, as columns: forward, left
    /// and up along the normal of its top.
    pub frame: Matrix3<f64>,
    /// The pelvis standing: the patient's axes tipped by the pelvic
    /// obliquity.
    pub upright: Matrix3<f64>,
    /// Midway between the femoral heads.
    pub hip_axis_centre: Vector3<f64>,
    /// The femoral head centres, patient left then right.
    pub hips: [Vector3<f64>; 2],
    pub hip_radius: f64,
}

/// A spine stacked from L5, top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct StackedSpine {
    pub levels: Vec<Level>,
    pub vertebrae: Vec<VertebraParams>,
    pub sizes: Vec<VertebraSize>,
    pub pelvis: Pelvis,
    /// How far the body was turned to balance it, degrees, positive when
    /// its right side went down.
    pub balance_deg: f64,
    /// `k`: the spine's lengths over the base table's.
    pub scale: f64,
}

impl StackedSpine {
    /// The vertebra at `level`, if the spine has it.
    pub fn vertebra(&self, level: Level) -> Option<&VertebraParams> {
        let index = self.levels.iter().position(|l| *l == level)?;
        self.vertebrae.get(index)
    }

    /// How far C7's centre lies in front of the back corner of the top of
    /// the sacrum, and to the left of its centre, mm.
    pub fn c7_offset(&self) -> Option<[f64; 2]> {
        let c7 = self.vertebra(Level::C7)?.centre;
        let top = &self.pelvis.sacrum_top;
        let corner = top.centre + top.depth_axis * top.half_depth;
        Some([c7.x - corner.x, c7.y - top.centre.y])
    }
}

/// Why a spine could not be stacked.
#[derive(Clone, Debug, PartialEq)]
pub enum StackFailure {
    /// The disc under this level closed.
    DiscCloses(Level),
    /// No lordosis in its range balances the spine from front to back; the
    /// lordosis tried last.
    OutOfBalance(f64),
    /// The plan has no levels, or sizes that do not match them.
    NothingToStack,
}

/// Stacks the plan's spine. See the module documentation for the steps.
pub fn stack(plan: &StackPlan) -> Result<StackedSpine, StackFailure> {
    let n = plan.levels.len();
    if n == 0 || plan.sizes.len() != n {
        return Err(StackFailure::NothingToStack);
    }
    let first_thoracic = plan
        .levels
        .iter()
        .position(|level| level.region() != Region::Cervical)
        .unwrap_or(0);
    let last_t = (n - 1 - first_thoracic) as f64;
    let positions: Vec<f64> = (0..n).map(|i| i as f64 - first_thoracic as f64).collect();
    let tilt = |t: f64| {
        let lean = ((last_t - t) / 10.0).clamp(0.0, 1.0);
        plan.coronal.tilt(t) + plan.trunk_lean * lean
    };

    let mut vertebrae: Vec<VertebraParams> = plan
        .levels
        .iter()
        .zip(plan.sizes)
        .zip(&positions)
        .map(|((level, size), &t)| {
            let lumbar = level.region() == Region::Lumbar;
            let (coronal_share, sagittal_share) = if lumbar {
                (plan.share.coronal_lumbar, plan.share.sagittal_lumbar)
            } else {
                (plan.share.coronal_thoracic, plan.share.sagittal_thoracic)
            };
            let (tilt_top, tilt_bottom) =
                (tilt(t - 0.5 * coronal_share), tilt(t + 0.5 * coronal_share));
            let heading = |t: f64| plan.sagittal.heading(t);
            let (heading_top, heading_bottom) = (
                heading(t - 0.5 * sagittal_share),
                heading(t + 0.5 * sagittal_share),
            );
            let rotation = rot_x(0.5 * (tilt_top + tilt_bottom))
                * rot_y(0.5 * (heading_top + heading_bottom))
                * rot_z(plan.coronal.twist(t));
            VertebraParams {
                centre: Vector3::zeros(),
                rotation: forward_model::rotation::rotation_vector_from_matrix(&rotation),
                height: size.height,
                wedge_front: tilt_top - tilt_bottom,
                wedge_side: heading_top - heading_bottom,
                top_half_width: size.top_half_width,
                top_half_depth: size.top_half_depth,
                bottom_half_width: size.bottom_half_width,
                bottom_half_depth: size.bottom_half_depth,
            }
        })
        .collect();

    // 1 and 2: from the last lumbar vertebra upward.
    let up = |v: &VertebraParams| v.rotation_matrix().column(2) * (0.5 * v.height);
    vertebrae[n - 1].centre = up(&vertebrae[n - 1]);
    for below in (1..n).rev() {
        let plates_below = Endplates::of(&vertebrae[below]);
        let mut above = vertebrae[below - 1];
        let plates_above = Endplates::of(&above);
        let along = (plates_below.top.normal + plates_above.bottom.normal).normalize();
        let bottom_above = plates_below.top.centre + along * plan.sizes[below - 1].disc_below;
        above.centre = bottom_above + up(&above);
        vertebrae[below - 1] = above;
    }

    // 3: the sacrum and the hips.
    let scale = plan.length_mm / TABLE_LENGTH_MM;
    let last = &vertebrae[n - 1];
    let last_plates = Endplates::of(last);
    let sacral_slope = plan.sagittal.heading(last_t + 1.0);
    let frame = rot_x(plan.pelvic_obliquity_deg.to_radians()) * rot_y(sacral_slope);
    let sacrum_normal: Vector3<f64> = frame.column(2).into();
    let along = (last_plates.bottom.normal + sacrum_normal).normalize();
    let sacrum_centre = -along * plan.sizes[n - 1].disc_below;
    let width_axis = {
        let left: Vector3<f64> = frame.column(1).into();
        (left - sacrum_normal * left.dot(&sacrum_normal)).normalize()
    };
    let sacrum_top = Endplate {
        centre: sacrum_centre,
        normal: sacrum_normal,
        width_axis,
        depth_axis: sacrum_normal.cross(&width_axis),
        half_width: last.bottom_half_width,
        half_depth: 0.95 * last.bottom_half_depth,
    };
    let length = plan.length_mm;
    let toward_hips =
        frame * rot_y(-plan.pelvic_incidence_deg.to_radians()) * Vector3::new(0.0, 0.0, -1.0);
    let hip_axis_centre = sacrum_centre + toward_hips * (0.24 * length);
    let half_apart: Vector3<f64> = frame.column(1) * (0.19 * length);
    let mut pelvis = Pelvis {
        sacrum_top,
        frame,
        upright: rot_x(plan.pelvic_obliquity_deg.to_radians()),
        hip_axis_centre,
        hips: [hip_axis_centre + half_apart, hip_axis_centre - half_apart],
        hip_radius: 0.05 * length,
    };

    // 4: balance from side to side by turning about the forward axis.
    let mut balance_deg = 0.0;
    if let Some(target) = plan.coronal_balance {
        let c7 = plan
            .levels
            .iter()
            .position(|l| *l == Level::C7)
            .ok_or(StackFailure::NothingToStack)?;
        let mut angle = 0.0;
        for _ in 0..8 {
            let turn = rot_x(angle);
            let a = turn * vertebrae[c7].centre;
            let base = turn * pelvis.sacrum_top.centre;
            // Turning about x moves a point at (y, z) along its circle; the
            // turn that puts C7 at the wanted y, with the sacrum held where
            // it now is, is found again until it settles.
            let radius = (a.y * a.y + a.z * a.z).sqrt();
            let wanted = (base.y + target).clamp(-radius, radius);
            angle += libm::atan2(a.y, a.z) - libm::asin(wanted / radius);
        }
        let most = MOST_BALANCE_DEG.to_radians();
        let angle = angle.clamp(-most, most);
        balance_deg = angle.to_degrees();
        let turn = rot_x(angle);
        for vertebra in &mut vertebrae {
            vertebra.centre = turn * vertebra.centre;
            vertebra.rotation = forward_model::rotation::rotation_vector_from_matrix(
                &(turn * vertebra.rotation_matrix()),
            );
        }
        pelvis = turned_pelvis(&pelvis, &turn);
    }

    Ok(StackedSpine {
        levels: plan.levels.to_vec(),
        vertebrae,
        sizes: plan.sizes.to_vec(),
        pelvis,
        balance_deg,
        scale,
    })
}

/// Whether every disc of `spine` stays open, the lumbosacral one
/// included: no point of an upper vertebra's lower rim comes within
/// `0.5·k` mm of the plane of the endplate below.
pub fn check_discs(spine: &StackedSpine) -> Result<(), StackFailure> {
    let clearance = 0.5 * spine.scale;
    let plates: Vec<Endplates> = spine.vertebrae.iter().map(Endplates::of).collect();
    for below in 1..plates.len() {
        if rim_clearance(&plates[below - 1].bottom, &plates[below].top) < clearance {
            return Err(StackFailure::DiscCloses(spine.levels[below - 1]));
        }
    }
    let (Some(last), Some(level)) = (plates.last(), spine.levels.last()) else {
        return Err(StackFailure::NothingToStack);
    };
    if rim_clearance(&last.bottom, &spine.pelvis.sacrum_top) < clearance {
        return Err(StackFailure::DiscCloses(*level));
    }
    Ok(())
}

/// The smallest height of `upper`'s rim above the plane of `lower`, over
/// 64 points round the rim.
fn rim_clearance(upper: &Endplate, lower: &Endplate) -> f64 {
    (0..64)
        .map(|i| {
            let t = core::f64::consts::TAU * f64::from(i) / 64.0;
            (upper.rim_point(t) - lower.centre).dot(&lower.normal)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The pelvis turned by `turn` about the origin.
fn turned_pelvis(pelvis: &Pelvis, turn: &Matrix3<f64>) -> Pelvis {
    let top = &pelvis.sacrum_top;
    Pelvis {
        sacrum_top: Endplate {
            centre: turn * top.centre,
            normal: turn * top.normal,
            width_axis: turn * top.width_axis,
            depth_axis: turn * top.depth_axis,
            half_width: top.half_width,
            half_depth: top.half_depth,
        },
        frame: turn * pelvis.frame,
        upright: turn * pelvis.upright,
        hip_axis_centre: turn * pelvis.hip_axis_centre,
        hips: pelvis.hips.map(|hip| turn * hip),
        hip_radius: pelvis.hip_radius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_sizes::base_sizes;
    use crate::curves::{Curve, CurveGenerator, SagittalAngles};
    use shapes::made_up::CurveRegion;
    use shapes::naming::Template;

    const FLAT: SagittalAngles = SagittalAngles {
        sacral_slope: 0.0,
        lordosis: 0.0,
        kyphosis: 0.0,
        upper_thoracic: 0.0,
        cervical_lordosis: 0.0,
    };

    const NO_SHARE: VertebralShare = VertebralShare {
        coronal_thoracic: 0.0,
        coronal_lumbar: 0.0,
        sagittal_thoracic: 0.0,
        sagittal_lumbar: 0.0,
    };

    fn levels() -> Vec<Level> {
        let mut levels = vec![Level::C6];
        levels.extend(Template::Standard.levels());
        levels
    }

    fn plan_stack(
        curves: &[Curve],
        sagittal: &SagittalAngles,
        share: VertebralShare,
        coronal_balance: Option<f64>,
    ) -> StackedSpine {
        let levels = levels();
        let sizes = base_sizes(&levels, 0.9);
        let coronal =
            CoronalProfile::new(CurveGenerator::Bump, curves, 17.0).expect("curves solve");
        let sagittal = SagittalProfile::new(sagittal, 12, 5);
        stack(&StackPlan {
            levels: &levels,
            sizes: &sizes,
            coronal: &coronal,
            sagittal: &sagittal,
            share,
            trunk_lean: 0.0,
            pelvic_incidence_deg: 50.0,
            pelvic_obliquity_deg: 0.0,
            length_mm: 405.0,
            coronal_balance,
        })
        .expect("the spine stacks")
    }

    #[test]
    fn a_straight_spine_stands_on_the_z_axis_at_its_table_length() {
        let spine = plan_stack(&[], &FLAT, NO_SHARE, None);
        for v in &spine.vertebrae {
            assert!(v.centre.x.abs() < 1e-12 && v.centre.y.abs() < 1e-12);
        }
        // Worked by hand: T1 top to S1 top straightened is the table's
        // heights and discs, 480.6 mm, times k = 405 / 450 = 0.9.
        let t1 = spine.vertebra(Level::T1).expect("T1");
        let t1_top = t1.centre.z + 0.5 * t1.height;
        let s1_top = spine.pelvis.sacrum_top.centre.z;
        assert!((t1_top - s1_top - 0.9 * 480.6).abs() < 1e-9);
        // L5's lower endplate centre is the origin.
        let l5 = spine.vertebra(Level::L5).expect("L5");
        assert!((l5.centre - Vector3::new(0.0, 0.0, 0.5 * l5.height)).norm() < 1e-12);
    }

    #[test]
    fn every_disc_has_its_height_along_the_bisector() {
        let curves = [Curve {
            region: CurveRegion::MainThoracic,
            upper_end: 4.0,
            lower_end: 11.0,
            apex: 7.5,
            cobb_deg: 50.0,
            convex_right: true,
            twist_per_cobb: 0.3,
        }];
        let sagittal = SagittalAngles {
            sacral_slope: 40.0,
            lordosis: 50.0,
            kyphosis: 40.0,
            upper_thoracic: 4.0,
            cervical_lordosis: 15.0,
        };
        let share = VertebralShare {
            coronal_thoracic: 0.5,
            coronal_lumbar: 0.3,
            sagittal_thoracic: 0.5,
            sagittal_lumbar: 0.2,
        };
        let spine = plan_stack(&curves, &sagittal, share, None);
        let l5 = spine.vertebrae.last().expect("L5");
        assert!(Endplates::of(l5).bottom.centre.norm() < 1e-12);
        for pair in spine.vertebrae.windows(2).zip(spine.sizes.iter()) {
            let ([above, below], size) = (pair.0, pair.1) else {
                unreachable!()
            };
            let (a, b) = (Endplates::of(above), Endplates::of(below));
            assert!(((a.top.centre - a.bottom.centre).norm() - above.height).abs() < 1e-9);
            let gap = a.bottom.centre - b.top.centre;
            let bisector = (a.bottom.normal + b.top.normal).normalize();
            assert!((gap.norm() - size.disc_below).abs() < 1e-9);
            assert!(gap.cross(&bisector).norm() < 1e-9);
        }
    }

    #[test]
    fn the_hips_sit_so_that_pelvic_tilt_is_incidence_minus_slope() {
        let sagittal = SagittalAngles {
            sacral_slope: 38.0,
            ..FLAT
        };
        let spine = plan_stack(&[], &sagittal, NO_SHARE, None);
        let pelvis = &spine.pelvis;
        // Pelvic tilt: the angle from vertical of the line from the hip
        // axis to the centre of the top of the sacrum, positive when the
        // hips are in front.
        let to_sacrum = pelvis.sacrum_top.centre - pelvis.hip_axis_centre;
        let tilt = libm::atan2(-to_sacrum.x, to_sacrum.z).to_degrees();
        assert!((tilt - (50.0 - 38.0)).abs() < 1e-9, "{tilt}");
        // Slope: the sacral normal leans forward by 38°.
        let slope = libm::atan2(pelvis.sacrum_top.normal.x, pelvis.sacrum_top.normal.z);
        assert!((slope.to_degrees() - 38.0).abs() < 1e-9);
        let apart = pelvis.hips[0] - pelvis.hips[1];
        assert!((apart.norm() - 0.38 * 405.0).abs() < 1e-9 && apart.y > 0.0);
    }

    #[test]
    fn balancing_puts_c7_where_the_recipe_asked() {
        let sagittal = SagittalAngles {
            sacral_slope: 40.0,
            lordosis: 50.0,
            kyphosis: 40.0,
            upper_thoracic: 4.0,
            cervical_lordosis: 15.0,
        };
        let curves = [Curve {
            region: CurveRegion::MainThoracic,
            upper_end: 4.0,
            lower_end: 11.0,
            apex: 7.5,
            cobb_deg: 40.0,
            convex_right: true,
            twist_per_cobb: 0.3,
        }];
        let level = plan_stack(&curves, &sagittal, NO_SHARE, None);
        let [_, as_stacked] = level.c7_offset().expect("a C7");
        // Ten millimetres further left: well inside a 4° turn.
        let spine = plan_stack(&curves, &sagittal, NO_SHARE, Some(as_stacked + 10.0));
        let [_, left] = spine.c7_offset().expect("a C7");
        assert!((left - as_stacked - 10.0).abs() < 1e-6, "{left}");
        assert!(spine.balance_deg.abs() > 0.1 && spine.balance_deg.abs() < MOST_BALANCE_DEG);
        // A metre further: the turn stops at 4° and the trunk stays
        // shifted.
        let far = plan_stack(&curves, &sagittal, NO_SHARE, Some(as_stacked + 1000.0));
        assert!((far.balance_deg.abs() - MOST_BALANCE_DEG).abs() < 1e-12);
        // A turn about the forward axis adds the same angle to every
        // frontal tilt, so tilt differences do not change.
        let tilt = |s: &StackedSpine, i: usize| {
            let n = Endplates::of(&s.vertebrae[i]).top.normal;
            libm::atan2(-n.y, n.z)
        };
        let before = tilt(&level, 5) - tilt(&level, 12);
        let after = tilt(&spine, 5) - tilt(&spine, 12);
        assert!((before - after).abs() < 1e-12);
    }

    #[test]
    fn a_disc_that_would_close_is_refused() {
        let levels = levels();
        let sizes = base_sizes(&levels, 0.9);
        let curves = [Curve {
            region: CurveRegion::MainThoracic,
            upper_end: 6.0,
            lower_end: 8.0,
            apex: 7.0,
            cobb_deg: 170.0,
            convex_right: true,
            twist_per_cobb: 0.0,
        }];
        let coronal =
            CoronalProfile::new(CurveGenerator::Bump, &curves, 17.0).expect("curves solve");
        let sagittal = SagittalProfile::new(&FLAT, 12, 5);
        let spine = stack(&StackPlan {
            levels: &levels,
            sizes: &sizes,
            coronal: &coronal,
            sagittal: &sagittal,
            share: NO_SHARE,
            trunk_lean: 0.0,
            pelvic_incidence_deg: 50.0,
            pelvic_obliquity_deg: 0.0,
            length_mm: 405.0,
            coronal_balance: None,
        })
        .expect("stacks");
        let result = check_discs(&spine);
        assert!(
            matches!(result, Err(StackFailure::DiscCloses(_))),
            "{result:?}"
        );
        assert_eq!(check_discs(&plan_stack(&[], &FLAT, NO_SHARE, None)), Ok(()));
    }
}
