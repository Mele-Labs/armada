//! Duplicate work: a Job started on what a live Job already holds, and a red main a live Job is near.
//!
//! **Fleet finds, a call decides, and the person is asked.** A start is put to `candidates::near`
//! against the live Jobs first, and only a Job found near it makes one call: with none there is no
//! call and no row. The call's ruling is a go-ahead, "Hand to <Job>" or "Keep both", answered Yes,
//! Always here, Always or No like any act, and a kept rule hands it alone.
//!
//! **A Job he started himself is never held or handed silently**: it starts, and the item is raised.
//! A Job's own start is held for him already, and Autopilot's is set aside while the go-ahead stands,
//! then started as it was asked if the answer is No or the call could not rule.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{CloseSession, Notice};

use super::arriving::{Arrival, Arrived, LiveJob, Source};
use super::candidates::{near, shared_paths};
use super::desk::started_by_chain;
use super::ruling::Way;
use super::thening::paths_in;
use super::NEEDS_YOU_WAIT;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// What a start's arrival id begins with: `start:<job>`, or `start:pick:<pick>` for Autopilot's, which has no Job yet.
pub(super) const START: &str = "start:";
pub(crate) const PICK: &str = "start:pick:";

/// What a pick set aside for the Operator says first in its reason, so Autopilot does not start it meanwhile.
pub(crate) const HELD_FOR_OVERLAP: &str = "Held for the Operator: ";

/// The most of a first line a title made of a brief keeps.
const TITLE_CHARS: usize = 60;

