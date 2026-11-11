//! Ribs, shoulders, the skull edge, the pelvis and the femoral heads.
//!
//! Everything a full-length film shows besides the spine, made up in the
//! usual adult proportions and scaled with the spine (`k`). These are what
//! make a film hard to read: ribs cross the thoracic spine on the side film
//! and run out from it on the front film, the shoulders and humeral heads
//! cover the cervicothoracic junction (more so with heavy shoulders), the
//! skull's edge and the jaw sit over the top of the neck, the iliac wings
//! cover L4 to S1 and the femoral heads lie below.
//!
//! - **Soft tissue:** the body is a stack of elliptical cross-sections,
//!   from the thighs through the hips, waist, chest, shoulders and neck to
//!   the head, each centred in front of the spine at its height. Its
//!   length along any horizontal beam is exact.
//! - **Lungs and bowel gas:** ellipsoids of air (less than soft tissue) in
//!   the chest, either side of the mediastinum, and in the abdomen.
//! - **Ribs:** each rib leaves its vertebra just behind the body, runs back
//!   and out, round the side and forward, along a quarter-ellipse sized to
//!   the chest at its level and dropping about 30° to the front; a hollow
//!   tube in mitred pieces, carried by its vertebra's lean and twist (so
//!   twisted vertebrae lift a rib hump) and part of its tilt.
//! - **Shoulders:** clavicles, scapulae, humeral heads and shafts; the
//!   arms hang at the sides on the front and bending films and reach
//!   forward on the side film.
//! - **Skull and jaw, pelvis:** a skull shell above C2 and a jaw in front
//!   of it; the sacrum, iliac wings, hollow femoral heads with their necks
//!   and shafts, and the pubic and ischial rami.

use super::solids::{Capsule, Ellipsoid, Frustum, HalfSpace, Ray, Solid};
use super::{CORTEX, Canvas, SOFT_TISSUE, TRABECULAR};
use crate::recipes::SpineRecipe;
use crate::seeded::Stream;
use crate::stack_from_l5::StackedSpine;
use forward_model::rotation::{rot_x, rot_y, rot_z};
use nalgebra::{Matrix3, Vector3};
use shapes::levels::{Level, Region};

/// Air in the lungs and bowel: less than soft tissue.
const AIR: f64 = -SOFT_TISSUE;
/// Lung is a quarter tissue, three quarters air.
const LUNG: f64 = -0.74 * SOFT_TISSUE;

/// One height of the body's soft-tissue outline.
#[derive(Clone, Copy, Debug)]
struct Section {
    z: f64,
    /// Half-width (patient left to right) and half-depth, mm.
    half_width: f64,
    half_depth: f64,
    /// How far in front of the spine its centre is, as a share of its
    /// half-depth.
    forward: f64,
}

/// The body's outline, bottom to top, and the spine's centre line it
/// follows.
pub struct Body {
    sections: Vec<Section>,
    spine: Vec<Vector3<f64>>,
}

