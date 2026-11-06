//! A call put to a person by a process Fleet does not host. `#1389`.
//!
//! **Driven through the surface and not through a process.** The door offers `ask_the_person` to a
//! hosted Job and to no one else, so the only caller Fleet cannot place is a hand-made POST. A
//! hosted Job's own ask is held on its thread, which `session_host` and `unanswered` cover.

use std::sync::Arc;
use std::time::Duration;

use api::Conversations;
use ipc::{AskingToRun, RunOrNot};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// An unplaced ask is answered `deny` before the hold, which `crate::tests::daemon::fittings` sets
/// at two seconds, could end it, and nothing is left on a table for a person to find.
#[tokio::test]
async fn an_unplaced_ask_is_denied_at_once_and_nothing_is_left_waiting() {
    let home = TempDir::new();
    let fleet = Arc::new(Fleet::assembled(fittings(
        &home,
        FakeWorkProduct::changed(&[]),
    )));
    let asking: AskingToRun = ipc::decode(
        "a permission question",
        br#"{"tool_name":"Bash","input":{"command":"git push"},"tool_use_id":"toolu_01"}"#,
    )
    .expect("the fixture decodes");

    let decided = tokio::time::timeout(
        Duration::from_millis(500),
        Arc::clone(&fleet).ask_the_person_at(asking, None, api::Caller::unplaceable()),
    )
    .await
    .expect("answered at once, not held")
    .expect("an answer");

    assert_eq!(
        decided,
        RunOrNot::Deny {
            message: String::from(
                "This call came from outside any job Armada hosts, so nobody could be asked and it was not run"
            ),
        }
    );
    assert!(fleet.hosts().asks().waiting().is_empty());
}
