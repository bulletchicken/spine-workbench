//! Each alternative changes one feature of the first placement. End
//! shifts use anatomical names, never the next available cropped body.

use crate::patterns::Pattern;
use crate::rods::{self, RodChoice};
use shapes::implant_layout::{ImplantLayout, LayoutChange, LayoutInput, RodMaterial};
use shapes::levels::Level;

/// A single change before its anchors and rods are constructed and checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Alternative {
    pub id: String,
    pub change: LayoutChange,
    pub upper: Level,
    pub lower: Level,
    pub pattern: Pattern,
    pub rod: RodChoice,
}

/// A fitted span constrained to T1 through L5, with no missing body hidden
/// inside it. Nonstandard counts retain their own anatomical order.
pub fn levels(input: &LayoutInput, upper: Level, lower: Level) -> Option<Vec<Level>> {
    let fit = &input.fit;
    if upper < Level::T1 || lower > Level::L5 || upper >= lower {
        return None;
    }
    let names = names(input);
    let start = names.iter().position(|&level| level == upper)?;
    let end = names.iter().position(|&level| level == lower)?;
    let span = names.get(start..=end)?;
    span.iter()
        .all(|level| fit.vertebrae.iter().any(|v| v.level == *level))
        .then(|| span.to_vec())
}

fn shift(input: &LayoutInput, level: Level, delta: i32) -> Option<Level> {
    let names = names(input);
    let i = names.iter().position(|&name| name == level)?;
    names.get(i.checked_add_signed(delta as isize)?).copied()
}

/// Stable order: lower end up/down, upper end up/down, fewer screws,
/// stiffer rods, softer rods, hooks at the top. Unsupported rod pairs are
/// absent, even when their diameter or material has a contract name.
pub fn one_step(input: &LayoutInput, first: &ImplantLayout, rod: &RodChoice) -> Vec<Alternative> {
    let mut result = Vec::new();
    for (is_lower, delta) in [(true, -1), (true, 1), (false, -1), (false, 1)] {
        let Some(level) = shift(
            input,
            if is_lower { first.lower } else { first.upper },
            delta,
        ) else {
            continue;
        };
        let (upper, lower) = if is_lower {
            (first.upper, level)
        } else {
            (level, first.lower)
        };
        if levels(input, upper, lower).is_none() {
            continue;
        }
        result.push(Alternative {
            id: format!(
                "{}-{level:?}",
                if is_lower { "ends-at" } else { "starts-at" }
            ),
            change: if is_lower {
                LayoutChange::EndsAt { level }
            } else {
                LayoutChange::StartsAt { level }
            },
            upper,
            lower,
            pattern: Pattern::AllPedicles,
            rod: rod.clone(),
        });
    }
    let mut add = |id: &str, change, pattern, rod: RodChoice| {
        result.push(Alternative {
            id: id.into(),
            change,
            upper: first.upper,
            lower: first.lower,
            pattern,
            rod,
        })
    };
    add(
        "fewer-screws",
        LayoutChange::FewerScrews {},
        Pattern::KeyLevels,
        rod.clone(),
    );
    let choices = rods::choices();
    if let Some(stiffer) = choices
        .iter()
        .filter(|c| c.material == rod.material && c.diameter_mm > rod.diameter_mm)
        .min_by(|a, b| a.diameter_mm.total_cmp(&b.diameter_mm))
    {
        add(
            "stiffer-rods",
            LayoutChange::StifferRods {},
            Pattern::AllPedicles,
            stiffer.clone(),
        );
    }
    if let Some(softer) = choices
        .iter()
        .find(|c| c.material == RodMaterial::TitaniumAlloy && c.diameter_mm == rod.diameter_mm)
    {
        add(
            "softer-rods",
            LayoutChange::SofterRods {},
            Pattern::AllPedicles,
            softer.clone(),
        );
    }
    add(
        "hooks-at-top",
        LayoutChange::HooksAtTop {},
        Pattern::HooksAtTop,
        rod.clone(),
    );
    result
}

fn names(input: &LayoutInput) -> Vec<Level> {
    input
        .front
        .as_ref()
        .map_or_else(
            || crate::level_rules::support::fit_template(&input.fit),
            |f| f.marks.naming.template,
        )
        .names()
}