impl Body {
    /// The outline for `spine`, with heavier shoulders when the recipe
    /// says so.
    pub fn new(spine: &StackedSpine, recipe: &SpineRecipe) -> Self {
        let k = spine.scale;
        let z_of = |level: Level| spine.vertebra(level).map(|v| v.centre.z);
        let top = spine.vertebrae.first().map_or(500.0, |v| v.centre.z);
        let hips = spine.pelvis.hip_axis_centre.z;
        let shoulders = if recipe.heavy_shoulders { 1.3 } else { 1.0 };
        let last_thoracic = spine
            .levels
            .iter()
            .rev()
            .find(|l| l.region() == Region::Thoracic)
            .copied()
            .unwrap_or(Level::T12);
        let at = |z: Option<f64>, w: f64, d: f64, forward: f64| {
            z.map(|z| Section {
                z,
                half_width: w * k,
                half_depth: d * k,
                forward,
            })
        };
        let mut sections: Vec<Section> = [
            at(Some(hips - 300.0 * k), 150.0, 100.0, 0.25),
            at(Some(hips - 40.0 * k), 165.0, 110.0, 0.3),
            at(z_of(Level::L4), 150.0, 105.0, 0.45),
            at(z_of(Level::L2), 128.0, 95.0, 0.45),
            at(z_of(last_thoracic), 140.0, 108.0, 0.5),
            at(z_of(Level::T6), 150.0, 115.0, 0.5),
            at(z_of(Level::T2), 165.0 * shoulders, 105.0 * shoulders, 0.45),
            at(
                z_of(Level::T1).map(|z| z + 15.0 * k),
                185.0 * shoulders,
                90.0 * shoulders,
                0.4,
            ),
            at(
                z_of(Level::C7).map(|z| z + 20.0 * k),
                110.0 * shoulders,
                72.0,
                0.25,
            ),
            at(z_of(Level::C5), 55.0, 58.0, 0.15),
            at(Some(top + 30.0 * k), 70.0, 90.0, 0.1),
            at(Some(top + 300.0 * k), 70.0, 90.0, 0.1),
        ]
        .into_iter()
        .flatten()
        .collect();
        sections.sort_by(|a, b| a.z.total_cmp(&b.z));
        let mut line: Vec<Vector3<f64>> = spine.vertebrae.iter().map(|v| v.centre).collect();
        line.sort_by(|a, b| a.z.total_cmp(&b.z));
        Body {
            sections,
            spine: line,
        }
    }

    /// The spine's centre line at height `z`, held at its ends.
    fn spine_at(&self, z: f64) -> Vector3<f64> {
        interpolate(&self.spine, z, |p| p.z, |a, b, s| a + (b - a) * s)
            .unwrap_or_else(Vector3::zeros)
    }

    fn section_at(&self, z: f64) -> Option<Section> {
        interpolate(
            &self.sections,
            z,
            |s| s.z,
            |a, b, s| Section {
                z,
                half_width: a.half_width + (b.half_width - a.half_width) * s,
                half_depth: a.half_depth + (b.half_depth - a.half_depth) * s,
                forward: a.forward + (b.forward - a.forward) * s,
            },
        )
    }

    /// The body's cross-section at height `z`.
    pub fn row(&self, z: f64) -> Option<Row> {
        let section = self.section_at(z)?;
        let spine = self.spine_at(z);
        Some(Row {
            centre: [spine.x + section.forward * section.half_depth, spine.y],
            inverse_squares: [
                1.0 / (section.half_depth * section.half_depth),
                1.0 / (section.half_width * section.half_width),
            ],
        })
    }

    /// How far a horizontal ray travels through the body.
    pub fn chord(&self, ray: &Ray) -> f64 {
        self.row(ray.origin.z).map_or(0.0, |row| {
            row.chord(
                ray.origin.x,
                ray.origin.y,
                [ray.direction.x, ray.direction.y],
            )
        })
    }

    /// The half-width and half-depth of the body at a level, and its
    /// centre.
    fn at_level(&self, spine: &StackedSpine, level: Level) -> Option<(f64, f64, Vector3<f64>)> {
        let v = spine.vertebra(level)?;
        let section = self.section_at(v.centre.z)?;
        let centre = v.centre + Vector3::x() * (section.forward * section.half_depth);
        Some((section.half_width, section.half_depth, centre))
    }
}

/// The body's elliptical cross-section at one height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    centre: [f64; 2],
    /// One over the squared half-depth and half-width.
    inverse_squares: [f64; 2],
}

impl Row {
    /// How far a horizontal ray from `(x, y)` along `direction` travels
    /// through the section: exact for an ellipse.
    pub fn chord(&self, x: f64, y: f64, direction: [f64; 2]) -> f64 {
        let (ox, oy) = (x - self.centre[0], y - self.centre[1]);
        let [ia, ib] = self.inverse_squares;
        let [dx, dy] = direction;
        let a = dx * dx * ia + dy * dy * ib;
        let b = 2.0 * (ox * dx * ia + oy * dy * ib);
        let c = ox * ox * ia + oy * oy * ib - 1.0;
        let discriminant = b * b - 4.0 * a * c;
        if a <= 0.0 || discriminant <= 0.0 {
            0.0
        } else {
            discriminant.sqrt() / a
        }
    }
}

