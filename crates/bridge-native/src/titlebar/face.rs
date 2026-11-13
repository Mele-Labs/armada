//! What the title bar draws, as small faces a view reads and never works out.
//!
//! Electron derives every one of these in TypeScript (`docs/spikes/native-titlebar-inventory`,
//! section 6). A change in Fleet is adding the ring band, the build switcher's state and the Jobs a
//! restart adopts to its own views; until it lands the walk's scenario fills these directly, and the
//! wiring later maps Fleet's fields into them. **No view holds a threshold or a sentence of its own**:
//! the words and the bands live here, so moving to Fleet's view deletes this file's builders and
//! leaves every view as it is.
//!
//! Fleet down is the exception that stays: Fleet is not there to send it, so it is read from
//! [`runtime_file::Presence`] by [`connection_of`].

use chrono::{DateTime, Local, Utc};
use gpui::SharedString;
use ipc::{
    BuildPosition, BuildSource, BuildStage, FleetBuildReport, Instant, ManifestId, ModelDownshift, ProjectPlace, Projects, ProtocolId,
    RepositorySummary, RunCloud, RunPlace,
};

use crate::link::LinkState;
use crate::runtime_file::Presence;

/// Which tone a ring and its percent are drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Normal,
    Warning,
    Danger,
}

/// What a person can do about the step down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remedy {
    Accept,
    Choose,
    Restore,
    Keep,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OfferAct {
    pub label: SharedString,
    pub remedy: Remedy,
    pub primary: bool,
    /// The card stays open after it, and shows the result.
    pub stays: bool,
}

/// The step-down offer standing on the weekly limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    /// The bell's tooltip. Absent once the step down is accepted, when no bell draws.
    pub attention: Option<SharedString>,
    /// The `Default model` fact's value.
    pub default_model: SharedString,
    pub acts: Vec<OfferAct>,
}

/// One plan window as the ring and its card draw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub name: &'static str,
    /// Whole percent, as the label says it.
    pub percent: u32,
    /// How far round the ring runs, 0 to 1.
    pub arc: f32,
    pub band: Band,
    pub resets_in: Option<SharedString>,
}

impl Limit {
    pub fn five_hour(percent: u32, band: Band, resets_in: &str) -> Limit {
        Limit::new("5-hour limit", percent, band, Some(resets_in))
    }

    pub fn weekly(percent: u32, band: Band, resets_in: &str) -> Limit {
        Limit::new("Weekly limit", percent, band, Some(resets_in))
    }

    fn new(name: &'static str, percent: u32, band: Band, resets_in: Option<&str>) -> Limit {
        Limit { name, percent, arc: (percent.min(100) as f32) / 100., band, resets_in: resets_in.map(SharedString::from) }
    }
}

/// The rings as Fleet's reading draws them (`PlanUsageFigures`, `plan-usage.tsx`): nothing before a reading
/// or off a subscription, the sweep while Fleet has none, and the five-hour and weekly limits, banded by Fleet.
pub fn usage_of(read: Option<&ipc::PlanUsage>, now: DateTime<Utc>) -> Usage {
    let Some(read) = read else { return Usage::Hidden };
    let limits: Vec<Limit> = read
        .windows
        .iter()
        .filter_map(|window| {
            let name = match window.kind.as_str() {
                "five_hour" => "5-hour limit",
                "seven_day" => "Weekly limit",
                _ => return None,
            };
            let band = match window.band {
                ipc::RingBand::Normal => Band::Normal,
                ipc::RingBand::Warning => Band::Warning,
                ipc::RingBand::Danger => Band::Danger,
            };
            let resets = window.resets_at.as_ref().and_then(|at| DateTime::parse_from_rfc3339(at.as_str()).ok()).map(|at| countdown((at.with_timezone(&Utc) - now).num_seconds()));
            Some(Limit::new(name, window.percent_used.min(100), band, resets.as_deref()))
        })
        .collect();
    if !limits.is_empty() {
        Usage::Limits(limits)
    } else if read.windows.is_empty() && read.read_at.is_some() {
        Usage::Hidden
    } else {
        Usage::Loading
    }
}

