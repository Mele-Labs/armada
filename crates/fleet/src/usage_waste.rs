//! The store's waste as the wire carries it, with the words Fleet gives each kind so a page and an agent
//! read the same ones. `docs/practices/protocol.md`, *The waste in the burn*.

use store::{UsageWasteRead, WasteKind};

/// The window `get_usage_waste` reads where a caller names none.
pub(crate) const DEFAULT_WINDOW: &str = "week";

/// What a kind is called, and the one act that cuts it. A new kind is a variant in the store and a line here.
fn words(kind: WasteKind) -> (&'static str, &'static str) {
    match kind {
        WasteKind::SubagentContext => (
            "Subagents ran past 300k context",
            "Brief a subagent on one file or one question and have it return a summary, so its context stays under 300k.",
        ),
        WasteKind::MainContext => (
            "Main thread ran past 300k context",
            "Send handoff_brief before the thread reaches 300k, so the next one starts from a brief.",
        ),
        WasteKind::ColdCache => (
            "Cold cache after a pause",
            "Answer a waiting Job inside 5 minutes, or compact it before a long wait, because a write after a longer gap is paid in full.",
        ),
        WasteKind::FreshStarts => (
            "Fresh starts and compactions",
            "Trim what every start loads, such as instructions and tool lists, because each compaction writes them again.",
        ),
        WasteKind::RepeatReads => (
            "Same file read again",
            "Read a file once and work from that read, because a second read adds the whole file to the context again.",
        ),
        WasteKind::BigResults => (
            "Tool results over 20 KB",
            "Read by line range, narrow a search and trim a command's output, because a result stays in the context for every request after it.",
        ),
    }
}

pub(crate) fn wire(read: UsageWasteRead) -> ipc::UsageWaste {
    let kinds: Vec<ipc::WasteKind> = read
        .kinds
        .into_iter()
        .map(|one| {
            let (title, fix) = words(one.kind);
            ipc::WasteKind {
                kind: one.kind.id().to_string(),
                title: title.to_string(),
                fix: fix.to_string(),
                units: one.units,
                occurrences: one.occurrences,
                jobs: one
                    .jobs
                    .into_iter()
                    .map(|job| ipc::WasteJob {
                        session_id: job.session_id,
                        units: job.units,
                        occurrences: job.occurrences,
                        peak_context: job.peak_context,
                        causes: job.causes.into_iter().map(|(name, units)| ipc::WasteCause { name, units }).collect(),
                    })
                    .collect(),
            }
        })
        .collect();
    ipc::UsageWaste {
        waste_units: kinds.iter().map(|one| one.units).sum(),
        until: read.until.clone(),
        window: ipc::WasteWindow { id: read.window, since: read.since, until: read.until },
        total_units: read.total_units,
        kinds,
    }
}
