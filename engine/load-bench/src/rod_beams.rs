//! Linear circular Timoshenko rods, condensed to their heads.
//!
//! Coordinates are patient millimetres: x anterior, y patient-left, z up;
//! x cross y = z. Nodal freedoms are `[tx,ty,tz,rx,ry,rz]`, translations
//! in mm and active right-hand rotations in radians. Their conjugate loads
//! are forces in N and moments in N·mm, applied **to the rod**. The rod's
//! force on a vertebra is the negative of the corresponding head load.
//! Positive x/y/z moments have exactly the signs in `forward-model`: x
//! tips an up axis right, y tips it forward, z turns anterior toward left.
//!
//! Each contour vertex and head is a node, with two straight elements
//! between successive such stations. Internal translations and rotations
//! are eliminated by static condensation. The six-freedom head stiffness
//! supports locked screws in subsequent load cases. Releasing every head
//! rotation gives the three-translation stiffness used during seating.
//! No connector rotations are tied during seating. A hook keeps that
//! release in subsequent load cases too.
//!
//! The compliance fixes a minimum Euclidean-displacement gauge for the
//! rod's rigid motions; it applies to balanced head forces. A gauge has no
//! effect on strain energy. Explicit rigid-motion projection removes only
//! round-off residue, rather than adding springs to ground.

use crate::chain_solver::BorderedSystem;
use nalgebra::{DMatrix, Matrix3, Matrix6, SMatrix, Vector3};

/// This project's choice when only E is supplied, not a material claim.
pub const ROD_POISSON_RATIO: f64 = 0.3;
/// This project's shear-coefficient choice, not a measured material value.
pub const ROD_SHEAR_COEFFICIENT: f64 = 0.9;
/// This project's mesh choice: two exact shear-flexible elements per interval.
pub const ELEMENTS_PER_INTERVAL: usize = 2;
/// This project's relative tolerance for identifying a rigid-motion basis.
const BASIS_TOLERANCE: f64 = 1e-12;

/// Plain section properties; MPa = N/mm². E and G may be independent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamProperties {
    pub young_modulus_mpa: f64,
    pub shear_modulus_mpa: f64,
    pub diameter_mm: f64,
    pub shear_coefficient: f64,
}

impl BeamProperties {
    /// Use this project's stated Poisson and shear-coefficient choices.
    /// Published E is supplied by the caller through the evidence reader.
    pub fn from_young_modulus(young_modulus_mpa: f64, diameter_mm: f64) -> Self {
        Self {
            young_modulus_mpa,
            shear_modulus_mpa: young_modulus_mpa / (2.0 * (1.0 + ROD_POISSON_RATIO)),
            diameter_mm,
            shear_coefficient: ROD_SHEAR_COEFFICIENT,
        }
    }

    pub fn area_mm2(self) -> f64 {
        core::f64::consts::PI * self.diameter_mm * self.diameter_mm / 4.0
    }

    pub fn second_moment_mm4(self) -> f64 {
        let d2 = self.diameter_mm * self.diameter_mm;
        core::f64::consts::PI * d2 * d2 / 64.0
    }

    fn usable(self) -> bool {
        [
            self.young_modulus_mpa,
            self.shear_modulus_mpa,
            self.diameter_mm,
            self.shear_coefficient,
        ]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    }
}

/// A named refusal, without numbers from a singular rod system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RodError {
    #[error("rod-invalid-properties")]
    InvalidProperties,
    #[error("rod-invalid-polyline")]
    InvalidPolyline,
    #[error("rod-invalid-head-stations")]
    InvalidHeadStations,
    #[error("rod-singular-stiffness")]
    SingularStiffness,
}

