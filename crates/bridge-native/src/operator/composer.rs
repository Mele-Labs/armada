//! The box pinned to the foot of the Operator's page: where he talks to the Operator. **Every send is a talk**
//! (`talk_to_operator`), words with the files and tags he put beside them, answered in the talk. The Operator reads
//! from his words whether it is work, and starts a Job, folds it into a live one, asks him or keeps it as an idea.
//!
//! **It is the Job's message box, not a copy of it.** The words, the chips in them, the tray of files, the list
//! `@` and `#` open, Send and the paste are `job_view::composer` and `job_view::offer`, drawn here from this
//! entity's state. What a Job's box has and this has not: its permission and model pickers, Talk, sketches and
//! quick lines, which are a running Job's.
//!
//! **The words are kept** under [`KEEP`], chips and all, so leaving the page, a crash or a restart does not lose them
//! (`drafts.rs` writes them to the app's file, as the Jobs' box is). With a card tagged they are kept under that card
//! instead (`operator:compose:<card id>`), and Discuss on it again brings them back into an empty box. They go when
//! Fleet answers the send, and stay where it refuses.
//!
//! **Discuss puts a tag in the box**, naming the card he is talking about. A send with the tag is a talk about that
//! card (`talk_to_operator`, `about` the card's id), files and all; dropping the tag makes it a talk about no card again.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, Context, Div, Entity, EventEmitter, ExternalPaths, FocusHandle, Focusable as _,
    FontWeight, KeyDownEvent, ModifiersChangedEvent, PathPromptOptions, Role, SharedString, Stateful, Subscription, Window,
};
use gpui_component::input::{Enter, Escape, IndentInline, InputEvent, MoveDown, MoveUp, TextareaState};
use ipc::{SessionTag, SessionUpload, TagKind, TrackedIssue};

use crate::dispatch::live::{served, Served};
use crate::fleet::FleetConnection;
use crate::job_view::composer::{clipped, encoded_files, read_files, send_face, tray, words_area, Attached, Kept, KeptTag};
use crate::job_view::format::sent_text;
use crate::job_view::head::{icon_button, tip};
use crate::job_view::offer::{chip_of, mention_of, numbers_for, offer_list, token_at, Held, Offer, Pick, Row};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::glyph;

/// The key the words are kept under, as Electron's `useKept` names a place.
pub const KEEP: &str = "operator:compose";

/// The key the words are kept under while the box is tagged with the card `id`.
fn keep_for(id: &str) -> String {
    format!("{KEEP}:{id}")
}

/// What the box asks of the page.
#[derive(Clone, Debug, PartialEq)]
pub enum Composed {
    /// Words to the Operator as a talk, with the files and tags he sent: about the card the tag names, or about no card.
    Talk { words: String, about: Option<String>, attachments: Vec<SessionUpload>, mentions: Vec<SessionTag> },
    /// Esc with nothing open: the keys go back to the page.
    Leave,
}

impl EventEmitter<Composed> for OperatorComposer {}

pub struct OperatorComposer {
    input: Entity<TextareaState>,
    /// The box's frame, so the page can tell whether the box has the keys.
    frame: FocusHandle,
    files: Vec<Attached>,
    next_key: u64,
    offer: Option<Offer>,
    /// Sent, and Fleet has not answered.
    sending: bool,
    /// Fleet's sentence where it refused the send.
    refused: Option<SharedString>,
    /// The box is being set to a draft, which is not the person typing.
    setting: bool,
    /// The card Discuss named: its id and title. A send with it is a talk about that card.
    tag: Option<(String, SharedString)>,
    /// The Jobs `@` offers, by id and title.
    jobs: Vec<(String, String)>,
    fleet: Option<Entity<FleetConnection>>,
    /// The repository's issues, by Manifest id, once read; and the ones asked for.
    issues: HashMap<String, Vec<TrackedIssue>>,
    asked: HashSet<String>,
    _subs: Vec<Subscription>,
}

/// A press, on a click and on Enter or Space while it has the keys: the key goes no further, so the page's own Enter is
/// not said as well.
pub(super) fn pressed<T: 'static>(element: Stateful<Div>, cx: &Context<T>, act: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static) -> Stateful<Div> {
    let act = std::rc::Rc::new(act);
    let by_key = act.clone();
    element.on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
        let key = &event.keystroke;
        if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
            cx.stop_propagation();
            by_key(this, window, cx);
        }
    }))
}

