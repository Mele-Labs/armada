//! Notes on their way to a live Job, as Classic's layer sends them (`shared/annotations.ts`
//! `requestOf`): notes on an element or a region go as one turn of notes, each with the window's
//! picture ringed and numbered; a note on the whole window goes as a message with the window attached.
//! Everything here blocks, and runs off the main thread.

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use gpui::{Bounds, Pixels};
use ipc::{
    DispatchJob, DispatchMessage, DispatchThen, JobDispatched, ManifestId, NoteCrop, NoteElement, NotePlace, NoteState, NotesRouted, SendNotesToFeatures, SendSessionMessage, SendSessionMessageBody,
    SendSessionNotes, SessionId, SessionNote, SessionUpload, StartSession,
};
use serde_json::Value;

use crate::refusal::Failed;

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

/// What a note's file keeps of a page's parts, as `sessionNoteBody` sends them: at most this many of each.
const MOST_OWNERS: usize = 16;
const MOST_NEARBY: usize = 8;
const MOST_SELECTED: usize = 12;

/// The strings of a list in a note's file, at most `most`.
fn strings(value: &Value, most: usize) -> Vec<String> {
    value.as_array().map(|all| all.iter().filter_map(|one| one.as_str().map(str::to_string)).take(most).collect()).unwrap_or_default()
}

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
    // The line the hooks read a feature's page by (`docs/concepts/feature-catalog.md`, Handing a Job its pages).
    let features = super::beside::slugs_of(&out.key);
    if !features.is_empty() {
        said.push(String::new());
        said.extend(features.iter().take(crate::features::of_path::MOST).map(|slug| format!("Feature: {slug}")));
    }
    let location = raw["location"].as_str().unwrap_or("/").to_string();
    let screen = raw["screen"].as_str().map(str::to_string);
    // A note on a Job's page carries the page's own address; any other is Bridge's own screen's.
    let address = raw["address"].as_str().map(str::to_string).unwrap_or_else(|| format!("armada://native/{}", screen.as_deref().unwrap_or("window")));
    SessionNote {
        said: said.join("\n"),
        address,
        note_id: Some(out.key.clone()),
        place: NotePlace {
            component: raw["component"].as_str().map(str::to_string),
            owners: strings(&raw["owners"], MOST_OWNERS),
            selector: text(raw, "selector"),
            element: NoteElement {
                tag: text(&raw["element"], "tag"),
                text: text(&raw["element"], "text").chars().take(200).collect(),
                label: raw["element"]["label"].as_str().map(str::to_string),
            },
            nearby: strings(&raw["nearby"], MOST_NEARBY),
            stamp: raw["stamp"].as_str().map(str::to_string),
            location,
            screen,
            layer: raw["layer"].as_str().map(str::to_string),
        },
        state: NoteState { selected: strings(&raw["selected"], MOST_SELECTED), scenario: raw["scenario"].as_str().map(str::to_string), ..NoteState::default() },
        crop: picture_of(out, ink).map(|data| NoteCrop { media_type: "image/png".into(), data }),
        job: Vec::new(),
    }
}

/// One POST of the send; a refusal keeps its route, what was sent and how many pictures went with it.
fn post(port: u16, path: &str, body: Result<String, ipc::Unencodable>, pictures: usize) -> Result<Vec<u8>, Failed> {
    let body = body.map_err(|why| format!("{why:?}"))?;
    match crate::refusal::post(port, path, body) {
        Err(Failed::Refused(refusal)) => Err(Failed::Refused(refusal.pictures(pictures))),
        answer => answer,
    }
}

fn pictured(notes: &[Outgoing]) -> usize {
    notes.iter().filter(|out| !matches!(out.picture, Picture::None)).count()
}

/// The notes into Job `session`'s thread: the ones on a spot as one turn, then each one on the window.
pub fn deliver(port: u16, session: &str, notes: Vec<Outgoing>, ink: Ink) -> Result<(), Failed> {
    let (window, spots): (Vec<Outgoing>, Vec<Outgoing>) = notes.into_iter().partition(|note| note.window);
    if !spots.is_empty() {
        let notes = spots.iter().map(|out| note_of(out, ink)).collect();
        post(port, "/sessions/notes", ipc::encode(&SendSessionNotes { session_id: SessionId::carried(session.to_string()), notes }), pictured(&spots))?;
    }
    for out in &window {
        let attachments = picture_of(out, ink).map(|data| SessionUpload { name: format!("annotation-{}.png", out.key), media_type: "image/png".into(), data }).into_iter().collect();
        let message = SendSessionMessage { session_id: SessionId::carried(session.to_string()), text: window_text(&out.raw, &out.key), attachments, mentions: Vec::new(), sketches: Vec::new() };
        post(port, "/sessions/message", ipc::encode(&SendSessionMessageBody { message, aim: None }), pictured(std::slice::from_ref(out)))?;
    }
    Ok(())
}

