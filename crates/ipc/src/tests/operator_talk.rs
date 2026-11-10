//! A standing order on the wire and in the store is only ever the one form, a hold on paths.

use crate::operator_talk::{OperatorHoldUntil, OperatorOrderForm};
use crate::SessionId;

#[test]
fn a_hold_reads_back_as_itself_and_anything_else_is_no_form() {
    let form = OperatorOrderForm::HoldPaths { manifest_id: "01REPO".into(), paths: vec!["apps/cockpit".into()], job: SessionId::carried("01JOB"), until: OperatorHoldUntil::Pushes };
    assert_eq!(OperatorOrderForm::read(&form.kept()), Some(form));
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"be_careful","words":"please"}"#), None);
    assert_eq!(OperatorOrderForm::read("hold everything"), None);
}

#[test]
fn the_forms_that_went_with_the_levels_read_as_no_form() {
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"role_off","role":"file","manifest_id":"01REPO"}"#), None);
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"always_raise","what":"note"}"#), None);
}
