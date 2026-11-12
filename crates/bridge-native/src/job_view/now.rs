//! Now, alone in its pane: only what runs at this moment (his ruling of 12 Nov, a departure from
//! Armada Classic's `StoryColumn`, whose bare Now also lists what is owed him, walks, a plan he has
//! not approved and Ahead). Its band holds Ask for an update and Plan, with no lamp: the tab already
//! names the zone, and a dot alone said nothing. Under it stand the agent's row, Fleet at work, the
//! route step under way and the running Checks, then the agent's tasks in progress, the plan's
//! group being worked with its drones, the Jobs it started that still run, and a live watch. A Job
//! with nothing running draws nothing.
//!
//! The Spine's Now keeps Classic's beats: what is owed him (Fleet's `waiting_for` alone, one row per
//! item, where Classic mixes in the asked question and so draws it twice), the walks shown, the lamp,
//! a plan he has not approved and the failed step or Check. A finished task of the agent's own list
//! is not listed (his word, 10 Nov).
//!
//! Owed: the live page, which Fleet serves no frame for.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{AttachmentState, FleetActState, JobState, PlanTaskState, SessionCheckState, SessionPlanGroup, SessionRecord, SessionRow, SessionTodoState, StopReason, WaitingSource};

use super::acts::{button, Tone};
use super::head::{act_button, icon_button};
use super::issues::{job_card, job_placeholder};
use super::plan::task_look;
use super::pulls::{pull_of, PullChecks, PullState};
use super::rows::{glyph, mono, tip, Size};
use super::side::{Stands, ASK_FOR_UPDATE};
use super::workflow::{walk_step, StretchState, Work};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// The lamp at the band's left (`nowState`): the first that holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Lamp {
    Asks,
    Failed,
    Draft,
    Live,
}

impl Lamp {
    /// Its hue, its word, and whether it pulses (`MARK`).
    pub(super) fn look(self) -> (Token, &'static str, bool) {
        match self {
            Lamp::Asks => (token::STATUS_AWAITING_REVIEW, "Waiting on you", true),
            Lamp::Failed => (token::STATUS_COMPLETED_FAILED, "Failed", false),
            Lamp::Draft => (token::STATUS_AWAITING_REVIEW, "Drafting", true),
            Lamp::Live => (token::STATUS_RUNNING, "Live", false),
        }
    }
}

/// A step of the delivery route (`routeOf`), as its task mark reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Step {
    pub label: &'static str,
    pub state: PlanTaskState,
}

/// The route a pull request takes to `main`, and where each step stands now. A step is to come
/// until what it waits on arrives: a pull request, its Checks, the queue, the merge.
pub(super) fn route_of(record: &SessionRecord) -> Vec<Step> {
    let pulls: Vec<_> = record.attachments.iter().filter_map(|one| pull_of(record, one, None)).collect();
    let built: Vec<_> = pulls.iter().filter(|pull| pull.state != PullState::Draft).collect();
    let merged = !pulls.is_empty() && pulls.iter().all(|pull| pull.state == PullState::Merged);
    let open: Vec<_> = built.iter().filter(|pull| pull.state != PullState::Merged).collect();
    let pull = if pulls.is_empty() {
        PlanTaskState::Open
    } else if built.is_empty() {
        PlanTaskState::Working
    } else {
        PlanTaskState::Done
    };
    let ci = if merged {
        PlanTaskState::Done
    } else if built.is_empty() {
        PlanTaskState::Open
    } else if built.iter().any(|pull| pull.checks == PullChecks::Failed) {
        PlanTaskState::Failed
    } else if built.iter().any(|pull| pull.checks == PullChecks::Running) {
        PlanTaskState::Working
    } else {
        PlanTaskState::Done
    };
    let queue = if merged {
        PlanTaskState::Done
    } else if open.iter().any(|pull| pull.queued || pull.auto) {
        PlanTaskState::Working
    } else {
        PlanTaskState::Open
    };
    let main = if merged { PlanTaskState::Done } else { PlanTaskState::Open };
    [("Pull request", pull), ("ci", ci), ("Merge queue", queue), ("main", main)].into_iter().map(|(label, state)| Step { label, state }).collect()
}

impl JobView {
    /// A Mod Job's route (`routeOf`, kind `mod`): the mod made, its last Check, and one of its mods on.
    pub(super) fn mod_route(&self, job: &SessionRecord, cx: &gpui::App) -> Vec<Step> {
        let made = self.mods_made(job.id.as_str(), cx);
        let check = match self.check_rows(job.id.as_str(), cx).first() {
            Some(SessionRow::Check { state: ipc::SessionCheckState::Running, .. }) => PlanTaskState::Working,
            Some(SessionRow::Check { state: ipc::SessionCheckState::Passed, .. }) => PlanTaskState::Done,
            Some(SessionRow::Check { .. }) => PlanTaskState::Failed,
            _ => PlanTaskState::Open,
        };
        let done = |yes: bool| if yes { PlanTaskState::Done } else { PlanTaskState::Open };
        [("Mod", done(!made.is_empty())), ("Check", check), ("On", done(made.iter().any(|one| one.enabled)))].into_iter().map(|(label, state)| Step { label, state }).collect()
    }

