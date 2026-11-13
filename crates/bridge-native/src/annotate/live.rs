//! Notes on their way to a live Job, as Classic's layer sends them (`shared/annotations.ts`
//! `requestOf`): notes on an element or a region go as one turn of notes, each with the window's
//! picture ringed and numbered; a note on the whole window goes as a message with the window attached.
//! Everything here blocks, and runs off the main thread.

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use gpui::{Bounds, Pixels};
use ipc::{DispatchJob, DispatchMessage, DispatchThen, JobDispatched, ManifestId, NoteCrop, NoteElement, NotePlace, NoteState, SendSessionMessage, SendSessionMessageBody, SendSessionNotes, SessionId, SessionNote, SessionUpload, StartSession};
use serde_json::Value;

/// One note as the send needs it: its file as it reads, and where its picture is.
pub struct Outgoing {
    pub key: String,
    pub raw: Value,
    pub window: bool,
    /// Its place among the notes sent together, the number burned beside its ring.
    pub number: usize,
    pub picture: Picture,
}

pub enum Picture {
    None,
    /// The window as it was when the note was held, its pixels per point, and the ring.
    Held(Arc<image::RgbaImage>, f32, Option<Bounds<Pixels>>),
    /// A note read back from disk: the picture saved beside it.
    File(PathBuf),
}

/// The ring's ink and the number's, as the walk's send takes them.
#[derive(Clone, Copy)]
pub struct Ink(pub [u8; 4], pub [u8; 4]);

fn text(raw: &Value, name: &str) -> String {
    raw[name].as_str().unwrap_or_default().to_string()
}

/// What he changed with the card's tools, as Classic's `changeLines`.
fn change_lines(raw: &Value) -> Vec<String> {
    let Some(changes) = raw["changes"].as_array().filter(|changes| !changes.is_empty()) else { return Vec::new() };
    let quoted = |kind: &str, value: &Value| {
        let value = value.as_str().unwrap_or_default();
        if kind == "text" {
            format!("{value:?}")
        } else if value.is_empty() {
            "none".to_string()
        } else {
            value.to_string()
        }
    };
    let mut lines = vec![format!("Changes he made on the page to show what he wants, on {}:", text(raw, "selector"))];
    for change in changes {
        let kind = change["kind"].as_str().unwrap_or_default();
        let token = change["token"].as_str().map(|token| format!(" ({token})")).unwrap_or_default();
        lines.push(format!("- {}: {} → {}{token}", change["property"].as_str().unwrap_or_default(), quoted(kind, &change["before"]), quoted(kind, &change["after"])));
    }
    lines
}

/// A whole-window note's message, as Classic's `windowRequestOf`.
fn window_text(raw: &Value, key: &str) -> String {
    let mut lines = vec![
        text(raw, "text"),
        String::new(),
        "This is feedback on the state of Bridge's whole window, left with the annotation layer. The screenshot is the window.".to_string(),
        format!("- Screen: {}", raw["screen"].as_str().unwrap_or("none marked")),
    ];
    if let Some(layer) = raw["layer"].as_str() {
        lines.push(format!("- Inside: {layer}"));
    }
    lines.push(format!("- Location: {}", raw["location"].as_str().unwrap_or("/")));
    lines.push(format!("- Window: {}×{}", raw["window"]["width"].as_f64().unwrap_or(0.), raw["window"]["height"].as_f64().unwrap_or(0.)));
    lines.push(format!("- Note file: .armada/annotations/{key}.json"));
    lines.join("\n")
}

/// The picture as base64 PNG: ringed and numbered for a note on a spot, whole for one on the window.
fn picture_of(out: &Outgoing, ink: Ink) -> Option<String> {
    match &out.picture {
        Picture::None => None,
        Picture::Held(picture, scale, Some(ring)) if !out.window => crate::capture::ringed(picture, *ring, *scale, out.number, ink.0, ink.1),
        Picture::Held(picture, _, _) => {
            let mut png = std::io::Cursor::new(Vec::new());
            picture.write_to(&mut png, image::ImageFormat::Png).ok()?;
            Some(base64::engine::general_purpose::STANDARD.encode(png.into_inner()))
        }
        Picture::File(path) => std::fs::read(path).ok().map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes)),
    }
}

