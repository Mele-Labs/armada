//! The open Job's message box (`docs/spikes/native-jobs-inventory/job-view.md`, section 4): the
//! words and the chips in them, what is attached, the lists `/` `@` `#` open, the permission and
//! model pickers, the quick bar, and Send.
//!
//! **A press answers at once.** Send puts the words in the thread the same frame and clears the
//! box; Fleet's refusal puts the words, the chips and the files back with its reason. A mode or
//! model picked is drawn as picked while Fleet is asked.
//!
//! What Electron works out in TypeScript is not worked out here: a hand-off to a fresh Job, the
//! learnt and forgotten quick lines, and the words of a tool Fleet could not find are asked of
//! Fleet or left out, and named in the report.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use gpui::{
    div, img, prelude::*, svg, Anchor, AnyElement, AnyView, App, ClickEvent, ClipboardEntry, ClipboardItem,
    Context, DismissEvent, Entity, ExternalPaths, FocusHandle, Focusable, FontWeight, Image, ImageFormat, KeyDownEvent, KeyUpEvent, ModifiersChangedEvent, PathPromptOptions,
    Role, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{Enter, Escape, IndentInline, InlineToken, InputContent, InputEvent, MoveDown, MoveUp, Textarea, TextareaState};
use ipc::{
    ModelChoices, SendSessionMessage, SendSessionMessageBody, SentFile, SessionMode, SessionRow, SessionTag, SessionUpload,
    SessionVoice, ThreadItem, TuneSession,
};
use serde::{Deserialize, Serialize};

use super::head::{drawn, icon_button, tip, Confirm};
use super::keys;
use super::offer::{mention_of, Offer};
use super::quick::Quick;
use super::tune::{effort_meter, effort_place, trigger_name, TunePicker, Tuned};
use super::wire::Done;
use super::{echoed, JobView, Page, Pending};
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};

/// A text paste this long, at four characters a token and 8000 tokens, becomes a file (`large-paste.ts`).
const LARGE_PASTE: usize = 8000 * 4;

// ---- the draft ---------------------------------------------------------------------------------

/// A chip in a draft: its place in the text, and what it names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct KeptTag {
    pub start: usize,
    pub end: usize,
    pub id: String,
    pub label: String,
}

/// What a box held: its words and the chips among them. Files are never kept (`keep.ts`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Kept {
    pub text: String,
    #[serde(default)]
    pub tags: Vec<KeptTag>,
}

impl Kept {
    /// The words and chips as the box takes them back.
    fn content(&self) -> InputContent {
        let mut content = InputContent::new(self.text.clone());
        for tag in &self.tags {
            let Some(shown) = self.text.get(tag.start..tag.end) else { continue };
            let token = InlineToken::new(tag.id.clone(), shown.to_string()).with_label(tag.label.clone());
            if let Ok(next) = content.clone().with_token(tag.start..tag.end, token) {
                content = next;
            }
        }
        content
    }
}

fn drafts_file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-drafts"))
}

/// The drafts left in Jobs' boxes, by Job id. Kept between runs as Electron keeps each box under
/// `keep` key `session:<id>`, except on mock data.
pub(super) struct Drafts {
    by_job: HashMap<String, Kept>,
    remember: bool,
}

impl Drafts {
    pub fn read(remember: bool) -> Self {
        let by_job = remember
            .then(|| drafts_file().and_then(|path| std::fs::read(path).ok()))
            .flatten()
            .and_then(|bytes| ipc::decode::<HashMap<String, Kept>>("drafts", &bytes).ok())
            .unwrap_or_default();
        Drafts { by_job, remember }
    }

    pub fn set(&mut self, job: &str, kept: Kept) -> bool {
        if kept.text.is_empty() {
            self.by_job.remove(job).is_some()
        } else if self.by_job.get(job) != Some(&kept) {
            self.by_job.insert(job.to_string(), kept);
            true
        } else {
            false
        }
    }

    pub fn get(&self, job: &str) -> Kept {
        self.by_job.get(job).cloned().unwrap_or_default()
    }

    /// The file's whole text, for a write off the main thread.
    fn encoded(&self) -> Option<(PathBuf, String)> {
        if !self.remember {
            return None;
        }
        Some((drafts_file()?, ipc::encode(&self.by_job).ok()?))
    }
}

// ---- what is attached --------------------------------------------------------------------------

#[derive(Clone)]
pub(super) struct Attached {
    pub key: u64,
    pub name: String,
    pub media_type: String,
    pub bytes: Arc<Vec<u8>>,
    pub preview: Option<Arc<Image>>,
}

impl Attached {
    /// The kind word on a tile: the extension in capitals.
    fn kind(&self) -> String {
        self.name.rsplit_once('.').map_or_else(|| "File".to_string(), |(_, ext)| ext.to_uppercase())
    }
}

fn media_type_of(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, ext)| ext.to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("json") => "application/json",
        Some("md" | "txt" | "rs" | "ts" | "tsx" | "toml" | "yml" | "yaml" | "css" | "html") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn preview_of(media_type: &str, bytes: &[u8]) -> Option<Arc<Image>> {
    let format = match media_type {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::Webp,
        _ => return None,
    };
    Some(Arc::new(Image::from_bytes(format, bytes.to_vec())))
}

