//! The side chat: questions put to a Job on a read-only copy of its conversation, which the Job never sees
//! (`ipc::Aside`). It is a view tab of the Job's panes, opened as a split beside the thread by the button under
//! the box, by ⌘J or by Ask on selected words. ⌘W puts the tab back among the thread's own and shuts the split;
//! the button or ⌘J brings it out again with every question read back through `list_asides`.
//!
//! **An aside is replaced by its id, never patched**: `session.aside` carries it whole, and the answer that
//! streams in is the one Fleet last said. Telling the Job is Fleet's act (`tell_aside`), which writes the
//! message as his own; this view only asks for it, and draws `told_at` as a chip.

use std::rc::Rc;

use gpui::{div, prelude::*, AnimationExt as _, px, AnyElement, ClipboardItem, Context, Entity, FocusHandle, Focusable, Role, ScrollHandle, SharedString, Subscription, Task, WeakEntity, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use gpui_component::text::TextView;
use ipc::{Aside, AsideList, AsideState, AskAside, TellAside};

use super::rows::{glyph, mono, tip, Size};
use super::view_tabs::{TabAct, ViewPane, ViewTab};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// The words under the tab: what the copy is, and what the Job does not do with it.
const BAND: &str = "Read-only copy. The Job doesn't see this.";
const PLACEHOLDER: &str = "Ask about this Job";
const QUOTED_PLACEHOLDER: &str = "What does this mean?";
const TELL_TIP: &str = "Tell the Job: sends this question and answer to the Job as your message.";
const TELL_WAITING: &str = "Tell the Job: there is no answer to send yet.";
const TOLD_TIP: &str = "Told: the Job has this question and answer as your message.";
const BUTTON_TIP: &str = "Side chat: ask about this Job without it seeing.";

/// The key that opens the side chat as the registry spells the Operator's, which a Job on the deck gives up to it.
pub(super) fn open_key() -> Option<gpui::Keystroke> {
    let row = crate::generated::actions::act("operator")?;
    let chord = crate::keymap::parse_chord(row.shortcut)?;
    let modifiers = gpui::Modifiers { control: chord.ctrl, alt: chord.alt, shift: chord.shift, platform: chord.meta, function: false };
    Some(gpui::Keystroke { modifiers, key: chord.key, key_char: None })
}

/// The tab's id among the Job's view tabs.
pub(super) fn tab_id(job: &str) -> String {
    format!("aside:{job}")
}

/// The key the box's draft is kept under, for the Job.
fn keep_key(job: &str) -> String {
    format!("aside:{job}")
}

/// Fleet's aside in place of the one held under its id, else after the others.
pub(super) fn replaced(held: &mut Vec<Aside>, moved: Aside) {
    match held.iter_mut().find(|one| one.aside_id == moved.aside_id) {
        Some(one) => *one = moved,
        None => held.push(moved),
    }
}

/// The question as it goes to Fleet: his words, trimmed, with the words he selected where he did.
pub(super) fn asked(job: &str, question: &str, quote: Option<&str>) -> Option<AskAside> {
    let question = question.trim();
    (!question.is_empty()).then(|| AskAside { session_id: ipc::SessionId::carried(job.to_string()), question: question.to_string(), quote: quote.map(str::trim).filter(|quote| !quote.is_empty()).map(str::to_string) })
}

/// Whether Tell the Job can be pressed: only an answered aside that was not told yet.
pub(super) fn tellable(aside: &Aside) -> bool {
    aside.state == AsideState::Answered && aside.told_at.is_none()
}

/// What the copy has looked at, one row each, in the order Fleet sent them.
pub(super) fn reads_of(aside: &Aside) -> &[String] {
    &aside.reads
}

/// Where the Job's asides stand in the view.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Read {
    Reading,
    Read,
    /// Fleet's sentence for why the list could not be read.
    Refused(String),
}

pub(super) struct SideChat {
    job: SharedString,
    view: WeakEntity<JobView>,
    /// No Fleet: the mock answers as a Fleet that took the question would.
    mock: bool,
    asides: Vec<Aside>,
    read: Read,
    /// The words he selected in the thread, quoted over the box until the question goes.
    quote: Option<String>,
    input: Entity<TextareaState>,
    /// Fleet's sentence for the last ask or tell it refused.
    refused: Option<String>,
    /// The question is on its way to Fleet.
    posting: bool,
    /// Asides whose Tell is on its way.
    telling: Vec<String>,
    /// What the mock was asked, oldest first.
    mock_asked: Vec<AskAside>,
    scroll: ScrollHandle,
    /// The log follows its tail until he scrolls up.
    follow: bool,
    tasks: Vec<Task<()>>,
    _subs: Vec<Subscription>,
}

