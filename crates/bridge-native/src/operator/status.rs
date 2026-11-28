//! The status card: what stands in the front card's place while nothing needs him, so a quiet page reads as the Operator
//! working, watching, stopped, not started yet or switched off. Stopped stands in front of any queued card, in red, until
//! Restart brings the Operator back. It reads the Operator's own session record from Fleet's held Jobs
//! (`operator: true`) and derives nothing Fleet has not said: the state comes from `turn_since`, `standing`, `state` and
//! `stopped`, the doing line from `activity`.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::operator_page::OperatorFeedEntry;
use ipc::{SessionRecord, SessionStanding, SessionState, StopReason};

use super::card::card_frame;
use super::face;
use super::page::OperatorPageView;
use super::view::{chip, press, pressed, Weight};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{glyph, spinner, tip};

/// The Operator's own session among Fleet's held Jobs: the live one, else the one Fleet heard from last.
pub fn session_of(jobs: &[SessionRecord]) -> Option<&SessionRecord> {
    let heard = |job: &SessionRecord| DateTime::parse_from_rfc3339(job.last_seen_at.as_str()).ok();
    jobs.iter().filter(|job| job.operator).max_by_key(|job| (job.state == SessionState::Live, heard(job)))
}

/// Where the Operator's session stands, as its record says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// A turn is running: when it began, and what Fleet saw the session doing.
    Working { since: String, doing: Option<String> },
    /// Between turns: when Fleet last heard from it.
    Watching { heard: String },
    /// Not running: when it stopped, and why in plain words.
    Stopped { at: String, why: String },
    /// The switch is on and no session exists yet.
    Starting,
    /// `features.operator` is off: every Job asks him directly.
    Off,
}

/// The state a session record reads as. No record is a session not started.
pub fn status_of(session: Option<&SessionRecord>) -> Status {
    let Some(job) = session else { return Status::Starting };
    if job.standing == Some(SessionStanding::Stopped) || job.state != SessionState::Live || job.stopped.is_some() {
        let at = job.stopped.as_ref().map(|stopped| stopped.at.as_str()).or(job.ended_at.as_ref().map(|at| at.as_str())).unwrap_or(job.last_seen_at.as_str());
        return Status::Stopped { at: at.to_string(), why: why_stopped(job) };
    }
    if let Some(since) = &job.turn_since {
        let doing = job.activity.as_ref().and_then(|activity| activity.now.clone().or_else(|| activity.account.as_ref().and_then(|account| account.now.clone()))).filter(|doing| !doing.is_empty());
        return Status::Working { since: since.as_str().to_string(), doing };
    }
    let heard = job.last_heard_at.as_ref().or(job.last_turn_at.as_ref()).unwrap_or(&job.last_seen_at);
    Status::Watching { heard: heard.as_str().to_string() }
}

/// What stopped it, in plain words.
fn why_stopped(job: &SessionRecord) -> String {
    match (&job.stopped, job.state) {
        (Some(stopped), _) => match stopped.reason {
            StopReason::TurnEnded => "It ended its turn with nothing running".into(),
            StopReason::TasksOpen => "It ended its turn with tasks open".into(),
            StopReason::ProcessGone => "Its process ended".into(),
            StopReason::FleetDown => "Fleet was down".into(),
        },
        (None, SessionState::Ended) => job.end_reason.clone().filter(|why| !why.is_empty()).unwrap_or_else(|| "Its session ended".into()),
        (None, SessionState::Live) => "Nothing is running".into(),
    }
}

/// How long since `at`: `<1m`, `5m`, `1h 05m`.
fn span(at: &str, now: DateTime<Local>) -> String {
    let Ok(then) = DateTime::parse_from_rfc3339(at) else { return String::new() };
    let minutes = (now.with_timezone(&chrono::Utc) - then.with_timezone(&chrono::Utc)).num_minutes().max(0);
    match minutes {
        0 => "<1m".into(),
        1..=59 => format!("{minutes}m"),
        _ => format!("{}h {:02}m", minutes / 60, minutes % 60),
    }
}

