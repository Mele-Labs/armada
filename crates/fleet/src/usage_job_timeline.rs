//! The store's reading of one Job's burn as the wire carries it, with the words Fleet gives what a pause waited on
//! so a page and an agent read the same ones. `docs/practices/protocol.md`, *A Job's burn over time*.

use store::{JobTimelineRead, WaitedOn};

/// What a pause is called. A new reason is a variant in the store and a line here.
fn label(waited_on: WaitedOn) -> &'static str {
    match waited_on {
        WaitedOn::You => "Waited on you",
        WaitedOn::Command => "Command ran",
        WaitedOn::Subagent => "Subagent ran",
        WaitedOn::Other => "Paused",
    }
}

pub(crate) fn wire(session_id: String, read: JobTimelineRead) -> ipc::JobTimeline {
    ipc::JobTimeline {
        session_id,
        started: read.started,
        ended: read.ended,
        bucket_seconds: read.bucket_seconds,
        total_units: read.total_units,
        week_units: read.week_units,
        buckets: read
            .buckets
            .into_iter()
            .map(|one| ipc::TimelineBucket { at: one.at, main_units: one.main_units, subagent_units: one.subagent_units })
            .collect(),
        context: read.context.into_iter().map(|(at, tokens)| ipc::TimelineContext { at, tokens }).collect(),
        compactions: read.compactions,
        pauses: read
            .pauses
            .into_iter()
            .map(|one| ipc::TimelinePause {
                from: one.from,
                to: one.to,
                waited_on: one.waited_on.id().to_string(),
                label: label(one.waited_on).to_string(),
                cold: one.cold,
            })
            .collect(),
        peak_context: read.peak_context,
        cold_wakes: read.cold_wakes,
        subagent_share: read.subagent_share,
    }
}