/// `2d 17h`, `2h 14m`, `9m`: how long until a window resets (`countdown`).
pub fn countdown(seconds: i64) -> String {
    let minutes = seconds.max(0) / 60;
    let (days, hours) = (minutes / (24 * 60), minutes / 60 % 24);
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

/// The step down with the stamp its words say: the shift's own end, else the weekly window's reset.
pub fn downshift_of(state: Option<&ipc::ModelDownshiftState>, usage: Option<&ipc::PlanUsage>) -> Option<(ModelDownshift, SharedString)> {
    let shift = state?.downshift.clone()?;
    let until = match &shift {
        ModelDownshift::Shifted { until, .. } => Some(until.clone()),
        _ => usage.and_then(|usage| usage.windows.iter().find(|window| window.kind == "seven_day")).and_then(|window| window.resets_at.clone()),
    };
    Some((shift, until.map(|at| built_said(&at)).unwrap_or_default().into()))
}

/// The rings in the end region.
#[derive(Clone, Debug, PartialEq)]
pub enum Usage {
    /// Nothing draws: no source, or an account off a subscription.
    Hidden,
    /// Fleet has no reading yet: the sweeping glyph, `Analyzing Usage`.
    Loading,
    Limits(Vec<Limit>),
}

impl Usage {
    /// The meter button's name: each limit and its percent, joined by `, `.
    pub fn meter_name(limits: &[Limit]) -> String {
        limits.iter().map(|one| format!("{} {}%", one.name, one.percent)).collect::<Vec<_>>().join(", ")
    }
}

/// The offer for a step-down phase. `until` is the window's reset, said as a stamp, for `Shifted`.
pub fn offer_of(state: &ModelDownshift, until: &str) -> Offer {
    let choose = OfferAct { label: "Choose…".into(), remedy: Remedy::Choose, primary: false, stays: false };
    match state {
        ModelDownshift::Offered { was, to, .. } => Offer {
            attention: Some(format!("Default to {to} offered").into()),
            default_model: was.clone().into(),
            acts: vec![OfferAct { label: format!("Default to {to}").into(), remedy: Remedy::Accept, primary: true, stays: true }, choose],
        },
        ModelDownshift::Shifted { to, .. } => Offer { attention: None, default_model: format!("{to} until {until}").into(), acts: Vec::new() },
        ModelDownshift::Reset { was, to } => Offer {
            attention: Some(format!("Back to {was} offered").into()),
            default_model: to.clone().into(),
            acts: vec![
                OfferAct { label: format!("Back to {was}").into(), remedy: Remedy::Restore, primary: true, stays: false },
                OfferAct { label: format!("Keep {to}").into(), remedy: Remedy::Keep, primary: false, stays: false },
                choose,
            ],
        },
    }
}

/// The notification while the offer or the restore stands: one sentence and one act, the card's primary.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub text: SharedString,
    pub action: SharedString,
    pub remedy: Remedy,
    /// Pressing the sentence settles and goes to the setting, which only an offer does.
    pub sentence_chooses: bool,
}

pub fn toast_of(state: &ModelDownshift) -> Option<Toast> {
    match state {
        ModelDownshift::Offered { percent, to, .. } => Some(Toast {
            text: format!("Weekly limit {percent}% used").into(),
            action: format!("Default to {to}").into(),
            remedy: Remedy::Accept,
            sentence_chooses: true,
        }),
        ModelDownshift::Reset { was, .. } => {
            Some(Toast { text: "Weekly limit reset".into(), action: format!("Back to {was}").into(), remedy: Remedy::Restore, sentence_chooses: false })
        }
        ModelDownshift::Shifted { .. } => None,
    }
}

// ---------------------------------------------------------------------------------------------
// The build switcher

/// The build report with the two facts Fleet does not send it: whether the build is level with
/// main, and the Jobs a restart would adopt.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub report: FleetBuildReport,
    pub aligned: bool,
    /// The Jobs a restart adopts. Any asks first; none restarts at once.
    pub adopts: Vec<String>,
}

/// The switcher's reading of Fleet's report: level with main as Fleet's `switcher` says, else as its position
/// counts (a position that cannot be counted is not level), and the Jobs a restart carries across.
pub fn build_of(report: &FleetBuildReport) -> Build {
    let aligned = report.switcher.map_or_else(|| report.position.is_some_and(|at| at.ahead == 0 && at.behind == 0), |switcher| switcher.aligned);
    Build { report: report.clone(), aligned, adopts: report.carried.clone() }
}

