//! Resultant head force against a published pull-out figure and a bone factor.
//!
//! Forces are N in any orthonormal frame (patient axes at rods-in, the
//! vertebra's seated axes at daily stops); only the resultant length is used.
//! Either action sign is equivalent. The ratio is a comparison
//! with cadaveric test figures, not a probability. Screw diameter and length
//! do not scale capacity: the thoracic reference dimensions and the lumbar
//! reference length are absent. Head couples are not part of this force ratio.

use crate::evidence::{Evidence, TableKind, Unit, Value};
use nalgebra::Vector3;
use shapes::implant_layout::AnchorKind;
use shapes::levels::Level;

/// A head's resultant force and its dimensionless demand against the
/// published figure multiplied by the supplied bone factor.
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorDemand {
    pub force_n: f64,
    pub demand: f64,
    /// Before applying the bone factor; absent for an empty pedicle.
    pub published_strength_n: Option<f64>,
    /// Published strength times bone factor; absent for an empty pedicle.
    pub capacity_n: Option<f64>,
    /// The one hook figure is from T4–T8. Its use elsewhere is a project
    /// borrowing, retained explicitly rather than a new per-level figure.
    pub borrowed_hook_region: bool,
    pub sources: Vec<String>,
}

/// An unavailable strength is refused rather than replaced by a number.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AnchorDemandError {
    #[error("anchor-force-must-be-finite")]
    InvalidForce,
    #[error("anchor-bone-factor-must-be-finite-and-positive")]
    InvalidBoneFactor,
    #[error("anchor-thoracic-quantile-must-be-in-zero-to-one")]
    InvalidQuantile,
    #[error("anchor-no-strength-for-level: {0:?}")]
    NoStrengthForLevel(Level),
    #[error("anchor-missing-evidence: {0}")]
    MissingEvidence(String),
    #[error("anchor-invalid-strength-cell: {0}")]
    InvalidEvidence(String),
    #[error("anchor-demand-is-not-finite")]
    NonFiniteDemand,
}

/// Compare one head's force with the region's test figure. The caller
/// supplies the bone draw and a quantile for the T1–T8 trajectory range;
/// no sampling occurs here. Other regions ignore that quantile after
/// checking it. The T1–T8 uniform interpolation and regional borrowing are
/// this project's choices. Empty pedicles return zero force and demand.
pub fn at_head(
    evidence: &Evidence,
    level: Level,
    kind: AnchorKind,
    force_n: Vector3<f64>,
    bone_factor: f64,
    thoracic_quantile: f64,
) -> Result<AnchorDemand, AnchorDemandError> {
    if force_n.iter().any(|value| !value.is_finite()) {
        return Err(AnchorDemandError::InvalidForce);
    }
    if !bone_factor.is_finite() || bone_factor <= 0.0 {
        return Err(AnchorDemandError::InvalidBoneFactor);
    }
    if !thoracic_quantile.is_finite() || !(0.0..=1.0).contains(&thoracic_quantile) {
        return Err(AnchorDemandError::InvalidQuantile);
    }
    if kind == AnchorKind::None {
        return Ok(AnchorDemand {
            force_n: 0.0,
            demand: 0.0,
            published_strength_n: None,
            capacity_n: None,
            borrowed_hook_region: false,
            sources: Vec::new(),
        });
    }
    if !(Level::T1..=Level::T12).contains(&level) && !(Level::L1..=Level::L5).contains(&level) {
        return Err(AnchorDemandError::NoStrengthForLevel(level));
    }
    let (published_strength_n, mut sources) = match kind {
        AnchorKind::Screw if level <= Level::T8 => {
            let lower = strength(evidence, "thoracic-sampling", "lower")?;
            let upper = strength(evidence, "thoracic-sampling", "upper")?;
            if upper.value < lower.value {
                return Err(AnchorDemandError::InvalidEvidence(
                    "thoracic-sampling".into(),
                ));
            }
            let value = lower.value + thoracic_quantile * (upper.value - lower.value);
            (value, vec![lower.source.clone(), upper.source.clone()])
        }
        AnchorKind::Screw => {
            let (row, term) = if level <= Level::T12 {
                ("t9-t12-screw", "mean")
            } else {
                ("lumbar-screw", "normal")
            };
            let cell = strength(evidence, row, term)?;
            (cell.value, vec![cell.source.clone()])
        }
        AnchorKind::Hook => {
            let cell = strength(evidence, "t4-t8-pedicle-hook", "mean")?;
            (cell.value, vec![cell.source.clone()])
        }
        AnchorKind::None => unreachable!("empty pedicle returned above"),
    };
    sources.push("project-assumptions".into());
    sources.sort();
    sources.dedup();
    let force_n = force_n.norm();
    let capacity_n = published_strength_n * bone_factor;
    let demand = force_n / capacity_n;
    if !force_n.is_finite() || !capacity_n.is_finite() || capacity_n <= 0.0 || !demand.is_finite() {
        return Err(AnchorDemandError::NonFiniteDemand);
    }
    Ok(AnchorDemand {
        force_n,
        demand,
        published_strength_n: Some(published_strength_n),
        capacity_n: Some(capacity_n),
        borrowed_hook_region: kind == AnchorKind::Hook && !(Level::T4..=Level::T8).contains(&level),
        sources,
    })
}

