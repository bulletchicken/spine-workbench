//! Strict reading and canonical writing of `case.json`.
//!
//! Reading happens in four steps, each with its own error code:
//!
//! 1. The text must be JSON, with no key written twice in one object
//!    (`case-invalid-value` otherwise). A repeated key would give a value
//!    two spellings, and which one wins would depend on the reader.
//! 2. `format` must be `"spine-case"` (`case-wrong-format`), and `version`
//!    must be 1; a higher version is `case-newer-version`.
//! 3. The value must match [`CaseFileV1`] exactly: an unknown field is
//!    `case-unknown-field`, a missing field or a wrong type
//!    `case-invalid-value`. Every field must be written out, `null`
//!    included.
//! 4. [`check_case_file`] checks the values themselves
//!    (`case-invalid-value`).
//!
//! Every error carries the JSON path of the offending value in `detail`.
//! [`check_case_file`] is also what validates a case that arrives from the
//! app as an object rather than as text.
//!
//! Writing is canonical: fields in declaration order, two-space indents,
//! one trailing newline. A file this module wrote reads back and writes out
//! byte for byte the same.

use crate::case_file_v1::CaseFileV1;
use crate::confidence::Confidence;
use crate::errors::{EngineError, ErrorCode};
use crate::film::{
    Film, FilmKind, KeptTags, MAX_FILM_SIDE, ModalityMap, PixelValues, Shutter, SourceFormat, View,
};
use crate::film_geometry::{FilmGeometry, ScaleSource};
use crate::landmarks::{C7_TOLERANCE_PX, HipSide, Landmarks, SacrumTop};
use crate::levels::Level;
use crate::marks::{Mark, MarkSource, PedicleMark};
use crate::naming::{LevelNaming, NamingFlag};
use crate::pixels::{Px, PxRect, is_near_film};
use crate::vertebra::{FilmMarks, VertebraMarks, VertebraOnFilm};
use crate::{CASE_FORMAT, CASE_VERSION};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

type Checked = Result<(), EngineError>;

/// The smallest and largest scale accepted, in mm per pixel.
pub const MM_PER_PX_RANGE: (f64, f64) = (0.01, 2.0);

/// Reads `case.json` strictly.
pub fn read_case_json(text: &str) -> Result<CaseFileV1, EngineError> {
    let value = parse_json(text)?;
    check_format_and_version(&value)?;

    let case: CaseFileV1 = serde_path_to_error::deserialize(&value).map_err(|error| {
        let path = error.path().to_string();
        let message = error.inner().to_string();
        let code = if message.starts_with("unknown field") {
            ErrorCode::CaseUnknownField
        } else {
            ErrorCode::CaseInvalidValue
        };
        EngineError::new(code, message).with_detail(path)
    })?;

    let canonical = serde_json::to_value(&case)
        .map_err(|error| EngineError::new(ErrorCode::CaseInvalidValue, error.to_string()))?;
    let mut path = String::new();
    if let Some(missing) = first_missing_field(&canonical, &value, &mut path) {
        return Err(EngineError::new(
            ErrorCode::CaseInvalidValue,
            "a field is missing; every field must be written out, null included",
        )
        .with_detail(missing));
    }

    check_case_file(&case)?;
    Ok(case)
}

/// Parses JSON text, refusing any object that has a key twice. The error
/// names the repeated key's path.
fn parse_json(text: &str) -> Result<Value, EngineError> {
    let repeated = RefCell::new(None);
    let seed = NoRepeatedKeys {
        path: String::new(),
        repeated: &repeated,
    };
    let mut reader = serde_json::Deserializer::from_str(text);
    let parsed = seed.deserialize(&mut reader).and_then(|value| {
        reader.end()?;
        Ok(value)
    });
    parsed.map_err(|error| match repeated.take() {
        Some(path) => invalid(&path, "a key appears twice in one object"),
        None => EngineError::new(ErrorCode::CaseInvalidValue, "case.json is not valid JSON")
            .with_detail(format!("line {} column {}", error.line(), error.column())),
    })
}

/// Reads one JSON value at `path` the way `serde_json::Value` does, but
/// stops at the first object key seen twice and records its path.
struct NoRepeatedKeys<'a> {
    path: String,
    repeated: &'a RefCell<Option<String>>,
}

