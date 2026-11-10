//! The bands over the message box that are not Needs you: the agent came up without some of its tools,
//! the agent has nothing left to do, and the Job waits on something that is not the person
//! (`ToolsBand`, `SessionEndBand`, `WaitingBand`). They share the Needs you band's shape: a bar in one
//! hue, and a panel that rides up from it. Needs you has the slot first, so the end and waiting bands
//! wait until nothing waits on the person.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{
    ActOnSessionMcp, CloseSession, DismissWait, DismissWaiting, HeldOpenBy, McpServerAct, MendSessionTools, PullRequestWait, SessionId,
    StopWatch, ToolMissing, ToolsMissing, WatchList,
};

use super::acts::{button, refusal, Tone};
use super::keys::{Card, Run};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

const DONE: &str = "done";

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// What a band remembers of one Job.
#[derive(Default)]
pub(super) struct BandState {
    /// The end band's panel; absent, it is open (it opens on coming into the Job).
    pub end_open: Option<bool>,
    pub wait_open: bool,
    /// The `done_since` of the ending Keep going put away.
    pub kept: Option<String>,
    /// The pieces that were missing when the person put the tools band away.
    pub tools_away: Option<Vec<ToolMissing>>,
    /// Pieces being put right now.
    pub mending: std::collections::HashSet<&'static str>,
    /// Why a piece, or the end, or a wait was refused.
    pub said: std::collections::HashMap<String, SharedString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaitKind {
    Watch,
    Merge,
    Checks,
}

impl WaitKind {
    fn glyph(self) -> &'static str {
        match self {
            WaitKind::Watch => "radar",
            WaitKind::Merge => "git-merge",
            WaitKind::Checks => "circle-dot",
        }
    }
}

/// One thing that keeps the Job open, in the words its tooltip says (`waitOf`).
pub(super) fn wait_of(held: &HeldOpenBy) -> (WaitKind, String) {
    match held {
        HeldOpenBy::Watch { name } => (WaitKind::Watch, format!("Held open by {name}")),
        HeldOpenBy::Job { name } => (WaitKind::Watch, format!("Waiting on {name}")),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::Queue } => (WaitKind::Merge, format!("#{number} in the merge queue")),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::Checks { auto_merge } } => (
            WaitKind::Checks,
            if *auto_merge { format!("#{number} to auto-merge once Checks pass") } else { format!("Checks running on #{number}") },
        ),
        HeldOpenBy::PullRequest { number, on: PullRequestWait::AutoMerge } => (WaitKind::Merge, format!("#{number} to auto-merge")),
    }
}

fn piece_said(piece: ToolMissing) -> &'static str {
    match piece {
        ToolMissing::Mod => "No armada mod.",
        ToolMissing::Door => "No armada tools.",
        ToolMissing::TaskTools => "No task tools.",
    }
}

fn piece_name(piece: ToolMissing) -> &'static str {
    match piece {
        ToolMissing::Mod => "mod",
        ToolMissing::Door => "door",
        ToolMissing::TaskTools => "task_tools",
    }
}

/// The harness and its version, then a sentence for each thing missing (`toolsMissingSaid`).
pub(super) fn tools_said(tools: &ToolsMissing) -> String {
    let mut parts = Vec::new();
    if !tools.harness_version.is_empty() {
        parts.push(format!("{} {}.", tools.harness, tools.harness_version));
    }
    parts.extend(tools.missing.iter().map(|piece| piece_said(*piece).to_string()));
    parts.join(" ")
}

/// The act that puts one piece right, where this build can do it.
fn piece_act(piece: ToolMissing) -> Option<&'static str> {
    match piece {
        ToolMissing::Mod => Some("Restart the agent"),
        ToolMissing::Door => Some("Reconnect"),
        // The update runs in the Job's terminal, which the native Job has no pane for yet.
        ToolMissing::TaskTools => None,
    }
}

/// Whether the tools band shows: not put away for these pieces. A piece newly missing brings it back.
pub(super) fn tools_shown(missing: &[ToolMissing], away: Option<&[ToolMissing]>) -> bool {
    !missing.is_empty() && away.is_none_or(|gone| missing.iter().any(|piece| !gone.contains(piece)))
}

/// Which bands one Job draws now.
pub(super) struct Shown {
    pub tools: bool,
    pub end: bool,
    pub waiting: bool,
}