/// `Nov 5 17:05:00`, in his clock with its day, as a kept build is named. Text that is no instant comes back as it came.
pub fn built_said(at: &Instant) -> String {
    match DateTime::parse_from_rfc3339(at.as_str()) {
        Ok(when) => crate::settings::time::stamp_of(when.with_timezone(&Local), ipc::time_format::TimeFormat { date: true, ..crate::settings::time::held() }, Local::now()),
        Err(_) => at.as_str().to_string(),
    }
}

pub fn stage_word(stage: BuildStage) -> &'static str {
    match stage {
        BuildStage::Merging => "Merging branches",
        BuildStage::FetchingMain => "Fetching main",
        BuildStage::BuildingFleet => "Building Fleet",
        BuildStage::BuildingBridge => "Building Bridge",
        BuildStage::RestartingFleet => "Restarting Fleet",
        BuildStage::ReopeningBridge => "Reopening Bridge",
        BuildStage::Retrying => "Retrying build",
    }
}

fn commits(count: u32) -> String {
    if count == 1 {
        "1 commit".into()
    } else {
        format!("{count} commits")
    }
}

pub fn ahead_said(count: u32) -> String {
    format!("{} ahead of main", commits(count))
}

pub fn behind_said(count: u32) -> String {
    format!("{} behind main", commits(count))
}

/// The restart button as the section draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Restart {
    pub label: &'static str,
    pub tip: &'static str,
    pub disabled: bool,
    pub pending: bool,
    /// The build it restarts onto: the one under way, else the one in use.
    pub target: BuildSource,
}

/// Where the section's last row stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Position {
    /// The build is level with main.
    Aligned,
    Marks { ahead: u32, behind: u32 },
    /// A restart is under way, with the stage once it says.
    Restarting { target: BuildSource, stage: Option<BuildStage> },
    /// Not counted: no mark.
    Unknown,
}

impl Build {
    pub fn restart(&self) -> Restart {
        let target = self.report.restarting.unwrap_or(self.report.on);
        let working = self.report.restarting.is_some();
        let idle_on_main = self.report.on == BuildSource::Main && !working && self.aligned;
        let busy = working || self.report.reverting || self.report.swapping;
        let (label, tip) = match (target, working) {
            (BuildSource::Preview, false) => ("Refresh preview", "Merge in-flight branches and restart on the preview"),
            (BuildSource::Preview, true) => ("Refreshing preview", "Merge in-flight branches and restart on the preview"),
            (BuildSource::Main, false) => ("Update to main", "Restart on latest main"),
            (BuildSource::Main, true) => ("Updating to main", "Restart on latest main"),
        };
        Restart { label, tip: if idle_on_main { "On latest main" } else { tip }, disabled: idle_on_main || busy, pending: working, target }
    }

    pub fn position(&self) -> Position {
        if let Some(target) = self.report.restarting {
            return Position::Restarting { target, stage: self.report.stage };
        }
        match self.report.position {
            None => Position::Unknown,
            Some(_) if self.aligned => Position::Aligned,
            Some(BuildPosition { ahead, behind }) => Position::Marks { ahead, behind },
        }
    }

    /// What Revert's tooltip says, where a build before is kept.
    pub fn revert_tip(&self) -> Option<String> {
        let previous = self.report.previous.as_ref()?;
        Some(format!("Back to the build of {}, {}", built_said(&previous.built_at), short(&previous.commit)))
    }
}

pub fn short(commit: &str) -> String {
    commit.chars().take(7).collect()
}

/// A kept build as the panel names it: when it was built, and its short commit.
pub fn kept_said(built_at: &Instant, commit: &str) -> String {
    format!("{} · {}", built_said(built_at), short(commit))
}

// ---------------------------------------------------------------------------------------------
// Fleet down

/// Bridge's own reading of Fleet, in `packages/protocol`'s words.
#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    Reading,
    NotRunning { detail: String },
    Refused { detail: String },
    Connecting { pid: u32, port: u16 },
    Starting { pid: u32, port: u16 },
    Unreachable { pid: u32, port: u16, silent: String, last_read: Option<String> },
    Mismatch { pid: u32, port: u16, fleet: Option<String>, bridge: String },
    Connected { pid: u32, port: u16, protocol: String, up: Option<String> },
}

/// What the live link says once the runtime file names a Fleet that speaks this protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    Connecting,
    Starting,
    Connected { up: Option<String> },
    Unreachable { silent: String, last_read: Option<String> },
}

