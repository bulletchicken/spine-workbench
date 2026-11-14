//! Placement proposals and placements changed by hand, natively and
//! through the browser bridge.

use crate::{crossing, faults};
use ::shapes::errors::EngineError;
use ::shapes::implant_layout::{LayoutInput, RebuildInput};
use wasm_bindgen::prelude::*;

/// Proposes placements on a fitted spine, counted over versions of it.
#[wasm_bindgen]
pub fn propose_layouts(input: JsValue) -> Result<JsValue, JsValue> {
    let input: LayoutInput = crossing::from_js(input)?;
    let proposals = implant_layout::propose_layouts(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&proposals)
}

/// Rebuilds a whole placement after its ends, anchors or rods change.
#[wasm_bindgen]
pub fn rebuild_layout(input: JsValue) -> Result<JsValue, JsValue> {
    let input: RebuildInput = crossing::from_js(input)?;
    let layout = implant_layout::rebuild_layout(&input)
        .map_err(|error| faults::thrown(EngineError::from(error)))?;
    crossing::to_js(&layout)
}
