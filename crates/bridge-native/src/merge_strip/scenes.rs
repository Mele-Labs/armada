//! The strip's mock scenario: every state in the inventory, reached by name. The views and calls are
//! JSON decoded through `ipc` as Fleet's own answers are, so a field that no longer matches the wire
//! fails here and not on screen. Instants are set from now, so an age reads as it would.

use chrono::{Duration, SecondsFormat, Utc};
use gpui::{App, SharedString};
use ipc::{MergeLineView, StandingCall};

use super::{live, Held, Line, MergeStrip, Opening, Strip, Tone};

fn ago(seconds: i64) -> String {
    (Utc::now() - Duration::seconds(seconds)).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn view(json: String) -> MergeLineView {
    ipc::decode::<MergeLineView>("MergeLineView", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's merge line view does not decode: {why}"))
}

fn call(json: String) -> StandingCall {
    ipc::decode::<StandingCall>("StandingCall", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's call does not decode: {why}"))
}

/// One pull request as Fleet's view lists it.
fn pull(number: u64, title: &str, branch: &str, author: &str, rest: &str) -> String {
    format!(
        r#"{{"number":{number},"title":"{title}","branch":"{branch}","url":"https://github.com/Mele-Labs/armada/pull/{number}","author":"{author}","opened_at":"{}","updated_at":"{}"{rest}}}"#,
        ago(3 * 3600 + 20 * 60),
        ago(6 * 60)
    )
}

const QUEUE: &str = "{\"state\":\"%S\",\"position\":%P,\"enqueued_at\":\"%T\"}";

fn queue(state: &str, position: u32) -> String {
    QUEUE.replace("%S", state).replace("%P", &position.to_string()).replace("%T", &ago(25 * 60))
}

/// The armada line: five in the queue by place, the rest open, a draft that stays out of the line.
fn armada(main: &str, checkout: &str, read: i64, quiet: Option<i64>) -> MergeLineView {
    let pulls = [
        pull(3395, "Port the title bar to gpui", "native/title-bar", "nick", &format!(r#","ci":"passed","queue":{},"mergeable":"clean","review":"approved""#, queue("mergeable", 1))),
        pull(3392, "Quiet the write-clash notices", "armada/quiet-clashes", "nick", &format!(r#","ci":"running","queue":{}"#, queue("awaiting_checks", 2))),
        pull(3390, "Name the native pieces Jobs screens mounts", "armada/session-4f03266c", "nick", &format!(r#","ci":"passed","queue":{}"#, queue("queued", 3))),
        pull(3376, "Bump the toolchain", "dependabot/toolchain", "rosa", &format!(r#","queue":{},"mine":false"#, queue("in_queue", 4))),
        pull(3388, "Fix the merge line freshness chip", "armada/merge-line-chip", "nick", r#","ci":"passed","mergeable":"clean","review":"approved""#),
        pull(3384, "Dispatch sheet notes", "native/dispatch", "nick", r#","ci":"running","mergeable":"clean""#),
        pull(3381, "Checkout page loads twice", "armada/checkout-twice", "nick", r#","ci":"failed","mergeable":"conflicting","review":"changes_requested""#),
        pull(3377, "Retry webhooks on a 503", "armada/retry-webhooks", "nick", r#","ci":"waiting_on_main""#),
        pull(3366, "Add the form edit to armada.yml", "armada/form-edit", "nick", r#","merge_by_hand":true"#),
        pull(3350, "Audit the docs index", "armada/audit-docs", "nick", r#","ci":"running","queue":{"state":"waiting_for_ci"}"#),
        pull(3371, "Draft: the lamps", "armada/lamps", "nick", r#","draft":true"#),
    ]
    .join(",");
    let quiet = quiet.map(|seconds| format!(r#","quiet_since":"{}""#, ago(seconds))).unwrap_or_default();
    view(format!(
        r#"{{"root":"/Users/nick/Development/armada","name":"armada"{main}{checkout},"pulls":[{pulls}],"read_at":"{}"{quiet}}}"#,
        ago(read)
    ))
}

/// Main red on `native`, broken by #3386; `more` adds fields to the red: the Job on it, newer checks running.
fn red(more: &str) -> String {
    format!(
        r#","main":{{"state":"red","failed":{{"check":"native","test":"jobs::tests::a_row_carries_its_issue"}},"also":[{{"check":"ci / lint","unmapped":true}}],"since":"2026-11-09T09:14:00Z","merge":{{"number":3386,"branch":"armada/native-rows","session":{{"id":"mock-02","title":"Native Bridge foundation"}}}}{more}}}"#
    )
}

const GREEN: &str = r#","main":{"state":"green"}"#;
const CHECKOUT: &str = r#","checkout":{"base":"main","behind":3,"ahead":0,"uncommitted":["crates/fleet/src/merge_hub.rs"],"clashing":[]}"#;

fn held() -> Vec<Held> {
    let job = |id: &str, title: &str| Some((SharedString::from(id.to_string()), SharedString::from(title.to_string())));
    vec![
        Held { number: 3395, job: job("mock-02", "Native Bridge foundation"), tone: Tone::Calm },
        Held { number: 3392, job: job("mock-06", "Quiet the write-clash notices"), tone: Tone::Calm },
        Held { number: 3388, job: job("mock-01", "Fix the merge line freshness chip"), tone: Tone::Ask },
        Held { number: 3381, job: job("mock-07", "Checkout page loads twice"), tone: Tone::Bad },
    ]
}

fn line(view: MergeLineView) -> Line {
    Line { view, held: held() }
}

fn calls(keys: &[(&str, &str, &str)]) -> Vec<StandingCall> {
    keys.iter()
        .map(|(key, kind, title)| {
            call(format!(
                r#"{{"key":"{key}","owner":"{key}","kind":"{kind}","title":"{title}","text":"{title}","since":"{}","tone":"waiting","opened":false}}"#,
                ago(120)
            ))
        })
        .collect()
}

/// A second and third repository, for the stack on All.
fn others() -> Vec<Line> {
    let simple = |root: &str, name: &str, main: &str, pulls: &str| {
        Line {
            view: view(format!(r#"{{"root":"/Users/nick/Development/{root}","name":"{name}"{main},"pulls":[{pulls}],"read_at":"{}"}}"#, ago(40))),
            held: Vec::new(),
        }
    };
    vec![
        simple(
            "sample-web",
            "sample-web",
            GREEN,
            &[
                pull(412, "Dark mode for the settings page", "web/dark-mode", "nick", r#","ci":"passed","queue":{"state":"mergeable","position":1}"#),
                pull(409, "Upgrade the router", "web/router", "nick", r#","ci":"running""#),
            ]
            .join(","),
        ),
        simple(
            "sample-api",
            "sample-api",
            &red(""),
            &pull(88, "Rate limits per token", "api/rate-limits", "nick", r#","ci":"waiting_on_main""#),
        ),
    ]
}

/// The strip at rest: one repository, main green, five in the queue, the rest open.
pub fn rest() -> Strip {
    Strip { lines: vec![line(armada(GREEN, "", 40, None))], calls: Vec::new(), on_all: false, selected_root: None, related: Vec::new(), loading: false }
}

/// Start a walk's strip at rest, and the one already open.
pub fn begin(cx: &mut App) {
    cx.set_global(Opening(rest()));
    if let Some(strip) = live(cx) {
        strip.update(cx, |strip: &mut MergeStrip, cx| {
            strip.reset(rest());
            cx.notify();
        });
    }
}

/// Another walk's start: nothing served, so the strip draws nothing.
pub fn end(cx: &mut App) {
    if cx.has_global::<Opening>() {
        cx.remove_global::<Opening>();
    }
    if let Some(strip) = live(cx) {
        strip.update(cx, |strip: &mut MergeStrip, cx| {
            strip.reset(Strip::default());
            cx.notify();
        });
    }
}

/// Move the strip to a named scene.
pub fn apply(name: &str, cx: &mut App) -> Result<(), String> {
    let strip = live(cx).ok_or("no merge strip is drawn yet")?;
    let data = match name {
        "strip-rest" => rest(),
        "strip-loading" => Strip { loading: true, ..Strip::default() },
        "strip-related" => Strip { related: vec![3384, 3366], ..rest() },
        "strip-calling" => Strip {
            calls: calls(&[("pull:3381", "pull", "Checks failed on #3381"), ("pull:3366", "pull", "Merge #3366 by hand")]),
            ..rest()
        },
        "strip-quiet" => Strip { lines: vec![line(armada(GREEN, "", 900, Some(7 * 60)))], ..rest() },
        "strip-checkout" => Strip { lines: vec![line(armada(GREEN, CHECKOUT, 40, None))], calls: calls(&[("checkout:/Users/nick/Development/armada", "checkout", "3 commits behind main")]), ..rest() },
        "strip-red" => Strip {
            lines: vec![line(armada(&red(""), "", 40, None))],
            calls: calls(&[("main:/Users/nick/Development/armada", "main", "Main is red")]),
            ..rest()
        },
        "strip-red-taken" => Strip { lines: vec![line(armada(&red(r#","taken_by":{"id":"mock-06","title":"Quiet the write-clash notices"}"#), "", 40, None))], ..rest() },
        "strip-red-held" => Strip {
            lines: vec![line(armada(
                &red(r#","checking":[{"commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","pull_request":{"number":3391}},{"commit":"4be1c07a52d9e8f6a3b2c1d0e9f8a7b6c5d4e3f2"}]"#),
                "",
                40,
                None,
            ))],
            ..rest()
        },
        "strip-all" => {
            let mut lines = vec![line(armada(GREEN, "", 40, None))];
            lines.extend(others());
            Strip { lines, on_all: true, ..rest() }
        }
        other => return Err(format!("no merge strip scene named {other}")),
    };
    strip.update(cx, |strip: &mut MergeStrip, cx| strip.set(data, cx));
    Ok(())
}