/// Read the runtime file's presence and the link into the connection the bar draws. **A state with a
/// Fleet behind it needs the link as well**, since the file alone cannot tell running from silent.
pub fn connection_of(presence: &Presence, link: Link, path: &str) -> Connection {
    match presence {
        Presence::Absent => Connection::NotRunning { detail: format!("no runtime file at {path}") },
        Presence::Stale(why) => Connection::NotRunning {
            detail: match why.strip_suffix(" is not running") {
                Some(pid) => format!("{pid} is held by nothing · Fleet exited without cleaning up"),
                None => format!("{} · its port is not Fleet's", why.replacen(" started at ", " is held by a process that started ", 1)),
            },
        },
        Presence::Refused(why) => Connection::Refused { detail: format!("{why} · {path}") },
        Presence::Running(file) => {
            if file.protocol_id != ProtocolId::current() {
                let sent = (file.protocol_id != ProtocolId::default()).then(|| file.protocol_id.to_string());
                return Connection::Mismatch { pid: file.pid, port: file.port, fleet: sent, bridge: ProtocolId::current().to_string() };
            }
            match link {
                Link::Connecting => Connection::Connecting { pid: file.pid, port: file.port },
                Link::Starting => Connection::Starting { pid: file.pid, port: file.port },
                Link::Connected { up } => Connection::Connected { pid: file.pid, port: file.port, protocol: file.protocol_id.to_string(), up },
                Link::Unreachable { silent, last_read } => Connection::Unreachable { pid: file.pid, port: file.port, silent, last_read },
            }
        }
    }
}

/// How long a Fleet that went down may stay down before its notice draws (`cockpit/boot.ts` `POWER_GRACE_MS`).
pub const POWER_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// The live link as the Fleet connection holds it: the state, the runtime file it was read from with
/// that file's path, and the instants the face is timed from.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveLink {
    pub state: LinkState,
    pub presence: Option<(Presence, String)>,
    /// When the link last went unreachable.
    pub silent_since: Option<std::time::Instant>,
    /// When Fleet last sent anything.
    pub read_at: Option<std::time::Instant>,
    /// When a connection last dropped.
    pub dropped: Option<std::time::Instant>,
}

/// A span as Electron's `elapsed` says it: seconds, then minutes, then hours.
fn elapsed(span: std::time::Duration) -> String {
    let seconds = span.as_secs_f64().round() as u64;
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = (seconds as f64 / 60.).round() as u64;
    if minutes < 60 { format!("{minutes}m") } else { format!("{}h", (minutes as f64 / 60.).round() as u64) }
}

impl LiveLink {
    /// The connection at `now`. A mismatch Fleet sent on the stream names the Fleet the runtime file does.
    pub fn connection(&self, now: std::time::Instant) -> Connection {
        let Some((presence, path)) = &self.presence else {
            return match &self.state {
                LinkState::Refused(why) => Connection::Refused { detail: why.clone() },
                _ => Connection::Reading,
            };
        };
        let link = match &self.state {
            LinkState::ProtocolMismatch { fleet, ours } => {
                if let Presence::Running(file) = presence {
                    let sent = (*fleet != ProtocolId::default()).then(|| fleet.to_string());
                    return Connection::Mismatch { pid: file.pid, port: file.port, fleet: sent, bridge: ours.to_string() };
                }
                Link::Connecting
            }
            LinkState::Starting => Link::Starting,
            LinkState::Connected => Link::Connected { up: None },
            LinkState::Unreachable(_) => Link::Unreachable {
                silent: elapsed(now.saturating_duration_since(self.silent_since.unwrap_or(now))),
                last_read: self.read_at.map(|at| elapsed(now.saturating_duration_since(at))),
            },
            LinkState::NotRunning(_) | LinkState::Refused(_) | LinkState::Connecting { .. } => Link::Connecting,
        };
        connection_of(presence, link, path)
    }

    /// Whether Fleet is just coming back (`cockpit/boot.ts` `powerOf`): a Fleet down holds its notice for the
    /// grace after the window opened or the connection last dropped, whichever is later.
    pub fn power(&self, opened: std::time::Instant, now: std::time::Instant) -> Power {
        match self.connection(now) {
            Connection::Connected { .. } => Power::Live,
            Connection::Reading | Connection::Connecting { .. } | Connection::Starting { .. } => Power::Standby,
            Connection::NotRunning { .. } | Connection::Unreachable { .. } if now < self.grace_ends(opened) => Power::Standby,
            _ => Power::Fault,
        }
    }