/// Head matrices in the supplied station order. Stiffness maps head
/// displacement to the external load holding the rod there.
#[derive(Clone, Debug)]
pub struct CondensedRod {
    pub head_points_mm: Vec<Vector3<f64>>,
    /// All six freedoms at each head; forces N and moments N·mm.
    pub stiffness: DMatrix<f64>,
    /// Three translations per head after every rotation is released, N/mm.
    pub translations: DMatrix<f64>,
    /// Translation compliance, mm/N, on balanced loads, with rigid gauge.
    pub compliance: DMatrix<f64>,
    /// Orthonormal columns spanning the head translations of rigid motions.
    /// A straight set of heads has five columns, a non-collinear set six.
    pub rigid_modes: DMatrix<f64>,
}

impl CondensedRod {
    /// Condense a stress-free piecewise-straight contour to sorted,
    /// strictly increasing head arclengths in mm, including free overhangs.
    /// At least two heads are needed. No source table is read here.
    pub fn new(
        polyline: &[Vector3<f64>],
        head_stations_mm: &[f64],
        properties: BeamProperties,
    ) -> Result<Self, RodError> {
        if !properties.usable() {
            return Err(RodError::InvalidProperties);
        }
        let vertices = polyline_stations(polyline)?;
        let length = *vertices.last().ok_or(RodError::InvalidPolyline)?;
        if head_stations_mm.len() < 2
            || head_stations_mm
                .iter()
                .any(|s| !s.is_finite() || *s < 0.0 || *s > length)
            || head_stations_mm.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(RodError::InvalidHeadStations);
        }
        let mut breaks = vertices;
        breaks.extend_from_slice(head_stations_mm);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup_by(|a, b| *a == *b);
        let mut stations = Vec::new();
        for pair in breaks.windows(2) {
            for i in 0..ELEMENTS_PER_INTERVAL {
                stations
                    .push(pair[0] + (pair[1] - pair[0]) * i as f64 / ELEMENTS_PER_INTERVAL as f64);
            }
        }
        stations.push(length);
        let nodes: Vec<_> = stations
            .iter()
            .map(|s| polyline_at(polyline, *s).map(|(point, _)| point))
            .collect::<Result<_, _>>()?;
        let heads: Vec<_> = head_stations_mm
            .iter()
            .map(|s| stations.iter().position(|candidate| candidate == s))
            .collect::<Option<_>>()
            .ok_or(RodError::InvalidHeadStations)?;
        let stiffness = condense_nodes(&nodes, &heads, properties)?;
        let head_points_mm: Vec<_> = heads.iter().map(|index| nodes[*index]).collect();
        let (translations, compliance, rigid_modes) =
            release_rotations(&stiffness, &head_points_mm)?;
        Ok(Self {
            head_points_mm,
            stiffness,
            translations,
            compliance,
            rigid_modes,
        })
    }
}

/// Arclength of every contour vertex, starting at zero. Consecutive
/// vertices must differ; all coordinates must be finite.
pub fn polyline_stations(polyline: &[Vector3<f64>]) -> Result<Vec<f64>, RodError> {
    if polyline.len() < 2 || polyline.iter().any(|p| p.iter().any(|x| !x.is_finite())) {
        return Err(RodError::InvalidPolyline);
    }
    let mut stations = vec![0.0];
    for pair in polyline.windows(2) {
        let length = (pair[1] - pair[0]).norm();
        if !length.is_finite() || length <= 0.0 {
            return Err(RodError::InvalidPolyline);
        }
        let next = stations.last().copied().unwrap_or(0.0) + length;
        if !next.is_finite() {
            return Err(RodError::InvalidPolyline);
        }
        stations.push(next);
    }
    Ok(stations)
}

/// Point and unit tangent at arclength s. At a corner the outgoing leg is
/// used, except at the final endpoint, where the incoming leg is used.
pub fn polyline_at(
    polyline: &[Vector3<f64>],
    station_mm: f64,
) -> Result<(Vector3<f64>, Vector3<f64>), RodError> {
    let stations = polyline_stations(polyline)?;
    let length = stations[stations.len() - 1];
    if !station_mm.is_finite() || station_mm < 0.0 || station_mm > length {
        return Err(RodError::InvalidHeadStations);
    }
    let mut index = stations
        .partition_point(|s| *s <= station_mm)
        .saturating_sub(1);
    index = index.min(polyline.len() - 2);
    let tangent = (polyline[index + 1] - polyline[index]) / (stations[index + 1] - stations[index]);
    Ok((
        polyline[index] + tangent * (station_mm - stations[index]),
        tangent,
    ))
}

