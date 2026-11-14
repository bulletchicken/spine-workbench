//! Named draws shared by all placements, independent of batch boundaries.
//! Geometry uses the layout sampler unchanged. Bench dimensions have their
//! own names; yield and ultimate always use the same material quantile.

use crate::BenchError;
use crate::evidence::{Evidence, TableKind, Unit};
use implant_layout::readings::{NamedSampler, Version};
use shapes::implant_layout::{ActivityLevel, ChildFacts, RodMaterial};
use shapes::load_bench::VersionDraws;
use shapes::spine_fit::SpineFit;

/// One shared geometry with the bench's extra draws. Material strengths can
/// be read for each rod without changing this version's identity.
#[derive(Clone, Debug)]
pub struct Draw {
    pub geometry: Version,
    pub values: VersionDraws,
    pub strength_quantile: f64,
    pub anchor_quantile: f64,
}

pub fn material_key(material: RodMaterial) -> &'static str {
    match material {
        RodMaterial::TitaniumAlloy => "titanium-alloy",
        RodMaterial::CobaltChrome => "cobalt-chrome",
        RodMaterial::StainlessSteel => "stainless-steel",
    }
}

/// Yield and ultimate in MPa, with their common quantile.
pub fn strengths(e: &Evidence, material: RodMaterial, q: f64) -> Result<(f64, f64), BenchError> {
    let p = e
        .bench_strengths(material)
        .ok_or_else(|| refusal("missing-rod-strength"))?;
    Ok((
        p.yield_lower.value + q * (p.yield_upper.value - p.yield_lower.value),
        p.ultimate_lower.value + q * (p.ultimate_upper.value - p.ultimate_lower.value),
    ))
}

pub fn draw(
    fit: &SpineFit,
    child: &ChildFacts,
    seed: u32,
    n: u32,
    k: u32,
    e: &Evidence,
) -> Result<Draw, BenchError> {
    let geometry = implant_layout::readings::version(fit, u64::from(seed), n, k)
        .map_err(|error| refusal(&error.to_string()))?;
    let sampler =
        NamedSampler::new(u64::from(seed), n).map_err(|error| refusal(&error.to_string()))?;
    let u = |name: &str| {
        sampler
            .uniform(&format!("versions-bench/{name}"), k)
            .map_err(|error| refusal(&error.to_string()))
    };
    parameters(geometry, child, e, u, || {
        sampler
            .normal("versions-bench/slice-offset", k)
            .map_err(|error| refusal(&error.to_string()))
    })
}

/// Only the unknowns read while seating rods, before any daily loads.
pub(crate) struct CorrectionDraw {
    pub stiffness_scale: f64,
    pub rib_factors: [f64; 3],
}

/// Drawing uses the unchanged fit and quantile one half for tissue draws.
/// Pedicle draws are deliberately absent: seating does not read them.
/// Weight, activity and the other load/strength draws are not used either.
pub(crate) fn middle(e: &Evidence) -> Result<CorrectionDraw, BenchError> {
    correction_parameters(e, &|_| Ok(0.5))
}

fn correction_parameters(
    e: &Evidence,
    u: &impl Fn(&str) -> Result<f64, BenchError>,
) -> Result<CorrectionDraw, BenchError> {
    let span = |row, name| -> Result<f64, BenchError> {
        let lower = cell(e, TableKind::JointSprings, row, "lower", Unit::Ratio)?;
        Ok(lower
            + u(name)? * (cell(e, TableKind::JointSprings, row, "upper", Unit::Ratio)? - lower))
    };
    let low = cell(
        e,
        TableKind::JointSprings,
        "stiffness-scale",
        "lower",
        Unit::Ratio,
    )?;
    let high = cell(
        e,
        TableKind::JointSprings,
        "stiffness-scale",
        "upper",
        Unit::Ratio,
    )?;
    let stiffness_scale = libm::exp(libm::log(low) + u("stiffness-scale")? * libm::log(high / low));
    // Flexion bounds are the printed with/without-rib ratios at T1–T4
    // and T8–T12. Interpolation between those bounds is a project prior.
    let ratio = |row| -> Result<f64, BenchError> {
        Ok(cell(
            e,
            TableKind::JointSprings,
            row,
            "fe-with-ribs",
            Unit::Unstated,
        )? / cell(
            e,
            TableKind::JointSprings,
            row,
            "fe-without-ribs",
            Unit::Unstated,
        )?)
    };
    let fe_low = ratio("rib-mannen-t1-t4")?;
    let fe_high = ratio("rib-mannen-t8-t12")?;
    let rib_factors = [
        fe_low + u("rib-flexion")? * (fe_high - fe_low),
        span("rib-lateral-bending", "rib-lateral")?,
        span("rib-axial-rotation", "rib-axial")?,
    ];
    Ok(CorrectionDraw {
        stiffness_scale,
        rib_factors,
    })
}

