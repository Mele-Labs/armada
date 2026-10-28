//! Where the build record meets the store: what is heard of a Session's work is kept, and what a row
//! needs is read back, under the lock the caller already holds. `docs/concepts/session.md`,
//! *The build record*. The rules are `build_record`'s.

use std::collections::HashMap;

use ipc::{BuildStopId, SessionBuild};
use store::{Holder, KeptAttachment, KeptBuild, KeptSession, KeptTask, RoadAsk, RoadTimes, Store, WriteError};

use crate::build_record::{
    current_pull, derived, landed_since, merged, reached, seeded, taken, usual_road, Observed, Read, FORECAST_WINDOW,
    NEVER,
};

/// Keep one thing heard of the Session's work. Answers whether the row shows it.
///
/// **A build that landed ends with the work after it.** An edit, a plan or a task list heard once its
/// pull request has merged starts another build, with no list and no road, so a Session that goes on
/// to a second piece of work is not shown as the first one landed.
///
/// **A row with no first edit is filled from the timeline first**, in one bounded read of that
/// Session's own entries: what Fleet heard before this run started, or before the row was made, is
/// there and not in the row.
pub(crate) fn build_heard(store: &mut Store, id: &str, heard: &Observed, now: &str) -> Result<bool, WriteError> {
    let mut build = store.session_build(id)?;
    let mut tasks = store.session_build_tasks(id)?;
    if heard.begins_work() {
        if let Some(begun) = build.as_ref().map(|one| one.begun_at.as_str()) {
            if landed_since(&store.attachments_of(&Holder::session(id))?, begun) {
                store.forget_session_build(id)?;
                build = None;
                tasks.clear();
            }
        }
    }
    if build.as_ref().is_none_or(|one| one.first_edit_at.is_none()) {
        let from = work_from(store, id)?;
        build = seeded(build, &store.session_work(id, &from)?);
    }
    let took = taken(build, tasks, heard, now);
    store.keep_session_build(id, &took.build)?;
    if matches!(heard, Observed::Tasks(_)) {
        store.keep_session_build_tasks(id, &took.tasks)?;
    }
    // The same list also moves the tasks of the Session's plan.
    let planned = match heard {
        Observed::Tasks(list) => crate::plan_keeping::tasks_heard(store, id, list, now)?,
        _ => false,
    };
    Ok(took.visible || planned)
}

/// Where a Session's work is read from: when it started, or when the last pull request it held merged,
/// whichever is later, since work before a landing belongs to the build that landed.
fn work_from(store: &Store, id: &str) -> Result<String, WriteError> {
    let started = store.session(id)?.map(|one| one.started_at).unwrap_or_default();
    let landed = store
        .attachments_of(&Holder::session(id))?
        .into_iter()
        .filter(|one| one.kind == "pr" && merged(one))
        .map(|one| one.changed_at)
        .max();
    Ok(landed.map_or(started.clone(), |landed| landed.max(started)))
}

/// What a row's build is derived from, taken from the store.
#[derive(Default)]
pub(crate) struct BuildReads {
    kept: Option<(KeptBuild, Vec<KeptTask>)>,
    road: RoadTimes,
    pull_since: Option<String>,
    merged_fallback: Option<String>,
    forecast: Vec<BuildStopId>,
}

/// Each repository's forecast, read once for every row a caller draws from one hold of the lock.
/// It is a reading of the repository's recent landings and says nothing of the Session, yet costs
/// twenty road reads, so a list of every Session paid it once per row.
#[derive(Default)]
pub(crate) struct Forecasts(HashMap<String, Vec<BuildStopId>>);

impl Forecasts {
    fn of(&mut self, store: &Store, manifest: &str) -> Result<Vec<BuildStopId>, WriteError> {
        if let Some(read) = self.0.get(manifest) {
            return Ok(read.clone());
        }
        let read = forecast_of(store, manifest)?;
        self.0.insert(manifest.to_string(), read.clone());
        Ok(read)
    }
}

/// Read what the build is derived from. A Session with no kept build reads nothing more. A caller
/// reading several rows under one hold of the lock shares `forecasts` between them.
pub(crate) fn build_read(
    store: &Store,
    session: &KeptSession,
    held: &[KeptAttachment],
    forecasts: &mut Forecasts,
) -> Result<BuildReads, WriteError> {
    let Some(build) = store.session_build(&session.id)? else {
        return Ok(BuildReads::default());
    };
    let tasks = store.session_build_tasks(&session.id)?;
    let manifest = session.manifest_id.as_deref().unwrap_or_default();
    let pull = current_pull(held, &build.begun_at);
    let road = store.session_road(&RoadAsk {
        session: &session.id,
        manifest,
        pull: pull.map(|one| one.target.as_str()),
        from: &build.begun_at,
        walk_from: build.first_edit_at.as_deref().unwrap_or(NEVER),
        until: None,
    })?;
    let landed = road.merged.is_some() || pull.is_some_and(merged);
    // A road already closed needs no forecast.
    let forecast = if landed || manifest.is_empty() { Vec::new() } else { forecasts.of(store, manifest)? };
    Ok(BuildReads {
        road,
        pull_since: pull.map(|one| one.since.clone()),
        merged_fallback: pull.filter(|one| merged(one)).map(|one| one.changed_at.clone()),
        forecast,
        kept: Some((build, tasks)),
    })
}

/// The stops a repository's last [`FORECAST_WINDOW`] landed pull requests (those a Session held)
/// usually reached. Each is read over the stretch from the Session's previous landing to its own.
pub(crate) fn forecast_of(store: &Store, manifest: &str) -> Result<Vec<BuildStopId>, WriteError> {
    let mut history = Vec::new();
    for landed in store.landed_pulls(manifest, FORECAST_WINDOW)? {
        let from = store.previous_landing(&landed.session, manifest, &landed.merged_at)?.unwrap_or_default();
        let road = store.session_road(&RoadAsk {
            session: &landed.session,
            manifest,
            pull: Some(&landed.number),
            from: &from,
            walk_from: &from,
            until: Some(&landed.merged_at),
        })?;
        history.push(reached(&road, Some(&landed.merged_at)).into_iter().map(|(id, _)| id).collect());
    }
    Ok(usual_road(&history))
}

impl BuildReads {
    /// The record, absent where nothing was observed of the Session's work.
    pub(crate) fn record(&self, stuck: bool, workers: Vec<String>) -> Option<SessionBuild> {
        let (build, tasks) = self.kept.as_ref()?;
        Some(derived(&Read {
            build,
            tasks,
            road: &self.road,
            pull_since: self.pull_since.as_deref(),
            merged_fallback: self.merged_fallback.as_deref(),
            forecast: &self.forecast,
            workers,
            stuck,
        }))
    }
}
