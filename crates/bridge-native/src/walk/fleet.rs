//! What a walk tells Fleet: Approve, the notes, and the window closed. **These three routes and no
//! others**, on the port Fleet handed over with `--fleet-port`, with no protocol check: a Job's
//! branch may have changed the wire, and its own walk must still reach the owner's Fleet. Fleet
//! pins their shapes in `crates/fleet/src/tests/native_walk.rs`, so a change to one fails there.

use ipc::{
    NoteCrop, NoteElement, NotePlace, NoteState, NoteTake, NoteWalk, SendSessionMessage, SendSessionMessageBody,
    SendSessionNotes, SessionId, SessionNote, WindowClosed, WireError,
};

/// Who the walk answers to: the Job that showed it, the address it showed it on, and Fleet's port.
#[derive(Clone, Debug)]
pub struct WalkFleet {
    pub job: String,
    pub url: String,
    pub port: u16,
}

/// The take on screen, its letter and name, as the bar shows it: `A`, `Pennant`.
pub type Taken = (String, String);

/// A note and where it was taken: the walk, the step, and what it was written on.
pub struct NoteOn {
    pub said: String,
    /// The take it was written on, where the walk has takes.
    pub take: Option<Taken>,
    pub walk: &'static str,
    pub step: u32,
    pub of: u32,
    pub role: String,
    pub name: String,
    /// The window's picture with what the note was written on outlined and numbered, PNG in base64.
    pub picture: Option<String>,
}

/// The line Approve sends, as Electron's `approvedText`: `Approved A, Pennant: <address>`.
fn approved(url: &str, take: Option<&Taken>) -> String {
    match take {
        Some((letter, name)) => format!("Approved {letter}, {name}: {url}"),
        None => format!("Approved: {url}"),
    }
}

impl WalkFleet {
    /// Approve, as Electron's bar sends it: the line Fleet's `walk_approval` reads, naming the
    /// take on screen where the walk has takes.
    pub fn approve(&self, take: Option<&Taken>) -> Result<(), String> {
        let body = SendSessionMessageBody {
            message: SendSessionMessage {
                session_id: SessionId::carried(self.job.clone()),
                text: approved(&self.url, take),
                attachments: Vec::new(),
                mentions: Vec::new(),
                sketches: Vec::new(),
            },
            aim: None,
        };
        self.send("/sessions/message", &body)
    }

    /// The held notes, in the order they were taken: Fleet gives them to the Job as one turn,
    /// numbered so.
    pub fn notes(&self, notes: Vec<NoteOn>) -> Result<(), String> {
        let notes = notes
            .into_iter()
            .map(|on| SessionNote {
                said: on.said,
                address: self.url.clone(),
                note_id: None,
                place: NotePlace {
                    selector: format!("{} \"{}\"", on.role, on.name),
                    element: NoteElement { tag: on.role.clone(), text: on.name.clone(), label: Some(on.name.clone()) },
                    location: format!("{}#step-{}", self.url, on.step),
                    ..NotePlace::default()
                },
                state: NoteState {
                    walk: Some(NoteWalk { name: on.walk.to_string(), step: on.step, of: on.of }),
                    take: on.take.map(|(letter, name)| NoteTake { letter, name }),
                    ..NoteState::default()
                },
                crop: on.picture.map(|data| NoteCrop { media_type: "image/png".into(), data }),
                job: Vec::new(),
            })
            .collect();
        self.send("/sessions/notes", &SendSessionNotes { session_id: SessionId::carried(self.job.clone()), notes })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_names_the_take_on_screen_as_electrons_bar_does() {
        let take = ("B".to_string(), "Brackets".to_string());
        assert_eq!(approved("armada://walk/jobs-lamps", Some(&take)), "Approved B, Brackets: armada://walk/jobs-lamps");
        assert_eq!(approved("armada://walk/jobs-list", None), "Approved: armada://walk/jobs-list");
    }
}
