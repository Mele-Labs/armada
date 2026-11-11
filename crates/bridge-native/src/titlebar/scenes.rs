//! The title bar's mock scenario: every state in the inventory, reached by name.
//!
//! A walk step names a scene and the bar's face changes to it, so a step goes straight to the state
//! it shows. The reports are JSON decoded through `ipc` as Fleet's own answers are, so a field that
//! no longer matches the wire fails here and not on screen.

use gpui::{App, Global, SharedString};
use ipc::{FleetBuildReport, ModelDownshift};

use super::face::{
    self, connection_of, Band, Build, Connection, Doctor, Dropped, Face, FleetFace, Limit, Link, Outcome, Place, Power, Restarting, RunsPlace, RunsPlaceKind, Usage,
};
use super::{live, TitleBar};
use crate::runtime_file::{Presence, RuntimeFile};

/// The face a bar opens with while a walk is playing, so a screen drawn fresh starts at rest.
pub struct Opening(pub Face);

impl Global for Opening {}

const PATH: &str = "/Users/user/Library/Application Support/Armada/fleet.json";
/// A window narrower than the row's 900px bound.
const NARROW: gpui::Pixels = gpui::px(720.);
const UNTIL: &str = "13 Nov, 09:00";

fn report(json: &str) -> FleetBuildReport {
    ipc::decode::<FleetBuildReport>("FleetBuildReport", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's build report does not decode: {why}"))
}

fn file(protocol: &str) -> RuntimeFile {
    let json = format!(r#"{{"protocol_id":"{protocol}","pid":41872,"port":52110,"started_at":"Mon Nov  9 08:14:02 2026"}}"#);
    ipc::decode::<RuntimeFile>("runtime file", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's runtime file does not decode: {why}"))
}

fn running_file() -> RuntimeFile {
    file(&ipc::ProtocolId::current().to_string())
}

const RUNNING: &str = r#""running":{"built_at":"2026-11-05T17:05:00Z","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b"}"#;
const PREVIOUS: &str = r#""previous":{"built_at":"2026-11-04T09:30:00Z","commit":"4be1c07a52d9e8f6a3b2c1d0e9f8a7b6c5d4e3f2","bridge_reopens":true,"takes_out":["Quiet the write-clash notices"],"brings_back":["Checkout page loads twice"]}"#;

fn build(json: String, aligned: bool, adopts: &[&str]) -> Build {
    Build { report: report(&json), aligned, adopts: adopts.iter().map(|one| one.to_string()).collect() }
}

fn main_aligned() -> Build {
    build(format!(r#"{{"on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","position":{{"ahead":0,"behind":0}},{RUNNING},{PREVIOUS},"carried":["Native Bridge foundation"]}}"#), true, &[])
}

fn main_behind() -> Build {
    build(format!(r#"{{"on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","position":{{"ahead":0,"behind":3}},{RUNNING},{PREVIOUS}}}"#), false, &[])
}

const LEFT_OUT: &str = r#""left_out":[{"branch":"armada/session-2c7f91d0","why":"merge conflict in crates/fleet/src/building.rs","moved":true},{"branch":"armada/fix-slot-pool","why":"does not merge with armada/session-4f03266c"}]"#;

fn preview() -> Build {
    build(format!(r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},{RUNNING},{PREVIOUS},{LEFT_OUT}}}"#), false, &[])
}

fn restarting(stage: &str) -> Build {
    build(
        format!(r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},"restarting":"preview","stage":"{stage}",{RUNNING},{PREVIOUS}}}"#),
        false,
        &[],
    )
}

fn picked_and_ready() -> Build {
    build(
        format!(
            r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},{RUNNING},{PREVIOUS},"picked":[{{"branch":"armada/session-4f03266c","title":"Native title bar"}},{{"branch":"armada/fix-merge-line","title":"Fix the merge line freshness chip"}}],"snapshot":{{"state":"ready","built_at":"2026-11-09T08:40:00Z","commit":"1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d","jobs":["Native title bar","Fix the merge line freshness chip"],"bridge_reopens":true}}}}"#
        ),
        false,
        &[],
    )
}

fn trying() -> Build {
    build(
        format!(
            r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},{RUNNING},{PREVIOUS},"trying":["Quiet the write-clash notices","Fix the merge line freshness chip"]}}"#
        ),
        false,
        &[],
    )
}

