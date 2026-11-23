//! The librarian's card over the Features list: where a project's librarian stands, and the press that runs it.
//!
//! What it says is worked out here from Fleet's `LibrarianState` and where a press is (`card`), and nothing there touches the
//! screen. The states Fleet can say today are drawn: never read, current, behind, reading and refused. The ones that wait on
//! Fleet's later parts (features waiting for him, a doc and the code that disagree, the night's share spent, a source it cannot
//! reach) are not drawn, and the card is a column of lines, so each takes a line of its own where it comes.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, AnyElement, App, Context, FocusHandle, FontWeight, Role, SharedString, Task, Window};
use ipc::LibrarianState;

use super::dossier::{button, tint};
use super::steps::{render_steps, Run};
use super::{FeaturesScreen, Load};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::{refusal, ring_colour, tip};
use crate::settings::wire::Refused;
use crate::titlebar::parts::{glyph, spinner};

/// Where a press stands, kept apart from what Fleet says so a read of the library does not undo it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Press {
    #[default]
    Idle,
    /// The act is in flight.
    Working,
    /// Fleet refused it, in its own words.
    Failed(String),
}

/// Whether the card is open, by the project it was opened for, and the two buttons the keys reach.
pub struct Shell {
    pub open: Option<String>,
    pub line: FocusHandle,
    pub close: FocusHandle,
}

impl Shell {
    pub fn new(cx: &App) -> Self {
        Shell { open: None, line: cx.focus_handle(), close: cx.focus_handle() }
    }
}

/// What the card says.
#[derive(Clone, Debug, PartialEq)]
pub enum Card {
    Never,
    Current { through: Option<u64>, when: String },
    Behind { since: Option<u64>, merged: Vec<u64> },
    Working,
    Failed(String),
}

/// How long ago `at` was: `just now`, `4 min ago`, `2 h ago`, `3 d ago`. Empty where `at` is no instant.
pub fn ago(at: &str, now: DateTime<Utc>) -> String {
    let Ok(then) = DateTime::parse_from_rfc3339(at) else { return String::new() };
    let minutes = (now - then.with_timezone(&Utc)).num_minutes().max(0);
    match minutes {
        0 => "just now".into(),
        1..=59 => format!("{minutes} min ago"),
        60..=1439 => format!("{} h ago", minutes / 60),
        _ => format!("{} d ago", minutes / 1440),
    }
}

pub fn card(state: &LibrarianState, press: &Press, run: Option<&Run>, now: DateTime<Utc>) -> Card {
    // A run going on, started here or elsewhere, is what the card shows, even over a refusal of a second one.
    if *press == Press::Working || run.is_some_and(Run::going) {
        return Card::Working;
    }
    if let Press::Failed(why) = press {
        return Card::Failed(why.clone());
    }
    let Some(ran) = &state.last_run else { return Card::Never };
    if state.merged_since.is_empty() {
        Card::Current { through: state.read_through, when: ago(ran.as_str(), now) }
    } else {
        Card::Behind { since: state.read_through, merged: state.merged_since.clone() }
    }
}

impl Card {
    /// The line at the top of the card.
    pub fn said(&self) -> String {
        match self {
            Card::Never => "Not read yet".into(),
            Card::Current { through, when } => {
                let head = through.map_or_else(|| "Read".to_string(), |n| format!("Read through #{n}"));
                if when.is_empty() {
                    head
                } else {
                    format!("{head}, {when}")
                }
            }
            Card::Behind { since: Some(n), .. } => format!("Not read since #{n}"),
            Card::Behind { since: None, .. } => "Merged since the last read".into(),
            Card::Working => "Catching up".into(),
            Card::Failed(why) => why.clone(),
        }
    }

    /// Whether the card lists what a press will do under its line: where a press would run, and nothing has run yet.
    pub fn lists_plan(&self) -> bool {
        matches!(self, Card::Never | Card::Behind { .. })
    }

    /// The line, with the lead-in to the steps listed under it where they are the ones a press will take.
    pub fn lead(&self, listing_plan: bool) -> String {
        if listing_plan {
            format!("{}. Catch up will:", self.said())
        } else {
            self.said()
        }
    }

    /// The pull requests merged since the last read, oldest first, as one run of numbers.
    pub fn merged_said(&self) -> Option<String> {
        match self {
            Card::Behind { merged, .. } => Some(merged.iter().map(|n| format!("#{n}")).collect::<Vec<_>>().join(" ")),
            _ => None,
        }
    }

    /// The button the card carries: its words, whether it is the primary one, and whether it is off. Current has none.
    pub fn button(&self) -> Option<(&'static str, bool, bool)> {
        match self {
            Card::Never => Some(("Catch up", false, false)),
            Card::Behind { .. } => Some(("Catch up", true, false)),
            Card::Working => Some(("Stop", false, false)),
            Card::Failed(_) => Some(("Retry", false, false)),
            Card::Current { .. } => None,
        }
    }

