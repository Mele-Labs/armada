//! The For you tab, in the look the owner approved on 15 Nov 2026 less its stripe, which he ruled out on 19 Nov: three sections,
//! each a card per row, tinted and outlined in the section's hue. Waiting on you lists what the Job owes him, numbered, in the order Fleet gives; Being built the
//! Jobs it started, drawn in the state their row in the Jobs list has; Landing on green its own pull requests still open.
//! Fleet's `get_for_you` is the whole list, read again where `session.for_you` says it moved. **Nothing here sorts it.**
//!
//! A section with no rows is not drawn, and a tab with none in any draws nothing. Up and down move the focus over the
//! rows, Enter opens one, Backspace withdraws a waiting one, and ⌥Up and ⌥Down move it one place, which tells Fleet the
//! whole order of the waiting rows. There is no drag.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Hsla, KeyDownEvent, Role, SharedString, Window};
use ipc::{ForYou, ForYouChanged, ForYouItem, ForYouOpens, OrderForYou, SessionId, SessionStanding, WithdrawForYou};

use super::artifacts_tab::step;
use super::dock::Thing;
use super::keys::Spot;
use super::menus::Pick;
use super::pulls::{pull_of, Pull, PullChecks, PullState};
use super::rows::{eyebrow, glyph, tip, Size};
use super::side::Zone;
use super::wire::Answer;
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// What the view holds of its Jobs' For you.
#[derive(Default)]
pub(super) struct ForYouTab {
    /// Each Job's list as Fleet last read it. None until the first answer, so the tab draws its shapes.
    read: HashMap<String, Option<ForYou>>,
    /// What Fleet refused a row's withdrawal or move for, by Job and row, until a read says the list again.
    refused: HashMap<(String, String), SharedString>,
    /// The Job whose first row takes the keys once its list arrives, where the tab was pressed before it had.
    want_focus: Option<String>,
    /// What the mock Fleet answers for its Job, taken when the view was made.
    mock: Option<ForYou>,
}

impl ForYouTab {
    pub(super) fn mock(mock: Option<ForYou>) -> Self {
        ForYouTab { mock, ..Default::default() }
    }
}

/// One of the tab's three sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Waiting,
    Building,
    Landing,
}

impl Part {
    fn word(self) -> &'static str {
        match self {
            Part::Waiting => "Waiting on you",
            Part::Building => "Being built",
            Part::Landing => "Landing on green",
        }
    }

    /// The hue of its cards' tint and outline, and its head: the warning hue for what waits on him, the running hue, the success hue.
    fn hue(self) -> Token {
        match self {
            Part::Waiting => token::STATUS_AWAITING_REVIEW,
            Part::Building => token::STATUS_RUNNING,
            Part::Landing => token::STATUS_COMPLETED_SUCCESS,
        }
    }
}

/// A row of the tab, by what Fleet names it with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Rowed {
    Waiting(String),
    Building(String),
    Landing(u64),
}

impl Rowed {
    /// The row's key among the tab's: the focus, the element and the keys' ground.
    pub(super) fn key(&self) -> String {
        match self {
            Rowed::Waiting(id) => format!("w:{id}"),
            Rowed::Building(job) => format!("b:{job}"),
            Rowed::Landing(pr) => format!("l:{pr}"),
        }
    }
}

/// The sections the tab draws, each with its rows in Fleet's order: only those that hold any.
pub(super) fn sections(read: &ForYou) -> Vec<(Part, Vec<Rowed>)> {
    let waiting = read.waiting.iter().map(|one| Rowed::Waiting(one.id.clone())).collect::<Vec<_>>();
    let building = read.building.iter().map(|one| Rowed::Building(one.job.as_str().to_string())).collect::<Vec<_>>();
    let landing = read.landing.iter().map(|one| Rowed::Landing(one.pr)).collect::<Vec<_>>();
    [(Part::Waiting, waiting), (Part::Building, building), (Part::Landing, landing)].into_iter().filter(|(_, rows)| !rows.is_empty()).collect()
}

/// Every row the keys move over, top to bottom.
pub(super) fn rows(read: &ForYou) -> Vec<Rowed> {
    sections(read).into_iter().flat_map(|(_, rows)| rows).collect()
}

/// The waiting rows' ids as they would stand with `id` moved `by` places, or None where it is not waiting or is at the end.
pub(super) fn moved(read: &ForYou, id: &str, by: i32) -> Option<Vec<String>> {
    let mut ids: Vec<String> = read.waiting.iter().map(|one| one.id.clone()).collect();
    let at = ids.iter().position(|held| held == id)?;
    let to = at as i32 + by.signum();
    if to < 0 || to as usize >= ids.len() {
        return None;
    }
    ids.swap(at, to as usize);
    Some(ids)
}

