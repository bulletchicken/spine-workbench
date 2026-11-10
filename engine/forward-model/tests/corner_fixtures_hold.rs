//! The hand-worked corner fixtures in `proving-ground/known-answers/corners/`
//! hold: every vertebra on every film lands its marks where the fixture
//! says, within the fixture's tolerance. `README.md` beside the fixtures
//! gives the format and how each expected value was worked out.
//!
//! The fixtures are read into the structs below, which refuse any unknown
//! field and any missing one (other than `mmPerPxVertical`), so a fixture
//! can never say something this test silently skips. The film's `kind` is
//! the shapes crate's `FilmKind`, and it picks which marks `expect` must
//! hold, so a missing or misspelt mark is refused by name.

// Reading the fixture files is the point of this test; the engine itself
// never touches files.
#![allow(clippy::disallowed_methods)]

use forward_model::cameras::parallel::ParallelCamera;
use forward_model::project::{endplate_lines, front_marks, side_marks};
use forward_model::vertebra_params::VertebraParams;
use nalgebra::Vector2;
use serde::Deserialize;
use shapes::film::{FilmKind, View};
use shapes::film_geometry::{FilmGeometry, ImageRight, QuarterTurns};
use shapes::marks::{Mark, PedicleMark};
use std::path::{Path, PathBuf};

type Point = [f64; 2];

/// One fixture file, as written.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureText {
    what: String,
    tolerance_px: f64,
    cases: Vec<CaseText>,
}

/// One case, as written: its expected marks are read once the film's kind
/// says which marks they are.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaseText {
    what: String,
    vertebra: Vertebra,
    film: Film,
    expect: serde_json::Value,
}

/// One fixture file, read and checked.
struct FixtureFile {
    what: String,
    tolerance_px: f64,
    cases: Vec<Case>,
}

struct Case {
    what: String,
    vertebra: Vertebra,
    film: Film,
    expect: Expected,
}

/// The thirteen numbers, named as in a made-up spine's saved pose.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Vertebra {
    centre: [f64; 3],
    rotation: [f64; 3],
    height: f64,
    wedge_front: f64,
    wedge_side: f64,
    top_half_width: f64,
    top_half_depth: f64,
    bottom_half_width: f64,
    bottom_half_depth: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Film {
    kind: FilmKind,
    image_right: ImageRight,
    quarter_turns: QuarterTurns,
    size: [u32; 2],
    mm_per_px: f64,
    mm_per_px_vertical: Option<f64>,
    origin: Point,
}

/// The marks a case expects, the set its film's kind calls for.
enum Expected {
    Front(FrontExpected),
    Side(SideExpected),
    Bending(BendingExpected),
}

