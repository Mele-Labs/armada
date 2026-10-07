//! Carrying what a clone already holds into the ledger, once.
//! `docs/capabilities/needs.md`, *Converting what exists*.
//!
//! **Read at the first start with the table, then never again.** Each file under
//! the clone's `armada-needs/` becomes a row, held by the Job whose branch it was
//! declared from where there is one, else by the branch alone
//! (`ledger::branch_holder`). A row at `kind` `needs_files` is the record that it
//! was done, because the files are left where they are: nothing is deleted until
//! a person has seen the list.
//!
//! **The Jobs' own slot and branch rows are written here too**, for a Job that
//! began before Fleet wrote them as it leased. That one is not once: it is
//! idempotent, and a Job that holds a branch should read as holding it however
//! it started.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::needs::Needs;
use core_model::Job;
use store::{AttachmentState, Holder, KeptAttachment};

use super::ledger::{self, NEED};
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// The record that a clone's files were read.
const CONVERTED: &str = "needs_files";
const CONVERTED_TARGET: &str = "armada-needs";

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
    /// Convert one repository's files, where that has not been done, and write
    /// its live Jobs' own branch and slot. **A fault is left for the next start**:
    /// nothing is recorded as converted that was not.
    pub(crate) async fn needs_converted(&self, served: &Served, jobs: &[Job]) {
        let manifest = served.manifest().id().as_str().to_string();
        self.live_jobs_on_the_ledger(&manifest, jobs).await;
        let done = {
            let store = self.store().lock().await;
            store
                .attachments_at(CONVERTED, CONVERTED_TARGET, Some(&manifest))
                .map(|rows| !rows.is_empty())
        };
        if done.unwrap_or(true) {
            return;
        }
        let root = served.root().to_string();
        let Ok(files) = tokio::task::spawn_blocking(move || {
            Needs::of(Path::new(&root)).map(|needs| needs.files())
        })
        .await
        else {
            return;
        };
        // A repository git will not answer for has no files to read, and is
        // read again at the next start rather than marked.
        let Ok(files) = files else {
            return;
        };
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        for need in files {
            let holder = jobs
                .iter()
                .find(|job| {
                    !job.status().is_terminal()
                        && job
                            .branch()
                            .is_some_and(|branch| branch.as_str() == need.branch)
                })
                .map(|job| Holder::job(job.id().as_str()))
                .unwrap_or_else(|| ledger::branch_holder(&need.branch));
            let path = adapters::needs::clean_path(&need.path);
            let held = store
                .attachments_of(&holder)
                .map(|rows| {
                    rows.iter().any(|row| {
                        row.kind == NEED && row.manifest_id == manifest && row.target == path
                    })
                })
                .unwrap_or(true);
            if held {
                continue;
            }
            let mut detail = BTreeMap::from([
                ("what".to_string(), need.what.clone()),
                ("branch".to_string(), need.branch.clone()),
            ]);
            if let Some(took) = &need.took {
                detail.insert("took".into(), took.clone());
            }
            // Its place in the line, from when it declared: nanoseconds there,
            // an instant here, so the order a clone had is the order it keeps.
            let at = rfc3339_utc(need.place / 1_000_000);
            let wrote = store.attach(
                &KeptAttachment {
                    holder,
                    kind: NEED.into(),
                    manifest_id: manifest.clone(),
                    target: path,
                    state: AttachmentState::Standing,
                    detail,
                    since: at.clone(),
                    changed_at: at,
                },
                false,
            );
            if wrote.is_err() {
                return;
            }
        }
        let _ = store.attach(
            &KeptAttachment {
                holder: Holder::session("fleet"),
                kind: CONVERTED.into(),
                manifest_id: manifest,
                target: CONVERTED_TARGET.into(),
                state: AttachmentState::Spent,
                detail: BTreeMap::new(),
                since: now.clone(),
                changed_at: now,
            },
            false,
        );
    }
}