    /// The route the Job's landing takes (`routeOf`, kind `landingOf`): a Mod Job's, the findings of a
    /// spike, a document or report and its reading, else a pull request's. A page waiting on his answer
    /// is still being written.
    pub(super) fn route(&self, job: &SessionRecord, cx: &gpui::App) -> Vec<Step> {
        if job.r#mod {
            return self.mod_route(job, cx);
        }
        let lands = self.work(job, cx).lands;
        let forms: &[&str] = if lands == Some(ipc::Lands::Report) { &["doc", "document", "file"] } else { &["doc", "document"] };
        let artifacts = || job.attachments.iter().filter(|one| one.kind == "artifact");
        let form = |one: &ipc::Attachment| one.detail.get("form").cloned().unwrap_or_default();
        let made = artifacts().any(|one| forms.contains(&form(one).as_str()));
        let waits = artifacts().filter(|one| form(one) == "document").any(|one| {
            let page = one.detail.get("page").cloned().unwrap_or_else(|| one.target.clone());
            job.waiting_for.iter().any(|item| item.source == ipc::WaitingSource::Document && item.id.strip_prefix("document:") == Some(page.as_str()))
        });
        let doc = if !made { PlanTaskState::Open } else if waits { PlanTaskState::Working } else { PlanTaskState::Done };
        let labels: &[&'static str] = match lands {
            Some(ipc::Lands::Spike) => &["Findings"],
            Some(ipc::Lands::Document | ipc::Lands::Report) => &["Document", "Read"],
            _ => return route_of(job),
        };
        labels.iter().enumerate().map(|(at, label)| Step { label, state: if at == 0 { doc } else { PlanTaskState::Open } }).collect()
    }
}

/// A step Armada names after something it reports is drawn as a reading (`SYSTEM`).
fn system(label: &str) -> bool {
    matches!(label, "ci" | "main")
}

/// Whole minutes since an instant, to the unit above an hour: `<1m`, `12m`, `1h 04m` (`spanSince`).
pub(super) fn span_since(since: &str, now: DateTime<Local>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(since).ok()?;
    let minutes = (now.with_timezone(&chrono::Utc) - then.with_timezone(&chrono::Utc)).num_minutes().max(0);
    Some(match minutes {
        0 => "<1m".to_string(),
        1..=59 => format!("{minutes}m"),
        _ => format!("{}h {:02}m", minutes / 60, minutes % 60),
    })
}

/// What a stopped Job's row says (`stoppedSaid`).
pub(super) fn stopped_said(stopped: &ipc::SessionStopped) -> String {
    match stopped.reason {
        StopReason::TurnEnded => "Stopped: ended its turn with nothing running".into(),
        StopReason::TasksOpen => "Stopped: ended its turn with tasks open".into(),
        StopReason::ProcessGone => "Stopped: process ended".into(),
        StopReason::FleetDown => {
            let clock = |at: &ipc::Instant| DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Local).format("%H:%M").to_string());
            match stopped.down.as_ref().and_then(|down| clock(&down.from).zip(clock(&down.to))) {
                Some((from, to)) => format!("Stopped: Fleet was down, {from} to {to}"),
                None => "Stopped: Fleet was down".into(),
            }
        }
    }
}

/// The line a run of tool calls that ends the thread reads as, while the turn runs (`detectedOf`).
pub(super) fn detected_of(rows: &[SessionRow], working: bool) -> Option<String> {
    if !working {
        return None;
    }
    let mut run: Vec<&str> = Vec::new();
    for row in rows.iter().rev() {
        match row {
            SessionRow::Tool { text, .. } => run.insert(0, text.as_str()),
            SessionRow::Message { .. } => break,
            _ if !run.is_empty() => break,
            _ => {}
        }
    }
    let line = if run.is_empty() { String::new() } else { super::calls::calls_line(&run, true) };
    (!line.is_empty()).then_some(line)
}

/// A glyph that names itself on hover, in the row's lead column (`Lead`).
pub(super) fn lead(look: &Look, key: &str, icon: &'static str, ink: Token, said: &str, pulse: bool) -> AnyElement {
    let mark = glyph(look, icon, Size::Small, look.color(ink));
    let mark = if pulse {
        mark.with_animation(
            SharedString::from(format!("now-pulse:{key}")),
            Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.35, 1.)),
            |mark, at| mark.opacity(at),
        )
        .into_any_element()
    } else {
        mark.into_any_element()
    };
    labelled(
        div().id(SharedString::from(format!("now-lead:{key}"))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip(said.to_string())).child(mark),
        Role::Image,
        said.to_string(),
    )
    .into_any_element()
}

/// A row's words: quiet unless the row is at work or calls him, a reading where the system named it.
pub(super) fn words(look: &Look, text: impl Into<SharedString>, ink: Token, strong: bool, reading: bool, wraps: bool) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .text_color(look.color(ink))
        .when(strong, |words| words.font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
        .when(reading, |words| words.font_family(mono(look)))
        .when(wraps, |words| words.line_clamp(2).text_ellipsis())
        .when(!wraps, |words| words.truncate())
        .child(text.into())
}

/// How long a call or an act has run, in the row's last column (`Since`).
fn since(look: &Look, key: &str, at: &str, said: &str) -> Option<AnyElement> {
    let span = span_since(at, Local::now())?;
    Some(
        labelled(
            div().id(SharedString::from(format!("now-since:{key}"))).flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip(said.to_string())).child(SharedString::from(span.clone())),
            Role::Image,
            format!("{said}: {span}"),
        )
        .into_any_element(),
    )
}

/// A row named to the accessibility tree, which needs an id to carry its label.
pub(super) fn named(key: &str, row: gpui::Div) -> gpui::Stateful<gpui::Div> {
    row.id(SharedString::from(format!("now-beat:{key}")))
}

/// One row (`.sv-beat`): the lead, the words, what stands last, and under them what the turn is doing.
pub(super) fn beat(look: &Look, lead: AnyElement, words: impl IntoElement, last: Option<AnyElement>, line: Option<(SharedString, Token)>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(div().flex().items_center().gap(look.length(token::SPACE_2)).min_h(look.length(token::H_ROW)).child(lead).child(words).children(last))
        .children(line.map(|(line, ink)| {
            div()
                .pl(look.length(token::SPACE_4) + look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(ink))
                .child(line)
        }))
}