impl SideChat {
    pub(super) fn new(job: SharedString, view: WeakEntity<JobView>, mock: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 6).submit_on_enter(true).placeholder(PLACEHOLDER));
        let key = keep_key(&job);
        if let Some(words) = crate::drafts::kept(&key, cx) {
            input.update(cx, |input, cx| input.set_value(words, window, cx));
        }
        let subs = vec![cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                crate::drafts::keep(&key, input.read(cx).value().as_ref(), cx);
                this.refused = None;
                cx.notify();
            }
            InputEvent::PressEnter { shift: false, .. } => this.ask(window, cx),
            _ => {}
        })];
        SideChat {
            job,
            view,
            mock,
            asides: Vec::new(),
            read: Read::Reading,
            quote: None,
            input,
            refused: None,
            posting: false,
            telling: Vec::new(),
            mock_asked: Vec::new(),
            scroll: ScrollHandle::new(),
            follow: true,
            tasks: Vec::new(),
            _subs: subs,
        }
    }

    pub(super) fn asides(&self) -> &[Aside] {
        &self.asides
    }

    pub(super) fn quote(&self) -> Option<&str> {
        self.quote.as_deref()
    }

    pub(super) fn asked(&self) -> &[AskAside] {
        &self.mock_asked
    }

    pub(super) fn box_focus(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }

    /// Put the words he selected over the box, and the keys in it.
    pub(super) fn quoting(&mut self, words: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let quoted = words.is_some();
        if words.is_some() || self.quote.is_some() {
            self.quote = words.map(|words| words.trim().to_string()).filter(|words| !words.is_empty());
        }
        let placeholder = if self.quote.is_some() { QUOTED_PLACEHOLDER } else { PLACEHOLDER };
        self.input.update(cx, |input, cx| input.set_placeholder(placeholder, window, cx));
        if quoted {
            self.refused = None;
        }
        cx.notify();
    }

    /// Fleet's aside, whole, in place of the one held.
    pub(super) fn moved(&mut self, moved: Aside, cx: &mut Context<Self>) {
        if moved.session_id.as_str() != self.job.as_ref() {
            return;
        }
        self.follow = self.at_tail();
        replaced(&mut self.asides, moved);
        cx.notify();
    }

    fn at_tail(&self) -> bool {
        let room = self.scroll.max_offset().y;
        room <= px(1.) || self.scroll.offset().y <= -room + px(8.)
    }

    fn port(&self, cx: &gpui::App) -> Option<u16> {
        self.view.upgrade().and_then(|view| view.read(cx).port(cx))
    }

    /// Read the Job's asides again, so what was asked before comes back, whole.
    pub(super) fn reread(&mut self, cx: &mut Context<Self>) {
        if self.mock {
            if self.read != Read::Read {
                self.asides = crate::mock::asides(&self.job);
                self.read = Read::Read;
                cx.notify();
            }
            return;
        }
        let Some(port) = self.port(cx) else {
            self.read = Read::Refused("Fleet is not connected".into());
            cx.notify();
            return;
        };
        let path = format!("/sessions/asides?session_id={}", super::wire::encoded(&self.job));
        let task = cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<AsideList>("AsideList", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(list) => {
                        this.follow = this.at_tail();
                        // What the events brought while the list was read stays: each is replaced by its id.
                        let newer = std::mem::take(&mut this.asides);
                        this.asides = list.asides;
                        for one in newer.into_iter().filter(|one| one.state == AsideState::Asking) {
                            replaced(&mut this.asides, one);
                        }
                        this.read = Read::Read;
                    }
                    Err(why) => this.read = Read::Refused(why),
                }
                cx.notify();
            });
        });
        self.tasks.push(task);
    }

    /// Enter: the words go to Fleet as a new aside, with the quote where there is one.
    pub(super) fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.posting {
            return;
        }
        let words = self.input.read(cx).value().to_string();
        let Some(ask) = asked(&self.job, &words, self.quote.as_deref()) else { return };
        self.refused = None;
        if self.mock {
            self.follow = true;
            let one = crate::mock::asked(&ask, self.asides.len());
            self.mock_asked.push(ask);
            replaced(&mut self.asides, one);
            self.sent(window, cx);
            return;
        }
        let Some(port) = self.port(cx) else {
            self.refused = Some("Fleet is not connected".into());
            cx.notify();
            return;
        };
        let body = match ipc::encode(&ask) {
            Ok(body) => body,
            Err(why) => {
                self.refused = Some(format!("{why:?}"));
                cx.notify();
                return;
            }
        };
        self.posting = true;
        cx.notify();
        let task = cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/aside", body))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<Aside>("Aside", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.posting = false;
                match answer {
                    Ok(one) => {
                        this.follow = true;
                        replaced(&mut this.asides, one);
                        this.sent(window, cx);
                    }
                    Err(why) => this.refused = Some(why),
                }
                cx.notify();
            });
        });
        self.tasks.push(task);
    }

    /// The question went: the box is empty, the quote is spent and the draft is let go.
    fn sent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.set_value("", window, cx));
        crate::drafts::clear(&keep_key(&self.job), cx);
        self.quoting(None, window, cx);
    }

    /// Tell the Job: Fleet sends the question and answer as his message and answers with `told_at` set.
    pub(super) fn tell(&mut self, aside_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(aside) = self.asides.iter().find(|one| one.aside_id == aside_id).filter(|one| tellable(one)).cloned() else { return };
        if self.telling.iter().any(|one| one == aside_id) {
            return;
        }
        self.refused = None;
        if self.mock {
            let told = crate::mock::told(&aside);
            let words = crate::mock::told_words(&aside);
            replaced(&mut self.asides, told);
            let job = self.job.clone();
            if let Some(view) = self.view.upgrade() {
                view.update(cx, |view, cx| view.mock_told(&job, words, cx));
            }
            cx.notify();
            return;
        }
        let Some(port) = self.port(cx) else {
            self.refused = Some("Fleet is not connected".into());
            cx.notify();
            return;
        };
        let body = match ipc::encode(&TellAside { session_id: ipc::SessionId::carried(self.job.to_string()), aside_id: aside_id.to_string() }) {
            Ok(body) => body,
            Err(why) => {
                self.refused = Some(format!("{why:?}"));
                cx.notify();
                return;
            }
        };
        self.telling.push(aside_id.to_string());
        cx.notify();
        let id = aside_id.to_string();
        let task = cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/aside/tell", body))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<Aside>("Aside", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.telling.retain(|one| *one != id);
                match answer {
                    Ok(one) => replaced(&mut this.asides, one),
                    Err(why) => this.refused = Some(why),
                }
                cx.notify();
            });
        });
        self.tasks.push(task);
    }

    fn copy(&mut self, aside_id: &str, cx: &mut Context<Self>) {
        if let Some(one) = self.asides.iter().find(|one| one.aside_id == aside_id).filter(|one| !one.answer.is_empty()) {
            cx.write_to_clipboard(ClipboardItem::new_string(one.answer.clone()));
        }
    }

    fn drop_quote(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.quote = None;
        self.quoting(None, window, cx);
        let focus = self.box_focus(cx);
        window.focus(&focus, cx);
    }
}

