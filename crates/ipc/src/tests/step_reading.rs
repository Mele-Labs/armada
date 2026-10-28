//! A step's ends on the wire, as `StepReading` in `packages/protocol/src/setup.ts` spells them.

use serde_json::json;

use crate::StepReading;

#[test]
fn a_step_that_ends_on_any_of_several_lists_them_in_order_and_one_with_none_leaves_the_field_out() {
    let read: StepReading = serde_json::from_value(json!({
        "id": "propose", "label": "Propose",
        "ends_when": "the person approves a sketch",
        "ends_any": ["the person approves a sketch", "the plan is approved"]
    }))
    .unwrap();
    assert_eq!(read.ends_any, ["the person approves a sketch", "the plan is approved"]);

    let bare: StepReading = serde_json::from_value(json!({"id": "build", "label": "Build"})).unwrap();
    assert!(bare.ends_any.is_empty());
    assert_eq!(serde_json::to_value(&bare).unwrap(), json!({"id": "build", "label": "Build"}));
}
