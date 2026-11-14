//! `spine frames [--check]`: records the watch frames of a real auto pass
//! on the front film of the right-thoracic-40 made-up spine into
//! `proving-ground/known-answers/recorded-frames/`, where the app's
//! stand-in engine replays them. `--check` fails on any drift.

use crate::{FramesArgs, Outcome};
use auto_pass::watch::{ChainPass, FilmPass};
use shapes::auto_pass::{
    Acquisition, AutoPassInput, AutoPassResult, ChainOptions, FrontFinder, Population,
};
use shapes::film::FilmKind;
use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};
use std::fs;
use std::path::Path;
use std::rc::Rc;

pub fn run(args: &FramesArgs) -> Outcome {
    let recipe = MadeUpRecipe {
        name: RecipeName::RightThoracic40,
        seed: 1,
        tier: Tier::T0,
    };
    let spine = made_up_spines::make_up_spine(&recipe)?;
    let png = spine
        .films
        .iter()
        .find(|film| film.kind == FilmKind::Front)
        .ok_or("the made-up spine has no front film")?;
    let truth = spine
        .answer_key
        .films
        .iter()
        .find(|film| film.kind == FilmKind::Front)
        .ok_or("the made-up spine has no front geometry")?;
    let decoded = Rc::new(film_reader::read_film(&png.png, FilmKind::Front)?);
    let film = super::pass::record(&decoded, truth.geometry.clone());
    let input = AutoPassInput {
        case_version: 0,
        films: vec![film.clone()],
        options: ChainOptions {
            acquisition: Acquisition::Unknown,
            population: Population::Adolescent,
            template: None,
        },
        first_vertebra_id: 1,
        front_finder: FrontFinder::HandBuilt,
        side_finder: shapes::SideFinder::HandBuilt,
    };
    let mut frames = Vec::new();
    let mut film_pass = FilmPass::new(Rc::clone(&decoded), film)?;
    loop {
        let step = film_pass.advance(1)?;
        frames.extend(step.frames);
        if step.done {
            break;
        }
    }
    let mut film_result = film_pass.result()?;
    let cache = film_pass.take_cache()?;
    let mut chain_pass = ChainPass::new(vec![film_result.clone()], input.options.clone())?;
    loop {
        let step = chain_pass.advance(1)?;
        frames.extend(step.frames);
        if step.done {
            break;
        }
    }
    let chain = chain_pass.result()?;
    let film_chain = chain
        .films
        .first()
        .ok_or("the chain pass returned no front film")?;
    let marks = auto_pass::run::place_points(&cache, film_chain, input.first_vertebra_id)?;
    let mut warnings = film_result.warnings.clone();
    for warning in &film_chain.warnings {
        if !warnings.contains(warning) {
            warnings.push(warning.clone());
        }
    }
    let result = AutoPassResult {
        case_version: input.case_version,
        marks: vec![marks],
        two_films: chain.two_films.clone(),
        warnings,
    };
    let direct = auto_pass::run::auto_pass(&input, std::slice::from_ref(decoded.as_ref()))?;
    if result != direct {
        return Err("the recorded pass differs from the one-call native answer".into());
    }
    // Keep the FFT-produced baton separately, so parity can run the chain
    // on identical bytes and require exact results from libm and arithmetic.
    let strip = std::mem::take(&mut film_result.strip);
    let mut fixture = serde_json::json!({
        "recipe": recipe,
        "input": input,
        "result": result,
        "frames": frames,
        "filmResult": film_result,
        "chainAnswer": chain,
    });
    let mut payloads = Vec::new();
    pack_payloads(&mut fixture, &mut payloads);
    let mut json = serde_json::to_vec(&fixture)?;
    json.push(b'\n');
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../proving-ground/known-answers/recorded-frames");
    if !args.check {
        fs::create_dir_all(&folder)?;
    }
    for (name, bytes) in [
        ("front.strip", strip),
        ("frame-payloads.bin", payloads),
        ("frames.json", json),
    ] {
        let path = folder.join(name);
        if args.check {
            if fs::read(&path).map_err(|_| "a recorded frame fixture is missing")? != bytes {
                return Err(
                    format!("recorded {name} differs; run spine frames to update it").into(),
                );
            }
        } else {
            fs::write(&path, bytes)?;
        }
    }
    println!(
        "{} recorded front-film frames and its native strip.",
        if args.check { "Checked" } else { "Wrote" }
    );
    Ok(())
}

/// Byte previews have long empty borders. Store either literal bytes or
/// (count, byte) runs, whichever is smaller; the frame shapes stay unchanged
/// after the fixture reader restores each field.
fn pack_payloads(value: &mut serde_json::Value, payloads: &mut Vec<u8>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                pack_payloads(value, payloads);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                let is_bytes = matches!(
                    key.as_str(),
                    "data" | "image" | "valid" | "values" | "highPass" | "heat" | "chain"
                ) && value.is_array();
                if !is_bytes {
                    pack_payloads(value, payloads);
                    continue;
                }
                let bytes: Vec<u8> = value
                    .as_array()
                    .expect("a byte array")
                    .iter()
                    .map(|value| {
                        u8::try_from(value.as_u64().expect("an unsigned byte"))
                            .expect("a byte fits u8")
                    })
                    .collect();
                let mut runs = Vec::new();
                let mut index = 0;
                while index < bytes.len() {
                    let byte = bytes[index];
                    let mut count = 1;
                    while count < 255 && index + count < bytes.len() && bytes[index + count] == byte
                    {
                        count += 1;
                    }
                    runs.extend([count as u8, byte]);
                    index += count;
                }
                let (encoded, encoding) = if runs.len() < bytes.len() {
                    (&runs, "runs")
                } else {
                    (&bytes, "raw")
                };
                *value = serde_json::json!({
                    "byteOffset": payloads.len(),
                    "byteLength": encoded.len(),
                    "length": bytes.len(),
                    "encoding": encoding,
                });
                payloads.extend(encoded);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_previews_keep_known_runs_and_literal_bytes() {
        let mut fixture = serde_json::json!({
            "tile": {"data": vec![0; 300]},
            "heat": [1, 2, 3, 4],
            "points": [[10, 20]],
        });
        let mut payloads = Vec::new();
        pack_payloads(&mut fixture, &mut payloads);
        // Object keys are ordered: heat's four literals, then 300 zeroes
        // split at the format's 255-cell run limit.
        assert_eq!(payloads, [1, 2, 3, 4, 255, 0, 45, 0]);
        assert_eq!(fixture["heat"]["encoding"], "raw");
        assert_eq!(fixture["tile"]["data"]["byteOffset"], 4);
        assert_eq!(fixture["tile"]["data"]["length"], 300);
        assert_eq!(fixture["points"], serde_json::json!([[10, 20]]));
    }
}