impl Focusable for SideChat {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.box_focus(cx)
    }
}

/// His question as his bubble, at the right, in the accent's wash.
fn bubble(look: &Look, id: SharedString, words: &str) -> AnyElement {
    div()
        .w_full()
        .flex()
        .justify_end()
        .child(labelled(
            div()
                .id(id)
                .max_w(gpui::relative(0.82))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .bg(look.color(token::ACCENT_MUTED))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(words.to_string())),
            Role::Article,
            "Your question",
        ))
        .into_any_element()
}

/// The words he selected, quoted by an edge in the tool's hue.
fn quoted(look: &Look, id: SharedString, words: &str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(look.length(token::SPACE_2))
        .border_l(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::TOOL_LOOK))
        .text_size(look.length(token::TEXT_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(words.to_string()))
}

impl SideChat {
    /// One row of what the copy looked at: muted, in the looking tools' hue, a search mark before it.
    fn read_row(&self, look: &Look, id: &str, ix: usize, words: &str) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("aside-read:{id}:{ix}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::TOOL_LOOK))
                .child(glyph(look, "search", Size::Small, look.color(token::TOOL_LOOK)))
                .child(div().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(words.to_string()))),
            Role::ListItem,
            words.to_string(),
        )
        .into_any_element()
    }

    /// The copy's answer, as markdown, where it has said something.
    fn answer_card(&self, look: &Look, one: &Aside) -> Option<AnyElement> {
        let failed = one.state == AsideState::Failed;
        if one.answer.trim().is_empty() && !failed {
            return None;
        }
        let ink = if failed { token::STATUS_COMPLETED_FAILED } else { token::FG_DEFAULT };
        let words = if failed { one.failed.clone().unwrap_or_default() } else { one.answer.clone() };
        let card = div()
            .id(SharedString::from(format!("aside-answer:{}", one.aside_id)))
            .w_full()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if failed { token::STATUS_COMPLETED_FAILED } else { token::BORDER_SUBTLE }))
            .bg(look.color(if failed { token::STATUS_COMPLETED_FAILED_BG } else { token::BG_RAISED }))
            .text_color(look.color(ink))
            .when(failed, |card| card.flex().items_start().gap(look.length(token::SPACE_2)).child(labelled(div().id(SharedString::from(format!("aside-failed:{}", one.aside_id))).flex_none().mt(px(2.)).tooltip(tip("Failed")).child(glyph(look, "x", Size::Small, look.color(token::STATUS_COMPLETED_FAILED))), Role::Image, "Failed")))
            .child(
                div().min_w_0().flex_1().child(
                    TextView::markdown(gpui::ElementId::Name(format!("aside-md:{}:{}", one.aside_id, one.answer.len()).into()), words)
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .selectable(true),
                ),
            );
        Some(labelled(card, Role::Article, if failed { "Side chat failed" } else { "Side chat answer" }).into_any_element())
    }

    /// While the copy is still asking: the working mark, until it has said something.
    fn asking_mark(&self, look: &Look, one: &Aside) -> Option<AnyElement> {
        (one.state == AsideState::Asking).then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("aside-asking:{}", one.aside_id)))
                    .flex()
                    .items_center()
                    .child(
                        glyph(look, "loader-circle", Size::Small, look.color(token::TOOL_LOOK)).with_animation(
                            SharedString::from(format!("aside-turn:{}", one.aside_id)),
                            gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat(),
                            |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                        ),
                    )
                    .tooltip(tip("Asking")),
                Role::Image,
                "Asking",
            )
            .into_any_element()
        })
    }

    /// Tell the Job and Copy, and the Told chip once Fleet says it was told.
    fn acts(&self, look: &Look, one: &Aside, cx: &Context<Self>) -> AnyElement {
        let id = one.aside_id.clone();
        let tellable = tellable(one) && !self.telling.contains(&id);
        let told = one.told_at.is_some();
        let has_answer = !one.answer.is_empty();
        let button = |name: &'static str, primary: bool, live: bool| {
            div()
                .id(SharedString::from(format!("aside-{name}:{id}")))
                .flex_none()
                .px(look.length(token::SPACE_3))
                .h(look.length(token::H_CONTROL_SM))
                .flex()
                .items_center()
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .border(look.length(token::BORDER_WIDTH))
                .when(primary && live, |button| button.bg(look.color(token::TOOL_LOOK)).border_color(look.color(token::TOOL_LOOK)).text_color(look.color(token::TEXT_ON_ACCENT)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING))))
                .when(!primary && live, |button| button.bg(look.color(token::BG_RAISED)).border_color(look.color(token::BORDER_DEFAULT)).text_color(look.color(token::FG_DEFAULT)))
                .when(!live, |button| button.border_color(look.color(token::BORDER_SUBTLE)).text_color(look.color(token::FG_SUBTLE)).cursor_default())
                .when(live, |button| button.cursor_pointer().hover(|style| style.opacity(0.85)))
                .child(name)
        };
        let tell_tip = if told { TOLD_TIP } else if tellable { TELL_TIP } else { TELL_WAITING };
        let tell = labelled(
            button("Tell the Job", true, tellable).tooltip(tip(tell_tip)).when(tellable, |button| {
                let id = one.aside_id.clone();
                button.on_click(cx.listener(move |this, _, window, cx| this.tell(&id, window, cx)))
            }),
            Role::Button,
            "Tell the Job",
        );
        let copy = labelled(
            button("Copy", false, has_answer).tooltip(tip("Copy the answer")).when(has_answer, |button| {
                let id = one.aside_id.clone();
                button.on_click(cx.listener(move |this, _, _, cx| this.copy(&id, cx)))
            }),
            Role::Button,
            "Copy",
        );
        div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(tell)
            .child(copy)
            .children(told.then(|| {
                labelled(
                    div()
                        .id(SharedString::from(format!("aside-told:{}", one.aside_id)))
                        .flex_none()
                        .px(look.length(token::SPACE_2))
                        .rounded_full()
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                        .bg(look.color(token::STATUS_COMPLETED_SUCCESS_BG))
                        .text_color(look.color(token::STATUS_COMPLETED_SUCCESS))
                        .tooltip(tip(TOLD_TIP))
                        .child("Told"),
                    Role::Status,
                    "Told",
                )
            }))
            .into_any_element()
    }

    /// One question and what came of it. Earlier ones are dimmed so the newest reads at full strength.
    fn exchange(&self, look: &Look, one: &Aside, newest: bool, cx: &Context<Self>) -> AnyElement {
        let id = one.aside_id.clone();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .when(!newest, |column| column.opacity(0.55))
            .children(one.quote.as_deref().map(|words| quoted(look, SharedString::from(format!("aside-quote:{id}")), words).into_any_element()))
            .child(bubble(look, SharedString::from(format!("aside-question:{id}")), &one.question))
            .children(reads_of(one).iter().enumerate().map(|(ix, words)| self.read_row(look, &id, ix, words)))
            .children(self.answer_card(look, one))
            .children(self.asking_mark(look, one))
            .children((one.state != AsideState::Failed).then(|| self.acts(look, one, cx)))
            .into_any_element()
    }

    fn band(&self, look: &Look) -> AnyElement {
        labelled(
            div()
                .id("aside-band")
                .flex_none()
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::TOOL_LOOK))
                .bg(look.color(token::TOOL_LOOK).opacity(0.10))
                .child(glyph(look, "lock", Size::Small, look.color(token::TOOL_LOOK)))
                .child(div().min_w_0().flex_1().child(BAND)),
            Role::Group,
            BAND,
        )
        .into_any_element()
    }

    /// The box: a ring in the tool's hue, the quote over it where he selected words, and Enter's key beside it.
    fn composer(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let empty = self.input.read(cx).value().is_empty();
        let asking = if self.quote.is_some() { QUOTED_PLACEHOLDER } else { PLACEHOLDER };
        // The Return key gives way where the pane is too narrow to show the placeholder whole beside it; a pane not yet measured has room.
        let width = self.scroll.bounds().size.width;
        let roomy = width == px(0.) || width >= look.length(token::SPACE_12) * 6.;
        div()
            .flex_none()
            .px(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_2))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .children(self.refused.as_ref().map(|why| {
                labelled(
                    div().id("aside-refused").px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(why.clone())),
                    Role::Alert,
                    why.clone(),
                )
            }))
            .child(labelled(
                div()
                    .id("aside-frame")
                    .flex()
                    .flex_col()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH) * 1.5)
                    .border_color(look.color(token::TOOL_LOOK))
                    .bg(look.color(token::BG_RAISED))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .children(self.quote.as_deref().map(|words| {
                        div()
                            .flex()
                            .items_start()
                            .gap(look.length(token::SPACE_2))
                            .pb(look.length(token::SPACE_1))
                            .child({
                                // One line, cut with an ellipsis; the whole of it in the tooltip.
                                let whole = SharedString::from(words.to_string());
                                let line = words.split_whitespace().collect::<Vec<_>>().join(" ");
                                quoted(look, "aside-quoting".into(), &line).flex_1().min_w_0().truncate().tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(whole.clone()).build(window, cx))
                            })
                            .child(labelled(
                                super::head::icon_button(look, "aside-quote-drop", "x", "Remove the quote".into(), false, false)
                                    .size(look.length(token::H_CONTROL_SM))
                                    .on_click(cx.listener(|this, _, window, cx| this.drop_quote(window, cx))),
                                Role::Button,
                                "Remove the quote",
                            ))
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(
                                div()
                                    .id("aside-box")
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(look.length(token::TEXT_SM))
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(Textarea::new(&self.input).appearance(false).bordered(false).aria_label(asking)),
                            )
                            .children((empty && roomy).then(|| crate::titlebar::parts::kbd("\u{21b5}", look))),
                    ),
                Role::TextInput,
                asking,
            ))
            .into_any_element()
    }
}

