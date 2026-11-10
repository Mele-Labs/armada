//! The Teach panel (`shared-panel/teach.tsx`, `TeachPanel.tsx`): what Teach was pressed on, a box for
//! what to do about it, the reach, then what Fleet understood, shown back before anything is kept.
//!
//! **Fleet reads the words and the person's instruction into a proposal (`POST /teachings/read`); a
//! save (`POST /teachings`) keeps what was shown.** Nothing is filed from here. The panel is a side
//! sheet held off the room's edges, so the thread stays in view behind it; Esc puts it away.

use gpui::{
    div, prelude::*, Animation, AnimationExt, AnyElement, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    KeyDownEvent, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{
    ProposedFleet, ReadTeaching, SaveTeaching, SavedTeaching, TeachingProposal, TeachingReach, TeachingSource, TeachingSourceKind,
};

use super::acts::{button, kbd, Tone};
use super::menus::{disable_label, Teach};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// The most of a thing's text a source carries: Fleet refuses more than 200 characters.
const MOST_TEXT: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    /// The source and the box.
    Writing,
    /// Fleet is reading them.
    Reading,
    /// What Fleet understood, to save or change.
    Shown,
}

/// The panel while it is open.
pub(super) struct Open {
    source: TeachingSource,
    say: Entity<TextareaState>,
    everywhere: bool,
    stage: Stage,
    proposal: Option<TeachingProposal>,
    saving: bool,
    job: SharedString,
    repository: String,
    root: Option<String>,
    focus: FocusHandle,
    chip: FocusHandle,
    forward: FocusHandle,
    change: FocusHandle,
    close: FocusHandle,
    back: FocusHandle,
    /// Each read takes a number; an answer to an older one, or to a panel since closed, is not drawn.
    latest: u64,
    _typing: Subscription,
}

/// One name and its value in a section of what Fleet understood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    name: Option<String>,
    value: String,
    mono_name: bool,
    mono_value: bool,
    /// A colour the value is, drawn as a swatch beside it.
    swatch: Option<String>,
}

/// A section of the reading: Agents, Bridge, Fleet, Rule, Theme tweak or Layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Section {
    caption: &'static str,
    facts: Vec<Fact>,
    /// A plain line under the facts: what Fleet could not test.
    line: Option<String>,
    /// A list under its own caption: the places a colour is drawn.
    also: Vec<String>,
}

fn fact(name: Option<&str>, value: impl Into<String>) -> Fact {
    Fact { name: name.map(str::to_string), value: value.into(), mono_name: false, mono_value: false, swatch: None }
}

fn mono_fact(name: Option<&str>, value: impl Into<String>) -> Fact {
    Fact { mono_value: true, ..fact(name, value) }
}

/// What a source is called, as the panel's first section captions it (`TEACH_SOURCE`).
fn source_name(kind: TeachingSourceKind) -> &'static str {
    match kind {
        TeachingSourceKind::Words => "Words",
        TeachingSourceKind::Command => "Command",
        TeachingSourceKind::Notice => "Notice",
        TeachingSourceKind::Task => "Task",
        TeachingSourceKind::Check => "Check",
        TeachingSourceKind::PullRequest => "Pull request",
        TeachingSourceKind::Ticket => "Ticket",
        TeachingSourceKind::Panel => "Panel",
        TeachingSourceKind::Row => "Row",
        TeachingSourceKind::TitleBar => "Title bar",
    }
}

fn kind_of(word: &str) -> TeachingSourceKind {
    match word {
        "command" => TeachingSourceKind::Command,
        "notice" => TeachingSourceKind::Notice,
        "task" => TeachingSourceKind::Task,
        "check" => TeachingSourceKind::Check,
        "pull_request" => TeachingSourceKind::PullRequest,
        "ticket" => TeachingSourceKind::Ticket,
        _ => TeachingSourceKind::Words,
    }
}

/// What Teach was pressed on. A thing with no text of its own is named by its kind.
fn source_of(kind: TeachingSourceKind, text: &str, ids: Vec<String>) -> TeachingSource {
    let text = if text.trim().is_empty() { source_name(kind) } else { text };
    TeachingSource { kind, text: text.chars().take(MOST_TEXT).collect(), ids }
}