    fn grace_ends(&self, opened: std::time::Instant) -> std::time::Instant {
        self.dropped.map_or(opened, |dropped| dropped.max(opened)) + POWER_GRACE
    }

    /// The next instant the face moves by the clock alone: each second while Fleet is silent, since the
    /// notice says for how long, and the grace's end while Fleet is down.
    pub fn next_at(&self, opened: std::time::Instant, now: std::time::Instant) -> Option<std::time::Instant> {
        match self.connection(now) {
            Connection::Unreachable { .. } => Some(now + std::time::Duration::from_secs(1)),
            Connection::NotRunning { .. } => Some(self.grace_ends(opened)).filter(|at| *at > now),
            _ => None,
        }
    }
}

/// The dot's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dot {
    Running,
    NotRunning,
    Unreachable,
    Starting,
    Unknown,
}

/// Whether the failure is Fleet gone (a fault) or Fleet alive and not answering (degraded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fault,
    Degraded,
}

/// The words for a connection: the label on the dot's tooltip, and the statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub dot: Dot,
    pub label: &'static str,
    pub headline: String,
    pub detail: Option<String>,
    pub next: Option<&'static str>,
    pub code: &'static str,
    pub kind: Kind,
}

impl Connection {
    pub fn statement(&self) -> Statement {
        let pid_port = |pid: u32, port: u16| Some(format!("pid {pid} · port {port}"));
        let (dot, label, headline, detail, next, code, kind): (Dot, &'static str, String, Option<String>, Option<&'static str>, &'static str, Kind) =
            match self {
                Connection::Reading => (Dot::Unknown, "Reading", "Reading Fleet's runtime file".into(), None, None, "bridge.fleet.reading", Kind::Degraded),
                Connection::NotRunning { detail } => (
                    Dot::NotRunning,
                    "Not running",
                    "Fleet is not running".into(),
                    Some(detail.clone()),
                    Some("Start Fleet. Bridge reconnects on its own."),
                    "bridge.fleet.not_running",
                    Kind::Fault,
                ),
                Connection::Refused { detail } => (
                    Dot::Unknown,
                    "Refused",
                    "Fleet's runtime file was refused".into(),
                    Some(detail.clone()),
                    Some("Check what wrote the file. Bridge will not connect to a port it names."),
                    "bridge.fleet.runtime_file_refused",
                    Kind::Fault,
                ),
                Connection::Connecting { pid, port } => {
                    (Dot::Unknown, "Connecting", "Connecting to Fleet".into(), pid_port(*pid, *port), None, "bridge.fleet.connecting", Kind::Degraded)
                }
                Connection::Starting { pid, port } => {
                    (Dot::Starting, "Starting", "Fleet is starting".into(), pid_port(*pid, *port), None, "bridge.fleet.starting", Kind::Degraded)
                }
                Connection::Unreachable { pid, port, silent, last_read } => (
                    Dot::Unreachable,
                    "Unreachable",
                    "Fleet unreachable".into(),
                    Some(format!(
                        "pid {pid} alive on port {port}, no answer for {silent} · last read {}",
                        last_read.as_deref().map_or("nothing read yet".to_string(), |ago| format!("{ago} ago"))
                    )),
                    Some("Fleet is up and not answering. What is shown below is not live."),
                    "bridge.fleet.unreachable",
                    Kind::Degraded,
                ),
                Connection::Mismatch { fleet, bridge, .. } => (
                    Dot::Unknown,
                    "Mismatch",
                    if fleet.is_some() { "Fleet and Bridge do not match" } else { "Fleet is out of date" }.into(),
                    Some(format!("Fleet {} · Bridge {bridge}", fleet.as_deref().unwrap_or("sent no ID"))),
                    Some("Restart Fleet onto the installed build."),
                    "bridge.fleet.protocol_mismatch",
                    Kind::Fault,
                ),
                Connection::Connected { pid, port, .. } => {
                    (Dot::Running, "Running", "Fleet running".into(), pid_port(*pid, *port), None, "bridge.fleet.running", Kind::Degraded)
                }
            };
        Statement { dot, label, headline, detail, next, code, kind }
    }

    /// The panel's figure rows for this state: only those it has a value for.
    pub fn figures(&self) -> Vec<(&'static str, String)> {
        match self {
            Connection::Connected { pid, port, protocol, up } => {
                let mut rows = vec![("pid", pid.to_string()), ("port", port.to_string()), ("protocol", protocol.clone())];
                rows.extend(up.clone().map(|up| ("up", up)));
                rows
            }
            Connection::Connecting { pid, port } | Connection::Starting { pid, port } | Connection::Unreachable { pid, port, .. } => {
                vec![("pid", pid.to_string()), ("port", port.to_string())]
            }
            _ => Vec::new(),
        }
    }

    /// Fleet answers: the one state Dispatch is offered in.
    pub fn is_live(&self) -> bool {
        matches!(self, Connection::Connected { .. })
    }

    pub fn is_mismatch(&self) -> bool {
        matches!(self, Connection::Mismatch { .. })
    }
}

/// Bridge's reading of whether Fleet is just coming up, which holds the notice back for 30 seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    /// Fleet is down within the grace, or kept from the last answer: no notice yet.
    Standby,
    Live,
    /// The grace ran out.
    Fault,
}

