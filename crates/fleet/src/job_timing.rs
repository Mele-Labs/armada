//! Where each Job's time went, read from the store and kept when the Job ends. `job_time.rs` is
//! the fold; this gathers its input and keeps its answer. `docs/concepts/session.md`.
//!
//! **One pass for the end of a Job and for the backfill.** Every [`EVERY`] a Job that has ended and
//! has no split, or whose split is of an earlier end (it was reopened and ended again), is split,
//! one Job at a time with a [`PAUSE`] between, newest first. The backfill is the same pass finding
//! the Jobs that ended before it existed, so there is no second path to drift.
//!
//! **The transcripts are read first**: the Job's tail, which the loop over live Jobs stopped
//! reading when it ended; and, where its turns lack the stamps the split reads, the whole file
//! again from the top ([`crate::turn_ingest`]'s `from_start`), which is paced like the rest. A
//! transcript that is gone leaves the Job's turns as they are, and the row says so.
//!
//! **The store's lock is held for the reads of one Job, never across a transcript or a pause.**

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::turn_lines;
use core_model::Timestamp;
use ipc::SessionRow;
use store::{AttachmentState, Holder, KeptBuildSplit, KeptJobTime, KeptPullRequestWait, KeptTurnTimed};

use crate::daemon::Fleet;
use crate::job_time::{self, Span, Times, Turn};

/// How long after Fleet starts the first Job is split, after the week's turn backfill has begun.
pub(crate) const AFTER: Duration = Duration::from_secs(45);
/// How often the pass looks for a Job that has ended.
pub(crate) const EVERY: Duration = Duration::from_secs(30);
/// How long it rests between Jobs: a Job read from the top is up to a few transcripts.
const PAUSE: Duration = Duration::from_millis(250);
/// How often Fleet says it is up, and how long unseen counts as away.
pub(crate) const BEAT: Duration = Duration::from_secs(15);
pub(crate) const OUTAGE_SECONDS: i64 = 90;

fn millis(stamp: &str) -> Option<i64> {
    Timestamp::from_rfc3339(stamp).epoch_millis()
}

/// The calls that return when the owner answers: their span is his time, not a tool's.
const OWNER_WAITS: &[&str] = &["AskUserQuestion"];

/// A kept turn as the fold reads it. `None` where its stamp does not read.
pub(crate) fn turn_of(kept: &KeptTurnTimed) -> Option<Turn> {
    Some(Turn {
        at: millis(&kept.at)?,
        model_began: kept.model_began.as_deref().and_then(millis),
        tools_began: kept.tools_began.as_deref().and_then(millis),
        tools_ended: kept.tools_ended.as_deref().and_then(millis),
        called_tools: !kept.tools.is_empty(),
        asked_owner: kept.tools.iter().any(|name| OWNER_WAITS.contains(&name.as_str())),
    })
}

