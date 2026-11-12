//! The Job at the other end of a message on a thread: `SendMessage`'s `to`, and a received
//! message's `from-name` or socket. `docs/practices/protocol.md`, *A tool row's picture and message*.
//!
//! **Resolved where Fleet writes or takes the row, and never where it is served.** A hosted Job's row
//! is stored and published by `row_put`, so the event and a later `get_session` carry the same peer;
//! a terminal Job's rows are never written, so they are resolved where Fleet takes them off the
//! transcript (`terminal_thread` and `watch_terminal`). A name Fleet cannot place stays as written.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{SessionId, SessionRow, SessionVoice, ToolMessage};

use super::address_of;
use crate::daemon::Fleet;

/// A Job a name was resolved to.
#[derive(Clone)]
struct Peer {
    id: String,
    title: Option<String>,
}

/// The eight hex characters of an `s-<8 hex>` address.
fn address_hex(name: &str) -> Option<&str> {
    let hex = name.strip_prefix("s-")?;
    (hex.len() == 8 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(hex)
}

/// The pid in a `uds:/tmp/cc-socks/<pid>.sock` address, which is the CLI's own messaging socket.
fn socket_pid(name: &str) -> Option<u32> {
    let path = name.strip_prefix("uds:").unwrap_or(name);
    let (folder, file) = path.rsplit_once('/')?;
    if !folder.ends_with("/cc-socks") {
        return None;
    }
    file.strip_suffix(".sock")?.parse().ok()
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
    /// Name the Job on each row's other end: the peer of a `SendMessage` row and the voice of a message
    /// a terminal Job received. **A row whose name does not place a Job is left as it is.**
    pub(crate) async fn peers_named(&self, rows: &mut [SessionRow]) {
        let mut known: HashMap<String, Option<Peer>> = HashMap::new();
        for row in rows {
            let name = match row {
                SessionRow::Tool { message: Some(ToolMessage { to, .. }), .. } if to.session_id.is_none() => {
                    to.name.clone()
                }
                SessionRow::Message { from: SessionVoice::Session { id, .. }, .. } => id.clone(),
                _ => continue,
            };
            let peer = match known.get(&name) {
                Some(peer) => peer.clone(),
                None => {
                    let peer = self.job_named(&name).await;
                    known.insert(name, peer.clone());
                    peer
                }
            };
            let Some(peer) = peer else { continue };
            match row {
                SessionRow::Tool { message: Some(message), .. } => {
                    message.to.session_id = Some(SessionId::carried(&peer.id));
                    message.to.title = peer.title;
                }
                SessionRow::Message { from: SessionVoice::Session { id, title }, .. } => {
                    *title = peer.title.unwrap_or_else(|| address_of(&peer.id));
                    *id = peer.id;
                }
                _ => {}
            }
        }
    }

    /// The Job `name` addresses. `s-<8 hex>` is the Job whose id begins with those eight. A socket
    /// path is the Job whose **hosted agent** has the pid in its file name: Fleet keeps a pid for
    /// the agent it hosts and for no terminal Job, so a terminal Job is found by its address only.
    async fn job_named(&self, name: &str) -> Option<Peer> {
        if let Some(hex) = address_hex(name) {
            let store = self.store().lock().await;
            let kept = store.sessions_beginning(hex).ok()?;
            return match kept.as_slice() {
                [one] => Some(Peer { id: one.id.clone(), title: one.title.clone() }),
                _ => None,
            };
        }
        let pid = socket_pid(name)?;
        let id = self
            .hosts()
            .all()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| self.hosts().of(id).state().process.as_ref().and_then(|one| one.pid()) == Some(pid))?;
        let title = self.store().lock().await.session(&id).ok().flatten().and_then(|one| one.title);
        Some(Peer { id, title })
    }
}

#[cfg(test)]
mod tests {
    use super::{address_hex, socket_pid};

    #[test]
    fn an_address_is_eight_hex_after_s() {
        assert_eq!(address_hex("s-76da9937"), Some("76da9937"));
        assert_eq!(address_hex("s-76da993"), None);
        assert_eq!(address_hex("s-76da99370"), None);
        assert_eq!(address_hex("s-76da993g"), None);
        assert_eq!(address_hex("a2f35e976899d156f"), None);
    }

    #[test]
    fn a_socket_is_the_pid_in_its_file_name() {
        assert_eq!(socket_pid("uds:/tmp/cc-socks/69566.sock"), Some(69566));
        assert_eq!(socket_pid("/tmp/cc-socks/5352.sock"), Some(5352));
        assert_eq!(socket_pid("uds:/tmp/other/5352.sock"), None);
        assert_eq!(socket_pid("uds:/tmp/cc-socks/fixture.sock"), None);
    }
}