/// A step's task mark: its glyph, hue and word, pulsing while it works (`Mark`).
/// A step of the workflow (`StepBeat`): its mark, which pulses on the step the Job is on, its name,
/// and what it made under it.
pub(super) fn step_beat(look: &Look, key: &str, name: &str, state: StretchState, under: Option<AnyElement>) -> AnyElement {
    let (icon, ink, said, about) = match state {
        StretchState::Done => ("circle-check", token::STEP_ADVANCED, "Done", "A workflow step, done"),
        StretchState::Now => ("circle-dot", token::STEP_RUNNING, "Now", "The workflow step the Job is on"),
        StretchState::Next => ("circle-dashed", token::FG_SUBTLE, "Next", "A workflow step still to come"),
    };
    let name = words(look, name.to_string(), if state == StretchState::Next { token::FG_MUTED } else { token::FG_DEFAULT }, false, false, false).id(SharedString::from(format!("{key}-said"))).tooltip(tip(about));
    let row = labelled(named(key, beat(look, lead(look, key, icon, ink, said, state == StretchState::Now), name, None, None)), Role::Group, about);
    div().flex().flex_col().child(row).children(under.map(|under| div().pl(look.length(token::SPACE_6)).child(under))).into_any_element()
}

fn step_lead(look: &Look, key: &str, state: PlanTaskState) -> AnyElement {
    let (icon, ink, said) = task_look(state);
    lead(look, key, icon, ink, said, state == PlanTaskState::Working)
}

/// A step of the route under way or to come (`RouteBeat`).
fn route_beat(look: &Look, step: Step) -> AnyElement {
    let ink = match step.state {
        PlanTaskState::Failed => token::STEP_FAILED,
        PlanTaskState::Open => token::FG_SUBTLE,
        _ => token::FG_DEFAULT,
    };
    let strong = matches!(step.state, PlanTaskState::Working | PlanTaskState::Failed);
    labelled(named(&format!("route:{}", step.label), beat(look, step_lead(look, &format!("route:{}", step.label), step.state), words(look, step.label, ink, strong, system(step.label), false), None, None)), Role::Group, step.label).into_any_element()
}

/// The walks shown to him that Fleet still holds an item for, the newest of each page (`walks`):
/// address, name, and the takes where it shows several.
pub(super) fn shown_walks(record: &SessionRecord, rows: &[SessionRow]) -> Vec<(String, String, Option<usize>)> {
    let held: Vec<&str> = record
        .waiting_for
        .iter()
        .filter(|one| one.source == WaitingSource::Walk)
        .filter_map(|one| one.act.as_ref().filter(|act| act.kind == ipc::WaitingActKind::Walk).map(|act| act.target.as_str()))
        .collect();
    let mut out = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        let SessionRow::Window { url, title, variations, ended_at: None, .. } = row else { continue };
        let again = rows[at + 1..].iter().any(|later| matches!(later, SessionRow::Window { url: other, ended_at: None, .. } if other == url));
        if held.contains(&url.as_str()) && !again {
            out.push((url.clone(), title.clone(), (!variations.is_empty()).then_some(variations.len())));
        }
    }
    out
}

/// A Job it started outside its plan that has not finished (`finishedChild`): Fleet has not ended it
/// (`session.dead`), and its row does not read done, ended or out of reach. An ended Job may carry no
/// row mark at all, so its own state is read first.
fn running_child(child: &SessionRecord) -> bool {
    child.state != ipc::SessionState::Ended && !matches!(child.mark.map(|mark| mark.state), Some(JobState::Done | JobState::Ended | JobState::Quiet))
}

/// A band's lamp (`StateMark`): a dot in the state's hue that names the state on hover, pulsing on
/// opacity alone where it calls him.
pub(super) fn lamp_mark(look: &Look, key: &str, hue: Token, said: &'static str, pulse: bool) -> AnyElement {
    let dot = div().id(SharedString::from(format!("{key}-lamp"))).flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(hue)).tooltip(tip(said));
    let dot = if pulse {
        dot.with_animation(SharedString::from(format!("{key}-lamp-pulse")), Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.35, 1.)), |dot, at| dot.opacity(at)).into_any_element()
    } else {
        dot.into_any_element()
    };
    labelled(div().id(SharedString::from(format!("{key}-lamp-mark"))).flex_none().child(dot), Role::Image, said).into_any_element()
}

