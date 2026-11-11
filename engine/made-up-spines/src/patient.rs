//! The made-up patient: one recipe's spine standing, and bent to the left
//! and to the right for the bending films, drawn again until it stacks.
//!
//! The standing spine is balanced over the sacrum. From front to back the
//! lordosis does it, as in a standing person: it is solved, by bisection
//! between 15° and 90°, so that C7 sits as far in front of the back corner
//! of the sacrum as the recipe asked, and the recipe records the lordosis
//! found. From side to side the whole body turns a little, at most 4°
//! (see [`crate::stack_from_l5`]). The bent spines keep the standing lordosis
//! and are not balanced, because a bending patient leans. Bending toward a curve's convexity
//! straightens it by its flexibility, bending away grows it a little, and
//! the trunk leans toward the bend: an extra tilt that is zero at the last
//! lumbar vertebra and full ten vertebrae up. The pelvis tips toward the
//! bend too. Sizes and the sagittal profile are shared by all three.
//!
//! The ranges behind the bending model are made up, guided by the spread
//! of curve flexibility on bending films reported for adolescent curves
//! (main thoracic curves straighten less than lumbar ones).

use crate::body_sizes::{VertebraSize, base_sizes, jittered};
use crate::curves::{CoronalProfile, Curve, SagittalProfile};
use crate::recipes::{Bend, SpineRecipe, counts, spine_recipe};
use crate::seeded::{Stream, attempt_seed};
use crate::stack_from_l5::{StackFailure, StackPlan, StackedSpine, check_discs, stack};
use shapes::levels::Level;
use shapes::made_up::RecipeName;
use shapes::naming::Template;

/// How many times a recipe is drawn before it is given up as
/// unsatisfiable.
pub const MOST_ATTEMPTS: u32 = 20;

/// The range the lordosis is solved in, degrees.
const LORDOSIS_RANGE: (f64, f64) = (15.0, 90.0);

/// The cervical vertebrae painted above C7, which carry no marks.
const NECK: [Level; 5] = [Level::C2, Level::C3, Level::C4, Level::C5, Level::C6];

/// One made-up patient.
#[derive(Clone, Debug, PartialEq)]
pub struct Patient {
    pub recipe: SpineRecipe,
    pub standing: StackedSpine,
    pub bent_left: StackedSpine,
    pub bent_right: StackedSpine,
    /// The seed of the attempt that stacked.
    pub seed: u64,
    /// How many attempts failed before it.
    pub retries: u32,
}

/// No attempt at a recipe stacked.
#[derive(Clone, Debug, PartialEq)]
pub struct Unsatisfiable {
    pub attempts: u32,
    pub last: StackFailure,
}

/// The levels painted for a count: C2 to C6, then the count's own levels
/// from C7 to the last lumbar vertebra.
pub fn painted_levels(template: Template) -> Vec<Level> {
    NECK.iter().copied().chain(template.levels()).collect()
}

/// The patient for a named recipe and seed: each attempt draws the recipe
/// afresh from its attempt's seed, until one stacks.
pub fn make_up_patient(name: RecipeName, seed: u32) -> Result<Patient, Unsatisfiable> {
    attempt_until_stacked(seed, |attempt| spine_recipe(name, attempt))
}

/// The first of up to [`MOST_ATTEMPTS`] attempts whose recipe stacks.
pub fn attempt_until_stacked(
    seed: u32,
    recipe_for: impl Fn(u64) -> SpineRecipe,
) -> Result<Patient, Unsatisfiable> {
    let mut last = StackFailure::NothingToStack;
    for attempt in 0..MOST_ATTEMPTS {
        let attempt_seed = attempt_seed(seed, attempt);
        match patient_from(recipe_for(attempt_seed), attempt_seed) {
            Ok(mut patient) => {
                patient.retries = attempt;
                return Ok(patient);
            }
            Err(failure) => last = failure,
        }
    }
    Err(Unsatisfiable {
        attempts: MOST_ATTEMPTS,
        last,
    })
}