/// The notes to the live Jobs on the features the files `paths` draw, as one send: Fleet sends each Job the notes, a note
/// on the window as a note too, or starts a Job with `words` where none is live.
pub fn to_features(port: u16, manifest: ManifestId, project: String, paths: Vec<String>, words: String, notes: Vec<Outgoing>, ink: Ink) -> Result<NotesRouted, Failed> {
    let pictures = pictured(&notes);
    let notes = notes.iter().map(|out| note_of(out, ink)).collect();
    let sent = SendNotesToFeatures { manifest_id: manifest, project: Some(project), paths, words, notes };
    let bytes = post(port, "/sessions/notes_to_features", ipc::encode(&sent), pictures)?;
    ipc::decode::<NotesRouted>("NotesRouted", &bytes).map_err(|why| Failed::from(why.to_string()))
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
pub fn dispatch(port: u16, manifest: ManifestId, words: String, notes: Vec<Outgoing>, ink: Ink) -> Result<Dispatched, Failed> {
    let pictures = pictured(&notes);
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
        workflow: None,
    };
    let bytes = post(port, "/sessions/dispatch", ipc::encode(&DispatchJob { start, kind: None, then }), pictures)?;
    let done = ipc::decode::<JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())?;
    let first_line: String = words.lines().next().unwrap_or_default().chars().take(60).collect();
    match done {
        JobDispatched { session: Some(record), .. } => {
            let title = record.title.clone().unwrap_or(first_line);
            Ok(Dispatched::Started { id: record.id.as_str().to_string(), title })
        }
        JobDispatched { operator, parked, .. } => match operator.or(parked) {
            Some(id) => Ok(Dispatched::Kept { id, title: first_line }),
            None => Err(Failed::from("Fleet answered the dispatch with nothing in it".to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn out(raw: Value) -> Outgoing {
        Outgoing { key: "note-1".into(), raw, window: false, number: 1, picture: Picture::None }
    }

    const INK: Ink = Ink([0, 0, 0, 255], [255, 255, 255, 255]);

    #[test]
    fn a_note_on_a_job_page_goes_with_the_pages_address_and_what_was_held() {
        let raw = json!({
            "text": " Make it bigger ", "address": "https://fast.com/search?scenario=wide", "selector": "form > input:nth-of-type(1)",
            "element": { "tag": "input", "text": "", "label": "Search" }, "location": "/search?scenario=wide", "screen": null,
            "owners": ["Search", "App"], "nearby": ["Main", "Search"], "stamp": "src/Search.tsx:42", "selected": ["tab \"Web\""], "scenario": "wide",
        });
        let note = note_of(&out(raw), INK);
        assert_eq!(note.address, "https://fast.com/search?scenario=wide");
        assert_eq!(note.said, "Make it bigger");
        assert_eq!(note.place.selector, "form > input:nth-of-type(1)");
        assert_eq!(note.place.element.tag, "input");
        assert_eq!(note.place.element.label.as_deref(), Some("Search"));
        assert_eq!(note.place.location, "/search?scenario=wide");
        assert_eq!(note.place.owners, ["Search", "App"]);
        assert_eq!(note.place.nearby, ["Main", "Search"]);
        assert_eq!(note.place.stamp.as_deref(), Some("src/Search.tsx:42"));
        assert_eq!(note.state.selected, ["tab \"Web\""]);
        assert_eq!(note.state.scenario.as_deref(), Some("wide"));
        assert_eq!(note.note_id.as_deref(), Some("note-1"));
    }

    #[test]
    fn a_note_on_bridges_own_screen_keeps_its_native_address() {
        let note = note_of(&out(json!({ "text": "x", "screen": "Jobs", "selector": "button \"Save\"", "element": { "tag": "button", "text": "Save" } })), INK);
        assert_eq!(note.address, "armada://native/Jobs");
        assert!(note.place.owners.is_empty() && note.place.nearby.is_empty() && note.place.stamp.is_none());
    }

    #[test]
    fn the_lists_of_a_note_are_cut_to_what_the_wire_allows() {
        let long: Vec<String> = (0..40).map(|one| format!("n{one}")).collect();
        let note = note_of(&out(json!({ "text": "x", "address": "https://a.test/", "owners": long, "nearby": long, "selected": long })), INK);
        assert_eq!((note.place.owners.len(), note.place.nearby.len(), note.state.selected.len()), (16, 8, 12));
    }

    fn out_with(key: &str, features: &[&str]) -> Outgoing {
        super::super::beside::remember(key, features.iter().map(|slug| slug.to_string()).collect());
        let raw = serde_json::json!({ "text": "The row clips its title", "selector": "div.row", "element": { "tag": "div", "text": "" }, "screen": "Jobs" });
        Outgoing { key: key.into(), raw, window: false, number: 1, picture: Picture::None }
    }

    #[test]
    fn a_note_goes_to_the_job_with_a_feature_line_for_each_feature_it_maps_to() {
        let said = note_of(&out_with("live-two", &["job-list-and-live-status", "pause-resume-stop-snooze"]), Ink([0; 4], [0; 4])).said;
        assert_eq!(said, "The row clips its title\n\nFeature: job-list-and-live-status\nFeature: pause-resume-stop-snooze");
    }

    #[test]
    fn a_note_with_no_feature_goes_as_it_did() {
        assert_eq!(note_of(&out_with("live-none", &[]), Ink([0; 4], [0; 4])).said, "The row clips its title");
    }

    #[test]
    fn no_more_feature_lines_go_than_a_read_hands_a_job() {
        let said = note_of(&out_with("live-four", &["a", "b", "c", "d"]), Ink([0; 4], [0; 4])).said;
        assert_eq!(said.lines().filter(|line| line.starts_with("Feature: ")).count(), 3);
    }
}
