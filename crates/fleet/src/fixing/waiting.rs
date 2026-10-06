//! Jobs whose Checks failed on a test another Job is fixing, pointed at that
//! fix and told how it settles. #1001.
//!
//! **Spotted, never asked for**: a failed Check a claim names whose output
//! carries the claimed test's name. **It informs and passes nothing** — a
//! pointed Job's gate still fails on that test until the fix lands.
//!
//! **A claim waits for the merge.** A fix Job completes before its pull request
//! merges, so its claim and its pointers are given back when the forge settles
//! it, or when the Job ends any other way.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Output;
use core_model::{BreakageClaim, FixWaiter, Job, JobId, JobStatus, ManifestId, Ulid};
use ipc::mcp::CheckReport;

use crate::daemon::Fleet;
use crate::peers::News;

/// Where a fix a Job was pointed at stands, as its peer turn says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FixStands {
    Fixing,
    Landed,
    /// Landed, and this Job's copy has taken it. #1673.
    InYourCopy,
    Gone,
}

impl FixStands {
    /// The item's line in the turn. `files` are what the fix holds off this
    /// Job, and empty where it holds nothing. #1673.
    pub(crate) fn line(self, title: &str, handle: &str, test: &str, files: &[String]) -> String {
        let named = files
            .iter()
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let said = match (self, files.is_empty()) {
            (FixStands::Fixing, true) => format!("is fixing `{test}`, which your checks failed on"),
            (FixStands::Fixing, false) => format!(
                "is fixing `{test}`, which your checks failed on. Until that fix lands and \
                 reaches your copy, {named} are outside what this Job may change"
            ),
            (FixStands::Landed, true) => format!("landed its fix for `{test}`"),
            (FixStands::Landed, false) => format!(
                "landed its fix for `{test}`. It reaches your copy when your next part starts, \
                 and until then {named} stay outside what this Job may change"
            ),
            (FixStands::InYourCopy, _) => format!(
                "landed its fix for `{test}`, and that fix is already in your copy. {named} \
                 are yours to change again"
            ),
            (FixStands::Gone, true) => {
                format!("ended without landing its fix for `{test}`, so nobody is fixing it now")
            }
            (FixStands::Gone, false) => format!(
                "ended without landing its fix for `{test}`, so nobody is fixing it now. \
                 {named} are yours to change again"
            ),
        };
        format!("\n- \"{title}\" ({handle}) {said}.")
    }
}

/// Each failed Check and both streams it printed, from the names that failed
/// and what every Check printed.
pub(crate) fn failures_said<'a>(
    failed: impl IntoIterator<Item = &'a str>,
    printed: &[(&str, &Output)],
) -> Vec<(String, String)> {
    failed
        .into_iter()
        .filter_map(|check| {
            printed
                .iter()
                .find(|(name, _)| *name == check)
                .map(|(_, output)| {
                    (
                        check.to_string(),
                        format!("{}\n{}", output.stdout, output.stderr),
                    )
                })
        })
        .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Point this Job at every claimed fix in its repository whose test one of
    /// its failed Checks printed. **Nothing here can fail the gate or the asked
    /// run that reached it**: a read that will not answer points nothing. A
    /// name cut from the output is missed: `[fleet-fix-pointing-reads-a-cut-output]`.
    pub(crate) async fn pointed_at_fixes(&self, job: &JobId, failed: &[(String, String)]) {
        if failed.is_empty() {
            return;
        }
        let Some(owner) = self.names().owner_of(job) else {
            return;
        };
        let repository = ManifestId::carried(Ulid::carried(owner));
        let Ok(claims) = self.store().lock().await.breakages_claimed_in(&repository) else {
            return;
        };
        for claim in claims {
            // The fix and the Job that reported the test already know.
            if claim.fix == *job || claim.reported_by == *job {
                continue;
            }
            let named = failed.iter().any(|(check, said)| {
                *check == claim.breakage.check && said.contains(&claim.breakage.test)
            });
            if named {
                self.point_at(job, &claim, true).await;
            }
        }
    }

    /// The same, from a asked run's report: each row that did not advance, with
    /// the tail of what it printed.
    pub(crate) async fn pointed_at_fixes_in(&self, job: &JobId, report: &CheckReport) {
        let failed: Vec<(String, String)> = report
            .ran
            .iter()
            .filter_map(|row| {
                row.output
                    .as_ref()
                    .map(|excerpt| (row.name.clone(), excerpt.lines.join("\n")))
            })
            .collect();
        self.pointed_at_fixes(job, &failed).await;
    }

    /// Record the pointer and, the first time and where `tell` asks, queue the
    /// news for the Job's Drone.
    pub(crate) async fn point_at(&self, job: &JobId, claim: &BreakageClaim, tell: bool) {
        let waiter = FixWaiter {
            waiting: job.clone(),
            fix: claim.fix.clone(),
            repository: claim.repository.clone(),
            check: claim.breakage.check.clone(),
            test: claim.breakage.test.clone(),
        };
        let now = self.now();
        let pointed = self
            .store()
            .lock()
            .await
            .point_at_fix(&waiter, &now)
            .unwrap_or(false);
        if !pointed || !tell {
            return;
        }
        let Ok(fix) = self.load(&claim.fix).await else {
            return;
        };
        self.owe(
            job,
            News::Fix {
                title: fix.title().as_str().to_string(),
                handle: fix.handle(),
                test: waiter.test,
                stands: FixStands::Fixing,
                files: self
                    .held_by(claim)
                    .await
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect(),
            },
        )
        .await;
    }

    /// A Job that ended: its claims wait for the merge where its pull request
    /// is still open, and are given back now otherwise.
    pub(crate) async fn fix_ended(&self, job: &Job) {
        let awaiting_merge = job.status() == JobStatus::CompletedSuccess
            && self
                .store()
                .lock()
                .await
                .pull_requests_unsettled()
                .is_ok_and(|open| open.iter().any(|pull| pull.job_id == *job.id()));
        if !awaiting_merge {
            self.fix_settled(job.id(), false).await;
        }
    }

    /// Tell every Job pointed at this fix whether it landed, then give back the
    /// fix's claims and the pointers at it. A Job fixing nothing has neither.
    pub(crate) async fn fix_settled(&self, fix: &JobId, landed: bool) {
        // Those it held files off hear it with the files. #1673.
        let told = self.settled_holds(fix, landed).await;
        let waiting: Vec<FixWaiter> = self
            .store()
            .lock()
            .await
            .waiting_on_fix(fix)
            .unwrap_or_default()
            .into_iter()
            .filter(|waiter| !told.contains(&waiter.waiting))
            .collect();
        if !waiting.is_empty() {
            if let Ok(record) = self.load(fix).await {
                for waiter in waiting {
                    let news = News::Fix {
                        title: record.title().as_str().to_string(),
                        handle: record.handle(),
                        test: waiter.test,
                        stands: match landed {
                            true => FixStands::Landed,
                            false => FixStands::Gone,
                        },
                        files: Vec::new(),
                    };
                    self.owe(&waiter.waiting, news).await;
                }
            }
        }
        let mut store = self.store().lock().await;
        let _ = store.release_waiters(fix);
        let _ = store.release_breakages(fix);
    }
}