impl JobView {
    pub(super) fn bands_of(&self, id: &str, cx: &gpui::App) -> Shown {
        let none = Shown { tools: false, end: false, waiting: false };
        let (Some(record), Some(page)) = (self.record(id, cx), self.pages.get(id)) else { return none };
        let (things, review) = self.things(id, cx);
        let waits = !things.is_empty() || !review.is_empty();
        let tools = record.tools_missing.as_ref().is_some_and(|tools| tools_shown(&tools.missing, page.bands.tools_away.as_deref()));
        let done = record.done_since.as_ref().map(|since| since.as_str().to_string());
        let asked = self.thread(id, cx).is_some_and(|held| held.head.asked.is_some());
        let end = done.is_some() && done != page.bands.kept && !self.working(id, cx) && !asked && !self.closed(id, cx) && !waits;
        let failed = record.attachments.iter().any(|one| one.kind == "handoff_failed" && one.state == ipc::AttachmentState::Standing);
        let held = record.mark.is_some_and(|mark| mark.state == ipc::JobState::Held);
        let waiting = held && !waits && !end && !failed && !tools && !record.held_open_by.is_empty();
        Shown { tools, end, waiting }
    }

    /// Whether the end or waiting panel is up over the thread.
    pub(super) fn band_up(&self, id: &str, cx: &gpui::App) -> bool {
        let shown = self.bands_of(id, cx);
        let Some(page) = self.pages.get(id) else { return false };
        (shown.end && page.bands.end_open.unwrap_or(true)) || (shown.waiting && page.bands.wait_open)
    }

    /// The bands, one over the other, each with its panel when it is open.
    pub(super) fn render_bands(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let shown = self.bands_of(id, cx);
        // The Done card is one of the Job's waiting cards while its band is drawn.
        if let Some(page) = self.pages.get(id) {
            page.keys.set(DONE, shown.end.then(|| self.done_card(id)));
        }
        if !(shown.tools || shown.end || shown.waiting) {
            return None;
        }
        let record = self.record(id, cx)?;
        let mut column = div().flex().flex_col().flex_none().w_full();
        if shown.tools {
            if let Some(tools) = &record.tools_missing {
                column = column.child(self.tools_band(id, tools, look, cx));
            }
        }
        if shown.end {
            column = column.child(self.end_band(id, record, look, cx));
        }
        if shown.waiting {
            column = column.child(self.waiting_band(id, &record.held_open_by, look, cx));
        }
        Some(column.into_any_element())
    }

    /// The bar every band draws: a trigger, and the cross of those that can be put away.
    fn bar(&self, look: &Look, hue: gpui::Hsla, open: bool, trigger: AnyElement, away: Option<AnyElement>) -> gpui::Div {
        div()
            .flex_none()
            .h(look.length(token::SPACE_6) + look.length(token::SPACE_1))
            .flex()
            .items_center()
            .rounded_t(look.length(if open { token::RADIUS_LG } else { token::RADIUS_MD }))
            .bg(hue)
            .text_color(look.color(token::FG_INVERSE))
            .overflow_hidden()
            .child(trigger)
            .children(away)
    }

    /// The panel above its bar: the same frame the Needs you panel has.
    fn drawer(&self, look: &Look, hue: gpui::Hsla, name: &'static str, body: AnyElement, cx: &Context<Self>) -> AnyElement {
        div()
            .id(SharedString::from(format!("{name}-panel")))
            .occlude()
            .on_key_down(cx.listener(Self::on_band_key))
            .max_h(look.length(token::SPACE_12) * 10.)
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
            .child(body)
            .into_any_element()
    }

    fn frame(&self, name: &'static str, panel: Option<AnyElement>, bar: gpui::Div) -> gpui::Stateful<gpui::Div> {
        div()
            .id(SharedString::from(format!("{name}-band")))
            .relative()
            .flex_none()
            .w_full()
            .children(panel.map(|panel| div().absolute().bottom_full().left_0().right_0().child(panel)))
            .child(bar)
    }

