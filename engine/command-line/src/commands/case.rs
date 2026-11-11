//! `spine case pack|unpack|check`: case files on the command line, read and
//! written by the same code the app uses.

use crate::{CaseAction, CaseArgs, Outcome};
use shapes::film::FilmPng;
use shapes::reading;
use std::fs;

pub fn run(args: &CaseArgs) -> Outcome {
    match &args.action {
        CaseAction::Pack { case, films, out } => {
            let record = reading::read_case_json(&fs::read_to_string(case)?)?;
            let films = films
                .iter()
                .map(|film| {
                    Ok(FilmPng {
                        kind: film.kind,
                        png: fs::read(&film.path)?,
                    })
                })
                .collect::<Result<Vec<_>, std::io::Error>>()?;
            let bytes = case_file::pack::pack_case(&record, &films)?;
            fs::write(out, bytes)?;
        }
        CaseAction::Unpack { file, out } => {
            let opened = case_file::unpack::unpack_case(&fs::read(file)?)?;
            // Only the fixed names below reach the filesystem; paths
            // supplied by the archive are never used as destinations.
            fs::create_dir_all(out)?;
            if !opened.films.is_empty() {
                fs::create_dir_all(out.join("films"))?;
            }
            fs::write(
                out.join("case.json"),
                reading::write_case_json(&opened.case),
            )?;
            for film in opened.films {
                fs::write(out.join(film.kind.png_path()), film.png)?;
            }
            fs::write(out.join("README.txt"), case_file::layout::README_TEXT)?;
        }
        CaseAction::Check { file } => {
            let opened = case_file::unpack::unpack_case(&fs::read(file)?)?;
            let findings = case_checks::check_case(&opened.case);
            println!("{}", serde_json::to_string_pretty(&findings)?);
        }
    }
    Ok(())
}