fn note_of(out: &Outgoing, ink: Ink) -> SessionNote {
    let raw = &out.raw;
    let mut said = vec![text(raw, "text").trim().to_string()];
    let changed = change_lines(raw);
    if !changed.is_empty() {
        said.push(String::new());
        said.extend(changed);
    }
    let location = raw["location"].as_str().unwrap_or("/").to_string();
    let screen = raw["screen"].as_str().map(str::to_string);
    SessionNote {
        said: said.join("\n"),
        address: format!("armada://native/{}", screen.as_deref().unwrap_or("window")),
        note_id: Some(out.key.clone()),
        place: NotePlace {
            component: raw["component"].as_str().map(str::to_string),
            selector: text(raw, "selector"),
            element: NoteElement {
                tag: text(&raw["element"], "tag"),
                text: text(&raw["element"], "text").chars().take(200).collect(),
                label: raw["element"]["label"].as_str().map(str::to_string),
            },
            location,
            screen,
            layer: raw["layer"].as_str().map(str::to_string),
            ..NotePlace::default()
        },
        state: NoteState { scenario: raw["scenario"].as_str().map(str::to_string), ..NoteState::default() },
        crop: picture_of(out, ink).map(|data| NoteCrop { media_type: "image/png".into(), data }),
        job: Vec::new(),
    }
}

fn post(port: u16, path: &str, body: Result<String, ipc::Unencodable>) -> Result<Vec<u8>, String> {
    let body = body.map_err(|why| format!("{why:?}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(crate::transport::post(port, path, body))?;
    if (200..300).contains(&status) {
        Ok(bytes)
    } else {
        Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
    }
}

/// The notes into Job `session`'s thread: the ones on a spot as one turn, then each one on the window.
pub fn deliver(port: u16, session: &str, notes: Vec<Outgoing>, ink: Ink) -> Result<(), String> {
    let (window, spots): (Vec<Outgoing>, Vec<Outgoing>) = notes.into_iter().partition(|note| note.window);
    if !spots.is_empty() {
        let notes = spots.iter().map(|out| note_of(out, ink)).collect();
        post(port, "/sessions/notes", ipc::encode(&SendSessionNotes { session_id: SessionId::carried(session.to_string()), notes }))?;
    }
    for out in &window {
        let attachments = picture_of(out, ink).map(|data| SessionUpload { name: format!("annotation-{}.png", out.key), media_type: "image/png".into(), data }).into_iter().collect();
        let message = SendSessionMessage { session_id: SessionId::carried(session.to_string()), text: window_text(&out.raw, &out.key), attachments, mentions: Vec::new(), sketches: Vec::new() };
        post(port, "/sessions/message", ipc::encode(&SendSessionMessageBody { message, aim: None }))?;
    }
    Ok(())
}

/// What `dispatch_job` made of a new Job's notes.
pub enum Dispatched {
    /// A Job exists, with the notes sent into it.
    Started { id: String, title: String },
    /// The Operator holds it, or kept it as an idea: the id `operator.dispatched` will name it by, and the words' first line.
    Kept { id: String, title: String },
}

/// A new Job on `manifest` with the notes' words as its request, as one dispatch: the notes go in `then`, a turn of
/// notes for the ones on a spot and a message for each on the window, so nothing is lost whoever starts the Job.
/// The Job's id and its title as Fleet named it, else the words' first line cut to 60 characters, as Classic's.
pub fn dispatch(port: u16, manifest: ManifestId, words: String, notes: Vec<Outgoing>, ink: Ink) -> Result<Dispatched, String> {
    let (window, spots): (Vec<Outgoing>, Vec<Outgoing>) = notes.into_iter().partition(|note| note.window);
    let mut then = Vec::new();
    if !spots.is_empty() {
        then.push(DispatchThen::Notes(spots.iter().map(|out| note_of(out, ink)).collect()));
    }
    for out in &window {
        let attachments = picture_of(out, ink).map(|data| SessionUpload { name: format!("annotation-{}.png", out.key), media_type: "image/png".into(), data }).into_iter().collect();
        then.push(DispatchThen::Message(DispatchMessage { text: window_text(&out.raw, &out.key), attachments, mentions: Vec::new(), sketches: Vec::new() }));
    }
    let start = StartSession {
        manifest_id: manifest,
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some(words.clone()),
        after: None,
        held: false,
        project_rows: Vec::new(),
        project: None,
        on_your_words: None,
    };
    let bytes = post(port, "/sessions/dispatch", ipc::encode(&DispatchJob { start, kind: None, then }))?;
    let done = ipc::decode::<JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())?;
    let first_line: String = words.lines().next().unwrap_or_default().chars().take(60).collect();
    match done {
        JobDispatched { session: Some(record), .. } => {
            let title = record.title.clone().unwrap_or(first_line);
            Ok(Dispatched::Started { id: record.id.as_str().to_string(), title })
        }
        JobDispatched { operator, parked, .. } => match operator.or(parked) {
            Some(id) => Ok(Dispatched::Kept { id, title: first_line }),
            None => Err("Fleet answered the dispatch with nothing in it".to_string()),
        },
    }
}
