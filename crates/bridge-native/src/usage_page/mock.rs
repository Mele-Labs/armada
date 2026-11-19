//! The Usage page on mock data: a week of burn with six kinds of waste, a day and a fortnight scaled from it, and the
//! titles of the Jobs it names. Replaced by Fleet's reading when the route lands.

use std::collections::HashMap;

use ipc::{UsageWaste, WasteCause, WasteJob, WasteKind, WasteWindow};

use super::model::Span;

/// The scenarios that open on this page.
pub const WEEK: &str = "usage-week";
pub const LOADING: &str = "usage-loading";
pub const EMPTY: &str = "usage-empty";
pub const FAILED: &str = "usage-failed";
/// What Fleet's refusal reads as on the failed scenario.
pub const REFUSED: &str = "Fleet answered 500";

/// Whether a mock scenario of this name opens on the Usage page.
pub fn is_usage(scenario: &str) -> bool {
    matches!(scenario, WEEK | LOADING | EMPTY | FAILED)
}

const THOUSAND: u64 = 1_000;

/// The Jobs the reading names, by id: the last has no title, so its short id stands.
const JOBS: [(&str, Option<&str>); 8] = [
    ("usage-job-1", Some("Native jobs screens")),
    ("usage-job-2", Some("Docs audit")),
    ("usage-job-3", Some("Fleet speed work")),
    ("usage-job-4", Some("Pocket sign-in on the phone")),
    ("usage-job-5", Some("Settings overlay")),
    ("usage-job-6", Some("Operator cards")),
    ("usage-job-7", Some("Review sheet keys")),
    ("5d1e9c42b7a8", None),
];

pub fn titles() -> HashMap<String, String> {
    JOBS.iter().filter_map(|(id, title)| title.map(|title| (id.to_string(), title.to_string()))).collect()
}

fn id_of(n: usize) -> &'static str {
    JOBS[n - 1].0
}

/// The repositories' roots the mock paths sit under.
pub fn roots() -> Vec<String> {
    vec!["/Users/nickmele/Development/armada".into()]
}

const ROOT: &str = "/Users/nickmele/Development/armada";

/// What a Job's share of a kind came of, for the two kinds that name it: files read again, and the tools whose results were large.
fn causes(kind: &str, job: usize, units: f64) -> Vec<WasteCause> {
    let named: &[(&str, f64)] = match (kind, job) {
        ("repeat_reads", 3) => &[("/.armada/slots/slot-3/crates/bridge-native/src/job_view/story/model.rs", 0.7), ("/crates/ipc/src/lib.rs", 0.3)],
        ("repeat_reads", 1) => &[("/docs/practices/native-bridge.md", 0.6), ("/.armada/slots/slot-1/CLAUDE.md", 0.4)],
        ("repeat_reads", 5) => &[("/.armada/slots/slot-2/crates/ipc/src/lib.rs", 1.0)],
        ("big_results", 4) => &[("mcp__armada-fleet__get_job_lifecycles", 0.8), ("Bash", 0.2)],
        ("big_results", 1) => &[("Bash", 0.6), ("Read", 0.4)],
        ("big_results", 2) => &[("Read", 1.0)],
        _ => &[],
    };
    named
        .iter()
        .map(|(name, share)| WasteCause { name: if name.starts_with('/') { format!("{ROOT}{name}") } else { (*name).to_string() }, units: units * share })
        .collect()
}

/// One kind with its Jobs: `(job, units, peak context in thousands)`.
fn kind(name: &str, title: &str, fix: &str, occurrences: u64, jobs: &[(usize, u64, Option<u64>)], scale: f64) -> WasteKind {
    let jobs: Vec<WasteJob> = jobs
        .iter()
        .map(|(job, units, peak)| WasteJob {
            session_id: id_of(*job).to_string(),
            units: *units as f64 * scale,
            occurrences: ((occurrences as f64 / jobs.len() as f64) * scale).ceil() as u32,
            peak_context: peak.map(|peak| peak * THOUSAND),
            causes: causes(name, *job, *units as f64 * scale),
        })
        .collect();
    WasteKind { kind: name.into(), title: title.into(), fix: fix.into(), units: jobs.iter().map(|job| job.units).sum(), occurrences: (occurrences as f64 * scale).round() as u32, jobs }
}

/// The reading for a span. The week is the example the page was approved on; a day and a fortnight are its figures scaled.
pub fn waste(span: Span) -> UsageWaste {
    let scale = match span {
        Span::Day => 0.2,
        Span::Week => 1.,
        Span::Fortnight => 1.9,
    };
    let kinds = vec![
        kind(
            "subagent_context",
            "Subagents ran past 300k context",
            "Brief a subagent with the files it needs and send it a narrow task. Stop it before its context passes 300k.",
            41,
            &[(1, 21, Some(870)), (2, 9, Some(610)), (3, 6, Some(420)), (4, 5, Some(540)), (5, 5, Some(510)), (6, 4, Some(380)), (7, 4, Some(330)), (8, 4, Some(300))],
            scale,
        ),
        kind(
            "main_context",
            "Main thread ran past 300k context",
            "Hand the Job on at a step's end, before the thread passes 300k.",
            17,
            &[(1, 12, Some(640)), (3, 8, Some(590)), (4, 5, Some(510)), (2, 4, Some(440)), (8, 2, Some(400))],
            scale,
        ),
        kind(
            "cold_cache",
            "Cold cache after a pause",
            "Answer a waiting Job inside five minutes, or let it rest until the answer is ready.",
            41,
            &[(2, 7, Some(310)), (5, 5, Some(280)), (6, 4, Some(240)), (1, 2, Some(190))],
            scale,
        ),
        kind(
            "repeat_reads",
            "Same file read again",
            "Name the file and the lines in the brief so the agent reads them once.",
            63,
            &[(3, 5, Some(260)), (1, 4, Some(330)), (5, 3, Some(220))],
            scale,
        ),
        kind(
            "big_results",
            "Tool results over 20 KB",
            "Narrow the command: a path, a count or a filter, so the result comes back small.",
            22,
            &[(4, 4, Some(300)), (1, 3, Some(410)), (2, 1, Some(180))],
            scale,
        ),
        kind(
            "fresh_starts",
            "Fresh starts and compactions",
            "Keep a Job on one thread through its step instead of starting it again.",
            9,
            &[(6, 2, None), (4, 1, Some(120)), (8, 1, None)],
            scale,
        ),
    ];
    let waste_units: f64 = kinds.iter().map(|kind| kind.units).sum();
    UsageWaste {
        until: "2026-11-18T09:00:00Z".into(),
        window: WasteWindow {
            id: span.id().into(),
            since: match span {
                Span::Day => "2026-11-17T09:00:00Z",
                Span::Week => "2026-11-11T09:00:00Z",
                Span::Fortnight => "2026-11-04T09:00:00Z",
            }
            .into(),
            until: "2026-11-18T09:00:00Z".into(),
        },
        total_units: 412. * scale,
        waste_units,
        kinds,
    }
}

/// A window that holds no burn.
pub fn empty(span: Span) -> UsageWaste {
    UsageWaste {
        until: "2026-11-18T09:00:00Z".into(),
        window: WasteWindow { id: span.id().into(), since: "2026-11-11T09:00:00Z".into(), until: "2026-11-18T09:00:00Z".into() },
        total_units: 0.,
        waste_units: 0.,
        kinds: Vec::new(),
    }
}
