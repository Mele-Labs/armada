//! What a walk tells Fleet: Approve, a note, and the window closed. **These three routes and no
//! others**, on the port Fleet handed over with `--fleet-port`, with no protocol check: a Job's
//! branch may have changed the wire, and its own walk must still reach the owner's Fleet. Fleet
//! pins their shapes in `crates/fleet/src/tests/native_walk.rs`, so a change to one fails there.

use ipc::{
    NoteElement, NotePlace, NoteState, NoteWalk, SendSessionMessage, SendSessionMessageBody,
    SendSessionNote, SessionId, SessionNote, WindowClosed, WireError,
};

/// Who the walk answers to: the Job that showed it, the address it showed it on, and Fleet's port.
#[derive(Clone, Debug)]
pub struct WalkFleet {
    pub job: String,
    pub url: String,
    pub port: u16,
}

/// Where a note was taken: the walk, the step, and the element the step was on.
pub struct NoteOn<'a> {
    pub walk: &'a str,
    pub step: u32,
    pub of: u32,
    pub role: &'a str,
    pub name: &'a str,
}

impl WalkFleet {
    /// Approve, as Electron's bar sends it: the line Fleet's `walk_approval` reads.
    pub fn approve(&self) -> Result<(), String> {
        let body = SendSessionMessageBody {
            message: SendSessionMessage {
                session_id: SessionId::carried(self.job.clone()),
                text: format!("Approved: {}", self.url),
                attachments: Vec::new(),
                mentions: Vec::new(),
            },
            aim: None,
        };
        self.send("/sessions/message", &body)
    }

    pub fn note(&self, said: &str, on: NoteOn<'_>) -> Result<(), String> {
        let body = SendSessionNote {
            session_id: SessionId::carried(self.job.clone()),
            note: SessionNote {
                said: said.to_string(),
                address: self.url.clone(),
                note_id: None,
                place: NotePlace {
                    selector: format!("{} \"{}\"", on.role, on.name),
                    element: NoteElement { tag: on.role.to_string(), text: on.name.to_string(), label: Some(on.name.to_string()) },
                    location: format!("{}#step-{}", self.url, on.step),
                    ..NotePlace::default()
                },
                state: NoteState {
                    walk: Some(NoteWalk { name: on.walk.to_string(), step: on.step, of: on.of }),
                    ..NoteState::default()
                },
                crop: None,
                job: Vec::new(),
            },
        };
        self.send("/sessions/note", &body)
    }

    pub fn closed(&self) -> Result<(), String> {
        let body = WindowClosed { session_id: SessionId::carried(self.job.clone()), url: self.url.clone() };
        self.send("/sessions/window/closed", &body)
    }

    /// One POST on a runtime of its own, so a walk needs no connection to Fleet's events. A
    /// refusal answers Fleet's own sentence.
    fn send<T: serde::Serialize>(&self, path: &str, body: &T) -> Result<(), String> {
        let text = ipc::encode(body).map_err(|why| why.to_string())?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = runtime.block_on(crate::transport::post(self.port, path, text))?;
        if (200..300).contains(&status) {
            return Ok(());
        }
        Err(ipc::decode::<WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
    }
}
