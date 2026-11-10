//! What waits on the person, over the message box: the Needs you band and the panel that opens from
//! it (`job-view.md`, B7). The question deck is `deck.rs` and the permission card `perm.rs`.
//!
//! **Electron draws a question or a permission in this panel, not in the thread.** The thread's tail
//! holds the plan and the pinned sketch; a question, a permission and the lapsed questions are here,
//! where `1` to `9` and the four `⌥` keys reach them with the caret still in the box.

use std::collections::{HashMap, HashSet};

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FocusHandle, Focusable as _, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{BuildSource, ConfirmWaiting, DismissWaiting, SessionId, Lapsed, ReachesTo, ThreadAsk, WaitingActKind, WaitingItem, WaitingSource};

use super::deck::Deck;
use super::perm::Perm;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What the panel remembers of one Job.
#[derive(Default)]
pub(super) struct DockState {
    /// The person's say on whether the panel is open; absent, it is shut.
    pub open: Option<bool>,
    /// The thing the panel is showing whole.
    pub detail: Option<String>,
    /// Questions that already opened the panel, so one opens it once.
    pub seen: HashSet<String>,
    pub decks: HashMap<String, Deck>,
    pub perms: HashMap<String, Perm>,
    /// Lapsed questions answered or skipped.
    pub hidden: HashSet<String>,
    /// Sentences read from a reply that the person said are questions, before Fleet's row says so.
    pub confirmed: HashSet<String>,
    pub said: HashMap<String, SharedString>,
    /// Where each command pressed here stands, by the command: a restart's key and a run item's row.
    pub runs: HashMap<String, RunMark>,
}

/// Where a command pressed in the panel stands, as Electron's `RestartKey` marks it.
#[derive(Clone)]
pub(super) enum RunMark {
    Running,
    Passed,
    Failed(SharedString),
}

impl RunMark {
    /// Its glyph and its words.
    fn look(&self) -> (&'static str, SharedString) {
        match self {
            RunMark::Running => ("loader-circle", "Running".into()),
            RunMark::Passed => ("circle-check", "Passed".into()),
            RunMark::Failed(why) => ("circle-x", why.clone()),
        }
    }
}

/// The build switch a run item's command asks for, as `build-change.ts` `buildChangeOf` reads it:
/// `scripts/preview --restart` or `scripts/restart --main`, with `--adopt` and nothing else beside.
/// Any other command, a dry run among them, is a command to run.
pub(super) fn restart_of(item: &WaitingItem) -> Option<(BuildSource, bool, &'static str)> {
    let act = item.act.as_ref().filter(|act| act.kind == WaitingActKind::Run)?;
    build_change_of(&act.target)
}

fn build_change_of(command: &str) -> Option<(BuildSource, bool, &'static str)> {
    let mut words = command.split_whitespace();
    let script = words.next()?;
    let flags: Vec<&str> = words.collect();
    let name = script.rsplit('/').next()?;
    let under = script.strip_suffix(name)?.trim_end_matches('/');
    if !(under == "scripts" || under.ends_with("/scripts")) {
        return None;
    }
    let (build, flag, label) = match name {
        "preview" => (BuildSource::Preview, "--restart", "Refresh preview"),
        "restart" => (BuildSource::Main, "--main", "Update to main"),
        _ => return None,
    };
    if !flags.contains(&flag) || flags.iter().any(|one| *one != flag && *one != "--adopt") {
        return None;
    }
    Some((build, flags.contains(&"--adopt"), label))
}