/// Linear interpolation through points sorted by `key`, held flat beyond
/// the ends. `None` when there are no points.
fn interpolate<T: Copy>(
    points: &[T],
    at: f64,
    key: impl Fn(&T) -> f64,
    blend: impl Fn(T, T, f64) -> T,
) -> Option<T> {
    let first = *points.first()?;
    let last = *points.last()?;
    if at <= key(&first) {
        return Some(blend(first, first, 0.0));
    }
    if at >= key(&last) {
        return Some(blend(last, last, 0.0));
    }
    let i = points.windows(2).position(|pair| at < key(&pair[1]))?;
    let (a, b) = (points[i], points[i + 1]);
    let span = key(&b) - key(&a);
    Some(blend(
        a,
        b,
        if span > 0.0 {
            (at - key(&a)) / span
        } else {
            0.0
        },
    ))
}

/// Paints the soft tissue, lungs and bowel gas.
pub fn paint_soft_tissue(
    canvas: &mut Canvas,
    body: &Body,
    spine: &StackedSpine,
    stream: &mut Stream,
) {
    let beam = canvas.layout.beam();
    let direction = [beam.x, beam.y];
    canvas.add_by_row(
        |z| body.row(z),
        |row, x, y| row.map_or(0.0, |row| SOFT_TISSUE * row.chord(x, y, direction)),
    );
    let k = spine.scale;
    let chest = body.at_level(spine, Level::T6);
    let top = body.at_level(spine, Level::T1);
    let base = body.at_level(spine, Level::T10);
    if let (Some((w, d, centre)), Some((_, _, t1)), Some((_, _, t10))) = (chest, top, base) {
        let half_height = 0.55 * (t1.z - t10.z);
        // Each lung stops short of the middle, leaving room for the
        // mediastinum over the spine.
        for side in [1.0, -1.0] {
            canvas.add(
                &Solid::Ellipsoid(Ellipsoid {
                    centre: Vector3::new(
                        centre.x - 0.05 * d,
                        centre.y + side * 0.52 * w,
                        0.5 * (t1.z + t10.z),
                    ),
                    axes: Matrix3::identity(),
                    radii: Vector3::new(0.65 * d, 0.3 * w, half_height),
                }),
                LUNG,
            );
        }
    }
    let (Some(l1), Some(sacrum)) = (
        spine.vertebra(Level::L1),
        Some(spine.pelvis.sacrum_top.centre),
    ) else {
        return;
    };
    let pockets = stream.whole(4, 12);
    for _ in 0..pockets {
        let z = stream.between(sacrum.z, l1.centre.z);
        let across = stream.between(-110.0, 110.0) * k;
        let forward = stream.between(10.0, 90.0) * k;
        let size = stream.between(5.0, 25.0) * k;
        let squash = stream.between(0.5, 1.0);
        canvas.add(
            &Solid::Ellipsoid(Ellipsoid {
                centre: Vector3::new(l1.centre.x + forward, l1.centre.y + across, z),
                axes: Matrix3::identity(),
                radii: Vector3::new(size, size * squash, size * stream.between(0.6, 1.2)),
            }),
            AIR,
        );
    }
}

/// Chest half-width a rib reaches, as a share of `0.3·L`, for ribs 1 to 12.
const RIB_REACH: [f64; 12] = [
    0.45, 0.6, 0.72, 0.82, 0.9, 0.96, 1.0, 1.0, 0.97, 0.9, 0.7, 0.45,
];

