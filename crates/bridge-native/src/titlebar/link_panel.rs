//! The centred Fleet panel's state (`fleet-link-panel.html`): stopped, restarting, or one side on an
//! older build, worked out from the connection and what was read from disk at one instant.
//!
//! **Every state is read, none is guessed by a timer.** A restart is `restart.lock` naming a live
//! pid. Stopped is no Fleet and no live restart once launchd's relaunch throttle has gone by. An
//! older build is the side whose kept build is not the one `builds/current` names. Reading the disk
//! and drawing are `link_layer.rs`'s; this file holds no I/O, so every row of the table is a test.

use chrono::{DateTime, Local, NaiveDateTime, Utc};

use super::face::Connection;

/// launchd relaunches a Fleet that exited no sooner than this. Gone longer, it is stopped.
pub const RELAUNCH_THROTTLE: std::time::Duration = std::time::Duration::from_secs(2);

/// How many of a restart's lines the Fleet unit shows: the one under way and those before it.
const SHOWN_STEPS: usize = 3;

/// What follows every restart's last step, as `scripts/restart` ends.
const REOPENS: &str = "Apps reopen if their protocol differs";

/// What was read from disk, which a scene stands in for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Disk {
    /// When `restart.lock` was taken, where it names a pid that is alive.
    pub restart: Option<DateTime<Local>>,
    /// The tail of `restart.log`, where there is one.
    pub restart_log: Option<String>,
    /// The tail of Fleet's own log.
    pub fleet_log: Option<String>,
    /// The kept build `builds/current` names, such as `2026-11-13T004852Z-126b4bb`.
    pub current: Option<String>,
    /// The kept build this app runs from.
    pub app_build: Option<String>,
}

/// The state's colour: red, blue or amber.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Stopped,
    Restarting,
    Older,
}

/// The one button that fixes the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    StartFleet,
    RestartFleet,
    ReopenApp,
}

impl Act {
    /// Its label, and the label while it is under way.
    pub fn label(self, pending: bool) -> &'static str {
        match (self, pending) {
            (Act::StartFleet, false) => "Start Fleet",
            (Act::StartFleet, true) => "Starting Fleet",
            (Act::RestartFleet, false) => "Restart Fleet",
            (Act::RestartFleet, true) => "Restarting Fleet",
            (Act::ReopenApp, false) => "Reopen Armada",
            (Act::ReopenApp, true) => "Reopening Armada",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Act::StartFleet => "zap",
            Act::RestartFleet | Act::ReopenApp => "refresh-cw",
        }
    }
}

/// Which log Open log opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Log {
    Restart,
    Fleet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Done,
    Now,
    Next,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub mark: Mark,
    /// The line's first clause, which is what the row shows.
    pub text: String,
    /// The line whole, for the row's tooltip.
    pub line: String,
}

/// Where a restart line's first clause ends: before what it names, such as a path.
const CLAUSE_ENDS: [&str; 4] = [":", " — ", " in /", " from /"];

/// `scripts/restart`'s own steps by how their line starts, each with the short name a row shows.
const KNOWN_STEPS: [(&str, &str); 12] = [
    ("Booting ", "Restarting Fleet"),
    ("Stopping the Fleet in ", "Stopping Fleet"),
    ("Fleet is running (pid ", "Checking for running Jobs"),
    ("No Drone is working", "No Job is running"),
    ("This Fleet runs no Drones", "No Job is running"),
    ("Adopting every working Drone", "Running Jobs keep running"),
    ("ipc was last built at protocol ", "Rebuilding the protocol"),
    ("Retiring ", "Retiring the phone app's service"),
    ("Kept ", "Kept the build and swapped nothing"),
    ("Bridge runs from ", "Reopening Bridge"),
    ("Quitting the running Bridge", "Quitting Bridge"),
    ("The native Bridge runs from ", "Reopening this app"),
];

/// What a restart step's row shows: a known step's short name, else the line's first clause.
pub fn step_name(line: &str) -> String {
    KNOWN_STEPS.iter().find(|(start, _)| line.starts_with(start)).map_or_else(|| clause_of(line), |(_, name)| name.to_string())
}

/// A restart line's first clause: `Keeping the build in /Users/…` is `Keeping the build`.
pub fn clause_of(line: &str) -> String {
    let cut = CLAUSE_ENDS.iter().filter_map(|end| line.find(end)).min().unwrap_or(line.len());
    line[..cut].trim_end().to_string()
}