/// The list as Fleet answers `order_for_you`: the waiting rows in `ids`' order, each one placed.
fn ordered(read: &mut ForYou, ids: &[String]) {
    let mut held = std::mem::take(&mut read.waiting);
    held.sort_by_key(|one| ids.iter().position(|id| *id == one.id).unwrap_or(usize::MAX));
    held.iter_mut().for_each(|one| one.placed = true);
    read.waiting = held;
}

/// Whether Needs you's own rows hold the item: it opens a row they list, or it is the question the thread carries.
fn held_by_a_row(item: &ForYouItem, things: &[Thing]) -> bool {
    let ForYouOpens::Waiting { id } = &item.opens else { return false };
    things.iter().any(|thing| match thing {
        Thing::Row(row) => row.id == *id,
        Thing::Ask(_) => id.starts_with("ask:") || id.starts_with("perm:"),
        Thing::Lapsed(_) => false,
        Thing::Sheet(item) => item.id == *id,
    })
}

fn empty(job: &str) -> ForYou {
    ForYou { job: SessionId::carried(job.to_string()), waiting: Vec::new(), building: Vec::new(), landing: Vec::new() }
}

/// The state a Job Fleet read of `standing` draws as in the Jobs list.
fn state_of(standing: SessionStanding) -> ipc::JobState {
    match standing {
        SessionStanding::Working => ipc::JobState::Working,
        SessionStanding::Stuck => ipc::JobState::Stuck,
        SessionStanding::Replied => ipc::JobState::Replied,
        SessionStanding::Stopped => ipc::JobState::Stopped,
        SessionStanding::Done => ipc::JobState::Done,
        SessionStanding::Idle => ipc::JobState::Idle,
    }
}

impl JobView {
    // ---- reading ---------------------------------------------------------------------------

    /// Read the shown Job's list the first time its tab is in front. Called as the view draws, never from a row.
    pub(super) fn sync_for_you(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        // Needs you's Open chip lists these rows too, so the band reads them as the tab does.
        let wanted = self.zones_shown().contains(&Zone::ForYou) || self.band_shown(super::column_bands::Band::Column);
        if self.for_you.read.contains_key(id.as_ref()) || !wanted {
            return;
        }
        self.for_you.read.insert(id.to_string(), None);
        self.read_for_you(id, window, cx);
    }