/// The ribs: one pair per thoracic vertebra.
pub fn rib_solids(spine: &StackedSpine, recipe: &SpineRecipe) -> Vec<(Solid, f64)> {
    let k = spine.scale;
    let length = recipe.length_mm;
    let radius = 0.011 * length;
    let wall = 0.9 * k;
    let mut solids = Vec::new();
    let thoracic: Vec<usize> = spine
        .levels
        .iter()
        .enumerate()
        .filter(|(_, l)| l.region() == Region::Thoracic)
        .map(|(i, _)| i)
        .collect();
    let count = thoracic.len();
    for (rib, &index) in thoracic.iter().enumerate() {
        let mut reach = RIB_REACH[rib.min(11)];
        if rib >= 12 {
            reach *= 0.6;
        }
        let last_two = rib + 2 >= count;
        if recipe.short_twelfth_rib && rib + 1 == count {
            reach *= 0.4;
        }
        let vertebra = &spine.vertebrae[index];
        let rotation = rib_rotation(&vertebra.rotation_matrix());
        let depth = 0.5 * (vertebra.top_half_depth + vertebra.bottom_half_depth);
        let width = 0.5 * (vertebra.top_half_width + vertebra.bottom_half_width);
        let chest = 0.3 * length * reach;
        let lateral = ((chest - 0.8 * width) / 1.479).max(5.0 * k);
        let front_back = if last_two { 0.6 } else { 0.75 } * lateral;
        let joint = Vector3::new(-0.9 * depth, 0.8 * width, 0.3 * vertebra.height);
        let lean = libm::atan2(rotation[(0, 2)], rotation[(2, 2)]);
        let drop = (30f64.to_radians() - lean).clamp(5f64.to_radians(), 50f64.to_radians());
        let tip = rot_y(drop);
        let (start, end) = (
            core::f64::consts::PI + 0.5,
            if last_two { 1.6 } else { 0.6 },
        );
        for side in [1.0, -1.0] {
            let points: Vec<Vector3<f64>> = (0..=10)
                .map(|i| {
                    let a = start + (end - start) * f64::from(i) / 10.0;
                    let local = Vector3::new(
                        joint.x + front_back * (0.8776 + libm::cos(a)),
                        joint.y + lateral * (0.4794 + libm::sin(a)),
                        joint.z,
                    );
                    let dropped = joint + tip * (local - joint);
                    let mirrored = Vector3::new(dropped.x, side * dropped.y, dropped.z);
                    vertebra.centre + rotation * mirrored
                })
                .collect();
            solids.extend(tube(&points, radius, wall, 0.85));
        }
    }
    solids
}

/// How the ribs of a vertebra turned by `rotation` are carried: with its
/// lean and twist, but only 60% of its sideways tilt, since the chest wall
/// holds the ribs back from tipping with every vertebra. The rotation is
/// read as `rotX(tilt) · rotY(lean) · rotZ(twist)`.
fn rib_rotation(rotation: &Matrix3<f64>) -> Matrix3<f64> {
    let tilt = libm::atan2(-rotation[(1, 2)], rotation[(2, 2)]);
    let lean = libm::asin(rotation[(0, 2)].clamp(-1.0, 1.0));
    let twist = libm::atan2(-rotation[(0, 1)], rotation[(0, 0)]);
    rot_x(0.6 * tilt) * rot_y(lean) * rot_z(twist)
}

/// A hollow tube along a polyline, in mitred pieces: each piece is cut by
/// the planes bisecting its joints, so neighbours meet exactly. `density`
/// scales the bone.
fn tube(points: &[Vector3<f64>], radius: f64, wall: f64, density: f64) -> Vec<(Solid, f64)> {
    let directions: Vec<Vector3<f64>> = points
        .windows(2)
        .map(|p| (p[1] - p[0]).normalize())
        .collect();
    let mut solids = Vec::new();
    for (i, pair) in points.windows(2).enumerate() {
        let dir = directions[i];
        let before = if i == 0 {
            dir
        } else {
            (directions[i - 1] + dir).normalize()
        };
        let after = directions
            .get(i + 1)
            .map_or(dir, |next| (dir + next).normalize());
        let faces = vec![
            HalfSpace::behind(pair[0], -before),
            HalfSpace::behind(pair[1], after),
        ];
        let axes = frame_along(&dir);
        let outer = Frustum {
            centre: 0.5 * (pair[0] + pair[1]),
            axes,
            half: [radius, radius],
            taper: 0.0,
            faces,
            trim: None,
            half_length: 0.5 * (pair[1] - pair[0]).norm() + 2.0 * radius,
        };
        let inner = Frustum {
            half: [radius - wall, radius - wall],
            ..outer.clone()
        };
        solids.push((Solid::Frustum(outer), density * CORTEX));
        solids.push((Solid::Frustum(inner), density * (TRABECULAR - CORTEX)));
    }
    solids
}

