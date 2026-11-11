//! Version 1 of the case file.

use shapes::case_file_v1::CaseFileV1;
use shapes::errors::EngineError;

/// Keep validation and canonical spelling in the frozen shapes reader.
pub fn write(case: &CaseFileV1) -> Result<String, EngineError> {
    shapes::reading::check_case_file(case)?;
    Ok(shapes::reading::write_case_json(case))
}

pub fn read(text: &str) -> Result<CaseFileV1, EngineError> {
    shapes::reading::read_case_json(text)
}