/// A kept build as a unit shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub commit: String,
    /// When it was built, in his clock: `00:48`.
    pub at: String,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// Build and protocol. No build where the side runs from no kept build.
    Builds { build: Option<Build>, protocol: String },
    /// The last line a log holds, and which log.
    LastWords { from: &'static str, line: String },
    Steps(Vec<Step>),
    Empty,
}

/// Fleet or this app. No tone is fine, drawn green.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    pub tone: Option<Tone>,
    pub state: &'static str,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub tone: Tone,
    pub title: &'static str,
    pub since: String,
    pub why: &'static str,
    pub fleet: Unit,
    pub app: Unit,
    /// The words under the wire between the units.
    pub link: &'static str,
    pub act: Option<Act>,
    /// Beside the button, or in its place where there is none.
    pub note: Option<&'static str>,
    pub log: Option<Log>,
}

/// The panel at `now`, or none. `since` is when the connection last dropped or the window opened,
/// whichever is later.
pub fn panel_of(connection: &Connection, disk: &Disk, since: DateTime<Local>, now: DateTime<Local>) -> Option<Panel> {
    match connection {
        // Unreachable is Fleet alive and silent: its own toast says so.
        Connection::Connected { .. } | Connection::Unreachable { .. } | Connection::Reading => None,
        _ if disk.restart.is_some() => Some(restarting(disk, disk.restart.unwrap_or(now), now)),
        Connection::Connecting { .. } | Connection::Starting { .. } => None,
        Connection::NotRunning { .. } | Connection::Refused { .. } if (now - since).num_milliseconds() < RELAUNCH_THROTTLE.as_millis() as i64 => None,
        Connection::NotRunning { .. } => Some(stopped(None, disk, since, now)),
        Connection::Refused { detail } => Some(stopped(Some(detail), disk, since, now)),
        Connection::Mismatch { fleet, build, .. } => Some(older(fleet.as_deref(), build.as_deref(), disk, since)),
    }
}

fn restarting(disk: &Disk, from: DateTime<Local>, now: DateTime<Local>) -> Panel {
    Panel {
        tone: Tone::Restarting,
        title: "Fleet restarting",
        since: span((now - from).num_seconds()),
        why: "The restart brings Fleet back itself. Nothing on screen is live until it answers.",
        fleet: Unit { tone: Some(Tone::Restarting), state: "Restarting", body: Body::Steps(steps_of(disk.restart_log.as_deref().unwrap_or_default())) },
        app: app_unit(disk, None, "Running"),
        link: "restart under way",
        act: None,
        note: None,
        log: Some(Log::Restart),
    }
}

/// A restart that did not finish speaks for the stop; else Fleet's own log does. A refused runtime
/// file is its own last words.
fn stopped(refused: Option<&str>, disk: &Disk, since: DateTime<Local>, now: DateTime<Local>) -> Panel {
    const LIVE: &str = "Nothing on screen is live until Fleet is back.";
    let unfinished = disk.restart_log.as_deref().filter(|tail| !finished(tail));
    let (body, why, log) = match (refused, unfinished.and_then(last_line)) {
        (Some(detail), _) => (
            Body::LastWords { from: "fleet.json", line: detail.to_string() },
            "This app refused Fleet's runtime file. Nothing on screen is live until Fleet is back.",
            Log::Fleet,
        ),
        (None, Some(line)) => (Body::LastWords { from: "restart.log", line }, "The restart that stopped it did not finish. Nothing on screen is live until Fleet is back.", Log::Restart),
        (None, None) => (disk.fleet_log.as_deref().and_then(last_line).map_or(Body::Empty, |line| Body::LastWords { from: "Fleet's log", line }), LIVE, Log::Fleet),
    };
    Panel {
        tone: Tone::Stopped,
        title: "Fleet stopped",
        since: format!("since {} · {}", clock(since), span((now - since).num_seconds())),
        why,
        fleet: Unit { tone: Some(Tone::Stopped), state: "Stopped", body },
        app: app_unit(disk, None, "Running"),
        link: "no answer",
        act: Some(Act::StartFleet),
        note: None,
        log: Some(log),
    }
}