impl JobView {
    /// Now's band alone in its pane: Sketch while a plan with a sketch is drafted, Ask for an update,
    /// and Plan once the plan is approved, at its end.
    fn now_band(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_ROW))
            .child(div().flex_1())
            .child(self.now_acts(id, look, cx))
            .into_any_element()
    }

    /// Now's acts: Sketch while a plan with a sketch is drafted, Ask for an update, and Plan once the
    /// plan is approved.
    pub(super) fn now_acts(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = self.record(id, cx);
        let plan = job.and_then(|job| job.plan.as_ref());
        let sketch = plan.filter(|plan| !plan.approved).and_then(|plan| plan.sketch.as_ref()).and_then(|sketch| self.sketch_card(id, &sketch.id, cx)).map(|card| {
            labelled(
                icon_button(look, "now-sketch", "pencil-ruler", "Sketch".into(), false, false).on_click(cx.listener(move |this, _, window, cx| this.read_sketch(&card, &card, window, cx))),
                Role::Button,
                "Sketch",
            )
        });
        let ask = (!self.closed(id, cx)).then(|| {
            labelled(
                icon_button(look, "side-ask-update", "message-circle-more", "Ask for an update".into(), false, false)
                    .on_click(cx.listener(|this, _, window, cx| this.send_line(ASK_FOR_UPDATE.to_string(), window, cx))),
                Role::Button,
                "Ask for an update",
            )
        });
        // On a Job in panes the whole plan opens as the Job's Plan tab.
        let whole = plan.filter(|plan| plan.approved).map(|_| {
            labelled(act_button(look, "now-plan", "list-tree", "whole", "Plan".into(), false, false).on_click(cx.listener(|this, _, window, cx| this.add_tab("plan", window, cx))), Role::Button, "Plan")
        });
        div().flex().items_center().gap(look.length(token::SPACE_1)).children(sketch).children(ask).children(whole).into_any_element()
    }

    /// Now alone in its pane: the band's acts and what runs, or nothing when nothing runs. The band
    /// draws no lamp here: the tab names the zone, and a dot alone said nothing (12 Nov).
    pub(super) fn side_now(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        // When nothing runs, Now draws nothing.
        let Some((_, Some(body))) = self.now_parts(id, false, look, cx) else { return div().into_any_element() };
        div().flex().flex_col().child(self.now_band(id, look, cx)).child(body).into_any_element()
    }

    /// Now's lamp and its rows, absent with none. On the Spine the route still to come stands in Open,
    /// as Classic's `aheadList` does outside a pane of its own.
    pub(super) fn now_parts(&self, id: &SharedString, spine: bool, look: &Look, cx: &Context<Self>) -> Option<(Option<Lamp>, Option<AnyElement>)> {
        let job = self.record(id, cx).cloned()?;
        // Now alone in its pane holds only what runs at this moment (his ruling, 12 Nov): the call,
        // tasks in progress, the group at work with its running subagents, Fleet at work, the Jobs it
        // started that run, running Checks and a live watch. What waits on him, what is finished and
        // what has not started leave it. The Spine's Now keeps Classic's beats.
        let only_now = !spine;
        let head = self.thread(id, cx).map(|held| held.head.clone());
        let rows = self.thread_rows(id, cx);
        let working = self.working(id, cx);
        let stuck = job.mark.is_some_and(|mark| mark.state == JobState::Stuck);
        let marks = self.plan_marks(id);
        let drafting = job.plan.as_ref().is_some_and(|plan| !plan.approved);
        let approved = job.plan.as_ref().is_some_and(|plan| plan.approved);
        let editing = job.plan.is_some() && (drafting || !marks.is_empty());
        let work = self.work(&job, cx);
        let gates = self.by_gates(id, &job);
        let after_plan = work.stepped() || gates;
        let place = super::side::placed(&job);
        let at_work: Vec<&SessionPlanGroup> = if editing { Vec::new() } else { job.plan.iter().filter(|plan| plan.approved).flat_map(|plan| plan.groups.iter()).filter(|group| place(group) == Stands::Working).collect() };

        let walks = shown_walks(&job, &rows);
        // What Fleet says he owes, as the band lists it (#80): a Done or a put-away takes the row out
        // of both; the plan being drafted is the lamp, and a walk shown is its own row.
        let hidden = self.dock_of(id).map(|dock| &dock.hidden);
        let owed: Vec<AnyElement> = self
            .needs_of(id, cx)
            .into_iter()
            .filter(|_| !only_now)
            .flat_map(|view| view.sections.iter())
            .filter(|section| section.kind != ipc::NeedsYouSectionKind::Done)
            .flat_map(|section| section.rows.iter())
            .filter(|row| !hidden.is_some_and(|hidden| hidden.contains(&row.id)))
            .filter(|row| !(drafting && row.source == WaitingSource::Plan))
            .filter(|row| !(row.source == WaitingSource::Walk && walks.iter().any(|(url, _, _)| row.id.strip_prefix("walk:") == Some(url.as_str()))))
            .map(|row| self.owed_row(row, look, cx))
            .collect();
        let walk_rows = walks.iter().filter(|_| !only_now).map(|(url, title, takes)| self.walk_beat(url, title, *takes, look, cx));

        // The agent's row says what Fleet saw: stopped and why, stuck on the call it has not come back
        // from, the call it is in and for how long, or what it detected. A Job doing nothing draws no row.
        let call = head.as_ref().and_then(|head| head.open_call.clone());
        let detected = detected_of(&rows, working && !self.closed(id, cx));
        let account = job.activity.as_ref().and_then(|activity| activity.account.as_ref()).and_then(|account| account.now.clone()).filter(|_| working && detected.is_none());
        let line_now = account.filter(|_| at_work.is_empty()).map(|line| (SharedString::from(line), token::STEP_RUNNING));
        let state = job.mark.map(|mark| mark.state);
        let waiting_on = state.filter(|_| detected.is_none()).filter(|state| matches!(state, JobState::Waiting | JobState::Held | JobState::Limited | JobState::Stuck | JobState::Failing | JobState::After)).map(|state| crate::jobs::glyph_of(state, None).1.to_string());
        let waiting_on = waiting_on.filter(|_| !only_now);
        let agent = if let Some(stopped) = job.stopped.as_ref().filter(|_| !only_now) {
            let asks = stopped.gave_up;
            let (icon, _) = crate::jobs::glyph_of(if asks { JobState::StoppedAsks } else { JobState::Stopped }, None);
            let ink = if asks { token::STATUS_AWAITING_REVIEW } else { token::FG_SUBTLE };
            let line = stopped.said.clone().map(|said| (SharedString::from(said), if asks { token::FG_MUTED } else { token::STEP_RUNNING }));
            Some((stopped_said(stopped), beat(look, lead(look, "agent", icon, ink, "Stopped", false), words(look, stopped_said(stopped), if asks { token::FG_DEFAULT } else { token::FG_MUTED }, asks, false, false), None, line)))
        } else if stuck {
            let quiet = job.last_heard_at.as_ref().and_then(|at| span_since(at.as_str(), Local::now())).map(|span| (SharedString::from(format!("No word for {span}")), token::FG_MUTED));
            let name = call.as_ref().map_or_else(|| "Stuck".to_string(), |call| call.text.clone());
            Some((name.clone(), beat(look, lead(look, "agent", "clock", token::STATUS_AWAITING_REVIEW, "Stuck", false), words(look, name, token::FG_DEFAULT, true, call.is_some(), false), None, quiet)))
        } else if let Some(call) = call.as_ref().filter(|_| working) {
            let (icon, ink, said) = task_look(PlanTaskState::Working);
            Some((call.text.clone(), beat(look, lead(look, "agent", icon, ink, said, false), words(look, call.text.clone(), token::FG_DEFAULT, true, true, false), since(look, "agent", call.since.as_str(), "On this call for"), line_now.clone())))
        } else if detected.is_some() || waiting_on.is_some() || line_now.is_some() || working {
            let (icon, ink, said) = task_look(if working { PlanTaskState::Working } else { PlanTaskState::Open });
            let strong = detected.is_some();
            let name = detected.clone().or_else(|| waiting_on.clone()).unwrap_or_else(|| "Agent".to_string());
            Some((name.clone(), beat(look, lead(look, "agent", icon, ink, said, false), words(look, name, if strong { token::FG_DEFAULT } else { token::FG_MUTED }, strong, false, false), None, line_now.clone())))
        } else {
            None
        };
        let agent = agent.map(|(name, row)| labelled(named("agent", row), Role::Group, name));

        // Fleet at work on the Job, one more worker beside the agent.
        let fleet = head.iter().flat_map(|head| head.fleet_acts.iter()).filter(|act| act.state == FleetActState::UnderWay).map(|act| {
            let key = format!("fleet:{}", act.id);
            let said = words(look, act.said.clone(), token::FG_MUTED, false, false, false).id(SharedString::from(format!("now-fleet-said:{}", act.id)));
            let said = match act.why.clone() {
                Some(why) => said.tooltip(tip(why)).into_any_element(),
                None => said.into_any_element(),
            };
            labelled(named(&key, beat(look, lead(look, &key, "server", token::FG_SUBTLE, "Fleet", false), said, since(look, &key, act.at.as_str(), "Under way for"), None)), Role::Group, act.said.clone()).into_any_element()
        });
        // A step held on a state an instance has yet to reach: a fact, so the clock does not pulse.
        let waits = job.workflow_waiting_on.iter().filter(|_| !only_now).map(|said| labelled(named(&format!("wait:{said}"), beat(look, lead(look, &format!("wait:{said}"), "clock", token::FG_SUBTLE, "Waiting", false), words(look, said.clone(), token::FG_MUTED, false, false, true), None, None)), Role::Group, said.clone()).into_any_element());
        let route = self.route(&job, cx);
        // A step of the route the workflow already names is that step, not a second row; a plan that
        // ends on gates says where it is in its stretches instead.
        let under_way = route
            .iter()
            .filter(|step| !gates && (step.state == PlanTaskState::Working || (!only_now && step.state == PlanTaskState::Failed)) && !work.names(step.label))
            .map(|step| route_beat(look, *step));
        let checks: Vec<(&String, SessionCheckState, String)> = rows.iter().filter_map(|row| if let SessionRow::Check { id, name, state, .. } = row { Some((name, *state, id.clone())) } else { None }).collect();
        let failed_check = checks.last().filter(|(_, state, _)| matches!(state, SessionCheckState::Failed | SessionCheckState::TimedOut)).cloned();
        let check_rows = checks
            .iter()
            .filter(|(_, state, _)| *state == SessionCheckState::Running)
            .map(|(name, _, row)| (name, PlanTaskState::Working, row))
            .chain(failed_check.as_ref().filter(|_| !only_now).map(|(name, _, row)| (name, PlanTaskState::Failed, row)))
            .map(|(name, state, row)| labelled(named(&format!("check:{row}"), beat(look, step_lead(look, &format!("check:{row}"), state), words(look, (*name).clone(), token::FG_MUTED, false, true, false), None, None)), Role::Group, (*name).clone()).into_any_element())
            .collect::<Vec<_>>();

        let asked = head.as_ref().is_some_and(|head| head.asked.is_some());
        let broke = approved && job.plan.iter().flat_map(|plan| plan.groups.iter()).any(|group| group.tasks.iter().any(|task| task.state == PlanTaskState::Failed));
        let lamp = if only_now {
            None
        } else if !owed.is_empty() || !walks.is_empty() || asked {
            Some(Lamp::Asks)
        } else if broke || failed_check.is_some() {
            Some(Lamp::Failed)
        } else if drafting {
            Some(Lamp::Draft)
        } else if (working && !stuck) || !at_work.is_empty() {
            Some(Lamp::Live)
        } else {
            None
        };

        let beats: Vec<AnyElement> = owed.into_iter().chain(walk_rows).chain(agent.map(|agent| agent.into_any_element())).chain(fleet).chain(waits).chain(under_way).chain(check_rows).collect();
        let beats = (!beats.is_empty()).then(|| labelled(div().id("now-beats").flex().flex_col().when(only_now, |beats| beats.gap(look.length(token::SPACE_1))).children(beats), Role::List, "Now").into_any_element());
        // With a workflow and a plan group being worked, the agent's list follows the plan's tasks; a
        // plan that ends on gates draws it under the live stretch's tasks instead.
        let list_after = after_plan && !at_work.is_empty();
        let list = if gates { None } else { self.now_tasks(&job, look, only_now) };
        let (list_before, list_after) = if list_after { (None, list) } else { (list, None) };
        let flight = if gates { None } else { self.flight_rows(&job, &at_work, false, only_now, look, cx) };
        let helpers = if gates { self.helpers(&job, look, cx) } else { None };
        let rows: Vec<AnyElement> = beats
            .into_iter()
            .chain(list_before)
            .chain(self.now_subagents(&job, look))
            .chain(flight)
            .chain(helpers)
            .chain(self.now_children(&job, look, cx))
            .chain(list_after)
            .chain(self.now_plan(id, &job, look, cx).filter(|_| !only_now))
            .chain(self.watching(&job, look, cx).filter(|_| !only_now || self.watches_of(id, cx).iter().any(|watch| watch.live)).map(|watching| watching.into_any_element()))
            .collect();
        let body = (!rows.is_empty()).then(|| div().flex().flex_col().when(only_now, |body| body.gap(look.length(token::SPACE_2))).children(rows).into_any_element());
        Some((lamp, body))
    }

    /// What is still to come (`aheadList`): the workflow's steps where the Job is at none of them,
    /// a walk to come under a step that asks for one, then the route.
    pub(super) fn ahead_list(&self, job: &SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let work = self.work(job, cx);
        let walked = !shown_walks(job, &self.thread_rows(job.id.as_str(), cx)).is_empty();
        let steps = work.steps.iter().enumerate().filter(|(at, _)| work.at.is_none_or(|now| *at > now)).map(|(at, name)| {
            let under = (walk_step(name) && !walked).then(|| JobView::walk_to_come(look));
            step_beat(look, &format!("ahead:{at}"), name, StretchState::Next, under)
        });
        let ahead: Vec<AnyElement> = steps.chain(self.route_ahead(job, &work, look, cx)).collect();
        (!ahead.is_empty()).then(|| labelled(div().id("now-ahead").flex().flex_col().children(ahead), Role::List, "Ahead").into_any_element())
    }

    /// The route's steps still to come that the workflow does not name already.
    fn route_ahead(&self, job: &SessionRecord, work: &Work, look: &Look, cx: &gpui::App) -> Vec<AnyElement> {
        let route = self.route(job, cx);
        route.iter().filter(|step| step.state == PlanTaskState::Open && !work.names(step.label)).map(|step| route_beat(look, *step)).collect()
    }

    /// What was to land, under the workflow's last step: the route's outputs still to be made (`landing`).
    pub(super) fn landing(&self, job: &SessionRecord, work: &Work, look: &Look, cx: &gpui::App) -> Option<AnyElement> {
        let ahead = self.route_ahead(job, work, look, cx);
        (!ahead.is_empty()).then(|| labelled(div().id("landing").flex().flex_col().children(ahead), Role::List, "Ahead").into_any_element())
    }
}

