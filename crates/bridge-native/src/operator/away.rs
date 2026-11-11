//! Away: the Operator answering and starting work while he is gone, as take A of the approved sketch
//! "Operator: walk-away mode" draws it. It is Fleet's Autopilot, one per repository, the same record
//! Electron's guarded switch (#3194) starts: the head's Away opens what it may do before anything starts,
//! Start runs one for each project ticked, Come back stops them, and what they answered waits under While
//! you were away until he agrees or changes it. `docs/concepts/autopilot.md`.
//!
//! **The marks on Your turn are read here, not served**: Fleet decides at the moment an item has waited
//! two minutes, from the same record, so a mark says what the boxes he ticked will do with it.

use std::collections::BTreeSet;

use gpui::{div, prelude::*, AnyElement, Context, Entity, FocusHandle, Focusable, FontWeight, MouseButton, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::operator_page::{OperatorTurnItem, OperatorTurnKind};
use ipc::{Autopilot, AutopilotDecided, AutopilotDecision, AutopilotPickState, AutopilotSettings, AutopilotState, AutopilotWork, WaitingActKind, WaitingSource};

use super::face;
use super::page::{Opened, OperatorPageView};
use super::view::{chip, minute, press, pressed, round, Weight};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, spinner, tip};

/// What the Autopilots he starts are told: the picker reads it beside the open issues.
const INSTRUCTION: &str = "Work the open issues while I'm away, the ones I would pick first.";

/// What always waits for him, whatever he ticks: Fleet has no box for it.
const NEVER: &str = "Never, whatever is ticked: anything that deletes or can't be undone, a prompt in a terminal, a command it can't read whole, a path outside its slots, or a question that asks you to run or approve something.";

/// The kinds it may answer, in his words, and whether a first Away ticks them.
pub const ANSWERS: [(AutopilotDecision, &str, bool); 10] = [
    (AutopilotDecision::Recommended, "Questions with a recommended answer", true),
    (AutopilotDecision::Open, "Questions with no recommendation", false),
    (AutopilotDecision::Plan, "Plans", true),
    (AutopilotDecision::Draft, "Drafts", false),
    (AutopilotDecision::Walk, "Walks", false),
    (AutopilotDecision::Land, "Landing a pull request once its checks pass", true),
    (AutopilotDecision::Red, "A red main: start its fix", true),
    (AutopilotDecision::Unsure, "Starting an issue I was unsure of", false),
    (AutopilotDecision::Reads, "Tools that only read", true),
    (AutopilotDecision::Writes, "Edits, and commands I can read whole", false),
];

/// What it works, in his words, with what each means.
pub const WORK: [(&str, &str); 3] = [
    ("The backlog", "Start Jobs on open issues nobody holds"),
    ("Jobs already running", "Answer for every Job in the project, not only the ones it started"),
    ("Bugs agents reported", "Work the bugs agents reported in Armada itself"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Until,
    Spend,
    Jobs,
    Week,
}

const LIMITS: [(Limit, &str); 4] = [(Limit::Until, "Until"), (Limit::Spend, "Spend, each project"), (Limit::Jobs, "Jobs started, each project"), (Limit::Week, "Week used")];

/// One line of the setup the arrows move through: left column first, then what it answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupRow {
    Project(usize),
    Work(usize),
    Limit(Limit),
    Answer(usize),
}

/// What he is ticking before Away starts.
#[derive(Clone, Debug)]
pub struct Setup {
    /// Each repository on the page: its manifest id, its name, and whether it goes away.
    pub projects: Vec<(String, String, bool)>,
    pub work: AutopilotWork,
    pub decisions: BTreeSet<AutopilotDecision>,
    pub until: chrono::DateTime<chrono::Local>,
    /// Dollars; 0 is no cap.
    pub spend: u32,
    /// 0 is no cap.
    pub jobs: u32,
    /// Percent of the week's plan; 100 is no cap.
    pub week: u32,
    /// The line the keys are on.
    pub at: usize,
}

impl Setup {
    /// The projects on the page, every one ticked, and what the last Away ticked, else the first one's boxes.
    pub fn new(projects: Vec<(String, String)>, last: Option<&Autopilot>, now: chrono::DateTime<chrono::Local>) -> Setup {
        let decisions = last.map_or_else(|| ANSWERS.iter().filter(|(_, _, on)| *on).map(|(kind, _, _)| *kind).collect(), |last| last.decisions.clone());
        let evening = now.date_naive().and_hms_opt(18, 0, 0).and_then(|at| at.and_local_timezone(chrono::Local).single());
        let until = evening.filter(|at| *at - now >= chrono::Duration::hours(1)).unwrap_or(now + chrono::Duration::hours(4));
        Setup {
            projects: projects.into_iter().map(|(id, name)| (id, name, true)).collect(),
            work: last.map_or(AutopilotWork { backlog: true, running: false, reports: false }, |last| last.on_autopilot),
            decisions,
            until,
            spend: 20,
            jobs: 3,
            week: 80,
            at: 0,
        }
    }

    pub fn rows(&self) -> Vec<SetupRow> {
        let mut rows: Vec<SetupRow> = (0..self.projects.len()).map(SetupRow::Project).collect();
        rows.extend((0..WORK.len()).map(SetupRow::Work));
        rows.extend(LIMITS.iter().map(|(limit, _)| SetupRow::Limit(*limit)));
        rows.extend((0..ANSWERS.len()).map(SetupRow::Answer));
        rows
    }

