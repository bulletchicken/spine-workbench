//! The answer key: the true marks, levels, geometry and landmarks of every
//! film, and the angles measured from the made-up spine's own geometry.
//!
//! Every true mark is the forward model's projection of the true pose
//! through the film's own camera, exactly: corners from
//! `forward_model::project`, the sacrum's from the camera's closed-form
//! corners, centres from `Camera::project`. What the forward model does not
//! say is whether the film shows a point, and that is decided here:
//!
//! - a point off the film or outside the collimated field is not visible;
//! - a pedicle is visible on a front film only when it would show as a
//!   ring: its axis within 35° of the beam, and at least half of its outline
//!   inside the body's outline (a pedicle turned past the side of its body
//!   is hidden in the body's shadow, as on real films of rotated
//!   vertebrae);
//! - a vertebra seen exactly along an endplate axis has no corners: its
//!   marks are then the projected endplate centres, none visible;
//! - C7 is the mean of the true C7 vertebra's four corners (a bending
//!   film's line ends), visible exactly when all four are.
//!
//! The angles come from the patient-frame endplate normals of the standing
//! spine, never from the recipe: the frontal Cobb angle of each curve is
//! the largest difference in frontal tilt `atan2(−n_y, n_z)` between an
//! upper endplate and a lower endplate at or below it, within one vertebra
//! of the curve's end vertebrae; kyphosis runs from the top of T4 to the
//! bottom of the last thoracic vertebra and lordosis from the top of L1 to
//! the top of S1, both in sagittal tilt `atan2(n_x, n_z)`; the apical twist
//! is the axial angle `ψ` of the vertebra nearest each apex, read from its
//! rotation as `rotX · rotY · rotZ(ψ)`.