/// Where Restart Fleet stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Restarting {
    Idle,
    Working,
    /// A new pid answered and it still does not match.
    Apart,
    /// launchd would not restart it, or no new Fleet answered inside 30 seconds; the detail is what was answered.
    Failed { why: RestartWhy, detail: String },
}

/// Why a restart did not take, in `FleetRestart`'s words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartWhy {
    NotStartedByArmada,
    NotRunning,
    Refused,
    NoAnswer,
}

impl RestartWhy {
    /// The headline and what to do, as `fleetRestartFailure` says them.
    pub fn said(self) -> (&'static str, &'static str) {
        match self {
            RestartWhy::NotStartedByArmada => ("Fleet was not started by Armada's service", "Stop it in the terminal it runs in, then start Fleet again."),
            RestartWhy::NotRunning => ("Fleet is not running", "Start Fleet. Bridge reconnects on its own."),
            RestartWhy::Refused => ("launchd refused to restart Fleet", "Fleet is as it was. Read the log, or restart it from a terminal."),
            RestartWhy::NoAnswer => ("Fleet did not come back inside 30 seconds", "Read the Fleet log, then restart it again."),
        }
    }

    /// The wire word, as the failure's Answer row says it.
    pub fn word(self) -> &'static str {
        match self {
            RestartWhy::NotStartedByArmada => "not_started_by_armada",
            RestartWhy::NotRunning => "not_running",
            RestartWhy::Refused => "refused",
            RestartWhy::NoAnswer => "no_answer",
        }
    }
}

/// What Doctor said of the modules it read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Warn,
    Fail,
    Reading,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Doctor {
    pub outcome: Outcome,
    pub modules: SharedString,
}

/// Everything the bar's Fleet side draws.
#[derive(Clone, Debug, PartialEq)]
pub struct FleetFace {
    pub connection: Connection,
    pub power: Power,
    pub restarting: Restarting,
    pub doctor: Option<Doctor>,
}

impl FleetFace {
    /// Whether the notice draws: the statement has a next line, and Fleet is not just powering on or swapping.
    pub fn notice(&self, swapping: bool) -> bool {
        self.connection.statement().next.is_some() && self.power != Power::Standby && !swapping
    }
}

/// A repository the picker lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Repository {
    pub name: SharedString,
    /// Carries the detail `Not set up`.
    pub not_set_up: bool,
    /// Where Fleet says this repository's Armada configuration is kept; none until it has said.
    pub kept: Option<RunsPlace>,
}

/// Where the runs of a repository are kept (`RunsPlaces.tsx`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RunsPlaceKind {
    Mac,
    Repository,
    Synced,
    Cloud,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunsPlace {
    pub kind: RunsPlaceKind,
    /// The folder or store, when Fleet names it.
    pub detail: Option<SharedString>,
}

impl RunsPlace {
    /// The place as a phrase (`placePhrase`): `on this Mac`, `in iCloud Drive`.
    pub fn phrase(&self) -> String {
        match (self.kind, &self.detail) {
            (RunsPlaceKind::Mac, _) => "on this Mac".into(),
            (RunsPlaceKind::Repository, _) => "in the repository".into(),
            (_, Some(detail)) => format!("in {detail}"),
            (RunsPlaceKind::Synced, None) => "in a synced folder".into(),
            (RunsPlaceKind::Cloud, None) => "in cloud storage".into(),
        }
    }
}