/// How long ago `at` was: `5m ago`.
fn ago(at: &str, now: DateTime<Local>) -> String {
    crate::job_view::age(at, now).unwrap_or_else(|| "a while ago".into())
}

impl Status {
    /// The card's title: the Operator stopped, or nothing needs him.
    pub fn title(&self, now: DateTime<Local>) -> String {
        match self {
            Status::Stopped { at, .. } => format!("The Operator stopped {}", ago(at, now)),
            Status::Off => "The Operator is off".into(),
            _ => "Nothing needs you".into(),
        }
    }

    /// The state chip: its word, what its tooltip says, and its hue and ground.
    pub fn chip(&self) -> (&'static str, &'static str, Token, Token) {
        match self {
            Status::Working { .. } => ("Working", "The Operator is in a turn", token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
            Status::Watching { .. } => ("Watching", "The Operator is between turns and waits for the next thing", token::ACCENT, token::ACCENT_MUTED),
            Status::Stopped { .. } => ("Stopped", "The Operator's session is not running", token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG),
            Status::Starting => ("Starting", "Fleet has not started the Operator's session yet", token::FG_MUTED, token::BG_HOVER),
            Status::Off => ("Off", "Nothing is handed to the Operator and it makes no calls", token::FG_MUTED, token::BG_HOVER),
        }
    }

    /// The line under the head: what it is doing, when it last spoke, why it stopped, or that it starts.
    pub fn line(&self, now: DateTime<Local>) -> String {
        match self {
            Status::Working { since, doing } => format!("{}, for {}", doing.as_deref().unwrap_or("In a turn"), span(since, now)),
            Status::Watching { heard } => format!("Between turns. Last heard from {}", ago(heard, now)),
            Status::Stopped { why, .. } => why.clone(),
            Status::Starting => "Fleet is starting its session".into(),
            Status::Off => "Every Job asks you directly".into(),
        }
    }

    /// The glyph beside the line and its tooltip, where the state is not a turning ring.
    fn mark(&self) -> (&'static str, &'static str) {
        match self {
            Status::Watching { .. } => ("eye", "Between turns"),
            Status::Stopped { .. } => ("circle-alert", "Stopped"),
            Status::Working { .. } => ("loader-circle", "In a turn"),
            Status::Starting => ("loader-circle", "Starting"),
            Status::Off => ("power", "Off"),
        }
    }
}

/// The last act line: the feed's newest entry, with its age, as `Last act 5m ago: Started a Job`.
pub fn last_act(entry: &OperatorFeedEntry, now: DateTime<Local>) -> String {
    format!("Last act {}: {}", ago(entry.at.as_str(), now), feed_title(entry))
}

fn feed_title(entry: &OperatorFeedEntry) -> String {
    match (&entry.kind, &entry.number) {
        (ipc::operator_page::OperatorFeedKind::Merged, Some(number)) => format!("#{number} {}", entry.title),
        _ => entry.title.clone(),
    }
}

impl OperatorPageView {
    /// The card in the front card's place: the Operator's state, what it is doing, and its last act.
    pub(super) fn status_card(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let status = self.status();
        let stopped = matches!(status, Status::Stopped { .. });
        let danger = look.color(token::STATUS_COMPLETED_FAILED);
        let title = status.title(now);
        let (word, said, hue, ground) = status.chip();
        let state = labelled(chip(word, look.color(hue), look.color(ground), look).id("status-chip").tooltip(tip(said)), Role::Status, word);
        let off = status == Status::Off;
        // Nothing here opens the Operator's session as a Job: its thread and box are the page's own.
        let button = if off {
            Some(self.switch_on_press(look, cx))
        } else if stopped && self.session_id().is_some() {
            let restart = press("status-restart", "Restart".into(), Some("rotate-cw"), None, Weight::Danger, look).tooltip(tip("Switch the Operator off and on, so Fleet starts a new session"));
            Some(labelled(pressed(restart, cx, |this, _, cx| this.restart(cx)), Role::Button, "Restart the Operator").into_any_element())
        } else {
            None
        };
        let head = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(look.color(if stopped { token::STATUS_COMPLETED_FAILED_BG } else { token::OPERATOR_FAINT }))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if stopped { token::STATUS_COMPLETED_FAILED } else { token::OPERATOR_EDGE }))
            .child(glyph("bot", look.length(token::SPACE_4) + look.length(token::SPACE_1), look.color(if off { token::FG_MUTED } else { token::OPERATOR })))
            .child(div().flex_none().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
            // Embedded, the page's header row already draws the chip and the line: the card does not say them again.
            .children((!self.embedded).then_some(state))
            .child(div().flex_1())
            .children(button);
        let (mark, mark_said) = status.mark();
        let ink = if stopped { token::STATUS_COMPLETED_FAILED } else { hue };
        let size = look.length(token::SPACE_4);
        let mark_element = match status {
            Status::Working { .. } | Status::Starting => spinner("status-spin", size, look.color(ink), cx).into_any_element(),
            _ => glyph(mark, size, look.color(ink)).into_any_element(),
        };
        let line = status.line(now);
        let doing = labelled(
            div()
                .id("status-line")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .bg(look.color(ground))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(if stopped { token::STATUS_COMPLETED_FAILED } else { token::FG_DEFAULT }))
                .child(div().id("status-mark").flex_none().flex().items_center().tooltip(tip(mark_said)).child(mark_element))
                .child(div().flex_1().min_w_0().child(line.clone())),
            Role::Status,
            line,
        );
        let last = self.page.as_ref().and_then(|page| page.feed.first()).map(|entry| {
            let (mark, hue, word) = face::feed_kind(entry.kind);
            let said = last_act(entry, now);
            labelled(
                div()
                    .id("status-last-act")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(div().id("status-last-mark").flex_none().flex().items_center().tooltip(tip(word)).child(glyph(mark, look.length(token::SPACE_4), look.color(hue))))
                    .child(div().flex_1().min_w_0().child(SharedString::from(said.clone()))),
                Role::Status,
                said,
            )
        });
        let refused = self.restart_refused.clone().map(|why| format!("Not restarted: {why}"));
        let refused = refused.or_else(|| self.switch_refused.clone().filter(|_| off).map(|why| format!("Not switched on: {why}"))).map(|said| {
            labelled(div().id("status-refused").text_size(look.length(token::TEXT_SM)).text_color(danger).child(SharedString::from(said.clone())), Role::Alert, said)
        });
        // Embedded with nothing left to say under the head, the card is its head alone.
        let body = (!self.embedded || last.is_some() || refused.is_some()).then(|| {
            div().flex().flex_col().gap(look.length(token::SPACE_2)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_4)).children((!self.embedded).then_some(doing)).children(last).children(refused)
        });
        labelled(card_frame("status", false, look).when(stopped, |card| card.border_color(danger)).child(head).children(body), Role::Region, title).into_any_element()
    }

    /// Switch on, in the Off card's head: the setting Electron's Permissions tab saves, or its spinner while Fleet saves it.
    fn switch_on_press(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        if self.switching {
            return div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_LG))
                .text_color(look.color(token::FG_MUTED))
                .child(spinner("switch-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                .child(if self.switching_off { "Switching off" } else { "Switching on" })
                .into_any_element();
        }
        // The press goes as the page reads on: the keys go to the page, where Tab starts at the pills.
        let switch = press("operator-switch", "Switch on".into(), Some("power"), None, Weight::Pick, look).tooltip(tip("Queue what needs you here"));
        labelled(
            pressed(switch, cx, |this, window, cx| {
                this.switch_on(cx);
                window.focus(&this.focus, cx);
            }),
            Role::Button,
            "Switch the Operator on",
        )
        .into_any_element()
    }
}