fn first_line(words: &str) -> String {
    words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().chars().take(TITLE_CHARS).collect()
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
    /// The live Jobs, as the Operator's own calls are shown them. None while no Operator is seated.
    async fn live_now(&self) -> Vec<LiveJob> {
        match self.operator_seat().operator() {
            Some(operator) => operator.live_jobs().await,
            None => Vec::new(),
        }
    }

    /// A Job `job` was made on `brief`, by `source`: where a live Job is near it, the Operator is
    /// handed it to judge. **Nothing is held or closed here**, and no call is made where no Job is near.
    pub(crate) async fn start_overlap_raised(&self, job: &str, brief: &str, source: Source) {
        if !self.operator_on() || brief.trim().is_empty() {
            return;
        }
        let (manifest, title, starters) = {
            let store = self.store().lock().await;
            let Some(hosting) = store.hosting(job).ok().flatten() else { return };
            let title = store.session(job).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| first_line(brief));
            (hosting.manifest_id, title, started_by_chain(&store, job))
        };
        // The Jobs that started it work beside it by design, so they are not duplicates of it.
        let live: Vec<LiveJob> = self.live_now().await.into_iter().filter(|one| !starters.iter().any(|(id, _)| *id == one.session_id)).collect();
        let arrival = Arrival {
            id: format!("{START}{job}"),
            arrived: Arrived::Start,
            source,
            session_id: job.to_string(),
            manifest_id: manifest,
            subject: title,
            words: brief.to_string(),
            paths: paths_in(brief),
            issues: Vec::new(),
            ways: vec![Way::ExistingJob, Way::Leave],
            open: Vec::new(),
        };
        if !near(&arrival, &live).is_empty() {
            self.hand_once(arrival).await;
        }
    }

    /// The arrival of Autopilot's start on an issue, which has no Job yet: `pick` is its pick's id.
    pub(crate) fn pick_arrival(manifest: &str, pick: &str, reference: &str, title: &str, words: String) -> Arrival {
        Arrival {
            id: format!("{PICK}{pick}"),
            arrived: Arrived::Start,
            source: Source::Fleet,
            session_id: String::new(),
            manifest_id: manifest.to_string(),
            subject: title.to_string(),
            paths: paths_in(&words),
            words,
            issues: vec![reference.to_string()],
            ways: vec![Way::ExistingJob, Way::Leave],
            open: Vec::new(),
        }
    }

    /// Whether a live Job is near `arrival`, an issue Autopilot would start a Job on.
    pub(crate) async fn start_has_near(&self, arrival: &Arrival) -> bool {
        self.operator_on() && !near(arrival, &self.live_now().await).is_empty()
    }

    /// Autopilot's start `arrival`, set aside, put to the Operator. Answers how it ended: a ruling that
    /// hands it over stands for a go-ahead, and anything else leaves nothing holding it, so `true` is
    /// that it was handed on to be asked.
    pub(crate) async fn start_ruled(&self, arrival: Arrival) -> bool {
        let Some(operator) = self.operator_seat().operator() else { return false };
        matches!(operator.ruled(arrival, NEEDS_YOU_WAIT).await, super::Ruled::Ruled(ruling) if ruling.way == Way::ExistingJob)
    }

    /// The Operator's own act on a go-ahead to hand a start to live Job `into`: its words go to that
    /// Job, the Job that started it is told where they went, and the duplicate is closed. For
    /// Autopilot's start, which has no Job yet, the pick is dismissed in place of a Job closed.
    pub(super) async fn operator_handed_start(self: &Arc<Self>, arrival: &Arrival, into: &str) -> Result<(), String> {
        let text = format!(
            "The Operator handed you work that another Job started, as yours is already on it:\n\n{}\n\nDo it as part of your work, or say why it is not yours.",
            arrival.words
        );
        if let Some(pick) = arrival.id.strip_prefix(PICK) {
            if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
                return Err("the Job did not take it".to_string());
            }
            if !self.pick_handed_over(pick).await {
                self.operator_warned("an issue handed to a live Job is still set aside", pick);
            }
            return Ok(());
        }
        let duplicate = arrival.session_id.as_str();
        let starter = self.starter_of(duplicate).await;
        if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
            return Err("the Job did not take it".to_string());
        }
        if let Some(starter) = starter {
            let theirs = self.title_of(into).await;
            let told = format!("Your Job \"{}\" was not started: Job \"{theirs}\" is already on that work, and the Operator handed your brief to it.", arrival.subject);
            let _ = self.told_by_fleet(&starter, &told, Notice::Operator).await;
        }
        // The duplicate is closed once its words are with the Job that holds the work.
        if let Err(why) = self.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(duplicate) }).await {
            self.operator_warned("a Job handed to a live Job was not closed", format!("{why:?}"));
        }
        Ok(())
    }

    /// A red main handed to live Job `job`, a row of the Log signed by the Operator that names the red
    /// and the Job, and says which rule did it where one did.
    pub(super) async fn red_handed_logged(&self, manifest: &str, commit: &str, job: &str, rule: Option<&str>) {
        let theirs = self.title_of(job).await;
        let red = format!("Main is red at {}", commit.get(..7).unwrap_or(commit));
        let mut because = format!("Handed the red at {} to \"{theirs}\", whose work is near it.", commit.get(..7).unwrap_or(commit));
        if let Some(rule) = rule {
            because = format!("By a rule, {rule}: {because}");
        }
        let at = self.now();
        let mut row = super::Logged::about(job, manifest, &red, super::Outcome::Ruled(Way::ExistingJob), core_model::Actor::Operator, at);
        row.arrival = format!("red_main:{manifest}:{commit}");
        row.arrived = Arrived::RedMain;
        row.session_id = String::new();
        row.other = Some(job.to_string());
        row.because = because.clone();
        self.operator_logged(row).await;
        self.operator_board_changed();
        self.operator_journaled(&because).await;
    }

    /// The Job that started `job`, where one did.
    async fn starter_of(&self, job: &str) -> Option<String> {
        let store = self.store().lock().await;
        started_by_chain(&store, job).into_iter().next().map(|(id, _)| id)
    }

    /// The live Job near `main`'s red, as `(id, title)`: the one that took the last red, else one whose
    /// paths meet the files the merge before this red changed. **A Job in another repository is never near.**
    pub(crate) async fn red_near(&self, served: &Served, main: &store::MainCi) -> Option<(String, String)> {
        let manifest = served.manifest().id().as_str().to_string();
        let live: Vec<LiveJob> = self.live_now().await.into_iter().filter(|one| one.manifest_id == manifest).collect();
        if live.is_empty() {
            return None;
        }
        let took = match &main.red_at {
            Some(at) => self.store().lock().await.last_main_fixer(&main.repository, at).ok().flatten(),
            None => None,
        };
        if let Some(job) = live.iter().find(|one| took.as_deref() == Some(one.session_id.as_str())) {
            return Some((job.session_id.clone(), job.title.clone()));
        }
        let number = main.merge.as_ref()?.number;
        let files = self.forge_asked(&main.repository, move |vcs: &V, root: &str| vcs.pull_request_diff(root, &number.to_string())).await?.files;
        live.into_iter().find(|one| !shared_paths(&files, &one.paths).is_empty()).map(|one| (one.session_id, one.title))
    }
}