impl Expected {
    fn read(kind: FilmKind, marks: serde_json::Value) -> Result<Expected, serde_json::Error> {
        match kind {
            FilmKind::Front => serde_json::from_value(marks).map(Expected::Front),
            FilmKind::Side => serde_json::from_value(marks).map(Expected::Side),
            FilmKind::BendLeft | FilmKind::BendRight => {
                serde_json::from_value(marks).map(Expected::Bending)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FrontExpected {
    sup_l: Point,
    sup_r: Point,
    inf_l: Point,
    inf_r: Point,
    ped_l: Point,
    ped_r: Point,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SideExpected {
    sup_a: Point,
    sup_p: Point,
    inf_a: Point,
    inf_p: Point,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BendingExpected {
    sup: [Point; 2],
    inf: [Point; 2],
}

fn fixture_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../proving-ground/known-answers/corners")
}

fn fixture_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(fixture_folder())
        .expect("the corners folder exists")
        .map(|entry| entry.expect("a readable entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

/// Reads a fixture, refusing a case whose film faces a way its kind
/// cannot, or whose marks are not exactly those of its kind.
fn read(text: &str) -> Result<FixtureFile, String> {
    let file: FixtureText = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let cases = file
        .cases
        .into_iter()
        .map(|case| {
            let front_view = case.film.kind.view() == View::Front;
            if front_view != case.film.image_right.suits_front_view() {
                return Err(format!(
                    "{}: a {:?} film cannot face {:?}",
                    case.what, case.film.kind, case.film.image_right
                ));
            }
            let expect = Expected::read(case.film.kind, case.expect)
                .map_err(|e| format!("{}: expect: {e}", case.what))?;
            Ok(Case {
                what: case.what,
                vertebra: case.vertebra,
                film: case.film,
                expect,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(FixtureFile {
        what: file.what,
        tolerance_px: file.tolerance_px,
        cases,
    })
}

#[test]
fn every_fixture_lands_where_it_says() {
    let files = fixture_files();
    assert!(!files.is_empty(), "no corner fixtures found");
    let mut checked = 0;
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = std::fs::read_to_string(&path).expect("the fixture is readable");
        let file = read(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!file.what.is_empty(), "{name}: says what its cases share");
        for case in &file.cases {
            check_case(case, file.tolerance_px, &format!("{name}: {}", case.what));
            checked += 1;
        }
    }
    assert!(checked > 0, "the fixtures hold no cases");
}

fn check_case(case: &Case, tolerance: f64, what: &str) {
    let vertebra = vertebra(&case.vertebra);
    let camera = camera(&case.film);
    let pairs: Vec<(&str, Mark, Point)> = match &case.expect {
        Expected::Front(e) => {
            let m = front_marks(&vertebra, &camera).expect("front corners exist");
            let pedicle = |p: PedicleMark| p.mark().expect("a projected pedicle is visible");
            vec![
                ("supL", m.sup_l, e.sup_l),
                ("supR", m.sup_r, e.sup_r),
                ("infL", m.inf_l, e.inf_l),
                ("infR", m.inf_r, e.inf_r),
                ("pedL", pedicle(m.ped_l), e.ped_l),
                ("pedR", pedicle(m.ped_r), e.ped_r),
            ]
        }
        Expected::Side(e) => {
            let m = side_marks(&vertebra, &camera).expect("side corners exist");
            vec![
                ("supA", m.sup_a, e.sup_a),
                ("supP", m.sup_p, e.sup_p),
                ("infA", m.inf_a, e.inf_a),
                ("infP", m.inf_p, e.inf_p),
            ]
        }
        Expected::Bending(e) => {
            let m = endplate_lines(&vertebra, &camera).expect("endplate lines exist");
            vec![
                ("sup left", m.sup[0], e.sup[0]),
                ("sup right", m.sup[1], e.sup[1]),
                ("inf left", m.inf[0], e.inf[0]),
                ("inf right", m.inf[1], e.inf[1]),
            ]
        }
    };
    for (name, mark, expected) in pairs {
        assert_near(mark, expected, tolerance, &format!("{what}: {name}"));
    }
}

#[track_caller]
fn assert_near(mark: Mark, expected: Point, tolerance: f64, what: &str) {
    let off = libm::hypot(mark.at.x() - expected[0], mark.at.y() - expected[1]);
    assert!(
        off <= tolerance,
        "{what}: expected {expected:?}, got {:?} ({off} px away)",
        mark.at.0
    );
}

fn vertebra(v: &Vertebra) -> VertebraParams {
    VertebraParams {
        centre: v.centre.into(),
        rotation: v.rotation.into(),
        height: v.height,
        wedge_front: v.wedge_front,
        wedge_side: v.wedge_side,
        top_half_width: v.top_half_width,
        top_half_depth: v.top_half_depth,
        bottom_half_width: v.bottom_half_width,
        bottom_half_depth: v.bottom_half_depth,
    }
}

fn camera(film: &Film) -> ParallelCamera {
    let geometry = FilmGeometry {
        mm_per_px: Some(film.mm_per_px),
        mm_per_px_vertical: film.mm_per_px_vertical,
        quarter_turns: film.quarter_turns,
        image_right: Some(film.image_right),
        ..FilmGeometry::UNKNOWN
    };
    ParallelCamera::for_film(&geometry, film.size, Vector2::from(film.origin))
        .expect("the fixture's film is complete")
}

/// A small fixture with one front case, in which `vertebra_extra` is
/// spliced into the vertebra and `film_extra` into the film.
fn small_fixture(vertebra_extra: &str, film_extra: &str) -> String {
    format!(
        r#"{{
  "what": "one case",
  "tolerancePx": 1e-9,
  "cases": [{{
    "what": "straight",
    "vertebra": {{
      "centre": [0, 10, 100], "rotation": [0, 0, 0], "height": 20,
      "wedgeFront": 0, "wedgeSide": 0,
      "topHalfWidth": 15, "topHalfDepth": 10{vertebra_extra}
    }},
    "film": {{
      "kind": "front", "imageRight": "patient-left", "quarterTurns": 0,
      "size": [2000, 4000], "mmPerPx": 0.5, "origin": [1000, 3000]{film_extra}
    }},
    "expect": {{
      "supL": [1050, 2780], "supR": [990, 2780],
      "infL": [1050, 2820], "infR": [990, 2820],
      "pedL": [1043.4, 2792.8], "pedR": [996.6, 2792.8]
    }}
  }}]
}}"#
    )
}

const BOTTOM: &str = r#", "bottomHalfWidth": 15, "bottomHalfDepth": 10"#;

#[test]
fn a_complete_fixture_is_read() {
    let file = read(&small_fixture(BOTTOM, "")).expect("a complete fixture reads");
    assert_eq!(file.cases.len(), 1);
    assert_eq!(file.cases[0].film.mm_per_px_vertical, None);
    let vertical = read(&small_fixture(BOTTOM, r#", "mmPerPxVertical": 0.4"#))
        .expect("mmPerPxVertical may be given");
    assert_eq!(vertical.cases[0].film.mm_per_px_vertical, Some(0.4));
}

#[test]
fn an_unknown_field_is_refused() {
    let misspelt = format!(r#"{BOTTOM}, "wedgeFrnot": 0.1"#);
    let error = read(&small_fixture(&misspelt, ""))
        .err()
        .expect("an unknown vertebra field is refused");
    assert!(error.contains("wedgeFrnot"), "{error}");
    let error = read(&small_fixture(BOTTOM, r#", "mmPerPxSide": 0.4"#))
        .err()
        .expect("an unknown film field is refused");
    assert!(error.contains("mmPerPxSide"), "{error}");
}

#[test]
fn a_missing_field_is_refused() {
    let no_bottom_depth = r#", "bottomHalfWidth": 15"#;
    let error = read(&small_fixture(no_bottom_depth, ""))
        .err()
        .expect("a missing vertebra field is refused");
    assert!(error.contains("bottomHalfDepth"), "{error}");
}

#[test]
fn a_missing_or_misspelt_mark_is_refused_by_name() {
    let missing = small_fixture(BOTTOM, "").replace(r#", "pedR": [996.6, 2792.8]"#, "");
    let error = read(&missing)
        .err()
        .expect("a front case missing pedR is refused");
    assert!(
        error.contains("straight") && error.contains("pedR"),
        "{error}"
    );
    let misspelt = small_fixture(BOTTOM, "").replace(r#""pedR""#, r#""pedRight""#);
    let error = read(&misspelt).err().expect("a misspelt mark is refused");
    assert!(error.contains("pedRight"), "{error}");
}

#[test]
fn the_marks_of_another_kind_are_refused() {
    let side = small_fixture(BOTTOM, "")
        .replace(r#""kind": "front""#, r#""kind": "side""#)
        .replace("patient-left", "anterior");
    let error = read(&side)
        .err()
        .expect("front marks on a side film are refused");
    assert!(error.contains("supA"), "{error}");
}

#[test]
fn a_film_facing_a_way_its_kind_cannot_is_refused() {
    let side = small_fixture(BOTTOM, "").replace(r#""kind": "front""#, r#""kind": "side""#);
    let error = read(&side)
        .err()
        .expect("a side film facing patient left is refused");
    assert!(
        error.contains("Side") && error.contains("PatientLeft"),
        "{error}"
    );
    let bending = small_fixture(BOTTOM, "")
        .replace(r#""kind": "front""#, r#""kind": "bend-left""#)
        .replace("patient-left", "posterior");
    let error = read(&bending)
        .err()
        .expect("a bending film facing posterior is refused");
    assert!(error.contains("BendLeft"), "{error}");
}

#[test]
fn the_readme_example_is_read_and_holds() {
    let readme = std::fs::read_to_string(fixture_folder().join("README.md"))
        .expect("the corners README is readable");
    let (_, after) = readme
        .split_once("```json\n")
        .expect("the README shows an example");
    let (example, _) = after.split_once("\n```").expect("the example is closed");
    let file = read(example).unwrap_or_else(|e| panic!("README example: {e}"));
    for case in &file.cases {
        check_case(
            case,
            file.tolerance_px,
            &format!("README example: {}", case.what),
        );
    }
}