/// One thing the panel lists.
pub(super) enum Thing<'a> {
    Ask(&'a ThreadAsk),
    Lapsed(&'a Lapsed),
    Item(&'a WaitingItem),
}

pub(super) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// The menu of a sentence Fleet read from a reply: A question, unless it was said already, and Not a question.
fn misread_entries(confirmed: bool) -> Vec<Entry> {
    let mut entries = Vec::new();
    if !confirmed {
        entries.push(Entry::Act { id: "yes", label: "A question".into(), icon: Some("thumbs-up"), danger: false });
    }
    entries.push(Entry::Act { id: "no", label: "Not a question".into(), icon: Some("thumbs-down"), danger: false });
    entries
}

/// A row of the list: a filled, rounded press the panel's width, edged in the band's hue on hover.
fn filled(look: &Look, row: gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> {
    row.flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_HOVER))
        .cursor_pointer()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .hover(|style| style.border_color(look.color(token::STEP_WAITING)))
}

/// What a waiting item stands for in the list: its glyph and the words that name it.
fn mark_of(item: &WaitingItem) -> (&'static str, &'static str) {
    if !item.options.is_empty() {
        return ("message-circle-question-mark", "Question");
    }
    match item.act.as_ref().map(|act| act.kind) {
        Some(WaitingActKind::Walk) => ("app-window", "Opens the walk"),
        Some(WaitingActKind::Answer) => ("message-square", "Goes to the question"),
        Some(WaitingActKind::ApprovePr) => ("git-pull-request-arrow", "Opens the pull request"),
        Some(WaitingActKind::Run) => ("terminal", "Runs the command"),
        Some(WaitingActKind::Message) => ("square-pen", "Goes to the message"),
        None if !item.steps.is_empty() => ("list-checks", "Steps"),
        None => ("bell-ring", "Waiting on you"),
    }
}

/// What the thread's ask says it asks: the first question, or the command's first line.
pub(super) fn ask_text(ask: &ThreadAsk) -> String {
    ask.call.questions.first().map(|question| question.question.clone()).unwrap_or_else(|| first_line(&ask.command).to_string())
}

pub(super) fn ask_label(ask: &ThreadAsk) -> String {
    let call = &ask.call;
    if call.questions.is_empty() {
        return match call.reaches_to {
            Some(ReachesTo::Read) => "Permission to read".into(),
            Some(ReachesTo::Write) => "Permission to write".into(),
            Some(ReachesTo::Run) => "Permission to run".into(),
            None => "Permission".into(),
        };
    }
    let header = call.questions[0].header.trim();
    if header.is_empty() { "Question".into() } else { header.to_string() }
}

impl JobView {
    pub(super) fn dock_of(&self, id: &str) -> Option<&DockState> {
        self.pages.get(id).map(|page| &page.dock)
    }

    pub(super) fn dock_mut(&mut self) -> Option<&mut DockState> {
        let id = self.shown.clone()?;
        self.pages.get_mut(&id).map(|page| &mut page.dock)
    }

    /// The things the panel lists, in the order Electron lists them, and the pages waiting for review.
    pub(super) fn things<'a>(&'a self, id: &str, cx: &'a gpui::App) -> (Vec<Thing<'a>>, Vec<&'a WaitingItem>) {
        let mut things = Vec::new();
        let mut review = Vec::new();
        if let Some(held) = self.thread(id, cx) {
            if let Some(ask) = &held.head.asked {
                things.push(Thing::Ask(ask));
            } else if let Some(lapsed) = held.head.lapsed.as_ref().filter(|lapsed| self.dock_of(id).is_none_or(|dock| !dock.hidden.contains(&lapsed.id))) {
                things.push(Thing::Lapsed(lapsed));
            }
        }
        if let Some(record) = self.record(id, cx) {
            let hidden = self.dock_of(id).map(|dock| &dock.hidden);
            for item in record.waiting_for.iter().filter(|item| !hidden.is_some_and(|hidden| hidden.contains(&item.id))) {
                match item.source {
                    WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document => review.push(item),
                    _ if restart_of(item).is_some() => {}
                    WaitingSource::Agent | WaitingSource::Walk | WaitingSource::Replied => things.push(Thing::Item(item)),
                    _ => {}
                }
            }
        }
        (things, review)
    }

    /// The restarts the Job's items ask for, one key per command, whichever items ask it.
    pub(super) fn restarts(&self, id: &str, cx: &gpui::App) -> Vec<(String, BuildSource, bool, &'static str)> {
        let mut keys: Vec<(String, BuildSource, bool, &'static str)> = Vec::new();
        let Some(record) = self.record(id, cx) else { return keys };
        let hidden = self.dock_of(id).map(|dock| &dock.hidden);
        let listed = |item: &&WaitingItem| matches!(item.source, WaitingSource::Agent | WaitingSource::Walk | WaitingSource::Replied);
        for item in record.waiting_for.iter().filter(listed).filter(|item| !hidden.is_some_and(|hidden| hidden.contains(&item.id))) {
            let (Some((build, adopt, label)), Some(act)) = (restart_of(item), item.act.as_ref()) else { continue };
            if !keys.iter().any(|(command, ..)| *command == act.target) {
                keys.push((act.target.clone(), build, adopt, label));
            }
        }
        keys
    }

    pub(super) fn thing_key(thing: &Thing) -> String {
        match thing {
            Thing::Ask(ask) => format!("question:{}", ask.call.call),
            Thing::Lapsed(lapsed) => format!("question:{}", lapsed.id),
            Thing::Item(item) => format!("item:{}", item.id),
        }
    }

    /// A question that has not opened the panel yet opens it, once.
    pub(super) fn arrive(&mut self, id: &SharedString, cx: &gpui::App) {
        // Nothing missing forgets that the tools band was put away.
        if self.record(id, cx).is_some_and(|record| record.tools_missing.is_none()) {
            if let Some(page) = self.pages.get_mut(id) {
                page.bands.tools_away = None;
            }
        }
        let key = {
            let (things, _) = self.things(id, cx);
            things.iter().find_map(|thing| match thing {
                Thing::Ask(ask) => Some(ask.call.call.clone()),
                Thing::Lapsed(lapsed) => Some(lapsed.id.clone()),
                Thing::Item(_) => None,
            })
        };
        let Some(key) = key else { return };
        if let Some(page) = self.pages.get_mut(id) {
            if page.dock.seen.insert(key) {
                page.dock.open = Some(true);
            }
        }
    }

    pub(super) fn set_needs(&mut self, open: bool, detail: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            dock.open = Some(open);
            dock.detail = detail;
        }
        if open {
            window.focus(&self.dock_focus, cx);
        } else if self.shown.as_ref().is_some_and(|id| !self.closed(id, cx)) {
            self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// ⌥⌘Y: the Needs you panel opens or folds from anywhere in the Job, where ⇧⌘Y would open the review sheet.
    pub(super) fn toggle_needs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.review_open() {
            return;
        }
        // Pages to review alone have no list to open: the band's press opens the sheet (`NeedsYouPanel`).
        if self.things(&id, cx).0.is_empty() {
            return;
        }
        let up = self.needs_up(&id, cx);
        self.set_needs(!up, None, window, cx);
    }

    /// Whether the panel is up over the thread now. With only pages to review it has nothing to list and
    /// stays folded, as Electron's does, however it was left.
    pub(super) fn needs_up(&self, id: &str, cx: &gpui::App) -> bool {
        !self.things(id, cx).0.is_empty() && self.dock_of(id).and_then(|dock| dock.open).unwrap_or(false)
    }

    /// The thread dims while the panel is up; a press on it, or a scroll of it, folds the panel so the
    /// thread can be read (his word, 10 Nov). The scroll goes on to the thread.
    pub(super) fn render_scrim(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.needs_up(id, cx) && !self.band_up(id, cx) {
            return None;
        }
        let look = cx.look();
        Some(
            div()
                .id("needs-scrim")
                .absolute()
                .inset_0()
                .bg(look.color(token::BG_BASE).opacity(0.5))
                .on_click(cx.listener(|this, _, window, cx| this.fold_over_thread(window, cx)))
                .on_scroll(cx.listener(|this, _, window, cx| this.fold_over_thread(window, cx)))
                .into_any_element(),
        )
    }

    fn fold_over_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fold_bands(false, window, cx) {
            self.set_needs(false, None, window, cx);
        }
    }

    /// The band, and over it the panel when it is open.
    pub(super) fn render_needs(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let (things, review) = self.things(id, cx);
        let restarts = self.restarts(id, cx);
        if things.is_empty() && review.is_empty() && restarts.is_empty() {
            if let Some(page) = self.pages.get(id) {
                page.keys.mark("question:");
                page.keys.sweep();
            }
            return None;
        }
        // A band with nothing to list, a restart or pages to review alone, is never drawn open and has no fold.
        let open = self.needs_up(id, cx);
        let hue = look.color(token::STEP_WAITING);
        let glyphs: Vec<AnyElement> = things
            .iter()
            .map(|thing| match thing {
                Thing::Ask(ask) => ("message-circle-question-mark", ask_label(ask)),
                Thing::Lapsed(_) => ("message-circle-question-mark", "Unanswered".to_string()),
                Thing::Item(item) => (mark_of(item).0, mark_of(item).1.to_string()),
            })
            .chain(review.iter().map(|item| {
                let icon = match item.source {
                    WaitingSource::Plan => "list-checks",
                    WaitingSource::Sketch => "pencil-ruler",
                    _ => "file-text",
                };
                (icon, item.text.clone())
            }))
            .enumerate()
            .map(|(at, (icon, said))| {
                div()
                    .id(SharedString::from(format!("needs-glyph:{at}")))
                    .child(glyph(look, icon, Size::Small, look.color(token::FG_INVERSE)))
                    .tooltip(tip(said))
                    .into_any_element()
            })
            .collect();
        let trigger = labelled(
            div()
                .id("needs-trigger")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .child(glyph(look, "bell-ring", Size::Small, look.color(token::FG_INVERSE)))
                .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Needs you"))
                .child(div().flex().items_center().gap(look.length(token::SPACE_1)).children(glyphs))
                .child(div().flex_1())
                .when(!things.is_empty(), |trigger| {
                    trigger.child(glyph(look, if open { "chevron-down" } else { "chevron-up" }, Size::Small, look.color(token::FG_INVERSE)))
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    let Some(id) = this.shown.clone() else { return };
                    // With nothing else to answer, the band opens the sheet.
                    let alone = this.things(&id, cx).0.is_empty() && this.review_applies(&id, cx);
                    if alone {
                        this.review_press(window, cx);
                    } else {
                        let up = this.needs_up(&id, cx);
                        this.set_needs(!up, None, window, cx);
                    }
                })),
            Role::Button,
            "Needs you",
        );
        let high = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        let edge = look.length(token::BORDER_WIDTH);
        let band = div()
            .flex_none()
            .h(high)
            .flex()
            .items_center()
            .rounded_t(look.length(token::RADIUS_LG) - edge)
            .bg(hue)
            .text_color(look.color(token::FG_INVERSE))
            .overflow_hidden()
            .child(trigger)
            .children(restarts.into_iter().map(|(command, build, adopt, label)| self.restart_key(id, command, build, adopt, label, look, cx)));
        let keys = self.pages.get(id).map(|page| page.keys.clone());
        if let Some(keys) = &keys {
            keys.mark("question:");
        }
        let panel = open.then(|| self.render_panel(id, &things, &review, look, cx));
        if let Some(keys) = &keys {
            keys.sweep();
        }
        // The band heads the composer's frame. Open, it rises over the thread with the panel under it,
        // and the box below stays where it was.
        let slot = div().relative().flex_none().w_full().h(high);
        Some(
            match panel {
                Some(panel) => slot.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left(-edge)
                        .right(-edge)
                        .flex()
                        .flex_col()
                        .child(band.rounded_t(look.length(token::RADIUS_LG)))
                        .child(panel),
                ),
                None => slot.child(band),
            }
            .into_any_element(),
        )
    }

    /// A restart's key at the band's end, as Electron's `RestartKey`: its mark and its label, pressed
    /// folded or open. Nothing starts it again while it runs or once it passed; a failure leaves it
    /// to press again.
    #[allow(clippy::too_many_arguments)]
    fn restart_key(&self, id: &str, command: String, build: BuildSource, adopt: bool, label: &'static str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let mark = self.dock_of(id).and_then(|dock| dock.runs.get(&command).cloned());
        let held = matches!(mark, Some(RunMark::Running | RunMark::Passed));
        let (icon, said) = mark.as_ref().map_or(("rotate-cw", SharedString::from(command.clone())), RunMark::look);
        let accent = look.color(token::ACCENT);
        let hover = look.color(token::ACCENT_HOVER);
        labelled(
            div()
                .id(SharedString::from(format!("needs-restart:{command}")))
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .bg(accent)
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(!held, move |key| key.cursor_pointer().hover(move |style| style.bg(hover)))
                .child(glyph(look, icon, Size::Small, look.color(token::FG_INVERSE)))
                .child(label)
                .tooltip(tip(said.to_string()))
                .when(!held, |key| {
                    key.on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.restart(command.clone(), build, adopt, window, cx);
                    }))
                }),
            Role::Button,
            label,
        )
        .into_any_element()
    }

    /// Mark a command pressed in the shown Job.
    fn mark_run(&mut self, job: &SharedString, command: &str, mark: Option<RunMark>, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            match mark {
                Some(mark) => page.dock.runs.insert(command.to_string(), mark),
                None => page.dock.runs.remove(command),
            };
        }
        cx.notify();
    }

    /// The restart key: Fleet's build switch, pressed as the title bar's Refresh preview presses it
    /// (`sessions-wired.tsx` `switching`). Fleet answering that it began is the pass.
    fn restart(&mut self, command: String, build: BuildSource, adopt: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&ipc::ChangeFleetBuild { build, adopt }).map_err(|why| format!("{why:?}"));
        self.mark_run(&job, &command, Some(RunMark::Running), cx);
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            self.mark_run(&job, &command, Some(RunMark::Failed("Fleet is not connected".into())), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/fleet/build/change", body))?;
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let mark = match answer {
                Ok(()) => RunMark::Passed,
                Err(why) => RunMark::Failed(why.into()),
            };
            let _ = this.update(cx, |this, cx| this.mark_run(&job, &command, Some(mark), cx));
        })
        .detach();
    }

    /// A run item pressed: its command runs where the Job works (`start_session_run`), and its row is
    /// marked until the run exits. A clean exit is settled by Fleet, which takes the item off the list.
    pub(super) fn run_item(&mut self, command: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.dock_of(&job).and_then(|dock| dock.runs.get(&command)).is_some_and(|mark| matches!(mark, RunMark::Running)) {
            return;
        }
        let start = ipc::StartSessionRun { session_id: SessionId::carried(job.to_string()), command: command.clone(), cols: None, rows: None };
        let body = ipc::encode(&start).map_err(|why| format!("{why:?}"));
        self.mark_run(&job, &command, Some(RunMark::Running), cx);
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            self.mark_run(&job, &command, Some(RunMark::Failed("Fleet is not connected".into())), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    runtime.block_on(async move {
                        let (status, bytes) = crate::transport::post(port, "/sessions/run/start", body).await?;
                        if !(200..300).contains(&status) {
                            return Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
                        }
                        let underway = ipc::decode::<ipc::SessionRunUnderway>("SessionRunUnderway", &bytes).map_err(|why| format!("{why:?}"))?;
                        let io = crate::transport::upgrade(port, &format!("/sessions/run/{}/observe", underway.run_id)).await?;
                        let mut messages = crate::frames::Messages::new(io);
                        while let Some(text) = messages.next().await {
                            if let Ok(ipc::SessionRunMessage::Exited { code, stopped }) = ipc::decode::<ipc::SessionRunMessage>("SessionRunMessage", text.as_bytes()) {
                                return match code {
                                    Some(0) => Ok(()),
                                    Some(code) => Err(format!("Exited {code}")),
                                    None if stopped => Err("Stopped".to_string()),
                                    None => Err("Ended by a signal".to_string()),
                                };
                            }
                        }
                        Err("The run closed before it exited".to_string())
                    })
                })
                .await;
            let mark = match answer {
                Ok(()) => RunMark::Passed,
                Err(why) => RunMark::Failed(why.into()),
            };
            let _ = this.update(cx, |this, cx| this.mark_run(&job, &command, Some(mark), cx));
        })
        .detach();
    }

    fn render_panel(&self, id: &SharedString, things: &[Thing], review: &[&WaitingItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STEP_WAITING);
        let detail = self.dock_of(id).and_then(|dock| dock.detail.clone());
        let only = things.len() == 1 && review.is_empty() && matches!(things[0], Thing::Ask(_) | Thing::Lapsed(_));
        let detail = detail.or_else(|| only.then(|| Self::thing_key(&things[0])));
        let content: AnyElement = match detail.as_ref().and_then(|key| things.iter().find(|thing| &Self::thing_key(thing) == key)) {
            Some(thing) => self.render_detail(id, thing, only, look, cx),
            None => self.render_list(id, things, review, look, cx),
        };
        // Up to 95% of the thread, band included, so the Job's head always shows.
        let thread = self.pages.get(id).map(|page| page.list.viewport_bounds().size.height).unwrap_or_default();
        let band = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        let most = if thread > band { thread * 0.95 - band } else { look.length(token::SPACE_12) * 10. };
        div()
            .id("needs-panel")
            .track_focus(&self.dock_focus)
            .occlude()
            .on_key_down(cx.listener(Self::on_panel_key))
            .max_h(most)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_4))
            .border_x(look.length(token::BORDER_WIDTH))
            .border_color(hue)
            .bg(look.color(token::BG_RAISED))
            .child(content)
            .into_any_element()
    }

    fn on_panel_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let typing = self.dock_typing(window, cx);
        match event.keystroke.key.as_str() {
            "escape" => {
                let in_detail = self.dock_of(&id).is_some_and(|dock| dock.detail.is_some());
                self.set_needs(in_detail, None, window, cx);
                cx.stop_propagation();
            }
            "backspace" if !typing && self.dock_of(&id).is_some_and(|dock| dock.detail.is_some()) => {
                self.set_needs(true, None, window, cx);
                cx.stop_propagation();
            }
            key if !typing && key.len() == 1 && key.chars().all(|c| c.is_ascii_digit()) && !event.keystroke.modifiers.alt => {
                let n = key.parse::<usize>().unwrap_or(0);
                if n >= 1 {
                    self.pick_digit(n, window, cx);
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    /// The caret is in a box of the panel.
    fn dock_typing(&self, window: &Window, cx: &gpui::App) -> bool {
        let Some(id) = self.shown.as_ref() else { return false };
        self.pages
            .get(id)
            .is_some_and(|page| page.cards.boxes.iter().any(|(name, area)| name.starts_with("dock:") && area.read(cx).focus_handle(cx).contains_focused(window, cx)))
    }

    fn render_list(&self, id: &SharedString, things: &[Thing], review: &[&WaitingItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        type Press = Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;
        let row = |at: String, icon: &'static str, said: String, text: String, press: Press, misread: Option<&WaitingItem>| {
            let name = text.clone();
            let page = self.shown.as_ref().and_then(|job| self.pages.get(job));
            let spot = misread.zip(page).map(|(_, page)| page.keys.spot(&format!("needs-row:{at}"), cx));
            let refused = misread.and_then(|item| self.dock_of(self.shown.as_deref()?).and_then(|dock| dock.said.get(&format!("item:{}", item.id)).cloned()));
            let confirmed = misread.is_some_and(|item| item.confirmed || self.shown.as_deref().and_then(|job| self.dock_of(job)).is_some_and(|dock| dock.confirmed.contains(&item.id)));
            let thumbs = misread.zip(spot.as_ref()).map(|(item, spot)| self.thumbs(&item.id, confirmed, spot.focus.clone(), look, cx));
            let line = filled(look, div().id(SharedString::from(format!("needs-row:{at}"))))
                    .child(div().id(SharedString::from(format!("needs-mark:{at}"))).flex_none().child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED))).tooltip(tip(said)))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(text)))
                    .children(thumbs)
                    .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)));
            // A sentence Fleet read from a reply: a right press offers A question and Not a question.
            let line = match (misread, spot.as_ref()) {
                (Some(item), Some(spot)) => {
                    let item_id = item.id.clone();
                    self.menu_on(
                        line.track_focus(&spot.focus),
                        spot,
                        "Question",
                        spot.focus.clone(),
                        Rc::new(move || misread_entries(confirmed)),
                        Rc::new(move |this, choice, window, cx| match choice {
                            "yes" => this.misread_yes(&item_id, window, cx),
                            _ => this.misread_no(&item_id, window, cx),
                        }),
                        cx,
                    )
                }
                _ => line,
            };
            let line = labelled(line, Role::Button, format!("Open {name}")).into_any_element();
            match refused {
                Some(why) => div().flex().flex_col().child(line).child(super::acts::refusal(look, &at, why)).into_any_element(),
                None => line,
            }
        };
        let mut groups: Vec<(&'static str, Vec<AnyElement>)> =
            vec![("Questions", vec![]), ("To review", vec![]), ("Walks", vec![]), ("Commands", vec![]), ("PRs", vec![])];
        for thing in things {
            let key = Self::thing_key(thing);
            match thing {
                Thing::Ask(ask) => {
                    let key2 = key.clone();
                    groups[0].1.push(row(key, "message-circle-question-mark", ask_label(ask), ask_text(ask), Box::new(move |this, window, cx| this.set_needs(true, Some(key2.clone()), window, cx)), None));
                }
                Thing::Lapsed(lapsed) => {
                    let key2 = key.clone();
                    let text = lapsed.questions.first().map(|q| q.question.clone()).unwrap_or_default();
                    groups[0].1.push(row(key, "message-circle-question-mark", "Unanswered".into(), text, Box::new(move |this, window, cx| this.set_needs(true, Some(key2.clone()), window, cx)), None));
                }
                Thing::Item(item) => {
                    let (icon, said) = mark_of(item);
                    let ran = item.act.as_ref().filter(|act| act.kind == WaitingActKind::Run).and_then(|act| self.dock_of(id)?.runs.get(&act.target)).map(RunMark::look);
                    let (icon, said): (&'static str, SharedString) = ran.unwrap_or((icon, said.into()));
                    let kind = item.act.as_ref().map(|act| act.kind);
                    let group = match kind {
                        Some(WaitingActKind::Walk) => 2,
                        Some(WaitingActKind::Run) => 3,
                        None if !item.steps.is_empty() => 3,
                        Some(WaitingActKind::ApprovePr) => 4,
                        _ => 0,
                    };
                    let key2 = key.clone();
                    let target = item.act.as_ref().map(|act| act.target.clone());
                    let opens_here = !item.options.is_empty();
                    groups[group].1.push(row(
                        key,
                        icon,
                        said.into(),
                        item.text.clone(),
                        Box::new(move |this, window, cx| {
                            if opens_here {
                                this.set_needs(true, Some(key2.clone()), window, cx);
                            } else if kind == Some(WaitingActKind::Run) {
                                if let Some(target) = &target {
                                    this.run_item(target.clone(), window, cx);
                                }
                            } else if kind == Some(WaitingActKind::Message) {
                                this.set_needs(false, None, window, cx);
                                this.to_composer(window, cx);
                            } else {
                                this.set_needs(false, None, window, cx);
                            }
                        }),
                        (item.source == WaitingSource::Replied).then_some(&**item),
                    ));
                }
            }
        }
        // The sheet takes what it has a body for; the rest of the pages waiting go to their cards.
        let on_sheet: Vec<String> = self.review_items(id, cx).into_iter().map(|item| item.id).collect();
        let sheet = self.review_items(id, cx);
        if !sheet.is_empty() {
            let said = sheet.iter().map(|item| item.name.as_str()).collect::<Vec<_>>().join(", ");
            let marks = sheet.iter().map(|item| glyph(look, item.kind.mark().0, Size::Mark, look.color(token::FG_MUTED)));
            groups[1].1.push(
                labelled(
                    filled(look, div().id("needs-row:review"))
                        .child(div().id("needs-mark:review").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).children(marks).tooltip(tip("Opens the review")))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(said)))
                        .on_click(cx.listener(|this, _, window, cx| this.review_press(window, cx))),
                    Role::Button,
                    "Review",
                )
                .into_any_element(),
            );
        }
        let apart: Vec<&&WaitingItem> = review.iter().filter(|item| !on_sheet.contains(&item.id)).collect();
        if !apart.is_empty() {
            let said: Vec<String> = apart.iter().map(|item| item.text.clone()).collect();
            groups[1].1.push(row(
                "cards".into(),
                "list-checks",
                "Goes to the card".into(),
                said.join(", "),
                Box::new(|this, window, cx| {
                    this.set_needs(false, None, window, cx);
                    this.pin_shown();
                    cx.notify();
                }),
                None,
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(groups.into_iter().filter(|(_, rows)| !rows.is_empty()).map(|(name, rows)| {
                labelled(
                    div()
                        .id(SharedString::from(format!("needs-group:{name}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(
                            div()
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_MUTED))
                                .child(name),
                        )
                        .children(rows),
                    Role::Region,
                    name,
                )
            }))
            .into_any_element()
    }

    /// The thread goes to its end.
    pub(super) fn pin_shown(&mut self) {
        if let Some(id) = self.shown.clone() {
            self.pin(&id);
        }
    }

    /// The two thumbs at the end of a sentence Fleet read from a reply: A question, and Not a question.
    /// Confirmed, Fleet has nothing left to ask, so there are none.
    fn thumbs(&self, item: &str, confirmed: bool, back: FocusHandle, look: &Look, cx: &Context<Self>) -> AnyElement {
        let thumb = |id: &'static str, said: &'static str, icon: &'static str, yes: bool| {
            let (item, back) = (item.to_string(), back.clone());
            labelled(
                div()
                    .id(SharedString::from(format!("{id}:{item}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        if yes {
                            this.misread_yes(&item, window, cx);
                            window.focus(&back, cx);
                        } else {
                            this.misread_no(&item, window, cx);
                        }
                    })),
                Role::Button,
                said,
            )
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .children((!confirmed).then(|| thumb("misread-yes", "A question", "thumbs-up", true)))
            .child(thumb("misread-no", "Not a question", "thumbs-down", false))
            .into_any_element()
    }

    /// Thumbs up: the sentence is a question. It reads confirmed at once; a refusal takes that back.
    pub(super) fn misread_yes(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(dock) = self.dock_mut() {
            dock.confirmed.insert(item.to_string());
            dock.said.remove(&format!("item:{item}"));
        }
        let body = ipc::encode(&ConfirmWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string() }).map_err(|why| format!("{why:?}"));
        let (back, item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            "/sessions/waiting/confirm",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.dock.confirmed.remove(&item);
                    page.dock.said.insert(format!("item:{item}"), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    /// Thumbs down: the sentence asked nothing. It leaves the list at once; a refusal puts it back.
    pub(super) fn misread_no(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(dock) = self.dock_mut() {
            dock.hidden.insert(item.to_string());
            if dock.detail.as_deref() == Some(&format!("item:{item}")) {
                dock.detail = None;
            }
            dock.said.remove(&format!("item:{item}"));
        }
        let body = ipc::encode(&DismissWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string(), reason: None, misread: true }).map_err(|why| format!("{why:?}"));
        let (back, item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            "/sessions/waiting/dismiss",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.dock.hidden.remove(&item);
                    page.dock.said.insert(format!("item:{item}"), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    fn render_detail(&self, id: &SharedString, thing: &Thing, only: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let back = labelled(
            div()
                .id("needs-back")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Back"))
                .child(glyph(look, "chevron-left", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _, window, cx| this.set_needs(!only, None, window, cx))),
            Role::Button,
            "Back",
        );
        let body = match thing {
            Thing::Ask(ask) if ask.call.questions.is_empty() => self.permission(id, ask, look, cx),
            Thing::Ask(ask) => self.deck(id, &ask.call.call, &ask.call.questions, true, false, super::deck::Source::Call(ask.call.call.clone()), look, cx),
            Thing::Lapsed(lapsed) => self.deck(id, &lapsed.id, &lapsed.questions, true, false, super::deck::Source::Lapsed(lapsed.id.clone()), look, cx),
            Thing::Item(item) => {
                let questions = vec![ipc::AskedQuestion {
                    question: item.text.clone(),
                    header: String::new(),
                    multi_select: false,
                    options: item
                        .options
                        .iter()
                        .map(|option| ipc::AskedChoice { label: option.label.clone(), description: option.description.clone().unwrap_or_default(), preview: String::new() })
                        .collect(),
                }];
                self.deck(id, &item.id, &questions, false, true, super::deck::Source::Waiting(item.id.clone()), look, cx)
            }
        };
        let said = self.dock_of(id).and_then(|dock| dock.said.get(&Self::thing_key(thing)).cloned());
        // A sentence Fleet read from a reply carries its thumbs in the head too.
        let head_thumbs = match thing {
            Thing::Item(item) if item.source == WaitingSource::Replied => {
                let confirmed = item.confirmed || self.dock_of(id).is_some_and(|dock| dock.confirmed.contains(&item.id));
                Some(self.thumbs(&item.id, confirmed, self.focus.clone(), look, cx))
            }
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(back).children(head_thumbs))
            .child(div().w_full().min_w_0().child(body))
            .children(said.map(|why| super::acts::refusal(look, "dock", why)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::build_change_of;
    use ipc::BuildSource;

    #[test]
    fn a_restart_is_read_as_the_build_switch_it_asks_for() {
        assert_eq!(build_change_of("scripts/preview --restart"), Some((BuildSource::Preview, false, "Refresh preview")));
        assert_eq!(build_change_of("./scripts/preview --restart --adopt"), Some((BuildSource::Preview, true, "Refresh preview")));
        assert_eq!(build_change_of("/Users/x/armada/scripts/restart --main"), Some((BuildSource::Main, false, "Update to main")));
    }

    #[test]
    fn any_other_command_is_one_to_run() {
        assert_eq!(build_change_of("scripts/preview --restart --dry-run"), None);
        assert_eq!(build_change_of("scripts/preview"), None);
        assert_eq!(build_change_of("other/preview --restart"), None);
        assert_eq!(build_change_of("armada check test chip"), None);
    }
}
