//! Watching, at the foot of Now, as Armada Classic's `WatchingBlock` draws it: the Job's live watches,
//! and one that ended not ours while Needs you still asks about it, until Show ended draws the ended
//! ones too, each with Delete in a column every row keeps. A row is the state's mark, the name, what
//! its last reading said and how long ago, with what the agent made of that reading under it. A press
//! opens the watch's panel (`shared-panel/watch.tsx`): Readings, Parts, Command, Output for a broken
//! run and Log, with its link out and Stop while it is live.
//!
//! The watches are Fleet's, read once when a Job is shown and replaced whole by `watches.changed`.
//! Stop and Delete take effect on screen at once; a refusal puts the watch back and is said in a toast.

use chrono::Local;
use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, FontWeight, Role, SharedString, Task, Window};
use ipc::{DeleteWatch, SessionId, SessionRecord, StopWatch, WaitingItem, WaitingSource, WatchPart, WatchReading, WatchReadingState, WatchState, WatchSummary, WatchVerdictGiven, WatchVerdictWord};

use super::pr_panel::{icon_button, small_button};
use super::rows::{age, clock, glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// What he did to a watch that Fleet has not yet said back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Acted {
    Stopped,
    Deleted,
}

/// The panel's tabs, in Classic's order. Output is there only for a broken run that kept its output,
/// and Log only for a reading that kept a log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WatchTab {
    Readings,
    Parts,
    Command,
    Output,
    Log,
}

impl WatchTab {
    fn word(self) -> &'static str {
        match self {
            WatchTab::Readings => "Readings",
            WatchTab::Parts => "Parts",
            WatchTab::Command => "Command",
            WatchTab::Output => "Output",
            WatchTab::Log => "Log",
        }
    }
}

/// A watch open in the side panel. The watch itself is read from the Job's each draw.
pub(super) struct WatchPanel {
    pub job: SharedString,
    pub watch: String,
    pub name: String,
    pub tab: WatchTab,
    /// The log a reading's Log pressed; absent is the newest.
    picked: Option<String>,
    /// The log read, by its path: its text, or why Fleet did not give it.
    log: Option<(String, Result<String, String>)>,
    _read: Option<Task<()>>,
}

/// A state's mark: its glyph, its word and its hue (`WATCH_MARK`, `watching-block.css`).
fn mark_of(state: WatchState) -> (&'static str, &'static str, Token) {
    match state {
        WatchState::Waiting => ("circle-dashed", "Waiting", token::FG_MUTED),
        WatchState::Running => ("circle-dot", "Running", token::STATUS_RUNNING),
        WatchState::Passed => ("check", "Passed", token::STATUS_COMPLETED_SUCCESS),
        WatchState::Failed => ("x", "Failed", token::STATUS_COMPLETED_FAILED),
        WatchState::NotOurs => ("corner-up-right", "Not ours", token::STATUS_ESCALATED),
        WatchState::GaveUp => ("clock", "Gave up", token::STATUS_ESCALATED),
        WatchState::Broken => ("unplug", "Broken", token::STATUS_ESCALATED),
        WatchState::Stopped => ("ban", "Stopped", token::STATUS_KILLED),
    }
}

fn state_of(reading: WatchReadingState) -> WatchState {
    match reading {
        WatchReadingState::Waiting => WatchState::Waiting,
        WatchReadingState::Running => WatchState::Running,
        WatchReadingState::Passed => WatchState::Passed,
        WatchReadingState::Failed => WatchState::Failed,
    }
}

/// The mark, named by its tooltip: `said` where the state alone says too little.
fn mark(look: &Look, id: String, state: WatchState, said: Option<String>) -> AnyElement {
    let (icon, word, hue) = mark_of(state);
    let said: SharedString = said.unwrap_or_else(|| word.to_string()).into();
    labelled(div().id(SharedString::from(id)).flex_none().flex().items_center().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, look.color(hue))), Role::Image, said).into_any_element()
}

