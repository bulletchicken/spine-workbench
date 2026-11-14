//! Every refusal crosses to the app as an existing error code, with its
//! own name in the detail, as the joint fit's refusals do.

use load_bench::BenchError;
use shapes::errors::{EngineError, ErrorCode};

#[test]
fn each_refusal_takes_an_existing_code_and_keeps_its_name() {
    let cases = [
        (
            BenchError::Refused("open-ended-activity-band".to_string()),
            ErrorCode::ToolRefused,
            "load-bench-refused: open-ended-activity-band",
        ),
        (
            BenchError::Refused("no-layouts".to_string()),
            ErrorCode::ToolRefused,
            "load-bench-refused: no-layouts",
        ),
        (
            BenchError::InvalidValue {
                field: "versionCount".to_string(),
                reason: "past the last version".to_string(),
            },
            ErrorCode::CaseInvalidValue,
            "load-bench-invalid-value: versionCount: past the last version",
        ),
        (
            BenchError::NoSpine,
            ErrorCode::PassNoSpineFound,
            "load-bench-no-spine",
        ),
    ];
    for (error, code, name) in cases {
        let crossed = EngineError::from(error);
        assert_eq!(crossed.code, code, "{name}");
        assert_eq!(crossed.message, name);
        assert_eq!(crossed.detail.as_deref(), Some(name));
    }
}