impl OperatorComposer {
    pub fn new(fleet: Option<Entity<FleetConnection>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).submit_on_enter(true));
        // What he left in the box comes back, before the box is listened to.
        if let Some(kept) = Self::left(KEEP, cx) {
            let content = kept.content();
            input.update(cx, |input, cx| input.set_value(content, window, cx));
        }
        let typed = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                this.typed(cx);
                this.refresh_offer(window, cx);
            }
            // Shift+Enter puts a new line in the box, which the box has done; a plain Enter sends.
            InputEvent::PressEnter { shift: false, .. } => this.send(window, cx),
            _ => {}
        });
        OperatorComposer {
            input,
            frame: cx.focus_handle(),
            files: Vec::new(),
            next_key: 0,
            offer: None,
            sending: false,
            refused: None,
            setting: false,
            tag: None,
            jobs: Vec::new(),
            fleet,
            issues: HashMap::new(),
            asked: HashSet::new(),
            _subs: vec![typed],
        }
    }

    // ---- what it holds -------------------------------------------------------------------

    /// The words and chips he left, if any stand.
    fn left(key: &str, cx: &App) -> Option<Kept> {
        let kept = crate::drafts::kept(key, cx)?;
        ipc::decode::<Kept>("the Operator's draft", kept.as_bytes()).ok()
    }

    /// What the box holds now: its words and chips.
    pub fn kept(&self, cx: &App) -> Kept {
        let input = self.input.read(cx);
        Kept {
            text: input.value().to_string(),
            tags: input
                .tokens()
                .iter()
                .map(|span| KeptTag { start: span.range().start, end: span.range().end, id: span.token().id().to_string(), label: span.token().label().to_string() })
                .collect(),
        }
    }

    pub fn words(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    pub fn is_sending(&self) -> bool {
        self.sending
    }

    pub fn refused(&self) -> Option<&SharedString> {
        self.refused.as_ref()
    }

    pub fn files(&self) -> &[Attached] {
        &self.files
    }

    /// The Jobs `@` offers.
    pub fn set_jobs(&mut self, jobs: Vec<(String, String)>, cx: &mut Context<Self>) {
        if self.jobs != jobs {
            self.jobs = jobs;
            cx.notify();
        }
    }

    pub fn has_keys(&self, window: &Window, cx: &App) -> bool {
        self.input.read(cx).focus_handle(cx).contains_focused(window, cx)
    }

    /// The key the words are kept under now: the page's, or the tagged card's.
    fn key(&self) -> String {
        self.tag.as_ref().map_or_else(|| KEEP.to_string(), |(id, _)| keep_for(id))
    }

    /// Discuss: the box names the card in a tag, until he drops it or the talk ends. Words already in the box go with the tag;
    /// an empty box takes back what was left for the card.
    pub fn tag(&mut self, id: String, title: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let was = self.key();
        self.tag = Some((id, title));
        self.refused = None;
        let now = self.key();
        if was != now {
            if self.input.read(cx).value().is_empty() {
                if let Some(kept) = Self::left(&now, cx) {
                    self.set_box(&kept, window, cx);
                }
            } else {
                crate::drafts::clear(&was, cx);
                self.typed(cx);
            }
        }
        cx.notify();
    }

    /// The card the box is tagged with, and its title.
    #[cfg(test)]
    pub fn tagged(&self) -> Option<(String, SharedString)> {
        self.tag.clone()
    }

    /// The tag goes; the words in the box stay, and are kept for the page again.
    pub fn untag(&mut self, cx: &mut Context<Self>) {
        let was = self.key();
        if self.tag.take().is_some() {
            crate::drafts::clear(&was, cx);
            self.typed(cx);
            cx.notify();
        }
    }

    /// The caret goes in the box.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    /// The words are kept for the place, so the page can be left and come back to them.
    fn typed(&mut self, cx: &mut Context<Self>) {
        if self.setting {
            return;
        }
        let kept = self.kept(cx);
        let key = self.key();
        match (kept.text.is_empty(), ipc::encode(&kept)) {
            (false, Ok(encoded)) => crate::drafts::keep(&key, &encoded, cx),
            _ => crate::drafts::clear(&key, cx),
        }
        cx.notify();
    }

    fn set_box(&mut self, kept: &Kept, window: &mut Window, cx: &mut Context<Self>) {
        self.setting = true;
        let content = kept.content();
        self.input.update(cx, |input, cx| input.set_value(content, window, cx));
        self.setting = false;
    }

    // ---- where it goes -------------------------------------------------------------------

    /// The repositories the window is scoped to, and the one `#` reads issues from: the scope's first.
    fn targets(&self, cx: &App) -> (Vec<Served>, Option<usize>) {
        let repositories = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().repositories.clone().unwrap_or_default(),
            None => crate::mock::repositories(),
        };
        let mut all = served(&repositories);
        let scoped = crate::dispatch::screen::scope(cx).map(|(_, names)| names);
        if let Some(names) = scoped.as_ref().filter(|names| !names.is_empty()) {
            all.retain(|one| names.contains(&one.name));
        }
        let at = scoped.and_then(|names| names.first().and_then(|first| all.iter().position(|one| &one.name == first))).or_else(|| (!all.is_empty()).then_some(0));
        (all, at)
    }

    // ---- what `@` and `#` offer ----------------------------------------------------------

    fn refresh_offer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let (text, cursor) = {
            let input = self.input.read(cx);
            (input.value().to_string(), input.cursor())
        };
        let Some((trigger, from, query)) = token_at(&text, cursor) else {
            self.offer = None;
            return;
        };
        let (label, rows): (&'static str, Vec<Row>) = match trigger {
            '#' => {
                let (all, at) = self.targets(cx);
                let manifest = at.map(|at| all[at].id.as_str().to_string());
                let numbered = manifest
                    .as_ref()
                    .and_then(|manifest| self.issues.get(manifest))
                    .map(|issues| issues.iter().map(|issue| (false, issue.reference.clone(), issue.title.clone(), Some(issue.url.clone()))).collect())
                    .unwrap_or_default();
                if let Some(manifest) = manifest {
                    self.want_issues(&manifest, cx);
                }
                ("Tag an issue or pull request", numbers_for(numbered, &query))
            }
            '@' => {
                let lowered = query.to_lowercase();
                let mut rows: Vec<Row> = self
                    .jobs
                    .iter()
                    .filter(|(_, title)| lowered.is_empty() || title.to_lowercase().contains(&lowered))
                    .take(5)
                    .map(|(id, title)| Row {
                        group: None,
                        glyph: "square-terminal",
                        name: title.clone(),
                        says: String::new(),
                        pick: Pick::Tag { kind: TagKind::Session, id: id.clone(), title: title.clone(), url: None },
                    })
                    .collect();
                if let Some(first) = rows.first_mut() {
                    first.group = Some("Jobs");
                }
                ("Tag a Job", rows)
            }
            // The Operator takes no commands.
            _ => {
                self.offer = None;
                return;
            }
        };
        if rows.is_empty() {
            self.offer = None;
            return;
        }
        let current = self.offer.as_ref().filter(|held| held.label == label).map_or(0, |held| held.current.min(rows.len().saturating_sub(1)));
        self.offer = Some(Offer { label, rows, current, from, to: cursor, query, loading: false, within: None, held: Held::default() });
    }

    /// Read the repository's issues once, for `#`. The walk's own are the fixture's.
    fn want_issues(&mut self, manifest: &str, cx: &mut Context<Self>) {
        if !self.asked.insert(manifest.to_string()) {
            return;
        }
        let Some(fleet) = self.fleet.clone() else {
            if let Ok(list) = ipc::decode::<ipc::IssueList>("IssueList", include_bytes!("../../fixtures/composer-issues.json")) {
                self.issues.insert(manifest.to_string(), list.issues);
            }
            return;
        };
        let Some(port) = fleet.read(cx).held().port() else { return };
        let asked = manifest.to_string();
        let path = format!("/issues/{}", crate::job_view::wire::encoded(manifest));
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move { crate::dispatch::live::call(port, path, None).and_then(|bytes| ipc::decode::<ipc::IssueList>("IssueList", &bytes).map_err(|why| why.to_string())) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(list) = answer {
                    this.issues.insert(asked, list.issues);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A key while a list is open is the list's, and the box does not see it. True where taken.
    fn offer_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.offer.as_mut() else { return false };
        let last = offer.rows.len().saturating_sub(1);
        let was = offer.current;
        match key {
            "down" => offer.current = (offer.current + 1).min(last),
            "up" => offer.current = offer.current.saturating_sub(1),
            "escape" => self.offer = None,
            "enter" | "tab" => {
                let current = offer.current;
                self.choose(current, window, cx);
            }
            _ => return false,
        }
        if self.offer.as_ref().is_some_and(|offer| offer.current != was) && matches!(key, "up" | "down") {
            crate::ui_sounds::moved();
        }
        if let Some(offer) = self.offer.as_ref().filter(|_| matches!(key, "up" | "down")) {
            offer.keep_marked_in_view();
        }
        cx.notify();
        true
    }

    fn choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(offer) = self.offer.take() else { return };
        let Some(row) = offer.rows.get(at).cloned() else { return };
        let range = offer.from..offer.to;
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
            if let Pick::Tag { kind, id, title, url } = row.pick {
                if input.replace_range_with_token(range, chip_of(kind, &id, &title, url.as_deref()), window, cx).is_ok() {
                    input.insert(" ", window, cx);
                }
            }
        });
        self.typed(cx);
    }

    /// The keys the text box would take, which a list over it takes first. Esc with no list gives the keys back.
    fn list_key(&mut self, key: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.offer_key(key, window, cx) {
            cx.stop_propagation();
        } else if key == "escape" {
            cx.emit(Composed::Leave);
            cx.stop_propagation();
        }
    }

    // ---- attachments ---------------------------------------------------------------------

    /// Attach opens the Mac's own picker. On the walk's Fleet there is none to answer, so it adds a sample.
    fn attach(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            self.add_file("sample.rs".into(), "text/plain".into(), b"fn main() {}\n".to_vec(), cx);
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

    fn take_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        read_files(paths, cx, |this, name, media_type, bytes, cx| this.add_file(name, media_type, bytes, cx));
    }

    fn add_file(&mut self, name: String, media_type: String, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.next_key += 1;
        let preview = crate::job_view::composer::preview_of(&media_type, &bytes);
        self.files.push(Attached { key: self.next_key, name, media_type, bytes: Arc::new(bytes), preview });
        cx.notify();
    }

    fn remove_file(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.files.retain(|one| one.key != key);
        // The caret goes back to the box where the chip was.
        self.focus(window, cx);
        cx.notify();
    }

    /// A paste with a picture or files on the clipboard attaches them; a long text becomes a file.
    fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let mut next = self.next_key;
        let taken = clipped(item, || {
            next += 1;
            next
        });
        self.next_key = next;
        for (name, media_type, bytes) in taken.files.iter().cloned() {
            self.add_file(name, media_type, bytes, cx);
        }
        let any = taken.taken();
        if !taken.paths.is_empty() {
            self.take_paths(taken.paths, cx);
        }
        any
    }

    // ---- sending -------------------------------------------------------------------------

    fn nothing(&self, cx: &App) -> bool {
        self.input.read(cx).value().trim().is_empty() && self.files.is_empty()
    }

    /// Send: the words and what he put beside them go to the Operator as a talk, and stay in the box until Fleet answers.
    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending || self.nothing(cx) {
            return;
        }
        let about = self.tag.clone().map(|(about, _)| about);
        let kept = self.kept(cx);
        let words = sent_text(kept.text.trim());
        let mentions = kept.tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect();
        self.sending = true;
        self.refused = None;
        self.offer = None;
        let files = self.files.clone();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let attachments = cx.background_executor().spawn(async move { encoded_files(files) }).await;
            let _ = this.update(cx, |_, cx| cx.emit(Composed::Talk { words, about, attachments, mentions }));
        })
        .detach();
    }

    /// Fleet took the talk: the box empties, and keeps its tag, so the next words are about the same card.
    pub fn sent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = false;
        self.set_box(&Kept::default(), window, cx);
        crate::drafts::clear(&self.key(), cx);
        self.files.clear();
        self.refused = None;
        self.offer = None;
        cx.notify();
    }

    /// Fleet refused the send: the words stay, with why.
    pub fn refuse(&mut self, why: String, cx: &mut Context<Self>) {
        self.sending = false;
        self.refused = Some(why.into());
        cx.notify();
    }

    // ---- drawing -------------------------------------------------------------------------

    /// The tag Discuss put in the box: a link glyph, the card's title and an x that drops it.
    fn tag_chip(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (_, title) = self.tag.as_ref()?;
        let said = format!("About {title}");
        let drop = labelled(
            pressed(
                div()
                    .id("tag-drop")
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::SPACE_4))
                    .rounded_full()
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.look().color(token::OPERATOR_FAINT)))
                    .focusable()
                    .tab_stop(true)
                    .tooltip(tip("Talk about no card".into()))
                    .child(glyph("x", look.length(token::SPACE_3), look.color(token::OPERATOR))),
                cx,
                |this, _, cx| this.untag(cx),
            ),
            Role::Button,
            "Drop the tag",
        );
        Some(
            labelled(
                div().id("tag").flex().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).child(
                    div()
                        .flex_none()
                        .max_w_full()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_CONTROL))
                        .pl(look.length(token::SPACE_3))
                        .pr(look.length(token::SPACE_1))
                        .rounded_full()
                        .bg(look.color(token::OPERATOR_MUTED))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::OPERATOR_EDGE))
                        .text_color(look.color(token::OPERATOR))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child(glyph("link", look.length(token::SPACE_3) + look.length(token::SPACE_1), look.color(token::OPERATOR)))
                        .child(div().min_w_0().truncate().child(title.clone()))
                        .child(drop),
                ),
                Role::Group,
                said,
            )
            .into_any_element(),
        )
    }

    fn tray(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if self.files.is_empty() {
            return None;
        }
        let remove = |key: u64| -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App)> {
            let this = cx.entity().downgrade();
            Box::new(move |_, window, cx| {
                let _ = this.update(cx, |this, cx| this.remove_file(key, window, cx));
            })
        };
        Some(tray(&self.files, Vec::new(), look, remove))
    }
}