/// The kept readings, oldest first: `readings` where Fleet sends them, else `last` alone (`readingsOf`).
fn readings_of(watch: &WatchSummary) -> Vec<&WatchReading> {
    if watch.readings.is_empty() {
        watch.last.iter().collect()
    } else {
        watch.readings.iter().collect()
    }
}

fn at_of(reading: &WatchReading) -> &str {
    match reading {
        WatchReading::Read { at, .. } | WatchReading::Broken { at, .. } => at.as_str(),
    }
}

fn says_of(reading: &WatchReading) -> &str {
    match reading {
        WatchReading::Read { says, .. } => says,
        WatchReading::Broken { why, .. } => why,
    }
}

fn reading_state(reading: &WatchReading) -> WatchState {
    match reading {
        WatchReading::Read { state, .. } => state_of(*state),
        WatchReading::Broken { .. } => WatchState::Broken,
    }
}

fn log_of(reading: &WatchReading) -> Option<&String> {
    match reading {
        WatchReading::Read { log, .. } | WatchReading::Broken { log, .. } => log.as_ref(),
    }
}

/// The weighings of `reading`: given at or after it, and before the reading after it (`verdictsOn`).
fn verdicts_on<'a>(watch: &'a WatchSummary, reading: &WatchReading) -> Vec<&'a WatchVerdictGiven> {
    let at = at_of(reading);
    let next = readings_of(watch).into_iter().map(at_of).find(|one| *one > at);
    watch.verdicts.iter().filter(|one| one.at.as_str() >= at && next.is_none_or(|next| one.at.as_str() < next)).collect()
}

/// The weighing in words (`verdictSaid`).
fn verdict_said(verdict: &WatchVerdictGiven) -> String {
    let who = if verdict.verdict == WatchVerdictWord::Ours { "Ours" } else { "Not ours" };
    if verdict.sentence.is_empty() {
        who.to_string()
    } else {
        format!("{who}: {}", verdict.sentence)
    }
}

