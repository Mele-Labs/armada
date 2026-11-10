//! A standing order on the wire and in the store is only ever one of its forms.

use crate::operator_positions::OperatorRole;
use crate::operator_talk::{OperatorHoldUntil, OperatorOrderForm};
use crate::SessionId;

#[test]
fn each_form_reads_back_as_itself_and_anything_else_is_no_form() {
    for form in [
        OperatorOrderForm::RoleOff { role: OperatorRole::Answer, manifest_id: "01REPO".into(), today: true },
        OperatorOrderForm::HoldPaths { manifest_id: "01REPO".into(), paths: vec!["apps/cockpit".into()], job: SessionId::carried("01JOB"), until: OperatorHoldUntil::Pushes },
        OperatorOrderForm::AlwaysRaise { what: "question".into() },
    ] {
        assert_eq!(OperatorOrderForm::read(&form.kept()), Some(form));
    }
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"be_careful","words":"please"}"#), None);
    assert_eq!(OperatorOrderForm::read("hold everything"), None);
}

#[test]
fn a_role_off_says_until_removed_unless_it_says_today() {
    let form = OperatorOrderForm::read(r#"{"kind":"role_off","role":"file","manifest_id":"01REPO"}"#).expect("a form");
    assert_eq!(form, OperatorOrderForm::RoleOff { role: OperatorRole::File, manifest_id: "01REPO".into(), today: false });
}