/// Exact linear shear-flexible two-node circular beam in patient axes.
/// Each node has six freedoms, with rotations about the patient axes.
pub fn element_stiffness(
    start_mm: Vector3<f64>,
    end_mm: Vector3<f64>,
    properties: BeamProperties,
) -> Result<SMatrix<f64, 12, 12>, RodError> {
    if !properties.usable() {
        return Err(RodError::InvalidProperties);
    }
    let direction = end_mm - start_mm;
    let length = direction.norm();
    if !length.is_finite() || length <= 0.0 {
        return Err(RodError::InvalidPolyline);
    }
    let tangent = direction / length;
    let reference = if tangent.y.abs() < 0.9 {
        Vector3::y()
    } else {
        Vector3::z()
    };
    let across = (reference - tangent * tangent.dot(&reference)).normalize();
    let frame = Matrix3::from_columns(&[tangent, across, tangent.cross(&across)]);
    let area = properties.area_mm2();
    let inertia = properties.second_moment_mm4();
    let ei = properties.young_modulus_mpa * inertia;
    let shear = properties.shear_coefficient * properties.shear_modulus_mpa * area;
    let phi = 12.0 * ei / (shear * length * length);
    let denominator = 1.0 + phi;
    let a = 12.0 * ei / (length * length * length * denominator);
    let b = 6.0 * ei / (length * length * denominator);
    let c = (4.0 + phi) * ei / (length * denominator);
    let d = (2.0 - phi) * ei / (length * denominator);
    let mut local = SMatrix::<f64, 12, 12>::zeros();
    for (indices, value) in [
        ([0, 6], properties.young_modulus_mpa * area / length),
        (
            [3, 9],
            properties.shear_modulus_mpa * 2.0 * inertia / length,
        ),
    ] {
        local[(indices[0], indices[0])] = value;
        local[(indices[1], indices[1])] = value;
        local[(indices[0], indices[1])] = -value;
        local[(indices[1], indices[0])] = -value;
    }
    let bending = [[a, b, -a, b], [b, c, -b, d], [-a, -b, a, -b], [b, d, -b, c]];
    for (indices, signs) in [
        ([1, 5, 7, 11], [1.0; 4]),
        ([2, 4, 8, 10], [1.0, -1.0, 1.0, -1.0]),
    ] {
        for i in 0..4 {
            for j in 0..4 {
                local[(indices[i], indices[j])] = signs[i] * signs[j] * bending[i][j];
            }
        }
    }
    let mut transform = SMatrix::<f64, 12, 12>::zeros();
    for offset in [0, 3, 6, 9] {
        transform
            .fixed_view_mut::<3, 3>(offset, offset)
            .copy_from(&frame.transpose());
    }
    let result = transform.transpose() * local * transform;
    if result.iter().any(|value| !value.is_finite()) {
        return Err(RodError::InvalidProperties);
    }
    Ok(result)
}

fn submatrix(matrix: &DMatrix<f64>, rows: &[usize], columns: &[usize]) -> DMatrix<f64> {
    DMatrix::from_fn(rows.len(), columns.len(), |i, j| {
        matrix[(rows[i], columns[j])]
    })
}