    fn trigger(&self, id: &SharedString, look: &Look, key: &str, name: &'static str, icon: &'static str, glyphs: Vec<AnyElement>, open: bool, cx: &Context<Self>, press: impl Fn(&mut JobView, &mut Window, &mut Context<JobView>) + 'static) -> AnyElement {
        let spot = self.pages.get(id).map(|page| page.keys.spot(key, cx));
        let base = div()
            .id(SharedString::from(key.to_string()))
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .cursor_pointer()
            .focus_visible(|style| style.bg(look.color(token::FG_INVERSE).opacity(0.18)))
            .child(glyph(look, icon, Size::Small, look.color(token::FG_INVERSE)))
            .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(name))
            .child(div().flex().items_center().gap(look.length(token::SPACE_1)).children(glyphs))
            .child(div().flex_1())
            .child(glyph(look, if open { "chevron-down" } else { "chevron-up" }, Size::Small, look.color(token::FG_INVERSE)))
            .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)));
        let base = match &spot {
            Some(spot) => base.track_focus(&spot.focus),
            None => base,
        };
        labelled(base, Role::Button, name).into_any_element()
    }

    /// Esc in a panel folds it and the band's own button has the focus again.
    fn on_band_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.fold_bands(true, window, cx) {
            cx.stop_propagation();
        }
    }

    /// Esc anywhere in the Job folds an open end or waiting panel, tools band excepted.
    pub(super) fn on_job_escape(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.fold_bands(false, window, cx) {
            cx.stop_propagation();
        }
    }

    /// Fold the panel that is up. The band's button takes the focus back only if it was inside the panel.
    pub(super) fn fold_bands(&mut self, from_inside: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        let shown = self.bands_of(&id, cx);
        let Some(page) = self.pages.get_mut(&id) else { return false };
        let key = if shown.end && page.bands.end_open.unwrap_or(true) {
            page.bands.end_open = Some(false);
            "end-trigger"
        } else if shown.waiting && page.bands.wait_open {
            page.bands.wait_open = false;
            "waiting-trigger"
        } else {
            return false;
        };
        if from_inside {
            let spot = page.keys.spot(key, cx);
            window.focus(&spot.focus, cx);
        }
        cx.notify();
        true
    }

    // ---- Tools missing ------------------------------------------------------------------------------

    fn tools_band(&self, id: &SharedString, tools: &ToolsMissing, look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_COMPLETED_FAILED);
        let said = tools_said(tools);
        let head = tools.missing.iter().fold(said.clone(), |left, piece| left.replace(piece_said(*piece), "")).trim().to_string();
        let page = self.pages.get(id);
        let rows = tools.missing.iter().map(|piece| {
            let piece = *piece;
            let running = page.is_some_and(|page| page.bands.mending.contains(piece_name(piece)));
            let error = page.and_then(|page| page.bands.said.get(&format!("tools:{}", piece_name(piece))).cloned());
            let act = piece_act(piece).map(|act| {
                labelled(
                    button(look, SharedString::from(format!("tools-fix:{}", piece_name(piece))), act, Tone::Secondary, false, None, !running)
                        .on_click(cx.listener(move |this, _, window, cx| this.mend(piece, window, cx))),
                    Role::Button,
                    act,
                )
            });
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(look.length(token::SPACE_3))
                        .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(piece_said(piece)))
                        .children(act),
                )
                .children(error.map(|why| refusal(look, piece_name(piece), why)))
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .children((!head.is_empty()).then(|| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(head))))
            .children(rows)
            .into_any_element();
        let trigger = labelled(
            div()
                .id("tools-trigger")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .child(glyph(look, "unplug", Size::Small, look.color(token::FG_INVERSE)))
                .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Tools missing")),
            Role::Label,
            "Tools missing",
        )
        .into_any_element();
        let away = labelled(
            div()
                .id("tools-dismiss")
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::FG_INVERSE).opacity(0.18)))
                .tooltip(tip("Dismiss"))
                .child(glyph(look, "x", Size::Small, look.color(token::FG_INVERSE)))
                .on_click(cx.listener(|this, _, _, cx| this.put_tools_away(cx))),
            Role::Button,
            "Dismiss",
        )
        .into_any_element();
        let panel = self.drawer(look, hue, "tools", body, cx);
        // Its body is always up, and Esc inside it puts the band away.
        let panel = div().id("tools-keys").on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                this.put_tools_away(cx);
                cx.stop_propagation();
            }
        })).child(panel);
        labelled(self.frame("tools", Some(panel.into_any_element()), self.bar(look, hue, true, trigger, Some(away))), Role::Region, SharedString::from(said)).into_any_element()
    }

    fn put_tools_away(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let missing = self.record(&id, cx).and_then(|record| record.tools_missing.as_ref().map(|tools| tools.missing.clone()));
        if let (Some(page), Some(missing)) = (self.pages.get_mut(&id), missing) {
            page.bands.tools_away = Some(missing);
        }
        cx.notify();
    }

    /// Put one piece right: the agent restarts with Fleet's copy of the mod, or its door reconnects.
    fn mend(&mut self, piece: ToolMissing, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let session_id = SessionId::carried(id.to_string());
        let (route, body): (&'static str, _) = match piece {
            ToolMissing::Mod => ("/sessions/tools/mend", encoded(&MendSessionTools { session_id })),
            ToolMissing::Door => ("/sessions/mcp/act", encoded(&ActOnSessionMcp { session_id, server: "armada-fleet".into(), act: McpServerAct::Reconnect })),
            ToolMissing::TaskTools => return,
        };
        let name = piece_name(piece);
        let Some(page) = self.pages.get_mut(&id) else { return };
        if !page.bands.mending.insert(name) {
            return;
        }
        page.bands.said.remove(&format!("tools:{name}"));
        cx.notify();
        self.post_then(
            route,
            body,
            move |this, outcome, cx| {
                if let Some(page) = this.pages.get_mut(&id) {
                    page.bands.mending.remove(name);
                    if let Some(why) = outcome {
                        page.bands.said.insert(format!("tools:{name}"), why);
                    }
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    // ---- Nothing left to do -------------------------------------------------------------------------

    /// The Done card's keys: `⌥↵` ends the Job, `⌥X` keeps it going.
    fn done_card(&self, id: &SharedString) -> Card {
        let (end_id, keep_id) = (id.clone(), id.clone());
        let end: Run = Rc::new(move |this, window, cx| this.end_job(&end_id, window, cx));
        let keep: Run = Rc::new(move |this, window, cx| this.keep_going(&keep_id, window, cx));
        Card {
            at: usize::MAX / 2,
            kind: "Done".into(),
            title: "Nothing left to do".into(),
            yes: Some(("End job".into(), end)),
            change: None,
            send_back: None,
            no: Some(("Keep going".into(), keep)),
        }
    }

    /// End the Job. The list of what it still holds in flight is the head's End job, which asks first.
    fn end_job(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let body = encoded(&CloseSession { session_id: SessionId::carried(id.to_string()) });
        let job = id.clone();
        self.post_then(
            "/sessions/close",
            body,
            move |this, outcome, cx| {
                if let (Some(page), Some(why)) = (this.pages.get_mut(&job), outcome) {
                    page.bands.said.insert(DONE.into(), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    /// Put the ending away: Fleet's `done` item goes too, and the caret goes to the box.
    fn keep_going(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let since = self.record(id, cx).and_then(|record| record.done_since.as_ref().map(|since| since.as_str().to_string()));
        let Some(since) = since else { return };
        if let Some(page) = self.pages.get_mut(id) {
            page.bands.kept = Some(since.clone());
        }
        let body = encoded(&DismissWaiting { session_id: SessionId::carried(id.to_string()), item_id: format!("done:{since}"), reason: None, misread: false });
        let job = id.clone();
        self.post_then(
            "/sessions/waiting/dismiss",
            body,
            move |this, outcome, cx| {
                if let (Some(page), Some(why)) = (this.pages.get_mut(&job), outcome) {
                    page.bands.said.insert(DONE.into(), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn end_band(&self, id: &SharedString, record: &ipc::SessionRecord, look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STEP_ADVANCED);
        let open = self.pages.get(id).is_some_and(|page| page.bands.end_open.unwrap_or(true));
        let target = self.is_target(DONE);
        let summary = record.done_summary.clone().filter(|text| !text.is_empty());
        let made: Vec<AnyElement> = record
            .attachments
            .iter()
            .filter(|one| one.kind == "pr")
            .map(|one| {
                let name = match one.detail.get("title") {
                    Some(title) => format!("#{} {title}", one.target),
                    None => format!("#{}", one.target),
                };
                labelled(
                    div()
                        .id(SharedString::from(format!("made:{}", one.target)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(glyph(look, "git-pull-request", Size::Small, look.color(token::FG_MUTED)))
                        .child(div().min_w_0().truncate().child(SharedString::from(name.clone()))),
                    Role::Row,
                    name,
                )
                .into_any_element()
            })
            .collect();
        let (end_id, keep_id) = (id.clone(), id.clone());
        let acts = div()
            .flex()
            .flex_none()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                button(look, "done-end", "End job", Tone::Secondary, false, target.then_some("\u{2325}\u{21b5}"), true)
                    .on_click(cx.listener(move |this, _, window, cx| this.end_job(&end_id, window, cx))),
                Role::Button,
                "End job",
            ))
            .child(labelled(
                button(look, "done-keep", "Keep going", Tone::Ghost, false, target.then_some("\u{2325}X"), true)
                    .on_click(cx.listener(move |this, _, window, cx| this.keep_going(&keep_id, window, cx))),
                Role::Button,
                "Keep going",
            ));
        let said = self.pages.get(id).and_then(|page| page.bands.said.get(DONE).cloned());
        let aim = DONE.to_string();
        let body = labelled(
            div()
                .id("done-body")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    this.aim(&aim);
                    cx.notify();
                }))
                .children(summary.map(|text| div().px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(SharedString::from(text))))
                .children((!made.is_empty()).then(|| {
                    labelled(div().id("done-made").flex().flex_col().gap(look.length(token::SPACE_2)).children(made), Role::Group, "Pull requests")
                }))
                .child(acts)
                .children(said.map(|why| refusal(look, DONE, why))),
            Role::Group,
            "Nothing left to do",
        )
        .into_any_element();
        let trigger = self.trigger(id, look, "end-trigger", "Nothing left to do", "check", Vec::new(), open, cx, |this, _, cx| {
            if let Some(id) = this.shown.clone() {
                if let Some(page) = this.pages.get_mut(&id) {
                    page.bands.end_open = Some(!page.bands.end_open.unwrap_or(true));
                }
            }
            cx.notify();
        });
        let panel = open.then(|| self.drawer(look, hue, "end", body, cx));
        labelled(self.frame("end", panel, self.bar(look, hue, open, trigger, None)), Role::Region, "Nothing left to do").into_any_element()
    }

    // ---- Waiting ------------------------------------------------------------------------------------

    fn waiting_band(&self, id: &SharedString, held: &[HeldOpenBy], look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_HANDED_IN);
        let open = self.pages.get(id).is_some_and(|page| page.bands.wait_open);
        let waits: Vec<(WaitKind, String)> = held.iter().map(wait_of).collect();
        let glyphs: Vec<AnyElement> = waits
            .iter()
            .enumerate()
            .map(|(at, (kind, said))| {
                labelled(
                    div()
                        .id(SharedString::from(format!("wait-glyph:{at}")))
                        .child(glyph(look, kind.glyph(), Size::Small, look.color(token::FG_INVERSE)))
                        .tooltip(tip(said.clone())),
                    Role::Image,
                    said.clone(),
                )
                .into_any_element()
            })
            .collect();
        let rows: Vec<AnyElement> = waits
            .iter()
            .enumerate()
            .map(|(at, (kind, said))| {
                let away = if *kind == WaitKind::Watch { "Stop the watch" } else { "Stop waiting on it" };
                let name = format!("{away}: {said}");
                let stop = labelled(
                    div()
                        .id(SharedString::from(format!("wait-stop:{at}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .self_stretch()
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .text_color(look.color(token::FG_MUTED))
                        .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                        .tooltip(tip(away))
                        .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| this.stop_waiting(at, window, cx))),
                    Role::Button,
                    name,
                );
                labelled(
                    div()
                        .id(SharedString::from(format!("wait:{at}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .min_w_0()
                                .child(glyph(look, kind.glyph(), Size::Small, look.color(token::FG_MUTED)))
                                .child(div().min_w_0().truncate().child(SharedString::from(said.clone()))),
                        )
                        .child(stop),
                    Role::Row,
                    said.clone(),
                )
                .into_any_element()
            })
            .collect();
        let said = self.pages.get(id).and_then(|page| page.bands.said.get("waiting").cloned());
        let body = labelled(div().id("waiting-on").flex().flex_col().gap(look.length(token::SPACE_2)).children(rows).children(said.map(|why| refusal(look, "waiting", why))), Role::List, "Waiting on").into_any_element();
        let trigger = self.trigger(id, look, "waiting-trigger", "Waiting", "radar", glyphs, open, cx, |this, _, cx| {
            if let Some(id) = this.shown.clone() {
                if let Some(page) = this.pages.get_mut(&id) {
                    page.bands.wait_open = !page.bands.wait_open;
                }
            }
            cx.notify();
        });
        let panel = open.then(|| self.drawer(look, hue, "waiting", body, cx));
        labelled(self.frame("waiting", panel, self.bar(look, hue, open, trigger, None)), Role::Region, "Waiting").into_any_element()
    }

    /// A wait's cross: a watch is stopped as its own Stop does, and a pull request only stops holding the Job open.
    fn stop_waiting(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(held) = self.record(&id, cx).and_then(|record| record.held_open_by.get(at).cloned()) else { return };
        let session_id = SessionId::carried(id.to_string());
        let job = id.clone();
        let failed = move |this: &mut JobView, outcome: Option<SharedString>, cx: &mut Context<JobView>| {
            if let (Some(page), Some(why)) = (this.pages.get_mut(&job), outcome) {
                page.bands.said.insert("waiting".into(), why);
            }
            cx.notify();
        };
        match held {
            HeldOpenBy::PullRequest { number, .. } => {
                let body = encoded(&DismissWait { session_id, pull_request: number });
                self.post_then("/watches/dismiss_wait", body, failed, window, cx);
            }
            HeldOpenBy::Watch { name } | HeldOpenBy::Job { name } => self.stop_watch_named(session_id, name, failed, window, cx),
        }
    }

    /// The Job's live watch by that name is read, then stopped.
    fn stop_watch_named(
        &mut self,
        session_id: SessionId,
        name: String,
        then: impl FnOnce(&mut JobView, Option<SharedString>, &mut Context<JobView>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(fleet) = self.fleet.clone() else { return };
        let Some(port) = fleet.read(cx).held().port() else {
            then(self, Some("Fleet is not connected".into()), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let path = format!("/watches?session_id={}", session_id.as_str());
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(format!("Fleet answered {status}"));
                    }
                    let list: WatchList = ipc::decode("WatchList", &bytes).map_err(|why| format!("{why:?}"))?;
                    let Some(watch) = list.watches.iter().find(|watch| watch.live && watch.name == name) else { return Ok(()) };
                    let body = ipc::encode(&StopWatch { session_id: Some(session_id.clone()), watch: watch.id.clone() }).map_err(|why| format!("{why:?}"))?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/watches/stop", body))?;
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let _ = this.update(cx, |this, cx| then(this, outcome.err().map(SharedString::from), cx));
        })
        .detach();
    }

    /// An act sent off the main thread; `then` runs with Fleet's reason when it was refused, and with none when it was taken.
    fn post_then(
        &mut self,
        route: &'static str,
        body: Result<String, String>,
        then: impl FnOnce(&mut JobView, Option<SharedString>, &mut Context<JobView>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(fleet) = self.fleet.clone() else { return };
        let Some(port) = fleet.read(cx).held().port() else {
            then(self, Some("Fleet is not connected".into()), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, route, body))?;
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let _ = this.update(cx, |this, cx| then(this, outcome.err().map(SharedString::from), cx));
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_is_worded_from_what_holds_the_job_open() {
        let said = |held: HeldOpenBy| wait_of(&held).1;
        assert_eq!(said(HeldOpenBy::Watch { name: "ci".into() }), "Held open by ci");
        assert_eq!(said(HeldOpenBy::Job { name: "Audit".into() }), "Waiting on Audit");
        assert_eq!(said(HeldOpenBy::PullRequest { number: 7, on: PullRequestWait::Queue }), "#7 in the merge queue");
        assert_eq!(said(HeldOpenBy::PullRequest { number: 7, on: PullRequestWait::AutoMerge }), "#7 to auto-merge");
        assert_eq!(said(HeldOpenBy::PullRequest { number: 7, on: PullRequestWait::Checks { auto_merge: true } }), "#7 to auto-merge once Checks pass");
        assert_eq!(said(HeldOpenBy::PullRequest { number: 7, on: PullRequestWait::Checks { auto_merge: false } }), "Checks running on #7");
    }

    #[test]
    fn the_tools_band_stays_away_until_a_new_piece_is_missing() {
        use ToolMissing::*;
        assert!(tools_shown(&[Mod], None));
        assert!(!tools_shown(&[Mod], Some(&[Mod])));
        assert!(!tools_shown(&[Mod], Some(&[Mod, Door])));
        assert!(tools_shown(&[Mod, Door], Some(&[Mod])));
        assert!(!tools_shown(&[], None));
    }

    #[test]
    fn the_tools_line_names_the_harness_then_each_piece() {
        let tools = ToolsMissing { harness: "Claude Code".into(), update: "claude update".into(), harness_version: "2.1".into(), missing: vec![ToolMissing::Mod, ToolMissing::TaskTools] };
        assert_eq!(tools_said(&tools), "Claude Code 2.1. No armada mod. No task tools.");
        let bare = ToolsMissing { harness_version: String::new(), ..tools };
        assert_eq!(tools_said(&bare), "No armada mod. No task tools.");
    }
}
