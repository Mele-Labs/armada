//! Servers a hosted job added to the agent CLI's files while its process ran, loaded into that
//! process with no restart. The CLI reads its files once, at start, and takes a server later only
//! through `mcp_set_servers`: `docs/spikes/040-can-a-running-job-load-a-server-it-added.md`.
//!
//! **Read when the Loadout reads the servers**, the owner's choice (18 Nov 2026): a stat of the
//! files on each read, and a read of them only once one has changed.
//!
//! **A set names only servers no file gave the process at start.** The CLI makes every name a set
//! gives its own, and disconnects it from the next set that leaves it out, so naming the door or a
//! server it loaded itself would hand that server to whatever this module reads next. A server
//! the process lists under any scope but `dynamic` came from its files, and the door is the one
//! `dynamic` server that was not set here.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::{Answered, ConfigPlaces, ConfigStamp, Configured, McpConfig, McpRequest, McpServer, McpState};
use api::Refusal;

use super::mcp_servers::SET_WITHIN;
use crate::daemon::Fleet;

/// The scope the CLI lists a server under once a set gave it.
const SET_SCOPE: &str = "dynamic";

/// What this Fleet set on one process. In memory, and begun again for each process, which reads
/// its files whole when it starts.
#[derive(Debug, Default)]
pub(crate) struct Added {
    generation: u64,
    /// The files as they stood when last read. `None` until they are.
    stamp: Option<ConfigStamp>,
    /// The last set sent, whole.
    sent: BTreeMap<String, McpConfig>,
    /// The servers the process would not take, with the scope their file gave and its words for why.
    refused: BTreeMap<String, (String, String)>,
}

impl Added {
    /// `listed`, with a row for each server the process would not take.
    fn rows(&self, mut listed: Vec<McpServer>) -> Vec<McpServer> {
        for (name, (scope, why)) in &self.refused {
            if listed.iter().any(|server| &server.name == name) {
                continue;
            }
            listed.push(McpServer {
                name: name.clone(),
                state: McpState::Failed,
                error: Some(format!(
                    "Added while this job ran, and its process would not load it: {why}. It loads when the job restarts."
                )),
                scope: Some(scope.clone()),
                tools: 0,
            });
        }
        listed
    }
}

/// The set to send: every server the files name that the process did not load from them itself.
fn wanted(configured: &BTreeMap<String, Configured>, listed: &[McpServer]) -> BTreeMap<String, McpConfig> {
    configured
        .iter()
        .filter(|(name, _)| name.as_str() != ipc::door::SERVER)
        .filter(|(name, _)| {
            !listed.iter().any(|server| &server.name == *name && server.scope.as_deref() != Some(SET_SCOPE))
        })
        .map(|(name, configured)| (name.clone(), configured.config.clone()))
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
    /// Job `id`'s servers as its process `listed` them, after loading any its files gained since it
    /// started.
    pub(crate) async fn with_servers_added(&self, id: &str, listed: Vec<McpServer>) -> Result<Vec<McpServer>, Refusal> {
        let runtime = self.hosts().of(id);
        let (directory, generation) = {
            let state = runtime.state();
            (state.directory.clone(), state.generation)
        };
        if directory.is_empty() {
            return Ok(listed);
        }
        let places = ConfigPlaces::of(Path::new(&self.host().home), Path::new(&directory));
        let stamp = places.stamp();
        {
            let mut state = runtime.state();
            if state.mcp_added.generation != generation {
                state.mcp_added = Added { generation, ..Added::default() };
            }
            if state.mcp_added.stamp.as_ref() == Some(&stamp) {
                return Ok(state.mcp_added.rows(listed));
            }
        }
        // A file caught mid-write reads as nothing known, never as no servers, which would send a
        // set that disconnects them. The next read tries again.
        let Ok(configured) = places.read() else {
            let rows = runtime.state().mcp_added.rows(listed);
            return Ok(rows);
        };
        let wanted = wanted(&configured, &listed);
        if wanted == runtime.state().mcp_added.sent {
            let mut state = runtime.state();
            state.mcp_added.stamp = Some(stamp);
            return Ok(state.mcp_added.rows(listed));
        }
        let refused: BTreeMap<String, String> = match self.asked(id, McpRequest::Set(&wanted), SET_WITHIN).await? {
            Ok(Answered::Set(answer)) => answer.refused,
            Ok(_) => BTreeMap::new(),
            // An older CLI that has no such request refuses all of it.
            Err(why) => wanted.keys().map(|name| (name.clone(), why.clone())).collect(),
        };
        {
            let mut state = runtime.state();
            if state.generation != generation {
                return Ok(listed);
            }
            let added = &mut state.mcp_added;
            added.stamp = Some(stamp);
            added.refused = refused
                .into_iter()
                .filter_map(|(name, why)| {
                    let scope = configured.get(&name)?.scope.to_string();
                    Some((name, (scope, why)))
                })
                .collect();
            added.sent = wanted;
        }
        let listed = self.servers_listed(id).await?;
        let rows = runtime.state().mcp_added.rows(listed);
        Ok(rows)
    }
}