    fn work_on(&self, at: usize) -> bool {
        match at {
            0 => self.work.backlog,
            1 => self.work.running,
            _ => self.work.reports,
        }
    }

    pub fn ticked(&self, row: SetupRow) -> bool {
        match row {
            SetupRow::Project(at) => self.projects.get(at).is_some_and(|(_, _, on)| *on),
            SetupRow::Work(at) => self.work_on(at),
            SetupRow::Answer(at) => ANSWERS.get(at).is_some_and(|(kind, _, _)| self.decisions.contains(kind)),
            SetupRow::Limit(_) => false,
        }
    }

    pub fn toggle(&mut self, row: SetupRow) {
        match row {
            SetupRow::Project(at) => {
                if let Some(project) = self.projects.get_mut(at) {
                    project.2 = !project.2;
                }
            }
            SetupRow::Work(0) => self.work.backlog = !self.work.backlog,
            SetupRow::Work(1) => self.work.running = !self.work.running,
            SetupRow::Work(_) => self.work.reports = !self.work.reports,
            SetupRow::Answer(at) => {
                if let Some((kind, _, _)) = ANSWERS.get(at) {
                    if !self.decisions.remove(kind) {
                        self.decisions.insert(*kind);
                    }
                }
            }
            SetupRow::Limit(_) => {}
        }
    }

    /// ← or − takes a limit down a step, → or + up: half an hour, $5, one Job, 5% of the week.
    pub fn nudge(&mut self, limit: Limit, by: i32, now: chrono::DateTime<chrono::Local>) {
        match limit {
            Limit::Until => {
                let until = self.until + chrono::Duration::minutes(30 * by as i64);
                self.until = until.clamp(now + chrono::Duration::minutes(30), now + chrono::Duration::hours(24));
            }
            Limit::Spend => self.spend = (self.spend as i32 + 5 * by).clamp(0, 500) as u32,
            Limit::Jobs => self.jobs = (self.jobs as i32 + by).clamp(0, 20) as u32,
            Limit::Week => self.week = (self.week as i32 + 5 * by).clamp(50, 100) as u32,
        }
    }

    pub fn said(&self, limit: Limit) -> String {
        match limit {
            Limit::Until => self.until.format("%H:%M").to_string(),
            Limit::Spend if self.spend == 0 => "No cap".into(),
            Limit::Spend => format!("${}", self.spend),
            Limit::Jobs if self.jobs == 0 => "No cap".into(),
            Limit::Jobs => self.jobs.to_string(),
            Limit::Week if self.week >= 100 => "No cap".into(),
            Limit::Week => format!("{}%", self.week),
        }
    }

    pub fn going(&self) -> Vec<String> {
        self.projects.iter().filter(|(_, _, on)| *on).map(|(id, _, _)| id.clone()).collect()
    }

    /// What `start_autopilot` is sent for each project ticked.
    pub fn settings(&self) -> AutopilotSettings {
        AutopilotSettings {
            instruction: INSTRUCTION.into(),
            at_once: 1,
            until: Some(ipc::Instant::carried(self.until.with_timezone(&chrono::Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))),
            spend_cap_micros: (self.spend > 0).then(|| self.spend as u64 * 1_000_000),
            session_cap: (self.jobs > 0).then_some(self.jobs),
            weekly_cap_percent: (self.week < 100).then_some(self.week),
            on_autopilot: self.work,
            decisions: self.decisions.clone(),
            project: None,
        }
    }
}

/// Away's state on the page.
pub struct Away {
    /// Every repository's Autopilot, as Fleet holds them, stopped ones included.
    pub autopilots: Vec<Autopilot>,
    /// What they answered that he has not looked at.
    pub to_review: Vec<AutopilotDecided>,
    pub setup: Option<Setup>,
    pub setup_focus: FocusHandle,
    /// The head's Away: where the keys go back to when the setup is cancelled.
    pub press_focus: FocusHandle,
    /// Back? over the page, when he comes back to the window with Away still on.
    pub back: bool,
    pub back_focus: FocusHandle,
    pub starting: bool,
    pub stopping: bool,
    pub refused: Option<SharedString>,
    /// Answers and picks whose press is in flight, by id.
    pub pending: BTreeSet<String>,
    /// The answer he is changing, and the box his words go in.
    pub change: Option<(String, Entity<InputState>)>,
    /// When the window last went behind another.
    pub left_at: Option<std::time::Instant>,
    _typed: Option<Subscription>,
}

impl Away {
    pub fn new(cx: &mut gpui::App) -> Away {
        Away {
            autopilots: Vec::new(),
            to_review: Vec::new(),
            setup: None,
            setup_focus: cx.focus_handle(),
            press_focus: cx.focus_handle(),
            back: false,
            back_focus: cx.focus_handle(),
            starting: false,
            stopping: false,
            refused: None,
            pending: BTreeSet::new(),
            change: None,
            left_at: None,
            _typed: None,
        }
    }

    /// The Autopilots answering for him now: running, or paused by a cap until he raises it.
    pub fn on(&self) -> Vec<&Autopilot> {
        self.autopilots.iter().filter(|one| one.state != AutopilotState::Stopped).collect()
    }

    pub fn is_away(&self) -> bool {
        self.autopilots.iter().any(|one| one.state != AutopilotState::Stopped)
    }

    fn put(&mut self, autopilot: Autopilot) {
        self.autopilots.retain(|one| one.manifest_id != autopilot.manifest_id);
        self.autopilots.push(autopilot);
    }