/// Eliminate interior six-freedom nodes as a block-tridiagonal chain.
/// Retained heads split that chain into independent pieces; no contour
/// samples are removed and the condensed quadratic form is unchanged.
fn condense_nodes(
    nodes: &[Vector3<f64>],
    heads: &[usize],
    properties: BeamProperties,
) -> Result<DMatrix<f64>, RodError> {
    let mut diagonal = vec![Matrix6::zeros(); nodes.len()];
    let mut lower = Vec::with_capacity(nodes.len() - 1);
    for i in 0..nodes.len() - 1 {
        let element = element_stiffness(nodes[i], nodes[i + 1], properties)?;
        diagonal[i] += element.fixed_view::<6, 6>(0, 0);
        diagonal[i + 1] += element.fixed_view::<6, 6>(6, 6);
        lower.push(element.fixed_view::<6, 6>(6, 0).into_owned());
    }
    let mut head_index = vec![None; nodes.len()];
    for (i, &node) in heads.iter().enumerate() {
        head_index[node] = Some(i);
    }
    let internal: Vec<_> = (0..nodes.len())
        .filter(|&i| head_index[i].is_none())
        .collect();
    let mut internal_index = vec![None; nodes.len()];
    for (i, &node) in internal.iter().enumerate() {
        internal_index[node] = Some(i);
    }
    let size = 6 * heads.len();
    let mut boundary = DMatrix::zeros(size, size);
    let mut coupling = DMatrix::zeros(6 * internal.len(), size);
    let chain_diagonal = internal.iter().map(|&i| diagonal[i]).collect();
    let mut chain_lower = vec![Matrix6::zeros(); internal.len().saturating_sub(1)];
    for (i, &node) in heads.iter().enumerate() {
        boundary
            .view_mut((6 * i, 6 * i), (6, 6))
            .copy_from(&diagonal[node]);
    }
    for (i, block) in lower.iter().enumerate() {
        match (head_index[i], head_index[i + 1]) {
            (Some(a), Some(b)) => {
                boundary.view_mut((6 * b, 6 * a), (6, 6)).copy_from(block);
                boundary
                    .view_mut((6 * a, 6 * b), (6, 6))
                    .copy_from(&block.transpose());
            }
            (Some(a), None) => coupling
                .view_mut((6 * internal_index[i + 1].unwrap(), 6 * a), (6, 6))
                .copy_from(block),
            (None, Some(b)) => coupling
                .view_mut((6 * internal_index[i].unwrap(), 6 * b), (6, 6))
                .copy_from(&block.transpose()),
            (None, None) => chain_lower[internal_index[i].unwrap()] = *block,
        }
    }
    if internal.is_empty() {
        return Ok(boundary);
    }
    // Factor only the clamped internal chain. The head Schur complement
    // remains a free rod with six rigid null modes, so it must not be
    // factored as a positive-definite border.
    let chain = BorderedSystem {
        diagonal: chain_diagonal,
        lower: chain_lower,
        coupling: DMatrix::zeros(6 * internal.len(), 0),
        border: DMatrix::zeros(0, 0),
    };
    let response = chain
        .factor()
        .map_err(|_| RodError::SingularStiffness)?
        .solve_matrix(&coupling)
        .map_err(|_| RodError::SingularStiffness)?;
    let result = boundary - coupling.transpose() * response;
    Ok((&result + result.transpose()) * 0.5)
}

/// Build orthonormal columns with two-pass Gram-Schmidt in a stable order.
fn orthonormal_columns(candidates: &DMatrix<f64>) -> DMatrix<f64> {
    let mut columns: Vec<nalgebra::DVector<f64>> = Vec::new();
    for column in candidates.column_iter() {
        let original_norm = column.norm();
        let mut residual = column.into_owned();
        for _ in 0..2 {
            for basis in &columns {
                residual -= basis * basis.dot(&residual);
            }
        }
        let norm = residual.norm();
        if norm > BASIS_TOLERANCE * original_norm.max(1.0) {
            columns.push(residual / norm);
        }
    }
    DMatrix::from_columns(&columns)
}

/// Orthonormal translation images of all free rigid motions at points.
pub fn rigid_translation_modes(points_mm: &[Vector3<f64>]) -> DMatrix<f64> {
    if points_mm.is_empty() {
        return DMatrix::zeros(0, 0);
    }
    let centre = points_mm.iter().copied().sum::<Vector3<f64>>() / points_mm.len() as f64;
    let candidates = DMatrix::from_fn(points_mm.len() * 3, 6, |row, column| {
        let point = points_mm[row / 3] - centre;
        if column < 3 {
            f64::from(row % 3 == column)
        } else {
            (-point.cross_matrix())[(row % 3, column - 3)]
        }
    });
    orthonormal_columns(&candidates)
}