/// What one Job's box holds that is not words: its files, and the picks Fleet has not confirmed.
#[derive(Default)]
pub(super) struct JobBox {
    pub files: Vec<Attached>,
    pub mode: Option<SessionMode>,
    pub model: Option<Option<String>>,
    pub effort: Option<Option<String>>,
}

/// What the box has read from Fleet that is not about one Job.
#[derive(Default)]
pub(super) struct Catalog {
    pub models: Option<ModelChoices>,
    pub models_asked: bool,
    pub issues: HashMap<String, Vec<ipc::TrackedIssue>>,
    pub issues_asked: HashSet<String>,
    /// The last file search answered, and the query it answered.
    pub files: Option<(String, Vec<String>)>,
    /// The repository's files on mock data, narrowed as Fleet narrows them.
    pub mock_files: Vec<String>,
    /// What a mock helper says when Talk is pressed in each Job, by Job id.
    pub scripts: HashMap<String, crate::voice::Script>,
}

/// A menu or picker open over the box, and the subscriptions that keep it answering.
pub(super) struct Popup {
    pub view: AnyView,
    pub _subs: Vec<Subscription>,
}

pub(super) struct Composer {
    pub input: Entity<TextareaState>,
    drafts: Drafts,
    /// The box is being set to a draft, which is not the person typing.
    pub setting: bool,
    keeping: Option<Task<()>>,
    pub next_key: u64,
    pub offer: Option<Offer>,
    pub lookup: Option<Task<()>>,
    pub catalog: Catalog,
    pub boxes: HashMap<SharedString, JobBox>,
    pub popup: Option<Popup>,
    pub quick: Quick,
    /// The Aa button's focus, where keys return when its panel is put away.
    pub aa: FocusHandle,
    /// The thread look the rows were last measured for.
    pub thread_drawn: Option<super::quick::Thread>,
    pub talk: super::talk::Talk,
    /// The ⌘ key is down, so Send draws its key.
    cmd: bool,
}

impl Composer {
    pub fn box_of(&mut self, id: &SharedString) -> &mut JobBox {
        self.boxes.entry(id.clone()).or_default()
    }

    pub fn files_of(&self, id: &str) -> &[Attached] {
        self.boxes.get(id).map_or(&[], |one| one.files.as_slice())
    }
}

/// The box, and the subscriptions that keep its parts answering.
pub(super) fn build(remember: bool, window: &mut Window, cx: &mut Context<JobView>) -> (Composer, Vec<Subscription>) {
    let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).submit_on_enter(true));
    let mut subs = Vec::new();
    subs.push(cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
        InputEvent::Change => {
            this.typed(cx);
            this.refresh_offer(window, cx);
        }
        InputEvent::PressEnter { .. } => this.send(window, cx),
        _ => {}
    }));
    subs.push(cx.observe_window_bounds(window, |this, window, cx| this.fit_box(window, cx)));
    let composer = Composer {
        input,
        drafts: Drafts::read(remember),
        setting: false,
        keeping: None,
        next_key: 0,
        offer: None,
        lookup: None,
        catalog: Catalog::default(),
        boxes: HashMap::new(),
        popup: None,
        quick: Quick::default(),
        aa: cx.focus_handle().tab_stop(true),
        thread_drawn: None,
        talk: super::talk::Talk::default(),
        cmd: false,
    };
    (composer, subs)
}

fn encoded_files(files: Vec<Attached>) -> Vec<SessionUpload> {
    let engine = base64::engine::general_purpose::STANDARD;
    files.into_iter().map(|one| SessionUpload { name: one.name, media_type: one.media_type, data: engine.encode(one.bytes.as_slice()) }).collect()
}

impl JobView {
    // ---- the words -----------------------------------------------------------------------

    /// What the box holds now: its words and chips.
    pub(super) fn kept(&self, cx: &App) -> Kept {
        let input = self.composer.input.read(cx);
        Kept {
            text: input.value().to_string(),
            tags: input
                .tokens()
                .iter()
                .map(|span| KeptTag {
                    start: span.range().start,
                    end: span.range().end,
                    id: span.token().id().to_string(),
                    label: span.token().label().to_string(),
                })
                .collect(),
        }
    }

    pub(super) fn set_box(&mut self, kept: &Kept, window: &mut Window, cx: &mut Context<Self>) {
        self.composer.setting = true;
        let content = kept.content();
        self.composer.input.update(cx, |input, cx| input.set_value(content, window, cx));
        self.composer.setting = false;
    }

    /// The words in the box are the shown Job's draft; they are written down after a pause, not on every key.
    pub(super) fn typed(&mut self, cx: &mut Context<Self>) {
        if self.composer.setting {
            return;
        }
        let Some(id) = self.shown.clone() else { return };
        let kept = self.kept(cx);
        if self.composer.drafts.set(&id, kept) {
            self.keep_soon(cx);
        }
        cx.notify();
    }

    pub(super) fn typed_now(&mut self, cx: &mut Context<Self>) {
        self.composer.setting = false;
        self.typed(cx);
    }