    /// The closed line's button: its words, and whether it is the primary one. Current offers Update, and a run going offers its steps.
    pub fn line_button(&self) -> (&'static str, bool) {
        match self {
            Card::Never | Card::Failed(_) => ("Catch up", false),
            Card::Behind { .. } => ("Catch up", true),
            Card::Current { .. } => ("Update", false),
            Card::Working => ("Steps", false),
        }
    }

    /// Whether a press runs the librarian: the button being there and on, and not the one that stops it.
    pub fn presses(&self) -> bool {
        !self.stops() && self.button().is_some_and(|(_, _, off)| !off)
    }

    /// Whether the button stops a run going on now.
    pub fn stops(&self) -> bool {
        matches!(self, Card::Working)
    }

    /// The hue of the edge band and the glyph, the glyph's name and what its tooltip says.
    fn mark(&self) -> (Token, &'static str, &'static str) {
        match self {
            Card::Never => (token::FG_SUBTLE, "circle-dashed", "The librarian has not read this project"),
            Card::Current { .. } => (token::STATUS_COMPLETED_SUCCESS, "circle-check", "Read through the last merged pull request"),
            Card::Behind { .. } => (token::STATUS_AWAITING_REVIEW, "clock-alert", "Pull requests merged since the last read"),
            Card::Working => (token::STATUS_RUNNING, "loader-circle", "Catching up"),
            Card::Failed(_) => (token::STATUS_COMPLETED_FAILED, "circle-alert", "Fleet refused the run"),
        }
    }
}

impl FeaturesScreen {
    /// The card for the library on screen, where one is.
    pub(super) fn librarian_card(&self) -> Option<Card> {
        match &self.library {
            Load::Read(library) => Some(card(&library.librarian, &self.press, self.run.as_ref(), Utc::now())),
            _ => None,
        }
    }

    /// Whether the card is open: a press opened it for this project, or Fleet refused the run, which stays until he closes it.
    fn card_open(&self, card: &Card) -> bool {
        matches!(card, Card::Failed(_)) || (self.library_project.is_some() && self.lib_card.open == self.library_project)
    }

    /// Whether the card shows its steps now.
    #[cfg(test)]
    pub(super) fn librarian_open(&self) -> bool {
        self.librarian_card().is_some_and(|card| self.card_open(&card))
    }

    /// The buttons the keys reach on the card as it stands: Catch up on the closed line, Close once the run has ended.
    fn card_stops(&self, card: &Card) -> Vec<FocusHandle> {
        match (self.card_open(card), card.stops()) {
            (false, _) => vec![self.lib_card.line.clone()],
            (true, false) => vec![self.lib_card.close.clone()],
            (true, true) => Vec::new(),
        }
    }

