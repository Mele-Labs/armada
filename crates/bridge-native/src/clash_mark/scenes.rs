//! The write clashes of the mock scenario: what `write_clashes.changed` would carry, by name. JSON
//! decoded through `ipc` as Fleet's own answer is. The Jobs are the Jobs list's mock Jobs.

use std::rc::Rc;

use gpui::App;
use ipc::AllWriteClashes;

use super::{hold, OpenHolder, OpenPath};

const SINCE: &str = "2026-11-09T09:40:00Z";

fn clash(one: (&str, &str), other: (&str, &str, &str), paths: &[&str]) -> String {
    let (kind, id, label) = other;
    format!(
        r#"{{"one":{{"kind":"session","id":"{}","label":"{}"}},"other":{{"kind":"{kind}","id":"{id}","label":"{label}"}},"paths":[{}],"since":"{SINCE}"}}"#,
        one.0,
        one.1,
        paths.iter().map(|path| format!("\"{path}\"")).collect::<Vec<_>>().join(",")
    )
}

fn all(clashes: Vec<String>) -> AllWriteClashes {
    let json = format!(r#"{{"repositories":[{{"manifest_id":"armada","clashes":[{}]}}]}}"#, clashes.join(","));
    ipc::decode::<AllWriteClashes>("AllWriteClashes", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's clashes do not decode: {why}"))
}

const FOUNDATION: (&str, &str) = ("mock-02", "Native Bridge foundation");
const NOTICES: (&str, &str, &str) = ("session", "mock-06", "Quiet the write-clash notices");
const POCKET: (&str, &str, &str) = ("session", "mock-05", "Pocket sign-in on the phone");
const MAIN: (&str, &str, &str) = ("main", "main", "main");

/// Nothing stands: no mark is drawn.
pub fn none() -> AllWriteClashes {
    all(Vec::new())
}

/// Two Jobs whose changes collide: the mark stands on both rows.
fn pair() -> AllWriteClashes {
    all(vec![clash(FOUNDATION, NOTICES, &["crates/bridge-native/src/jobs.rs", "crates/ipc/src/clashes.rs"])])
}

/// One pair whose changes collide in five files: the card folds the list to its first three.
fn files() -> AllWriteClashes {
    all(vec![clash(
        FOUNDATION,
        NOTICES,
        &[
            "crates/bridge-native/src/jobs.rs",
            "crates/bridge-native/src/clash_mark.rs",
            "crates/ipc/src/clashes.rs",
            "crates/fleet/src/write_clashes/detect.rs",
            "docs/concepts/write-clashes.md",
        ],
    )])
}

/// Every kind at once, on four rows. A Job in two clashes, one with another Job and one with a main
/// that moved, wears one mark and a card; the other Job of the pair wears a button; a Job against a
/// main alone wears a picture, since there is nothing of main to open.
fn states() -> AllWriteClashes {
    all(vec![
        clash(FOUNDATION, NOTICES, &["crates/bridge-native/src/jobs.rs", "crates/ipc/src/clashes.rs"]),
        clash(FOUNDATION, MAIN, &["crates/bridge-native/src/jobs.rs"]),
        clash(FOUNDATION, POCKET, &["crates/bridge-native/src/main.rs"]),
        clash(("mock-04", "Audit the docs index"), MAIN, &["docs/INDEX.md", "docs/OPEN.md", "docs/scope.md", "docs/concepts/fleet.md"]),
    ])
}

/// Start a walk with nothing clashing.
pub fn begin(cx: &mut App) {
    hold(none(), cx);
}

pub fn clear(cx: &mut App) {
    if cx.has_global::<super::Held>() {
        cx.remove_global::<super::Held>();
    }
    if cx.has_global::<OpenHolder>() {
        cx.remove_global::<OpenHolder>();
    }
    if cx.has_global::<OpenPath>() {
        cx.remove_global::<OpenPath>();
    }
}

/// What a press on a mark does in a walk, in place of the Jobs screen's own: press the other Job's row.
pub fn open_by_press(cx: &mut App) {
    // A path is asked for and nothing opens: this walk has no files panel for it to land in.
    cx.set_global(OpenPath(Rc::new(|_, _, _| {})));
    cx.set_global(OpenHolder(Rc::new(|holder, window, cx| {
        let label = holder.label.clone();
        window.defer(cx, move |window, cx| crate::walk::stage::press_row(&label, window, cx));
    })));
}

pub fn apply(name: &str, cx: &mut App) -> Result<(), String> {
    let next = match name {
        "clash-pair" => pair(),
        "clash-states" => states(),
        "clash-files" => files(),
        "clash-none" => none(),
        other => return Err(format!("no clash scene named {other}")),
    };
    hold(next, cx);
    Ok(())
}