use crate::body_sizes::VertebraSize;
use crate::curves::Curve;
use crate::film_layout::FilmLayout;
use crate::film_painter::bodies::{PedicleTube, pedicles};
use crate::patient::Patient;
use crate::stack_from_l5::StackedSpine;
use forward_model::cameras::Camera;
use forward_model::corners::CornerAxis;
use forward_model::endplates::Endplates;
use forward_model::project::{endplate_lines, front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector3;
use shapes::film::FilmKind;
use shapes::levels::{Level, Region};
use shapes::made_up::{
    AnswerKey, CurveAngle, FilmAnswer, MadeUpRecipe, TrueLandmarks, TrueMark, TrueMarks,
    TrueVertebra, TruthAngles,
};
use shapes::marks::Mark;
use shapes::pixels::Px;

/// The largest angle between a pedicle's axis and the beam at which it
/// still shows as a ring.
pub const RING_MOST_ANGLE_DEG: f64 = 35.0;

/// The answer key of a made-up case.
pub fn answer_key(recipe: &MadeUpRecipe, patient: &Patient, layouts: &[FilmLayout]) -> AnswerKey {
    let template = patient.recipe.template;
    let levels = template.levels();
    let standing = &patient.standing;
    AnswerKey {
        recipe: *recipe,
        template,
        poses: poses(standing, &levels),
        films: layouts
            .iter()
            .map(|layout| {
                let spine = spine_for(patient, layout.kind);
                let mut answer = film_answer(layout, spine, &levels);
                if layout.kind.is_bending() {
                    answer.bent_poses = Some(poses(spine, &levels));
                }
                answer
            })
            .collect(),
        angles: truth_angles(standing, &patient.recipe.curves),
    }
}

/// The spine a film shows: standing on the front and side films, bent on
/// the bending films.
pub fn spine_for(patient: &Patient, kind: FilmKind) -> &StackedSpine {
    match kind {
        FilmKind::Front | FilmKind::Side => &patient.standing,
        FilmKind::BendLeft => &patient.bent_left,
        FilmKind::BendRight => &patient.bent_right,
    }
}

/// The poses of `levels`, top to bottom.
fn poses(spine: &StackedSpine, levels: &[Level]) -> Vec<shapes::made_up::VertebraPose> {
    levels
        .iter()
        .filter_map(|&level| spine.vertebra(level).map(|v| v.pose(level)))
        .collect()
}

/// The truth on one film of `spine`, for `levels` (a count's levels,
/// which always start at C7). `bent_poses` is left `None`.
pub fn film_answer(layout: &FilmLayout, spine: &StackedSpine, levels: &[Level]) -> FilmAnswer {
    let vertebrae: Vec<TrueVertebra> = levels
        .iter()
        .filter_map(|&level| {
            let index = spine.levels.iter().position(|l| *l == level)?;
            Some(TrueVertebra {
                level,
                marks: true_marks(layout, &spine.vertebrae[index], level, &spine.sizes[index]),
            })
        })
        .collect();
    let c7 = vertebrae
        .iter()
        .find(|v| v.level == Level::C7)
        .map(|v| mean_of_corners(&v.marks))
        .expect("every count's levels start at C7");
    FilmAnswer {
        kind: layout.kind,
        geometry: layout.geometry.clone(),
        vertebrae,
        landmarks: TrueLandmarks {
            c7,
            sacrum_top: sacrum_top(layout, spine),
            hips: spine
                .pelvis
                .hips
                .map(|hip| seen(layout, layout.project(&hip))),
        },
        bent_poses: None,
    }
}

/// A point and whether the film shows it.
fn seen(layout: &FilmLayout, at: Px) -> TrueMark {
    TrueMark {
        at,
        visible: layout.shows(at),
    }
}

fn hidden(at: Px) -> TrueMark {
    TrueMark { at, visible: false }
}

fn at(mark: &Mark) -> Px {
    mark.at
}

/// One vertebra's true marks on a film.
pub fn true_marks(
    layout: &FilmLayout,
    vertebra: &VertebraParams,
    level: Level,
    size: &VertebraSize,
) -> TrueMarks {
    let camera = &layout.camera;
    let plates = Endplates::of(vertebra);
    let top = hidden(layout.project(&plates.top.centre));
    let bottom = hidden(layout.project(&plates.bottom.centre));
    match layout.kind {
        FilmKind::Front => match front_marks(vertebra, camera) {
            Ok(marks) => {
                let corners = [
                    at(&marks.sup_l),
                    at(&marks.sup_r),
                    at(&marks.inf_r),
                    at(&marks.inf_l),
                ];
                let tubes = pedicles(vertebra, level, size);
                let pedicle = |tube: &PedicleTube| {
                    let at = layout.project(&tube.centre);
                    TrueMark {
                        at,
                        visible: layout.shows(at) && shows_as_ring(layout, tube, &corners),
                    }
                };
                TrueMarks::Front {
                    sup_l: seen(layout, corners[0]),
                    sup_r: seen(layout, corners[1]),
                    inf_l: seen(layout, corners[3]),
                    inf_r: seen(layout, corners[2]),
                    ped_l: pedicle(&tubes[0]),
                    ped_r: pedicle(&tubes[1]),
                }
            }
            Err(_) => {
                let tubes = pedicles(vertebra, level, size);
                TrueMarks::Front {
                    sup_l: top,
                    sup_r: top,
                    inf_l: bottom,
                    inf_r: bottom,
                    ped_l: hidden(layout.project(&tubes[0].centre)),
                    ped_r: hidden(layout.project(&tubes[1].centre)),
                }
            }
        },
        FilmKind::Side => match side_marks(vertebra, camera) {
            Ok(marks) => TrueMarks::Side {
                sup_a: seen(layout, marks.sup_a.at),
                sup_p: seen(layout, marks.sup_p.at),
                inf_a: seen(layout, marks.inf_a.at),
                inf_p: seen(layout, marks.inf_p.at),
            },
            Err(_) => TrueMarks::Side {
                sup_a: top,
                sup_p: top,
                inf_a: bottom,
                inf_p: bottom,
            },
        },
        FilmKind::BendLeft | FilmKind::BendRight => match endplate_lines(vertebra, camera) {
            Ok(lines) => TrueMarks::Bending {
                sup: lines.sup.map(|mark| seen(layout, mark.at)),
                inf: lines.inf.map(|mark| seen(layout, mark.at)),
            },
            Err(_) => TrueMarks::Bending {
                sup: [top, top],
                inf: [bottom, bottom],
            },
        },
    }
}

/// Whether a pedicle shows as a ring: its axis close enough to the beam,
/// and at least half of its outline inside the body outline `corners`
/// (in quad order).
fn shows_as_ring(layout: &FilmLayout, tube: &PedicleTube, corners: &[Px; 4]) -> bool {
    let along_beam = tube.axis.dot(&layout.beam()).abs().min(1.0);
    if libm::acos(along_beam).to_degrees() > RING_MOST_ANGLE_DEG {
        return false;
    }
    let up = tube.axis.cross(&tube.across);
    let inside = (0..16)
        .filter(|&i| {
            let t = core::f64::consts::TAU * f64::from(i) / 16.0;
            let point = tube.centre
                + tube.across * (tube.half_width * libm::cos(t))
                + up * (tube.half_height * libm::sin(t));
            inside_polygon(layout.project(&point), corners)
        })
        .count();
    inside >= 8
}

/// Even–odd test of a point against a polygon.
fn inside_polygon(point: Px, polygon: &[Px]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + n - 1) % n]);
        if (a.y() > point.y()) != (b.y() > point.y()) {
            let cross_x = a.x() + (point.y() - a.y()) * (b.x() - a.x()) / (b.y() - a.y());
            if point.x() < cross_x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The four corners (a bending film's line ends) of a vertebra's marks.
pub fn corners_of(marks: &TrueMarks) -> [TrueMark; 4] {
    match *marks {
        TrueMarks::Front {
            sup_l,
            sup_r,
            inf_l,
            inf_r,
            ..
        } => [sup_l, sup_r, inf_r, inf_l],
        TrueMarks::Side {
            sup_a,
            sup_p,
            inf_a,
            inf_p,
        } => [sup_a, sup_p, inf_p, inf_a],
        TrueMarks::Bending { sup, inf } => [sup[0], sup[1], inf[1], inf[0]],
    }
}

/// The mean of the four corners, visible when all four are.
fn mean_of_corners(marks: &TrueMarks) -> TrueMark {
    let corners = corners_of(marks);
    let mean = |axis: fn(Px) -> f64| corners.iter().map(|c| axis(c.at)).sum::<f64>() / 4.0;
    TrueMark {
        at: Px::new(mean(Px::x), mean(Px::y)),
        visible: corners.iter().all(|c| c.visible),
    }
}

/// The top of the sacrum: its patient-left (or anterior) corner, its
/// centre, then its patient-right (or posterior) corner.
fn sacrum_top(layout: &FilmLayout, spine: &StackedSpine) -> [TrueMark; 3] {
    let top = &spine.pelvis.sacrum_top;
    let centre = layout.project(&top.centre);
    let axis = if layout.kind == FilmKind::Side {
        CornerAxis::FrontBack
    } else {
        CornerAxis::LeftRight
    };
    match layout.camera.corners(top, axis) {
        Ok([first, second]) => [
            seen(layout, Px::new(first.x, first.y)),
            seen(layout, centre),
            seen(layout, Px::new(second.x, second.y)),
        ],
        Err(_) => [hidden(centre); 3],
    }
}

/// The frontal tilt of a plane with normal `n`: the angle `rotX` gives it,
/// positive when its right side is lower.
pub fn frontal_tilt(n: &Vector3<f64>) -> f64 {
    libm::atan2(-n.y, n.z)
}

/// The sagittal tilt of a plane with normal `n`: the angle `rotY` gives
/// it, positive when its front is lower.
pub fn sagittal_tilt(n: &Vector3<f64>) -> f64 {
    libm::atan2(n.x, n.z)
}

/// The angles measured from the standing spine's endplates, one Cobb
/// angle and apical twist for each of `curves`.
pub fn truth_angles(spine: &StackedSpine, curves: &[Curve]) -> TruthAngles {
    let first_thoracic = spine
        .levels
        .iter()
        .position(|l| l.region() != Region::Cervical)
        .unwrap_or(0) as i64;
    let c7 = spine
        .levels
        .iter()
        .position(|l| *l == Level::C7)
        .unwrap_or(0) as i64;
    let last = spine.levels.len() as i64 - 1;
    let plates: Vec<Endplates> = spine.vertebrae.iter().map(Endplates::of).collect();
    let index_of = |t: f64| first_thoracic + libm::round(t) as i64;
    let frontal_cobb = curves
        .iter()
        .map(|curve| {
            let from = (index_of(libm::floor(curve.upper_end)) - 1).clamp(c7, last) as usize;
            let to = (index_of(libm::ceil(curve.lower_end)) + 1).clamp(c7, last) as usize;
            let mut most = 0.0f64;
            for upper in from..=to {
                for lower in upper..=to {
                    let difference = frontal_tilt(&plates[upper].top.normal)
                        - frontal_tilt(&plates[lower].bottom.normal);
                    most = most.max(difference.abs());
                }
            }
            CurveAngle {
                region: curve.region,
                degrees: most.to_degrees(),
            }
        })
        .collect();
    let apical_twist = curves
        .iter()
        .map(|curve| {
            let apex = index_of(curve.apex).clamp(c7, last) as usize;
            let r = spine.vertebrae[apex].rotation_matrix();
            CurveAngle {
                region: curve.region,
                degrees: libm::atan2(-r[(0, 1)], r[(0, 0)]).to_degrees(),
            }
        })
        .collect();
    let plate_of = |level: Level| {
        spine
            .levels
            .iter()
            .position(|l| *l == level)
            .map(|index| plates[index])
    };
    let last_thoracic = spine
        .levels
        .iter()
        .rev()
        .find(|l| l.region() == Region::Thoracic)
        .copied()
        .unwrap_or(Level::T12);
    let sagittal = |a: Option<Vector3<f64>>, b: Option<Vector3<f64>>| match (a, b) {
        (Some(a), Some(b)) => (sagittal_tilt(&a) - sagittal_tilt(&b)).to_degrees(),
        _ => 0.0,
    };
    TruthAngles {
        frontal_cobb,
        apical_twist,
        kyphosis_degrees: sagittal(
            plate_of(Level::T4).map(|p| p.top.normal),
            plate_of(last_thoracic).map(|p| p.bottom.normal),
        ),
        lordosis_degrees: sagittal(
            Some(spine.pelvis.sacrum_top.normal),
            plate_of(Level::L1).map(|p| p.top.normal),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_sizes::base_sizes;
    use crate::curves::{CoronalProfile, CurveGenerator, SagittalAngles, SagittalProfile};
    use crate::film_layout::FilmLayout;
    use crate::patient::{make_up_patient, painted_levels};
    use crate::recipes::{VertebralShare, film_recipes};
    use crate::stack_from_l5::{StackPlan, stack};
    use forward_model::cameras::parallel::ParallelCamera;
    use nalgebra::Vector2;
    use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
    use shapes::made_up::{CurveRegion, RecipeName, Tier};
    use shapes::naming::Template;

    fn case(name: RecipeName, seed: u32, tier: Tier) -> (Patient, Vec<FilmLayout>, AnswerKey) {
        let recipe = MadeUpRecipe { name, seed, tier };
        let patient = make_up_patient(name, seed).expect("stacks");
        let layouts: Vec<FilmLayout> = film_recipes(&recipe, u64::from(recipe.seed))
            .iter()
            .map(|film| FilmLayout::new(film, spine_for(&patient, film.kind)))
            .collect();
        let key = answer_key(&recipe, &patient, &layouts);
        (patient, layouts, key)
    }

    /// A film with a hand-chosen camera.
    fn film(kind: FilmKind, image_right: ImageRight) -> FilmLayout {
        let geometry = FilmGeometry {
            mm_per_px: Some(0.5),
            image_right: Some(image_right),
            ..FilmGeometry::UNKNOWN
        };
        let size = [2000, 4000];
        let camera = ParallelCamera::for_film(&geometry, size, Vector2::new(1000.0, 3000.0))
            .expect("a camera");
        FilmLayout {
            kind,
            geometry,
            size,
            upright_size: size,
            camera,
            field: None,
        }
    }

    fn plain_vertebra() -> (VertebraParams, VertebraSize) {
        let size = VertebraSize {
            height: 20.0,
            disc_below: 5.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
            pedicle_width: 6.0,
            pedicle_height: 10.0,
        };
        let vertebra = VertebraParams {
            centre: Vector3::new(0.0, 10.0, 100.0),
            rotation: Vector3::zeros(),
            height: 20.0,
            wedge_front: 0.0,
            wedge_side: 0.0,
            top_half_width: 15.0,
            top_half_depth: 10.0,
            bottom_half_width: 15.0,
            bottom_half_depth: 10.0,
        };
        (vertebra, size)
    }

    fn xy(mark: TrueMark) -> (f64, f64) {
        (mark.at.x(), mark.at.y())
    }

    #[test]
    fn hand_worked_corners_land_where_the_conventions_put_them() {
        // Worked by hand from the conventions alone, as in the corner
        // fixtures: a vertebra centred at (0, 10, 100), 20 tall, 30 wide and
        // 20 deep, on films at 0.5 mm a pixel with the origin at
        // (1000, 3000). Front, patient left on screen right: u = 1000 +
        // y/0.5 and v = 3000 − z/0.5, so the left corners are at
        // y = 10 + 15 → u = 1050 and the upper ones at z = 110 → v = 2780.
        let (vertebra, size) = plain_vertebra();
        let front = true_marks(
            &film(FilmKind::Front, ImageRight::PatientLeft),
            &vertebra,
            Level::T8,
            &size,
        );
        let TrueMarks::Front {
            sup_l,
            sup_r,
            inf_l,
            inf_r,
            ped_l,
            ped_r,
        } = front
        else {
            panic!("front marks");
        };
        assert_eq!(xy(sup_l), (1050.0, 2780.0));
        assert_eq!(xy(sup_r), (990.0, 2780.0));
        assert_eq!(xy(inf_l), (1050.0, 2820.0));
        assert_eq!(xy(inf_r), (990.0, 2820.0));
        // Pedicles at y = 10 ± 0.78·15 and z = 100 + 0.18·20.
        assert!((ped_l.at.x() - 1043.4).abs() < 1e-9 && (ped_l.at.y() - 2792.8).abs() < 1e-9);
        assert!((ped_r.at.x() - 996.6).abs() < 1e-9);
        // Seen straight on, both rings show.
        assert!(ped_l.visible && ped_r.visible && sup_l.visible);
        // Mirrored: patient right on screen right, u = 1000 − y/0.5.
        let mirrored = true_marks(
            &film(FilmKind::Front, ImageRight::PatientRight),
            &vertebra,
            Level::T8,
            &size,
        );
        let TrueMarks::Front { sup_l, sup_r, .. } = mirrored else {
            panic!("front marks");
        };
        assert_eq!(xy(sup_l), (950.0, 2780.0));
        assert_eq!(xy(sup_r), (1010.0, 2780.0));
        // Side, anterior on screen right: u = 1000 + x/0.5, depth ±10.
        let side = true_marks(
            &film(FilmKind::Side, ImageRight::Anterior),
            &vertebra,
            Level::T8,
            &size,
        );
        let TrueMarks::Side { sup_a, sup_p, .. } = side else {
            panic!("side marks");
        };
        assert_eq!(xy(sup_a), (1020.0, 2780.0));
        assert_eq!(xy(sup_p), (980.0, 2780.0));
        // Bending: each line's ends patient left, then right.
        let bend = true_marks(
            &film(FilmKind::BendLeft, ImageRight::PatientLeft),
            &vertebra,
            Level::T8,
            &size,
        );
        let TrueMarks::Bending { sup, inf } = bend else {
            panic!("bending marks");
        };
        assert_eq!(
            (xy(sup[0]), xy(sup[1])),
            ((1050.0, 2780.0), (990.0, 2780.0))
        );
        assert_eq!(xy(inf[1]), (990.0, 2820.0));
    }

    #[test]
    fn a_pedicle_turned_past_its_body_is_hidden() {
        let (mut vertebra, size) = plain_vertebra();
        let layout = film(FilmKind::Front, ImageRight::PatientLeft);
        // Turned 50°: both pedicle axes are more than 35° off the beam.
        vertebra.rotation = Vector3::new(0.0, 0.0, 50f64.to_radians());
        let TrueMarks::Front {
            ped_l,
            ped_r,
            sup_l,
            ..
        } = true_marks(&layout, &vertebra, Level::T8, &size)
        else {
            panic!("front marks");
        };
        assert!(!ped_l.visible && !ped_r.visible && sup_l.visible);
        // Turned 25° toward the left: the left pedicle's axis is 17° off
        // the beam (it converges 8°) and stays in the body, the right one
        // is 33° off and slides past the body's right edge.
        vertebra.rotation = Vector3::new(0.0, 0.0, 25f64.to_radians());
        let TrueMarks::Front { ped_l, ped_r, .. } =
            true_marks(&layout, &vertebra, Level::T8, &size)
        else {
            panic!("front marks");
        };
        assert!(ped_l.visible);
        assert!(!ped_r.visible);
    }

    #[test]
    fn a_vertebra_seen_end_on_has_no_visible_corners() {
        let (mut vertebra, size) = plain_vertebra();
        vertebra.rotation = Vector3::new(0.0, 0.0, core::f64::consts::FRAC_PI_2);
        let marks = true_marks(
            &film(FilmKind::Front, ImageRight::PatientLeft),
            &vertebra,
            Level::T8,
            &size,
        );
        assert!(corners_of(&marks).iter().all(|c| !c.visible));
        assert_eq!(xy(corners_of(&marks)[0]), (1020.0, 2780.0));
    }

    #[test]
    fn answer_key_marks_are_exactly_the_forward_models_projection() {
        for (name, seed) in [
            (RecipeName::Random, 1),
            (RecipeName::Random, 2),
            (RecipeName::DoubleMajor55To45, 3),
            (RecipeName::UpsideDown, 4),
            (RecipeName::SideFacingLeft, 5),
        ] {
            let (patient, layouts, key) = case(name, seed, Tier::T4);
            for (answer, layout) in key.films.iter().zip(&layouts) {
                let spine = spine_for(&patient, layout.kind);
                for truth in &answer.vertebrae {
                    let v = spine.vertebra(truth.level).expect("the level");
                    let camera = &layout.camera;
                    let got = corners_of(&truth.marks).map(|c| c.at);
                    let expected = match layout.kind {
                        FilmKind::Front => {
                            let m = front_marks(v, camera).expect("corners");
                            let peds = match truth.marks {
                                TrueMarks::Front { ped_l, ped_r, .. } => [ped_l.at, ped_r.at],
                                _ => unreachable!(),
                            };
                            assert_eq!(Some(peds[0]), m.ped_l.mark().map(|p| p.at));
                            assert_eq!(Some(peds[1]), m.ped_r.mark().map(|p| p.at));
                            [m.sup_l.at, m.sup_r.at, m.inf_r.at, m.inf_l.at]
                        }
                        FilmKind::Side => {
                            let m = side_marks(v, camera).expect("corners");
                            [m.sup_a.at, m.sup_p.at, m.inf_p.at, m.inf_a.at]
                        }
                        _ => {
                            let m = endplate_lines(v, camera).expect("corners");
                            [m.sup[0].at, m.sup[1].at, m.inf[1].at, m.inf[0].at]
                        }
                    };
                    assert_eq!(
                        got, expected,
                        "{name:?} {:?} {:?}",
                        layout.kind, truth.level
                    );
                }
            }
        }
    }

    #[test]
    fn the_key_lists_the_counts_levels_on_every_film() {
        for (name, template) in [
            (RecipeName::SixLumbar, Template::ExtraLumbar),
            (RecipeName::ElevenRibs, Template::MissingThoracic),
            (RecipeName::Straight, Template::Standard),
        ] {
            let (_, _, key) = case(name, 6, Tier::T1);
            assert_eq!(key.template, template);
            let levels: Vec<Level> = key.poses.iter().map(|p| p.level).collect();
            assert_eq!(levels, template.levels());
            for film in &key.films {
                let on_film: Vec<Level> = film.vertebrae.iter().map(|v| v.level).collect();
                assert_eq!(on_film, levels);
                assert_eq!(film.bent_poses.is_some(), film.kind.is_bending());
            }
            assert_eq!(
                key.films.iter().map(|f| f.kind).collect::<Vec<_>>(),
                FilmKind::ALL
            );
        }
    }

    #[test]
    fn c7_is_the_mean_of_its_corners_and_the_landmarks_sit_in_order() {
        let (_, layouts, key) = case(RecipeName::RightThoracic40, 7, Tier::T2);
        for (film, layout) in key.films.iter().zip(&layouts) {
            let c7 = film
                .vertebrae
                .iter()
                .find(|v| v.level == Level::C7)
                .expect("C7");
            let corners = corners_of(&c7.marks);
            let x = corners.iter().map(|c| c.at.x()).sum::<f64>() / 4.0;
            assert!((film.landmarks.c7.at.x() - x).abs() < 1e-9);
            assert_eq!(film.landmarks.c7.visible, corners.iter().all(|c| c.visible));
            let [first, centre, second] = film.landmarks.sacrum_top;
            assert!((centre.at.x() - 0.5 * (first.at.x() + second.at.x())).abs() < 1e-9);
            if layout.kind != FilmKind::Side {
                // Patient left on screen right: the left corner and left
                // hip lie right of the right ones.
                assert!(first.at.x() > second.at.x());
                assert!(film.landmarks.hips[0].at.x() > film.landmarks.hips[1].at.x());
            } else {
                // Anterior on screen right.
                assert!(first.at.x() > second.at.x());
            }
            assert!(film.landmarks.hips.iter().all(|h| h.visible));
        }
    }

    /// One bump curve stacked with no wedging, twist or sagittal shape and
    /// no balancing.
    fn bare_curve(cobb_deg: f64) -> (StackedSpine, Curve) {
        let levels = painted_levels(Template::Standard);
        let sizes = base_sizes(&levels, 0.9);
        let curve = Curve {
            region: CurveRegion::MainThoracic,
            upper_end: 3.0,
            lower_end: 13.0,
            apex: 8.0,
            cobb_deg,
            convex_right: true,
            twist_per_cobb: 0.0,
        };
        let coronal = CoronalProfile::new(CurveGenerator::Bump, &[curve], 17.0).expect("solves");
        let flat = SagittalAngles {
            sacral_slope: 0.0,
            lordosis: 0.0,
            kyphosis: 0.0,
            upper_thoracic: 0.0,
            cervical_lordosis: 0.0,
        };
        let sagittal = SagittalProfile::new(&flat, 12, 5);
        let spine = stack(&StackPlan {
            levels: &levels,
            sizes: &sizes,
            coronal: &coronal,
            sagittal: &sagittal,
            share: VertebralShare {
                coronal_thoracic: 0.0,
                coronal_lumbar: 0.0,
                sagittal_thoracic: 0.0,
                sagittal_lumbar: 0.0,
            },
            trunk_lean: 0.0,
            pelvic_incidence_deg: 50.0,
            pelvic_obliquity_deg: 0.0,
            length_mm: 405.0,
            coronal_balance: None,
        })
        .expect("stacks");
        (spine, curve)
    }

    #[test]
    fn a_bare_curve_measures_exactly_its_cobb_angle() {
        // With no wedge, twist or lean every endplate tilts exactly as the
        // profile, whose end vertebrae differ by the Cobb angle.
        for cobb in [10.0, 20.0, 40.0, 60.0, 80.0] {
            let (spine, curve) = bare_curve(cobb);
            let measured = truth_angles(&spine, &[curve]).frontal_cobb[0].degrees;
            assert!((measured - cobb).abs() < 1e-9, "{cobb}: {measured}");
        }
    }

    /// The Cobb angle read off a front film's true marks: the largest angle
    /// between an upper endplate line and a lower one at or below it.
    fn film_cobb(film: &FilmAnswer, from: Level, to: Level) -> f64 {
        let lines: Vec<(f64, f64)> = film
            .vertebrae
            .iter()
            .filter(|v| v.level >= from && v.level <= to)
            .map(|v| {
                let [sl, sr, ir, il] = corners_of(&v.marks).map(|c| c.at);
                let angle = |a: Px, b: Px| libm::atan2(a.y() - b.y(), a.x() - b.x());
                (angle(sl, sr), angle(il, ir))
            })
            .collect();
        let mut most = 0.0f64;
        for (i, upper) in lines.iter().enumerate() {
            for lower in &lines[i..] {
                most = most.max((upper.0 - lower.1).abs());
            }
        }
        most.to_degrees()
    }

    #[test]
    fn named_spines_measure_the_cobb_angles_their_recipes_asked_for() {
        // The endplates of a bump spine's end vertebrae sit ν/2 either side
        // of the bump's peak, where it falls by about (ν/2s)² of its height:
        // at most (0.8/5)² ≈ 2.6% here (ν ≤ 0.8, half-spans of at least
        // 2.5), and wedging turned by twist tips an endplate sideways by
        // about ωs·sin ψ, a few tenths of a degree. So the angle in 3D stays
        // within 3% of the recipe, or 1°. On the film, a twisted vertebra
        // leaning forward also turns its endplate line by about ψ·sin φ;
        // the end vertebrae are not twisted, but their neighbours are, and
        // a severe curve's are twisted a lot: the angle read off the front
        // film's true marks stays within 5% of the recipe, or 1.5°.
        let cases = [
            (RecipeName::RightThoracic40, vec![40.0]),
            (RecipeName::LumbarLeft30, vec![30.0]),
            (RecipeName::DoubleMajor55To45, vec![55.0, 45.0]),
            (RecipeName::Severe90, vec![90.0]),
        ];
        for (name, asked) in cases {
            for seed in 0..8 {
                let (patient, _, key) = case(name, seed, Tier::T0);
                for (index, (angle, asked)) in
                    key.angles.frontal_cobb.iter().zip(&asked).enumerate()
                {
                    assert!(
                        (angle.degrees - asked).abs() < (0.03 * asked).max(1.0),
                        "{name:?} seed {seed}: {} against {asked}",
                        angle.degrees
                    );
                    let curve = patient.recipe.curves[index];
                    let level_at = |t: f64| {
                        let thoracic = Template::Standard.levels()[1..].to_vec();
                        thoracic[(libm::round(t) as usize).min(thoracic.len() - 1)]
                    };
                    let on_film = film_cobb(
                        &key.films[0],
                        level_at(curve.upper_end),
                        level_at(curve.lower_end),
                    );
                    assert!(
                        (on_film - asked).abs() < (0.05 * asked).max(1.5),
                        "{name:?} seed {seed} on film: {on_film} against {asked}"
                    );
                }
            }
        }
        let (_, _, straight) = case(RecipeName::Straight, 1, Tier::T0);
        assert!(straight.angles.frontal_cobb.is_empty());
    }

    #[test]
    fn a_right_thoracic_curve_bulges_right_and_turns_its_front_right() {
        let (patient, layouts, key) = case(RecipeName::RightThoracic40, 2, Tier::T0);
        let curve = patient.recipe.curves[0];
        let spine = &patient.standing;
        let levels = Template::Standard.levels();
        let level = |t: f64| levels[1 + libm::round(t) as usize];
        let at = |t: f64| spine.vertebra(level(t)).expect("level").centre;
        let (upper, apex, lower) = (at(curve.upper_end), at(curve.apex), at(curve.lower_end));
        let along = (apex.z - lower.z) / (upper.z - lower.z);
        let chord_y = lower.y + along * (upper.y - lower.y);
        // Patient right is −y.
        assert!(apex.y < chord_y - 5.0);
        // On the front film (patient left on screen right) the apex lies
        // left of the chord.
        let front = &layouts[0];
        let u = |p: &Vector3<f64>| front.project(p).x();
        assert!(u(&apex) < u(&lower) + along * (u(&upper) - u(&lower)));
        assert!(key.angles.apical_twist[0].degrees < -3.0);
        // The kyphosis and lordosis measured are near what the recipe
        // asked and the balance solved.
        assert!((key.angles.kyphosis_degrees - patient.recipe.sagittal.kyphosis).abs() < 3.0);
        assert!((key.angles.lordosis_degrees - patient.recipe.sagittal.lordosis).abs() < 3.0);
    }

    #[test]
    fn spline_and_bump_spines_measure_alike() {
        // Random spines reach 110° with vertebral shares up to 0.8 and
        // half-spans down to 1.5, where the bump falls by up to
        // (0.8 / 3)² ≈ 7% of its height at the end vertebrae's endplates
        // (the spline holds its end vertebrae flat, so it does not): within
        // 7% of the recipe, plus half a degree for the twist.
        for seed in 0..40 {
            let (patient, _, key) = case(RecipeName::Random, seed, Tier::T0);
            for (curve, angle) in patient.recipe.curves.iter().zip(&key.angles.frontal_cobb) {
                assert!(
                    (angle.degrees - curve.cobb_deg).abs() < 0.5 + 0.07 * curve.cobb_deg,
                    "seed {seed} {:?} {:?}: {} against {}",
                    patient.recipe.generator,
                    curve.region,
                    angle.degrees,
                    curve.cobb_deg
                );
            }
        }
    }

    #[test]
    fn turned_films_keep_every_mark_on_the_same_bone() {
        let (_, layouts, key) = case(RecipeName::UpsideDown, 3, Tier::T0);
        let front = &key.films[0];
        assert_eq!(
            layouts[0].geometry.quarter_turns,
            QuarterTurns::new(2).expect("two")
        );
        // Upside down: the head is at the bottom of the stored film.
        let c7 = front.landmarks.c7.at.y();
        assert!(c7 > front.landmarks.hips[0].at.y());
    }
}
