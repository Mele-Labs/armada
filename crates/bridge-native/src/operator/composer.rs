//! The box pinned to the foot of the Operator's page: where he throws an idea, a feature or a bug at the
//! Operator without opening a Job. A send is a dispatch (`dispatch_job`), unheld, which the Operator may start,
//! fold into a live Job, ask about or keep as an idea; a toast says which when `operator.dispatched` arrives.
//!
//! **It is the Job's message box, not a copy of it.** The words, the chips in them, the tray of files, the list
//! `@` and `#` open, Send and the paste are `job_view::composer` and `job_view::offer`, drawn here from this
//! entity's state. What a Job's box has and this has not: its permission and model pickers, Talk, sketches and
//! quick lines, which are a running Job's.
//!
//! **The kind chips are hints.** One picked rides as `kind`; none sends none. A chip never picks a workflow
//! and never changes the route, and pressing the picked chip again unpicks it.
//!
//! **The words are kept** under [`KEEP`], chips and all, so leaving the page does not lose them. They go when
//! Fleet answers the send, and stay where it refuses.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, Context, Div, Entity, EventEmitter, ExternalPaths, FocusHandle, Focusable as _,
    FontWeight, KeyDownEvent, ModifiersChangedEvent, PathPromptOptions, Role, SharedString, Stateful, Subscription, Window,
};
use gpui_component::input::{Enter, Escape, IndentInline, InputEvent, MoveDown, MoveUp, TextareaState};
use ipc::{DispatchJob, DispatchKind, DispatchMessage, DispatchThen, JobDispatched, StartSession, TagKind, TrackedIssue};

use super::face::{dispatch_kind, dispatch_kind_tip, DISPATCH_KINDS};
use crate::dispatch::live::{served, Served};
use crate::fleet::FleetConnection;
use crate::job_view::composer::{clipped, encoded_files, read_files, send_face, tray, words_area, Attached, Kept, KeptTag};
use crate::job_view::format::sent_text;
use crate::job_view::head::{icon_button, tip};
use crate::job_view::offer::{chip_of, mention_of, numbers_for, offer_list, token_at, Offer, Pick, Row};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::glyph;

/// The key the words are kept under, as Electron's `useKept` names a place.
pub const KEEP: &str = "operator:compose";

/// A dispatch ready to go, and the words it carries, which a toast names it by while the Operator holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Outgoing {
    pub dispatch: DispatchJob,
    pub words: String,
}