    /// Whether Away answers this item, and why, in a tooltip's words. `None` while he is here.
    pub fn answers(&self, turn: &OperatorTurnItem) -> Option<(bool, String)> {
        if !self.is_away() {
            return None;
        }
        let place = turn.repository.clone().or_else(|| turn.manifest_id.clone()).unwrap_or_else(|| "This project".into());
        let Some(autopilot) = self.on().into_iter().find(|one| Some(one.manifest_id.as_str()) == turn.manifest_id.as_deref()) else {
            return Some((false, format!("{place} is not away")));
        };
        let Some(kind) = decision_of(turn) else { return Some((false, "Nothing here is one it may answer: it waits for you".into())) };
        let words = ANSWERS.iter().find(|(one, _, _)| *one == kind).map_or("These", |(_, words, _)| words);
        if !autopilot.decisions.contains(&kind) {
            return Some((false, format!("{words} are not ticked for {place}")));
        }
        let its_own = autopilot.picks.iter().any(|pick| pick.session_id.is_some() && pick.session_id == turn.session_id);
        if !autopilot.on_autopilot.running && !its_own {
            return Some((false, "It answers only for the Jobs it started: Jobs already running is not ticked".into()));
        }
        Some((true, format!("{words} are ticked for {place}: it answers once this has waited two minutes")))
    }
}

/// The kind of decision an item is, as Fleet's Autopilot boxes name them. `None` is one it never answers.
pub fn decision_of(turn: &OperatorTurnItem) -> Option<AutopilotDecision> {
    if face::is_red_main(turn) {
        return Some(AutopilotDecision::Red);
    }
    if turn.kind == OperatorTurnKind::GoAhead {
        return None;
    }
    let item = &turn.item;
    let act = item.act.as_ref().map(|act| act.kind);
    Some(match (item.source, act) {
        (_, Some(WaitingActKind::Run)) => return None,
        (WaitingSource::Plan, _) => AutopilotDecision::Plan,
        (WaitingSource::Walk, _) | (_, Some(WaitingActKind::Walk)) => AutopilotDecision::Walk,
        (_, Some(WaitingActKind::ApprovePr)) => AutopilotDecision::Land,
        (WaitingSource::Draft, _) => AutopilotDecision::Draft,
        (WaitingSource::Permission, _) => AutopilotDecision::Writes,
        (WaitingSource::AskCard, _) | (WaitingSource::WindowAsk, _) => recommended(turn),
        _ if !item.options.is_empty() => recommended(turn),
        _ => return None,
    })
}

fn recommended(turn: &OperatorTurnItem) -> AutopilotDecision {
    if face::pick_of(turn).is_some() { AutopilotDecision::Recommended } else { AutopilotDecision::Open }
}

/// The places named in a sentence: `armada`, `armada and storefront`, `a, b and c`.
fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

impl OperatorPageView {
    // -- reading -----------------------------------------------------------------------------------

    /// Every Autopilot and what they answered, read whole.
    pub fn read_away(&mut self, cx: &mut Context<Self>) {
        let (autopilots, decisions) = (self.wire.autopilots(cx), self.wire.decisions(cx));
        cx.spawn(async move |this, cx| {
            let (autopilots, decisions) = (autopilots.await, decisions.await);
            let _ = this.update(cx, |this, cx| {
                if let Ok(list) = autopilots {
                    this.away.autopilots = list.autopilots;
                }
                if let Ok(decisions) = decisions {
                    this.away.to_review = decisions.to_review;
                }
                cx.notify();
            });
        })
        .detach();
    }

    // -- the keys ----------------------------------------------------------------------------------