/// Unit axes whose third is `along`.
fn frame_along(along: &Vector3<f64>) -> Matrix3<f64> {
    let helper = if along.z.abs() < 0.9 {
        Vector3::z()
    } else {
        Vector3::x()
    };
    let x = helper.cross(along).normalize();
    let y = along.cross(&x);
    Matrix3::from_columns(&[x, y, *along])
}

/// The clavicles, scapulae, humeri and the soft tissue over the shoulders.
/// `arms_forward` on side films.
pub fn shoulder_solids(
    spine: &StackedSpine,
    body: &Body,
    recipe: &SpineRecipe,
    arms_forward: bool,
) -> Vec<(Solid, f64)> {
    let k = spine.scale;
    let heavy = if recipe.heavy_shoulders { 1.15 } else { 1.0 };
    let (Some((w1, _, c1)), Some((_, d2, c2)), Some((w4, d4, c4))) = (
        body.at_level(spine, Level::T1),
        body.at_level(spine, Level::T2),
        body.at_level(spine, Level::T4),
    ) else {
        return Vec::new();
    };
    let spine_t1 = spine.vertebra(Level::T1).map_or(c1, |v| v.centre);
    let mut solids = Vec::new();
    for side in [1.0, -1.0] {
        let sternal = Vector3::new(c2.x + 0.85 * d2, c2.y + side * 18.0 * k, c2.z - 10.0 * k);
        let acromion = Vector3::new(
            spine_t1.x - 5.0 * k,
            c1.y + side * 0.88 * w1,
            spine_t1.z + 18.0 * k,
        );
        let bend = |share: f64, forward: f64| {
            let p = sternal + (acromion - sternal) * share;
            p + Vector3::new(forward * k, 0.0, 6.0 * k * share)
        };
        let clavicle = [sternal, bend(0.35, -8.0), bend(0.7, 6.0), acromion];
        for pair in clavicle.windows(2) {
            solids.push((
                Solid::Capsule(Capsule {
                    start: pair[0],
                    end: pair[1],
                    radius: 6.0 * k * heavy,
                }),
                0.8 * CORTEX,
            ));
        }
        solids.push((
            Solid::Ellipsoid(Ellipsoid {
                centre: Vector3::new(c4.x - 0.8 * d4, c4.y + side * 0.5 * w4, c4.z),
                axes: rot_z(side * 25f64.to_radians()),
                radii: Vector3::new(4.0 * k, 50.0 * k * heavy, 70.0 * k * heavy),
            }),
            0.7 * CORTEX,
        ));
        let head_centre = acromion + Vector3::new(-5.0 * k, side * 15.0 * k, -25.0 * k);
        let head = Ellipsoid::sphere(head_centre, 23.0 * k * heavy);
        solids.push((Solid::Ellipsoid(head), CORTEX));
        solids.push((Solid::Ellipsoid(head.shrunk(2.0 * k)), TRABECULAR - CORTEX));
        let toward = if arms_forward {
            Vector3::new(0.94, 0.0, -0.34)
        } else {
            Vector3::new(0.0, side * 0.17, -0.985)
        };
        let shaft_end = head_centre + toward * (290.0 * k);
        solids.push((
            Solid::Capsule(Capsule {
                start: head_centre,
                end: shaft_end,
                radius: 11.0 * k * heavy,
            }),
            0.8 * CORTEX,
        ));
        solids.push((
            Solid::Ellipsoid(Ellipsoid::sphere(
                head_centre + Vector3::new(0.0, side * 8.0 * k, 10.0 * k),
                50.0 * k * heavy,
            )),
            SOFT_TISSUE * (heavy - 0.7),
        ));
    }
    solids
}