/// The parts of the newest reading that named any.
fn parts_of(watch: &WatchSummary) -> &[WatchPart] {
    readings_of(watch)
        .into_iter()
        .rev()
        .find_map(|one| match one {
            WatchReading::Read { parts, .. } if !parts.is_empty() => Some(parts.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

/// A watch's mark in words: its state's, and for not ours what failed (`watchSaid`).
fn watch_said(watch: &WatchSummary) -> Option<String> {
    if watch.state != WatchState::NotOurs {
        return None;
    }
    let failed: Vec<&str> = parts_of(watch)
        .iter()
        .filter(|part| part.state == WatchReadingState::Failed && watch.set_aside.iter().any(|one| one.part == part.name))
        .map(|part| part.name.as_str())
        .collect();
    (!failed.is_empty()).then(|| format!("Not ours: {} failed", failed.join(", ")))
}

/// The watches Needs you asks about, by id.
fn asked(waiting: &[WaitingItem]) -> Vec<&str> {
    waiting.iter().filter(|one| one.source == WaitingSource::WatchNotOurs).filter_map(|one| one.act.as_ref().map(|act| act.target.as_str())).collect()
}

/// `60s`, `2m`, `2h`: a cadence or a limit as a recipe writes one.
fn span(seconds: u32) -> String {
    if seconds % 3600 == 0 {
        format!("{}h", seconds / 3600)
    } else if seconds % 60 == 0 {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

/// A monospace block on the sunken ground, as `side-log__lines` and `side-watch__error` draw one.
fn block(look: &Look, id: String, text: &str, wraps: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id))
        .p(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(token::BG_SUNKEN))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_DEFAULT))
        .when(!wraps, |block| block.overflow_x_scroll())
        .child(div().flex().flex_col().children(text.split('\n').map(|line| {
            let line = div().child(SharedString::from(line.to_string()));
            if wraps { line } else { line.whitespace_nowrap() }
        })))
}

impl JobView {
    /// The Job's watches, live and ended, newest first: Fleet's, or on mock data the mock's, with what
    /// he did to them that Fleet has not yet said back.
    pub(super) fn watches_of(&self, id: &str, cx: &gpui::App) -> Vec<WatchSummary> {
        let mut list = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().watches.get(id).cloned().unwrap_or_default(),
            None => crate::mock::watches(id).unwrap_or_default(),
        };
        if let Some(page) = self.pages.get(id) {
            list.retain(|watch| page.side.watch_acted.get(&watch.id) != Some(&Acted::Deleted));
            for watch in &mut list {
                if page.side.watch_acted.get(&watch.id) == Some(&Acted::Stopped) {
                    watch.state = WatchState::Stopped;
                    watch.live = false;
                }
            }
        }
        list
    }

    /// Ask Fleet for the Job's watches once it is on screen; `watches.changed` keeps them after.
    pub(super) fn want_watches(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if self.shown.as_ref() != Some(id) {
            return;
        }
        if let Some(fleet) = self.fleet.clone() {
            fleet.update(cx, |fleet, cx| fleet.read_watches(id, cx));
        }
    }

    /// The Watching block, or nothing for a Job with no watch (`WatchingBlock`).
    pub(super) fn watching(&self, job: &SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let id = job.id.as_str();
        let watches = self.watches_of(id, cx);
        if watches.is_empty() {
            return None;
        }
        let ended = self.pages.get(id).is_some_and(|page| page.side.watches_ended);
        let any_ended = watches.iter().any(|watch| !watch.live);
        let asks = asked(&job.waiting_for);
        let shown: Vec<&WatchSummary> = watches.iter().filter(|watch| ended || watch.live || asks.contains(&watch.id.as_str())).collect();
        // A watch that ended not ours is his to settle: the block wears Needs you's hue, never running's.
        let hue = look.color(if shown.iter().any(|watch| watch.state == WatchState::NotOurs) { token::STEP_WAITING } else { token::STEP_RUNNING });
        let edge = look.length(token::BORDER_WIDTH);
        let now = Local::now();
        let rows: Vec<AnyElement> = shown
            .iter()
            .enumerate()
            .map(|(at, watch)| {
                let says = watch.last.as_ref().map(|last| says_of(last).to_string()).unwrap_or_default();
                let ago = watch.last.as_ref().and_then(|last| age(at_of(last), now)).unwrap_or_default();
                let open = (watch.id.clone(), watch.name.clone());
                let row = labelled(
                    div()
                        .id(SharedString::from(format!("watch-row:{}", watch.id)))
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .cursor_pointer()
                        .tab_index(0)
                        .hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.05)))
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .on_click(cx.listener(move |this, _, window, cx| this.open_watch_panel(open.0.clone(), open.1.clone(), window, cx)))
                        .child(mark(look, format!("watch-mark:{}", watch.id), watch.state, watch_said(watch)))
                        .child(div().flex_none().whitespace_nowrap().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(watch.name.clone())))
                        .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(says)))
                        .child(div().flex_none().whitespace_nowrap().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(ago))),
                    Role::Button,
                    format!("Watch {}", watch.name),
                );
                let delete = (!watch.live).then(|| {
                    let gone = watch.id.clone();
                    labelled(
                        icon_button(look, SharedString::from(format!("watch-delete:{}", watch.id)), "trash", "Delete".into())
                            .tab_index(0)
                            .on_click(cx.listener(move |this, _, window, cx| this.watch_delete(gone.clone(), window, cx))),
                        Role::Button,
                        format!("Delete watch {}", watch.name),
                    )
                });
                let line = div()
                    .flex()
                    .items_center()
                    .child(row)
                    .when(any_ended && ended, |line| line.child(div().flex_none().w(look.length(token::H_CONTROL_SM)).flex().justify_center().children(delete)));
                let weighed: Vec<AnyElement> = watch
                    .last
                    .as_ref()
                    .map(|last| verdicts_on(watch, last))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|one| {
                        div()
                            .text_color(look.color(if one.verdict == WatchVerdictWord::Ours { token::FG_DEFAULT } else { token::FG_MUTED }))
                            .child(SharedString::from(verdict_said(one)))
                            .into_any_element()
                    })
                    .collect();
                let verdicts = (!weighed.is_empty()).then(|| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("watch-verdicts:{}", watch.id)))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .pb(look.length(token::SPACE_2))
                            .pr(look.length(token::SPACE_3))
                            .pl(look.length(token::SPACE_3) * 2. + look.length(token::SPACE_2))
                            .children(weighed),
                        Role::List,
                        format!("Weighed by the agent: {}", watch.name),
                    )
                });
                // While Needs you asks about it, the row pulses in Needs you's hue, as the title bar's call does.
                let pulse = asks.contains(&watch.id.as_str()).then(|| {
                    div()
                        .absolute()
                        .inset_0()
                        .bg(look.color(token::STEP_WAITING))
                        .with_animation(
                            SharedString::from(format!("watch-asks:{}", watch.id)),
                            Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.05, 0.14)),
                            |layer, at| layer.opacity(at),
                        )
                });
                labelled(
                    div()
                        .id(SharedString::from(format!("watch:{}", watch.id)))
                        .relative()
                        .flex()
                        .flex_col()
                        .when(at > 0, |row| row.border_t(edge).border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.25))))
                        .children(pulse)
                        .child(line)
                        .children(verdicts),
                    Role::ListItem,
                    watch.name.clone(),
                )
                .into_any_element()
            })
            .collect();
        let job_id = job.id.as_str().to_string();
        let switch = any_ended.then(|| {
            labelled(
                super::aa::switch(look, "watching-ended", "Show ended", ended, false).on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(page) = this.pages.get_mut(job_id.as_str()) {
                        page.side.watches_ended = !page.side.watches_ended;
                    }
                    cx.notify();
                })),
                Role::Switch,
                "Show ended",
            )
        });
        let list = (!rows.is_empty()).then(|| {
            div()
                .id("watching-list")
                .relative()
                .overflow_hidden()
                .flex()
                .flex_col()
                .border(edge)
                .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED).blend(hue.opacity(0.05)))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).bg(hue))
                .children(rows)
        });
        Some(
            labelled(
                div()
                    .id("watching")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .pt(look.length(token::SPACE_3))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(look.length(token::SPACE_2))
                            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(hue).child("Watching"))
                            .children(switch),
                    )
                    .children(list),
                Role::Group,
                "Watching",
            )
            .into_any_element(),
        )
    }

    fn open_watch_panel(&mut self, watch: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let panel = WatchPanel { job, watch, name, tab: WatchTab::Readings, picked: None, log: None, _read: None };
        self.open_watch(panel, window, cx);
    }

    /// The tabs the watch has, in order.
    fn watch_tabs(watch: &WatchSummary) -> Vec<WatchTab> {
        let broken = readings_of(watch).into_iter().rev().any(|one| matches!(one, WatchReading::Broken { output: Some(_), .. }));
        let logged = readings_of(watch).into_iter().any(|one| log_of(one).is_some());
        [WatchTab::Readings, WatchTab::Parts, WatchTab::Command]
            .into_iter()
            .chain(broken.then_some(WatchTab::Output))
            .chain(logged.then_some(WatchTab::Log))
            .collect()
    }

    /// `[` and `]` go round the panel's tabs, as the side panel's keys do.
    pub(super) fn watch_panel_key(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.watch_panel() else { return };
        let Some(watch) = self.watches_of(&panel.job, cx).into_iter().find(|one| one.id == panel.watch) else { return };
        let tabs = Self::watch_tabs(&watch);
        let at = tabs.iter().position(|one| *one == panel.tab).unwrap_or(0);
        let next = tabs[(at + if back { tabs.len() - 1 } else { 1 }) % tabs.len()];
        self.watch_tab(next, window, cx);
    }

    fn watch_tab(&mut self, tab: WatchTab, window: &mut Window, cx: &mut Context<Self>) {
        let _ = window;
        if let Some(panel) = self.watch_panel_mut() {
            panel.tab = tab;
        }
        if tab == WatchTab::Log {
            self.watch_log_read(cx);
        }
        cx.notify();
    }

    /// Read the log the Log tab shows, unless it is the one held (`GET /watches/log?watch=&log=`).
    fn watch_log_read(&mut self, cx: &mut Context<Self>) {
        let Some(panel) = self.watch_panel() else { return };
        let Some(watch) = self.watches_of(&panel.job, cx).into_iter().find(|one| one.id == panel.watch) else { return };
        let logs: Vec<String> = readings_of(&watch).into_iter().rev().filter_map(log_of).cloned().collect();
        let Some(log) = panel.picked.clone().filter(|one| logs.contains(one)).or_else(|| logs.first().cloned()) else { return };
        if panel.log.as_ref().is_some_and(|(held, _)| *held == log) {
            return;
        }
        let watch_id = panel.watch.clone();
        let read = match self.port(cx) {
            None => {
                let answer = crate::mock::watch_log(&watch_id, &log).map(|one| one.text).ok_or_else(|| "Fleet kept no such log".to_string());
                if let Some(panel) = self.watch_panel_mut() {
                    panel.log = Some((log, answer));
                }
                return;
            }
            Some(port) => {
                let path = format!("/watches/log?watch={}&log={}", crate::link::percent_encoded(&watch_id), crate::link::percent_encoded(&log));
                let asked = log.clone();
                cx.spawn(async move |this, cx| {
                    let answer = cx
                        .background_executor()
                        .spawn(async move {
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                            if !(200..300).contains(&status) {
                                return Err(super::wire::reason(status, &bytes));
                            }
                            ipc::decode::<ipc::WatchLog>("WatchLog", &bytes).map(|one| one.text).map_err(|why| why.to_string())
                        })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(panel) = this.watch_panel_mut() {
                            if panel.picked.as_ref().is_none_or(|one| *one == asked) {
                                panel.log = Some((asked, answer));
                            }
                        }
                        cx.notify();
                    });
                })
            }
        };
        if let Some(panel) = self.watch_panel_mut() {
            panel._read = Some(read);
        }
    }

    /// The watch's panel body: its heading with the link out, its mark, Stop while live, and its tabs.
    pub(super) fn watch_body(&self, panel: &WatchPanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(watch) = self.watches_of(&panel.job, cx).into_iter().find(|one| one.id == panel.watch) else {
            return div().into_any_element();
        };
        let readings = readings_of(&watch);
        let link = readings.iter().rev().find_map(|one| match one {
            WatchReading::Read { link: Some(link), .. } => Some(link.clone()),
            _ => None,
        });
        let open = link.map(|address| labelled(icon_button(look, "watch-open", "external-link", "Open in browser".into()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&address))), Role::Button, "Open in browser"));
        let heading = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(watch.name.clone())))
            .children(open);
        let (_, word, hue) = mark_of(watch.state);
        let meta = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .child(mark(look, "watch-panel-mark".into(), watch.state, watch_said(&watch)))
            .child(div().text_color(look.color(hue)).child(SharedString::from(watch_said(&watch).unwrap_or_else(|| word.to_string()))))
            .children(watch.follows.map(|number| div().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("#{number}")))));
        let stopping = watch.id.clone();
        let acts = watch.live.then(|| {
            labelled(
                div().id("watch-acts").flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(
                    small_button(look, "watch-stop", "Stop", false, true)
                        .border_1()
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .tab_index(0)
                        .on_click(cx.listener(move |this, _, window, cx| this.watch_stop(stopping.clone(), window, cx))),
                    Role::Button,
                    format!("Stop watch {}", watch.name),
                )),
                Role::Group,
                "Acts",
            )
        });
        let tabs = Self::watch_tabs(&watch);
        let tab = if tabs.contains(&panel.tab) { panel.tab } else { WatchTab::Readings };
        let strip = div().id("watch-tabs").flex().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children(tabs.iter().map(|which| {
            let (which, front) = (*which, *which == tab);
            labelled(
                div()
                    .id(SharedString::from(format!("watch-tab:{}", which.word())))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .border_b(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(if front { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                    .on_click(cx.listener(move |this, _, window, cx| this.watch_tab(which, window, cx)))
                    .child(which.word()),
                Role::Tab,
                which.word(),
            )
        }));
        let shown = match tab {
            WatchTab::Readings => self.watch_readings(&watch, look, cx),
            WatchTab::Parts => watch_parts(&watch, look, cx),
            WatchTab::Command => watch_command(&watch, look),
            WatchTab::Output => {
                let broken = readings.iter().rev().find_map(|one| match one {
                    WatchReading::Broken { why, output: Some(output), .. } => Some((why.clone(), output.clone())),
                    _ => None,
                });
                let (why, output) = broken.unwrap_or_default();
                labelled(
                    div().id("watch-output").flex().flex_col().gap(look.length(token::SPACE_2)).child(div().text_size(look.length(token::TEXT_XS)).child(SharedString::from(why))).child(block(look, "watch-output-lines".into(), &output, false)),
                    Role::Region,
                    "Output",
                )
                .into_any_element()
            }
            WatchTab::Log => {
                let logs: Vec<String> = readings.iter().rev().filter_map(|one| log_of(one)).cloned().collect();
                let log = panel.picked.clone().filter(|one| logs.contains(one)).or_else(|| logs.first().cloned()).unwrap_or_default();
                let read = panel.log.as_ref().filter(|(held, _)| *held == log).map(|(_, read)| read.clone());
                let copy = read.as_ref().and_then(|read| read.as_ref().ok()).cloned().map(|text| {
                    labelled(icon_button(look, "watch-log-copy", "copy", "Copy log".into()).on_click(cx.listener(move |_, _, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone())))), Role::Button, "Copy log")
                });
                let body = match read {
                    None => div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Reading the log").into_any_element(),
                    Some(Err(said)) => div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said)).into_any_element(),
                    Some(Ok(text)) => block(look, "watch-log-lines".into(), &text, false).into_any_element(),
                };
                labelled(
                    div()
                        .id("watch-log")
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(log))).children(copy))
                        .child(body),
                    Role::Region,
                    "Log",
                )
                .into_any_element()
            }
        };
        div()
            .id("watch-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p(look.length(token::PAD_CARD))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(heading)
            .child(meta)
            .children(acts)
            .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).child(labelled(strip, Role::TabList, "Watch")).child(shown))
            .into_any_element()
    }

    /// Each reading, newest first: its time, or a run's first above its last, its mark, what it said,
    /// how many it stands for or its Log, and what the agent made of it under it (`Readings`).
    fn watch_readings(&self, watch: &WatchSummary, look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = Local::now();
        let rows = readings_of(watch).into_iter().rev().enumerate().map(|(at, reading)| {
            let (count, last_at) = match reading {
                WatchReading::Read { count, last_at, .. } | WatchReading::Broken { count, last_at, .. } => (*count, last_at.clone()),
            };
            let when = div()
                .flex_none()
                .flex()
                .flex_col()
                .text_color(look.color(token::FG_SUBTLE))
                .whitespace_nowrap()
                .child(SharedString::from(clock(at_of(reading), now).unwrap_or_default()))
                .children(last_at.and_then(|last| clock(last.as_str(), now)).map(SharedString::from));
            let says = says_of(reading).to_string();
            let end: AnyElement = if count > 1 {
                labelled(div().id(SharedString::from(format!("watch-count:{at}"))).flex_none().text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("\u{d7}{count}"))), Role::Image, format!("Read {count} times in a row")).into_any_element()
            } else if let Some(log) = log_of(reading).cloned() {
                labelled(
                    small_button(look, SharedString::from(format!("watch-log:{at}")), "Log", false, true).on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(panel) = this.watch_panel_mut() {
                            panel.picked = Some(log.clone());
                        }
                        this.watch_tab(WatchTab::Log, window, cx);
                    })),
                    Role::Button,
                    format!("Log of {says}"),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            };
            let verdicts: Vec<AnyElement> = verdicts_on(watch, reading)
                .into_iter()
                .map(|one| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .text_color(look.color(if one.verdict == WatchVerdictWord::Ours { token::FG_DEFAULT } else { token::FG_MUTED }))
                        .child(SharedString::from(verdict_said(one)))
                        .children(evidence(one, look))
                        .into_any_element()
                })
                .collect();
            div()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(when)
                .child(mark(look, format!("watch-reading-mark:{at}"), reading_state(reading), None))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(SharedString::from(says.clone())).children(verdicts))
                .child(end)
        });
        labelled(div().id("watch-readings").flex().flex_col().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).children(rows), Role::List, "Readings").into_any_element()
    }

    fn watch_stop(&mut self, watch: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.watch_panel().map(|panel| panel.job.clone()).or_else(|| self.shown.clone()) else { return };
        self.watch_act(&job, &watch, Some(Acted::Stopped), cx);
        // Stop goes with the watch's life: the keys stay in the panel.
        self.keys_to_panel(window, cx);
        let body = ipc::encode(&StopWatch { session_id: Some(SessionId::carried(job.to_string())), watch: watch.clone() }).map_err(|why| format!("{why:?}"));
        let owner = job.clone();
        self.post_with(job, "/watches/stop", body, move |this, why, cx| this.watch_refused(&owner, &watch, why, cx), window, cx);
    }

    fn watch_delete(&mut self, watch: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.watch_act(&job, &watch, Some(Acted::Deleted), cx);
        // The pressed button is gone with its row: the keys go to the message box, so they still reach the Job.
        let keys = gpui::Focusable::focus_handle(self.composer.input.read(cx), cx);
        window.focus(&keys, cx);
        let body = ipc::encode(&DeleteWatch { session_id: Some(SessionId::carried(job.to_string())), watch: watch.clone() }).map_err(|why| format!("{why:?}"));
        let owner = job.clone();
        self.post_with(job, "/watches/delete", body, move |this, why, cx| this.watch_refused(&owner, &watch, why, cx), window, cx);
    }

    fn watch_act(&mut self, job: &SharedString, watch: &str, acted: Option<Acted>, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            match acted {
                Some(acted) => page.side.watch_acted.insert(watch.to_string(), acted),
                None => page.side.watch_acted.remove(watch),
            };
        }
        cx.notify();
    }

    fn watch_refused(&mut self, job: &SharedString, watch: &str, why: SharedString, cx: &mut Context<Self>) {
        self.watch_act(job, watch, None, cx);
        cx.defer(move |cx| crate::toasts::tell(why, cx));
    }
}