    /// The card over the list (`.lib`): the closed line until a press opens it, and then the card itself.
    pub(super) fn render_librarian(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let card = self.librarian_card()?;
        let (hue, name, why) = card.mark();
        let ink = look.color(hue);
        let size = look.length(token::SPACE_4);
        let mark = match card {
            Card::Working => spinner("features-librarian-spin", size, ink, cx).into_any_element(),
            _ => glyph(name, size, ink).into_any_element(),
        };
        let mark = labelled(div().id("features-librarian-state").flex_none().tooltip(tip(why)).child(mark), Role::Image, why);
        let title = div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Librarian");
        let ground = tint(look, look.color(token::BG_SUNKEN), hue, 0.06);
        let edge = tint(look, look.color(token::BORDER_SUBTLE), hue, 0.55);
        let shell = div()
            .id("features-librarian")
            .flex_none()
            .mx(look.length(token::SPACE_3))
            .mb(look.length(token::SPACE_3))
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .overflow_hidden()
            .on_action(cx.listener(|this, act: &Act, window, cx| {
                if !(act.is("features_open") && this.press_card_stop(window, cx)) {
                    cx.propagate();
                }
            }));
        let shell = if self.card_open(&card) {
            // At most a fifth of the list's column, which spans the window: the title row stays, and what is under it scrolls.
            let cap = gpui::relative(0.2);
            let press = card.button().map(|(words, primary, off)| {
                let pill = button(look, "features-librarian-press", words, primary, false, off, None);
                let pill = if off { pill } else { pill.on_click(cx.listener(|this, _, _, cx| this.catch_up(cx))) };
                labelled(pill, Role::Button, words)
            });
            let close = (!card.stops()).then(|| self.close_button(look, cx));
            let head = div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_2))
                .child(mark)
                .child(title)
                .child(div().flex_1())
                .children(press)
                .children(close);
            let rows = self.step_rows(card.lists_plan());
            let listing_plan = card.lists_plan() && self.run.is_none() && !rows.is_empty();
            let said = card.lead(listing_plan);
            let line = match &card {
                Card::Failed(_) => refusal(look, said.clone()),
                _ => div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())),
            };
            let line = labelled(div().id("features-librarian-said").child(line), Role::Label, said);
            let merged = card.merged_said().filter(|_| !listing_plan).map(|numbers| {
                labelled(
                    div()
                        .id("features-librarian-merged")
                        .max_h(look.length(token::LEADING_2XS) * 2.)
                        .overflow_hidden()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(SharedString::from(numbers.clone())),
                    Role::Label,
                    format!("Merged since: {numbers}"),
                )
            });
            let steps = (!rows.is_empty()).then(|| render_steps(look, rows, cx));
            let body = div()
                .id("features-librarian-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_3))
                .child(line)
                .children(merged)
                .children(steps);
            shell.max_h(cap).child(head).child(body)
        } else {
            let said = if matches!(card, Card::Working) { "Reading".to_string() } else { card.said() };
            let (words, primary) = card.line_button();
            let press = button(look, "features-librarian-open", words, primary, false, false, Some(&self.lib_card.line)).on_click(cx.listener(|this, _, window, cx| this.press_line(window, cx)));
            let status = labelled(
                div().id("features-librarian-said").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())),
                Role::Label,
                said,
            );
            let row = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .child(mark)
                .child(div().flex_1().min_w_0().flex().flex_col().child(title).child(status))
                .child(labelled(press, Role::Button, words));
            shell.child(row)
        };
        Some(labelled(shell, Role::Region, "Librarian").into_any_element())
    }

    /// The glyph button at the card's top right that shuts it, once the run has ended.
    fn close_button(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let hover = look.color(token::BG_HOVER);
        let button = div()
            .id("features-librarian-close")
            .track_focus(&self.lib_card.close)
            .tab_stop(true)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::H_CONTROL_SM))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip("Close"))
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(|this, _, window, cx| this.close_card(window, cx)))
            .child(glyph("x", look.length(token::SPACE_4), look.color(token::FG_MUTED)));
        labelled(button, Role::Button, "Close").into_any_element()
    }

    /// The closed line's button: Catch up or Update runs the librarian and opens the card, and where a run is going it opens the card
    /// to show the steps, with no second run.
    fn press_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.lib_card.line.is_focused(window) {
            window.focus(&self.list_focus, cx);
        }
        match self.librarian_card() {
            Some(card) if card.stops() => {
                self.lib_card.open = self.library_project.clone();
                cx.notify();
            }
            _ => self.catch_up(cx),
        }
    }

    /// Close, or Esc once the run has ended: back to the closed line. A refusal he has read is let go.
    fn close_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.librarian_card().is_none_or(|card| card.stops()) {
            return;
        }
        if self.lib_card.close.is_focused(window) {
            window.focus(&self.list_focus, cx);
        }
        self.lib_card.open = None;
        if matches!(self.press, Press::Failed(_)) {
            self.press = Press::Idle;
        }
        cx.notify();
    }

    /// Esc where the keys are on the card or the list: the open card, once its run has ended, shuts.
    pub(super) fn escape_librarian(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(card) = self.librarian_card() else { return false };
        if !self.card_open(&card) || card.stops() {
            return false;
        }
        let on_card = self.card_stops(&card).iter().any(|stop| stop.is_focused(window));
        if !(on_card || self.list_focus.is_focused(window)) {
            return false;
        }
        self.close_card(window, cx);
        true
    }

    /// Enter on the card's button that has the keys.
    fn press_card_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.lib_card.line.is_focused(window) {
            self.press_line(window, cx);
        } else if self.lib_card.close.is_focused(window) {
            self.close_card(window, cx);
        } else {
            return false;
        }
        true
    }

    /// Tab and Shift+Tab on the card's button: the keys go back to the list. Shift+Tab from the list reaches the button, as the card
    /// is over it.
    pub(super) fn tab_librarian(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(card) = self.librarian_card() else { return false };
        let stops = self.card_stops(&card);
        if stops.iter().any(|stop| stop.is_focused(window)) {
            window.focus(&self.list_focus, cx);
            return true;
        }
        match (back && self.list_focus.is_focused(window), stops.first()) {
            (true, Some(stop)) => {
                window.focus(stop, cx);
                true
            }
            _ => false,
        }
    }
}

/// How long each step of the mock's run takes, so a walk and a test can see it going.
const MOCK_STEP: std::time::Duration = std::time::Duration::from_millis(200);

