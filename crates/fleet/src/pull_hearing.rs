//! Hearing from the forge that a pull request changed. A Job's open pull request is subscribed to,
//! and an event reads that one again through the same path as a person's Refresh, so each notice
//! is still told once (`crate::pull_refreshing`). **A pull request whose stream is attached is left
//! out of the minute sweep** (`engine::Live`) and read on its events and every ten minutes besides.
//! An account with events off, a stream that dropped and a pull request past the limit are all the
//! sweep's to read, a minute late. `engine` holds the subscriptions.

mod engine;

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, PullEvents, Vcs, WorkProduct};
use api::PullRequests;
use ipc::{ManifestId, PullRequestStanding};
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use engine::{Hearing, Timing, Want};
pub(crate) use engine::Live;

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
    /// The open pull requests a Session holds, as the sweep last read them. **No forge read**: the
    /// sweep's list and the Sessions' `pr` rows are what Fleet already knows.
    async fn pulls_wanted(&self) -> Vec<Want> {
        let mut wanted = Vec::new();
        for served in self.repositories().served() {
            let held = self.held_pulls(&served).await;
            let sweep = self.sweeping().lock().await;
            let Some(open) = sweep.pulls.get(served.root()) else { continue };
            for number in held {
                if open.iter().any(|one| one.pull.number == number) {
                    wanted.push(Want {
                        manifest: served.manifest().id().as_str().to_string(),
                        root: served.root().to_string(),
                        number,
                    });
                }
            }
        }
        wanted
    }

    /// The pull requests whose stream is attached now, which the minute sweep leaves to their events.
    pub(crate) async fn heard_live(&self) -> Live {
        self.sweeping().lock().await.heard_live.clone()
    }

    /// Read one pull request again as a person's Refresh does. Says whether it is over.
    async fn pull_heard_again(self: &Arc<Self>, want: Want) -> bool {
        let manifest = ManifestId::carried(&want.manifest);
        match Arc::clone(self).refresh_pull_request(manifest, want.number).await {
            Ok(state) => matches!(state.state, PullRequestStanding::Merged | PullRequestStanding::Closed),
            Err(_) => false,
        }
    }
}

/// Subscribe to the pull requests Jobs hold and read each again when the forge says it changed.
pub fn keep_hearing_pulls<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    source: Arc<dyn PullEvents>,
    told: impl Fn(&str) + Send + Sync + 'static,
) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let live = fleet.heard_live().await;
        let (hearing, heard) = Hearing::opened(source, Timing::SHIPPED, Arc::new(told), live);
        let (asked, reading) = (Arc::clone(&fleet), fleet);
        hearing
            .run(
                heard,
                move || {
                    let fleet = Arc::clone(&asked);
                    async move { fleet.pulls_wanted().await }
                },
                move |want| {
                    let fleet = Arc::clone(&reading);
                    async move { fleet.pull_heard_again(want).await }
                },
            )
            .await
    })
}

#[cfg(test)]
#[path = "pull_hearing/tests.rs"]
mod tests;