/// The skull's shell above C2 and the jaw in front of the neck.
pub fn skull_solids(spine: &StackedSpine) -> Vec<(Solid, f64)> {
    let k = spine.scale;
    let Some(top) = spine.vertebrae.first() else {
        return Vec::new();
    };
    let c = top.centre;
    let skull = Ellipsoid {
        centre: c + Vector3::new(15.0 * k, 0.0, 85.0 * k),
        axes: Matrix3::identity(),
        radii: Vector3::new(95.0 * k, 75.0 * k, 88.0 * k),
    };
    let mut solids = vec![
        (Solid::Ellipsoid(skull), CORTEX),
        (Solid::Ellipsoid(skull.shrunk(6.0 * k)), -CORTEX),
    ];
    let chin = c + Vector3::new(85.0 * k, 0.0, -25.0 * k);
    for side in [1.0, -1.0] {
        let angle = c + Vector3::new(25.0 * k, side * 45.0 * k, 10.0 * k);
        let middle = c + Vector3::new(60.0 * k, side * 35.0 * k, -20.0 * k);
        for (start, end) in [(angle, middle), (middle, chin)] {
            solids.push((
                Solid::Capsule(Capsule {
                    start,
                    end,
                    radius: 7.0 * k,
                }),
                0.8 * CORTEX,
            ));
        }
    }
    solids
}