    /// Fold what is in the box into the shown Job's draft before it leaves the screen.
    pub(super) fn composer_leave(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let kept = self.kept(cx);
        if self.composer.drafts.set(&id, kept) {
            self.keep_soon(cx);
        }
        self.composer.offer = None;
        self.composer.popup = None;
        self.talk_leave();
    }

    /// The box takes the Job's own draft, and reads what the bar and the pickers need.
    pub(super) fn composer_enter(&mut self, to: Option<&SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let kept = to.map(|id| self.composer.drafts.get(id)).unwrap_or_default();
        self.set_box(&kept, window, cx);
        if to.is_some() {
            self.want_models(cx);
            self.want_look(cx);
            self.fit_box(window, cx);
        }
        self.quick_follow(cx);
    }

    /// The plain words kept under `key`: a Teach panel's, a review note's or a fork's, which carry no chips.
    pub(super) fn draft_words(&self, key: &str) -> String {
        self.composer.drafts.get(key).text
    }

    /// Keep plain words under `key`, or none to let them go, and write the drafts soon if that changed them.
    pub(super) fn keep_words(&mut self, key: &str, text: &str, cx: &mut Context<Self>) {
        if self.composer.drafts.set(key, Kept { text: text.to_string(), tags: Vec::new() }) {
            self.keep_soon(cx);
        }
    }