impl gpui::Render for SideChat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let look = &look;
        if self.follow {
            let room = self.scroll.max_offset().y;
            self.scroll.set_offset(gpui::point(px(0.), -room));
            // The room is measured while the rows are laid out: once more after this frame settles it.
            let view = cx.entity();
            window.on_next_frame(move |_, cx| {
                view.update(cx, |this, cx| {
                    let room = this.scroll.max_offset().y;
                    if this.scroll.offset().y != -room {
                        this.scroll.set_offset(gpui::point(px(0.), -room));
                        cx.notify();
                    }
                })
            });
        }
        let last = self.asides.len().saturating_sub(1);
        let log = div()
            .id("aside-log")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll(cx.listener(|this, _, _, cx| {
                this.follow = this.at_tail();
                cx.notify();
            }))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .children(match &self.read {
                Read::Refused(why) => Some(labelled(div().id("aside-unread").text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(why.clone())), Role::Alert, why.clone()).into_any_element()),
                _ => None,
            })
            .children(self.asides.iter().enumerate().map(|(ix, one)| self.exchange(look, one, ix == last, cx)));
        labelled(
            div()
                .id("side-chat")
                .size_full()
                .flex()
                .flex_col()
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(token::FG_DEFAULT))
                .child(self.band(look))
                .child(log)
                .child(self.composer(look, cx)),
            Role::Region,
            "Side chat",
        )
    }
}