/// The sacrum, iliac wings, femoral heads, necks and shafts, and the rami.
pub fn pelvis_solids(spine: &StackedSpine) -> Vec<(Solid, f64)> {
    let k = spine.scale;
    let pelvis = &spine.pelvis;
    let frame = pelvis.frame;
    let upright = pelvis.upright;
    let in_pelvis = |x: f64, y: f64, z: f64| upright * Vector3::new(x * k, y * k, z * k);
    let mut solids = Vec::new();

    // The sacrum: a wedge down and back from the top of S1.
    let top = &pelvis.sacrum_top;
    let length = 100.0 * k;
    let axis = frame * (rot_y(-20f64.to_radians()) * Vector3::z());
    let narrowing = (1.0 - 0.35) / (1.0 + 0.35);
    let centre = top.centre - axis * (0.5 * length);
    let sacrum = Frustum {
        centre,
        axes: {
            let x = (top.depth_axis * -1.0 - axis * axis.dot(&-top.depth_axis)).normalize();
            Matrix3::from_columns(&[x, axis.cross(&x), axis])
        },
        half: [
            top.half_depth / (1.0 + narrowing),
            top.half_width / (1.0 + narrowing),
        ],
        taper: 2.0 * narrowing / length,
        faces: vec![
            HalfSpace::behind(top.centre, top.normal),
            HalfSpace::behind(centre - axis * (0.5 * length), -axis),
        ],
        trim: None,
        half_length: 0.5 * length + top.half_depth,
    };
    let core = Frustum {
        half: [sacrum.half[0] - 1.5 * k, sacrum.half[1] - 1.5 * k],
        ..sacrum.clone()
    };
    solids.push((Solid::Frustum(sacrum), CORTEX));
    solids.push((Solid::Frustum(core), TRABECULAR - CORTEX));

    let hips = pelvis.hip_axis_centre;
    for (index, side) in [1.0, -1.0].into_iter().enumerate() {
        solids.push((
            Solid::Ellipsoid(Ellipsoid {
                centre: hips + in_pelvis(-25.0, side * 60.0, 55.0),
                axes: upright * rot_z(side * 30f64.to_radians()),
                radii: Vector3::new(5.0 * k, 55.0 * k, 60.0 * k),
            }),
            0.8 * CORTEX,
        ));
        let head_centre = pelvis.hips[index];
        let head = Ellipsoid::sphere(head_centre, pelvis.hip_radius);
        solids.push((Solid::Ellipsoid(head), CORTEX));
        solids.push((Solid::Ellipsoid(head.shrunk(2.0 * k)), TRABECULAR - CORTEX));
        let trochanter = head_centre + in_pelvis(0.0, side * 45.0, -35.0);
        solids.push((
            Solid::Capsule(Capsule {
                start: head_centre,
                end: trochanter,
                radius: 14.0 * k,
            }),
            0.8 * CORTEX,
        ));
        let knee = trochanter + in_pelvis(5.0, -side * 25.0, -330.0);
        solids.extend(tube(&[trochanter, knee], 13.0 * k, 4.0 * k, 1.0));
        let symphysis = hips + in_pelvis(55.0, 0.0, -45.0);
        solids.push((
            Solid::Capsule(Capsule {
                start: head_centre + in_pelvis(20.0, -side * 20.0, -30.0),
                end: symphysis,
                radius: 7.0 * k,
            }),
            0.8 * CORTEX,
        ));
        solids.push((
            Solid::Capsule(Capsule {
                start: head_centre + in_pelvis(-10.0, -side * 10.0, -30.0),
                end: head_centre + in_pelvis(-5.0, -side * 35.0, -75.0),
                radius: 9.0 * k,
            }),
            0.8 * CORTEX,
        ));
    }
    solids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patient::make_up_patient;
    use shapes::made_up::RecipeName;

    #[test]
    fn the_body_outline_is_widest_at_the_shoulders_and_hips() {
        let patient = make_up_patient(RecipeName::Straight, 1).expect("stacks");
        let body = Body::new(&patient.standing, &patient.recipe);
        let spine = &patient.standing;
        let across = |level: Level| {
            let z = spine.vertebra(level).expect("level").centre.z;
            body.chord(&Ray {
                origin: Vector3::new(0.0, -500.0, z),
                direction: Vector3::y(),
            })
        };
        assert!(across(Level::T1) > across(Level::L2));
        assert!(across(Level::L2) < across(Level::L5));
        assert!(across(Level::C7) > across(Level::C2) * 0.5);
        // Through the middle of the chest, front to back: twice the
        // half-depth.
        let z = spine.vertebra(Level::T6).expect("T6").centre.z;
        let deep = body.chord(&Ray {
            origin: Vector3::new(-500.0, spine.vertebra(Level::T6).expect("T6").centre.y, z),
            direction: Vector3::x(),
        });
        assert!((deep - 2.0 * 115.0 * spine.scale).abs() < 1e-6, "{deep}");
    }

    #[test]
    fn heavy_shoulders_are_heavier() {
        let patient = make_up_patient(RecipeName::HeavyShoulders, 1).expect("stacks");
        let mut light = patient.recipe.clone();
        light.heavy_shoulders = false;
        let heavy_body = Body::new(&patient.standing, &patient.recipe);
        let light_body = Body::new(&patient.standing, &light);
        let z = patient.standing.vertebra(Level::T2).expect("T2").centre.z;
        let ray = Ray {
            origin: Vector3::new(-500.0, 0.0, z),
            direction: Vector3::x(),
        };
        assert!(heavy_body.chord(&ray) > 1.2 * light_body.chord(&ray));
        let bones = |recipe: &SpineRecipe, body: &Body| {
            shoulder_solids(&patient.standing, body, recipe, false)
                .iter()
                .map(|(s, mu)| mu * s.chord(&ray))
                .sum::<f64>()
        };
        assert!(bones(&patient.recipe, &heavy_body) >= bones(&light, &light_body));
    }

    #[test]
    fn every_thoracic_vertebra_has_a_pair_of_ribs_and_eleven_ribs_has_eleven() {
        for (name, pairs) in [(RecipeName::Straight, 12), (RecipeName::ElevenRibs, 11)] {
            let patient = make_up_patient(name, 2).expect("stacks");
            let ribs = rib_solids(&patient.standing, &patient.recipe);
            // Ten pieces a rib, outer and inner each.
            assert_eq!(ribs.len(), pairs * 2 * 10 * 2);
        }
    }

    #[test]
    fn ribs_start_behind_their_body_and_reach_round_to_the_front() {
        let patient = make_up_patient(RecipeName::Straight, 3).expect("stacks");
        let spine = &patient.standing;
        let ribs = rib_solids(spine, &patient.recipe);
        let t7 = spine.vertebra(Level::T7).expect("T7").centre;
        // A side beam through the chest well in front of the spine crosses
        // ribs; one through the space above the head does not.
        let crossed = |origin: Vector3<f64>| {
            let ray = Ray {
                origin,
                direction: Vector3::y(),
            };
            ribs.iter()
                .filter(|(s, mu)| *mu > 0.0 && s.chord(&ray) > 0.0)
                .count()
        };
        let in_front = (0..40)
            .map(|i| {
                crossed(Vector3::new(
                    t7.x + 60.0,
                    -400.0,
                    t7.z - 80.0 + 4.0 * f64::from(i),
                ))
            })
            .max()
            .unwrap_or(0);
        assert!(in_front >= 2, "{in_front}");
        assert_eq!(crossed(Vector3::new(t7.x, -400.0, t7.z + 1000.0)), 0);
    }
}