/// The patient built from one recipe, or why it does not stack.
pub fn patient_from(mut recipe: SpineRecipe, seed: u64) -> Result<Patient, StackFailure> {
    let levels = painted_levels(recipe.template);
    let base = base_sizes(
        &levels,
        recipe.length_mm / crate::body_sizes::TABLE_LENGTH_MM,
    );
    let sizes = if recipe.jitter_sizes {
        jittered(&base, &mut Stream::new("sizes", seed))
    } else {
        base
    };
    recipe.sagittal.lordosis = balancing_lordosis(&recipe, &levels, &sizes)?;
    let standing = stand(&recipe, &levels, &sizes, &recipe.curves, None)?;
    check_discs(&standing)?;
    let bent_left = stand(
        &recipe,
        &levels,
        &sizes,
        &bent_curves(&recipe.curves, &recipe.bend_left, false),
        Some((&recipe.bend_left, false)),
    )?;
    check_discs(&bent_left)?;
    let bent_right = stand(
        &recipe,
        &levels,
        &sizes,
        &bent_curves(&recipe.curves, &recipe.bend_right, true),
        Some((&recipe.bend_right, true)),
    )?;
    check_discs(&bent_right)?;
    Ok(Patient {
        recipe,
        standing,
        bent_left,
        bent_right,
        seed,
        retries: 0,
    })
}

/// The curves as bent toward the patient's right (`toward_right`) or
/// left: a curve convex toward the bend loses its flexibility's share of
/// its angle, one convex away gains its growth share.
pub fn bent_curves(curves: &[Curve], bend: &Bend, toward_right: bool) -> Vec<Curve> {
    curves
        .iter()
        .enumerate()
        .map(|(index, curve)| {
            let toward_convexity = curve.convex_right == toward_right;
            let factor = if toward_convexity {
                1.0 - bend.flexibility.get(index).copied().unwrap_or(0.0)
            } else {
                1.0 + bend.growth.get(index).copied().unwrap_or(0.0)
            };
            Curve {
                cobb_deg: curve.cobb_deg * factor,
                ..*curve
            }
        })
        .collect()
}