impl FeaturesScreen {
    /// Catch up, Update, Retry or `c`: run the project's librarian now and open its card, or stop the run going on now. Nothing happens
    /// on the open card where it has read through the last merge.
    pub(super) fn catch_up(&mut self, cx: &mut Context<Self>) {
        let shut = self.librarian_card().is_some_and(|card| !self.card_open(&card));
        match self.librarian_card() {
            Some(card) if card.stops() => return self.stop(cx),
            Some(card) if card.presses() => {}
            // The closed line offers Update where the library is current.
            Some(Card::Current { .. }) if shut => {}
            _ => return,
        }
        let Some(project) = self.library_project.clone() else { return };
        self.lib_card.open = Some(project.clone());
        let ask = match &self.source {
            super::Source::Live(fleet) => match fleet.read(cx).held().port() {
                Some(port) => super::wire::run_librarian(port, &project, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            },
            super::Source::Mock => self.mock_run(&project, cx),
        };
        self.press = Press::Working;
        self.stopping = false;
        self.ran = Some(cx.spawn(async move |this, cx| {
            let answer = ask.await;
            let _ = this.update(cx, |this, cx| this.ran_took(&project, answer, cx));
        }));
        cx.notify();
    }

    /// The mock's run: each step of the plan starts, takes a moment and ends, told through the same fold a live step is.
    fn mock_run(&mut self, project: &str, cx: &mut Context<Self>) -> Task<Result<ipc::LibrarianRan, Refused>> {
        let refused = self.mock_refusal.clone();
        let (through, plan) = match &self.library {
            Load::Read(library) => (library.librarian.merged_since.last().copied().or(library.librarian.read_through), library.librarian.steps.clone()),
            _ => (None, Vec::new()),
        };
        let project = project.to_string();
        let run = ipc::Instant::carried(Utc::now().to_rfc3339());
        cx.spawn(async move |this, cx| {
            let wait = cx.background_executor().clone();
            if let Some(said) = refused {
                wait.timer(MOCK_STEP).await;
                return Err(Refused::said(said));
            }
            let tell = |step: &str, state, said: &str| ipc::LibrarianStepMoved { project: project.clone(), run: run.clone(), step: step.to_string(), state, said: said.to_string() };
            let mut stopped = false;
            for step in &plan {
                let halt = this.update(cx, |this, _| this.stopping).unwrap_or(false);
                let moved = if halt || stopped {
                    stopped = true;
                    tell(&step.id, ipc::LibrarianStepState::Skipped, "Stopped")
                } else {
                    let _ = this.update(cx, |this, cx| this.step_moved(&tell(&step.id, ipc::LibrarianStepState::Running, ""), cx));
                    wait.timer(MOCK_STEP).await;
                    tell(&step.id, ipc::LibrarianStepState::Done, &step.detail)
                };
                let _ = this.update(cx, |this, cx| this.step_moved(&moved, cx));
            }
            Ok(ipc::LibrarianRan { project, ran_at: run, ran: Vec::new(), read_through: through, skipped_for_share: Vec::new(), stopped })
        })
    }

    /// Stop, or `c` while a run is going: it stops before its next step. Asked once.
    fn stop(&mut self, cx: &mut Context<Self>) {
        if self.stopping {
            return;
        }
        let Some(project) = self.library_project.clone() else { return };
        self.stopping = true;
        if let super::Source::Live(fleet) = &self.source {
            let ask = match fleet.read(cx).held().port() {
                Some(port) => super::wire::stop_librarian(port, &project, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            };
            self.stopped = Some(cx.spawn(async move |this, cx| {
                let answer = ask.await;
                // Fleet refused, or had none going: Stop is there to press again.
                let _ = this.update(cx, |this, cx| {
                    if answer.as_ref().map_or(true, |said| !said.stopping) {
                        this.stopping = false;
                    }
                    cx.notify();
                });
            }));
        }
        cx.notify();
    }

    /// The press came back: what Fleet said it read through is the card's now, with nothing behind it, and Fleet is read again for the truth.
    fn ran_took(&mut self, project: &str, answer: Result<ipc::LibrarianRan, Refused>, cx: &mut Context<Self>) {
        if self.library_project.as_deref() != Some(project) {
            return;
        }
        self.stopping = false;
        match answer {
            Ok(ran) => {
                if let Some(run) = self.run.as_mut().filter(|run| run.project == project && run.run == ran.ran_at) {
                    run.close(ran.stopped);
                }
                // A run that was stopped did not read through, so what the card said before it stands until Fleet is read.
                if let (Load::Read(library), false) = (&mut self.library, ran.stopped) {
                    let state = &mut library.librarian;
                    state.read_through = ran.read_through.or(state.read_through);
                    state.last_run = Some(ran.ran_at);
                    state.merged_since.clear();
                }
                self.press = Press::Idle;
                if matches!(self.source, super::Source::Live(_)) && self.entered {
                    self.read(cx);
                }
            }
            Err(why) => self.press = Press::Failed(why.said),
        }
        cx.notify();
    }
}
