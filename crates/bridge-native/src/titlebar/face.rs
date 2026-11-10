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

use chrono::{DateTime, Local};
use gpui::SharedString;
use ipc::{
    BuildPosition, BuildSource, BuildStage, FleetBuildReport, Instant, ModelDownshift, ProtocolId,
};

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

/// `5 Nov, 17:05`, in this machine's time, as a kept build is named. Text that is no instant comes back as it came.
pub fn built_said(at: &Instant) -> String {
    match DateTime::parse_from_rfc3339(at.as_str()) {
        Ok(when) => when.with_timezone(&Local).format("%-d %b, %H:%M").to_string(),
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
    /// No new runtime file inside 30 seconds.
    NoAnswer,
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
    pub name: SharedString,
    pub members: Vec<Member>,
    /// The tracker and its key, as `Linear, STORE`.
    pub tracker: Option<SharedString>,
    pub kept: Option<RunsPlace>,
    /// The look it wears: `Yours` where it keeps the person's own.
    pub look: Option<SharedString>,
}

/// A repository of a project by its place in [`Face::repositories`], or a folder of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub repository: usize,
    pub folder: Option<SharedString>,
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
