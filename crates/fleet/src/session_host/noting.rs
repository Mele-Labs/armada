//! `send_session_note`: a note the person pinned on a page, sent to a Session whole.
//!
//! **Fleet finds the lines first, then sends as `send_session_message` sends**: the thread keeps
//! the person's words, and where he pointed, the candidates and the page's state are lines Fleet
//! adds after them, as a mention's line is added ahead.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{NoteSent, NoteState, SendSessionMessage, SendSessionNote, SessionNote, SessionUpload};

use crate::daemon::Fleet;
use crate::note_locating::{located, runs_of, Located, StampRead};

/// A file larger than this is not a page's source.
const LARGEST_READ: u64 = 1024 * 1024;

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
    pub(crate) async fn note_sent(self: Arc<Self>, sent: SendSessionNote) -> Result<NoteSent, Refusal> {
        let SendSessionNote { session_id, note } = sent;
        let id = session_id.as_str().to_string();
        let terminal = self.terminal_session(&id).await?;
        let checkout = self.note_checkout(&id, &note.address).await;
        let found = match &checkout {
            Some(checkout) => {
                let (checkout, place) = (checkout.clone(), note.place.clone());
                tokio::task::spawn_blocking(move || searched(&checkout, &place))
                    .await
                    .unwrap_or_else(|_| Located { candidates: Vec::new(), stamp: StampRead::Absent })
            }
            None => Located { candidates: Vec::new(), stamp: StampRead::Absent },
        };
        let lines = told(&note, &found, checkout.as_deref());
        let sent = SendSessionMessage {
            session_id,
            text: note.said.trim().to_string(),
            attachments: note
                .crop
                .iter()
                .map(|crop| SessionUpload {
                    name: "pinned-note.png".to_string(),
                    media_type: crop.media_type.clone(),
                    data: crop.data.clone(),
                })
                .collect(),
            mentions: Vec::new(),
        };
        let session = Arc::clone(&self).send_known_added(terminal, sent, None, Some(lines)).await?;
        // A note taken in a window: the session will now change the page.
        if note.address.starts_with("http://") || note.address.starts_with("https://") {
            self.window_noted(&id).await;
        }
        Ok(NoteSent { session, candidates: found.candidates })
    }

    /// The checkout that served `address`: the one a server Fleet holds answers on its port, else
    /// the Session's slot, else its repository's root.
    async fn note_checkout(&self, session: &str, address: &str) -> Option<PathBuf> {
        if let Some(port) = port_of(address) {
            let serving = self
                .servers()
                .every()
                .into_iter()
                .find(|one| one.phase != ipc::ServerPhase::Exited && one.ports.iter().any(|held| held.port == port));
            if let Some(server) = serving {
                return Some(PathBuf::from(server.checkout.path));
            }
        }
        let who = crate::clashing::Who::Session(session.to_string());
        if let Some(slot) = self.writing_in(&who).await.and_then(|standing| standing.worktrees.into_iter().next()) {
            return Some(slot);
        }
        let manifest = self.store().lock().await.session(session).ok().flatten()?.manifest_id?;
        self.repositories().serving(&manifest).map(|served| PathBuf::from(served.root()))
    }
}

/// The port `address` was loaded from, its scheme's own where it names none.
fn port_of(address: &str) -> Option<u16> {
    let (scheme, rest) = address.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    match host.rsplit_once(':') {
        Some((_, port)) if !port.ends_with(']') => port.parse().ok(),
        _ => match scheme {
            "http" => Some(80),
            "https" => Some(443),
            _ => None,
        },
    }
}

/// The files of `checkout` holding the screen's words, and the stamp's own wherever it exists,
/// ranked. A search git would not answer finds nothing, and the note is still sent.
fn searched(checkout: &Path, place: &ipc::NotePlace) -> Located {
    let runs: Vec<String> = runs_of(place).into_iter().map(|run| run.words).collect();
    let mut paths = adapters::source_search::files_holding(checkout, &runs).unwrap_or_default();
    let stamped = place
        .stamp
        .as_deref()
        .and_then(|stamp| stamp.rsplit_once(':'))
        .map(|(path, _)| path.trim_start_matches("./").to_string())
        .filter(|path| Path::new(path).components().all(|part| matches!(part, Component::Normal(_))));
    if let Some(path) = stamped {
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    let files: Vec<(String, String)> = paths
        .into_iter()
        .filter_map(|path| {
            let at = checkout.join(&path);
            let size = std::fs::metadata(&at).ok().filter(|meta| meta.is_file())?.len();
            (size <= LARGEST_READ).then_some(())?;
            Some((path, std::fs::read_to_string(&at).ok()?))
        })
        .collect();
    located(place, &files)
}

/// How to deliver is the owner's answer to the delivery question, never something to assume here.
const HANDOFF: &str = "Handoff: deliver the way the owner chose when Fleet asked how. A change the owner would see waits for his walk before it lands.";

/// What the agent reads after the person's words.
fn told(note: &SessionNote, found: &Located, checkout: Option<&Path>) -> String {
    let place = &note.place;
    let element = &place.element;
    let named = element.label.as_deref().filter(|label| !label.trim().is_empty()).unwrap_or(&element.text);
    let mut lines = vec![format!(
        "Pointed at: {} \"{}\"{}",
        element.tag,
        named.trim(),
        note.note_id.as_deref().map(|id| format!(" (note {id})")).unwrap_or_default()
    )];
    if found.candidates.is_empty() {
        let searched = checkout.map_or_else(|| "no checkout".to_string(), |at| at.display().to_string());
        lines.push(format!("No line on screen matched the code in {searched}"));
    } else {
        lines.push("Candidates:".to_string());
        for (at, one) in found.candidates.iter().enumerate() {
            lines.push(format!("{}. {}:{} — {}", at + 1, one.file, one.line, one.why));
            lines.push(format!("   {}", one.text));
        }
    }
    if let StampRead::Dropped(why) = &found.stamp {
        lines.push(format!("Stamp: {why}"));
    }
    if let Some(state) = state_line(&note.state) {
        lines.push(format!("State: {state}"));
    }
    lines.push(format!("Page: {}", note.address));
    let chain: Vec<&str> = place.component.iter().chain(place.owners.iter()).map(String::as_str).collect();
    let mut inside = Vec::new();
    if !place.nearby.is_empty() {
        inside.push(place.nearby.join(" ← "));
    }
    if !chain.is_empty() {
        inside.push(format!("component chain {}", chain.join(" ← ")));
    }
    if !inside.is_empty() {
        lines.push(format!("Inside: {}", inside.join("; ")));
    }
    lines.push(HANDOFF.to_string());
    lines.join("\n")
}

fn state_line(state: &NoteState) -> Option<String> {
    let mut parts = Vec::new();
    if !state.selected.is_empty() {
        parts.push(format!("{} selected", state.selected.join(", ")));
    }
    if let Some(walk) = &state.walk {
        parts.push(format!("walk {} step {} of {}", walk.name, walk.step, walk.of));
    }
    if let Some(take) = &state.take {
        parts.push(format!("take {}, {}", take.letter, take.name));
    }
    if let Some(scenario) = &state.scenario {
        parts.push(format!("scenario {scenario}"));
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}