impl NoRepeatedKeys<'_> {
    fn child(&self, step: String) -> Self {
        NoRepeatedKeys {
            path: step,
            repeated: self.repeated,
        }
    }
}

impl<'de> DeserializeSeed<'de> for NoRepeatedKeys<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for NoRepeatedKeys<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) =
            items.next_element_seed(self.child(format!("{}[{}]", self.path, values.len())))?
        {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = entries.next_key::<String>()? {
            let path = if self.path.is_empty() {
                key.clone()
            } else {
                format!("{}.{key}", self.path)
            };
            if object.contains_key(&key) {
                *self.repeated.borrow_mut() = Some(path);
                return Err(de::Error::custom("a key appears twice in one object"));
            }
            let value = entries.next_value_seed(self.child(path))?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

/// Writes `case.json` in its one canonical form.
pub fn write_case_json(case: &CaseFileV1) -> String {
    let mut text = serde_json::to_string_pretty(case)
        .expect("every case shape serialises: string keys, no maps with other keys");
    text.push('\n');
    text
}

fn check_format_and_version(value: &Value) -> Checked {
    let wrong_format =
        |problem: &str| EngineError::new(ErrorCode::CaseWrongFormat, problem).with_detail("format");
    let object = value
        .as_object()
        .ok_or_else(|| wrong_format("case.json is not an object"))?;
    match object.get("format") {
        Some(Value::String(format)) if format == CASE_FORMAT => {}
        Some(_) => return Err(wrong_format("format is not \"spine-case\"")),
        None => return Err(wrong_format("format is missing")),
    }
    let version = object.get("version").ok_or_else(|| {
        EngineError::new(ErrorCode::CaseWrongFormat, "version is missing").with_detail("version")
    })?;
    match version.as_u64() {
        Some(v) if v == u64::from(CASE_VERSION) => Ok(()),
        Some(v) if v > u64::from(CASE_VERSION) => Err(EngineError::new(
            ErrorCode::CaseNewerVersion,
            format!("made by a newer version (case file version {v})"),
        )
        .with_detail("version")),
        _ => Err(invalid("version", "version must be 1")),
    }
}

/// The path of the first field present in `canonical` but absent from
/// `input`, if any.
fn first_missing_field(canonical: &Value, input: &Value, path: &mut String) -> Option<String> {
    match (canonical, input) {
        (Value::Object(want), Value::Object(have)) => {
            for (key, want_value) in want {
                let before = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                let found = match have.get(key) {
                    None => Some(path.clone()),
                    Some(have_value) => first_missing_field(want_value, have_value, path),
                };
                path.truncate(before);
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        (Value::Array(want), Value::Array(have)) => {
            for (index, (want_value, have_value)) in want.iter().zip(have).enumerate() {
                let before = path.len();
                path.push_str(&format!("[{index}]"));
                let found = first_missing_field(want_value, have_value, path);
                path.truncate(before);
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        _ => None,
    }
}

fn invalid(path: &str, problem: impl Into<String>) -> EngineError {
    EngineError::new(ErrorCode::CaseInvalidValue, problem).with_detail(path)
}

fn ensure(ok: bool, path: &str, problem: &str) -> Checked {
    if ok {
        Ok(())
    } else {
        Err(invalid(path, problem))
    }
}

fn finite(value: f64, path: &str) -> Checked {
    ensure(value.is_finite(), path, "not a finite number")
}

fn positive(value: f64, path: &str) -> Checked {
    ensure(
        value.is_finite() && value > 0.0,
        path,
        "must be a finite number above zero",
    )
}

fn within(value: f64, (low, high): (f64, f64), path: &str) -> Checked {
    ensure(
        value.is_finite() && value >= low && value <= high,
        path,
        &format!("must be between {low} and {high}"),
    )
}

fn unique<T: Ord + Copy>(items: &[T], path: &str) -> Checked {
    let mut seen = BTreeSet::new();
    ensure(
        items.iter().all(|item| seen.insert(*item)),
        path,
        "a value appears twice",
    )
}

fn plain_text(text: &str, max_chars: usize, path: &str) -> Checked {
    ensure(
        text.chars().count() <= max_chars && !text.chars().any(char::is_control),
        path,
        &format!("must be at most {max_chars} characters, with no control characters"),
    )
}

/// Checks a case's values: everything [`read_case_json`] checks after the
/// shape itself.
pub fn check_case_file(case: &CaseFileV1) -> Checked {
    if case.format != CASE_FORMAT {
        return Err(
            EngineError::new(ErrorCode::CaseWrongFormat, "format is not \"spine-case\"")
                .with_detail("format"),
        );
    }
    if case.version > CASE_VERSION {
        return Err(EngineError::new(
            ErrorCode::CaseNewerVersion,
            format!(
                "made by a newer version (case file version {})",
                case.version
            ),
        )
        .with_detail("version"));
    }
    ensure(case.version == CASE_VERSION, "version", "version must be 1")?;

    let app = &case.saved_with.app;
    ensure(
        !app.is_empty()
            && app.len() <= 32
            && app
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-')),
        "savedWith.app",
        "must be a version: letters, digits, '.', '+' and '-', at most 32 characters",
    )?;

    let mut kinds = BTreeSet::new();
    for (index, film) in case.films.iter().enumerate() {
        let path = format!("films[{index}]");
        ensure(
            kinds.insert(film.kind),
            &format!("{path}.kind"),
            "a film kind appears twice",
        )?;
        check_film(film, &path)?;
    }

    let mut ids = BTreeSet::new();
    let mut marked = BTreeSet::new();
    for (index, marks) in case.marks.iter().enumerate() {
        let path = format!("marks[{index}]");
        let film = case
            .films
            .iter()
            .find(|film| film.kind == marks.film)
            .ok_or_else(|| {
                invalid(
                    &format!("{path}.film"),
                    "marks for a film the case does not have",
                )
            })?;
        ensure(
            marked.insert(marks.film),
            &format!("{path}.film"),
            "two sets of marks for one film",
        )?;
        check_film_marks(marks, film, &path, &mut ids)?;
    }
    if let Some(&highest) = ids.last() {
        ensure(
            case.next_vertebra_id > highest,
            "nextVertebraId",
            "must be above every vertebra id, so ids are never reused",
        )?;
    }
    Ok(())
}

fn check_film(film: &Film, path: &str) -> Checked {
    let image = &film.image;
    ensure(
        image.path == film.kind.png_path(),
        &format!("{path}.image.path"),
        "must be the film kind's own path, such as films/front.png",
    )?;
    ensure(
        (1..=MAX_FILM_SIDE).contains(&image.width),
        &format!("{path}.image.width"),
        "must be between 1 and 16384",
    )?;
    ensure(
        (1..=MAX_FILM_SIDE).contains(&image.height),
        &format!("{path}.image.height"),
        "must be between 1 and 16384",
    )?;
    ensure(
        (1..=16).contains(&image.bits_stored),
        &format!("{path}.image.bitsStored"),
        "must be between 1 and 16",
    )?;
    match (&film.kept_tags, film.source) {
        (Some(tags), SourceFormat::Dicom) => check_kept_tags(tags, &format!("{path}.keptTags"))?,
        (None, SourceFormat::Dicom) => {
            return Err(invalid(
                &format!("{path}.keptTags"),
                "a DICOM film always has kept tags",
            ));
        }
        (Some(_), _) => {
            return Err(invalid(
                &format!("{path}.keptTags"),
                "only a DICOM film has kept tags",
            ));
        }
        (None, _) => {}
    }
    check_values(
        &film.values,
        film.source,
        image.bits_stored,
        &format!("{path}.values"),
    )?;
    unique(&film.flags, &format!("{path}.flags"))?;
    for (index, rect) in film.blanked.iter().enumerate() {
        check_rect(
            rect,
            image.width,
            image.height,
            &format!("{path}.blanked[{index}]"),
        )?;
    }
    check_geometry(&film.geometry, film, &format!("{path}.geometry"))
}

fn check_rect(rect: &PxRect, width: u32, height: u32, path: &str) -> Checked {
    ensure(
        rect.is_on_film(width, height),
        path,
        "must be a box with left <= right and top <= bottom, inside the film's outer edges",
    )
}

fn check_kept_tags(tags: &KeptTags, path: &str) -> Checked {
    let field = |name: &str| format!("{path}.{name}");
    if let Some(class) = &tags.sop_class {
        ensure(
            !class.is_empty()
                && class.len() <= 64
                && class.chars().all(|c| c.is_ascii_digit() || c == '.'),
            &field("sopClass"),
            "must be a standard class identifier: digits and dots",
        )?;
    }
    if let Some(modality) = &tags.modality {
        ensure(
            !modality.is_empty()
                && modality.len() <= 16
                && modality
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            &field("modality"),
            "must be a short code such as DX",
        )?;
    }
    if let Some(manufacturer) = &tags.manufacturer {
        plain_text(manufacturer, 64, &field("manufacturer"))?;
    }
    if let Some(model) = &tags.model_name {
        plain_text(model, 64, &field("modelName"))?;
    }
    if let Some(bits) = tags.bits_stored {
        ensure(
            (1..=16).contains(&bits),
            &field("bitsStored"),
            "must be between 1 and 16",
        )?;
    }
    for (name, spacing) in [
        ("pixelSpacing", tags.pixel_spacing),
        ("imagerPixelSpacing", tags.imager_pixel_spacing),
        (
            "nominalScannedPixelSpacing",
            tags.nominal_scanned_pixel_spacing,
        ),
    ] {
        if let Some([row, column]) = spacing {
            positive(row, &format!("{}[0]", field(name)))?;
            positive(column, &format!("{}[1]", field(name)))?;
        }
    }
    for (name, value) in [
        ("estimatedMagnification", tags.estimated_magnification),
        ("distanceSourceToDetector", tags.distance_source_to_detector),
        ("distanceSourceToPatient", tags.distance_source_to_patient),
        ("windowWidth", tags.window_width),
        ("lossyRatio", tags.lossy_ratio),
    ] {
        if let Some(value) = value {
            positive(value, &field(name))?;
        }
    }
    for (name, value) in [
        ("rescaleSlope", tags.rescale_slope),
        ("rescaleIntercept", tags.rescale_intercept),
        ("windowCentre", tags.window_centre),
    ] {
        if let Some(value) = value {
            finite(value, &field(name))?;
        }
    }
    if let Some([vertical, horizontal]) = tags.pixel_aspect_ratio {
        ensure(
            vertical >= 1 && horizontal >= 1,
            &field("pixelAspectRatio"),
            "both parts must be at least 1",
        )?;
    }
    if let Some(directions) = &tags.patient_orientation {
        for (index, direction) in directions.iter().enumerate() {
            ensure(
                (1..=3).contains(&direction.len())
                    && direction.chars().all(|c| "APLRHF".contains(c)),
                &format!("{}[{index}]", field("patientOrientation")),
                "must be one to three of the letters A, P, L, R, H and F",
            )?;
        }
    }
    if let Some(shutters) = &tags.shutters {
        for (index, shutter) in shutters.iter().enumerate() {
            let shutter_path = format!("{}[{index}]", field("shutters"));
            match shutter {
                Shutter::Rectangle {
                    left,
                    right,
                    upper,
                    lower,
                } => ensure(
                    left <= right && upper <= lower,
                    &shutter_path,
                    "edges must not cross",
                )?,
                Shutter::Circle {
                    centre_row,
                    centre_column,
                    radius,
                } => {
                    finite(*centre_row, &format!("{shutter_path}.centreRow"))?;
                    finite(*centre_column, &format!("{shutter_path}.centreColumn"))?;
                    positive(*radius, &format!("{shutter_path}.radius"))?;
                }
                Shutter::Polygon { vertices } => {
                    ensure(
                        vertices.len() >= 3,
                        &format!("{shutter_path}.vertices"),
                        "a polygon needs at least three vertices",
                    )?;
                    for (v, [row, column]) in vertices.iter().enumerate() {
                        finite(*row, &format!("{shutter_path}.vertices[{v}][0]"))?;
                        finite(*column, &format!("{shutter_path}.vertices[{v}][1]"))?;
                    }
                }
            }
        }
    }
    if let Some(frames) = tags.number_of_frames {
        ensure(frames >= 1, &field("numberOfFrames"), "must be at least 1")?;
    }
    Ok(())
}

fn check_values(values: &PixelValues, source: SourceFormat, bits: u8, path: &str) -> Checked {
    let signed_offset = -(1i32 << (bits.clamp(1, 16) - 1));
    ensure(
        values.offset == 0 || values.offset == signed_offset,
        &format!("{path}.offset"),
        "must be 0, or -2^(bits - 1) for signed data",
    )?;
    if source != SourceFormat::Dicom {
        ensure(
            values.offset == 0 && values.modality == PixelValues::AS_STORED.modality,
            path,
            "a PNG or JPEG film's values are as stored",
        )?;
    }
    match &values.modality {
        ModalityMap::Linear { slope, intercept } => {
            ensure(
                slope.is_finite() && *slope != 0.0,
                &format!("{path}.modality.slope"),
                "must be a finite number other than zero",
            )?;
            finite(*intercept, &format!("{path}.modality.intercept"))
        }
        ModalityMap::Lookup {
            bits: table_bits,
            table,
            ..
        } => {
            ensure(
                (1..=16).contains(table_bits),
                &format!("{path}.modality.bits"),
                "must be between 1 and 16",
            )?;
            ensure(
                !table.is_empty() && table.len() <= 65_536,
                &format!("{path}.modality.table"),
                "must hold 1 to 65536 entries",
            )?;
            let limit = 1u32 << table_bits;
            ensure(
                table.iter().all(|&entry| u32::from(entry) < limit),
                &format!("{path}.modality.table"),
                "an entry does not fit in the table's bits",
            )
        }
    }
}

fn check_geometry(geometry: &FilmGeometry, film: &Film, path: &str) -> Checked {
    let (width, height) = (film.image.width, film.image.height);
    let field = |name: &str| format!("{path}.{name}");
    if let Some(mm) = geometry.mm_per_px {
        within(mm, MM_PER_PX_RANGE, &field("mmPerPx"))?;
    }
    if let Some(mm) = geometry.mm_per_px_vertical {
        within(mm, MM_PER_PX_RANGE, &field("mmPerPxVertical"))?;
        ensure(
            geometry.mm_per_px.is_some(),
            &field("mmPerPxVertical"),
            "a vertical scale needs a horizontal one",
        )?;
    }
    ensure(
        geometry.scale.is_some() == geometry.mm_per_px.is_some(),
        &field("scale"),
        "a scale and its source go together",
    )?;
    match &geometry.scale {
        None => {}
        Some(ScaleSource::File { .. }) => ensure(
            film.kept_tags.is_some(),
            &field("scale"),
            "a scale from the file needs the file's kept tags",
        )?,
        Some(ScaleSource::Ruler { rulers }) => {
            ensure(
                !rulers.is_empty(),
                &field("scale.rulers"),
                "needs at least one ruler",
            )?;
            for (index, ruler) in rulers.iter().enumerate() {
                let ruler_path = format!("{}[{index}]", field("scale.rulers"));
                for (end, point) in [("a", ruler.a), ("b", ruler.b)] {
                    ensure(
                        is_near_film(point, width, height),
                        &format!("{ruler_path}.{end}"),
                        "point is far outside the film",
                    )?;
                }
                ensure(
                    ruler.a != ruler.b,
                    &ruler_path,
                    "the two ends are the same point",
                )?;
                within(ruler.mm, (1.0, 2000.0), &format!("{ruler_path}.mm"))?;
            }
        }
        Some(ScaleSource::KnownObject {
            centre,
            radius_px,
            diameter_mm,
        }) => {
            ensure(
                is_near_film(*centre, width, height),
                &field("scale.centre"),
                "point is far outside the film",
            )?;
            positive(*radius_px, &field("scale.radiusPx"))?;
            within(*diameter_mm, (0.1, 2000.0), &field("scale.diameterMm"))?;
        }
    }
    let front_view = film.kind.view() == View::Front;
    if let Some(right) = geometry.image_right {
        ensure(
            right.suits_front_view() == front_view,
            &field("imageRight"),
            "front and bending films face patient left or right; side films anterior or posterior",
        )?;
    }
    let confirmed = &geometry.confirmed;
    ensure(
        !confirmed.left_right || (front_view && geometry.image_right.is_some()),
        &field("confirmed.leftRight"),
        "only a front or bending film with a known imageRight can have left and right confirmed",
    )?;
    ensure(
        !confirmed.facing || (!front_view && geometry.image_right.is_some()),
        &field("confirmed.facing"),
        "only a side film with a known imageRight can have its facing confirmed",
    )?;
    ensure(
        !confirmed.scale || geometry.mm_per_px.is_some(),
        &field("confirmed.scale"),
        "a scale cannot be confirmed without one",
    )
}

fn check_film_marks(
    marks: &FilmMarks,
    film: &Film,
    path: &str,
    ids: &mut BTreeSet<u32>,
) -> Checked {
    let front_view = film.kind.view() == View::Front;
    ensure(
        marks.named_for.suits_front_view() == front_view,
        &format!("{path}.namedFor"),
        "front and bending films are named for patient left or right; side films for anterior or posterior",
    )?;
    let naming = &marks.naming;
    let naming_path = format!("{path}.naming");
    check_naming(naming, marks.vertebrae.len(), &naming_path)?;
    let names = naming.template.names();
    let template_levels = naming.template.levels();
    let non_standard = naming.flags.contains(&NamingFlag::NonStandardCount);
    let mut previous = None;
    for (index, vertebra) in marks.vertebrae.iter().enumerate() {
        let vertebra_path = format!("{path}.vertebrae[{index}]");
        ensure(
            ids.insert(vertebra.id),
            &format!("{vertebra_path}.id"),
            "a vertebra id appears twice in the case",
        )?;
        ensure(
            previous.is_none_or(|above| vertebra.level > above),
            &format!("{vertebra_path}.level"),
            "levels must run top to bottom with none repeated",
        )?;
        ensure(
            names.contains(&vertebra.level),
            &format!("{vertebra_path}.level"),
            "this level is not a name the film's vertebra count (naming.template) can give",
        )?;
        ensure(
            non_standard || template_levels.contains(&vertebra.level),
            &format!("{vertebra_path}.level"),
            "a level outside the film's vertebra count needs the non-standard-count flag",
        )?;
        previous = Some(vertebra.level);
        check_vertebra(vertebra, film, &vertebra_path)?;
    }
    if let Some(id) = naming.l5_confirmed_on {
        ensure(
            marks
                .vertebrae
                .iter()
                .any(|vertebra| vertebra.id == id && vertebra.level == Level::L5),
            &format!("{naming_path}.l5ConfirmedOn"),
            "must be the id of this film's L5",
        )?;
    }
    let landmarks_path = format!("{path}.landmarks");
    check_c7(
        marks.landmarks.c7.as_ref(),
        &marks.vertebrae,
        film,
        &format!("{landmarks_path}.c7"),
    )?;
    check_landmarks(&marks.landmarks, film, &landmarks_path)
}

fn check_naming(naming: &LevelNaming, vertebrae: usize, path: &str) -> Checked {
    let mut templates = BTreeSet::new();
    for (index, alternative) in naming.alternatives.iter().enumerate() {
        let alternative_path = format!("{path}.alternatives[{index}]");
        ensure(
            templates.insert(alternative.template),
            &format!("{alternative_path}.template"),
            "a template appears twice",
        )?;
        within(
            alternative.chance,
            (0.0, 1.0),
            &format!("{alternative_path}.chance"),
        )?;
    }
    let names = naming.template.names();
    for (name, count) in [
        ("fromTop", &naming.from_top),
        ("fromBottom", &naming.from_bottom),
    ] {
        if let Some(levels) = count {
            let count_path = format!("{path}.{name}");
            ensure(
                levels.len() == vertebrae,
                &count_path,
                "must name each of the film's vertebrae, in order",
            )?;
            ensure(
                levels.windows(2).all(|pair| pair[0] < pair[1]),
                &count_path,
                "levels must run top to bottom with none repeated",
            )?;
            ensure(
                levels.iter().all(|level| names.contains(level)),
                &count_path,
                "every level must be a name the film's vertebra count (template) can give",
            )?;
        }
    }
    unique(&naming.flags, &format!("{path}.flags"))
}

fn check_point(point: Px, film: &Film, path: &str) -> Checked {
    ensure(
        is_near_film(point, film.image.width, film.image.height),
        path,
        "point is far outside the film",
    )
}

fn check_mark(mark: &Mark, film: &Film, path: &str) -> Checked {
    check_point(mark.at, film, &format!("{path}.at"))
}

fn check_vertebra(vertebra: &VertebraOnFilm, film: &Film, path: &str) -> Checked {
    let marks_path = format!("{path}.marks");
    ensure(
        vertebra.marks.suits(film.kind),
        &format!("{marks_path}.kind"),
        "front films take front marks, side films side marks, bending films endplate lines",
    )?;
    match &vertebra.marks {
        VertebraMarks::Front { points } => {
            let points_path = format!("{marks_path}.points");
            for (name, mark) in [
                ("supL", points.sup_l),
                ("supR", points.sup_r),
                ("infL", points.inf_l),
                ("infR", points.inf_r),
            ] {
                check_mark(&mark, film, &format!("{points_path}.{name}"))?;
            }
            for (name, pedicle) in [("pedL", points.ped_l), ("pedR", points.ped_r)] {
                if let PedicleMark::Visible { at, ring_score, .. } = pedicle {
                    check_point(at, film, &format!("{points_path}.{name}.at"))?;
                    if let Some(score) = ring_score {
                        finite(score, &format!("{points_path}.{name}.ringScore"))?;
                    }
                }
            }
        }
        VertebraMarks::Side { points } => {
            let points_path = format!("{marks_path}.points");
            for (name, mark) in [
                ("supA", points.sup_a),
                ("supP", points.sup_p),
                ("infA", points.inf_a),
                ("infP", points.inf_p),
            ] {
                check_mark(&mark, film, &format!("{points_path}.{name}"))?;
            }
        }
        VertebraMarks::Bending { lines } => {
            for (name, line) in [("sup", lines.sup), ("inf", lines.inf)] {
                for (end, mark) in line.iter().enumerate() {
                    check_mark(mark, film, &format!("{marks_path}.lines.{name}[{end}]"))?;
                }
            }
        }
    }
    check_confidence(&vertebra.confidence, &format!("{path}.confidence"))?;
    if let Some(outline) = &vertebra.outline {
        ensure(
            outline.len() >= 3,
            &format!("{path}.outline"),
            "an outline needs at least three points",
        )?;
        for (index, point) in outline.iter().enumerate() {
            check_point(*point, film, &format!("{path}.outline[{index}]"))?;
        }
    }
    Ok(())
}

fn check_confidence(confidence: &Confidence, path: &str) -> Checked {
    within(confidence.score, (0.0, 1.0), &format!("{path}.score"))?;
    ensure(
        confidence.band_fits_score(),
        &format!("{path}.band"),
        "the band claims more than the score earns",
    )?;
    unique(&confidence.reasons, &format!("{path}.reasons"))?;
    if let Some(spread) = confidence.endplate_spread_px {
        within(
            spread.sup,
            (0.0, f64::MAX),
            &format!("{path}.endplateSpreadPx.sup"),
        )?;
        within(
            spread.inf,
            (0.0, f64::MAX),
            &format!("{path}.endplateSpreadPx.inf"),
        )?;
    }
    if let Some(margin) = confidence.margin {
        finite(margin, &format!("{path}.margin"))?;
    }
    Ok(())
}

/// `c7` must be what [`Landmarks::c7_of`] works out from the film's C7
/// vertebra, so the C7 plumb line can never come from a point that
/// disagrees with the vertebra.
fn check_c7(c7: Option<&Mark>, vertebrae: &[VertebraOnFilm], film: &Film, path: &str) -> Checked {
    let Some(expected) = Landmarks::c7_of(vertebrae) else {
        return ensure(
            c7.is_none(),
            path,
            "the film has no C7 vertebra, so c7 must be null",
        );
    };
    let Some(c7) = c7 else {
        return Err(invalid(
            path,
            "the film has a C7 vertebra, so c7 must be the centre of its corners",
        ));
    };
    check_mark(c7, film, path)?;
    let (dx, dy) = (c7.at.x() - expected.at.x(), c7.at.y() - expected.at.y());
    ensure(
        dx * dx + dy * dy <= C7_TOLERANCE_PX * C7_TOLERANCE_PX,
        &format!("{path}.at"),
        "must be the mean of the C7 vertebra's four corners",
    )?;
    ensure(
        !c7.pinned,
        &format!("{path}.pinned"),
        "c7 follows the C7 vertebra's corners and is never pinned",
    )?;
    ensure(
        c7.source == MarkSource::Auto,
        &format!("{path}.source"),
        "c7 is worked out from the C7 corners, never placed, so its source is auto",
    )?;
    ensure(
        c7.inferred == expected.inferred,
        &format!("{path}.inferred"),
        "c7 is inferred exactly when one of the C7 vertebra's corners is",
    )
}

fn check_landmarks(landmarks: &Landmarks, film: &Film, path: &str) -> Checked {
    let front_view = film.kind.view() == View::Front;
    match &landmarks.sacrum_top {
        None => {}
        Some(SacrumTop::Front { sac_l, sac_r }) => {
            ensure(
                front_view,
                &format!("{path}.sacrumTop.view"),
                "a side film's sacrum top is named sacA and sacP",
            )?;
            check_mark(sac_l, film, &format!("{path}.sacrumTop.sacL"))?;
            check_mark(sac_r, film, &format!("{path}.sacrumTop.sacR"))?;
        }
        Some(SacrumTop::Side { sac_a, sac_p }) => {
            ensure(
                !front_view,
                &format!("{path}.sacrumTop.view"),
                "a front or bending film's sacrum top is named sacL and sacR",
            )?;
            check_mark(sac_a, film, &format!("{path}.sacrumTop.sacA"))?;
            check_mark(sac_p, film, &format!("{path}.sacrumTop.sacP"))?;
        }
    }

    let sides: Vec<HipSide> = landmarks.hips.iter().map(|hip| hip.side).collect();
    let hips_path = format!("{path}.hips");
    let fits = match film.kind {
        FilmKind::Front => {
            sides
                .iter()
                .all(|side| matches!(side, HipSide::Left | HipSide::Right))
                && sides.iter().filter(|side| **side == HipSide::Left).count() <= 1
                && sides.iter().filter(|side| **side == HipSide::Right).count() <= 1
        }
        FilmKind::Side => matches!(
            sides.as_slice(),
            [] | [HipSide::Unpaired] | [HipSide::Merged] | [HipSide::Unpaired, HipSide::Unpaired]
        ),
        FilmKind::BendLeft | FilmKind::BendRight => sides.is_empty(),
    };
    ensure(
        fits,
        &hips_path,
        "front films have one left and one right hip at most; side films one or two unpaired or one merged; bending films none",
    )?;
    for (index, hip) in landmarks.hips.iter().enumerate() {
        check_mark(&hip.centre, film, &format!("{hips_path}[{index}].centre"))?;
        if let Some(radius) = hip.radius_px {
            positive(radius, &format!("{hips_path}[{index}].radiusPx"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_case_round_trips() {
        let case = CaseFileV1::new("0.1.0");
        let text = write_case_json(&case);
        assert!(text.ends_with("}\n"));
        assert_eq!(read_case_json(&text), Ok(case));
    }

    #[test]
    fn a_missing_field_is_named_by_its_path() {
        let text = r#"{"format":"spine-case","version":1,"savedWith":{"app":"0.1.0"},"films":[],"marks":[]}"#;
        let error = read_case_json(text).expect_err("nextVertebraId is missing");
        assert_eq!(error.code, ErrorCode::CaseInvalidValue);
        assert!(error.message.contains("nextVertebraId"), "{error}");
    }

    #[test]
    fn the_app_version_cannot_carry_free_text() {
        let mut case = CaseFileV1::new("0.1.0");
        case.saved_with.app = "made for Jane Doe".to_owned();
        let error = check_case_file(&case).expect_err("spaces are not part of a version");
        assert_eq!(error.detail.as_deref(), Some("savedWith.app"));
    }

    #[test]
    fn missing_fields_are_found_deep_inside_arrays() {
        let canonical: Value =
            serde_json::from_str(r#"{"a":[{"b":1,"c":null}]}"#).expect("valid JSON");
        let input: Value = serde_json::from_str(r#"{"a":[{"b":1}]}"#).expect("valid JSON");
        let mut path = String::new();
        assert_eq!(
            first_missing_field(&canonical, &input, &mut path),
            Some("a[0].c".to_owned())
        );
    }
}