impl JobView {
    /// A walk shown to him (`WalkBeat`): the lamp pulses, its name opens it, and a tick stands for
    /// each take, never counted in words.
    fn walk_beat(&self, url: &str, title: &str, takes: Option<usize>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = url.to_string();
        let name = labelled(
            words(look, title.to_string(), token::FG_DEFAULT, true, false, false)
                .id(SharedString::from(format!("now-walk:{url}")))
                .cursor_pointer()
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .on_click(cx.listener(move |this, _, window, cx| this.press_walk(&key, window, cx))),
            Role::Button,
            format!("Walk: {title}"),
        );
        let ticks = takes.map(|takes| {
            labelled(
                div()
                    .id(SharedString::from(format!("now-takes:{url}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .tooltip(tip("Takes"))
                    .children((0..takes).map(|_| glyph(look, "circle", Size::Small, look.color(token::FG_SUBTLE)))),
                Role::Image,
                "Takes",
            )
            .into_any_element()
        });
        beat(look, lead(look, &format!("walk:{url}"), "footprints", token::STATUS_AWAITING_REVIEW, "Walk waiting on you", true), name, ticks, None).into_any_element()
    }

    /// One thing waiting on him: a press opens it where it is answered.
    fn owed_row(&self, item: &ipc::NeedsYouRow, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.id.clone();
        let source = item.source;
        labelled(
            div()
                .id(SharedString::from(format!("owed:{}", item.id)))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_ROW))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(div().id(SharedString::from(format!("owed-mark:{}", item.id))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().pt(look.length(token::SPACE_1) / 2.).tooltip(tip(item.glyph.said.clone())).child(glyph(look, super::dock::icon_of(item.glyph.kind), Size::Small, look.color(token::STATUS_AWAITING_REVIEW))))
                .child(div().flex_1().min_w_0().line_clamp(2).child(SharedString::from(item.text.clone())))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .on_click(cx.listener(move |this, _, window, cx| this.open_owed(&key, source, window, cx))),
            Role::Button,
            format!("Waiting on you: {}", item.text),
        )
        .into_any_element()
    }

    fn open_owed(&mut self, item: &str, source: WaitingSource, window: &mut gpui::Window, cx: &mut Context<Self>) {
        match source {
            // A draft opens its card as a tab, and a page owed alone is read in its own (`openOwedDocument`).
            WaitingSource::Draft => self.open_draft_tab(item.strip_prefix("draft:").unwrap_or(item), window, cx),
            WaitingSource::Document if self.shown.as_ref().and_then(|id| self.sole_document(id, cx)).is_some_and(|page| item.strip_prefix("document:") == Some(page.as_str())) => {
                self.open_document_tab(item.strip_prefix("document:").unwrap_or(item), window, cx);
            }
            WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document => {
                self.open_review(window, cx);
                self.review_at(item, window, cx);
            }
            WaitingSource::FirstMessage | WaitingSource::Done => {}
            _ => {
                // A question's item is `ask:<call>`, which the panel holds as `question:<call>`.
                let key = item.strip_prefix("ask:").map_or_else(|| format!("item:{item}"), |call| format!("question:{call}"));
                self.set_needs(true, Some(key), window, cx);
            }
        }
    }

    /// The agent's own list, open and working items only; a Job that stopped lists the tasks it
    /// stopped on, open.
    pub(super) fn now_tasks(&self, job: &SessionRecord, look: &Look, only_now: bool) -> Option<AnyElement> {
        let left = job.stopped.as_ref().filter(|_| !only_now).map(|stopped| stopped.tasks.clone()).unwrap_or_default();
        let todos: Vec<(String, PlanTaskState)> = if left.is_empty() {
            job.activity
                .iter()
                .flat_map(|activity| activity.todos.iter())
                .filter(|todo| todo.task.is_none() && todo.state != SessionTodoState::Done)
                .filter(|todo| !only_now || todo.state == SessionTodoState::Doing)
                .map(|todo| (todo.text.clone(), if todo.state == SessionTodoState::Doing { PlanTaskState::Working } else { PlanTaskState::Open }))
                .collect()
        } else {
            left.into_iter().map(|text| (text, PlanTaskState::Open)).collect()
        };
        if todos.is_empty() {
            return None;
        }
        let rows = todos.into_iter().enumerate().map(|(at, (text, state))| {
            let ink = if state == PlanTaskState::Working { token::FG_DEFAULT } else { token::FG_MUTED };
            beat(look, step_lead(look, &format!("todo:{at}"), state), words(look, text, ink, false, false, true), None, None)
        });
        Some(labelled(div().id("now-tasks").flex().flex_col().when(only_now, |tasks| tasks.gap(look.length(token::SPACE_1))).children(rows), Role::List, "Agent tasks").into_any_element())
    }

    /// The subagents at work on a Job with no plan (`SubagentsAtWork`): each its task and its type.
    fn now_subagents(&self, job: &SessionRecord, look: &Look) -> Option<AnyElement> {
        if job.plan.is_some() {
            return None;
        }
        let rows: Vec<_> = job
            .attachments
            .iter()
            .filter(|one| one.kind == "subagent" && one.state == AttachmentState::Standing)
            .map(|one| {
                let subagent = super::ledger::subagent_of(one);
                let kind = subagent.agent_type.clone().map(|kind| div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(kind));
                labelled(named(&format!("subagent:{}", one.target), beat(look, step_lead(look, &format!("subagent:{}", one.target), PlanTaskState::Working), words(look, subagent.task.clone(), token::FG_DEFAULT, false, false, false), kind.map(IntoElement::into_any_element), None)), Role::Group, subagent.task).into_any_element()
            })
            .collect();
        (!rows.is_empty()).then(|| labelled(div().id("now-subagents").flex().flex_col().children(rows), Role::List, "Subagents").into_any_element())
    }

    /// A plan's groups (`FlightRows`), each its drones beside it and, open, its issue, its tasks and its
    /// pull requests. Those being worked stand open until pressed shut; `calm`, the groups still to come,
    /// stand shut until pressed, read as titles. A group handed to a Job is that Job's card, and one
    /// going to a Job not started yet is its placeholder card.
    /// `only_now`, in Now alone: a group's finished drones, its tasks not under way, its issue and pull
    /// requests, and a Job it is handed to that is not running, leave it.
    pub(super) fn flight_rows(&self, job: &SessionRecord, groups: &[&SessionPlanGroup], calm: bool, only_now: bool, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if groups.is_empty() {
            return None;
        }
        let rows: Vec<AnyElement> = groups.iter().filter_map(|group| {
            if let Some(child) = job.attachments.iter().find(|one| one.kind == "started_to" && one.detail.get("group") == Some(&group.id)).and_then(|one| self.record(&one.target, cx)) {
                return (!only_now || running_child(child)).then(|| job_card(child, look, cx));
            }
            let issue = group.issue.as_ref().and_then(|number| job.attachments.iter().find(|one| one.kind == "issue" && one.target == *number)).filter(|_| !only_now);
            if group.own_job {
                return (!only_now).then(|| job_placeholder(&group.id, &group.title, issue.map(|one| super::issues::issue_of(job, one)), look, cx));
            }
            let shut_key = if calm { format!("open-open:{}", group.id) } else { format!("now-shut:{}", group.id) };
            let open = self.side_unfolded(&shut_key) == calm;
            let drones = job.attachments.iter().filter(|one| one.kind == "subagent" && one.detail.get("group") == Some(&group.id)).filter(|one| !only_now || one.state == AttachmentState::Standing).map(|one| {
                let subagent = super::ledger::subagent_of(one);
                let running = one.state == AttachmentState::Standing;
                let said = if running { format!("Subagent at work on {}", subagent.task) } else { format!("Subagent, finished: {}", subagent.task) };
                let target = one.target.clone();
                labelled(
                    div()
                        .id(SharedString::from(format!("now-drone:{}", one.target)))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::H_CONTROL_SM))
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .tooltip(tip(said.clone()))
                        .child(glyph(look, "drone", Size::Mark, look.color(if running { token::STEP_RUNNING } else { token::FG_SUBTLE })))
                        .tab_index(0)
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .on_click(cx.listener(move |this, _, window, cx| this.open_subagent_tab(&target, window, cx))),
                    Role::Button,
                    said,
                )
            });
            let track = (!only_now && !open && !group.tasks.is_empty()).then(|| super::done::task_track(&group.id, &group.tasks, look));
            let (icon, ink, said) = task_look(if calm { PlanTaskState::Open } else { PlanTaskState::Working });
            let row = labelled(
                div()
                    .id(SharedString::from(format!("now-group:{}", group.id)))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_ROW))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(lead(look, &format!("group:{}", group.id), icon, ink, said, !calm))
                    .child(div().min_w_0().truncate().child(SharedString::from(group.title.clone())))
                    .child(div().flex_1())
                    .children(track)
                    .tab_index(0)
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.side_fold(&shut_key);
                        cx.notify();
                    })),
                Role::Button,
                format!("Plan group: {}", group.title),
            );
            let body = open.then(|| {
                let tasks = group.tasks.iter().enumerate().filter(|(_, task)| !only_now || task.state == PlanTaskState::Working).map(|(at, task)| {
                    let ink = if task.state == PlanTaskState::Working { token::FG_DEFAULT } else { token::FG_MUTED };
                    beat(look, step_lead(look, &format!("task:{}:{at}", group.id), task.state), words(look, task.text.clone(), ink, false, false, true), None, None)
                });
                let pulls: Vec<_> = issue
                    .map(|issue| job.attachments.iter().filter(|one| super::done::works(one, issue)).filter_map(|one| pull_of(job, one, None)).collect())
                    .unwrap_or_default();
                let pulls = (!pulls.is_empty()).then(|| {
                    let rows = pulls.into_iter().map(|pull| self.pull_row(super::pulls::pull_line(pull.clone()), pull, "pull_request", look, cx));
                    labelled(div().id(SharedString::from(format!("now-pulls:{}", group.id))).flex().flex_col().children(rows), Role::List, "Pull requests")
                });
                div()
                    .flex()
                    .flex_col()
                    .pl(look.length(token::SPACE_6))
                    .children(issue.map(|one| self.issue_row(super::issues::issue_of(job, one), look, cx)))
                    .child(labelled(div().id(SharedString::from(format!("now-tasks:{}", group.id))).flex().flex_col().children(tasks), Role::List, "Tasks"))
                    .children(pulls)
            });
            Some(div().flex().flex_col().child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(row).children(drones)).children(body).into_any_element())
        }).collect();
        (!rows.is_empty()).then(|| labelled(div().id("now-groups").flex().flex_col().when(!only_now, |groups| groups.pt(look.length(token::SPACE_2))).when(only_now, |groups| groups.gap(look.length(token::SPACE_1))).children(rows), Role::List, "Groups").into_any_element())
    }

    /// The Jobs it started outside its plan that still run, each its card.
    fn now_children(&self, job: &SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let cards: Vec<_> = job
            .attachments
            .iter()
            .filter(|one| one.kind == "started_to" && !one.detail.contains_key("group"))
            .filter_map(|one| self.record(&one.target, cx))
            .filter(|child| running_child(child))
            .map(|child| job_card(child, look, cx))
            .collect();
        (!cards.is_empty()).then(|| labelled(div().id("now-jobs").flex().flex_col().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_2)).children(cards), Role::List, "Jobs").into_any_element())
    }

    /// A plan he has not approved, or one he marked, edited in place with its answers.
    fn now_plan(&self, id: &SharedString, job: &SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let marks = self.plan_marks(id);
        let plan = job.plan.clone().filter(|plan| !plan.approved || !marks.is_empty()).filter(|_| !self.review_open())?;
        let send = labelled(
            button(look, "side-plan-send", "Send changes", Tone::Secondary, false, None, !marks.is_empty())
                .when(!marks.is_empty(), |send| send.on_click(cx.listener(|this, _, window, cx| this.side_plan(false, window, cx)))),
            Role::Button,
            "Send the plan's changes",
        );
        let approve = labelled(
            button(look, "side-plan-approve", "Approve", Tone::Primary, false, None, true)
                .tooltip(tip(if marks.is_empty() { "Approve the plan" } else { "Approve with these changes" }))
                .on_click(cx.listener(|this, _, window, cx| this.side_plan(true, window, cx))),
            Role::Button,
            "Approve the plan",
        );
        Some(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_3))
                .child(self.plan_editor(id, &plan, None, true, look, cx))
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(send).child(approve))
                .into_any_element(),
        )
    }

    /// Approve or Send changes from Now, with what he marked.
    fn side_plan(&mut self, approve: bool, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let Some((job, plan)) = self.plan_of_shown(cx) else { return };
        let marks = self.take_marks(&job);
        let session_id = ipc::SessionId::carried(job.to_string());
        let (route, body) = if approve && marks.is_empty() {
            ("/sessions/plan/approve", ipc::encode(&ipc::ApprovePlan { session_id, version: plan.version }))
        } else {
            ("/sessions/plan/changes", ipc::encode(&ipc::SendPlanChanges { session_id, version: plan.version, marks, approve }))
        };
        self.post_with(job, route, body.map_err(|why| format!("{why:?}")), |_, _, _| {}, window, cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_started_job_fleet_ended_is_not_running_with_or_without_a_row_mark() {
        let jobs = crate::mock::jobs("job-now").expect("the fixture decodes");
        let of = |id: &str| jobs.iter().find(|job| job.id.as_str() == id).expect("in the fixture");
        assert!(running_child(of("mock-09")));
        for ended in ["mock-15", "mock-16", "mock-17"] {
            assert!(!running_child(of(ended)), "{ended} has ended");
        }
        assert!(!running_child(of("mock-12")), "out of reach");
    }
}