impl JobView {
    /// The side chat of Job `job`: one entity for the Job, kept for as long as the app runs, so what was asked stays.
    fn side_chat_of(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) -> Entity<SideChat> {
        if let Some(held) = self.side_chats.get(job.as_ref()) {
            return held.clone();
        }
        let (view, mock) = (cx.weak_entity(), self.fleet.is_none());
        let chat = cx.new(|cx| SideChat::new(job.clone(), view, mock, window, cx));
        self.side_chats.insert(job.to_string(), chat.clone());
        chat
    }

    /// The tab the side chat shows as, in the Job's panes.
    fn side_tab(&self, job: &str, chat: &Entity<SideChat>) -> ViewTab {
        let close = {
            let job = job.to_string();
            Rc::new(move |window: &mut Window, cx: &mut gpui::App| {
                let Some(view) = JobView::live(cx) else { return };
                view.update(cx, |view, cx| {
                    view.shut_side_split_of(&job, window, cx);
                });
            })
        };
        ViewTab {
            id: tab_id(job),
            title: "Side chat".into(),
            glyph: "eye",
            view: chat.clone().into(),
            menu: vec![TabAct { label: "Close split".into(), danger: false, act: close }],
            closing: None,
            closed: None,
            renamed: None,
        }
    }

    /// Open the side chat as a split of the thread's pane, the thread on the left and the tab on the right, the keys in its box. `quote` is the words he selected.
    pub(super) fn open_side_chat(&mut self, quote: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let chat = self.side_chat_of(&job, window, cx);
        if self.side_split_open() {
            self.front_view_tab(&job, &tab_id(&job), cx);
        } else {
            let tab = self.side_tab(&job, &chat);
            // In the thread's own strip first, behind the thread; then the split takes it, at the pane's full height.
            self.open_view_tab_in(&job, ViewPane::Thread, tab, false, cx);
            let terminal = super::panes_place::terminal_key(&tab_id(&job)).to_string();
            // A pane too narrow to hold two halves refuses the split: the tab then stands in front of the thread, in its strip.
            let split = self.rearrange_in(&job, None, false, move |arrangement| {
                let tab = super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal.as_str())).map(|tab| tab.id.clone());
                let Some((tab, thread)) = tab.zip(super::pane_tree::holder_of(arrangement, "thread").map(|pane| pane.id.clone())) else { return std::borrow::Cow::Borrowed(arrangement) };
                let split = super::pane_tree::split_pane(arrangement, &thread, super::pane_tree::SplitInto::Right).into_owned();
                let beside = split.focus.clone();
                std::borrow::Cow::Owned(super::pane_tree::move_tab_to_pane(&split, &tab, &beside, None).into_owned())
            }, cx);
            if !split {
                self.front_view_tab(&job, &tab_id(&job), cx);
            }
        }
        chat.update(cx, |chat, cx| {
            chat.reread(cx);
            chat.quoting(quote, window, cx);
        });
        let focus = chat.read(cx).box_focus(cx);
        window.focus(&focus, cx);
        window.on_next_frame(move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// A press on the Side chat tab among the thread's own opens the split again.
    pub(super) fn side_tab_pressed(&mut self, tab: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let pressed = self.arrangement_of(&job).and_then(|arrangement| super::pane_tree::tab_by_id(arrangement, tab)).and_then(|tab| self.view_id_of(&job, tab));
        if pressed.as_deref() != Some(tab_id(&job).as_str()) || self.side_split_open() {
            return false;
        }
        self.open_side_chat(None, window, cx);
        true
    }

    /// Whether the side chat stands in a split beside the thread, and not among the thread's own tabs.
    pub(super) fn side_split_open(&self) -> bool {
        let Some(job) = self.shown.as_ref() else { return false };
        self.view_tab_pane(job, &tab_id(job)).is_some_and(|pane| pane == ViewPane::Beside)
    }

    /// Whether the side chat is the front tab of the thread's pane, as it is where the pane was too narrow to split.
    pub(super) fn side_chat_in_front(&self) -> bool {
        let Some(job) = self.shown.as_ref() else { return false };
        let Some(arrangement) = self.arrangement_of(job) else { return false };
        let id = tab_id(job);
        let terminal = super::panes_place::terminal_key(&id);
        super::pane_tree::panes_of(&arrangement.root).into_iter().any(|pane| {
            pane.active.as_deref().is_some_and(|front| pane.tabs.iter().any(|tab| tab.id == front && tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal))) && super::panes_place::view_pane_of(arrangement, &pane.id) == ViewPane::Thread
        })
    }

    /// ⌘W: the split shuts and the tab stays among the thread's own, behind them. True where there was a split.
    pub(super) fn shut_side_split(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        self.shut_side_split_of(&job, window, cx)
    }

    fn shut_side_split_of(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let id = tab_id(job);
        if self.view_tab_pane(job, &id).is_none_or(|pane| pane == ViewPane::Thread) {
            // In front of the thread in its own strip: the thread comes back in front.
            if !self.side_chat_in_front() {
                return false;
            }
            self.front_thread();
            self.to_composer(window, cx);
            cx.notify();
            return true;
        }
        let Some(chat) = self.side_chats.get(job).cloned() else { return false };
        let tab = self.side_tab(job, &chat);
        self.open_view_tab_in(job, ViewPane::Thread, tab, false, cx);
        self.to_composer(window, cx);
        cx.notify();
        true
    }

    /// ⌘J, which the registry gives the Operator everywhere: with a Job open on the deck it opens the Job's side chat.
    pub fn side_chat_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !act.is("operator") || self.shown.is_none() {
            return false;
        }
        // Words selected in the thread are what the box quotes.
        let quote = self.take_picked_words(window, cx);
        self.open_side_chat(quote, window, cx);
        true
    }

    /// ⌘W while the split is open.
    pub(super) fn side_chat_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keys = &event.keystroke;
        let alone = keys.modifiers.platform && !keys.modifiers.shift && !keys.modifiers.alt && !keys.modifiers.control;
        alone && keys.key == "w" && (self.side_split_open() || self.side_chat_in_front()) && self.shut_side_split(window, cx)
    }

    /// `session.aside` news: each aside replaces the one its Job's side chat holds.
    pub(super) fn asides_moved(&mut self, moved: &[Aside], cx: &mut Context<Self>) {
        for one in moved {
            if let Some(chat) = self.side_chats.get(one.session_id.as_str()).cloned() {
                chat.update(cx, |chat, cx| chat.moved(one.clone(), cx));
            }
        }
    }

    /// Mock data: the Job's thread holds the message Fleet writes for a told aside, as his own.
    pub(super) fn mock_told(&mut self, job: &str, words: String, cx: &mut Context<Self>) {
        let id = SharedString::from(job.to_string());
        self.composer.next_key += 1;
        let key = self.composer.next_key;
        self.mock_sent(&id, key, words, Vec::new(), Vec::new(), Vec::new());
        self.sync(&id, cx);
        cx.notify();
    }

    /// The button under the box: the eye and its name, in the tool's hue, lit while the split is open.
    pub(super) fn side_chat_button(&self, look: &Look, narrow: bool, cx: &Context<Self>) -> AnyElement {
        let lit = self.side_split_open() || self.side_chat_in_front();
        let ink = look.color(token::TOOL_LOOK);
        let key = open_key();
        labelled(
            div()
                .id("side-chat-button")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(if narrow { token::SPACE_2 } else { token::SPACE_3 }))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(ink.opacity(0.55))
                .text_size(look.length(token::TEXT_SM))
                .text_color(ink)
                .when(lit, |button| button.bg(ink.opacity(0.22)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING))))
                .cursor_pointer()
                .hover(|style| style.bg(ink.opacity(0.14)))
                .child(glyph(look, "eye", Size::Small, ink))
                .children((!narrow).then(|| div().child("Side chat")))
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(BUTTON_TIP).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .on_click(cx.listener(|this, _, window, cx| this.open_side_chat(None, window, cx))),
            Role::Button,
            "Side chat",
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Role;

    fn aside(id: &str, state: AsideState) -> Aside {
        ipc::decode::<Aside>(
            "Aside",
            format!(r#"{{"aside_id":"{id}","session_id":"mock-01","question":"why","answer":"because","state":"{}","asked_at":"2026-11-16T10:00:00Z","reads":[]}}"#, match state {
                AsideState::Asking => "asking",
                AsideState::Answered => "answered",
                AsideState::Failed => "failed",
            })
            .as_bytes(),
        )
        .expect("an aside")
    }

    #[test]
    fn an_aside_the_event_carries_replaces_the_one_held_by_its_id_and_a_new_one_goes_last() {
        let mut held = vec![aside("a1", AsideState::Asking), aside("a2", AsideState::Answered)];
        let mut grown = aside("a1", AsideState::Answered);
        grown.answer = "because, and more".into();
        replaced(&mut held, grown);
        assert_eq!(held.len(), 2);
        assert_eq!((held[0].aside_id.as_str(), held[0].state, held[0].answer.as_str()), ("a1", AsideState::Answered, "because, and more"), "replaced where it stood");
        replaced(&mut held, aside("a3", AsideState::Asking));
        assert_eq!(held.iter().map(|one| one.aside_id.as_str()).collect::<Vec<_>>(), ["a1", "a2", "a3"]);
    }

    #[test]
    fn tell_the_job_waits_for_an_answer_and_ends_once_told() {
        assert!(!tellable(&aside("a", AsideState::Asking)));
        assert!(!tellable(&aside("a", AsideState::Failed)));
        let mut answered = aside("a", AsideState::Answered);
        assert!(tellable(&answered));
        answered.told_at = Some(ipc::Instant::carried("2026-11-16T10:05:00Z"));
        assert!(!tellable(&answered), "Fleet refuses a second tell, so it is not offered");
    }

    #[test]
    fn the_question_goes_to_fleet_trimmed_with_the_quote_where_there_is_one_and_never_blank() {
        let plain = asked("mock-01", "  what is this?  ", None).expect("a question");
        assert_eq!((plain.question.as_str(), plain.quote.as_deref(), plain.session_id.as_str()), ("what is this?", None, "mock-01"));
        let quoted = asked("mock-01", "what does this mean?", Some(" write-clash probe ")).expect("a question");
        assert_eq!(quoted.quote.as_deref(), Some("write-clash probe"));
        assert!(asked("mock-01", "   ", Some("words")).is_none());
        assert_eq!(asked("mock-01", "q", Some("  ")).expect("a question").quote, None, "a blank quote is none");
    }

    fn job<'a>(cx: &'a mut gpui::TestAppContext, open: &str) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("side-chat").expect("the fixture decodes");
        let threads = crate::mock::threads("side-chat").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open(open.to_string().into(), cx));
        cx.run_until_parked();
        (view, cx)
    }

    fn operator() -> crate::keymap::Act {
        crate::keymap::Act { id: "operator", slot: 0, digit: None }
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| super::super::head::drawn(window, cx, role, name)).is_some()
    }

    #[gpui::test]
    fn what_the_copy_read_stands_one_muted_row_each_in_the_order_fleet_sent_them(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        view.update_in(cx, |view, window, cx| view.open_side_chat(None, window, cx));
        cx.run_until_parked();
        let chat = view.read_with(cx, |view, _| view.side_chats.get("mock-02").cloned()).expect("the Job's side chat");
        let reads: Vec<Vec<String>> = chat.read_with(cx, |chat, _| chat.asides().iter().map(|one| reads_of(one).to_vec()).collect());
        assert_eq!(reads[0], ["Read crates/store/migrations"]);
        assert_eq!(reads[3], ["Grep retry in crates/api/src", "Read crates/api/src/webhook.rs"], "the asking one has read two things so far");
        for name in ["Read crates/store/migrations", "Grep retry in crates/api/src", "Read crates/api/src/webhook.rs"] {
            assert!(drawn(cx, Role::ListItem, name), "{name} is drawn as a row");
        }
    }

    #[gpui::test]
    fn tell_the_job_does_nothing_until_answered_and_a_told_one_shows_told(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        view.update_in(cx, |view, window, cx| view.open_side_chat(None, window, cx));
        cx.run_until_parked();
        let chat = view.read_with(cx, |view, _| view.side_chats.get("mock-02").cloned()).expect("the Job's side chat");
        let told = |cx: &mut gpui::VisualTestContext| chat.read_with(cx, |chat, _| chat.asides().iter().map(|one| one.told_at.is_some()).collect::<Vec<_>>());
        assert_eq!(told(cx), [true, false, false, false], "only the first was told");
        // The one still asking, and the one answered: Tell goes only for the answered.
        chat.update_in(cx, |chat, window, cx| chat.tell("mock-aside-4", window, cx));
        assert_eq!(told(cx), [true, false, false, false], "an asking aside is not told");
        chat.update_in(cx, |chat, window, cx| chat.tell("mock-aside-2", window, cx));
        assert_eq!(told(cx), [true, true, false, false]);
        // The Job's thread now holds the message Fleet writes, as his own.
        let said = view.read_with(cx, |view, cx| view.thread("mock-02", cx).map(|held| format!("{:?}", held.items.last())));
        assert!(said.is_some_and(|said| said.contains("Aside, answered on a read-only copy of this Job:")), "the told words are his message in the thread");
        assert!(drawn(cx, Role::Status, "Told"));
    }

    #[gpui::test]
    fn the_selected_words_reach_the_question_as_its_quote(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-01");
        view.update_in(cx, |view, window, cx| view.open_side_chat(Some("write-clash probe".into()), window, cx));
        cx.run_until_parked();
        let chat = view.read_with(cx, |view, _| view.side_chats.get("mock-01").cloned()).expect("the Job's side chat");
        assert_eq!(chat.read_with(cx, |chat, _| chat.quote().map(str::to_string)).as_deref(), Some("write-clash probe"));
        assert!(drawn(cx, Role::TextInput, "What does this mean?"), "the box asks what the words mean");
        chat.update_in(cx, |chat, window, cx| {
            chat.input.update(cx, |input, cx| input.set_value("what does this mean?", window, cx));
            chat.ask(window, cx);
        });
        let sent = chat.read_with(cx, |chat, _| chat.asked().to_vec());
        assert_eq!(sent.len(), 1);
        assert_eq!((sent[0].question.as_str(), sent[0].quote.as_deref(), sent[0].session_id.as_str()), ("what does this mean?", Some("write-clash probe"), "mock-01"));
        assert_eq!(chat.read_with(cx, |chat, _| chat.quote().map(str::to_string)), None, "the quote is spent with the question");
        assert!(drawn(cx, Role::TextInput, "Ask about this Job"));
    }

    #[gpui::test]
    fn w_shuts_the_split_and_j_brings_it_back_with_every_aside(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        let split = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, _| (view.side_split_open(), view.view_tab_ids("mock-02")));
        assert_eq!(split(&view, cx), (false, Vec::<String>::new()));
        let took = view.update_in(cx, |view, window, cx| view.side_chat_act(&operator(), window, cx));
        cx.run_until_parked();
        assert!(took, "⌘J with a Job open is the side chat's");
        assert_eq!(split(&view, cx), (true, vec!["aside:mock-02".to_string()]));
        let w = gpui::KeyDownEvent { keystroke: gpui::Keystroke::parse("cmd-w").expect("a key"), is_held: false, prefer_character_input: false };
        assert!(view.update_in(cx, |view, window, cx| view.side_chat_key(&w, window, cx)), "⌘W shuts the split");
        cx.run_until_parked();
        assert_eq!(split(&view, cx), (false, vec!["aside:mock-02".to_string()]), "the tab stays among the Job's own");
        assert_eq!(view.read_with(cx, |view, _| view.view_tab_pane("mock-02", "aside:mock-02")), Some(ViewPane::Thread));
        assert!(!view.update_in(cx, |view, window, cx| view.side_chat_key(&w, window, cx)), "with no split ⌘W is left alone");
        view.update_in(cx, |view, window, cx| view.side_chat_act(&operator(), window, cx));
        cx.run_until_parked();
        assert!(split(&view, cx).0, "⌘J opens it again");
        let chat = view.read_with(cx, |view, _| view.side_chats.get("mock-02").cloned()).expect("the Job's side chat");
        assert_eq!(chat.read_with(cx, |chat, _| chat.asides().len()), 4, "every question is kept");
    }

    /// The panes of Job `mock-02`, as (the thread's pane tabs, the side chat's pane tabs, how many panes).
    fn shape(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> (Vec<String>, Vec<String>, usize) {
        view.read_with(cx, |view, _| {
            let arrangement = view.arrangement_of("mock-02").expect("the Job's panes");
            let panes = super::super::pane_tree::panes_of(&arrangement.root);
            let held = |pane: &super::super::pane_tree::Pane| pane.tabs.iter().map(|tab| tab.kind.clone()).collect::<Vec<_>>();
            let thread = super::super::pane_tree::holder_of(arrangement, "thread").expect("the thread's pane");
            let side = panes.iter().find(|pane| pane.tabs.iter().any(|tab| tab.terminal.as_deref() == Some("aside:mock-02")));
            (held(thread), side.map(|pane| held(pane)).unwrap_or_default(), panes.len())
        })
    }

    #[gpui::test]
    fn the_side_chat_splits_the_thread_pane_in_two_and_its_tab_goes_back_to_the_thread_strip(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        let (_, _, before) = shape(&view, cx);
        view.update_in(cx, |view, window, cx| view.open_side_chat(None, window, cx));
        cx.run_until_parked();
        let (thread, side, panes) = shape(&view, cx);
        assert_eq!(panes, before + 1, "the thread's pane splits");
        assert_eq!(side, vec!["terminal".to_string()], "the Side chat tab stands alone in the pane the split makes");
        assert!(thread.contains(&"thread".to_string()) && !thread.contains(&"terminal".to_string()), "the thread keeps its own pane");
        view.update_in(cx, |view, window, cx| view.shut_side_split(window, cx));
        cx.run_until_parked();
        let (thread, side, panes) = shape(&view, cx);
        assert_eq!(panes, before, "the split is gone");
        assert!(thread.contains(&"thread".to_string()) && thread.contains(&"terminal".to_string()), "the tab stays in the thread's strip beside Thread");
        assert_eq!(side, thread, "that strip is where the tab stands");
        assert!(!view.read_with(cx, |view, _| view.side_chat_in_front()), "the thread is in front");
        let tab = view.read_with(cx, |view, _| {
            let arrangement = view.arrangement_of("mock-02").expect("the Job's panes");
            super::super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.terminal.as_deref() == Some("aside:mock-02")).map(|tab| tab.id.clone()).expect("the tab")
        });
        view.update_in(cx, |view, window, cx| view.activate_tab(&tab, window, cx));
        cx.run_until_parked();
        let (thread, side, panes) = shape(&view, cx);
        assert_eq!(panes, before + 1, "a press on the tab splits again");
        assert_eq!(side, vec!["terminal".to_string()]);
        assert!(!thread.contains(&"terminal".to_string()));
    }

    #[gpui::test]
    fn a_pane_too_narrow_for_two_halves_shows_the_side_chat_in_front_of_the_thread_and_w_brings_the_thread_back(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        let (_, _, before) = shape(&view, cx);
        view.update(cx, |view, _| view.arranged.area.set(Some(super::super::pane_tree::Size { w: 300., h: 600. })));
        view.update_in(cx, |view, window, cx| view.open_side_chat(None, window, cx));
        cx.run_until_parked();
        let (_, _, panes) = shape(&view, cx);
        assert_eq!(panes, before, "no room for a split");
        assert!(view.read_with(cx, |view, _| view.side_chat_in_front()), "the tab stands in front of the thread instead");
        let w = gpui::KeyDownEvent { keystroke: gpui::Keystroke::parse("cmd-w").expect("a key"), is_held: false, prefer_character_input: false };
        assert!(view.update_in(cx, |view, window, cx| view.side_chat_key(&w, window, cx)), "⌘W brings the thread back");
        cx.run_until_parked();
        assert!(!view.read_with(cx, |view, _| view.side_chat_in_front()), "the thread is in front again");
        assert_eq!(view.read_with(cx, |view, _| view.view_tab_ids("mock-02")), vec!["aside:mock-02".to_string()], "the tab stays");
    }

    #[gpui::test]
    fn a_tab_asked_for_the_top_with_the_split_open_lands_in_the_right_column(cx: &mut gpui::TestAppContext) {
        let (view, cx) = job(cx, "mock-02");
        view.update_in(cx, |view, window, cx| view.open_side_chat(None, window, cx));
        cx.run_until_parked();
        let chat = view.read_with(cx, |view, _| view.side_chats.get("mock-02").cloned()).expect("the Job's side chat");
        let mut other = view.update(cx, |view, _| view.side_tab("mock-02", &chat));
        other.id = "terminal:files".into();
        view.update(cx, |view, cx| view.open_view_tab("mock-02", other, true, cx));
        cx.run_until_parked();
        let (column, side) = view.read_with(cx, |view, _| {
            let arrangement = view.arrangement_of("mock-02").expect("the Job's panes");
            (super::super::panes_place::pane_for(arrangement, ViewPane::Top), view.view_tab_pane("mock-02", "terminal:files"))
        });
        assert_eq!(side, Some(ViewPane::Top), "it stands in the column's top pane");
        let held = view.read_with(cx, |view, _| {
            let arrangement = view.arrangement_of("mock-02").expect("the Job's panes");
            super::super::pane_tree::pane_of_tab(arrangement, &super::super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.terminal.as_deref() == Some("files")).expect("the tab").id).map(|pane| pane.id.clone())
        });
        assert_eq!(held, column, "and not beside the side chat");
        assert!(view.read_with(cx, |view, _| view.side_split_open()), "the side chat's split is as it was");
    }
}