impl Render for OperatorComposer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let focused = self.has_keys(window, cx);
        let nothing = self.nothing(cx) || self.sending;
        let said = if self.sending { "Sending" } else { "Send" };
        let attach = labelled(
            icon_button(look, "attach", "paperclip", "Attach a file or a picture".into(), false, false).on_click(cx.listener(|this, _, window, cx| this.attach(window, cx))),
            Role::Button,
            "Attach file",
        );
        let send = labelled(
            send_face(look, nothing, said, window.modifiers().platform).when(!nothing, |button| button.on_click(cx.listener(|this, _, window, cx| this.send(window, cx)))),
            Role::Button,
            said,
        );
        let words = div()
            .id("message")
            .key_context(crate::keymap::TEXT_BOX)
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if !action.secondary && !action.shift && this.offer_key("enter", window, cx) {
                    cx.stop_propagation();
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
                words_area(&self.input, look, move |item, _, cx| view.update(cx, |this, cx| this.paste(item, cx)).unwrap_or(false))
            });
        let bar = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(attach)
            .child(div().flex_1())
            .child(send);
        let refused = self.refused.clone().map(|why| {
            labelled(
                div().id("compose-refused").px(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()),
                Role::Alert,
                why,
            )
        });
        let offer = self.offer.as_ref().map(|offer| {
            offer_list(offer, look, |at| {
                let this = cx.entity().downgrade();
                Box::new(move |_, window, cx| {
                    let _ = this.update(cx, |this, cx| this.choose(at, window, cx));
                })
            }, None)
        });
        // The same name as the Job's box has, so what stands over the foot of a page, a toast, lifts clear of it.
        let frame = labelled(
            div()
                .id("message-box")
                .track_focus(&self.frame)
                .relative()
                .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.take_paths(paths.paths().to_vec(), cx)))
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if focused { token::BORDER_DEFAULT } else { token::BORDER_SUBTLE }))
                .bg(look.color(token::BG_SUNKEN))
                .children(offer)
                .children(self.tag_chip(look, cx))
                .children(self.tray(look, cx))
                .child(labelled(words, Role::TextInput, "Message"))
                .child(bar),
            Role::Group,
            "Message box",
        );
        div()
            .flex_none()
            .pt(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_4))
            .on_modifiers_changed(cx.listener(|_, _: &ModifiersChangedEvent, _, cx| cx.notify()))
            .child(div().w_full().flex().flex_col().gap(look.length(token::SPACE_2)).children(refused).child(frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext) -> (Entity<OperatorComposer>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (composer, cx) = cx.add_window_view(|window, cx| OperatorComposer::new(None, window, cx));
        (composer, cx)
    }

    /// What the box asked of the page, as it asked it.
    fn hearing(composer: &Entity<OperatorComposer>, cx: &mut VisualTestContext) -> (std::rc::Rc<std::cell::RefCell<Vec<Composed>>>, Subscription) {
        let heard = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = heard.clone();
        let held = cx.update(|_, cx| cx.subscribe(composer, move |_, event: &Composed, _| seen.borrow_mut().push(event.clone())));
        (heard, held)
    }

    #[gpui::test]
    fn a_send_is_a_talk_about_no_card_with_the_files_he_put_beside_the_words(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        let (heard, _held) = hearing(&composer, cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.input.update(cx, |input, cx| input.set_value("This toast is wrong", window, cx));
            composer.add_file("shot.png".into(), "image/png".into(), b"png".to_vec(), cx);
            composer.send(window, cx);
        });
        cx.run_until_parked();
        assert!(
            matches!(heard.borrow().as_slice(), [Composed::Talk { words, about: None, attachments, .. }] if words == "This toast is wrong" && attachments.iter().map(|one| one.name.as_str()).collect::<Vec<_>>() == ["shot.png"]),
            "{:?}",
            heard.borrow()
        );
        heard.borrow_mut().clear();
        composer.update_in(cx, |composer, window, cx| {
            composer.sent(window, cx);
            assert!(composer.files().is_empty(), "a sent talk empties the tray");
            composer.add_file("shot.png".into(), "image/png".into(), b"png".to_vec(), cx);
            composer.send(window, cx);
        });
        cx.run_until_parked();
        assert!(matches!(heard.borrow().as_slice(), [Composed::Talk { words, attachments, .. }] if words.is_empty() && attachments.len() == 1), "a picture alone is a talk too: {:?}", heard.borrow());
    }

    #[gpui::test]
    fn a_pasted_picture_goes_with_the_talk_as_an_attached_one_does(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        let (heard, _held) = hearing(&composer, cx);
        composer.update_in(cx, |composer, window, cx| {
            let picture = gpui::Image::from_bytes(gpui::ImageFormat::Png, b"png".to_vec());
            assert!(composer.paste(&ClipboardItem::new_image(&picture), cx), "the paste is taken, not typed");
            composer.attach(window, cx);
            composer.input.update(cx, |input, cx| input.set_value("Both of these", window, cx));
            composer.send(window, cx);
        });
        cx.run_until_parked();
        let names = |attachments: &[SessionUpload]| attachments.iter().map(|one| (one.name.clone(), one.media_type.clone())).collect::<Vec<_>>();
        assert!(
            matches!(heard.borrow().as_slice(), [Composed::Talk { attachments, .. }] if names(attachments) == [("Pasted file.png".to_string(), "image/png".to_string()), ("sample.rs".to_string(), "text/plain".to_string())]),
            "the pasted picture, then the attached file: {:?}",
            heard.borrow()
        );
    }

    #[gpui::test]
    fn an_empty_box_sends_nothing(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        let sent = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = sent.clone();
        let _held = cx.update(|_, cx| cx.subscribe(&composer, move |_, event: &Composed, _| seen.borrow_mut().push(event.clone())));
        composer.update_in(cx, |composer, window, cx| composer.send(window, cx));
        cx.run_until_parked();
        assert!(sent.borrow().is_empty());
        assert!(!composer.read_with(cx, |composer, _| composer.is_sending()));
    }

    #[gpui::test]
    fn a_tagged_send_is_a_talk_about_its_card_with_its_files_and_keeps_the_tag(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        let (heard, _held) = hearing(&composer, cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.tag("turn:31".into(), "Two Jobs are changing paging.rs".into(), window, cx);
            composer.input.update(cx, |input, cx| input.set_value("Why hold Pocket?", window, cx));
            composer.add_file("shot.png".into(), "image/png".into(), b"png".to_vec(), cx);
            composer.send(window, cx);
        });
        cx.run_until_parked();
        assert!(
            matches!(heard.borrow().as_slice(), [Composed::Talk { words, about: Some(about), attachments, .. }] if words == "Why hold Pocket?" && about == "turn:31" && attachments.len() == 1),
            "{:?}",
            heard.borrow()
        );
        composer.update_in(cx, |composer, window, cx| {
            composer.sent(window, cx);
            assert_eq!(composer.words(cx), "", "a sent talk empties the box");
            assert!(composer.tagged().is_some(), "and keeps the tag, for the next words");
            composer.untag(cx);
            assert!(composer.tagged().is_none());
        });
    }

    #[gpui::test]
    fn the_words_stay_until_fleet_answers_and_go_with_the_answer(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.input.update(cx, |input, cx| input.set_value("A thought", window, cx));
            composer.send(window, cx);
        });
        cx.run_until_parked();
        composer.update_in(cx, |composer, window, cx| {
            assert!(composer.is_sending());
            assert_eq!(composer.words(cx), "A thought", "the box keeps its words while Fleet is asked");
            composer.refuse("Fleet did not take it".into(), cx);
            assert_eq!(composer.words(cx), "A thought", "a refusal keeps the words");
            assert_eq!(composer.refused().map(|why| why.as_ref()), Some("Fleet did not take it"));
            composer.sent(window, cx);
            assert_eq!(composer.words(cx), "");
            assert_eq!(composer.refused(), None);
        });
        assert!(cx.update(|_, cx| crate::drafts::kept(KEEP, cx)).is_none(), "an answered send leaves nothing kept");
    }

    #[gpui::test]
    fn what_was_typed_is_kept_for_the_place_and_comes_back(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        // A value set from code does not say it changed, as typing does; the box is told what the keys would have told it.
        composer.update_in(cx, |composer, window, cx| {
            composer.input.update(cx, |input, cx| input.set_value("Half a thought", window, cx));
            composer.typed(cx);
        });
        cx.run_until_parked();
        let (again, cx) = {
            let (second, cx) = cx.add_window_view(|window, cx| OperatorComposer::new(None, window, cx));
            (second, cx)
        };
        assert_eq!(again.read_with(cx, |composer, cx| composer.words(cx)), "Half a thought");
    }

    #[gpui::test]
    fn words_typed_with_a_card_tagged_are_kept_for_that_card_and_come_back_on_its_next_discuss(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.tag("turn:31".into(), "Two Jobs are changing paging.rs".into(), window, cx);
            composer.input.update(cx, |input, cx| input.set_value("Why hold Pocket?", window, cx));
            composer.typed(cx);
        });
        cx.update(|_, cx| {
            assert_eq!(crate::drafts::kept(KEEP, cx), None, "not the page's");
            assert!(crate::drafts::kept("operator:compose:turn:31", cx).is_some(), "the card's");
        });
        // The box is made again, as at a restart: the page's draft comes back, and the tag does not.
        let (again, cx) = cx.add_window_view(|window, cx| OperatorComposer::new(None, window, cx));
        assert_eq!(again.read_with(cx, |composer, cx| composer.words(cx)), "");
        again.update_in(cx, |composer, window, cx| {
            composer.tag("turn:31".into(), "Two Jobs are changing paging.rs".into(), window, cx);
            assert_eq!(composer.words(cx), "Why hold Pocket?", "Discuss on the card brings its words back");
            // Dropping the tag keeps the words for the page.
            composer.untag(cx);
            assert_eq!(composer.words(cx), "Why hold Pocket?");
        });
        cx.update(|_, cx| {
            assert!(crate::drafts::kept(KEEP, cx).is_some());
            assert_eq!(crate::drafts::kept("operator:compose:turn:31", cx), None);
        });
    }
}
