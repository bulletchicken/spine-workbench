//! Every refusal crosses to the app as an existing error code, with its
//! own name in the detail, as the joint fit's refusals do.

use implant_layout::LayoutError;
use shapes::errors::{EngineError, ErrorCode};

#[test]
fn each_refusal_takes_an_existing_code_and_keeps_its_name() {
    let cases = [
        (
            LayoutError::Refused("no-instrumented-levels".to_string()),
            ErrorCode::ToolRefused,
            "implant-layout-refused: no-instrumented-levels",
        ),
        (
            LayoutError::InvalidValue {
                field: "versions".to_string(),
                reason: "zero".to_string(),
            },
            ErrorCode::CaseInvalidValue,
            "implant-layout-invalid-value: versions: zero",
        ),
        (
            LayoutError::NoSpine,
            ErrorCode::PassNoSpineFound,
            "implant-layout-no-spine",
        ),
    ];
    for (error, code, name) in cases {
        let crossed = EngineError::from(error);
        assert_eq!(crossed.code, code, "{name}");
        assert_eq!(crossed.message, name);
        assert_eq!(crossed.detail.as_deref(), Some(name));
    }
}