    /// Write the drafts 400 ms after the last change, off the main thread.
    fn keep_soon(&mut self, cx: &mut Context<Self>) {
        let Some((path, text)) = self.composer.drafts.encoded() else { return };
        self.composer.keeping = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            cx.background_executor()
                .spawn(async move {
                    let kept = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text));
                    if let Err(why) = kept {
                        Note::warn("the drafts were not kept").field("why", why.to_string()).emit();
                    }
                })
                .await;
        }));
    }

    /// The box grows to half the window and then scrolls (`SessionComposer.tsx:768`).
    fn fit_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let look = cx.look();
        let line = look.length(token::SPACE_6);
        let half = window.viewport_size().height * 0.5;
        let rows = ((half / line) as usize).clamp(2, 40);
        self.composer.input.update(cx, |input, cx| input.set_auto_grow(1, rows, cx));
    }

    /// ⌥M: the caret goes in the box, from anywhere in the Job's view (`message_job`).
    pub fn to_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shown.as_ref().is_some_and(|id| !self.closed(id, cx)) {
            self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        }
    }

    /// Words go in the box as a quote, and the caret with them (Reply, `quoteIntoDraft`).
    pub(crate) fn quote(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let quoted: String = text
            .trim()
            .lines()
            .map(|line| if line.is_empty() { ">".to_string() } else { format!("> {line}") })
            .collect::<Vec<_>>()
            .join("\n");
        let held = self.kept(cx);
        let joined = if held.text.trim().is_empty() { format!("{quoted}\n\n") } else { format!("{}\n\n{quoted}\n\n", held.text) };
        let kept = Kept { text: joined, tags: held.tags };
        self.set_box(&kept, window, cx);
        self.composer.input.update(cx, |input, cx| {
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
            input.focus(window, cx);
        });
        self.typed_now(cx);
    }

    /// A line half typed outside the box (the Aa panel's new quick reply) survives the panel closing, and the run.
    pub(super) fn keep_text(&mut self, key: &str, text: String, cx: &mut Context<Self>) {
        if self.composer.drafts.set(key, Kept { text, tags: Vec::new() }) {
            self.keep_soon(cx);
        }
    }

    pub(super) fn kept_text(&self, key: &str) -> String {
        self.composer.drafts.get(key).text
    }

    /// The box takes `kept` and the caret goes to its end, where Talk's words join.
    pub(super) fn set_words(&mut self, kept: Kept, window: &mut Window, cx: &mut Context<Self>) {
        self.set_box(&kept, window, cx);
        self.composer.input.update(cx, |input, cx| {
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
        });
        self.typed_now(cx);
    }

    // ---- sending -------------------------------------------------------------------------

    /// Send what is in the box. A paused Job is asked about first.
    pub(super) fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.closed(&id, cx) {
            return;
        }
        let kept = self.kept(cx);
        if kept.text.trim().is_empty() && self.composer.files_of(&id).is_empty() {
            return;
        }
        if self.paused(&id, cx) {
            let confirm = cx.new(|cx| {
                Confirm::new("Job is paused", &["Resuming takes a slot again. The note goes after."], ("", Vec::new()), "Resume and send", "Cancel", false, cx)
            });
            let job = id.clone();
            self.open_confirm(confirm, window, cx, move |this, window, cx| {
                this.head_act("resume", window, cx);
                this.deliver(job.clone(), None, window, cx);
            });
            return;
        }
        self.deliver(id, None, window, cx);
    }

    /// A quick line goes as it is, and the box keeps its draft.
    pub(super) fn send_line(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.closed(&id, cx) || text.trim().is_empty() {
            return;
        }
        self.deliver(id, Some(text), window, cx);
    }

    /// The words are in the thread this frame; Fleet's row replaces them when it arrives, and a
    /// refusal puts them back with Fleet's reason. `line` is a quick line sent beside the draft.
    fn deliver(&mut self, id: SharedString, line: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let (kept, files) = match &line {
            Some(text) => (Kept { text: text.clone(), tags: Vec::new() }, Vec::new()),
            None => (self.kept(cx), std::mem::take(&mut self.composer.box_of(&id).files)),
        };
        let text = kept.text.trim().to_string();
        if text.is_empty() && files.is_empty() {
            return;
        }
        self.composer.next_key += 1;
        let key = self.composer.next_key;
        let seen = self.thread(&id, cx).map_or(0, |held| echoed(&held.items, &text));
        let mentions: Vec<SessionTag> = kept.tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect();
        if line.is_none() {
            self.set_box(&Kept::default(), window, cx);
            if self.composer.drafts.set(&id, Kept::default()) {
                self.keep_soon(cx);
            }
            self.composer.offer = None;
        }
        let page = self.pages.entry(id.clone()).or_insert_with(Page::new);
        page.refused = None;
        match self.fleet.clone() {
            Some(_) => {
                page.pending.push(Pending { key, text: text.clone(), seen });
                let job = id.clone();
                let sent_files = files.clone();
                let message = SendSessionMessage { session_id: Self::session_id(&id), text: text.clone(), attachments: Vec::new(), mentions };
                cx.spawn_in(window, async move |this, cx| {
                    let uploads = cx.background_executor().spawn(async move { encoded_files(sent_files) }).await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        let body = SendSessionMessageBody { message: SendSessionMessage { attachments: uploads, ..message }, aim: None };
                        let back = (kept, files, line.is_some());
                        let done: Done = Box::new(move |this, answer, window, cx| {
                            if let Err(why) = answer {
                                this.refused(&job, key, back, why.into(), window, cx);
                            }
                        });
                        this.post("/sessions/message".into(), ipc::encode(&body), window, cx, done);
                    });
                })
                .detach();
            }
            None => {
                // Mock data: Fleet's row for these words arrives at once.
                let at = ipc::Instant::carried(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                if let Some(held) = self.mock.get_mut(id.as_str()) {
                    held.items.push(ThreadItem::Row {
                        row: SessionRow::Message {
                            id: format!("sent-{key}"),
                            at,
                            from: SessionVoice::You,
                            text,
                            files: files
                                .iter()
                                .map(|one| SentFile { id: format!("file-{}", one.key), name: one.name.clone(), media_type: one.media_type.clone() })
                                .collect(),
                            tags: mentions,
                            notice: None,
                            queued: false,
                        },
                    });
                    held.version += 1;
                    let at = held.items.len() - 1;
                    held.mark_dirty(at);
                }
            }
        }
        self.pin(&id);
        self.sync(&id, cx);
        cx.notify();
    }

    /// Fleet refused the words: they go back in the box unless something was typed since, with their
    /// chips and files, and its reason stands over the box.
    fn refused(&mut self, id: &SharedString, key: u64, back: (Kept, Vec<Attached>, bool), why: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let (kept, files, was_line) = back;
        if let Some(page) = self.pages.get_mut(id) {
            page.pending.retain(|one| one.key != key);
            page.refused = Some(why);
        }
        if !was_line {
            self.composer.box_of(id).files.splice(0..0, files);
            if self.shown.as_ref() == Some(id) {
                if self.composer.input.read(cx).value().is_empty() {
                    self.set_box(&kept, window, cx);
                    self.composer.drafts.set(id, kept);
                    self.keep_soon(cx);
                }
            } else if self.composer.drafts.get(id).text.is_empty() {
                self.composer.drafts.set(id, kept);
                self.keep_soon(cx);
            }
        }
        self.sync(id, cx);
        cx.notify();
    }

    pub(super) fn on_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) {
        if self.review_act(act, window, cx) {
            return;
        }
        if act.is("needs_you") {
            self.needs_key(window, cx);
            return;
        }
        let in_box = self.composer.input.read(cx).focus_handle(cx).contains_focused(window, cx);
        let thread_key = [keys::Which::Yes, keys::Which::Change, keys::Which::SendBack, keys::Which::No].into_iter().find(|which| act.is(which.act()));
        if let Some(which) = thread_key {
            self.thread_key(which, window, cx);
        } else if act.is("send_message") && in_box {
            // ⌘Enter sends from the box too.
            self.send(window, cx);
        } else if act.is("deck_tabs") && in_box && act.digit.is_some_and(|digit| self.press_quick(digit as usize - 1, false, window, cx)) {
            // ⌘1 to ⌘9 in the box send quick line n, and the tabs do not move.
        } else if act.is("handoff_retry") && self.handoff_band_focused(window, cx) {
            self.retry_handoff(window, cx);
        } else {
            cx.propagate();
        }
    }

    /// ⇧⌘1 to ⇧⌘9 put quick line n after the draft (⌘1 to ⌘9 reach the line through `deck_tabs`).
    pub(super) fn on_box_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.talk_key(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        let keys = &event.keystroke;
        if keys.modifiers.platform && keys.modifiers.shift && !keys.modifiers.alt && !keys.modifiers.control {
            if let Some(n) = keys.key.parse::<usize>().ok().filter(|n| (1..=9).contains(n)) {
                if self.press_quick(n - 1, true, window, cx) {
                    cx.stop_propagation();
                }
            }
        }
    }

    /// The keys the text box would take, which a list over it takes first.
    fn list_key(&mut self, key: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if key == "escape" && self.talk_put_away(cx) {
            cx.stop_propagation();
        } else if self.offer_key(key, window, cx) {
            cx.stop_propagation();
        }
    }

    // ---- attachments ---------------------------------------------------------------------

    /// Attach opens the Mac's own picker. On mock data there is none to answer, so it adds a sample.
    pub(super) fn attach(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.fleet.is_none() {
            let bytes = b"fn main() {}\n".to_vec();
            self.add_file(&id, "sample.rs".into(), "text/plain".into(), bytes, cx);
            return;
        }
        let picked = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: None });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
                let _ = this.update(cx, |this, cx| this.take_paths(paths, cx));
            }
        })
        .detach();
    }

    /// Files named by a drop or a paste are read off the main thread and attached to the Job in view.
    pub(super) fn take_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        cx.spawn(async move |this, cx| {
            for path in paths {
                let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_else(|| "File".into());
                let read = cx.background_executor().spawn(async move { std::fs::read(&path) }).await;
                let id = id.clone();
                match read {
                    Ok(bytes) => {
                        let _ = this.update(cx, |this, cx| {
                            let media_type = media_type_of(&name).to_string();
                            this.add_file(&id, name, media_type, bytes, cx);
                        });
                    }
                    Err(why) => {
                        Note::warn("a file was not read for attaching").field("name", name).field("why", why.to_string()).emit();
                    }
                }
            }
        })
        .detach();
    }

    fn add_file(&mut self, id: &SharedString, name: String, media_type: String, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.composer.next_key += 1;
        let key = self.composer.next_key;
        let preview = preview_of(&media_type, &bytes);
        self.composer.box_of(id).files.push(Attached { key, name, media_type, bytes: Arc::new(bytes), preview });
        cx.notify();
    }

    fn remove_file(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.composer.box_of(&id).files.retain(|one| one.key != key);
        // The caret goes back to the box where the chip was.
        self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// A paste with a picture or files on the clipboard attaches them; a long text becomes a file.
    fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        let mut taken = false;
        for entry in item.entries() {
            match entry {
                ClipboardEntry::Image(image) => {
                    let media_type = image.format.mime_type().to_string();
                    let ext = media_type.rsplit('/').next().unwrap_or("png").to_string();
                    self.add_file(&id, format!("Pasted file.{ext}"), media_type, image.bytes.clone(), cx);
                    taken = true;
                }
                ClipboardEntry::ExternalPaths(paths) => {
                    self.take_paths(paths.paths().to_vec(), cx);
                    taken = true;
                }
                ClipboardEntry::String(text) if text.text().len() > LARGE_PASTE => {
                    self.composer.next_key += 1;
                    let name = format!("pasted-snippet-{}.txt", self.composer.next_key);
                    self.add_file(&id, name, "text/plain".into(), text.text().clone().into_bytes(), cx);
                    taken = true;
                }
                ClipboardEntry::String(_) => {}
            }
        }
        taken
    }

    // ---- pickers -------------------------------------------------------------------------

    pub(super) fn want_models(&mut self, cx: &mut Context<Self>) {
        if self.composer.catalog.models_asked || self.fleet.is_none() {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        self.composer.catalog.models_asked = true;
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, "/models"))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<ModelChoices>("ModelChoices", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(models) => this.composer.catalog.models = Some(models),
                    Err(_) => this.composer.catalog.models_asked = false,
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The mode, model and effort the next turn runs on: what was just picked, else the record's.
    pub(super) fn tuning(&self, id: &str, cx: &App) -> (SessionMode, Option<String>, Option<String>) {
        let hosted = self.record(id, cx).and_then(|job| job.hosted.as_ref());
        let held = self.composer.boxes.get(id);
        let mode = held.and_then(|one| one.mode).or_else(|| hosted.map(|hosted| hosted.mode)).unwrap_or_default();
        let model = held.and_then(|one| one.model.clone()).unwrap_or_else(|| hosted.and_then(|hosted| hosted.model.clone()));
        let effort = held.and_then(|one| one.effort.clone()).unwrap_or_else(|| hosted.and_then(|hosted| hosted.effort.clone()));
        (mode, model, effort)
    }

    fn ask_tune(&mut self, id: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let (mode, model, effort) = self.tuning(&id, cx);
        let body = ipc::encode(&TuneSession { session_id: Self::session_id(&id), model, effort, mode });
        let job = id.clone();
        let done: Done = Box::new(move |this, answer, _, cx| {
            match answer {
                Ok(_) => {
                    // Fleet's record carries the pick now; the overlay is let go of a beat later.
                    if this.fleet.is_some() {
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_millis(2500)).await;
                            let _ = this.update(cx, |this, cx| {
                                if let Some(one) = this.composer.boxes.get_mut(&job) {
                                    (one.mode, one.model, one.effort) = (None, None, None);
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                }
                Err(why) => {
                    if let Some(one) = this.composer.boxes.get_mut(&job) {
                        (one.mode, one.model, one.effort) = (None, None, None);
                    }
                    if let Some(page) = this.pages.get_mut(&job) {
                        page.refused = Some(why.into());
                    }
                }
            }
            cx.notify();
        });
        self.post("/sessions/tune".into(), body, window, cx, done);
    }

    fn mode_entries(&self, id: &str, cx: &App) -> Vec<Entry> {
        let (mode, ..) = self.tuning(id, cx);
        [(SessionMode::Ask, "ask", "Ask"), (SessionMode::Auto, "auto", "Auto"), (SessionMode::AcceptEdits, "accept_edits", "Accept edits"), (SessionMode::Plan, "plan", "Plan")]
            .into_iter()
            .map(|(one, key, label)| Entry::Choice { id: key, label: label.into(), on: one == mode })
            .collect()
    }

    pub(super) fn open_mode(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(at) = drawn(window, cx, Role::Button, "Permission mode").map(|bounds| bounds.origin) else { return };
        let entries = self.mode_entries(&id, cx);
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Permission mode", entries, at, Anchor::BottomLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: pick, .. } = picked;
            let mode = match *pick {
                "ask" => SessionMode::Ask,
                "accept_edits" => SessionMode::AcceptEdits,
                "plan" => SessionMode::Plan,
                _ => SessionMode::Auto,
            };
            if let Some(job) = this.shown.clone() {
                this.composer.box_of(&job).mode = Some(mode);
                this.ask_tune(job, window, cx);
            }
        });
        self.open_popup(menu, back, vec![picked], window, cx);
    }

    /// Open the Aa panel upward from its button.
    pub(super) fn open_aa(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = drawn(window, cx, Role::Button, "Thread look").map(|bounds| bounds.origin) else { return };
        self.want_look(cx);
        let job = self.shown.clone();
        let view = cx.entity();
        let held = (self.composer.quick.panel().agent_name, self.kept_text("thread-look:new-line"), self.composer.quick.mock_whisper());
        let panel = cx.new(|cx| super::aa::Aa::new(view, job, at, held, false, window, cx));
        let back = Some(self.composer.aa.clone());
        self.open_popup(panel, back, Vec::new(), window, cx);
    }

    pub(super) fn open_tune(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(at) = drawn(window, cx, Role::Button, &self.tune_name(&id, cx)).map(|bounds| bounds.origin) else { return };
        let (_, model, effort) = self.tuning(&id, cx);
        let models = self.composer.catalog.models.as_ref();
        let (offered, default) = (models.map(|models| models.offered.clone()), models.map(|models| models.session.clone()).unwrap_or_default());
        let back = window.focused(cx);
        let picker = cx.new(|cx| TunePicker::new(offered, default, model, effort, at, cx));
        let tuned = cx.subscribe_in(&picker, window, |this, _, tuned: &Tuned, window, cx| {
            if let Some(job) = this.shown.clone() {
                let one = this.composer.box_of(&job);
                one.model = Some(tuned.model.clone());
                one.effort = Some(tuned.effort.clone());
                this.ask_tune(job, window, cx);
            }
        });
        self.open_popup(picker, back, vec![tuned], window, cx);
    }

    /// Show a menu or picker over the box, give it the keys, and give them back where they were when it shuts.
    pub(super) fn open_popup<T: gpui::Render + gpui::EventEmitter<DismissEvent> + gpui::Focusable>(
        &mut self,
        view: Entity<T>,
        back: Option<FocusHandle>,
        mut subs: Vec<Subscription>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dismissed = cx.subscribe_in(&view, window, move |this, _, _: &DismissEvent, window, cx| {
            this.composer.popup = None;
            let box_keys = this.composer.input.read(cx).focus_handle(cx);
            window.focus(back.as_ref().unwrap_or(&box_keys), cx);
            cx.notify();
        });
        subs.push(dismissed);
        let keys = gpui::Focusable::focus_handle(view.read(cx), cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.composer.popup = Some(Popup { view: view.into(), _subs: subs });
        cx.notify();
    }

    fn tune_name(&self, id: &str, cx: &App) -> SharedString {
        let (_, model, effort) = self.tuning(id, cx);
        let models = self.composer.catalog.models.as_ref();
        let default = models.map(|models| models.session.as_str()).unwrap_or_default();
        trigger_name(models.map(|models| models.offered.as_slice()), default, model.as_deref(), effort.as_deref()).1.into()
    }

    // ---- drawing -------------------------------------------------------------------------

    /// A quiet text button on the bar: its words, a chevron, and a tooltip that says what it sets.
    fn bar_button(&self, look: &Look, id: &'static str, said: SharedString, tooltip: SharedString, meter: Option<AnyElement>) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_MUTED))
            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
            .child(div().whitespace_nowrap().child(said))
            .children(meter)
            .child(svg().path(icon_path("chevron-down")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)))
            .tooltip(tip(tooltip))
    }

    fn render_tray(&self, id: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let files = self.composer.files_of(id);
        if files.is_empty() {
            return None;
        }
        let tiles = files.iter().map(|one| {
            let key = one.key;
            let name: SharedString = one.name.clone().into();
            let remove = labelled(
                icon_button(look, "remove-file", "trash", format!("Remove {}", one.name).into(), false, false)
                    .absolute()
                    .top(look.length(token::SPACE_1))
                    .right(look.length(token::SPACE_1))
                    .size(look.length(token::SPACE_6))
                    .rounded_full()
                    .bg(look.color(token::BG_OVERLAY))
                    .on_click(cx.listener(move |this, _, window, cx| this.remove_file(key, window, cx))),
                Role::Button,
                format!("Remove {}", one.name),
            );
            let body = match &one.preview {
                Some(image) => div()
                    .h_full()
                    .min_w(look.length(token::SPACE_12))
                    .max_w(look.length(token::SPACE_12) * 2.7)
                    .child(img(image.clone()).h_full().object_fit(gpui::ObjectFit::Cover))
                    .into_any_element(),
                None => div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).overflow_hidden().child(name.clone()))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(one.kind()),
                    )
                    .into_any_element(),
            };
            labelled(
                div()
                    .id(SharedString::from(format!("file-{key}")))
                    .relative()
                    .flex_none()
                    .h(look.length(token::SPACE_12) + look.length(token::SPACE_4))
                    .when(one.preview.is_none(), |tile| tile.w(look.length(token::SPACE_12) * 3.))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .child(body)
                    .child(remove)
                    .tooltip(tip(name.clone())),
                Role::ListItem,
                name,
            )
        });
        Some(
            labelled(
                div()
                    .id("tray")
                    .flex()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .pt(look.length(token::SPACE_3))
                    .overflow_x_scroll()
                    .children(tiles),
                Role::Group,
                "Attached",
            )
            .into_any_element(),
        )
    }

    /// The box and what hangs on it: refusal, hand-off band, quick bar, tray, words, bar and acts.
    pub(super) fn render_dock(&self, id: &SharedString, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let refused = self.pages.get(id).and_then(|page| page.refused.clone());
        let kept_empty = self.composer.input.read(cx).value().trim().is_empty();
        let nothing = kept_empty && self.composer.files_of(id).is_empty();
        let focused = self.composer.input.read(cx).focus_handle(cx).contains_focused(window, cx);
        let (mode, ..) = self.tuning(id, cx);
        let mode_said: SharedString = match mode {
            SessionMode::Ask => "Ask",
            SessionMode::Auto => "Auto",
            SessionMode::AcceptEdits => "Accept edits",
            SessionMode::Plan => "Plan",
        }
        .into();
        let mode_tip: SharedString = match mode {
            SessionMode::Ask => "Asks before it edits or runs anything",
            SessionMode::Auto => "Runs on its own, and asks before a push or anything destructive",
            SessionMode::AcceptEdits => "Edits files without asking, and asks before running a command",
            SessionMode::Plan => "Reads and plans, and changes nothing",
        }
        .into();
        let (_, model, effort) = self.tuning(id, cx);
        let models = self.composer.catalog.models.as_ref();
        let offered = models.map(|models| models.offered.as_slice());
        let default = models.map(|models| models.session.as_str()).unwrap_or_default();
        let (shown, tune_said) = trigger_name(offered, default, model.as_deref(), effort.as_deref());
        let (steps, lit) = effort_place(offered, default, model.as_deref(), effort.as_deref());
        let meter = (steps > 0).then(|| effort_meter(look, steps, lit).into_any_element());
        let key_text = window.modifiers().platform;
        let send = labelled(
            div()
                .id("send")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(nothing, |button| button.opacity(0.4))
                .when(!nothing, |button| {
                    button
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                        .on_click(cx.listener(|this, _, window, cx| this.send(window, cx)))
                })
                .child(svg().path(icon_path("send")).size(look.length(token::SPACE_3)).text_color(look.color(token::TEXT_ON_ACCENT)))
                .child("Send")
                .when(key_text, |button| button.child(div().text_color(look.color(token::TEXT_ON_ACCENT).opacity(0.7)).child("\u{2318}\u{21b5}"))),
            Role::Button,
            "Send",
        );
        let attach = labelled(
            icon_button(look, "attach", "paperclip", "Attach a file or a picture".into(), false, false)
                .on_click(cx.listener(|this, _, window, cx| this.attach(window, cx))),
            Role::Button,
            "Attach file",
        );
        let mode_button = labelled(
            self.bar_button(look, "mode", mode_said, mode_tip, None).on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                this.open_mode(event.is_keyboard(), window, cx);
            })),
            Role::Button,
            "Permission mode",
        );
        let tune_button = labelled(
            self.bar_button(look, "tune", shown.into(), "The model and effort the next turn runs on".into(), meter).on_click(cx.listener(
                |this, _, window, cx| this.open_tune(window, cx),
            )),
            Role::Button,
            tune_said,
        );
        let column = look.length(token::SPACE_12) * 20. + look.length(token::SPACE_6) + look.length(token::SPACE_2) + look.length(token::SPACE_4);
        let words = div()
            .id("message")
            .key_context(crate::keymap::TEXT_BOX)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.on_box_key(event, window, cx)))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _, cx| this.talk_key_up(event, cx)))
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if !action.secondary && !action.shift {
                    this.list_key("enter", window, cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, window, cx| this.list_key("up", window, cx)))
            .capture_action(cx.listener(|this, _: &MoveDown, window, cx| this.list_key("down", window, cx)))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| this.list_key("escape", window, cx)))
            .capture_action(cx.listener(|this, _: &IndentInline, window, cx| this.list_key("tab", window, cx)))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_BASE))
            .text_color(look.color(token::FG_DEFAULT))
            .child({
                let view = cx.entity().downgrade();
                Textarea::new(&self.composer.input).appearance(false).aria_label("Message").on_paste(move |item, _, cx| {
                    view.update(cx, |this, cx| this.paste(item, cx)).unwrap_or(false)
                })
            });
        let bar = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(mode_button)
            .child(tune_button)
            .child(attach)
            .child(div().flex_1())
            .child(div().id("context-figure").flex_none())
            .child(self.render_talk(look, cx))
            .child(send);
        let popup = self.composer.popup.as_ref().map(|popup| popup.view.clone());
        let needs = self.render_needs(id, cx);
        let risen = needs.is_some() && self.needs_up(id, cx);
        div()
            .flex_none()
            .flex()
            .justify_center()
            .px(look.length(token::SPACE_4))
            .pb(look.length(token::SPACE_4))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                this.composer.cmd = event.modifiers.platform;
                this.talk_modifiers(event, cx);
                cx.notify();
            }))
            .child(
                div()
                    .w_full()
                    .max_w(column)
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(refused.map(|why| {
                        labelled(
                            div()
                                .id("refused")
                                .px(look.length(token::SPACE_3))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_SM))
                                .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                                .child(why),
                            Role::Status,
                            "Refused",
                        )
                    }))
                    .children(self.render_handoff_band(id, cx))
                    .child(
                        div()
                            .relative()
                            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.take_paths(paths.paths().to_vec(), cx)))
                            .flex()
                            .flex_col()
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(if needs.is_some() {
                                token::STEP_WAITING
                            } else if focused {
                                token::BORDER_DEFAULT
                            } else {
                                token::BORDER_SUBTLE
                            }))
                            // The band is the frame's top edge; a drawn one would cross the panel risen over it.
                            .when(needs.is_some(), |frame| frame.border_t(gpui::px(0.)))
                            .when(risen, |frame| frame.rounded_t(gpui::px(0.)))
                            .bg(look.color(token::BG_SUNKEN))
                            .children(needs)
                            .children(self.render_offer(look, cx))
                            .children(self.render_quick(id, look, window, cx))
                            .children(self.render_tray(id, look, cx))
                            .child(labelled(words, Role::TextInput, "Message"))
                            .child(bar),
                    )
                    .children(self.render_toolbelt(id, cx))
                    .children(popup),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(text: &str) -> Kept {
        Kept { text: text.into(), tags: Vec::new() }
    }

    #[test]
    fn a_draft_is_set_cleared_and_only_a_change_asks_for_a_write() {
        let mut drafts = Drafts { by_job: HashMap::new(), remember: false };
        assert!(drafts.set("a", kept("words")));
        assert!(!drafts.set("a", kept("words")));
        assert_eq!(drafts.get("a").text, "words");
        assert!(drafts.set("a", kept("")));
        assert!(!drafts.set("a", kept("")));
        assert_eq!(drafts.get("a").text, "");
        assert!(drafts.encoded().is_none(), "mock data keeps nothing");
    }

    #[test]
    fn a_draft_keeps_its_chips_and_a_chip_that_no_longer_fits_the_text_is_dropped() {
        let tag = KeptTag { start: 4, end: 9, id: "issue\u{1f}33\u{1f}".into(), label: "#33".into() };
        let past = Kept { text: "see @#33 now".into(), tags: vec![KeptTag { end: 99, ..tag.clone() }] };
        assert_eq!(past.content().tokens().len(), 0, "the range is not in the text");
        let right = Kept { text: "see @#33 now".into(), tags: vec![KeptTag { start: 4, end: 8, ..tag }] };
        assert_eq!(right.content().tokens().len(), 1);
    }

    #[test]
    fn a_file_is_named_by_its_extension_in_capitals_and_typed_by_it() {
        let file = Attached { key: 1, name: "shot.png".into(), media_type: media_type_of("shot.png").into(), bytes: Arc::new(Vec::new()), preview: None };
        assert_eq!((file.kind().as_str(), file.media_type.as_str()), ("PNG", "image/png"));
        assert_eq!(media_type_of("notes"), "application/octet-stream");
    }

    #[test]
    fn an_upload_carries_its_bytes_as_base64() {
        let file = Attached { key: 1, name: "a.txt".into(), media_type: "text/plain".into(), bytes: Arc::new(b"hi".to_vec()), preview: None };
        assert_eq!(encoded_files(vec![file])[0].data, "aGk=");
    }
}
