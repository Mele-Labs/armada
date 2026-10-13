//! Issues, as the fake answers them: none, read just now. What Fleet reads and
//! keeps is `fleet::tracking`'s, against a scripted tracker.

use ipc::{Instant, IssueList, IssueRead, IssueState, ManifestId, WireError};

use super::FakeDaemon;
use crate::{Issues, Refusal};

impl Issues for FakeDaemon {
    async fn list_issues(
        &self,
        manifest_id: ManifestId,
        _refresh: bool,
    ) -> Result<IssueList, Refusal> {
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried("2026-10-11T12:00:00Z"),
            issues: Vec::new(),
            stale: None,
        })
    }

    /// Issue 214 is open and 215 is closed; any other number is none.
    async fn get_issue(&self, _manifest_id: ManifestId, number: u64) -> Result<IssueRead, Refusal> {
        let state = match number {
            214 => IssueState::Open,
            215 => IssueState::Closed,
            _ => {
                return Err(Refusal::NoSuchJob(WireError::raised(
                    "fleet.no_such_issue",
                    format!("the tracker has no issue {number}"),
                    crate::tests::shapes::run_id(),
                )))
            }
        };
        Ok(IssueRead {
            number,
            title: String::from("A title"),
            url: format!("https://forge.invalid/o/r/issues/{number}"),
            state,
        })
    }
}