/// What the box asks of the page.
#[derive(Clone, Debug, PartialEq)]
pub enum Composed {
    Send(Outgoing),
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
    /// The kind he picked, which rides as `kind`; none is none.
    kind: Option<DispatchKind>,
    /// The repository he picked, by the name its chip carries; none is the first the window is scoped to.
    repository: Option<SharedString>,
    /// Sent, and Fleet has not answered.
    sending: bool,
    /// Fleet's sentence where it refused the send.
    refused: Option<SharedString>,
    /// The box is being set to a draft, which is not the person typing.
    setting: bool,
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
        if let Some(kept) = Self::left(cx) {
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
            kind: None,
            repository: None,
            sending: false,
            refused: None,
            setting: false,
            jobs: Vec::new(),
            fleet,
            issues: HashMap::new(),
            asked: HashSet::new(),
            _subs: vec![typed],
        }
    }

    // ---- what it holds -------------------------------------------------------------------

    /// The words and chips he left, if any stand.
    fn left(cx: &App) -> Option<Kept> {
        let kept = crate::drafts::kept(KEEP, cx)?;
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

    pub fn kind(&self) -> Option<DispatchKind> {
        self.kind
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
        match (kept.text.is_empty(), ipc::encode(&kept)) {
            (false, Ok(encoded)) => crate::drafts::keep(KEEP, &encoded, cx),
            _ => crate::drafts::clear(KEEP, cx),
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

    /// The repositories he may send to, narrowed to the project the window is scoped to, and the one picked.
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
        let at = self
            .repository
            .as_ref()
            .and_then(|name| all.iter().position(|one| &one.name == name))
            .or_else(|| scoped.and_then(|names| names.first().and_then(|first| all.iter().position(|one| &one.name == first))))
            .or_else(|| (!all.is_empty()).then_some(0));
        (all, at)
    }

    /// Pick the repository the next send is for; the one picked stays picked.
    pub fn pick_repository(&mut self, name: SharedString, cx: &mut Context<Self>) {
        self.repository = Some(name);
        self.offer = None;
        cx.notify();
    }

    /// Pick a kind, or unpick it with a second press.
    pub fn pick_kind(&mut self, kind: DispatchKind, cx: &mut Context<Self>) {
        self.kind = if self.kind == Some(kind) { None } else { Some(kind) };
        cx.notify();
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
        self.offer = Some(Offer { label, rows, current, from, to: cursor, query, loading: false });
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

    /// What a send would be, or why it cannot be yet. Nothing in the box is nothing to send.
    fn outgoing(&self, files: Vec<ipc::SessionUpload>, cx: &App) -> Result<Outgoing, SharedString> {
        let kept = self.kept(cx);
        let text = sent_text(kept.text.trim());
        let (all, at) = self.targets(cx);
        let Some(target) = at.map(|at| &all[at]) else { return Err("Fleet serves no repository".into()) };
        let mentions = kept.tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect();
        let start = StartSession {
            manifest_id: target.id.clone(),
            title: None,
            model: None,
            effort: None,
            mode: None,
            fork: None,
            root: None,
            request: Some(text.clone()),
            held: false,
            after: None,
            project_rows: Vec::new(),
            on_your_words: None,
            project: crate::dispatch::screen::scope(cx).and_then(|(id, _)| id).map(|id| id.to_string()),
        };
        // His words are the brief and the first message, so nothing is lost whoever starts the Job.
        let then = vec![DispatchThen::Message(DispatchMessage { text: text.clone(), attachments: files, mentions, sketches: Vec::new() })];
        Ok(Outgoing { dispatch: DispatchJob { start, kind: self.kind, then }, words: text })
    }

    fn nothing(&self, cx: &App) -> bool {
        self.input.read(cx).value().trim().is_empty() && self.files.is_empty()
    }

    /// Send: the words go to Fleet as a dispatch, unheld, and stay in the box until Fleet answers.
    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending || self.nothing(cx) {
            return;
        }
        // A target is wanted before the files are read, so a refusal costs nothing.
        if let Err(why) = self.outgoing(Vec::new(), cx) {
            self.refused = Some(why);
            cx.notify();
            return;
        }
        self.sending = true;
        self.refused = None;
        self.offer = None;
        let files = self.files.clone();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let uploads = cx.background_executor().spawn(async move { encoded_files(files) }).await;
            let _ = this.update(cx, |this, cx| match this.outgoing(uploads, cx) {
                Ok(outgoing) => cx.emit(Composed::Send(outgoing)),
                Err(why) => {
                    this.sending = false;
                    this.refused = Some(why);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Fleet answered. A dispatch taken empties the box and drops the kind; a refusal keeps all of it, with why.
    pub fn answered(&mut self, answer: Result<&JobDispatched, String>, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = false;
        match answer {
            Ok(_) => {
                self.set_box(&Kept::default(), window, cx);
                crate::drafts::clear(KEEP, cx);
                self.files.clear();
                self.kind = None;
                self.refused = None;
                self.offer = None;
            }
            Err(why) => self.refused = Some(why.into()),
        }
        cx.notify();
    }

    // ---- drawing -------------------------------------------------------------------------

    /// A chip he presses: a kind or a repository. Focusable, so Tab reaches it and Enter or Space presses it.
    fn chip(id: impl Into<SharedString>, look: &Look, hue: Token, ground: Token, picked: bool) -> Stateful<Div> {
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let (back, ink) = if picked { (look.color(hue), look.color(token::FG_INVERSE)) } else { (look.color(ground), look.color(hue)) };
        div()
            .id(id.into())
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .bg(back)
            .text_color(ink)
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .cursor_pointer()
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
    }

    fn kind_chips(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let chips = DISPATCH_KINDS.into_iter().map(|kind| {
            let face = dispatch_kind(kind);
            let picked = self.kind == Some(kind);
            let chip = Self::chip(format!("kind-{}", kind.as_wire()), look, face.hue, face.ground, picked)
                .aria_toggled(if picked { gpui::accesskit::Toggled::True } else { gpui::accesskit::Toggled::False })
                .tooltip(tip(dispatch_kind_tip(kind).into()))
                .child(glyph(face.glyph, look.length(token::SPACE_3) + look.length(token::SPACE_1), if picked { look.color(token::FG_INVERSE) } else { look.color(face.hue) }))
                .child(face.word);
            labelled(pressed(chip, cx, move |this, _, cx| this.pick_kind(kind, cx)), Role::CheckBox, face.word)
        });
        labelled(div().id("kinds").flex().items_center().gap(look.length(token::SPACE_1)).children(chips), Role::Group, "Kind").into_any_element()
    }

    /// The repositories the send may go to, one picked: his project's chips.
    fn project_chips(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (all, at) = self.targets(cx);
        if all.is_empty() {
            return None;
        }
        let chips = all.iter().enumerate().map(|(n, one)| {
            let picked = at == Some(n);
            let name = one.name.clone();
            let chip = Self::chip(format!("project-{n}"), look, token::FG_MUTED, token::BG_RAISED, false)
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .border_color(if picked { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE) })
                .bg(if picked { look.color(token::ACCENT_MUTED) } else { gpui::transparent_black() })
                .text_color(if picked { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
                .aria_selected(picked)
                .child(glyph("folder-git-2", look.length(token::SPACE_3) + look.length(token::SPACE_1), look.color(token::FG_MUTED)))
                .child(name.clone());
            let picks = name.clone();
            labelled(pressed(chip, cx, move |this, _, cx| this.pick_repository(picks.clone(), cx)), Role::RadioButton, name)
        });
        Some(
            labelled(
                div()
                    .id("projects")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_3))
                    .pt(look.length(token::SPACE_3))
                    .children(chips),
                Role::RadioGroup,
                "Project",
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
            .child(self.kind_chips(look, cx))
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
            })
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
                .children(self.project_chips(look, cx))
                .children(self.tray(look, cx))
                .child(labelled(words, Role::TextInput, "Message"))
                .child(bar),
            Role::Group,
            "Message box",
        );
        div()
            .flex_none()
            .flex()
            .justify_center()
            .px(look.length(token::SPACE_6))
            .pt(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_4))
            .on_modifiers_changed(cx.listener(|_, _: &ModifiersChangedEvent, _, cx| cx.notify()))
            .child(div().w_full().max_w(look.length(token::SPACE_12) * 20.).flex().flex_col().gap(look.length(token::SPACE_2)).children(refused).child(frame))
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

    #[gpui::test]
    fn no_chip_sends_no_kind_and_a_picked_chip_rides_as_the_kind(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.input.update(cx, |input, cx| input.set_value("Keep the scrollback", window, cx));
            let plain = composer.outgoing(Vec::new(), cx).expect("a send");
            assert_eq!(plain.dispatch.kind, None);
            assert!(!plain.dispatch.start.held, "a dispatch from the composer is unheld");
            composer.pick_kind(DispatchKind::Bug, cx);
            assert_eq!(composer.outgoing(Vec::new(), cx).expect("a send").dispatch.kind, Some(DispatchKind::Bug));
        });
    }

    #[gpui::test]
    fn the_picked_chip_pressed_again_is_unpicked(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        composer.update(cx, |composer, cx| {
            composer.pick_kind(DispatchKind::Idea, cx);
            assert_eq!(composer.kind(), Some(DispatchKind::Idea));
            composer.pick_kind(DispatchKind::Feature, cx);
            assert_eq!(composer.kind(), Some(DispatchKind::Feature), "another chip takes its place");
            composer.pick_kind(DispatchKind::Feature, cx);
            assert_eq!(composer.kind(), None);
        });
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
    fn the_words_stay_until_fleet_answers_and_go_with_the_answer(cx: &mut TestAppContext) {
        let (composer, cx) = open(cx);
        composer.update_in(cx, |composer, window, cx| {
            composer.input.update(cx, |input, cx| input.set_value("A thought", window, cx));
            composer.pick_kind(DispatchKind::Idea, cx);
            composer.send(window, cx);
        });
        cx.run_until_parked();
        composer.update_in(cx, |composer, window, cx| {
            assert!(composer.is_sending());
            assert_eq!(composer.words(cx), "A thought", "the box keeps its words while Fleet is asked");
            composer.answered(Err("Fleet did not take it".into()), window, cx);
            assert_eq!(composer.words(cx), "A thought", "a refusal keeps the words");
            assert_eq!(composer.kind(), Some(DispatchKind::Idea), "and the kind");
            assert_eq!(composer.refused().map(|why| why.as_ref()), Some("Fleet did not take it"));
            let taken = JobDispatched { session: None, operator: Some("dispatch:1".into()), parked: None };
            composer.answered(Ok(&taken), window, cx);
            assert_eq!(composer.words(cx), "");
            assert_eq!(composer.kind(), None);
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
}