fn spans(pairs: impl IntoIterator<Item = (Option<i64>, Option<i64>)>) -> Vec<Span> {
    pairs.into_iter().filter_map(|(from, to)| Some(Span { from: from?, to: to? })).collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Split every Job that has ended and has no split, one after another with a pause, and then
    /// return. How many.
    pub(crate) async fn jobs_timed(&self) -> usize {
        let mut done = 0;
        loop {
            let Ok(Some(id)) = self.store().lock().await.job_awaiting_time(job_time::FOLD) else {
                return done;
            };
            if !self.job_time_kept(&id).await {
                return done;
            }
            done += 1;
            tokio::time::sleep(PAUSE).await;
        }
    }

    /// Read the Job's transcripts, split its time and keep it. Whether it was kept: a store that
    /// will not answer leaves the Job for the next pass.
    pub(crate) async fn job_time_kept(&self, id: &str) -> bool {
        let from_start = self.store().lock().await.turns_lack_times(id).unwrap_or(false);
        let home = self.host().home.clone();
        let session = id.to_string();
        let files = tokio::task::spawn_blocking(move || turn_lines::transcripts_of(&home, &session)).await.unwrap_or_default();
        for file in files {
            self.transcript_taken_in(&file, from_start).await;
            tokio::time::sleep(PAUSE).await;
        }
        let Some(rows) = self.rows_of(id).await.ok() else {
            return false;
        };
        let Some(time) = self.job_time_read(id, &rows).await else {
            return false;
        };
        self.store().lock().await.keep_job_time(&time).is_ok()
    }

    /// The Job's times from the store, folded. `None` where the store would not answer.
    async fn job_time_read(&self, id: &str, rows: &[SessionRow]) -> Option<KeptJobTime> {
        let kept_at = self.now().as_str().to_string();
        let store = self.store().lock().await;
        let session = store.session(id).ok()??;
        let hosting = store.hosting(id).ok()?;
        let began_at = session.started_at.clone();
        let ended_at = session.ended_at.clone().unwrap_or_else(|| session.last_seen_at.clone());
        let turns = store.turns_timed(id).ok()?;
        let checks = store.check_spans_of(id).ok()?;
        let slot_waits = store.slot_waits_of(id).ok()?;
        let asked = store.timeline_stamps(id, &["session.waiting", "session.asked"]).ok()?;
        let answered = store.timeline_stamps(id, &["session.answered", "session.resolved"]).ok()?;
        let versions = store.attachment_versions(&Holder::session(id)).ok()?;
        let restarts = store.session_restarts(id).ok()?;
        let stops = store.session_stops_of(id).ok()?;
        let outages = store.fleet_outages_between(&began_at, &ended_at).ok()?;
        let plan = store.session_plan(id).ok()?;
        let oldest = store.timeline_oldest().ok()?;
        let builds = store.builds().ok()?;
        drop(store);

        let (began, ended) = (millis(&began_at), millis(&ended_at));
        let (Some(began), Some(ended)) = (began, ended) else {
            return Some(self.job_time_of(id, &session.manifest_id, &hosting, &began_at, &ended_at, job_time::Folded::default(), false, kept_at, Vec::new(), Vec::new()));
        };
        let at = |stamp: &String| millis(stamp);

        // The owner: what opens a wait, and what closes the first after it. A walk and a plan close
        // themselves; the owner's next message closes whatever is left.
        let mut opens: Vec<i64> = asked.iter().filter_map(at).collect();
        let mut closes: Vec<i64> = answered.iter().filter_map(at).collect();
        for row in rows {
            match row {
                SessionRow::Message { at: when, from: ipc::SessionVoice::You, .. } => closes.extend(millis(when.as_str())),
                SessionRow::Window { at: when, ended_at: over, .. } => {
                    opens.extend(millis(when.as_str()));
                    closes.extend(over.as_ref().and_then(|over| millis(over.as_str())));
                }
                SessionRow::WindowAsk { at: when, .. } | SessionRow::Draft { at: when, .. } => opens.extend(millis(when.as_str())),
                _ => {}
            }
        }
        if let Some(plan) = &plan {
            opens.extend(millis(&plan.recorded_at));
            closes.extend(plan.approved_at.iter().chain(&plan.set_aside_at).filter_map(at));
        }
        let mut owner = job_time::spans_between(&opens, &closes, ended);
        if hosting.as_ref().is_some_and(|hosting| hosting.start_held) {
            owner.push(Span { from: began, to: ended });
        }

        // A subagent and a pull request each stand from the first standing version to the first
        // that does not, one stint a target.
        let stood = |kind: &str| {
            let mut kept: Vec<(String, Span)> = Vec::new();
            let mut targets: Vec<(&str, &str)> = versions
                .iter()
                .filter(|one| one.kind == kind)
                .map(|one| (one.manifest_id.as_str(), one.target.as_str()))
                .collect();
            targets.sort_unstable();
            targets.dedup();
            for (manifest, target) in targets {
                let one: Vec<(i64, bool)> = versions
                    .iter()
                    .filter(|one| one.kind == kind && one.manifest_id == manifest && one.target == target)
                    .filter_map(|one| Some((millis(&one.at)?, one.state == AttachmentState::Standing)))
                    .collect();
                kept.extend(job_time::stints(&one, ended).into_iter().map(|span| (target.to_string(), span)));
            }
            kept
        };
        let pull_requests = stood(crate::pull_requesting::PR);

        // A death is a stop, and the process coming back ends it; a process Fleet let go is neither.
        let stop_opens: Vec<i64> = stops.iter().filter_map(at).chain(restarts.iter().filter(|one| one.kind == "ended").filter_map(|one| millis(&one.at))).collect();
        let back: Vec<i64> = restarts.iter().filter(|one| one.kind != "ended").filter_map(|one| millis(&one.at)).collect();
        let mut stopped = job_time::spans_between(&stop_opens, &back, ended);
        stopped.extend(spans(outages.iter().map(|(from, to)| (millis(from), millis(to)))));

        let times = Times {
            began,
            ended,
            turns: turns.iter().filter_map(turn_of).collect(),
            // A run that never ended has no end to count.
            checks: spans(checks.iter().map(|run| {
                let from = millis(&run.started_at);
                let to = run.ended_at.as_deref().and_then(millis).or_else(|| Some(from? + run.took_ms? as i64));
                (from, to)
            })),
            subagents: stood("subagent").into_iter().map(|(_, span)| span).collect(),
            slot: spans(slot_waits.iter().map(|wait| (millis(&wait.began_at), Some(wait.ended_at.as_deref().and_then(millis).unwrap_or(ended))))),
            owner,
            pull_request: pull_requests.iter().map(|(_, span)| *span).collect(),
            stopped,
            restarts: restarts.iter().filter_map(|one| millis(&one.at)).chain(stops.iter().filter_map(at)).collect(),
            builds: builds.iter().filter_map(|one| millis(&one.build.started_at)).collect(),
        };
        let folded = job_time::fold(&times);
        let started: Vec<(i64, i64)> = builds.iter().filter_map(|one| Some((millis(&one.build.started_at)?, one.id))).collect();
        let by_build = job_time::by_build(&folded, began, ended, &started)
            .into_iter()
            .map(|(build_id, split)| KeptBuildSplit {
                build_id,
                model_ms: split.model,
                tools_ms: split.tools,
                checks_ms: split.checks,
                subagents_ms: split.subagents,
                slot_ms: split.slot,
                you_ms: split.you,
                pull_request_ms: split.pull_request,
                stopped_ms: split.stopped,
                idle_ms: split.idle,
            })
            .collect();
        // Another stint's pull request time is not this one's: each is folded alone.
        let waits = pull_requests
            .iter()
            .map(|(target, span)| {
                let alone = Times { pull_request: vec![*span], builds: Vec::new(), ..times.clone() };
                KeptPullRequestWait {
                    target: target.clone(),
                    began_at: crate::clock::rfc3339_utc(span.from),
                    ended_at: crate::clock::rfc3339_utc(span.to),
                    wait_ms: job_time::fold(&alone).split.pull_request,
                }
            })
            .collect();
        let cut = oldest.as_deref().is_some_and(|oldest| began_at.as_str() < oldest);
        Some(self.job_time_of(id, &session.manifest_id, &hosting, &began_at, &ended_at, folded, cut, kept_at, by_build, waits))
    }

    #[allow(clippy::too_many_arguments)]
    fn job_time_of(
        &self,
        id: &str,
        repository: &Option<String>,
        hosting: &Option<store::KeptHosting>,
        began_at: &str,
        ended_at: &str,
        folded: job_time::Folded,
        timeline_cut: bool,
        kept_at: String,
        builds: Vec<KeptBuildSplit>,
        waits: Vec<KeptPullRequestWait>,
    ) -> KeptJobTime {
        let split = folded.split;
        KeptJobTime {
            session_id: id.to_string(),
            repository: repository.clone().unwrap_or_default(),
            workflow: hosting.as_ref().and_then(|hosting| hosting.workflow.clone()).unwrap_or_default(),
            began_at: began_at.to_string(),
            ended_at: ended_at.to_string(),
            model_ms: split.model,
            tools_ms: split.tools,
            checks_ms: split.checks,
            subagents_ms: split.subagents,
            slot_ms: split.slot,
            you_ms: split.you,
            pull_request_ms: split.pull_request,
            stopped_ms: split.stopped,
            idle_ms: split.idle,
            tool_times_missing: folded.tool_times_missing,
            model_starts_guessed: folded.model_starts_guessed,
            timeline_cut,
            kept_at,
            fold: job_time::FOLD,
            builds,
            waits,
        }
    }
}