/// What a weighing carried to back it: each exact error, link and log, one a line (`Evidence`).
fn evidence(verdict: &WatchVerdictGiven, look: &Look) -> Option<AnyElement> {
    let found = &verdict.evidence;
    if found.is_empty() {
        return None;
    }
    let errors = found.errors.iter().enumerate().map(|(at, one)| block(look, format!("watch-evidence-error:{at}"), one, true).into_any_element());
    let lines = found.links.iter().chain(found.logs.iter()).map(|one| div().font_family(mono(look)).child(SharedString::from(one.clone())).into_any_element());
    Some(labelled(div().id("watch-evidence").flex().flex_col().gap(look.length(token::SPACE_1)).children(errors).children(lines), Role::List, "Evidence").into_any_element())
}

/// The newest reading's parts, those set aside struck through with the sentence that set them aside,
/// and a failed part's own link, the lines it failed on and the end of its log (`Parts`).
fn watch_parts(watch: &WatchSummary, look: &Look, cx: &Context<JobView>) -> AnyElement {
    let parts = parts_of(watch);
    let aside = |name: &str| watch.set_aside.iter().find(|one| one.part == name).map(|one| one.sentence.clone());
    let row = |key: String, state: Option<WatchState>, name: &str, sentence: Option<String>, part: Option<&WatchPart>| {
        let struck = sentence.is_some();
        let said = part.map(|part| {
            let link = part.link.clone().map(|address| {
                let to = address.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_color(look.color(token::FG_MUTED))
                    .child(div().min_w_0().font_family(mono(look)).child(SharedString::from(address)))
                    .child(labelled(icon_button(look, SharedString::from(format!("watch-part-open:{}", part.name)), "external-link", "Open in browser".into()).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&to))), Role::Button, "Open in browser"))
            });
            let error = part.error.as_ref().map(|error| labelled(block(look, format!("watch-part-error:{}", part.name), error, true), Role::Region, format!("{} failed on", part.name)));
            let tail = part.tail.as_ref().map(|tail| {
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().text_color(look.color(token::FG_MUTED)).child("The end of its log"))
                    .child(block(look, format!("watch-part-tail:{}", part.name), tail, true))
            });
            (link.is_some() || error.is_some() || tail.is_some()).then(|| {
                labelled(div().id(SharedString::from(format!("watch-part-evidence:{}", part.name))).flex().flex_col().gap(look.length(token::SPACE_1)).pt(look.length(token::SPACE_1)).children(link).children(error).children(tail), Role::Group, format!("{} evidence", part.name))
            })
        });
        div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(div().flex_none().w(look.length(token::SPACE_3)).children(state.map(|state| mark(look, key.clone(), state, None))))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().font_family(mono(look)).text_color(look.color(if struck { token::FG_MUTED } else { token::FG_DEFAULT })).when(struck, |name| name.line_through()).child(SharedString::from(name.to_string())))
                    .children(sentence.map(|sentence| div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("Not ours: {sentence}")))))
                    .children(said.flatten()),
            )
            .into_any_element()
    };
    let named = parts.iter().map(|part| row(format!("watch-part-mark:{}", part.name), Some(state_of(part.state)), &part.name, aside(&part.name), Some(part)));
    // A part set aside that the newest reading no longer names still shows why it was set aside.
    let gone = watch.set_aside.iter().filter(|one| !parts.iter().any(|part| part.name == one.part)).map(|one| row(format!("watch-part-gone:{}", one.part), None, &one.part, Some(one.sentence.clone()), None));
    let rows: Vec<AnyElement> = named.chain(gone).collect();
    labelled(div().id("watch-parts").flex().flex_col().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).children(rows), Role::List, "Parts").into_any_element()
}

/// The whole command Fleet runs, with its recipe, cadence, limit and start (`Command`).
fn watch_command(watch: &WatchSummary, look: &Look) -> AnyElement {
    let fact = |term: &'static str, value: AnyElement| {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(div().flex_none().w(look.length(token::SPACE_8) * 3.).text_color(look.color(token::FG_MUTED)).child(term))
            .child(value)
    };
    let monospace = |text: String| div().font_family(mono(look)).child(SharedString::from(text)).into_any_element();
    let started = clock(watch.started_at.as_str(), Local::now()).unwrap_or_default();
    labelled(
        div()
            .id("watch-command")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(block(look, "watch-command-line".into(), &watch.command.join(" "), true))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(watch.recipe.clone().map(|recipe| fact("Recipe", monospace(recipe))))
                    .child(fact("Every", monospace(span(watch.every_seconds))))
                    .child(fact("Gives up after", monospace(span(watch.gives_up_after_seconds))))
                    .child(fact("Started", div().child(SharedString::from(started)).into_any_element())),
            ),
        Role::Region,
        "Command",
    )
    .into_any_element()
}