    /// The setup and Back? take the keys over the page, and a correction's box keeps its own. True where
    /// the act was theirs.
    pub fn away_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.away.change.as_ref().is_some_and(|(_, input)| input.read(cx).focus_handle(cx).contains_focused(window, cx)) {
            cx.propagate();
            return true;
        }
        if self.away.back {
            match act.id {
                "call_send" if self.away.back_focus.is_focused(window) => self.come_back(window, cx),
                "call_send" => cx.propagate(),
                id if id.starts_with("call_") => {}
                _ => cx.propagate(),
            }
            return true;
        }
        if self.away.setup.is_some() {
            match act.id {
                "call_move_focus" => self.setup_step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
                "call_send" if self.away.setup_focus.is_focused(window) => self.start_away(window, cx),
                "call_send" => cx.propagate(),
                id if id.starts_with("call_") => {}
                _ => cx.propagate(),
            }
            return true;
        }
        false
    }

    /// Space ticks the line the keys are on, ← and → move a limit, Esc cancels.
    fn setup_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.modified() {
            return;
        }
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let Some(setup) = self.away.setup.as_mut() else { return };
        let row = setup.rows().get(setup.at).copied();
        match (key.key.as_str(), row) {
            ("escape", _) => {
                cx.stop_propagation();
                return self.close_setup(window, cx);
            }
            ("space", Some(row)) if self.away.setup_focus.is_focused(window) => setup.toggle(row),
            ("left", Some(SetupRow::Limit(limit))) => setup.nudge(limit, -1, now),
            ("right", Some(SetupRow::Limit(limit))) => setup.nudge(limit, 1, now),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // -- the acts ----------------------------------------------------------------------------------

    /// Away in the head: nothing starts until the setup's Start.
    pub fn open_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let projects = self.page.as_ref().map_or(Vec::new(), |page| page.mains.iter().map(|main| (main.manifest_id.clone(), main.repository.clone())).collect());
        let last = self.away.autopilots.iter().max_by(|a, b| a.started_at.as_str().cmp(b.started_at.as_str()));
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        self.away.setup = Some(Setup::new(projects, last, now));
        self.away.refused = None;
        window.focus(&self.away.setup_focus, cx);
        cx.notify();
    }

    pub fn close_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.away.setup = None;
        window.focus(&self.away.press_focus, cx);
        cx.notify();
    }

    fn setup_step(&mut self, by: isize, cx: &mut Context<Self>) {
        if let Some(setup) = self.away.setup.as_mut() {
            setup.at = (setup.at as isize + by).clamp(0, setup.rows().len() as isize - 1) as usize;
            cx.notify();
        }
    }

    /// Start: an Autopilot for each project ticked. The band stands at once; a refusal keeps the setup
    /// open with why, and whatever did start stays on.
    pub fn start_away(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(setup) = self.away.setup.as_ref() else { return };
        if self.away.starting {
            return;
        }
        let going = setup.going();
        if going.is_empty() {
            self.away.refused = Some("Tick a project to go away from".into());
            return cx.notify();
        }
        let settings = setup.settings();
        self.away.starting = true;
        self.away.refused = None;
        let tasks: Vec<_> = going.iter().map(|manifest| self.wire.start_away(manifest, settings.clone(), cx)).collect();
        window.focus(&self.focus, cx);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut answers = Vec::new();
            for task in tasks {
                answers.push(task.await);
            }
            let _ = this.update(cx, |this, cx| {
                this.away.starting = false;
                let mut refused = Vec::new();
                for answer in answers {
                    match answer {
                        Ok(autopilot) => this.away.put(autopilot),
                        Err(why) => refused.push(why),
                    }
                }
                if refused.is_empty() {
                    this.away.setup = None;
                } else {
                    this.away.refused = Some(refused.join(". ").into());
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Come back, or I'm back: every Autopilot stops, and what they answered stays to look at.
    pub fn come_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.away.back = false;
        if self.away.stopping {
            return;
        }
        let on: Vec<String> = self.away.on().iter().map(|one| one.manifest_id.as_str().to_string()).collect();
        let tasks: Vec<_> = on.iter().map(|manifest| self.wire.stop_away(manifest, cx)).collect();
        self.away.stopping = true;
        window.focus(&self.focus, cx);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut answers = Vec::new();
            for task in tasks {
                answers.push(task.await);
            }
            let _ = this.update(cx, |this, cx| {
                this.away.stopping = false;
                for answer in answers {
                    match answer {
                        Ok(autopilot) => this.away.put(autopilot),
                        Err(why) => this.away.refused = Some(why.into()),
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Keep it on: Back? goes, and Away stays.
    pub fn keep_away(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.away.back = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The window came back to the front: after ten minutes behind others with Away on, Back? asks.
    pub fn came_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let gone = self.away.left_at.take().is_some_and(|at| at.elapsed() >= std::time::Duration::from_secs(600));
        if gone && self.away.is_away() {
            self.ask_back(window, cx);
        }
    }

    pub fn ask_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.away.back = true;
        window.focus(&self.away.back_focus, cx);
        cx.notify();
    }

    /// Agree: the answer stays as it was given, and leaves the list.
    pub fn agree(&mut self, id: String, cx: &mut Context<Self>) {
        self.reviewed(id, None, cx);
    }

    /// Change: a box under the answer, for what it should have done. Enter sends it to the Job.
    pub fn open_change(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("What should it have done?"));
        let about = id.clone();
        let typed = cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { secondary: false, .. } = event {
                let words = input.read(cx).value().trim().to_string();
                if !words.is_empty() {
                    this.away.change = None;
                    this.away._typed = None;
                    window.focus(&this.focus, cx);
                    this.reviewed(about.clone(), Some(words), cx);
                }
            }
        });
        window.focus(&input.read(cx).focus_handle(cx), cx);
        self.away.change = Some((id, input));
        self.away._typed = Some(typed);
        cx.notify();
    }

    pub fn drop_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.away.change = None;
        self.away._typed = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn reviewed(&mut self, id: String, change: Option<String>, cx: &mut Context<Self>) {
        self.away.pending.insert(id.clone());
        cx.notify();
        let task = self.wire.reviewed(&id, change, cx);
        cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.away.pending.remove(&id);
                match answer {
                    Ok(decisions) => this.away.to_review = decisions.to_review,
                    Err(why) => this.away.refused = Some(why.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Start or Dismiss on an issue it set aside.
    pub fn settle_pick(&mut self, manifest: String, pick: String, start: bool, cx: &mut Context<Self>) {
        self.away.pending.insert(pick.clone());
        cx.notify();
        let task = self.wire.pick(&manifest, &pick, start, cx);
        cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.away.pending.remove(&pick);
                match answer {
                    Ok(autopilot) => this.away.put(autopilot),
                    Err(why) => this.away.refused = Some(why.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    // -- drawn -------------------------------------------------------------------------------------

    /// Away, at the end of the head, while he is here.
    pub(super) fn away_press(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if self.page.is_none() || self.away.is_away() {
            return None;
        }
        let press = press("away", "Away".into(), Some("plane-takeoff"), None, Weight::Plain, look)
            .track_focus(&self.away.press_focus)
            .h(look.length(token::H_CONTROL))
            .text_size(look.length(token::TEXT_XS))
            .tooltip(tip("Hand your turn to the Operator while you're away: you choose what it may do first"));
        Some(labelled(pressed(press, cx, |this, window, cx| this.open_setup(window, cx)), Role::Button, "Away").into_any_element())
    }

    /// The band under the head while Away is on: since when, where and until when, what it has done, and Come back.
    pub(super) fn away_band(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let on = self.away.on();
        let first = on.iter().min_by(|a, b| a.started_at.as_str().cmp(b.started_at.as_str()))?;
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let places: Vec<String> = on.iter().map(|one| self.place_of(one.manifest_id.as_str())).collect();
        let until = on.iter().filter_map(|one| one.until.as_ref()).max_by(|a, b| a.as_str().cmp(b.as_str())).and_then(|at| minute(at.as_str(), now));
        let started: u32 = on.iter().map(|one| one.sessions_started).sum();
        let spent: u64 = on.iter().map(|one| one.spent_micros).sum();
        // Each project's Autopilot holds its own cap, the one the setup set for all of them.
        let caps: BTreeSet<Option<u64>> = on.iter().map(|one| one.spend_cap_micros).collect();
        let cap = if caps.len() == 1 { caps.into_iter().next().flatten() } else { None };
        let mut where_ = format!("answering for you in {}", and_list(&places));
        if let Some(until) = until {
            where_.push_str(&format!(" until {until}"));
        }
        let mut done = vec![match started {
            1 => "1 Job started".to_string(),
            n => format!("{n} Jobs started"),
        }];
        done.push(match cap {
            Some(cap) if on.len() > 1 => format!("${:.2} spent, ${:.0} each", spent as f64 / 1e6, cap as f64 / 1e6),
            Some(cap) => format!("${:.2} of ${:.0}", spent as f64 / 1e6, cap as f64 / 1e6),
            None => format!("${:.2} spent", spent as f64 / 1e6),
        });
        let paused = on.iter().any(|one| one.state == AutopilotState::Paused).then(|| "paused at a limit".to_string());
        let said = format!("Away since {}, {where_}. {}", minute(first.started_at.as_str(), now).unwrap_or_default(), done.join(", "));
        let back = if self.away.stopping {
            div().flex().items_center().gap(look.length(token::SPACE_2)).text_color(look.color(token::FG_MUTED)).child(spinner("away-stop", look.length(token::SPACE_4), look.color(token::OPERATOR), cx)).child("Coming back").into_any_element()
        } else {
            let press = press("come-back", "Come back".into(), Some("plane-landing"), None, Weight::Plain, look).h(look.length(token::H_CONTROL)).text_size(look.length(token::TEXT_XS)).tooltip(tip("Stop answering for you: what it did stays under While you were away"));
            labelled(pressed(press, cx, |this, window, cx| this.come_back(window, cx)), Role::Button, "Come back").into_any_element()
        };
        Some(
            labelled(
                div()
                    .id("away-band")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_6))
                    .py(look.length(token::SPACE_2))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::OPERATOR_EDGE))
                    .bg(look.color(token::OPERATOR_FAINT))
                    .text_size(look.length(token::TEXT_SM))
                    .child(glyph("plane", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Away"))
                    .child(self.stamp(first.started_at.as_str(), Some("since"), (token::TEXT_SM, token::FG_DEFAULT, token::WEIGHT_HEADING), look, cx))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(format!("{where_} · {}", done.into_iter().chain(paused).collect::<Vec<_>>().join(" · "))))
                    .child(back),
                Role::Status,
                said,
            )
            .into_any_element(),
        )
    }

    /// A repository's name on the page, by its manifest id.
    fn place_of(&self, manifest: &str) -> String {
        self.page.as_ref().and_then(|page| page.mains.iter().find(|main| main.manifest_id == manifest)).map_or_else(|| manifest.to_string(), |main| main.repository.clone())
    }

    /// On a row of Your turn while Away is on: whether it answers this or leaves it for him.
    pub(super) fn away_mark(&self, at: usize, turn: &OperatorTurnItem, look: &Look) -> Option<AnyElement> {
        let (answers, why) = self.away.answers(turn)?;
        let (word, hue, ground, mark) = if answers {
            ("I answer this", token::OPERATOR, token::OPERATOR_FAINT, "plane")
        } else {
            ("Waits for you", token::FG_MUTED, token::BG_HOVER, "hand")
        };
        Some(
            labelled(
                div().id(gpui::SharedString::from(format!("away-mark-{at}"))).flex_none().tooltip(tip(why.clone())).child(chip(word, look.color(hue), look.color(ground), look).child(glyph(mark, look.length(token::SPACE_3), look.color(hue)))),
                Role::Status,
                format!("{word}: {why}"),
            )
            .into_any_element(),
        )
    }

    /// While you were away, atop the digest: each answer with Agree and Change, each issue it set aside
    /// with Start and Dismiss, each Job it started, under its project.
    pub(super) fn away_tile(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let set_aside = self.away.autopilots.iter().any(|one| one.picks.iter().any(|pick| pick.state == AutopilotPickState::SetAside));
        if self.away.to_review.is_empty() && !set_aside {
            return None;
        }
        let small = |id: String, said: &str, weight: Weight| press(id, said.to_string().into(), None, None, weight, look).h(look.length(token::H_CONTROL_SM)).px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS));
        let mut places: Vec<String> = self.away.to_review.iter().map(|one| one.manifest_id.as_str().to_string()).chain(self.away.autopilots.iter().filter(|one| !one.picks.is_empty()).map(|one| one.manifest_id.as_str().to_string())).collect();
        places.sort();
        places.dedup();
        let mut body: Vec<AnyElement> = Vec::new();
        for manifest in places {
            body.push(div().pt(look.length(if body.is_empty() { token::SPACE_1 } else { token::SPACE_2 })).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(self.place_of(&manifest)).into_any_element());
            for decided in self.away.to_review.iter().filter(|one| one.manifest_id.as_str() == manifest) {
                let id = decided.id.clone();
                let pending = self.away.pending.contains(&id);
                let changing = self.away.change.as_ref().filter(|(about, _)| *about == id).map(|(_, input)| input.clone());
                let said = format!("{}: {}. It chose {}", decided.who, decided.asked, decided.chose);
                let (agree_id, change_id) = (id.clone(), id.clone());
                let acts = if pending {
                    spinner(SharedString::from(format!("review-spin-{id}")), look.length(token::SPACE_4), look.color(token::FG_SUBTLE), cx).into_any_element()
                } else {
                    div()
                        .flex()
                        .gap(look.length(token::SPACE_1))
                        .child(labelled(pressed(small(format!("agree-{id}"), "Agree", Weight::Plain).tooltip(tip("It was right: keep the answer as it is")), cx, move |this, _, cx| this.agree(agree_id.clone(), cx)), Role::Button, format!("Agree with {}", decided.asked)))
                        .child(labelled(pressed(small(format!("change-{id}"), "Change", Weight::Bare).tooltip(tip("Tell its Job what it should have done instead")), cx, move |this, window, cx| this.open_change(change_id.clone(), window, cx)), Role::Button, format!("Change {}", decided.asked)))
                        .into_any_element()
                };
                let box_ = changing.map(|input| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("change-box-{id}")))
                            .mt(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::OPERATOR_EDGE))
                            .bg(look.color(token::BG_RAISED))
                            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                                if event.keystroke.key == "escape" {
                                    cx.stop_propagation();
                                    this.drop_change(window, cx);
                                }
                            }))
                            .child(Input::new(&input).appearance(false).bordered(false)),
                        Role::TextInput,
                        "What should it have done?",
                    )
                });
                body.push(
                    labelled(
                        div()
                            .id(SharedString::from(format!("decided-{id}")))
                            .flex()
                            .flex_col()
                            .py(look.length(token::SPACE_1))
                            .gap(look.length(token::SPACE_1))
                            .child(
                                div()
                                    .id(SharedString::from(format!("decided-words-{id}")))
                                    .min_w_0()
                                    .text_size(look.length(token::TEXT_XS))
                                    .tooltip(tip(decided.because.clone()))
                                    .child(div().line_clamp(2).text_ellipsis().child(format!("{}: {}", decided.who, decided.asked)))
                                    .child(div().line_clamp(2).text_ellipsis().text_color(look.color(token::FG_MUTED)).child(format!("Chose {}. {}", decided.chose, decided.because))),
                            )
                            .child(acts)
                            .children(box_),
                        Role::ListItem,
                        said,
                    )
                    .into_any_element(),
                );
            }
            for autopilot in self.away.autopilots.iter().filter(|one| one.manifest_id.as_str() == manifest) {
                for pick in autopilot.picks.iter().filter(|pick| matches!(pick.state, AutopilotPickState::Started | AutopilotPickState::SetAside)) {
                    let aside = pick.state == AutopilotPickState::SetAside;
                    let what = format!("{}: {} {}", if aside { "Set aside" } else { "Started" }, pick.issue_reference, pick.issue_title);
                    let acts = if self.away.pending.contains(&pick.pick_id) {
                        spinner(SharedString::from(format!("pick-spin-{}", pick.pick_id)), look.length(token::SPACE_4), look.color(token::FG_SUBTLE), cx).into_any_element()
                    } else if aside {
                        let (start, dismiss) = ((manifest.clone(), pick.pick_id.clone()), (manifest.clone(), pick.pick_id.clone()));
                        div()
                            .flex()
                            .gap(look.length(token::SPACE_1))
                            .child(labelled(pressed(small(format!("start-{}", pick.pick_id), "Start", Weight::Plain).tooltip(tip("Start a Job on it now")), cx, move |this, _, cx| this.settle_pick(start.0.clone(), start.1.clone(), true, cx)), Role::Button, format!("Start {}", pick.issue_reference)))
                            .child(labelled(pressed(small(format!("dismiss-{}", pick.pick_id), "Dismiss", Weight::Bare).tooltip(tip("Leave it: this Away won't pick it again")), cx, move |this, _, cx| this.settle_pick(dismiss.0.clone(), dismiss.1.clone(), false, cx)), Role::Button, format!("Dismiss {}", pick.issue_reference)))
                            .into_any_element()
                    } else {
                        gpui::Empty.into_any_element()
                    };
                    let opened = pick.session_id.clone().filter(|_| !aside).map(Opened::Job);
                    let row = div()
                        .id(SharedString::from(format!("pick-{}", pick.pick_id)))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .py(look.length(token::SPACE_1))
                        .child(
                            div()
                                .id(SharedString::from(format!("pick-words-{}", pick.pick_id)))
                                .min_w_0()
                                .text_size(look.length(token::TEXT_XS))
                                .tooltip(tip(pick.reason.clone()))
                                .child(div().line_clamp(2).text_ellipsis().child(what.clone()))
                                .child(div().line_clamp(2).text_ellipsis().text_color(look.color(token::FG_MUTED)).child(pick.reason.clone())),
                        )
                        .child(acts)
                        .when_some(opened, |row, opened| row.cursor_pointer().on_click(cx.listener(move |_, _, _, cx| cx.emit(opened.clone()))));
                    body.push(labelled(row, Role::ListItem, what).into_any_element());
                }
            }
        }
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let since = self.away.autopilots.iter().map(|one| one.started_at.as_str()).min().and_then(|at| minute(at, now));
        let refused = self.away.refused.clone().filter(|_| self.away.setup.is_none()).map(|why| {
            labelled(div().id("away-refused").pt(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()), Role::Alert, why)
        });
        Some(
            labelled(
                div()
                    .id("while-away")
                    .flex_none()
                    .rounded(look.length(token::RADIUS_LG) * 1.25)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::OPERATOR_EDGE))
                    .bg(look.color(token::BG_RAISED))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_3))
                            .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
                            .child(glyph("plane-landing", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                            .child(labelled(div().id("while-away-title").font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_size(look.length(token::TEXT_SM)).child("While you were away"), Role::Heading, "While you were away"))
                            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).children(since.map(|since| format!("since {since}")))),
                    )
                    .child(div().flex().flex_col().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_1)).pb(look.length(token::SPACE_2)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).children(body).children(refused)),
                Role::Region,
                "While you were away",
            )
            .into_any_element(),
        )
    }

    /// A panel over the page, as the sheets draw one, its keys on `focus`.
    fn over(&self, id: &'static str, width: f32, focus: &FocusHandle, panel: gpui::Div, title: &'static str, cx: &Context<Self>, look: &Look) -> AnyElement {
        let shut = move |this: &mut OperatorPageView, window: &mut Window, cx: &mut Context<OperatorPageView>| {
            if this.away.setup.is_some() {
                this.close_setup(window, cx);
            } else {
                this.keep_away(window, cx);
            }
        };
        labelled(
            div()
                .id(SharedString::from(format!("{id}-over")))
                .track_focus(focus)
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if this.away.setup.is_some() {
                        this.setup_key(event, window, cx);
                    } else if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.keep_away(window, cx);
                    }
                }))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(look.color(token::SCRIM))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| shut(this, window, cx)))
                .child(
                    panel
                        .id(SharedString::from(format!("{id}-panel")))
                        .occlude()
                        .w(look.length(token::SPACE_12) * width)
                        .max_h(gpui::relative(0.9))
                        .flex()
                        .flex_col()
                        .rounded(round(look))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_OVERLAY))
                        .shadow(look.shadows(token::SHADOW_OVERLAY))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                ),
            Role::Dialog,
            title,
        )
        .into_any_element()
    }

    /// The setup over the page: projects, what it works and its limits on the left, what it answers on
    /// the right, then what it never does, Cancel and Start.
    pub(super) fn setup_sheet(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let setup = self.away.setup.as_ref()?;
        let rows = setup.rows();
        let line = |n: usize, row: SetupRow| {
            let on = n == setup.at;
            let frame = div()
                .id(SharedString::from(format!("setup-{n}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { look.color(token::OPERATOR_EDGE) } else { gpui::transparent_black() })
                .when(on, |row| row.bg(look.color(token::OPERATOR_FAINT)))
                .text_size(look.length(token::TEXT_SM));
            match row {
                SetupRow::Limit(limit) => {
                    let name = LIMITS.iter().find(|(one, _)| *one == limit).map_or("", |(_, name)| name);
                    let step = |by: i32, icon: &'static str| {
                        let id = format!("setup-{n}-{}", if by < 0 { "down" } else { "up" });
                        let button = press(id, "".into(), Some(icon), None, Weight::Bare, look).h(look.length(token::H_CONTROL_SM)).px(look.length(token::SPACE_1));
                        labelled(
                            pressed(button, cx, move |this, _, cx| {
                                cx.stop_propagation();
                                let now = this.clock_now.unwrap_or_else(chrono::Local::now);
                                if let Some(setup) = this.away.setup.as_mut() {
                                    setup.at = n;
                                    setup.nudge(limit, by, now);
                                }
                                cx.notify();
                            }),
                            Role::Button,
                            format!("{} {name}", if by < 0 { "Less" } else { "More" }),
                        )
                    };
                    let said = setup.said(limit);
                    labelled(
                        frame
                            .child(div().flex_1().text_color(look.color(token::FG_MUTED)).child(name))
                            .child(step(-1, "minus"))
                            .child(div().w(look.length(token::SPACE_12) + look.length(token::SPACE_4)).flex().justify_center().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(said.clone()))
                            .child(step(1, "plus")),
                        Role::Group,
                        format!("{name} {said}"),
                    )
                    .into_any_element()
                }
                row => {
                    let ticked = setup.ticked(row);
                    let (words, why) = match row {
                        SetupRow::Project(at) => (setup.projects[at].1.clone(), None),
                        SetupRow::Work(at) => (WORK[at].0.to_string(), Some(WORK[at].1)),
                        SetupRow::Answer(at) => (ANSWERS[at].1.to_string(), None),
                        SetupRow::Limit(_) => (String::new(), None),
                    };
                    let box_ = glyph(if ticked { "square-check" } else { "square" }, look.length(token::SPACE_4), look.color(if ticked { token::OPERATOR } else { token::FG_SUBTLE }));
                    labelled(
                        frame
                            .cursor_pointer()
                            .when_some(why, |row, why| row.tooltip(tip(why)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(setup) = this.away.setup.as_mut() {
                                    setup.at = n;
                                    setup.toggle(row);
                                }
                                window.focus(&this.away.setup_focus, cx);
                                cx.notify();
                            }))
                            .child(box_)
                            .child(div().flex_1().min_w_0().truncate().child(words.clone())),
                        Role::CheckBox,
                        if ticked { format!("{words}, ticked") } else { words },
                    )
                    .into_any_element()
                }
            }
        };
        let section = |title: &'static str| div().pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_1)).px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(title);
        let mut left = div().flex_1().min_w_0().flex().flex_col();
        let mut right = div().flex_1().min_w_0().flex().flex_col().child(section("What I answer for you"));
        let mut last: Option<&'static str> = None;
        for (n, row) in rows.iter().enumerate() {
            let head = match row {
                SetupRow::Project(_) => "Projects",
                SetupRow::Work(_) => "What I work",
                SetupRow::Limit(_) => "Limits",
                SetupRow::Answer(_) => "What I answer for you",
            };
            if let SetupRow::Answer(_) = row {
                right = right.child(line(n, *row));
                continue;
            }
            if last != Some(head) {
                last = Some(head);
                left = left.child(section(head));
            }
            left = left.child(line(n, *row));
        }
        let never = div()
            .id("setup-never")
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .mx(look.length(token::SPACE_4))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(glyph("shield", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
            .child(div().flex_1().min_w_0().child(NEVER));
        let never = labelled(never, Role::Group, "Never, whatever is ticked");
        let refused = self.away.refused.clone().map(|why| {
            labelled(div().id("setup-refused").px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(format!("Not started: {why}")), Role::Alert, format!("Not started: {why}"))
        });
        let until = setup.said(Limit::Until);
        let start = if self.away.starting {
            div().flex().items_center().gap(look.length(token::SPACE_2)).text_color(look.color(token::FG_MUTED)).child(spinner("away-start", look.length(token::SPACE_4), look.color(token::OPERATOR), cx)).child("Starting").into_any_element()
        } else {
            labelled(
                pressed(press("away-start", "Start".into(), Some("plane-takeoff"), Some("⏎".into()), Weight::Pick, look).tooltip(tip(format!("Answer and start work in the ticked projects until {until}"))), cx, |this, window, cx| this.start_away(window, cx)),
                Role::Button,
                "Start",
            )
            .into_any_element()
        };
        let cancel = labelled(pressed(press("away-cancel", "Cancel".into(), None, Some("Esc".into()), Weight::Bare, look), cx, |this, window, cx| this.close_setup(window, cx)), Role::Button, "Cancel");
        let panel = div()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(glyph("plane-takeoff", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .child(
                        div()
                            .flex_1()
                            .child(labelled(div().id("setup-title").text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("While you're away"), Role::Heading, "While you're away"))
                            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("The Operator answers and starts work for you until you come back, within what you tick here.")),
                    ),
            )
            .child(div().id("setup-list").flex_1().min_h_0().overflow_y_scroll().flex().gap(look.length(token::SPACE_4)).px(look.length(token::SPACE_2)).pb(look.length(token::SPACE_3)).child(left).child(right))
            .child(never)
            .children(refused)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .child(div().flex_1().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child("↑↓ move · Space ticks · ←→ change a limit"))
                    .child(cancel)
                    .child(start),
            );
        Some(self.over("setup", 16., &self.away.setup_focus, panel, "While you're away", cx, look))
    }

    /// Back? over the page when he returns to the window with Away still on.
    pub(super) fn back_prompt(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.away.back {
            return None;
        }
        let on = self.away.on();
        let places: Vec<String> = on.iter().map(|one| self.place_of(one.manifest_id.as_str())).collect();
        let answered = self.away.to_review.len();
        let started: u32 = on.iter().map(|one| one.sessions_started).sum();
        let did = match (answered, started) {
            (0, 0) => "It has not answered anything yet.".to_string(),
            (a, s) => format!("It answered {a} {} and started {s} {} while you were gone.", if a == 1 { "thing" } else { "things" }, if s == 1 { "Job" } else { "Jobs" }),
        };
        let keep = labelled(pressed(press("keep-away", "Keep it on".into(), None, Some("Esc".into()), Weight::Plain, look).tooltip(tip("It goes on answering for you")), cx, |this, window, cx| this.keep_away(window, cx)), Role::Button, "Keep it on");
        let back = labelled(pressed(press("im-back", "I'm back".into(), Some("plane-landing"), Some("⏎".into()), Weight::Pick, look).tooltip(tip("Stop answering for you: what it did stays under While you were away")), cx, |this, window, cx| this.come_back(window, cx)), Role::Button, "I'm back");
        let panel = div()
            .p(look.length(token::SPACE_4))
            .gap(look.length(token::SPACE_3))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(glyph("plane", look.length(token::SPACE_4), look.color(token::OPERATOR))).child(labelled(div().id("back-title").text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Back?"), Role::Heading, "Back?")))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(format!("Away is still on in {}. {did}", and_list(&places))))
            .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(keep).child(back));
        Some(self.over("back", 9., &self.away.back_focus, panel, "Back?", cx, look))
    }
}