/// The side off `builds/current` is the older one. Both off it, or neither on a kept build, and
/// neither is named: both protocols show, and Restart Fleet.
fn older(fleet_protocol: Option<&str>, fleet_build: Option<&str>, disk: &Disk, since: DateTime<Local>) -> Panel {
    let current = disk.current.as_deref();
    let on_current = |build: Option<&str>| current.is_some() && build == current;
    let (fleet_on, app_on) = (on_current(fleet_build), on_current(disk.app_build.as_deref()));
    // A side named older is off current with a kept build of its own; else it is only apart.
    let state = |build: Option<&str>, on: bool| if build.is_some() && !on { "Older build" } else { "Running" };
    let (title, fleet_tone, app_tone, act, note) = match (fleet_on, app_on) {
        (false, true) => ("Fleet runs an older build", Some(Tone::Older), None, Act::RestartFleet, "Running Jobs keep running"),
        (true, false) => ("This app runs an older build", None, Some(Tone::Older), Act::ReopenApp, "Fleet and running Jobs keep running"),
        _ => ("Fleet and this app do not match", Some(Tone::Older), Some(Tone::Older), Act::RestartFleet, "Running Jobs keep running"),
    };
    let fleet_state = if fleet_tone.is_some() { state(fleet_build, fleet_on) } else { "Running" };
    let app_state = if app_tone.is_some() { state(disk.app_build.as_deref(), app_on) } else { "Running" };
    Panel {
        tone: Tone::Older,
        title,
        since: format!("since {}", clock(since)),
        why: "Fleet and this app speak different protocols, so this app cannot connect.",
        fleet: Unit {
            tone: fleet_tone,
            state: fleet_state,
            body: Body::Builds { build: build_of(fleet_build, current), protocol: fleet_protocol.map_or_else(|| "none".to_string(), protocol_said) },
        },
        app: app_unit(disk, app_tone, app_state),
        link: "protocols differ",
        act: Some(act),
        note: Some(note),
        log: None,
    }
}

fn app_unit(disk: &Disk, tone: Option<Tone>, state: &'static str) -> Unit {
    Unit {
        tone,
        state,
        body: Body::Builds { build: build_of(disk.app_build.as_deref(), disk.current.as_deref()), protocol: protocol_said(ipc::ProtocolId::current().as_str()) },
    }
}

fn build_of(name: Option<&str>, current: Option<&str>) -> Option<Build> {
    let name = name?;
    let (built, commit) = kept_of(name)?;
    Some(Build { commit, at: clock(built.with_timezone(&Local)), current: Some(name) == current })
}

/// A protocol as a unit shows it: its first eight characters.
fn protocol_said(protocol: &str) -> String {
    protocol.chars().take(8).collect()
}

/// A kept build's folder name, `%Y-%m-%dT%H%M%SZ-<commit>`: when it was built, in UTC, and its commit.
pub fn kept_of(name: &str) -> Option<(DateTime<Utc>, String)> {
    let (stamp, commit) = name.split_once("Z-")?;
    let built = NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H%M%S").ok()?.and_utc();
    (!commit.is_empty()).then(|| (built, commit.to_string()))
}

fn clock(at: DateTime<Local>) -> String {
    at.format("%H:%M").to_string()
}

/// `48 s`, `6 min`, `2 h`.
fn span(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < 60 {
        format!("{seconds} s")
    } else if seconds < 3600 {
        format!("{} min", seconds / 60)
    } else {
        format!("{} h", seconds / 3600)
    }
}

/// A log line with its terminal colour codes taken out.
pub fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(one) = chars.next() {
        if one != '\x1b' {
            out.push(one);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        }
    }
    out
}

