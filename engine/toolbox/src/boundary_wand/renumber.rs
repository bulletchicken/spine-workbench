//! Names follow an anchor, never a click's image evidence. When
//! `l5ConfirmedOn` names a surviving vertebra, that id stays L5 and every
//! other name is counted from it through the film template's names. Thus
//! names on the far side of a split or merge shift while L5 keeps its
//! confirmation. Without a confirmed L5, counting starts at the top as
//! in automatic naming, and names below the edit shift. Retiring the
//! confirmed id clears its confirmation and reports a naming conflict.
//! Both independent counts are rebuilt, and names outside the template's
//! usual levels remain explicitly flagged as a non-standard count.

use shapes::levels::Level;
use shapes::naming::{NamingFlag, Template};
use shapes::toolbox::{MissReason, ToolWarning, VertebraLevel};
use shapes::vertebra::FilmMarks;

fn counted(template: Template, count: usize, index: usize, level: Level) -> Option<Vec<Level>> {
    let names = template.names();
    let start = names.iter().position(|v| *v == level)?.checked_sub(index)?;
    names
        .get(start..start.checked_add(count)?)
        .map(<[_]>::to_vec)
}

fn independent(before: &FilmMarks, after: &FilmMarks, bottom: bool) -> Option<Vec<Level>> {
    let old = if bottom {
        before.naming.from_bottom.as_ref()
    } else {
        before.naming.from_top.as_ref()
    }?;
    let mut indices: Vec<_> = (0..before.vertebrae.len()).collect();
    if bottom {
        indices.reverse();
    }
    for old_index in indices {
        let v = &before.vertebrae[old_index];
        if let Some(index) = after.vertebrae.iter().position(|new| new.id == v.id) {
            return counted(
                before.naming.template,
                after.vertebrae.len(),
                index,
                *old.get(old_index)?,
            );
        }
    }
    None
}

pub(super) fn apply(
    before: &FilmMarks,
    after: &mut FilmMarks,
    warnings: &mut Vec<ToolWarning>,
) -> Result<Vec<VertebraLevel>, MissReason> {
    let top = independent(before, after, false);
    let bottom = independent(before, after, true);
    let confirmed = before
        .naming
        .l5_confirmed_on
        .and_then(|id| after.vertebrae.iter().position(|v| v.id == id));
    let primary = if let Some(index) = confirmed {
        counted(
            before.naming.template,
            after.vertebrae.len(),
            index,
            Level::L5,
        )
    } else {
        top.clone().or_else(|| {
            let first = before.vertebrae.first()?;
            let index = after.vertebrae.iter().position(|v| v.id == first.id)?;
            counted(
                before.naming.template,
                after.vertebrae.len(),
                index,
                first.level,
            )
        })
    }
    .ok_or(MissReason::NamesRunOut)?;
    if before.naming.l5_confirmed_on.is_some() && confirmed.is_none() {
        after.naming.l5_confirmed_on = None;
        warnings.push(ToolWarning::NamingConflict);
    }
    after.naming.from_top = top;
    after.naming.from_bottom = bottom;
    after
        .naming
        .flags
        .retain(|f| !matches!(f, NamingFlag::EndsDisagree | NamingFlag::NonStandardCount));
    if let (Some(top), Some(bottom)) = (&after.naming.from_top, &after.naming.from_bottom)
        && top != bottom
    {
        after.naming.flags.push(NamingFlag::EndsDisagree);
        if !warnings.contains(&ToolWarning::NamingConflict) {
            warnings.push(ToolWarning::NamingConflict);
        }
    }
    let usual = before.naming.template.levels();
    if before.naming.template != Template::Standard
        || primary.iter().any(|level| !usual.contains(level))
    {
        after.naming.flags.push(NamingFlag::NonStandardCount);
    }
    after.naming.flags.sort();
    after.naming.flags.dedup();
    let mut levels = Vec::new();
    for (vertebra, level) in after.vertebrae.iter_mut().zip(primary) {
        if before
            .vertebrae
            .iter()
            .find(|v| v.id == vertebra.id)
            .is_none_or(|v| v.level != level)
        {
            levels.push(VertebraLevel {
                vertebra: vertebra.id,
                level,
            });
        }
        vertebra.level = level;
    }
    Ok(levels)
}