fn drones() -> Build {
    build(
        format!(r#"{{"on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","position":{{"ahead":0,"behind":3}},{RUNNING},{PREVIOUS}}}"#),
        false,
        &["Native Bridge foundation", "Audit the docs index"],
    )
}

fn failed() -> Build {
    build(
        format!(
            r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},"failed":"scripts/preview stopped: cargo build failed in crates/fleet",{RUNNING},{PREVIOUS}}}"#
        ),
        false,
        &[],
    )
}

fn reverting() -> Build {
    build(
        format!(r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},{RUNNING},{PREVIOUS},"reverting":true}}"#),
        false,
        &[],
    )
}

fn swapping() -> Build {
    build(
        format!(r#"{{"on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{{"ahead":5,"behind":2}},{RUNNING},{PREVIOUS},"swapping":true}}"#),
        false,
        &[],
    )
}

fn swapped_back() -> Build {
    build(
        format!(
            r#"{{"on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","position":{{"ahead":0,"behind":0}},{RUNNING},{PREVIOUS},"failed":"Swapped back, since the new Fleet did not answer: No new Fleet answered in 90 s"}}"#
        ),
        true,
        &[],
    )
}

/// A build as the live bar reads Fleet's report (`face::build_of`): level with main and the Jobs a restart carries
/// come from the report, not from the scene.
fn live_build(rest: &str) -> Build {
    face::build_of(&report(&format!(r#"{{{rest},{RUNNING},{PREVIOUS}}}"#)))
}

const LIVE_MAIN: &str = r#""on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b""#;
const LIVE_PREVIEW: &str = r#""on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{"ahead":5,"behind":2},"switcher":{"aligned":false,"idle_on_main":false,"restart":"refresh_preview"}"#;

fn build_live(name: &str) -> Option<Build> {
    Some(match name {
        "build-live-main" => live_build(&format!(r#"{LIVE_MAIN},"position":{{"ahead":0,"behind":0}},"switcher":{{"aligned":true,"idle_on_main":true,"restart":"update_to_main"}}"#)),
        "build-live-preview" => live_build(&format!("{LIVE_PREVIEW},{LEFT_OUT}")),
        "build-live-refused" => live_build(&format!(r#"{LIVE_PREVIEW},"failed":"no served repository holds scripts/restart-build""#)),
        "build-live-carried" => live_build(&format!(
            r#"{LIVE_MAIN},"position":{{"ahead":0,"behind":3}},"switcher":{{"aligned":false,"idle_on_main":false,"restart":"update_to_main"}},"carried":["Native Bridge foundation","Audit the docs index"]"#
        )),
        "build-live-trying" => live_build(&format!(r#"{LIVE_PREVIEW},"trying":["Quiet the write-clash notices","Fix the merge line freshness chip"]"#)),
        _ => return None,
    })
}

fn limits(five: (u32, Band, &str), week: (u32, Band, &str)) -> Usage {
    Usage::Limits(vec![Limit::five_hour(five.0, five.1, five.2), Limit::weekly(week.0, week.1, week.2)])
}

/// Doctor's line follows how Fleet is: all passed while it runs, none read while it is away.
fn fleet(connection: Connection, power: Power) -> FleetFace {
    let (outcome, modules) = match &connection {
        Connection::Connected { .. } => (Outcome::Pass, "14 modules read"),
        Connection::Unreachable { .. } => (Outcome::Warn, "11 of 14 modules read"),
        Connection::Starting { .. } | Connection::Connecting { .. } | Connection::Reading => (Outcome::Reading, "reading"),
        _ => (Outcome::Fail, "0 of 14 modules read"),
    };
    FleetFace { connection, power, restarting: Restarting::Idle, doctor: Some(Doctor { outcome, modules: SharedString::from(modules) }) }
}

fn connected() -> Connection {
    connection_of(&Presence::Running(running_file()), Link::Connected { up: Some("3h 12m".into()) }, PATH)
}

fn offered() -> ModelDownshift {
    ModelDownshift::Offered { percent: 84, was: "Opus".into(), to: "Sonnet".into() }
}

/// The bar as the live app draws Fleet's answers: each window `(percent, band, seconds to its reset)`, a band
/// of `""` leaving the windows out; `read` whether Fleet has read the plan at all.
fn plan_live(face: &mut Face, five: (u32, &str, i64), seven: (u32, &str, i64), shift: Option<&str>, read: bool) {
    let now = chrono::Utc::now();
    // Half a minute on, so the countdown reads the whole minute it was set to.
    let at = |seconds: i64| (now + chrono::Duration::seconds(seconds + 30)).to_rfc3339();
    let window = |kind: &str, (percent, band, seconds): (u32, &str, i64)| format!(r#"{{"kind":"{kind}","percent_used":{percent},"resets_at":"{}","band":"{band}"}}"#, at(seconds));
    let windows = if five.1.is_empty() { String::new() } else { format!("{},{}", window("five_hour", five), window("seven_day", seven)) };
    let read_at = if read { format!(r#","read_at":"{}""#, now.to_rfc3339()) } else { String::new() };
    let usage = ipc::decode::<ipc::PlanUsage>("PlanUsage", format!(r#"{{"windows":[{windows}]{read_at}}}"#).as_bytes()).unwrap_or_else(|why| panic!("the plan usage does not decode: {why}"));
    let state = shift.map(|shift| ipc::decode::<ipc::ModelDownshiftState>("ModelDownshiftState", format!(r#"{{"downshift":{shift}}}"#).as_bytes()).unwrap_or_else(|why| panic!("the step down does not decode: {why}")));
    face.usage = face::usage_of(Some(&usage), now);
    face.downshift = face::downshift_of(state.as_ref(), Some(&usage));
}

/// The bar at rest: Fleet running on an aligned main, the plan windows low.
pub fn rest() -> Face {
    // The repositories and projects are what Fleet would serve, put through the reading the live bar uses.
    let served = crate::mock::repositories();
    let mut repositories = face::repositories_of(&served, &[]);
    let kept = [RunsPlaceKind::Repository, RunsPlaceKind::Mac, RunsPlaceKind::Synced];
    for (one, kind) in repositories.iter_mut().zip(kept) {
        let detail = (kind == RunsPlaceKind::Synced).then(|| SharedString::from("iCloud Drive"));
        one.kept = Some(RunsPlace { kind, detail });
    }
    Face {
        usage: limits((42, Band::Normal, "2h 14m"), (71, Band::Normal, "2d 17h")),
        build: Some(main_aligned()),
        fleet: fleet(connected(), Power::Live),
        repositories,
        chosen: None,
        projects: face::projects_of(&served, &crate::mock::projects()),
        scoped: Some(0),
        signed_in: true,
        downshift: None,
        places: vec![Place { id: "jobs", label: "Jobs", glyph: "box", shown: true, hideable: true }],
        here: Some("jobs"),
        dropped: Dropped { missed: 0, noted: 0 },
    }
}

/// Start a walk's bar at rest: the face a fresh bar opens with, and the one already open.
pub fn begin(cx: &mut App) {
    cx.set_global(Opening(rest()));
    if let Some(bar) = live(cx) {
        bar.update(cx, |bar: &mut TitleBar, cx| {
            bar.reset(rest());
            cx.notify();
        });
    }
}

/// Move the bar to a named scene. A scene changes the part of the face it is about and leaves the rest.
pub fn apply(name: &str, cx: &mut App) -> Result<(), String> {
    // A window's width is the bar's own, not the face's: a resize is the one thing here that is not data.
    let resized = match name {
        "narrow-window" => Some((Some(NARROW), None)),
        "back-to-rest" => Some((None, Some(true))),
        _ => None,
    };
    if let Some((width, signed_in)) = resized {
        let bar = live(cx).ok_or("no title bar is drawn yet")?;
        bar.update(cx, |bar: &mut TitleBar, cx| {
            bar.width = width;
            if let Some(signed_in) = signed_in {
                bar.face.signed_in = signed_in;
            }
            cx.notify();
        });
        return Ok(());
    }
    let change: fn(&mut Face) = match name {
        "rings-loading" => |face| face.usage = Usage::Loading,
        "rings-normal" => |face| {
            face.usage = limits((42, Band::Normal, "2h 14m"), (71, Band::Normal, "2d 17h"));
            face.downshift = None;
        },
        "rings-warning" => |face| face.usage = limits((62, Band::Normal, "1h 40m"), (84, Band::Warning, "2d 17h")),
        "rings-danger" => |face| face.usage = limits((97, Band::Danger, "9m"), (96, Band::Danger, "17h 3m")),
        "rings-off" => |face| face.usage = Usage::Hidden,
        "offer" => |face| {
            face.usage = limits((62, Band::Normal, "1h 40m"), (84, Band::Warning, "2d 17h"));
            face.downshift = Some((offered(), UNTIL.into()));
        },
        // Fleet's reading and step down, through the reading the live bar makes of them (`face::usage_of`).
        "plan-live" => |face| plan_live(face, (42, "normal", 2 * 3600 + 14 * 60), (71, "normal", 2 * 86_400 + 17 * 3600), None, true),
        "plan-live-loading" => |face| plan_live(face, (0, "", 0), (0, "", 0), None, false),
        "plan-live-off" => |face| plan_live(face, (0, "", 0), (0, "", 0), None, true),
        "plan-live-offered" => |face| {
            plan_live(face, (62, "normal", 3600 + 40 * 60), (84, "warning", 2 * 86_400 + 17 * 3600), Some(r#"{"phase":"offered","percent":84,"was":"Opus","to":"Sonnet"}"#), true)
        },
        "plan-live-reset" => |face| plan_live(face, (8, "normal", 4 * 3600 + 51 * 60), (3, "normal", 6 * 86_400 + 23 * 3600), Some(r#"{"phase":"reset","was":"Opus","to":"Sonnet"}"#), true),
        "offer-reset" => |face| {
            face.usage = limits((8, Band::Normal, "4h 51m"), (3, Band::Normal, "6d 23h"));
            face.downshift = Some((ModelDownshift::Reset { was: "Opus".into(), to: "Sonnet".into() }, UNTIL.into()));
        },
        "build-main-behind" => |face| face.build = Some(main_behind()),
        "build-preview" => |face| face.build = Some(preview()),
        "build-restarting" => |face| face.build = Some(restarting("building_fleet")),
        "build-picked" => |face| face.build = Some(picked_and_ready()),
        "build-trying" => |face| face.build = Some(trying()),
        "build-drones" => |face| face.build = Some(drones()),
        "build-failed" => |face| face.build = Some(failed()),
        "build-reverting" => |face| face.build = Some(reverting()),
        "build-swapping" => |face| face.build = Some(swapping()),
        "build-swapped-back" => |face| face.build = Some(swapped_back()),
        "events-dropped" => |face| face.dropped = Dropped { missed: 3, noted: 0 },
        "events-noted" => |face| face.dropped = Dropped { missed: 3, noted: 3 },
        "signed-out" => |face| face.signed_in = false,
        "signed-in" => |face| face.signed_in = true,
        "fleet-running" => |face| face.fleet = fleet(connected(), Power::Live),
        "fleet-unreachable" => |face| {
            let link = Link::Unreachable { silent: "2m".into(), last_read: Some("2m".into()) };
            face.fleet = fleet(connection_of(&Presence::Running(running_file()), link, PATH), Power::Fault);
        },
        "fleet-starting" => |face| face.fleet = fleet(connection_of(&Presence::Running(running_file()), Link::Starting, PATH), Power::Standby),
        "fleet-powering" => |face| face.fleet = fleet(connection_of(&Presence::Absent, Link::Connecting, PATH), Power::Standby),
        "fleet-down" => |face| face.fleet = fleet(connection_of(&Presence::Absent, Link::Connecting, PATH), Power::Fault),
        "fleet-mismatch" => |face| face.fleet = fleet(connection_of(&Presence::Running(file("0000000000000000")), Link::Connecting, PATH), Power::Fault),
        "restart-apart" => |face| {
            face.fleet = fleet(connection_of(&Presence::Running(file("0000000000000000")), Link::Connecting, PATH), Power::Fault);
            face.fleet.restarting = Restarting::Apart;
        },
        "restart-no-answer" => |face| {
            face.fleet = fleet(connection_of(&Presence::Running(file("0000000000000000")), Link::Connecting, PATH), Power::Fault);
            face.fleet.restarting = Restarting::NoAnswer;
        },
        // Fleet's build report through the reading the live bar makes of it (`face::build_of`).
        "build-live-main" => |face| face.build = build_live("build-live-main"),
        "build-live-preview" => |face| face.build = build_live("build-live-preview"),
        "build-live-refused" => |face| face.build = build_live("build-live-refused"),
        "build-live-carried" => |face| face.build = build_live("build-live-carried"),
        "build-live-trying" => |face| face.build = build_live("build-live-trying"),
        other => return Err(format!("no scene named {other}")),
    };
    let bar = live(cx).ok_or("no title bar is drawn yet")?;
    bar.update(cx, |bar: &mut TitleBar, cx| {
        change(&mut bar.face);
        bar.faced(cx);
    });
    Ok(())
}