/// What a restart said as it went: `say`'s bold lines and its `restart:` lines, plain.
fn said(tail: &str) -> Vec<String> {
    tail.lines()
        .filter_map(|line| {
            if line.contains("\x1b[1m") {
                Some(plain(line))
            } else {
                line.strip_prefix("restart: ").map(plain)
            }
        })
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// A restart ends its run on `Done`.
fn finished(tail: &str) -> bool {
    said(tail).last().is_some_and(|line| line == "Done")
}

/// The restart under way: the last few lines of its run, the last one now and those before done,
/// then the apps reopening, unless that is the step now.
pub fn steps_of(tail: &str) -> Vec<Step> {
    let said = said(tail);
    let from = said.iter().rposition(|line| line == "Done").map_or(0, |at| at + 1);
    let run = &said[from..];
    let shown = &run[run.len().saturating_sub(SHOWN_STEPS)..];
    let mut steps: Vec<Step> = shown
        .iter()
        .enumerate()
        .map(|(at, line)| Step { mark: if at + 1 == shown.len() { Mark::Now } else { Mark::Done }, text: step_name(line), line: line.clone() })
        .collect();
    if steps.last().is_some_and(|now| !now.line.to_lowercase().contains("reopen")) {
        steps.push(Step { mark: Mark::Next, text: REOPENS.to_string(), line: REOPENS.to_string() });
    }
    steps
}

/// The last line with words on it, plain.
pub fn last_line(tail: &str) -> Option<String> {
    tail.lines().rev().map(|line| plain(line).trim().to_string()).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const CURRENT: &str = "2026-11-13T004852Z-126b4bb";
    const OLDER: &str = "2026-11-12T211500Z-2f88990";

    fn at(h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 13, h, m, s).single().expect("a local time")
    }

    fn down() -> Connection {
        Connection::NotRunning { detail: "no runtime file".into() }
    }

    fn mismatch(build: Option<&str>) -> Connection {
        Connection::Mismatch { pid: 1, port: 2, fleet: Some("af0a982d5c7e1b40".into()), bridge: ipc::ProtocolId::current().to_string(), build: build.map(str::to_string) }
    }

    fn kept(app: Option<&str>) -> Disk {
        Disk { current: Some(CURRENT.into()), app_build: app.map(str::to_string), ..Disk::default() }
    }

    const RUN: &str = "\x1b[1mDone\x1b[0m\n\n\x1b[1mBuilding armada from /src\x1b[0m\nrestart: leave out armada/x\n\n\x1b[1mKeeping the build in /b\x1b[0m\n\n\x1b[1mBooting gui/501/com.armada.fleet out and back in from /p\x1b[0m\n";

    #[test]
    fn fleet_answering_on_our_protocol_draws_no_panel_even_mid_restart() {
        let connected = Connection::Connected { pid: 1, port: 2, protocol: "p".into(), up: None };
        let restart = Disk { restart: Some(at(0, 39, 0)), ..Disk::default() };
        assert_eq!(panel_of(&connected, &restart, at(0, 39, 0), at(0, 45, 0)), None);
    }

    #[test]
    fn unreachable_reading_and_coming_up_draw_no_panel() {
        let unreachable = Connection::Unreachable { pid: 1, port: 2, silent: "2m".into(), last_read: None };
        for connection in [unreachable, Connection::Reading, Connection::Connecting { pid: 1, port: 2 }, Connection::Starting { pid: 1, port: 2 }] {
            assert_eq!(panel_of(&connection, &Disk::default(), at(0, 0, 0), at(1, 0, 0)), None, "{connection:?}");
        }
    }

    #[test]
    fn a_live_restart_lock_is_restarting_with_its_steps_and_no_button() {
        let disk = Disk { restart: Some(at(0, 39, 0)), restart_log: Some(RUN.into()), ..kept(Some(CURRENT)) };
        for connection in [down(), Connection::Connecting { pid: 1, port: 2 }, mismatch(None)] {
            let panel = panel_of(&connection, &disk, at(0, 39, 0), at(0, 39, 48)).expect("a panel");
            assert_eq!((panel.tone, panel.title, panel.since.as_str(), panel.act, panel.log), (Tone::Restarting, "Fleet restarting", "48 s", None, Some(Log::Restart)));
            assert_eq!(panel.note, None, "an empty slot stays empty");
            assert_eq!(panel.link, "restart under way");
        }
    }

    #[test]
    fn a_step_shows_its_lines_first_clause() {
        assert_eq!(clause_of("Snapshot of the database: /Users/user/Library/Application Support/Armada/backups/armada-20261113T004910Z.db"), "Snapshot of the database");
        assert_eq!(clause_of("Keeping the build in /Users/user/Library/Application Support/Armada/builds/x"), "Keeping the build");
        assert_eq!(clause_of("Fleet is running (pid 4120, port 40000) — checking for a working Drone"), "Fleet is running (pid 4120, port 40000)");
        assert_eq!(clause_of("Booting gui/501/com.armada.fleet out and back in from /Users/user/x.plist"), "Booting gui/501/com.armada.fleet out and back in");
        assert_eq!(clause_of("No Drone is working"), "No Drone is working");
    }

    #[test]
    fn a_known_restart_step_shows_its_short_name_and_an_unknown_one_its_first_clause() {
        assert_eq!(step_name("Booting gui/501/com.armada.fleet out and back in from /Users/user/x.plist"), "Restarting Fleet");
        assert_eq!(step_name("Adopting every working Drone — --adopt skips the refusal"), "Running Jobs keep running");
        assert_eq!(step_name("Keeping the build in /Users/user/Library/Application Support/Armada/builds/x"), "Keeping the build");
        assert_eq!(step_name("leave out armada/x"), "leave out armada/x");
    }

    #[test]
    fn steps_are_the_last_lines_of_the_run_the_last_one_now() {
        let steps = steps_of(RUN);
        let read: Vec<(Mark, &str)> = steps.iter().map(|step| (step.mark, step.text.as_str())).collect();
        assert_eq!(read, [(Mark::Done, "leave out armada/x"), (Mark::Done, "Keeping the build"), (Mark::Now, "Restarting Fleet"), (Mark::Next, REOPENS)]);
        assert_eq!(steps[1].line, "Keeping the build in /b", "the tooltip carries the line whole");
        let reopening = steps_of("\x1b[1mReopening the native Bridge from /b\x1b[0m\n");
        assert_eq!(reopening.iter().map(|step| step.mark).collect::<Vec<_>>(), [Mark::Now], "no next where the step now is the reopening");
        assert!(steps_of("").is_empty());
    }

    #[test]
    fn stopped_waits_out_the_relaunch_throttle() {
        assert_eq!(panel_of(&down(), &Disk::default(), at(0, 39, 0), at(0, 39, 1)), None);
        let panel = panel_of(&down(), &Disk::default(), at(0, 39, 0), at(0, 45, 10)).expect("stopped");
        assert_eq!((panel.tone, panel.title, panel.act, panel.link), (Tone::Stopped, "Fleet stopped", Some(Act::StartFleet), "no answer"));
        assert_eq!(panel.since, "since 00:39 · 6 min");
    }

    #[test]
    fn a_restart_that_did_not_finish_gives_the_last_words_and_the_cause() {
        let disk = Disk { restart_log: Some(format!("{RUN}restart: Booting com.armada.fleet out and back in\n\n")), fleet_log: Some("fleet: listening\n".into()), ..Disk::default() };
        let panel = panel_of(&down(), &disk, at(0, 39, 0), at(0, 45, 0)).expect("stopped");
        assert_eq!(panel.fleet.body, Body::LastWords { from: "restart.log", line: "restart: Booting com.armada.fleet out and back in".into() });
        assert_eq!(panel.why, "The restart that stopped it did not finish. Nothing on screen is live until Fleet is back.");
        assert_eq!(panel.log, Some(Log::Restart));
    }

    #[test]
    fn with_no_restart_or_a_finished_one_fleets_own_log_speaks() {
        let finished = Disk { restart_log: Some(format!("{RUN}\x1b[1mDone\x1b[0m\n")), fleet_log: Some("one\n\x1b[31mpanicked at store.rs\x1b[0m\n\n".into()), ..Disk::default() };
        let none = Disk { restart_log: None, ..finished.clone() };
        for disk in [finished, none] {
            let panel = panel_of(&down(), &disk, at(0, 39, 0), at(0, 45, 0)).expect("stopped");
            assert_eq!(panel.fleet.body, Body::LastWords { from: "Fleet's log", line: "panicked at store.rs".into() });
            assert_eq!(panel.why, "Nothing on screen is live until Fleet is back.");
            assert_eq!(panel.log, Some(Log::Fleet));
        }
        let silent = panel_of(&down(), &Disk::default(), at(0, 39, 0), at(0, 45, 0)).expect("stopped");
        assert_eq!(silent.fleet.body, Body::Empty);
    }

    #[test]
    fn a_refused_runtime_file_is_stopped_with_its_detail_as_last_words() {
        let refused = Connection::Refused { detail: "the pid probe failed: no ps · fleet.json".into() };
        assert_eq!(panel_of(&refused, &Disk::default(), at(0, 39, 0), at(0, 39, 1)), None);
        let panel = panel_of(&refused, &Disk::default(), at(0, 39, 0), at(0, 40, 0)).expect("stopped");
        assert_eq!(panel.tone, Tone::Stopped);
        assert_eq!(panel.fleet.body, Body::LastWords { from: "fleet.json", line: "the pid probe failed: no ps · fleet.json".into() });
    }

    #[test]
    fn fleet_off_current_is_fleet_older_with_restart_fleet() {
        let panel = panel_of(&mismatch(Some(OLDER)), &kept(Some(CURRENT)), at(19, 49, 0), at(20, 0, 0)).expect("older");
        assert_eq!((panel.tone, panel.title, panel.act, panel.note), (Tone::Older, "Fleet runs an older build", Some(Act::RestartFleet), Some("Running Jobs keep running")));
        assert_eq!((panel.fleet.tone, panel.fleet.state, panel.app.tone, panel.app.state), (Some(Tone::Older), "Older build", None, "Running"));
        let Body::Builds { build: Some(build), protocol } = &panel.fleet.body else { panic!("builds") };
        assert_eq!((build.commit.as_str(), build.current, protocol.as_str()), ("2f88990", false, "af0a982d"));
        let Body::Builds { build: Some(app), .. } = &panel.app.body else { panic!("builds") };
        assert!(app.current);
        assert_eq!((panel.since.as_str(), panel.link, panel.log), ("since 19:49", "protocols differ", None));
    }

    #[test]
    fn the_app_off_current_is_app_older_with_reopen_armada() {
        let panel = panel_of(&mismatch(Some(CURRENT)), &kept(Some(OLDER)), at(0, 49, 0), at(0, 50, 0)).expect("older");
        assert_eq!((panel.title, panel.act, panel.note), ("This app runs an older build", Some(Act::ReopenApp), Some("Fleet and running Jobs keep running")));
        assert_eq!((panel.fleet.tone, panel.app.tone, panel.app.state), (None, Some(Tone::Older), "Older build"));
    }

    #[test]
    fn both_off_current_or_neither_kept_restarts_fleet_and_shows_both_protocols() {
        let both = panel_of(&mismatch(Some(OLDER)), &kept(Some(OLDER)), at(0, 0, 0), at(0, 1, 0)).expect("older");
        let neither = panel_of(&mismatch(None), &Disk::default(), at(0, 0, 0), at(0, 1, 0)).expect("older");
        for panel in [both, neither] {
            assert_eq!((panel.title, panel.act), ("Fleet and this app do not match", Some(Act::RestartFleet)));
            assert_eq!((panel.fleet.tone, panel.app.tone), (Some(Tone::Older), Some(Tone::Older)));
            assert!(matches!(&panel.fleet.body, Body::Builds { protocol, .. } if protocol == "af0a982d"));
            assert!(matches!(&panel.app.body, Body::Builds { protocol, .. } if *protocol == protocol_said(ipc::ProtocolId::current().as_str())));
        }
        let neither = panel_of(&mismatch(None), &Disk::default(), at(0, 0, 0), at(0, 1, 0)).expect("older");
        assert_eq!((neither.fleet.state, neither.app.state), ("Running", "Running"), "no kept build is not called older");
    }

    #[test]
    fn a_kept_builds_name_is_its_utc_build_time_and_commit() {
        let (built, commit) = kept_of(CURRENT).expect("a kept build");
        assert_eq!(built, Utc.with_ymd_and_hms(2026, 11, 13, 0, 48, 52).single().expect("a time"));
        assert_eq!(commit, "126b4bb");
        assert_eq!(build_of(Some(CURRENT), Some(CURRENT)).map(|build| build.at), Some(clock(built.with_timezone(&Local))));
        for wrong in ["current", "2026-11-13T004852-126b4bb", "2026-11-13T004852Z-", "13 Nov Z-126b4bb"] {
            assert_eq!(kept_of(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn colour_codes_come_out_of_a_log_line() {
        assert_eq!(plain("\x1b[1mKeeping the build\x1b[0m"), "Keeping the build");
        assert_eq!(plain("\x1b[38;5;196mred\x1b[0m and plain"), "red and plain");
        assert_eq!(last_line("one\n\x1b[1mtwo\x1b[0m\n\n  \n"), Some("two".into()));
        assert_eq!(last_line("\n\n"), None);
    }
}
