//! The forge's verdict on a merge, as `classic-final:packages/protocol/src/pull-requests.ts` spells it: on the
//! state a Session's row follows and on the page, and `unknown` from a Fleet that never sent it.

use crate::{decode, encode, PullRequestMergeState, PullRequestPage, PullRequestState};

const STATE: &str = r#"{"manifest_id":"m","number":7,"state":"open","auto_merge":false,"checks":{"state":"passed"},"title":"t","branch":"b","address":"https://forge.invalid/acme/widgets/pull/7""#;

const PAGE: &str = r#"{"manifest_id":"m","number":7,"base":"main","additions":1,"deletions":0,"mergeable":"mergeable","review":"required","approvals":0,"conversation":[],"checks":[],"files":[],"commits":[]"#;

#[test]
fn a_state_or_a_page_without_a_merge_state_reads_as_unknown() {
    let state: PullRequestState = decode("a state", format!("{STATE}}}").as_bytes()).expect("it reads");
    assert_eq!(state.merge_state, PullRequestMergeState::Unknown);
    let page: PullRequestPage = decode("a page", format!("{PAGE}}}").as_bytes()).expect("it reads");
    assert_eq!(page.merge_state, PullRequestMergeState::Unknown);
}

#[test]
fn a_merge_state_is_spelled_as_the_ledger_row_spells_it() {
    use PullRequestMergeState::*;
    for state in [Clean, Blocked, Behind, Dirty, Unstable, HasHooks, Draft, Unknown] {
        assert_eq!(encode(&state).expect("plain data"), format!("\"{}\"", state.word()));
    }
    let page: PullRequestPage =
        decode("a page", format!(r#"{PAGE},"merge_state":"has_hooks"}}"#).as_bytes()).expect("it reads");
    assert_eq!(page.merge_state, HasHooks);
    assert!(encode(&page).expect("plain data").contains(r#""merge_state":"has_hooks""#));
}
