//! Saying which files clash and who the other side is, to each agent it concerns. Bare facts, with
//! the way to reach the other: `docs/contracts/agent-prompt.md`, *The peer turn*, for a Drone, and
//! `docs/concepts/session.md`, *What Fleet tells a Session*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::standing::Peer;
use super::{Opposed, Side, Who};
use crate::daemon::Fleet;

/// The first three paths, and a count for the rest.
pub(crate) fn listed(paths: &[String]) -> String {
    const NAMED: usize = 3;
    let named = paths
        .iter()
        .take(NAMED)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    match paths.len().saturating_sub(NAMED) {
        0 => named,
        rest => format!("{named} and {rest} more"),
    }
}

/// What the clash is with, from the point of view of the one told.
enum About {
    Peer(Peer),
    Main { base: String, landed: Option<String> },
}

/// A Session's message: the words are Fleet's, the reach is the other side's.
fn session_says(about: &About, paths: &[String]) -> String {
    match about {
        About::Peer(Peer::Session {
            address, branch, paused, ..
        }) => {
            let with = match (branch, paused) {
                (Some(branch), true) => format!("branch {branch} (Job {address}, paused)"),
                (Some(branch), false) => format!("branch {branch} (Job {address})"),
                (None, true) => format!("Job {address} (paused)"),
                (None, false) => format!("Job {address}"),
            };
            format!("Git says your changes and {with} write the same lines of {}.", listed(paths))
        }
        About::Main { base, landed } => {
            let landed = landed
                .as_ref()
                .map(|landed| format!(", landed in {landed}"))
                .unwrap_or_default();
            format!("Git says {base} and your changes write the same lines of {}{landed}.", listed(paths))
        }
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
    /// for a Drone or a Session took the message. **Words sent before are not sent again**: the
    /// table is memory, and a restart refilled it and told a Job the same clash three times.
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
                    Some(Peer::Session { id, address, title, branch, paused }) => {
                        let branch = reading.branches.get(from).cloned().or(branch);
                        About::Peer(Peer::Session { id, address, title, branch, paused })
                    }
                    None => return,
                }
            }
        };
        let told = match to {
            Who::Session(id) => {
                // **A file is told once per peer**: a later commit that adds another file to the
                // overlap names that one, never the files the Jobs were told of and have settled.
                // Main is a peer of its own, named by its base.
                let peer = match &about {
                    About::Peer(Peer::Session { id: peer, .. }) => peer.clone(),
                    About::Main { base, .. } => format!("main:{base}"),
                };
                let before = self.store().lock().await.clash_paths_told(id, &peer).unwrap_or_default();
                let paths: Vec<String> = paths.into_iter().filter(|path| !before.contains(path)).collect();
                if paths.is_empty() {
                    self.writes().told(opposed, to);
                    return;
                }
                let said = session_says(&about, &paths);
                if matches!(self.store().lock().await.clash_told_before(id, &said), Ok(true)) {
                    true
                } else {
                    let sent = matches!(self.told_by_fleet(id, &said, ipc::Notice::WriteClash).await, Ok(true));
                    if sent {
                        let mut store = self.store().lock().await;
                        let _ = store.keep_clash_told(id, &said, &self.now());
                        let _ = store.keep_clash_paths_told(id, &peer, &paths, &self.now());
                    }
                    sent
                }
            }
        };
        if told {
            self.writes().told(opposed, to);
        }
    }
}
