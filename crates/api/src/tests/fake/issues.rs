//! Issues, as the fake answers them: none, read just now. What Fleet reads and
//! keeps is `fleet::tracking`'s, against a scripted tracker.

use ipc::{Instant, IssueList, ManifestId};

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
}
