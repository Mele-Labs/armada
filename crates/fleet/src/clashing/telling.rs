//! Saying which files clash and who the other side is, to each agent it concerns. Bare facts, with
//! the way to reach the other: `docs/contracts/agent-prompt.md`, *The peer turn*, for a Drone, and
//! `docs/concepts/session.md`, *What Fleet tells a Session*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::standing::Peer;
use super::{Opposed, Side, Who};
use crate::daemon::Fleet;
use crate::peers::{listed, ClashWith, News};

/// What the clash is with, from the point of view of the one told.
enum About {
    Peer(Peer),
    Main { base: String, landed: Option<String> },
}

/// A Session's message: the words are Fleet's, the reach is the other side's.
fn session_says(about: &About, paths: &[String]) -> String {
    match about {
        About::Peer(Peer::Job { id, handle, title }) => format!(
            "Your changes to {} conflict with Job {handle} (\"{title}\"). Send its Drone a \
             message with redirect_drone, job_id {id}, to agree who changes what.",
            listed(paths)
        ),
        About::Peer(Peer::Session {
            address, branch, ..
        }) => {
            let on = branch
                .as_ref()
                .map(|branch| format!(" (branch {branch})"))
                .unwrap_or_default();
            format!(
                "Your changes to {} conflict with Session {address}{on}. Message it with \
                 SendMessage to {address} to agree who changes what.",
                listed(paths)
            )
        }
        About::Main { base, landed } => {
            let landed = landed
                .as_ref()
                .map(|landed| format!(", landed in {landed}"))
                .unwrap_or_default();
            format!(
                "{base} now conflicts with your changes to {}{landed}. Merge it into your \
                 branch and resolve the conflict.",
                listed(paths)
            )
        }
    }
}

/// A Drone's line, queued for its next peer turn.
fn drone_news(about: &About, paths: Vec<String>) -> News {
    let with = match about {
        About::Peer(Peer::Job { handle, title, .. }) => ClashWith::Job {
            title: title.clone(),
            handle: handle.clone(),
        },
        About::Peer(Peer::Session {
            address,
            title,
            branch,
            ..
        }) => ClashWith::Session {
            address: address.clone(),
            title: title.clone(),
            branch: branch.clone(),
        },
        About::Main { base, landed } => ClashWith::Main {
            base: base.clone(),
            landed: landed.clone(),
        },
    };
    News::Clash { paths, with }
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
    /// A clash is new, or its files changed: tell each holder in it.
    pub(crate) async fn clash_announced(self: &Arc<Self>, opposed: &Opposed) {
        self.clash_told(opposed, &opposed.0).await;
        if let Side::Holder(other) = &opposed.1 {
            self.clash_told(opposed, other).await;
        }
    }

    /// Anyone a standing clash has not reached yet, a Session whose terminal was not listening.
    pub(crate) async fn clashes_told(self: &Arc<Self>) {
        for (opposed, reading) in self.writes().untold() {
            for who in std::iter::once(&opposed.0).chain(match &opposed.1 {
                Side::Holder(other) => Some(other),
                Side::Main => None,
            }) {
                if !reading.told.contains(who) {
                    self.clash_told(&opposed, who).await;
                }
            }
        }
    }

    /// What `to` is told of one clash, from what Fleet kept of it. Marked told once it was queued
    /// for a Drone or a Session took the message.
    async fn clash_told(self: &Arc<Self>, opposed: &Opposed, to: &Who) {
        let Some(reading) = self.writes().reading(opposed) else {
            return;
        };
        let paths: Vec<String> = reading.paths.iter().cloned().collect();
        let about = match &opposed.1 {
            Side::Main => {
                let base = match self.writes().manifest_of(to) {
                    Some(manifest) => self
                        .repositories()
                        .serving(&manifest)
                        .and_then(|served| served.manifest().base().map(str::to_string)),
                    None => None,
                };
                About::Main {
                    base: base.unwrap_or_else(|| String::from("main")),
                    landed: reading.landed.clone(),
                }
            }
            Side::Holder(other) => {
                let from = if *to == opposed.0 { other } else { &opposed.0 };
                match self.peer_of(from).await {
                    Some(peer) => About::Peer(peer),
                    None => return,
                }
            }
        };
        let told = match to {
            Who::Job(job) => {
                self.owe(job, drone_news(&about, paths.clone())).await;
                true
            }
            Who::Session(id) => matches!(
                self.told_by_fleet(id, &session_says(&about, &paths)).await,
                Ok(true)
            ),
        };
        if told {
            self.writes().told(opposed, to);
        }
    }

    /// Notes a Job's Drone left Sessions, sent as messages from Fleet naming that Job.
    pub(crate) async fn notes_sent(self: &Arc<Self>) {
        for (session, text) in self.writes().notes_for_sessions() {
            // A Session that cannot hear it now has lost the note: the Drone may leave another.
            let _ = self.told_by_fleet(&session, &text).await;
        }
    }
}
