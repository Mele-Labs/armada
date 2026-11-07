//! Fleet telling the Operator, by itself, that a hosted Job ended or one of its pull requests
//! merged. **Every hosted Job, whoever started it**, on its own tap beside the one that tells a
//! parent (`linked_following`), which it leaves as it is.
//!
//! **Each is handed once.** The store keeps `operator_ended` and `operator_merged`, under names of
//! their own so neither shadows what a parent was told. **Nothing is replayed**: when the tap starts,
//! every end and merge already there is marked seen without a word to the Operator, and one that
//! happens while the Operator is off is marked as it happens, so turning it on hands nothing. A Job
//! handed on for context is no end: its work goes on in its successor.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{Event, ManifestId, Notice, SessionRecord, SessionVoice, StartSession};
use store::{AttachmentState, Holder, KeptAttachment, Store};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::candidates::open_near;
use super::desk::{held_of, started_by_chain};
use super::ending::{Ended, Merged, Pulled, Tied, Unblocks};
use super::{held, Arrival, Ruling};
use crate::daemon::Fleet;
use crate::session_host::replacing::REPLACED_BY;

const ENDED: &str = "operator_ended";
const MERGED: &str = "operator_merged";
const OPERATOR: &str = "Operator";
/// The most of anything one end lists: Later items, Needs you items and Jobs.
const MOST_LISTED: usize = 6;
/// The most of a Needs you item's words an end carries.
const MOST_WORDS: usize = 160;

/// Start telling the Operator of ends and merges. **The end and merge already there are marked seen
/// first**, so only those after this are handed. Production starts it beside the parents' tap, and a
/// test starts it itself.
pub async fn keep_the_ends_told<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fleet.ends_marked_seen().await;
    let (tell, mut heard) = mpsc::unbounded_channel::<SessionRecord>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.state == ipc::SessionState::Ended || record.attachments.iter().any(|one| one.kind == "pr" && crate::after_following::merged(one)) {
                let _ = tell.send(record.clone());
            }
        }
    });
    tokio::spawn(async move {
        while let Some(record) = heard.recv().await {
            fleet.end_heard(record).await;
        }
    })
}

/// Whether `record` is a Job that ended with its work its own: not one handed on for context.
fn ended_for_good(record: &SessionRecord) -> bool {
    record.state == ipc::SessionState::Ended && !record.attachments.iter().any(|one| one.kind == REPLACED_BY)
}

/// The state of a pull request's row, in the words the wire uses.
fn pull_state(one: &KeptAttachment) -> String {
    match one.detail.get("state") {
        Some(state) => state.clone(),
        None if one.state == AttachmentState::Spent => "merged".to_string(),
        None => "open".to_string(),
    }
}

fn cut(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", text[..at].trim_end()),
        None => text.to_string(),
    }
}

/// What a wait on a Job is for, in a few words.
fn wait_said(condition: &str) -> &'static str {
    match condition {
        "ends" => "it ends",
        "group_done" => "a group of its plan is done",
        "task_done" => "a task of its plan is done",
        "pr_pushed" => "its pull request is pushed",
        "pr_lands" => "its pull request lands",
        _ => "it moves",
    }
}