/// A notice's word as the menu names it, with a capital.
fn notice_name(word: &str) -> String {
    let name = disable_label(word).and_then(|label| label.strip_prefix("Disable ")).map_or_else(|| word.replace('_', " "), str::to_string);
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// What Fleet does with a notice, as a clause: "Batch, and tell only when it blocks a merge".
fn does_words(fleet: &ProposedFleet) -> String {
    let word = |does: ipc::Does| match does {
        ipc::Does::Tell => "Tell",
        ipc::Does::Quiet => "Quiet",
        ipc::Does::Batch => "Batch",
        ipc::Does::Call => "Call",
    };
    let instead = |does: ipc::Does| match does {
        ipc::Does::Tell => "tell",
        ipc::Does::Quiet => "stay quiet",
        ipc::Does::Batch => "batch",
        ipc::Does::Call => "call you",
    };
    let condition = |condition: ipc::Condition| match condition {
        ipc::Condition::BlocksMerge => "it blocks a merge",
        ipc::Condition::OpenPullRequest => "it has an open pull request",
        ipc::Condition::Working => "its agent is working",
        ipc::Condition::OwnerAway => "you are away",
    };
    std::iter::once(word(fleet.rule.does).to_string())
        .chain(fleet.rule.when.iter().map(|one| format!("{} only when {}", instead(one.does), condition(one.condition))))
        .collect::<Vec<_>>()
        .join(", and ")
}

/// Where a rule reaches, from the place Fleet wrote.
fn scope_words(scope: &ipc::RuleScope, repository: &str) -> String {
    match scope {
        ipc::RuleScope::Everywhere => "Every repository".to_string(),
        ipc::RuleScope::Job(_) => "This Job".to_string(),
        ipc::RuleScope::Repo(_) => repository.to_string(),
    }
}

/// Where an address came from, in words.
fn found_said(found: &ipc::AddressFound) -> String {
    match found {
        ipc::AddressFound::Settings => "Found in this repository's settings".to_string(),
        ipc::AddressFound::Person => "You said".to_string(),
        ipc::AddressFound::PullRequest(number) => format!("Found in pull request #{number}"),
    }
}

fn untestable_words(words: &[String]) -> Option<String> {
    (!words.is_empty()).then(|| format!("Fleet cannot test: {}", words.join(", ")))
}

/// What the panel shows back of a proposal: a section for each part Fleet understood, in the panel's order.
pub(super) fn reading_of(proposal: &TeachingProposal, repository: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let section = |caption, facts, line, also| Section { caption, facts, line, also };
    if let Some(note) = &proposal.note {
        sections.push(section("Agents", vec![fact(None, note.text.clone())], None, Vec::new()));
    }
    if let Some(words) = &proposal.words {
        let mut facts = vec![mono_fact(Some(&words.name), words.pattern.clone())];
        if let Some(address) = &words.address {
            facts.push(Fact { mono_name: true, ..mono_fact(Some(&words.example), address.template.replace("{match}", &words.example)) });
            facts.push(fact(None, found_said(&address.found)));
        }
        sections.push(section("Bridge", facts, None, Vec::new()));
    }
    if let Some(fleet) = &proposal.fleet {
        let mut facts = vec![fact(Some("Notice"), notice_name(&fleet.rule.notice))];
        if let Some(matches) = &fleet.rule.matches {
            facts.push(mono_fact(Some("Matches"), matches.clone()));
        }
        facts.push(fact(Some("Does"), does_words(fleet)));
        facts.push(fact(Some("Applies to"), scope_words(&fleet.rule.scope, repository)));
        sections.push(section("Fleet", facts, untestable_words(&fleet.untestable), Vec::new()));
    }
    if let Some(rule) = &proposal.rule {
        sections.push(section("Rule", vec![fact(Some("Name"), rule.rule.name.clone())], untestable_words(&rule.untestable), Vec::new()));
    }
    if let Some(tweak) = proposal.tweak.first() {
        let colour = |name, value: &str| Fact { swatch: Some(value.to_string()), ..mono_fact(Some(name), value) };
        sections.push(section(
            "Theme tweak",
            vec![fact(Some("Colour"), tweak.note.clone().unwrap_or_else(|| tweak.token.clone())), mono_fact(Some("Token"), tweak.token.clone()), colour("Was", &tweak.was), colour("Now", &tweak.now)],
            None,
            tweak.used_in.clone(),
        ));
    }
    if let Some(layout) = &proposal.layout {
        sections.push(section("Layout", vec![fact(Some("Hides"), layout.name.clone()), fact(Some("In"), "Layout")], None, Vec::new()));
    }
    sections
}

fn reach_of(everywhere: bool) -> TeachingReach {
    if everywhere {
        TeachingReach::Everywhere
    } else {
        TeachingReach::Repository
    }
}

/// `read_teaching`'s body for what the person wrote about it.
fn read_body(open: &Open, said: &str) -> ReadTeaching {
    ReadTeaching {
        words: if open.source.kind == TeachingSourceKind::Words { open.source.text.clone() } else { String::new() },
        message: String::new(),
        said: said.to_string(),
        repository: (!open.everywhere).then(|| open.root.clone()).flatten(),
        reach: reach_of(open.everywhere),
        session_id: Some(open.job.to_string()),
        source: Some(open.source.clone()),
    }
}

/// `save_teaching`'s body: the proposal as it was shown, and where it was taught.
fn save_body(open: &Open, proposal: &TeachingProposal) -> SaveTeaching {
    let mut proposal = proposal.clone();
    // The panel shows one colour, so one colour is what is kept.
    proposal.tweak.truncate(1);
    SaveTeaching {
        proposal,
        reach: reach_of(open.everywhere),
        repository: (!open.everywhere).then(|| open.root.clone()).flatten(),
        session_id: Some(open.job.to_string()),
        cite: Some(format!("Teach, {}", source_name(open.source.kind).to_lowercase())),
        source: Some(open.source.clone()),
    }
}

/// What a walk's Fleet-less panel reads the instruction as: the words, as a note for agents.
fn mock_proposal(said: &str) -> TeachingProposal {
    TeachingProposal {
        note: Some(ipc::TeachingNote { section: ipc::LogbookSection::Habits, kind: ipc::LogbookKind::Habit, text: said.trim().to_string() }),
        ..TeachingProposal::default()
    }
}

/// A call to Fleet off the main thread: the answer's bytes, or Fleet's reason.
fn ask(port: u16, route: &'static str, body: Result<String, String>) -> Result<Vec<u8>, String> {
    let body = body?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(crate::transport::post(port, route, body))?;
    if (200..300).contains(&status) {
        return Ok(bytes);
    }
    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
}

impl JobView {
    /// Teach was pressed: the panel opens over the Job, its box holding what was left in it before.
    pub(super) fn teach_open(&mut self, teach: Teach, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let record = self.record(&job, cx);
        let repository = record.map(|record| record.manifest_id.as_ref().map_or_else(|| record.cwd.clone(), |id| id.as_str().to_string())).unwrap_or_default();
        let root = record.map(|record| record.cwd.clone());
        let kept = self.drafts.get(&format!("teach:{job}")).to_string();
        let say = cx.new(|cx| TextareaState::new(window, cx).auto_grow(4, 12));
        say.update(cx, |say, cx| say.set_value(kept, window, cx));
        let typing = cx.subscribe_in(&say, window, |this, say, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let Some(job) = this.shown.clone() else { return };
                let text = say.read(cx).value().to_string();
                if this.drafts.set(&format!("teach:{job}"), &text) {
                    this.keep_soon(cx);
                }
                cx.notify();
            }
        });
        let back = window.focused(cx).unwrap_or_else(|| self.focus.clone());
        let source = source_of(kind_of(teach.kind), &teach.text, teach.ids.clone());
        cx.emit(teach);
        self.teach_clock += 1;
        let latest = self.teach_clock;
        self.teach_said = None;
        self.teach = Some(Open {
            source,
            say: say.clone(),
            everywhere: false,
            stage: Stage::Writing,
            proposal: None,
            saving: false,
            job,
            repository,
            root,
            focus: cx.focus_handle(),
            chip: cx.focus_handle(),
            forward: cx.focus_handle(),
            change: cx.focus_handle(),
            close: cx.focus_handle(),
            back,
            latest,
            _typing: typing,
        });
        say.update(cx, |say, cx| say.focus(window, cx));
        cx.notify();
    }

    /// The panel is put away, and focus goes back to where Teach was pressed.
    pub(super) fn teach_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = self.teach.take() {
            window.focus(&open.back, cx);
        }
        cx.notify();
    }

    fn teach_stage(&mut self, stage: Stage, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.teach.as_mut() else { return };
        open.stage = stage;
        match stage {
            Stage::Writing => {
                let say = open.say.clone();
                say.update(cx, |say, cx| say.focus(window, cx));
            }
            Stage::Shown => window.focus(&open.forward, cx),
            Stage::Reading => window.focus(&open.focus, cx),
        }
        cx.notify();
    }

    /// Teach: Fleet reads what was pressed on and what was written into a proposal.
    pub(super) fn teach_read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.teach.as_mut() else { return };
        let said = open.say.read(cx).value().to_string();
        if open.stage != Stage::Writing || said.trim().is_empty() {
            return;
        }
        self.teach_clock += 1;
        open.latest = self.teach_clock;
        let mine = open.latest;
        let body = ipc::encode(&read_body(open, &said)).map_err(|why| format!("{why:?}"));
        self.teach_stage(Stage::Reading, window, cx);
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            // No Fleet: the walk reads the words back as a note for agents.
            if let Some(open) = self.teach.as_mut() {
                open.proposal = Some(mock_proposal(&said));
            }
            self.teach_stage(Stage::Shown, window, cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { ask(port, "/teachings/read", body) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.teach_read_done(mine, answer, window, cx));
        })
        .detach();
    }

    fn teach_read_done(&mut self, mine: u64, answer: Result<Vec<u8>, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.teach.as_mut() else { return };
        if open.latest != mine {
            return;
        }
        match answer.and_then(|bytes| ipc::decode::<TeachingProposal>("TeachingProposal", &bytes).map_err(|why| why.to_string())) {
            Ok(proposal) => {
                open.proposal = Some(proposal);
                self.teach_stage(Stage::Shown, window, cx);
            }
            Err(why) => {
                // Fleet's refusal stands under the box, which keeps what was written.
                self.teach_refused(why.into(), cx);
                self.teach_stage(Stage::Writing, window, cx);
            }
        }
    }

    fn teach_refused(&mut self, why: SharedString, cx: &mut Context<Self>) {
        self.teach_said = Some(why);
        cx.notify();
    }

    /// Change: back to the box, with the source kept.
    fn teach_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.teach_clock += 1;
        if let Some(open) = self.teach.as_mut() {
            open.latest = self.teach_clock;
        }
        self.teach_said = None;
        self.teach_stage(Stage::Writing, window, cx);
    }

    /// Save: Fleet keeps what was shown, and the box starts empty the next time.
    pub(super) fn teach_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.teach.as_mut() else { return };
        let Some(proposal) = open.proposal.clone() else { return };
        if open.stage != Stage::Shown || open.saving {
            return;
        }
        open.saving = true;
        let job = open.job.clone();
        let body = ipc::encode(&save_body(open, &proposal)).map_err(|why| format!("{why:?}"));
        let mine = open.latest;
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            self.teach_saved(&job, window, cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { ask(port, "/teachings", body) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.teach.as_ref().is_none_or(|open| open.latest != mine) {
                    return;
                }
                match answer.and_then(|bytes| ipc::decode::<SavedTeaching>("SavedTeaching", &bytes).map_err(|why| why.to_string())) {
                    Ok(_) => this.teach_saved(&job, window, cx),
                    Err(why) => {
                        if let Some(open) = this.teach.as_mut() {
                            open.saving = false;
                        }
                        this.teach_refused(why.into(), cx);
                    }
                }
            });
        })
        .detach();
    }

    fn teach_saved(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        // What was written is now taught.
        if self.drafts.set(&format!("teach:{job}"), "") {
            self.keep_soon(cx);
        }
        self.teach_said = None;
        self.teach_close(window, cx);
    }

    fn teach_keys(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(stage) = self.teach.as_ref().map(|open| open.stage) else { return };
        let key = event.keystroke.key.as_str();
        let forward = key == "enter" && (event.keystroke.modifiers.platform || event.keystroke.modifiers.control);
        if forward {
            cx.stop_propagation();
            match stage {
                Stage::Writing => self.teach_read(window, cx),
                Stage::Shown => self.teach_save(window, cx),
                Stage::Reading => {}
            }
        } else if key == "escape" {
            cx.stop_propagation();
            self.teach_close(window, cx);
        } else if key == "tab" {
            let order = self.teach_order(cx);
            super::pop::trap_tab(&order, event.keystroke.modifiers.shift, window, cx);
        }
    }

    /// The panel's own stops, in the order Tab takes them.
    fn teach_order(&self, cx: &Context<Self>) -> Vec<FocusHandle> {
        let Some(open) = self.teach.as_ref() else { return Vec::new() };
        let mut order = vec![open.close.clone()];
        match open.stage {
            Stage::Writing => {
                order.push(open.say.read(cx).focus_handle(cx));
                order.push(open.chip.clone());
                order.push(open.forward.clone());
            }
            Stage::Reading => {}
            Stage::Shown => {
                order.push(open.chip.clone());
                order.push(open.change.clone());
                order.push(open.forward.clone());
            }
        }
        order
    }

    /// The panel, held off the room's edges on its trailing side, over the thread.
    pub(super) fn render_teach(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let open = self.teach.as_ref()?;
        let look = cx.look();
        let on_bridge = matches!(open.source.kind, TeachingSourceKind::Panel | TeachingSourceKind::Row | TeachingSourceKind::TitleBar);
        let bare = matches!(open.source.kind, TeachingSourceKind::Panel | TeachingSourceKind::TitleBar);
        let ready = !open.say.read(cx).value().trim().is_empty();
        let margin = look.length(token::SPACE_4);
        let caption = |said: &str| div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(said.to_string()));
        let section = |name: &str, children: Vec<AnyElement>| {
            labelled(
                div().id(SharedString::from(format!("teach-section:{name}"))).flex().flex_col().gap(look.length(token::SPACE_2)).min_w_0().child(caption(name)).children(children),
                Role::Group,
                name.to_string(),
            )
            .into_any_element()
        };
        let text = |said: String, mono_font: bool| {
            let line = div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).min_w_0().child(SharedString::from(said));
            if mono_font {
                line.font_family(mono(look))
            } else {
                line
            }
        };
        let name_of = if open.source.kind == TeachingSourceKind::Panel { format!("Panel {}", open.source.text) } else { source_name(open.source.kind).to_string() };
        let words_mono = !matches!(open.source.kind, TeachingSourceKind::Words | TeachingSourceKind::Notice | TeachingSourceKind::Task | TeachingSourceKind::Row);
        let mut body: Vec<AnyElement> = Vec::new();
        body.push(section(
            &name_of,
            if bare {
                Vec::new()
            } else {
                vec![text(open.source.text.clone(), words_mono)
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .into_any_element()]
            },
        ));
        if open.stage == Stage::Writing {
            body.push(
                labelled(
                    div()
                        .id("teach-instruction")
                        .key_context(crate::keymap::TEXT_BOX)
                        .w_full()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(Textarea::new(&open.say).appearance(false).aria_label("Instruction")),
                    Role::TextInput,
                    "Instruction",
                )
                .into_any_element(),
            );
        } else {
            body.push(section("Instruction", vec![text(open.say.read(cx).value().to_string(), false).into_any_element()]));
        }
        if !on_bridge {
            let everywhere = open.everywhere;
            let said = if everywhere { "Every repository".to_string() } else { open.repository.clone() };
            let live = open.stage == Stage::Writing;
            body.push(
                div()
                    .flex()
                    .child(
                        labelled(
                            div()
                                .id("teach-reach")
                                .track_focus(&open.chip)
                                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                                .flex()
                                .items_center()
                                .min_w_0()
                                .max_w_full()
                                .h(look.length(token::SPACE_6))
                                .px(look.length(token::SPACE_3))
                                .rounded(look.length(token::RADIUS_SM))
                                .border(look.length(token::BORDER_WIDTH))
                                .border_color(if everywhere { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                                .bg(look.color(token::BG_RAISED))
                                .text_size(look.length(token::TEXT_SM))
                                .text_color(look.color(token::FG_DEFAULT))
                                .when(live, |chip| chip.cursor_pointer().hover(|style| style.border_color(look.color(token::BORDER_STRONG))))
                                .tooltip(tip("Where it applies"))
                                .aria_toggled(if everywhere { gpui::Toggled::True } else { gpui::Toggled::False })
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                                        cx.stop_propagation();
                                        this.teach_reach(window, cx);
                                    }
                                }))
                                .on_click(cx.listener(|this, _, window, cx| this.teach_reach(window, cx)))
                                .child(div().truncate().when(!everywhere, |chip| chip.font_family(mono(look))).child(SharedString::from(said.clone()))),
                            Role::Button,
                            "Where it applies",
                        ),
                    )
                    .into_any_element(),
            );
        }
        if let Some(why) = &self.teach_said {
            body.push(
                labelled(
                    div().id("teach-refused").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::STATUS_ESCALATED)).child(why.clone()),
                    Role::Status,
                    why.to_string(),
                )
                .into_any_element(),
            );
        }
        match open.stage {
            Stage::Writing => body.push(
                div()
                    .flex()
                    .items_center()
                    .child(labelled(
                        button(look, "teach-go", "Teach", Tone::Primary, false, None, ready)
                            .track_focus(&open.forward)
                            .when(!ready, |button| button.opacity(0.5))
                            .child(kbd(look, "\u{2318}\u{21b5}"))
                            .on_click(cx.listener(|this, _, window, cx| this.teach_read(window, cx))),
                        Role::Button,
                        "Teach",
                    ))
                    .into_any_element(),
            ),
            Stage::Reading => body.push(
                labelled(
                    div()
                        .id("teach-reading")
                        .flex()
                        .tooltip(tip("Reading what you wrote"))
                        .child(glyph(look, "loader-circle", Size::Small, look.color(token::ACCENT)).with_animation(
                            "teach-reading-pulse",
                            Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.4, 1.)),
                            |mark, delta| mark.opacity(delta),
                        )),
                    Role::Status,
                    "Reading what you wrote",
                )
                .into_any_element(),
            ),
            Stage::Shown => {
                if let Some(proposal) = &open.proposal {
                    for one in reading_of(proposal, &open.repository) {
                        body.push(self.teach_section(&one, look));
                    }
                }
                body.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(
                            button(look, "teach-change", "Change", Tone::Secondary, false, None, true).track_focus(&open.change).on_click(cx.listener(|this, _, window, cx| this.teach_change(window, cx))),
                            Role::Button,
                            "Change",
                        ))
                        .child(labelled(
                            button(look, "teach-save", "Save", Tone::Primary, false, None, !open.saving)
                                .track_focus(&open.forward)
                                .child(kbd(look, "\u{2318}\u{21b5}"))
                                .on_click(cx.listener(|this, _, window, cx| this.teach_save(window, cx))),
                            Role::Button,
                            "Save",
                        ))
                        .into_any_element(),
                );
            }
        }
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(look.length(token::SPACE_1))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Teach"))
                    .when(!on_bridge, |titles| {
                        titles.child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).truncate().child(SharedString::from(open.repository.clone())))
                    }),
            )
            .child(labelled(
                div()
                    .id("teach-close")
                    .track_focus(&open.close)
                    .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL))
                    .rounded_full()
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip("Close (Esc)"))
                    .child(glyph(look, "x", Size::Large, look.color(token::FG_MUTED)))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            this.teach_close(window, cx);
                        }
                    }))
                    .on_click(cx.listener(|this, _, window, cx| this.teach_close(window, cx))),
                Role::Button,
                "Close",
            ));
        let panel = labelled(
            div()
                .id("teach-panel")
                .track_focus(&open.focus)
                .key_context("teach")
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.teach_keys(event, window, cx)))
                .w(look.length(token::W_DIALOG_WIDE))
                .max_w_full()
                .h_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .occlude()
                .child(head)
                .child(
                    div()
                        .id("teach-body")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p(look.length(token::SPACE_4))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_4))
                        .children(body),
                ),
            Role::Dialog,
            "Teach",
        );
        // Held off the room's edges on its trailing side; the room around it still takes presses.
        Some(div().absolute().inset_0().p(margin).flex().justify_end().child(panel).into_any_element())
    }

    /// One section of what Fleet understood.
    fn teach_section(&self, one: &Section, look: &Look) -> AnyElement {
        let caption = div()
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_SUBTLE))
            .child(one.caption);
        let line = |said: &str, muted: bool| {
            div()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(if muted { token::FG_MUTED } else { token::FG_DEFAULT }))
                .min_w_0()
                .child(SharedString::from(said.to_string()))
        };
        let facts = one.facts.iter().map(|fact| {
            let value = {
                let said = line(&fact.value, false);
                let said = if fact.mono_value { said.font_family(mono(look)) } else { said };
                let swatch = fact.swatch.as_deref().and_then(|text| gpui::Rgba::try_from(text).ok()).map(|colour| {
                    div().flex_none().size(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_SM)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).bg(gpui::Hsla::from(colour))
                });
                div().flex().items_center().gap(look.length(token::SPACE_2)).min_w_0().children(swatch).child(said)
            };
            match &fact.name {
                Some(name) => {
                    let name = line(name, true);
                    div().flex().gap(look.length(token::SPACE_4)).child(div().flex_none().w(look.length(token::SPACE_12) * 2.).child(if fact.mono_name { name.font_family(mono(look)) } else { name })).child(value)
                }
                None => div().flex().child(value),
            }
        });
        let also = (!one.also.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("Also drawn in"))
                .children(one.also.iter().map(|place| line(place, false)))
        });
        labelled(
            div()
                .id(SharedString::from(format!("teach-section:{}", one.caption)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .child(caption)
                .children(facts)
                .children(one.line.as_deref().map(|said| line(said, true)))
                .children(also),
            Role::Group,
            one.caption,
        )
        .into_any_element()
    }

    /// The reach chip: this repository, or every one. Only while the box is up.
    fn teach_reach(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = self.teach.as_mut().filter(|open| open.stage == Stage::Writing) {
            open.everywhere = !open.everywhere;
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet(does: ipc::Does, when: Vec<ipc::Instead>) -> ProposedFleet {
        ProposedFleet {
            rule: ipc::FleetRule { notice: "behind_base".into(), does, scope: ipc::RuleScope::Everywhere, matches: Some("main".into()), when },
            untestable: vec!["it is late".into()],
        }
    }

    #[test]
    fn a_rule_reads_as_what_it_does_then_each_condition_in_order() {
        let fleet = fleet(ipc::Does::Batch, vec![ipc::Instead { condition: ipc::Condition::BlocksMerge, does: ipc::Does::Tell }]);
        assert_eq!(does_words(&fleet), "Batch, and tell only when it blocks a merge");
    }

    #[test]
    fn the_reading_has_a_section_for_each_part_fleet_understood() {
        let proposal = TeachingProposal {
            note: Some(ipc::TeachingNote { section: ipc::LogbookSection::Repo, kind: ipc::LogbookKind::Gotcha, text: "Run it twice".into() }),
            fleet: Some(fleet(ipc::Does::Quiet, Vec::new())),
            ..TeachingProposal::default()
        };
        let sections = reading_of(&proposal, "armada");
        let captions: Vec<&str> = sections.iter().map(|one| one.caption).collect();
        assert_eq!(captions, ["Agents", "Fleet"]);
        assert_eq!(sections[1].line.as_deref(), Some("Fleet cannot test: it is late"));
        assert_eq!(sections[1].facts[0].value, "Slot behind notice");
        assert_eq!(sections[1].facts.last().map(|fact| fact.value.as_str()), Some("Every repository"));
    }

    #[test]
    fn a_source_with_no_text_is_named_by_its_kind_and_a_long_one_is_cut() {
        assert_eq!(source_of(TeachingSourceKind::TitleBar, "  ", Vec::new()).text, "Title bar");
        assert_eq!(source_of(TeachingSourceKind::Words, &"a".repeat(300), Vec::new()).text.len(), MOST_TEXT);
    }
}