/// A project the switcher lists: whole repositories, or folders of one, and what it holds of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    /// Fleet's id for it, which keeps the window on the project when the list changes.
    pub id: SharedString,
    pub name: SharedString,
    pub members: Vec<Member>,
    /// The tracker and its key, as `Linear, STORE`. Fleet serves none yet, so live it is absent.
    pub tracker: Option<SharedString>,
    pub kept: Option<RunsPlace>,
    /// The look it wears: `Yours` where it keeps the person's own. Fleet serves none yet.
    pub look: Option<SharedString>,
}

/// A repository of a project by its place in [`Face::repositories`], or a folder of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub repository: usize,
    pub folder: Option<SharedString>,
}

/// The repository Fleet names a served repository by: its Manifest's id, else the id it gives one with none.
fn manifest_of(one: &RepositorySummary) -> Option<&ManifestId> {
    one.manifest.as_ref().map(|manifest| &manifest.id).or(one.implicit_id.as_ref())
}

/// The repositories the picker lists: each by its directory's name, `Not set up` where no Manifest has
/// loaded. Where one is in `held` by name, what Fleet said of where it is kept stays.
pub fn repositories_of(served: &[RepositorySummary], held: &[Repository]) -> Vec<Repository> {
    served
        .iter()
        .map(|one| {
            let name = one.manifest.as_ref().map_or_else(|| one.root.rsplit('/').find(|part| !part.is_empty()).unwrap_or(&one.root).to_string(), |manifest| manifest.repository.clone());
            let kept = held.iter().find(|one| one.name.as_ref() == name).and_then(|one| one.kept.clone());
            Repository { name: name.into(), not_set_up: one.manifest.is_none(), kept }
        })
        .collect()
}

/// Where Fleet keeps a project's file, as a place the pane can phrase.
fn place_of(place: &ProjectPlace) -> RunsPlace {
    let (kind, detail) = match place.place {
        RunPlace::ThisMac => (RunsPlaceKind::Mac, None),
        RunPlace::Repository => (RunsPlaceKind::Repository, None),
        RunPlace::Synced => (RunsPlaceKind::Synced, place.synced_folder.clone()),
        RunPlace::Cloud => (
            RunsPlaceKind::Cloud,
            match &place.cloud {
                Some(RunCloud::Bucket(address)) => Some(address.clone()),
                Some(RunCloud::Store {}) | None => None,
            },
        ),
    };
    RunsPlace { kind, detail: detail.map(SharedString::from) }
}

/// The projects the switcher lists, in the order Fleet keeps them. A project whose file would not read
/// has no members to draw and is left out, as the Workspaces page leaves it. A member Fleet serves no
/// repository for is left out of its project.
pub fn projects_of(served: &[RepositorySummary], read: &Projects) -> Vec<Project> {
    read.projects
        .iter()
        .filter(|one| one.unread.is_none())
        .map(|one| Project {
            id: one.id.clone().into(),
            name: one.name.clone().into(),
            members: one
                .members
                .iter()
                .filter_map(|member| {
                    let repository = served.iter().position(|each| manifest_of(each) == Some(&member.manifest_id))?;
                    Some(Member { repository, folder: member.folder.clone().map(SharedString::from) })
                })
                .collect(),
            tracker: None,
            kept: Some(place_of(&one.place)),
            look: None,
        })
        .collect()
}

/// One place in the bar's centre (`DK/TabBar.tsx`, `DECK_TABS`): a screen the window can be in.
///
/// **Only the places this app has a screen for are listed**, so the strip holds Jobs today. A place
/// is added here, and drawn, when its screen lands; `⌘1` to `⌘9` follow whatever is drawn, as the
/// registry's `deck_tabs` row says.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: &'static str,
    pub label: &'static str,
    /// The registry glyph of the surface, drawn before the name.
    pub glyph: &'static str,
    /// The person's layout draws it (`rail` region), as the View group ticks.
    pub shown: bool,
    /// The layout may hide it. Cockpit and System may not: `Always shown`.
    pub hideable: bool,
}

