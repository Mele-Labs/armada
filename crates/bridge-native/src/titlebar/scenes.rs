//! The title bar's mock scenario: every state in the inventory, reached by name.
//!
//! A walk step names a scene and the bar's face changes to it, so a step goes straight to the state
//! it shows. The reports are JSON decoded through `ipc` as Fleet's own answers are, so a field that
//! no longer matches the wire fails here and not on screen.

use gpui::{App, Global, SharedString};
use ipc::{FleetBuildReport, ModelDownshift};

use super::face::{
    connection_of, Band, Build, Connection, Doctor, Dropped, Face, FleetFace, Limit, Link, Outcome, Place, Power, Repository, Restarting, Usage, Workspace,
};
use super::{live, TitleBar};
use crate::runtime_file::{Presence, RuntimeFile};

/// The face a bar opens with while a walk is playing, so a screen drawn fresh starts at rest.
pub struct Opening(pub Face);

impl Global for Opening {}

const PATH: &str = "/Users/nick/Library/Application Support/Armada/fleet.json";
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

/// The bar at rest: Fleet running on an aligned main, the plan windows low.
pub fn rest() -> Face {
    Face {
        usage: limits((42, Band::Normal, "2h 14m"), (71, Band::Normal, "2d 17h")),
        build: Some(main_aligned()),
        fleet: fleet(connected(), Power::Live),
        repositories: vec![
            Repository { name: "armada".into(), not_set_up: false },
            Repository { name: "sample-web".into(), not_set_up: false },
            Repository { name: "sample-api".into(), not_set_up: false },
            Repository { name: "sample-docs".into(), not_set_up: true },
        ],
        chosen: Some(0),
        workspaces: vec![Workspace { name: "Storefront".into(), members: vec![1, 2] }],
        scoped: None,
        signed_in: true,
        manifest: true,
        downshift: None,
        places: vec![Place { id: "sessions", label: "Jobs", glyph: "square-terminal", shown: true, hideable: true }],
        here: Some("sessions"),
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
        other => return Err(format!("no scene named {other}")),
    };
    let bar = live(cx).ok_or("no title bar is drawn yet")?;
    bar.update(cx, |bar: &mut TitleBar, cx| {
        change(&mut bar.face);
        cx.notify();
    });
    Ok(())
}