/// What the store holds that an end or a merge of `id` may unblock: those that wait on it, those it
/// started, its Later items. The rest is read after the store is let go.
fn held_unblocks(store: &Store, id: &str) -> Unblocks {
    let title_of = |id: &str| store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
    let waiting = store
        .waits_on(id)
        .unwrap_or_default()
        .into_iter()
        .take(MOST_LISTED)
        .map(|wait| Tied { title: title_of(&wait.session_id), how: format!("waits to start until {}", wait_said(&wait.condition)), session_id: wait.session_id })
        .collect();
    let started = store
        .attachments_at("started_by", id, None)
        .unwrap_or_default()
        .into_iter()
        .map(|one| one.holder.id)
        .filter(|child| store.session(child).ok().flatten().is_some_and(|one| one.state != store::SessionState::Ended))
        .take(MOST_LISTED)
        .map(|child| Tied { title: title_of(&child), how: "was started by it".to_string(), session_id: child })
        .collect();
    let later = store.later_of(id).unwrap_or_default().into_iter().take(MOST_LISTED).map(|one| one.sentence).collect();
    Unblocks { waiting, started, later, ..Unblocks::default() }
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
    /// Every end and merge of a hosted Job that is there now, marked handed. Nothing is said.
    async fn ends_marked_seen(&self) {
        let now = self.now();
        let mut store = self.store().lock().await;
        for hosting in store.hostings().unwrap_or_default() {
            let id = hosting.session_id;
            let held = store.attachments_of(&Holder::session(&id)).unwrap_or_default();
            if store.session(&id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Ended) && !held.iter().any(|one| one.kind == REPLACED_BY) {
                let _ = store.keep_linked_told(&id, ENDED, "", &now);
            }
            for pull in held.iter().filter(|one| one.kind == "pr" && crate::pull_requesting::pull_merged(&one.detail, one.state == AttachmentState::Spent)) {
                let _ = store.keep_linked_told(&id, MERGED, &pull.target, &now);
            }
        }
    }

    /// Whether this is the first time `event` under `key` is seen for `job`, and keeps it.
    async fn heard_once(&self, job: &str, event: &str, key: &str) -> bool {
        let now = self.now();
        self.store().lock().await.keep_linked_told(job, event, key, &now).unwrap_or(false)
    }

    /// One record of a Job that ended or holds a merged pull request: each end and merge in it that
    /// was not heard before is marked, and handed to the Operator where it is on.
    async fn end_heard(self: &Arc<Self>, record: SessionRecord) {
        let id = record.id.as_str().to_string();
        let Some(manifest) = self.store().lock().await.hosting(&id).ok().flatten().map(|one| one.manifest_id) else { return };
        if ended_for_good(&record) && self.heard_once(&id, ENDED, "").await && self.operator_on() {
            if let Some(operator) = self.operator_seat().operator() {
                operator.settled_by_end(&id).await;
            }
            self.hand_once(self.ended_of(&id, &manifest).await).await;
        }
        for pull in record.attachments.iter().filter(|one| one.kind == "pr" && crate::after_following::merged(one)) {
            if self.heard_once(&id, MERGED, &pull.target).await && self.operator_on() {
                let title = pull.detail.get("title").cloned().unwrap_or_default();
                self.hand_once(self.merged_of(&id, &manifest, &pull.target, &title).await).await;
            }
        }
    }

    /// The end of `id` with what it may unblock.
    async fn ended_of(&self, id: &str, manifest: &str) -> Arrival {
        let (session, summary, pulls, starter, paths, issues, mut unblocks) = {
            let store = self.store().lock().await;
            let pulls = store
                .attachments_of(&Holder::session(id))
                .unwrap_or_default()
                .iter()
                .filter(|one| one.kind == "pr")
                .map(|one| Pulled { number: one.target.clone(), title: one.detail.get("title").cloned().unwrap_or_default(), state: pull_state(one) })
                .collect::<Vec<_>>();
            let (paths, issues, _) = held_of(&store, id);
            (store.session(id).ok().flatten(), store.done_summary(id).ok().flatten(), pulls, starter_of(&store, id), paths, issues, held_unblocks(&store, id))
        };
        let title = session.as_ref().and_then(|one| one.title.clone()).unwrap_or_else(|| id.to_string());
        let reason = session.and_then(|one| one.end_reason);
        let (open, files) = self.near_and_files(manifest, &format!("{title} {}", summary.as_deref().unwrap_or_default())).await;
        unblocks.open = open;
        unblocks.needs = self.needs_naming(id, &[id, &title]).await;
        Ended { session_id: id.to_string(), manifest_id: manifest.to_string(), title, summary, reason, pulls, starter, unblocks, paths, issues, files }.arrival()
    }

    /// The merge of pull request `number` of Job `id`, with what it may unblock.
    async fn merged_of(&self, id: &str, manifest: &str, number: &str, title: &str) -> Arrival {
        let (job, starter, paths, issues, mut unblocks) = {
            let store = self.store().lock().await;
            let job = store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
            let (paths, issues, _) = held_of(&store, id);
            (job, starter_of(&store, id), paths, issues, held_unblocks(&store, id))
        };
        let (open, files) = self.near_and_files(manifest, &format!("{title} {job}")).await;
        unblocks.open = open;
        unblocks.needs = self.needs_naming(id, &[id, &format!("#{number}"), title]).await;
        Merged { session_id: id.to_string(), manifest_id: manifest.to_string(), job, number: number.to_string(), title: title.to_string(), unblocks, starter, paths, issues, files }.arrival()
    }

    /// The open issues sharing `text`'s words, and whether the repository's tracker is beads.
    async fn near_and_files(&self, manifest: &str, text: &str) -> (Vec<super::Filed>, bool) {
        let open = open_near(text, &self.open_of(manifest).await);
        let files = self.served_named(Some(&ManifestId::carried(manifest))).ok().is_some_and(|served| self.backlog_of(&served).is_some());
        (open, files)
    }

    /// The Needs you items of other Jobs on this Mac that name one of `names`, or wait on a restart.
    async fn needs_naming(&self, job: &str, names: &[&str]) -> Vec<String> {
        let needs = self.operator_board().await.needs_you;
        needs
            .into_iter()
            .filter(|one| one.session_id != job)
            .filter(|one| {
                let text = one.text.to_lowercase();
                text.contains("restart") || names.iter().any(|name| !name.is_empty() && one.text.contains(name))
            })
            .take(MOST_LISTED)
            .map(|one| format!("Job {} \"{}\" asks: {}", one.session_id, one.title, cut(one.text.trim(), MOST_WORDS)))
            .collect()
    }

    /// An end or a merge ruled to a live Job it is tied to: told, with what happened and why.
    pub(super) async fn operator_told_ending(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling, into: &str) -> Result<(), String> {
        let text = format!(
            "The Operator tells you what may unblock your work. {}\n\nWhy the Operator tells you: {}\n\nCarry on with it, or say why it changes nothing for you.",
            arrival.words, ruling.because
        );
        match self.told_by_fleet(into, &text, Notice::Operator).await {
            Ok(true) => Ok(()),
            _ => Err("the Job did not take the word".to_string()),
        }
    }

    /// The next piece of work an end or a merge was ruled to start, as a Job of its own, briefed with
    /// what happened. The Job's id.
    pub(super) async fn operator_started_next(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling) -> Option<String> {
        let what = ruling.what.as_deref()?.trim();
        let started = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(&arrival.manifest_id),
                title: Some(cut(what.lines().next().unwrap_or(what), 120)),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await;
        let record = started.ok()?;
        let job = record.id.as_str().to_string();
        let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
        let brief = format!("The Operator starts this as the next piece of work: {what}\n\nWhat happened: {}\n\nWhy: {}", arrival.words, ruling.because);
        if self.first_message_from(&job, voice, &brief).await.is_err() {
            let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
            return None;
        }
        Some(job)
    }

    /// The next piece of work an end or a merge was ruled to file, in the repository's beads. The issue.
    pub(super) async fn operator_filed_next(&self, arrival: &Arrival, ruling: &Ruling) -> Option<String> {
        let what = ruling.what.as_deref()?.trim().to_string();
        let served = self.served_named(Some(&ManifestId::carried(&arrival.manifest_id))).ok()?;
        let backlog = self.backlog_of(&served)?;
        let root = served.root().to_string();
        let body = format!("Filed by the Operator. {}\n\nWhy: {}", arrival.words, ruling.because);
        let filed = tokio::task::spawn_blocking(move || backlog.file(&root, &what, &body)).await;
        let Ok(Ok(reference)) = filed else { return None };
        held(&self.operator_seat().sources.filed).insert(reference.clone());
        self.tracking().forget(&arrival.manifest_id);
        Some(reference)
    }
}

/// The Job that started `id`, nearest, as it is tied to what `id` did.
fn starter_of(store: &Store, id: &str) -> Option<Tied> {
    started_by_chain(store, id).into_iter().next().map(|(session_id, title)| Tied { session_id, title, how: "started the Job this is about".to_string() })
}
