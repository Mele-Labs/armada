//! The store's time sums as the wire carries them. `docs/practices/protocol.md`, *Where a Job's
//! time went*. A figure the store has none for stays absent here, and is never made a zero.

use ipc::{BuildFigures, BuildSource, HourFigures, JobSplit, JobTime, NamedUnits, RecordSplit, SlowJob, UsageHours, UsageTime, UsualTime};
use store::{KeptJobTime, SplitMs, TimeRead, Usual, UNIT_SCALE};

fn ms(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn split(one: SplitMs) -> RecordSplit {
    RecordSplit {
        model: ms(one.model),
        tools: ms(one.tools),
        checks: ms(one.checks),
        subagents: ms(one.subagents),
        slot: ms(one.slot),
        you: ms(one.you),
        pull_request: ms(one.pull_request),
        stopped: ms(one.stopped),
        idle: ms(one.idle),
    }
}

fn usual(workflow: String, one: Usual) -> UsualTime {
    UsualTime { workflow, jobs: one.jobs, split: split(one.split) }
}

/// A Job's kept split beside the usual of its workflow, where the workflow has a name and any kept
/// Job to be usual from.
pub fn job(session_id: String, kept: Option<(KeptJobTime, Vec<(String, Usual)>)>) -> JobTime {
    let time = kept.map(|(kept, usuals)| {
        let spent = split(SplitMs {
            model: kept.model_ms,
            tools: kept.tools_ms,
            checks: kept.checks_ms,
            subagents: kept.subagents_ms,
            slot: kept.slot_ms,
            you: kept.you_ms,
            pull_request: kept.pull_request_ms,
            stopped: kept.stopped_ms,
            idle: kept.idle_ms,
        });
        let usual = usuals.into_iter().find(|(workflow, _)| *workflow == kept.workflow).map(|(workflow, one)| usual(workflow, one));
        JobSplit { workflow: kept.workflow, began_at: kept.began_at, ended_at: kept.ended_at, spent, usual }
    });
    JobTime { session_id, time }
}

pub fn time(read: TimeRead) -> UsageTime {
    UsageTime {
        since: read.since,
        until: read.until,
        jobs: read.jobs,
        total: split(read.total),
        slowest: read
            .slowest
            .into_iter()
            .map(|one| SlowJob {
                session_id: one.session_id,
                title: one.title,
                workflow: one.workflow,
                began_at: one.began_at,
                ended_at: one.ended_at,
                split: split(one.split),
            })
            .collect(),
        usual: read.usual.into_iter().map(|(workflow, one)| usual(workflow, one)).collect(),
    }
}

fn short(commit: &str) -> String {
    commit.chars().take(7).collect()
}

fn source(said: Option<&str>) -> Option<BuildSource> {
    match said {
        Some("main") => Some(BuildSource::Main),
        Some("preview") => Some(BuildSource::Preview),
        _ => None,
    }
}

fn build(one: store::BuildFigures) -> BuildFigures {
    let new = one.build.build;
    BuildFigures {
        commit: short(&new.commit_id),
        started: new.started_at,
        until: one.until,
        source: source(new.source.as_deref()),
        main_commit: new.main_commit.as_deref().map(short),
        branches: new.branches,
        turns: one.turns,
        // The store's units are millions; a build's burn is read per turn in the units themselves.
        burn: (one.turns > 0).then(|| one.units * UNIT_SCALE / one.turns as f64),
        model: one.medians.model,
        tools: one.medians.tools,
        checks: one.medians.checks,
        pull_request: one.medians.pull_request,
        jobs: one.jobs,
        split: split(one.split),
    }
}

pub fn hours(read: store::HoursRead, builds: Vec<store::BuildFigures>) -> UsageHours {
    UsageHours {
        since: read.since,
        until: read.until,
        hours: read
            .hours
            .into_iter()
            .map(|one| HourFigures {
                hour: one.hour,
                at: one.at,
                burn: one.burn.into_iter().map(|(name, units)| NamedUnits { name, units }).collect(),
                model: one.medians.model,
                tools: one.medians.tools,
                checks: one.medians.checks,
                pull_request: one.medians.pull_request,
            })
            .collect(),
        builds: builds.into_iter().map(build).collect(),
    }
}