/// Events Fleet dropped before Bridge saw them, and how many of them the person has said they have seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dropped {
    /// Events that will never arrive, as Fleet says.
    pub missed: u32,
    /// How many the person has said they have seen.
    pub noted: u32,
}

/// Everything the title bar draws, in one place the scenario fills and the wiring will.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub usage: Usage,
    pub build: Option<Build>,
    /// The branches Jobs offered for the preview, and the Refresh row Fleet reads from them.
    pub offers: Option<ipc::PreviewOffers>,
    pub fleet: FleetFace,
    /// The repositories the picker lists, and the one chosen; none chosen is All.
    pub repositories: Vec<Repository>,
    pub chosen: Option<usize>,
    /// The projects, and the one the window is scoped to, if any.
    pub projects: Vec<Project>,
    pub scoped: Option<usize>,
    /// Whether a profile is the active one: the menu's account entry says Account, else Sign in.
    pub signed_in: bool,
    /// Where the step down stands, with the window's reset said as a stamp. The weekly limit's card
    /// and the notification both read it; absent is none, or settled.
    pub downshift: Option<(ModelDownshift, SharedString)>,
    /// The places the strip in the centre could draw, in order, and the one the window is in. Empty:
    /// the centre is the search trigger alone, at its full width.
    pub places: Vec<Place>,
    pub here: Option<&'static str>,
    /// Events Fleet dropped before Bridge saw them (`Standing.tsx`): the alert stands until `noted` catches up.
    pub dropped: Dropped,
}

impl Face {
    /// The face of a bar with nothing read yet.
    pub fn bare() -> Face {
        Face {
            usage: Usage::Hidden,
            build: None,
            offers: None,
            fleet: FleetFace { connection: Connection::Reading, power: Power::Standby, restarting: Restarting::Idle, doctor: None },
            repositories: Vec::new(),
            chosen: None,
            projects: Vec::new(),
            scoped: None,
            signed_in: false,
            downshift: None,
            places: Vec::new(),
            here: None,
            dropped: Dropped { missed: 0, noted: 0 },
        }
    }

    /// The places the strip draws now, in order: the ones the layout has not hidden.
    pub fn drawn_places(&self) -> impl Iterator<Item = &Place> {
        self.places.iter().filter(|place| place.shown)
    }

    /// Whether the layout may hide this place now: not a fixed one, and not the last one drawn, since
    /// the strip is how a hidden place comes back.
    pub fn may_hide(&self, id: &str) -> bool {
        self.places.iter().find(|place| place.id == id).is_some_and(|place| place.hideable)
            && !(self.drawn_places().count() == 1 && self.drawn_places().any(|place| place.id == id))
    }

    /// The switcher's trigger: the project and, muted, the repositories it draws on; else the
    /// repository chosen, else `All`, or the reason none is listed.
    pub fn chosen_said(&self) -> (SharedString, Option<SharedString>) {
        if let Some(one) = self.scoped.and_then(|at| self.projects.get(at)) {
            return (one.name.clone(), self.members_said(one));
        }
        match self.chosen.and_then(|at| self.repositories.get(at)) {
            Some(one) => (one.name.clone(), None),
            None if self.repositories.is_empty() => ("No repository read yet".into(), None),
            None => ("All".into(), None),
        }
    }

    /// The repositories a project draws on, each once, in the order its members name them.
    pub fn members_said(&self, project: &Project) -> Option<SharedString> {
        let mut names: Vec<&str> = Vec::new();
        for member in &project.members {
            if let Some(one) = self.repositories.get(member.repository) {
                if !names.contains(&one.name.as_ref()) {
                    names.push(one.name.as_ref());
                }
            }
        }
        (!names.is_empty()).then(|| names.join(", ").into())
    }

    /// The repositories in no project, by place.
    pub fn lone(&self) -> Vec<usize> {
        (0..self.repositories.len()).filter(|&at| !self.projects.iter().any(|one| one.members.iter().any(|member| member.repository == at))).collect()
    }

    /// What the Settings button beside the trigger names: the project, else the repository, else nothing.
    pub fn settings_said(&self) -> SharedString {
        if let Some(one) = self.scoped.and_then(|at| self.projects.get(at)) {
            return format!("Settings for {}", one.name).into();
        }
        match self.chosen.and_then(|at| self.repositories.get(at)) {
            Some(one) => format!("Settings for {}", one.name).into(),
            None => "Settings".into(),
        }
    }
}