    fn read_for_you(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let held = self.for_you.mock.clone().filter(|held| held.job.as_str() == id).unwrap_or_else(|| empty(id));
            self.for_you.read.insert(id.to_string(), Some(held));
            self.for_you_arrived(window, cx);
            return;
        }
        let (job, path) = (id.to_string(), format!("/sessions/for_you?session_id={}", super::wire::encoded(id)));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                match answer.ok().and_then(|bytes| ipc::decode::<ForYou>("For you", &bytes).ok()) {
                    Some(read) => {
                        this.for_you.refused.retain(|(held, _), _| *held != job);
                        this.for_you.read.insert(job, Some(read));
                    }
                    // A refusal leaves what the tab held, and draws nothing where it held none.
                    None => {
                        let held = this.for_you.read.entry(job.clone()).or_insert(None);
                        if held.is_none() {
                            *held = Some(empty(&job));
                        }
                    }
                }
                this.for_you_arrived(window, cx);
                cx.notify();
            }),
        );
    }

    /// Fleet says these Jobs' For you moved: each one held is read again.
    pub(super) fn for_you_moved(&mut self, moved: &[ForYouChanged], window: &mut Window, cx: &mut Context<Self>) {
        let mut jobs: Vec<&str> = moved.iter().map(|one| one.session_id.as_str()).filter(|job| self.for_you.read.contains_key(*job)).collect();
        jobs.dedup();
        for job in jobs {
            self.read_for_you(job, window, cx);
        }
    }

    fn for_you_arrived(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.for_you.want_focus.take().is_some() {
            self.focus_for_you(window, cx);
        }
    }

    // ---- the keys ---------------------------------------------------------------------------

    /// The keys go to the first row of the tab in front, as a press on its tab gives them to a body that takes them.
    pub(super) fn focus_for_you(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let first = match self.for_you.read.get(job.as_ref()) {
            Some(Some(read)) => rows(read).first().map(Rowed::key),
            Some(None) => None,
            None => {
                self.for_you.read.insert(job.to_string(), None);
                self.for_you.want_focus = Some(job.to_string());
                self.read_for_you(&job, window, cx);
                return;
            }
        };
        let Some(key) = first else { return };
        let Some(spot) = self.pages.get(&job).map(|page| page.keys.spot(&format!("for-you:{key}"), cx)) else { return };
        window.defer(cx, move |window, cx| window.focus(&spot.focus, cx));
    }

    fn for_you_spot(&self, job: &str, key: &str) -> Option<Spot> {
        self.pages.get(job)?.keys.spot_of(&format!("for-you:{key}"))
    }

    /// The row the keys are on.
    fn for_you_at(&self, job: &str, window: &Window) -> Option<Rowed> {
        let read = self.for_you.read.get(job)?.as_ref()?;
        rows(read).into_iter().find(|row| self.for_you_spot(job, &row.key()).is_some_and(|spot| spot.focus.is_focused(window)))
    }

    /// A key on a row. True where the tab took it.
    fn for_you_key(&mut self, job: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.modified();
        let alt = modifiers.alt && !modifiers.control && !modifiers.platform && !modifiers.shift && !modifiers.function;
        let key = event.keystroke.key.as_str();
        let on = self.for_you_at(job, window);
        match key {
            "down" | "up" if plain => {
                let Some(Some(read)) = self.for_you.read.get(job) else { return false };
                let keys: Vec<String> = rows(read).iter().map(Rowed::key).collect();
                let to = step(&keys, on.as_ref().map(Rowed::key).as_deref(), if key == "down" { 1 } else { -1 });
                if let Some(spot) = to.as_ref().and_then(|to| self.for_you_spot(job, to)) {
                    if to.as_deref() != on.as_ref().map(Rowed::key).as_deref() {
                        crate::ui_sounds::moved();
                    }
                    window.focus(&spot.focus, cx);
                }
                true
            }
            "down" | "up" if alt => {
                if let Some(Rowed::Waiting(id)) = on {
                    self.for_you_move(job, &id, if key == "down" { 1 } else { -1 }, window, cx);
                }
                true
            }
            "enter" if plain => {
                if let Some(on) = on {
                    self.for_you_open(job, &on, window, cx);
                }
                true
            }
            "backspace" | "delete" if plain => {
                if let Some(Rowed::Waiting(id)) = on {
                    self.for_you_withdraw(job, &id, window, cx);
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    // ---- what a press does -------------------------------------------------------------------------

    /// A press on a row, or Enter on it, opens what it names.
    fn for_you_open(&mut self, job: &str, row: &Rowed, window: &mut Window, cx: &mut Context<Self>) {
        match row {
            Rowed::Waiting(id) => {
                let Some(item) = self.for_you_item(job, id).cloned() else { return };
                match item.opens {
                    ForYouOpens::Artifact { id } => self.open_artifact(job, &id, window, cx),
                    ForYouOpens::Walk { url } => self.press_walk(&url, window, cx),
                    // A Job's builds to try open in the review panel, as the Needs you row does.
                    ForYouOpens::Waiting { id } if super::ready_card::is_card(&id) => {
                        self.open_ready_review(job, &id, window, cx);
                    }
                    // An ask the review sheet holds opens there.
                    ForYouOpens::Waiting { id } if self.sheet_item(job, &id, cx).is_some() => self.open_ask(&id, window, cx),
                    ForYouOpens::Waiting { id } => self.set_needs(true, Some(format!("item:{id}")), window, cx),
                    ForYouOpens::Plan => self.add_tab("plan", window, cx),
                }
            }
            Rowed::Building(started) => self.open_job(started.clone(), cx),
            Rowed::Landing(number) => {
                let title = self.for_you.read.get(job).and_then(Option::as_ref).and_then(|read| read.landing.iter().find(|one| one.pr == *number)).map(|one| one.title.clone()).unwrap_or_default();
                let pull = self.record(job, cx).and_then(|record| record.attachments.iter().find_map(|one| pull_of(record, one, None).filter(|pull| pull.number == *number))).unwrap_or_else(|| Pull {
                    number: *number,
                    title,
                    branch: String::new(),
                    address: String::new(),
                    state: PullState::Open,
                    checks: PullChecks::Running,
                    failing: String::new(),
                    auto: false,
                    queued: false,
                    job: job.to_string(),
                    manifest: self.record(job, cx).and_then(|record| record.manifest_id.as_ref().map(|id| id.as_str().to_string())),
                    from: None,
                    base: None,
                    root_base: None,
                    stacked_on: None,
                    merge: ipc::PullRequestMergeState::Unknown,
                });
                self.open_pull(pull, window, cx);
            }
        }
    }

    fn for_you_item(&self, job: &str, id: &str) -> Option<&ForYouItem> {
        self.for_you.read.get(job)?.as_ref()?.waiting.iter().find(|one| one.id == id)
    }

    /// Take a waiting row off For you: it goes at once and the keys go to the row beside it. A refusal puts it back
    /// with Fleet's sentence on it. Where the row is not one, nothing is sent.
    pub(super) fn for_you_withdraw(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<WithdrawForYou> {
        let held = self.for_you.read.get(job)?.as_ref()?;
        if !held.waiting.iter().any(|one| one.id == id) {
            return None;
        }
        let before = held.clone();
        let keys: Vec<String> = rows(held).iter().map(Rowed::key).collect();
        let me = Rowed::Waiting(id.to_string()).key();
        let beside = step(&keys, Some(&me), 1).filter(|to| *to != me).or_else(|| step(&keys, Some(&me), -1).filter(|to| *to != me));
        if let Some(spot) = beside.and_then(|to| self.for_you_spot(job, &to)) {
            window.focus(&spot.focus, cx);
        }
        if let Some(Some(held)) = self.for_you.read.get_mut(job) {
            held.waiting.retain(|one| one.id != id);
        }
        cx.notify();
        let wire = WithdrawForYou { session_id: Some(SessionId::carried(job.to_string())), id: id.to_string() };
        let (owner, row) = (job.to_string(), id.to_string());
        self.post("/sessions/for_you/withdraw".into(), ipc::encode(&wire), window, cx, Box::new(move |this, answer, _, cx| this.for_you_answered(&owner, &row, before, answer, cx)));
        Some(wire)
    }

    /// Move a waiting row one place, which tells Fleet the order of all of them. It moves at once; a refusal puts it back.
    pub(super) fn for_you_move(&mut self, job: &str, id: &str, by: i32, window: &mut Window, cx: &mut Context<Self>) -> Option<OrderForYou> {
        let held = self.for_you.read.get(job)?.as_ref()?;
        let ids = moved(held, id, by)?;
        let before = held.clone();
        if let Some(Some(held)) = self.for_you.read.get_mut(job) {
            ordered(held, &ids);
        }
        cx.notify();
        let wire = OrderForYou { session_id: Some(SessionId::carried(job.to_string())), ids };
        let (owner, row) = (job.to_string(), id.to_string());
        self.post("/sessions/for_you/order".into(), ipc::encode(&wire), window, cx, Box::new(move |this, answer, _, cx| this.for_you_answered(&owner, &row, before, answer, cx)));
        Some(wire)
    }

    /// Fleet's answer to a withdrawal or a move: its list replaces ours, and on a refusal ours goes back as it was.
    fn for_you_answered(&mut self, job: &str, row: &str, before: ForYou, answer: Answer, cx: &mut Context<Self>) {
        match answer {
            Ok(bytes) => {
                self.for_you.refused.retain(|(held, _), _| held != job);
                // On mock data there is no answer: the list as it was changed here stands.
                if let Ok(read) = ipc::decode::<ForYou>("For you", &bytes) {
                    self.for_you.read.insert(job.to_string(), Some(read));
                }
            }
            Err(why) => {
                self.for_you.read.insert(job.to_string(), Some(before));
                self.for_you.refused.insert((job.to_string(), row.to_string()), why.into());
            }
        }
        cx.notify();
    }

    // ---- the tab -------------------------------------------------------------------------

    /// What the tab lists under Waiting on you that Needs you's own rows do not already hold: the items Fleet added (a
    /// document, a plan, a preview, a sketch) and any the agent stated that the rows left out. Needs you's Open chip lists
    /// these after its rows, so it holds everything the tab does.
    pub(super) fn for_you_beyond_rows(&self, id: &str, cx: &gpui::App) -> Vec<&ForYouItem> {
        let Some(read) = self.for_you.read.get(id).and_then(Option::as_ref) else { return Vec::new() };
        let (things, _) = self.things(id, cx);
        read.waiting.iter().filter(|item| !held_by_a_row(item, &things)).collect()
    }

    /// Those rows as Open draws them: the title, and Review, which presses as the tab's own does.
    pub(super) fn for_you_beyond_row_views(&self, job: &SharedString, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        self.for_you_beyond_rows(job, cx)
            .into_iter()
            .map(|item| {
                let (press, review) = ((job.to_string(), Rowed::Waiting(item.id.clone())), (job.to_string(), Rowed::Waiting(item.id.clone())));
                let button = labelled(
                    div()
                        .id(SharedString::from(format!("needs-for-you-review:{}", item.id)))
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(look.color(token::ACCENT))
                        .text_color(look.color(token::TEXT_ON_ACCENT))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .cursor_pointer()
                        .tab_index(0)
                        .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                        .focus_visible(|style| style.border_1().border_color(look.color(token::FG_DEFAULT)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.for_you_open(&review.0, &review.1, window, cx);
                        }))
                        .child("Review"),
                    Role::Button,
                    format!("Review {}", item.title),
                );
                labelled(
                    div()
                        .id(SharedString::from(format!("needs-for-you:{}", item.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .min_h(look.length(token::H_ROW))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| this.for_you_open(&press.0, &press.1, window, cx)))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(item.title.clone())))
                        .child(button),
                    Role::ListItem,
                    item.title.clone(),
                )
                .into_any_element()
            })
            .collect()
    }

    /// The tab: its sections, or its shapes while Fleet has not answered. None where there is nothing to say.
    pub(super) fn pane_for_you(&self, id: &SharedString, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let Some(read) = self.for_you.read.get(id.as_ref()) else { return None };
        let Some(read) = read.as_ref() else { return Some(self.for_you_shapes(look)) };
        let parts = sections(read);
        if parts.is_empty() {
            return None;
        }
        let job = id.to_string();
        let mut number = 0;
        let sections = parts.into_iter().map(|(part, rows)| {
            let hue = look.color(part.hue());
            let cards: Vec<AnyElement> = rows
                .iter()
                .filter_map(|row| {
                    let spot = page.keys.spot(&format!("for-you:{}", row.key()), cx);
                    let on = spot.focus.is_focused(window);
                    match row {
                        Rowed::Waiting(wid) => {
                            number += 1;
                            read.waiting.iter().find(|one| one.id == *wid).map(|item| self.waiting_row(&job, item, number, spot, on, hue, look, cx))
                        }
                        Rowed::Building(started) => read.building.iter().find(|one| one.job.as_str() == started).map(|build| self.building_row(&job, build, spot, on, hue, look, cx)),
                        Rowed::Landing(pr) => read.landing.iter().find(|one| one.pr == *pr).map(|landing| self.landing_row(&job, landing, spot, on, hue, look, cx)),
                    }
                })
                .collect();
            labelled(
                div()
                    .id(SharedString::from(format!("for-you-part:{}", part.word())))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(eyebrow(look, part.word()).id(SharedString::from(format!("for-you-head:{}", part.word()))).px(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH)).text_color(hue), Role::Heading, part.word()))
                    .children(cards),
                Role::Group,
                part.word(),
            )
            .into_any_element()
        });
        let keyed = job.clone();
        Some(
            div()
                .id("pane-for-you")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if this.for_you_key(&keyed, event, window, cx) {
                        cx.stop_propagation();
                    }
                }))
                .children(sections)
                .into_any_element(),
        )
    }

    /// The card every row is: a tint of its section's hue, with a full hairline outline in the hue and no stripe down one side.
    fn for_you_card(&self, key: &str, spot: &Spot, on: bool, hue: Hsla, look: &Look) -> gpui::Stateful<gpui::Div> {
        let edge = look.length(token::BORDER_WIDTH);
        let fill = look.color(token::BG_HOVER);
        let tinted = fill.blend(hue.opacity(0.16));
        div()
            .id(SharedString::from(format!("for-you-row:{key}")))
            .track_focus(&spot.focus)
            .relative()
            .overflow_hidden()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .border(edge)
            .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.55)) })
            .rounded(look.length(token::RADIUS_MD))
            .bg(if on { tinted } else { fill.blend(hue.opacity(0.08)) })
            .hover(move |style| style.bg(tinted))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
    }

    /// A row's title, and under it the muted line where there is one.
    fn for_you_words(&self, job: &str, key: &str, row: &str, title: AnyElement, line: Option<&str>, look: &Look) -> AnyElement {
        let line = line.map(str::trim).filter(|line| !line.is_empty());
        let said = self.for_you.refused.get(&(job.to_string(), row.to_string())).cloned();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(title)
            .children(line.map(|line| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(line.to_string()))))
            .children(said.map(|why| super::acts::refusal(look, key, why)))
            .into_any_element()
    }

    fn for_you_title(&self, title: impl Into<SharedString>, look: &Look) -> AnyElement {
        div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.into()).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn waiting_row(&self, job: &str, item: &ForYouItem, number: usize, spot: Spot, on: bool, hue: Hsla, look: &Look, cx: &Context<Self>) -> AnyElement {
        let row = Rowed::Waiting(item.id.clone());
        let key = row.key();
        let (press, review, menu) = ((job.to_string(), row.clone()), (job.to_string(), row), (job.to_string(), item.id.clone()));
        let button = labelled(
            div()
                .id(SharedString::from(format!("for-you-review:{key}")))
                .flex_none()
                .self_center()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.for_you_open(&review.0, &review.1, window, cx);
                }))
                .child("Review"),
            Role::Button,
            format!("Review {}", item.title),
        );
        let card = self
            .for_you_card(&key, &spot, on, hue, look)
            .flex_wrap()
            .child(
                div()
                    .flex_1()
                    .min_w(look.length(token::SPACE_12) * 3.)
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().w(look.length(token::SPACE_4)).text_color(hue).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(number.to_string())))
                    .child(self.for_you_words(job, &key, &item.id, self.for_you_title(item.title.clone(), look), item.line.as_deref(), look)),
            )
            .child(button)
            .on_click(cx.listener(move |this, _, window, cx| this.for_you_open(&press.0, &press.1, window, cx)));
        let entries = Rc::new(|| vec![Entry::Item { id: "withdraw", label: "Withdraw".into(), icon: None, danger: false, key: Some("\u{232b}".into()), off: None }]);
        let pick: Pick = Rc::new(move |this, id, window, cx| {
            if id == "withdraw" {
                this.for_you_withdraw(&menu.0, &menu.1, window, cx);
            }
        });
        let card = self.menu_on(card, &spot, item.title.clone(), spot.focus.clone(), entries, pick, cx);
        labelled(card, Role::Row, item.title.clone()).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn building_row(&self, job: &str, build: &ipc::ForYouBuild, spot: Spot, on: bool, hue: Hsla, look: &Look, cx: &Context<Self>) -> AnyElement {
        let row = Rowed::Building(build.job.as_str().to_string());
        let key = row.key();
        let press = (job.to_string(), row);
        let state = self.record(build.job.as_str(), cx).and_then(|record| record.mark.map(|mark| mark.state)).or_else(|| build.standing.map(state_of));
        let mark = state.map(|state| {
            let (icon, said) = crate::jobs::glyph_of(state, None);
            let ink = look.color(crate::jobs::hue_of(state));
            labelled(div().id(SharedString::from(format!("for-you-state:{key}"))).flex_none().h(look.length(token::LEADING_SM)).flex().items_center().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, ink)), Role::Image, said)
        });
        let card = self
            .for_you_card(&key, &spot, on, hue, look)
            .child(div().flex_none().w(look.length(token::SPACE_4)).children(mark))
            .child(self.for_you_words(job, &key, &key, self.for_you_title(build.title.clone(), look), build.line.as_deref(), look))
            .on_click(cx.listener(move |this, _, window, cx| this.for_you_open(&press.0, &press.1, window, cx)));
        labelled(card, Role::Row, format!("Job {}", build.title)).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn landing_row(&self, job: &str, landing: &ipc::ForYouLanding, spot: Spot, on: bool, hue: Hsla, look: &Look, cx: &Context<Self>) -> AnyElement {
        let row = Rowed::Landing(landing.pr);
        let key = row.key();
        let press = (job.to_string(), row);
        let said: SharedString = "Pull request open".into();
        let mark = labelled(div().id(SharedString::from(format!("for-you-state:{key}"))).flex_none().h(look.length(token::LEADING_SM)).flex().items_center().tooltip(tip(said.clone())).child(glyph(look, "check", Size::Small, hue)), Role::Image, said);
        let title = div()
            .flex()
            .flex_wrap()
            .gap_x(look.length(token::SPACE_2))
            .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(format!("#{}", landing.pr))))
            .children((!landing.title.trim().is_empty()).then(|| div().min_w_0().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(landing.title.clone()))))
            .into_any_element();
        let card = self
            .for_you_card(&key, &spot, on, hue, look)
            .child(div().flex_none().w(look.length(token::SPACE_4)).child(mark))
            .child(self.for_you_words(job, &key, &key, title, None, look))
            .on_click(cx.listener(move |this, _, window, cx| this.for_you_open(&press.0, &press.1, window, cx)));
        labelled(card, Role::Row, format!("Pull request #{} {}", landing.pr, landing.title).trim().to_string()).into_any_element()
    }

    /// The tab's shapes while Fleet has not answered.
    pub(super) fn for_you_shapes(&self, look: &Look) -> AnyElement {
        use gpui::AnimationExt as _;
        let bar = |width: f32| div().h(look.length(token::SPACE_3)).w(gpui::relative(width)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07));
        labelled(
            div().id("for-you-loading").w_full().child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(bar(0.3))
                    .child(bar(0.9))
                    .child(bar(0.7))
                    .with_animation(
                        "for-you-loading-pulse",
                        gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shapes, at| shapes.opacity(at),
                    ),
            ),
            Role::Status,
            "Loading",
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Entity, TestAppContext, VisualTestContext};

    const JOB: &str = "mock-01";

    /// The For you walk's Job on the mock Fleet, with its list read as the tab reads it the first time.
    fn open<'a>(cx: &'a mut TestAppContext) -> (Entity<JobView>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("job-for-you").expect("the fixture decodes");
        let threads = crate::mock::threads("job-for-you").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view(|window, cx| JobView::new(list, Some(threads), window, cx));
        view.update_in(cx, |view, window, cx| view.read_for_you(JOB, window, cx));
        (view, cx)
    }

    fn waiting(view: &Entity<JobView>, cx: &mut VisualTestContext) -> Vec<(String, bool)> {
        view.read_with(cx, |view, _| view.for_you.read.get(JOB).and_then(Option::as_ref).map(|read| read.waiting.iter().map(|one| (one.id.clone(), one.placed)).collect()).unwrap_or_default())
    }

    #[gpui::test]
    fn the_sections_stand_waiting_then_building_then_landing_and_one_with_no_rows_is_not_drawn(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let mut read = view.read_with(cx, |view, _| view.for_you.read.get(JOB).cloned().flatten()).expect("read");
        let parts = |read: &ForYou| sections(read).into_iter().map(|(part, rows)| (part, rows.len())).collect::<Vec<_>>();
        assert_eq!(parts(&read), [(Part::Waiting, 2), (Part::Building, 2), (Part::Landing, 2)]);
        assert_eq!(sections(&read)[0].1[0], Rowed::Waiting("document:design".into()), "Fleet's order is kept: the placed row first");
        read.building.clear();
        assert_eq!(parts(&read), [(Part::Waiting, 2), (Part::Landing, 2)]);
        read.waiting.clear();
        read.landing.clear();
        assert!(sections(&read).is_empty(), "a Job with nothing in any section draws nothing");
        assert!(rows(&read).is_empty());
    }

    /// The band's Open chip holds what the tab lists under Waiting on you: a document Fleet added is a row of it, Open is
    /// the chip Needs you stands on, and it presses as the tab's Review does.
    #[gpui::test]
    fn open_lists_the_fleet_added_document_the_for_you_tab_lists(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let ids = view.read_with(cx, |view, cx| view.for_you_beyond_rows(JOB, cx).into_iter().map(|one| one.id.clone()).collect::<Vec<_>>());
        assert!(ids.contains(&"document:design".to_string()), "the document waiting on him is in Open, got {ids:?}");
        let held = view.read_with(cx, |view, cx| view.needs_held(&SharedString::from(JOB), cx));
        assert!(held.open, "with a Fleet-added item waiting, Open holds a row");
        assert_eq!(super::super::column_bands::standing(None, held), Some(super::super::column_bands::Chip::Open), "so the band stands on Open, not Deferred");
        let drawn = view.update_in(cx, |view, _, cx| {
            use crate::look::ActiveLook;
            let look = cx.look().clone();
            view.for_you_beyond_row_views(&SharedString::from(JOB), &look, cx).len()
        });
        assert_eq!(drawn, ids.len(), "one row each");
    }

    /// A For you card is a tint and a full outline in its section's hue, and nothing is laid along one edge of it.
    #[test]
    fn a_for_you_card_has_no_stripe_down_one_side() {
        let source = include_str!("for_you_tab.rs");
        let card = &source[source.find("fn for_you_card(").expect("the card")..source.find("fn for_you_words(").expect("its end")];
        assert!(!card.contains(".absolute()") && !card.contains("rounded_l(") && !card.contains("border_l("), "nothing is laid along the card's edge");
        assert!(card.contains(".border(edge)") && card.contains("hue.opacity("), "a full outline and a tint, both in the hue");
    }

    #[gpui::test]
    fn option_down_on_the_first_waiting_row_posts_every_waiting_id_in_the_new_order(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let sent = view.update_in(cx, |view, window, cx| view.for_you_move(JOB, "document:design", 1, window, cx));
        assert_eq!(sent, Some(OrderForYou { session_id: Some(SessionId::carried(JOB.to_string())), ids: vec!["sketch:Chip states".into(), "document:design".into()] }));
        assert_eq!(waiting(&view, cx), [("sketch:Chip states".to_string(), true), ("document:design".to_string(), true)], "moved at once, every row placed");
        let past_the_end = view.update_in(cx, |view, window, cx| view.for_you_move(JOB, "document:design", 1, window, cx));
        assert_eq!(past_the_end, None, "the last row does not move down");
        let up = view.update_in(cx, |view, window, cx| view.for_you_move(JOB, "document:design", -1, window, cx));
        assert_eq!(up.map(|order| order.ids), Some(vec!["document:design".to_string(), "sketch:Chip states".to_string()]));
    }

    #[gpui::test]
    fn withdraw_posts_the_id_of_the_row_and_takes_only_that_row_off(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let sent = view.update_in(cx, |view, window, cx| view.for_you_withdraw(JOB, "sketch:Chip states", window, cx));
        assert_eq!(sent, Some(WithdrawForYou { session_id: Some(SessionId::carried(JOB.to_string())), id: "sketch:Chip states".into() }));
        assert_eq!(waiting(&view, cx), [("document:design".to_string(), true)]);
        let unknown = view.update_in(cx, |view, window, cx| view.for_you_withdraw(JOB, "nothing:here", window, cx));
        assert_eq!(unknown, None, "a row that is not there sends nothing");
    }

    #[gpui::test]
    fn a_refused_withdrawal_puts_the_row_back_with_fleets_sentence(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let before = view.read_with(cx, |view, _| view.for_you.read.get(JOB).cloned().flatten()).expect("read");
        view.update_in(cx, |view, _, cx| {
            view.for_you_answered(JOB, "sketch:Chip states", before, Err("The id names no row".to_string()), cx);
        });
        assert_eq!(waiting(&view, cx).len(), 2);
        let said = view.read_with(cx, |view, _| view.for_you.refused.get(&(JOB.to_string(), "sketch:Chip states".to_string())).cloned());
        assert_eq!(said.as_deref(), Some("The id names no row"));
    }

    #[gpui::test]
    /// A question Fleet also names for him is not listed twice; its one line in Needs you brings the thread to its card.
    fn a_question_needs_you_already_lists_has_no_second_row_and_its_line_brings_the_thread_to_its_card(cx: &mut TestAppContext) {
        use super::super::parts_fold::{redraw, rooted_job};
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", gpui::size(gpui::px(1440.), gpui::px(900.)), |_| {});
        let question = "Where does the chip show how far behind it is?";
        let row = ForYouItem {
            id: "ask:call-1#1".into(),
            kind: ipc::ForYouKind::Question,
            title: question.into(),
            line: None,
            placed: false,
            since: ipc::Instant::carried("2026-11-15T14:02:00Z"),
            added_by: ipc::ForYouBy::Fleet,
            opens: ForYouOpens::Waiting { id: "ask:call-1#1".into() },
            approves: false,
        };
        let read = ForYou { job: SessionId::carried("mock-01".to_string()), waiting: vec![row], building: Vec::new(), landing: Vec::new() };
        view.update(cx, |view, cx| {
            view.for_you.read.insert("mock-01".into(), Some(read));
            cx.notify();
        });
        redraw(cx);
        let find = |name: String, cx: &mut gpui::VisualTestContext| cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, &name).map(|one| one.bounds.center()));
        assert!(find(format!("Review {question}"), cx).is_none(), "the question is not drawn a second time as a row of its own");
        // With the panel folded, its line stands in the thread and the right column's band.
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        let at = find(question.to_string(), cx).expect("the question is one line");
        cx.simulate_click(at, gpui::Modifiers::none());
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the sheet stays shut");
        let card = cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Article, &format!("Question: {question}")).map(|one| one.bounds));
        let thread = cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Region, "Thread").map(|one| one.bounds));
        assert!(card.zip(thread).is_some_and(|(card, thread)| thread.contains(&card.origin)), "the thread stands at the question's card");
    }

    #[gpui::test]
    fn the_review_button_on_a_ready_to_try_row_opens_the_review_panel_with_the_newest_build_chosen(cx: &mut TestAppContext) {
        use super::super::parts_fold::{drawn, redraw};
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(900.)));
        list.update(cx, |list, cx| list.open("tool-edit".to_string().into(), cx));
        let row = ForYouItem {
            id: "try:tool-edit".into(),
            kind: ipc::ForYouKind::Approve,
            title: "Ready to try".into(),
            line: None,
            placed: false,
            since: ipc::Instant::carried("2026-11-15T14:02:00Z"),
            added_by: ipc::ForYouBy::Fleet,
            opens: ForYouOpens::Waiting { id: "try:tool-edit".into() },
            approves: false,
        };
        let read = ForYou { job: SessionId::carried("tool-edit".to_string()), waiting: vec![row], building: Vec::new(), landing: Vec::new() };
        view.update(cx, |view, cx| {
            view.for_you.read.insert("tool-edit".into(), Some(read));
            cx.notify();
        });
        redraw(cx);
        let at = cx
            .update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, "Review Ready to try").map(|one| one.bounds.center()))
            .expect("the row has its Review button");
        cx.simulate_click(at, gpui::Modifiers::none());
        redraw(cx);
        let chosen = view.read_with(cx, |view, _| view.review().filter(|review| review.open).and_then(|_| view.review_chosen()));
        let chosen = chosen.expect("the review panel is open on an item");
        assert_eq!(chosen.kind, super::super::review::Kind::Build);
        assert_eq!(chosen.name, "The reviewer's brief names its files", "the newest open build is chosen");
        assert!(drawn(cx, gpui::Role::Region, "The reviewer's brief names its files"), "drawn in the detail");
        assert!(drawn(cx, gpui::Role::Button, "Looks good"), "with its answer in the bar");
    }

    #[test]
    fn a_row_moves_one_place_and_stays_inside_the_waiting_rows() {
        let at = ipc::Instant::carried("2026-11-09T13:45:15Z");
        let item = |id: &str| ForYouItem { id: id.into(), kind: ipc::ForYouKind::Question, title: id.into(), line: None, placed: false, since: at.clone(), added_by: ipc::ForYouBy::Fleet, opens: ForYouOpens::Plan, approves: false };
        let read = ForYou { job: SessionId::carried("j".to_string()), waiting: vec![item("a"), item("b"), item("c")], building: Vec::new(), landing: Vec::new() };
        assert_eq!(moved(&read, "b", 1), Some(vec!["a".into(), "c".into(), "b".into()]));
        assert_eq!(moved(&read, "b", -1), Some(vec!["b".into(), "a".into(), "c".into()]));
        assert_eq!(moved(&read, "a", -1), None);
        assert_eq!(moved(&read, "c", 1), None);
        assert_eq!(moved(&read, "gone", 1), None);
    }

    #[test]
    fn for_you_is_a_tab_a_pane_can_hold_the_plus_offers_and_a_zone_that_draws_it() {
        assert!(super::super::pane_tree::is_tab_kind("for_you"));
        assert_eq!(super::super::panes_place::zone_of("for_you").map(Zone::id), Some("for_you"));
        // A Job kept before the column's bands has it first in the column, before Now.
        let top = super::super::pane_tree::legacy_arrangement();
        let panes = super::super::pane_tree::panes_of(&top.root);
        assert_eq!(panes[1].tabs.first().map(|tab| tab.kind.as_str()), Some("for_you"), "first in the column, before Now");
    }
}