fn parameters(
    geometry: Version,
    child: &ChildFacts,
    e: &Evidence,
    u: impl Fn(&str) -> Result<f64, BenchError>,
    normal_offset: impl Fn() -> Result<f64, BenchError>,
) -> Result<Draw, BenchError> {
    let span = |table, row, lo, hi, unit, name| -> Result<f64, BenchError> {
        let lower = cell(e, table, row, lo, unit)?;
        Ok(lower + u(name)? * (cell(e, table, row, hi, unit)? - lower))
    };
    let CorrectionDraw {
        stiffness_scale,
        rib_factors,
    } = correction_parameters(e, &u)?;
    let bone_u = u("bone")?;
    let low_share = cell(
        e,
        TableKind::AnchorStrength,
        "bone-prior",
        "low-share-numerator",
        Unit::Ratio,
    )? / cell(
        e,
        TableKind::AnchorStrength,
        "bone-prior",
        "low-share-denominator",
        Unit::Ratio,
    )?;
    let bone_factor = if bone_u < low_share {
        let lower = cell(
            e,
            TableKind::AnchorStrength,
            "bone-prior",
            "low-lower",
            Unit::Ratio,
        )?;
        lower
            + bone_u / low_share
                * (cell(
                    e,
                    TableKind::AnchorStrength,
                    "bone-prior",
                    "low-upper",
                    Unit::Ratio,
                )? - lower)
    } else {
        cell(
            e,
            TableKind::AnchorStrength,
            "bone-prior",
            "normal-factor",
            Unit::Ratio,
        )?
    };
    let weight_kg = match child.weight_kg {
        Some(w) if w.is_finite() && w > 0.0 => w,
        Some(_) => return Err(refusal("weight-must-be-positive")),
        None => span(
            TableKind::DailyLoads,
            "weight-if-absent",
            "lower",
            "upper",
            Unit::Kg,
            "weight",
        )?,
    };
    let (row, lower, upper) = match child.activity {
        None => ("steps-if-absent", "lower", "upper"),
        Some(ActivityLevel::Low) => ("steps-low", "lower-inclusive", "upper-inclusive"),
        Some(ActivityLevel::Typical) => ("steps-typical", "lower-inclusive", "upper-inclusive"),
        Some(ActivityLevel::Active) => ("steps-active", "lower-inclusive", "upper-inclusive"),
        Some(ActivityLevel::VeryLow | ActivityLevel::VeryActive) => {
            return Err(refusal("open-ended-activity-band"));
        }
    };
    let strength_quantile = u("rod-strength")?;
    Ok(Draw {
        geometry,
        strength_quantile,
        anchor_quantile: u("thoracic-strength")?,
        values: VersionDraws {
            stiffness_scale,
            rib_factors,
            bone_factor,
            weight_kg,
            steps_per_day: span(
                TableKind::DailyLoads,
                row,
                lower,
                upper,
                Unit::StepsPerDay,
                "steps",
            )?,
            walking_multiple: span(
                TableKind::DailyLoads,
                "walking",
                "lower",
                "upper",
                Unit::Ratio,
                "walking",
            )?,
            // Filled for each placement's first rod by the batch builder.
            rod_yield_mpa: strengths(e, RodMaterial::CobaltChrome, strength_quantile)?.0,
            preload_factor: span(
                TableKind::JointSprings,
                "preload",
                "lower",
                "upper",
                Unit::Ratio,
                "preload",
            )?,
            slice_offset_mm: cell(e, TableKind::DailyLoads, "slice-offset", "mean", Unit::Mm)?
                + cell(e, TableKind::DailyLoads, "slice-offset", "sd", Unit::Mm)?
                    * normal_offset()?,
            contour_knock_down: span(
                TableKind::Fatigue,
                "contour-knock-down",
                "lower",
                "upper",
                Unit::Ratio,
                "contour-knock-down",
            )?,
        },
    })
}

pub(crate) fn cell(
    e: &Evidence,
    table: TableKind,
    row: &str,
    term: &str,
    unit: Unit,
) -> Result<f64, BenchError> {
    e.value(table, row, term)
        .filter(|v| v.unit == unit && v.value.is_finite())
        .map(|v| v.value)
        .ok_or_else(|| refusal(&format!("missing-evidence:{row}/{term}")))
}

pub(crate) fn refusal(reason: &str) -> BenchError {
    BenchError::Refused(reason.to_owned())
}