/// The lordosis that puts C7 the recipe's distance in front of the back
/// corner of the sacrum. More lordosis turns everything above the lumbar
/// spine further back, so the distance falls as the lordosis grows.
fn balancing_lordosis(
    recipe: &SpineRecipe,
    levels: &[Level],
    sizes: &[VertebraSize],
) -> Result<f64, StackFailure> {
    let ahead = |lordosis: f64| -> Result<f64, StackFailure> {
        let mut trial = recipe.clone();
        trial.sagittal.lordosis = lordosis;
        let spine = stand(&trial, levels, sizes, &trial.curves, None)?;
        let [forward, _] = spine.c7_offset().ok_or(StackFailure::NothingToStack)?;
        Ok(forward - recipe.sagittal_balance_mm)
    };
    let (mut low, mut high) = LORDOSIS_RANGE;
    let (at_low, at_high) = (ahead(low)?, ahead(high)?);
    if at_low < 0.0 || at_high > 0.0 {
        return Err(StackFailure::OutOfBalance(if at_low < 0.0 {
            low
        } else {
            high
        }));
    }
    for _ in 0..40 {
        let middle = 0.5 * (low + high);
        if ahead(middle)? > 0.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(0.5 * (low + high))
}

/// One spine of the patient: standing, or bent with the given bend.
fn stand(
    recipe: &SpineRecipe,
    levels: &[Level],
    sizes: &[VertebraSize],
    curves: &[Curve],
    bend: Option<(&Bend, bool)>,
) -> Result<StackedSpine, StackFailure> {
    let (thoracic, lumbar) = counts(recipe.template);
    let sacrum = (thoracic + lumbar) as f64;
    let coronal = CoronalProfile::new(recipe.generator, curves, sacrum)
        .ok_or(StackFailure::NothingToStack)?;
    let sagittal = SagittalProfile::new(&recipe.sagittal, thoracic, lumbar);
    let side = |toward_right: bool| if toward_right { 1.0 } else { -1.0 };
    let (trunk_lean, pelvic_tilt) = bend.map_or((0.0, 0.0), |(bend, toward_right)| {
        (
            side(toward_right) * bend.trunk_lean_deg.to_radians(),
            side(toward_right) * bend.pelvic_tilt_deg,
        )
    });
    stack(&StackPlan {
        levels,
        sizes,
        coronal: &coronal,
        sagittal: &sagittal,
        share: recipe.vertebral_share,
        trunk_lean,
        pelvic_incidence_deg: recipe.pelvic_incidence_deg,
        pelvic_obliquity_deg: recipe.pelvic_obliquity_deg + pelvic_tilt,
        length_mm: recipe.length_mm,
        coronal_balance: bend.is_none().then_some(recipe.coronal_balance_mm),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::spine_recipe;
    use shapes::made_up::CurveRegion;

    #[test]
    fn every_named_recipe_stacks_for_many_seeds() {
        let names = [
            RecipeName::Random,
            RecipeName::Straight,
            RecipeName::RightThoracic40,
            RecipeName::DoubleMajor55To45,
            RecipeName::LumbarLeft30,
            RecipeName::Severe90,
            RecipeName::SixLumbar,
            RecipeName::ElevenRibs,
            RecipeName::UpsideDown,
            RecipeName::MirroredFront,
            RecipeName::SideFacingLeft,
            RecipeName::HeavyShoulders,
        ];
        for name in names {
            for seed in 0..12 {
                let patient = make_up_patient(name, seed)
                    .unwrap_or_else(|failure| panic!("{name:?} seed {seed}: {failure:?}"));
                assert!(patient.retries < MOST_ATTEMPTS);
                assert_eq!(
                    patient.standing.levels,
                    painted_levels(patient.recipe.template)
                );
            }
        }
    }

    #[test]
    fn the_standing_spine_stands_where_the_recipe_asked() {
        for seed in 0..10 {
            let patient = make_up_patient(RecipeName::Random, seed).expect("stacks");
            let [forward, left] = patient.standing.c7_offset().expect("a C7");
            assert!((forward - patient.recipe.sagittal_balance_mm).abs() < 1e-6);
            // From side to side the body turns at most 4°; past that the
            // trunk stays shifted.
            if patient.standing.balance_deg.abs() < 4.0 - 1e-9 {
                assert!((left - patient.recipe.coronal_balance_mm).abs() < 1e-6);
            }
            let lordosis = patient.recipe.sagittal.lordosis;
            assert!((LORDOSIS_RANGE.0..=LORDOSIS_RANGE.1).contains(&lordosis));
        }
    }

    #[test]
    fn bending_toward_a_curve_straightens_it_and_away_grows_it() {
        let curve = Curve {
            region: CurveRegion::MainThoracic,
            upper_end: 4.0,
            lower_end: 11.0,
            apex: 7.5,
            cobb_deg: 40.0,
            convex_right: true,
            twist_per_cobb: 0.3,
        };
        let bend = Bend {
            flexibility: vec![0.4],
            growth: vec![0.2],
            trunk_lean_deg: 15.0,
            pelvic_tilt_deg: 3.0,
        };
        assert!((bent_curves(&[curve], &bend, true)[0].cobb_deg - 24.0).abs() < 1e-12);
        assert!((bent_curves(&[curve], &bend, false)[0].cobb_deg - 48.0).abs() < 1e-12);
    }

    #[test]
    fn a_bent_spine_leans_toward_its_bend() {
        let patient = make_up_patient(RecipeName::Straight, 4).expect("a straight spine stacks");
        let t1 = |spine: &StackedSpine| spine.vertebra(Level::T1).expect("T1").centre.y;
        // Patient left is +y.
        assert!(t1(&patient.bent_left) > t1(&patient.standing) + 30.0);
        assert!(t1(&patient.bent_right) < t1(&patient.standing) - 30.0);
    }

    #[test]
    fn a_recipe_that_never_stacks_is_given_up_after_twenty_attempts() {
        let impossible = |seed: u64| {
            let mut recipe = spine_recipe(RecipeName::Straight, seed);
            recipe.curves = vec![Curve {
                region: CurveRegion::MainThoracic,
                upper_end: 6.0,
                lower_end: 8.0,
                apex: 7.0,
                cobb_deg: 170.0,
                convex_right: true,
                twist_per_cobb: 0.0,
            }];
            recipe
        };
        let failure = attempt_until_stacked(1, impossible).expect_err("never stacks");
        assert_eq!(failure.attempts, MOST_ATTEMPTS);
        assert!(matches!(failure.last, StackFailure::DiscCloses(_)));
    }
}
