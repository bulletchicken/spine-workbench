//! `spine make-up <recipe> --seed N [--tier t0..t4] [--format png|dicom]
//! [--count N] [--out DIR]`: four films and their answer key per case,
//! written to a folder. The same seed gives the same films as "Try a
//! made-up spine" in the app. DICOM films are written through
//! `film_reader::test_support`.
//!
//! Each case goes in its own folder, `<recipe>-<seed>-<tier>/`, holding
//! `front.png`, `side.png`, `bend-left.png`, `bend-right.png` and
//! `answer-key.json`. Accuracy on made-up films is never clinical
//! accuracy.

use crate::{FilmFormat, MakeUpArgs, Outcome, Recipe, Tier, not_built};
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier as RecipeTier};
use std::fs;

pub fn run(args: &MakeUpArgs) -> Outcome {
    if let FilmFormat::Dicom = args.format {
        return not_built("make-up --format dicom");
    }
    for offset in 0..args.count {
        let recipe = MadeUpRecipe {
            name: recipe_name(args.recipe),
            seed: args
                .seed
                .checked_add(offset)
                .ok_or("the seeds run past 4294967295")?,
            tier: tier(args.tier),
        };
        let spine = made_up_spines::make_up_spine(&recipe)?;
        let folder = args.out.join(format!(
            "{}-{}-{}",
            text(serde_json::to_value(recipe.name)?)?,
            recipe.seed,
            text(serde_json::to_value(recipe.tier)?)?
        ));
        fs::create_dir_all(&folder)?;
        for film in &spine.films {
            fs::write(folder.join(format!("{}.png", film.kind.name())), &film.png)?;
        }
        let key = serde_json::to_string_pretty(&spine.answer_key)?;
        fs::write(folder.join("answer-key.json"), key + "\n")?;
        println!("{}", folder.display());
    }
    Ok(())
}

/// A name as JSON spells it.
fn text(value: serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    Ok(value.as_str().ok_or("a name is a string")?.to_owned())
}

fn recipe_name(recipe: Recipe) -> RecipeName {
    match recipe {
        Recipe::Random => RecipeName::Random,
        Recipe::Straight => RecipeName::Straight,
        Recipe::RightThoracic40 => RecipeName::RightThoracic40,
        Recipe::DoubleMajor55To45 => RecipeName::DoubleMajor55To45,
        Recipe::LumbarLeft30 => RecipeName::LumbarLeft30,
        Recipe::Severe90 => RecipeName::Severe90,
        Recipe::SixLumbar => RecipeName::SixLumbar,
        Recipe::ElevenRibs => RecipeName::ElevenRibs,
        Recipe::UpsideDown => RecipeName::UpsideDown,
        Recipe::MirroredFront => RecipeName::MirroredFront,
        Recipe::SideFacingLeft => RecipeName::SideFacingLeft,
        Recipe::HeavyShoulders => RecipeName::HeavyShoulders,
    }
}

fn tier(tier: Tier) -> RecipeTier {
    match tier {
        Tier::T0 => RecipeTier::T0,
        Tier::T1 => RecipeTier::T1,
        Tier::T2 => RecipeTier::T2,
        Tier::T3 => RecipeTier::T3,
        Tier::T4 => RecipeTier::T4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use made_up_spines::seeded::fnv1a64;
    use serde_json::Value;

    /// A fingerprint of a JSON value that does not depend on how it is
    /// written out: each value as a tag byte and its contents (numbers as
    /// their f64 bytes, little-endian; strings with their length), object
    /// keys in byte order. `proving-ground/parity/made-up-spines.test.ts`
    /// takes the same fingerprint of the browser build's answer key.
    fn walk(value: &Value, bytes: &mut Vec<u8>) {
        let string = |text: &str, bytes: &mut Vec<u8>| {
            bytes.push(b's');
            bytes.extend((text.len() as u32).to_le_bytes());
            bytes.extend(text.as_bytes());
        };
        match value {
            Value::Null => bytes.push(b'n'),
            Value::Bool(true) => bytes.push(b't'),
            Value::Bool(false) => bytes.push(b'f'),
            Value::Number(number) => {
                bytes.push(b'd');
                bytes.extend(number.as_f64().unwrap_or(f64::NAN).to_le_bytes());
            }
            Value::String(text) => string(text, bytes),
            Value::Array(items) => {
                bytes.push(b'[');
                for item in items {
                    walk(item, bytes);
                }
                bytes.push(b']');
            }
            Value::Object(fields) => {
                bytes.push(b'{');
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                for key in keys {
                    string(key, bytes);
                    walk(&fields[key], bytes);
                }
                bytes.push(b'}');
            }
        }
    }

    #[test]
    fn the_native_build_makes_the_spine_the_browser_build_must_match() {
        // The browser build must give exactly these fingerprints for the
        // same recipe: proving-ground/parity/made-up-spines.test.ts holds
        // the same numbers. Recompute both if the generator changes on
        // purpose.
        let recipe = MadeUpRecipe {
            name: RecipeName::Random,
            seed: 7,
            tier: RecipeTier::T4,
        };
        let spine = made_up_spines::make_up_spine(&recipe).expect("a made-up spine");
        let films: Vec<String> = spine
            .films
            .iter()
            .map(|film| format!("{:016x}", fnv1a64(&film.png)))
            .collect();
        let mut bytes = Vec::new();
        walk(
            &serde_json::to_value(&spine.answer_key).expect("a key serialises"),
            &mut bytes,
        );
        let key = format!("{:016x}", fnv1a64(&bytes));
        assert_eq!(
            films,
            [
                "792bc998dadbd857",
                "4ccfd22296e9cd69",
                "a2f25a44abfd4e99",
                "1d07a6839bde1423"
            ]
        );
        assert_eq!(key, "912e2bc926a677d1");
    }
}