type ReleasedMatrices = (DMatrix<f64>, DMatrix<f64>, DMatrix<f64>);

fn release_rotations(
    stiffness: &DMatrix<f64>,
    points: &[Vector3<f64>],
) -> Result<ReleasedMatrices, RodError> {
    let translation_indices: Vec<_> = (0..points.len())
        .flat_map(|i| (0..3).map(move |j| i * 6 + j))
        .collect();
    let rotation_indices: Vec<_> = (0..points.len())
        .flat_map(|i| (3..6).map(move |j| i * 6 + j))
        .collect();
    let tt = submatrix(stiffness, &translation_indices, &translation_indices);
    let rt = submatrix(stiffness, &rotation_indices, &translation_indices);
    let mut rr = submatrix(stiffness, &rotation_indices, &rotation_indices);
    // A rigid rotation about a line through collinear heads has zero head
    // translation. Its arbitrary angle is a gauge in the released rr block.
    let furthest = points
        .iter()
        .max_by(|a, b| {
            (*a - points[0])
                .norm_squared()
                .total_cmp(&(*b - points[0]).norm_squared())
        })
        .ok_or(RodError::InvalidHeadStations)?;
    let extent = (furthest - points[0]).norm();
    if extent <= 0.0 {
        return Err(RodError::InvalidHeadStations);
    }
    let axis = (furthest - points[0]) / extent;
    if points
        .iter()
        .all(|p| (p - points[0]).cross(&axis).norm() <= BASIS_TOLERANCE * extent.max(1.0))
    {
        let mode = nalgebra::DVector::from_fn(points.len() * 3, |row, _| {
            axis[row % 3] / libm::sqrt(points.len() as f64)
        });
        let scale = rr.diagonal().max();
        rr += (&mode * mode.transpose()) * scale;
    }
    let factor = rr.cholesky().ok_or(RodError::SingularStiffness)?;
    let released = tt - rt.transpose() * factor.solve(&rt);
    let modes = rigid_translation_modes(points);
    let rigid_projector = &modes * modes.transpose();
    let projector = DMatrix::identity(points.len() * 3, points.len() * 3) - &rigid_projector;
    let result = &projector * released * &projector;
    let result = (&result + result.transpose()) * 0.5;
    let scale = stiffness.diagonal().max();
    let gauged = &result + rigid_projector * scale;
    let factor = gauged.cholesky().ok_or(RodError::SingularStiffness)?;
    let compliance = &projector * factor.solve(&projector);
    let compliance = (&compliance + compliance.transpose()) * 0.5;
    Ok((result, compliance, modes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::DVector;

    fn properties() -> BeamProperties {
        // Deliberately synthetic choices, unrelated to material tables.
        BeamProperties {
            young_modulus_mpa: 90000.0,
            shear_modulus_mpa: 35000.0,
            diameter_mm: 4.0,
            shear_coefficient: 0.8,
        }
    }

    #[test]
    fn a_clamped_tip_has_the_closed_form_shear_flexible_compliance() {
        let p = properties();
        for length in [12.0, 100.0] {
            let rod = CondensedRod::new(
                &[Vector3::zeros(), Vector3::x() * length],
                &[0.0, length],
                p,
            )
            .unwrap();
            let tip = rod.stiffness.view((6, 6), (6, 6)).into_owned();
            let mut load = DVector::zeros(6);
            load[1] = 11.0;
            let displacement = tip.cholesky().unwrap().solve(&load);
            let ei = p.young_modulus_mpa * p.second_moment_mm4();
            let expected = 11.0 * length * length * length / (3.0 * ei)
                + 11.0 * length / (p.shear_coefficient * p.shear_modulus_mpa * p.area_mm2());
            assert!((displacement[1] - expected).abs() < expected.abs() * 1e-11);
            assert!((displacement[5] - 11.0 * length * length / (2.0 * ei)).abs() < 1e-12);
            assert!(displacement[0].abs() < 1e-12 && displacement[2].abs() < 1e-12);
        }
    }

    #[test]
    fn dense_contour_samples_keep_the_closed_form_cantilever_compliance() {
        let p = properties();
        let length = 160.0;
        let points: Vec<_> = (0..=80).map(|i| Vector3::x() * (2.0 * i as f64)).collect();
        let rod = CondensedRod::new(&points, &[0.0, length], p).unwrap();
        let tip = rod.stiffness.view((6, 6), (6, 6)).into_owned();
        let mut load = DVector::zeros(6);
        load[1] = 1.0;
        let displacement = tip.cholesky().unwrap().solve(&load);
        let expected = length * length * length
            / (3.0 * p.young_modulus_mpa * p.second_moment_mm4())
            + length / (p.shear_coefficient * p.shear_modulus_mpa * p.area_mm2());
        assert!((displacement[1] - expected).abs() < expected * 1e-7);
    }

    #[test]
    fn every_rigid_motion_costs_no_strain_energy() {
        let points = [
            Vector3::new(-3.0, 0.0, 0.0),
            Vector3::new(2.0, 0.0, 20.0),
            Vector3::new(-1.0, 0.0, 40.0),
        ];
        let stations = polyline_stations(&points).unwrap();
        let rod = CondensedRod::new(&points, &stations, properties()).unwrap();
        assert_eq!(rod.rigid_modes.ncols(), 6);
        assert!((&rod.translations * &rod.rigid_modes).norm() < 1e-8);
        for mode in 0..6 {
            let axis = if mode >= 3 {
                Vector3::ith(mode - 3, 1.0)
            } else {
                Vector3::zeros()
            };
            let translation = if mode < 3 {
                Vector3::ith(mode, 1.0)
            } else {
                Vector3::zeros()
            };
            let movement = DVector::from_fn(points.len() * 6, |row, _| {
                if row % 6 < 3 {
                    (translation + axis.cross(&points[row / 6]))[row % 6]
                } else {
                    axis[row % 6 - 3]
                }
            });
            assert!((&rod.stiffness * movement).norm() < 1e-6);
        }
    }

    #[test]
    fn releasing_two_collinear_heads_leaves_only_axial_stiffness() {
        let length = 30.0;
        let p = properties();
        let points = [Vector3::zeros(), Vector3::z() * length];
        let rod = CondensedRod::new(&points, &[0.0, length], p).unwrap();
        assert_eq!(rod.rigid_modes.ncols(), 5);
        let axial = p.young_modulus_mpa * p.area_mm2() / length;
        let mut expected = DMatrix::zeros(6, 6);
        expected[(2, 2)] = axial;
        expected[(5, 5)] = axial;
        expected[(2, 5)] = -axial;
        expected[(5, 2)] = -axial;
        assert!((&rod.translations - expected).norm() < axial * 1e-12);
        let forces = DVector::from_vec(vec![0.0, 0.0, -9.0, 0.0, 0.0, 9.0]);
        let displacement = &rod.compliance * &forces;
        assert!((&rod.translations * displacement - forces).norm() < 1e-10);
    }
    #[test]
    fn a_bent_rods_compliance_and_forces_scale_with_its_moduli() {
        let points = [
            Vector3::new(-3.0, 2.0, 0.0),
            Vector3::new(2.0, -1.0, 17.0),
            Vector3::new(-1.0, 3.0, 41.0),
        ];
        let stations = polyline_stations(&points).unwrap();
        let p = properties();
        let rod = CondensedRod::new(&points, &stations, p).unwrap();
        let raw_load = DVector::from_vec(vec![3.0, -2.0, 1.0, -5.0, 4.0, 2.0, 7.0, -3.0, -1.0]);
        let balanced = &raw_load - &rod.rigid_modes * (rod.rigid_modes.transpose() * &raw_load);
        let motion = &rod.compliance * &balanced;
        assert!((&rod.translations * &motion - &balanced).norm() < 2e-9);
        assert!((rod.rigid_modes.transpose() * &rod.compliance).norm() < 1e-12);
        for scale in [0.25, 4.0] {
            let scaled = CondensedRod::new(
                &points,
                &stations,
                BeamProperties {
                    young_modulus_mpa: p.young_modulus_mpa * scale,
                    shear_modulus_mpa: p.shear_modulus_mpa * scale,
                    ..p
                },
            )
            .unwrap();
            assert!(
                (&scaled.stiffness - &rod.stiffness * scale).norm() < rod.stiffness.norm() * 1e-13
            );
            assert!(
                (&scaled.translations - &rod.translations * scale).norm()
                    < rod.translations.norm() * 1e-11
            );
            assert!(
                (&scaled.compliance * scale - &rod.compliance).norm()
                    < rod.compliance.norm() * 1e-10
            );
            assert!(
                (&scaled.compliance * (&balanced * scale) - &motion).norm() < motion.norm() * 1e-10
            );
        }
    }

    #[test]
    fn turning_and_mirroring_a_rod_turns_its_stiffness_and_compliance() {
        use forward_model::rotation::{rot_x, rot_z};
        let points = [
            Vector3::new(-3.0, 2.0, 0.0),
            Vector3::new(2.0, -1.0, 17.0),
            Vector3::new(-1.0, 3.0, 41.0),
        ];
        let stations = polyline_stations(&points).unwrap();
        let rod = CondensedRod::new(&points, &stations, properties()).unwrap();
        for frame in [
            rot_x(0.43) * rot_z(-0.82),
            Matrix3::from_diagonal(&Vector3::new(1.0, -1.0, 1.0)),
        ] {
            let translation = Vector3::new(8.0, 4.0, -12.0);
            let moved_points: Vec<_> = points.iter().map(|p| frame * p + translation).collect();
            // Derive new arclengths after rounding the moved coordinates.
            let moved_stations = polyline_stations(&moved_points).unwrap();
            let moved = CondensedRod::new(&moved_points, &moved_stations, properties()).unwrap();
            let mut t6 = DMatrix::zeros(18, 18);
            let mut t3 = DMatrix::zeros(9, 9);
            let axial_frame = frame * frame.determinant();
            for node in 0..3 {
                t6.fixed_view_mut::<3, 3>(node * 6, node * 6)
                    .copy_from(&frame);
                t6.fixed_view_mut::<3, 3>(node * 6 + 3, node * 6 + 3)
                    .copy_from(&axial_frame);
                t3.fixed_view_mut::<3, 3>(node * 3, node * 3)
                    .copy_from(&frame);
            }
            assert!(
                (&moved.stiffness - &t6 * &rod.stiffness * t6.transpose()).norm()
                    < rod.stiffness.norm() * 1e-12
            );
            assert!(
                (&moved.translations - &t3 * &rod.translations * t3.transpose()).norm()
                    < rod.translations.norm() * 1e-10
            );
            assert!(
                (&moved.compliance - &t3 * &rod.compliance * t3.transpose()).norm()
                    < rod.compliance.norm() * 1e-9
            );
        }
    }
    #[test]
    fn a_long_rods_balanced_compliance_retains_its_bending_modes() {
        let points: Vec<_> = (0..17)
            .map(|i| {
                let z = i as f64 * 25.0;
                Vector3::new(0.0008 * z * (400.0 - z), 0.00005 * z * (400.0 - z), z)
            })
            .collect();
        let stations = polyline_stations(&points).unwrap();
        let rod = CondensedRod::new(&points, &stations, properties()).unwrap();
        let raw_load = DVector::from_fn(points.len() * 3, |row, _| {
            ((row * 37 % 17) as f64 - 8.0) * 0.3
        });
        let balanced = &raw_load - &rod.rigid_modes * (rod.rigid_modes.transpose() * &raw_load);
        let displacement = &rod.compliance * &balanced;
        let residue = &rod.translations * displacement - &balanced;
        assert!(
            residue.norm() < balanced.norm() * 1e-9,
            "relative residual {}",
            residue.norm() / balanced.norm()
        );
    }
}