fn strength<'a>(
    evidence: &'a Evidence,
    row: &str,
    term: &str,
) -> Result<&'a Value, AnchorDemandError> {
    let key = format!("{row}/{term}");
    let value = evidence
        .value(TableKind::AnchorStrength, row, term)
        .ok_or_else(|| AnchorDemandError::MissingEvidence(key.clone()))?;
    if value.unit != Unit::N || !value.value.is_finite() || value.value <= 0.0 {
        return Err(AnchorDemandError::InvalidEvidence(key));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_three_four_five_force_uses_the_region_and_bone_factor() {
        let evidence = Evidence::read().unwrap();
        let force = Vector3::new(3.0, -4.0, 0.0);
        let normal = at_head(&evidence, Level::L3, AnchorKind::Screw, force, 1.0, 0.5).unwrap();
        let weaker = at_head(&evidence, Level::L3, AnchorKind::Screw, -force, 0.5, 0.5).unwrap();
        let cell = evidence
            .value(TableKind::AnchorStrength, "lumbar-screw", "normal")
            .unwrap();
        // Pythagoras gives 5 N; half the capacity doubles the ratio.
        assert_eq!(normal.force_n, 5.0);
        assert_eq!(normal.published_strength_n, Some(cell.value));
        assert!((normal.demand * cell.value - 5.0).abs() < 1e-14);
        assert_eq!(weaker.force_n, normal.force_n);
        assert_eq!(weaker.demand, 2.0 * normal.demand);
        assert_eq!(
            weaker.capacity_n,
            normal.capacity_n.map(|value| value * 0.5)
        );
        assert!(normal.sources.contains(&cell.source));
    }

    #[test]
    fn thoracic_regions_and_hook_borrowing_remain_explicit() {
        let evidence = Evidence::read().unwrap();
        let force = Vector3::z() * 10.0;
        let lower = evidence
            .value(TableKind::AnchorStrength, "thoracic-sampling", "lower")
            .unwrap();
        let upper = evidence
            .value(TableKind::AnchorStrength, "thoracic-sampling", "upper")
            .unwrap();
        let mid = at_head(&evidence, Level::T8, AnchorKind::Screw, force, 1.0, 0.5).unwrap();
        assert_eq!(
            mid.published_strength_n,
            Some((lower.value + upper.value) * 0.5)
        );
        let distal = at_head(&evidence, Level::T9, AnchorKind::Screw, force, 1.0, 0.5).unwrap();
        let distal_cell = evidence
            .value(TableKind::AnchorStrength, "t9-t12-screw", "mean")
            .unwrap();
        assert_eq!(distal.published_strength_n, Some(distal_cell.value));
        let hook = at_head(&evidence, Level::T4, AnchorKind::Hook, force, 1.0, 0.5).unwrap();
        let borrowed = at_head(&evidence, Level::T2, AnchorKind::Hook, force, 1.0, 0.5).unwrap();
        let hook_cell = evidence
            .value(TableKind::AnchorStrength, "t4-t8-pedicle-hook", "mean")
            .unwrap();
        assert_eq!(hook.published_strength_n, Some(hook_cell.value));
        assert!(!hook.borrowed_hook_region);
        assert!(borrowed.borrowed_hook_region);
        assert_eq!(borrowed.capacity_n, hook.capacity_n);
        assert_eq!(borrowed.demand, hook.demand);
        for source in mid.sources.iter().chain(&borrowed.sources) {
            assert!(evidence.source(source).is_some());
        }
    }

    #[test]
    fn an_empty_pedicle_has_no_force_or_capacity_and_bad_inputs_are_refused() {
        let evidence = Evidence::read().unwrap();
        let empty = at_head(
            &evidence,
            Level::T5,
            AnchorKind::None,
            Vector3::new(3.0, 4.0, 0.0),
            1.0,
            0.5,
        )
        .unwrap();
        assert_eq!(empty.force_n, 0.0);
        assert_eq!(empty.demand, 0.0);
        assert_eq!(empty.capacity_n, None);
        assert_eq!(empty.published_strength_n, None);
        assert!(empty.sources.is_empty());
        let at = |level, bone, q| {
            at_head(
                &evidence,
                level,
                AnchorKind::Screw,
                Vector3::zeros(),
                bone,
                q,
            )
        };
        assert_eq!(
            at(Level::T13, 1.0, 0.5).unwrap_err(),
            AnchorDemandError::NoStrengthForLevel(Level::T13)
        );
        assert_eq!(
            at(Level::T5, 0.0, 0.5).unwrap_err(),
            AnchorDemandError::InvalidBoneFactor
        );
        assert_eq!(
            at(Level::T5, 1.0, 1.5).unwrap_err(),
            AnchorDemandError::InvalidQuantile
        );
    }
}
