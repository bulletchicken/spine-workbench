//! Level naming: which count of vertebrae a film's chain was solved with,
//! the two independent counts (from the first rib down, from the sacrum
//! up), and what the user settled.
//!
//! The count and the names come only from image evidence and from the
//! user's choice in the naming panel. A click on a film never feeds them.

use crate::levels::{Level, Region};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vertebra count the chain search can solve with.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    /// C7, T1 to T12, L1 to L5.
    Standard,
    /// Adds T13.
    ExtraThoracic,
    /// Adds L6.
    ExtraLumbar,
    /// No T12.
    MissingThoracic,
    /// No L5.
    MissingLumbar,
}

impl Template {
    pub const ALL: [Template; 5] = [
        Template::Standard,
        Template::ExtraThoracic,
        Template::ExtraLumbar,
        Template::MissingThoracic,
        Template::MissingLumbar,
    ];

    /// The levels of this count, top to bottom, C7 included: the chain the
    /// auto pass solves.
    pub fn levels(self) -> Vec<Level> {
        self.names()
            .into_iter()
            .filter(|level| level.region() != Region::Cervical || *level == Level::C7)
            .filter(|level| match level {
                Level::L5 => self != Template::MissingLumbar,
                Level::L6 => self == Template::ExtraLumbar,
                Level::L7 => false,
                _ => true,
            })
            .collect()
    }

    /// Every name a vertebra on a film with this count can carry, top to
    /// bottom: [`Template::levels`], run on upward through C6 to C1 and
    /// downward through the lumbar names that follow its last, to L7.
    ///
    /// A split or merge shifts the names on one side of the edit by one
    /// step along this list, and confirming L5 shifts every name to fit.
    /// T13 is a name only under `extra-thoracic`, and T12 only where the
    /// count has one: shifting never invents a thoracic vertebra. A level
    /// outside [`Template::levels`] is always flagged
    /// [`NamingFlag::NonStandardCount`]; an edit that would need a name
    /// beyond the ends of this list is refused.
    pub fn names(self) -> Vec<Level> {
        Level::ALL
            .into_iter()
            .filter(|level| match level {
                Level::T13 => self == Template::ExtraThoracic,
                Level::T12 => self != Template::MissingThoracic,
                _ => true,
            })
            .collect()
    }
}

/// Something about a film's naming the user should look at.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum NamingFlag {
    /// The film's count is not the standard one: the chosen template is
    /// not `standard`, or a split or merge left levels outside the
    /// template (see [`Template::names`]).
    NonStandardCount,
    /// Another count scored close to the standard one.
    CloseAlternative,
    /// Counting from the top and from the bottom gave one vertebra two
    /// names.
    EndsDisagree,
    /// The first rib is unclear: named from the sacrum only.
    TopAnchorWeak,
    /// The sacrum is unclear: named from the first rib only.
    BottomAnchorWeak,
    /// Neither end is clear: the vertebrae are shown as ?1, ?2 and so on.
    NamesUnknown,
    /// The front and side films chose different counts.
    FilmsDisagreeOnCount,
}

/// How likely a count is, from the template scores. Not calibrated, and
/// shown as such.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateChance {
    pub template: Template,
    /// 0 to 1.
    pub chance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ChosenBy {
    /// The engine, from image evidence.
    Engine,
    /// The user, in the naming panel.
    User,
}

/// One film's level naming, as saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelNaming {
    /// The count the film's chain is solved with.
    pub template: Template,
    pub chosen_by: ChosenBy,
    /// Every count's chance, best first.
    pub alternatives: Vec<TemplateChance>,
    /// The name each vertebra gets counting down from the first rib: one
    /// per vertebra, in the order of `FilmMarks.vertebrae` (of the chain's
    /// bodies, in a `FilmChain`), each from [`Template::names`]. `null`
    /// when that end could not be counted. An edit that changes the chain
    /// counts again and rewrites both lists.
    pub from_top: Option<Vec<Level>>,
    /// The same, counting up from the sacrum.
    pub from_bottom: Option<Vec<Level>>,
    pub flags: Vec<NamingFlag>,
    /// The id of the vertebra the user said is L5, while it still carries
    /// that name; `null` until then, and always `null` in a `FilmChain`.
    /// Anchored to the id, not a position: a split or merge that renames
    /// or retires that vertebra clears it and raises `naming-conflict`.
    /// Until it is set, anything that depends on the level names is
    /// unconfirmed.
    pub l5_confirmed_on: Option<u32>,
}

impl LevelNaming {
    /// True when the two counts named every vertebra alike.
    pub fn counts_agreed(&self) -> bool {
        !self.flags.contains(&NamingFlag::EndsDisagree)
    }

    /// True when the user has confirmed which vertebra is L5.
    pub const fn l5_confirmed(&self) -> bool {
        self.l5_confirmed_on.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_template_has_its_count() {
        assert_eq!(Template::Standard.levels().len(), 18);
        assert_eq!(Template::ExtraThoracic.levels().len(), 19);
        assert_eq!(Template::ExtraLumbar.levels().len(), 19);
        assert_eq!(Template::MissingThoracic.levels().len(), 17);
        assert_eq!(Template::MissingLumbar.levels().len(), 17);
        assert!(Template::ExtraLumbar.levels().contains(&Level::L6));
        assert!(!Template::MissingThoracic.levels().contains(&Level::T12));
        for template in Template::ALL {
            assert_eq!(template.levels().first(), Some(&Level::C7));
        }
    }

    #[test]
    fn names_run_on_from_each_template_without_inventing_a_thoracic() {
        for template in Template::ALL {
            let names = template.names();
            let levels = template.levels();
            let start = names
                .iter()
                .position(|level| *level == Level::C7)
                .expect("every template names C7");
            assert_eq!(
                names[start..start + levels.len()],
                levels[..],
                "{template:?}"
            );
            assert_eq!(names.first(), Some(&Level::C1));
            assert_eq!(names.last(), Some(&Level::L7));
            assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
        }
        assert!(!Template::Standard.names().contains(&Level::T13));
        assert!(Template::ExtraThoracic.names().contains(&Level::T13));
        assert!(!Template::MissingThoracic.names().contains(&Level::T12));
        assert!(Template::MissingLumbar.names().contains(&Level::L5));
    }
}
