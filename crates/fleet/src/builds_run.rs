//! Keeping every build Fleet runs, for the Usage page to read its figures against. Read on the sweep
//! beside the notices, for the reason `build_notices.rs` gives. `docs/concepts/fleet.md`.
//!
//! A build begins when the restart wrote `restart-commit`, not when this sweep saw it. What a
//! preview holds is `preview-held`, which `scripts/preview` writes; a build from before that file
//! existed is asked of git once, and what git cannot say stays unknown.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::BuildSource;
use store::NewBuild;

use crate::building::{build_word, commit_written_in, held_in};
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;

/// The build `commit` is, as the files `scripts/restart` and `scripts/preview` left say it is.
pub(crate) fn build_on_file(support: &Path, build: BuildSource, commit: &str, now_millis: i64) -> NewBuild {
    let started = commit_written_in(support).unwrap_or(now_millis);
    let (main_commit, branches, looked) = match build {
        BuildSource::Main => (Some(commit.to_string()), Vec::new(), true),
        BuildSource::Preview => match held_in(support, commit) {
            Some(held) => (Some(held.main_commit), held.branches, true),
            None => (None, Vec::new(), false),
        },
    };
    NewBuild {
        commit_id: commit.to_string(),
        started_at: rfc3339_utc(started),
        source: Some(build_word(build).to_string()),
        main_commit,
        branches,
        looked,
    }
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
    /// Keep the build on file, if it is not the newest kept, and ask git about any that were kept
    /// without being told what they hold.
    pub(crate) async fn build_kept(&self, support: &Path, build: BuildSource, commit: &str) {
        let now = self.now().epoch_millis().unwrap_or(0);
        let one = build_on_file(support, build, commit, now);
        let _ = self.store().lock().await.keep_build(&one);
        self.builds_looked_at().await;
    }

    /// The kept builds nothing recorded the contents of, read from git in the repositories Fleet
    /// serves. One that no repository holds is left for the next time.
    pub(crate) async fn builds_looked_at(&self) {
        let unlooked = self.store().lock().await.builds_unlooked().unwrap_or_default();
        if unlooked.is_empty() {
            return;
        }
        let roots: Vec<String> = self.repositories().served().iter().map(|served| served.root().to_string()).collect();
        for kept in unlooked {
            let (roots, commit) = (roots.clone(), kept.build.commit_id.clone());
            let made = tokio::task::spawn_blocking(move || roots.iter().find_map(|root| adapters::build_made_of(root, &commit))).await.ok().flatten();
            let Some(made) = made else {
                continue;
            };
            let looked = NewBuild {
                source: Some(if made.preview { "preview" } else { "main" }.to_string()),
                main_commit: Some(made.main_commit),
                branches: made.branches,
                ..kept.build
            };
            let _ = self.store().lock().await.build_looked(kept.id, &looked);
        }
    }
}
