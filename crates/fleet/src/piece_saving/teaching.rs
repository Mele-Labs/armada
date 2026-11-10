//! `teach_ticket_word`: a tracker's own status read as one Ticket state, from the menu on a ticket.
//!
//! **The one exception to a Job keeping the copy it started with** (`lifecycles::copying`): the
//! owner chose that a taught word reaches every live Job in the scope it was written at, so a
//! ticket in front of him moves the moment he says what its status means. The move is made on each
//! Job's own Ticket, never by handing it Kit's, so what the Job changed in its copy stays.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{LifecycleFile, Projects, Reach};
use core_model::{Lifecycle, LifecycleScope, ManifestId, Observed, Project, ScopeId, StateSource, Ulid};
use ipc::{
    LifecycleDefinition, LifecyclePiece, SaveLifecyclePiece, SessionId, StateReadFrom, TeachTicketWord, TicketWordTaught,
};
use store::{KeptCopyMark, KeptPiece, PieceKind, Store};

use super::{written_as, PIECE_UNFIT};
use crate::daemon::Fleet;
use crate::lifecycles::copying::frozen_lifecycles;
use crate::lifecycles::wire;

const TICKET: &str = "ticket";

/// `word` taken off every state of `ticket` that lists it and added to `state`'s, matched exactly.
/// The sentence says why where `state` is not a state the tracker is read for.
pub(crate) fn taught(ticket: &mut LifecycleDefinition, word: &str, state: &str) -> Result<(), String> {
    let Some(target) = ticket.states.iter().position(|one| one.id == state) else {
        let held: Vec<&str> = ticket.states.iter().map(|one| one.id.as_str()).collect();
        return Err(format!("The {} has no state `{state}`; its states are {}", ticket.name, held.join(", ")));
    };
    if !matches!(ticket.states[target].read, StateReadFrom::Tracker { .. }) {
        return Err(format!("The {} reads `{state}` from something other than the tracker", ticket.name));
    }
    for one in &mut ticket.states {
        if let StateReadFrom::Tracker { words } = &mut one.read {
            words.retain(|held| held != word);
        }
    }
    if let StateReadFrom::Tracker { words } = &mut ticket.states[target].read {
        words.push(word.to_string());
    }
    Ok(())
}

/// Where the word is written. **The running Ticket's own scope where it is a repository's or a
/// Project's**, so Kit's file there is edited or, for a committed one, outranked at the same reach.
/// Otherwise the Project that holds the repository, the lowest id of several, and else the
/// repository: a Project-scoped file when a repository-scoped one runs would never be read.
fn taught_at(repository: &ManifestId, running: &LifecycleFile, projects: &Projects) -> Option<LifecycleScope> {
    if matches!(running.rank().reach, Reach::Repository | Reach::Project) {
        return Some(running.lifecycle().scope().clone());
    }
    let mut holding: Vec<&ScopeId> = projects.holding(repository).into_iter().map(Project::id).collect();
    holding.sort();
    match holding.first() {
        Some(id) => Some(LifecycleScope::Within((*id).clone())),
        None => ScopeId::new(repository.as_str()).ok().map(LifecycleScope::Within),
    }
}

/// Whether a Job in `manifest` reads what is scoped to `scope`.
fn reaches(scope: &LifecycleScope, projects: &Projects, manifest: &str) -> bool {
    match scope {
        LifecycleScope::Everywhere => true,
        LifecycleScope::Within(id) => {
            id.as_str() == manifest || {
                let repository = ManifestId::carried(Ulid::carried(manifest));
                projects.running().any(|one| one.id() == id && one.holds(&repository))
            }
        }
    }
}

/// A live Job the word reaches: its repository, and the lifecycles it read before the word was taught.
struct Reached {
    job: String,
    manifest: String,
    before: Vec<Lifecycle>,
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
    /// Teach `word` as `state` in Kit's Ticket for the repository, then in each live Job's copy the
    /// scope reaches, then read again each ticket those Jobs hold that last said `word`.
    pub(crate) async fn ticket_word_taught(&self, asked: TeachTicketWord) -> Result<TicketWordTaught, Refusal> {
        let word = asked.word.trim().to_string();
        let unfit = |said: String| self.piece_refusal(PIECE_UNFIT, said, None);
        if word.is_empty() {
            return Err(unfit("The tracker's status is blank; send it as the tracker says it".into()));
        }
        let repository = asked.manifest_id.to_domain();
        let applying = self.lifecycles_applying_to(&repository)?;
        let running = applying
            .files()
            .iter()
            .find(|file| file.overridden_by().is_none() && file.lifecycle().id().as_str() == TICKET)
            .ok_or_else(|| unfit(format!("No Ticket lifecycle applies to {}", repository.as_str())))?;
        let projects = self.projects_held();
        let scope = taught_at(&repository, running, &projects)
            .ok_or_else(|| unfit(format!("{} names no scope a Ticket can be kept at", repository.as_str())))?;
        let mut ticket = wire::lifecycle(running.lifecycle());
        ticket.scope = wire::scope(&scope);
        taught(&mut ticket, &word, &asked.state).map_err(unfit)?;

        // What each Job read before the save, so the ticket it holds can be seen moving off it.
        let reached = self.reached_by(&scope, &projects).await;
        let saved = self.lifecycle_piece_saved(SaveLifecyclePiece {
            piece: LifecyclePiece::Lifecycle { lifecycle: ticket },
            overwrite: true,
        })?;

        let mut jobs = Vec::new();
        for one in &reached {
            if self.copy_taught(one, &word, &asked.state).await {
                jobs.push(SessionId::carried(one.job.clone()));
            }
        }
        for one in &reached {
            self.tickets_read_again(one, &word).await;
        }
        for job in &jobs {
            self.copy_published(job.as_str()).await;
        }
        Ok(TicketWordTaught { saved, jobs })
    }

    /// The live Jobs that froze a copy or hold a ticket, in a repository `scope` reaches.
    async fn reached_by(&self, scope: &LifecycleScope, projects: &Projects) -> Vec<Reached> {
        let store = self.store().lock().await;
        let mut live = store.live_job_copies().unwrap_or_default();
        live.extend(store.live_lifecycle_holders(TICKET).unwrap_or_default());
        live.sort();
        live.dedup();
        live.into_iter()
            .filter_map(|job| {
                let manifest = store.session(&job).ok().flatten()?.manifest_id?;
                reaches(scope, projects, &manifest).then(|| {
                    let before = self.lifecycles_of_job(&store, &job, &manifest);
                    Reached { job, manifest, before }
                })
            })
            .collect()
    }

    /// The same move made on the Ticket in `one`'s own copy, kept with a mark on its lane. **False for
    /// a Job that froze none**, which reads Kit's, and for one whose own Ticket has no such state.
    async fn copy_taught(&self, one: &Reached, word: &str, state: &str) -> bool {
        let built_in = self.built_in_lifecycles(Some(one.manifest.as_str()));
        let mut store = self.store().lock().await;
        let Some(frozen) = frozen_lifecycles(&store, &one.job, built_in) else {
            return false;
        };
        let Some(own) = frozen.iter().find(|lifecycle| lifecycle.id().as_str() == TICKET) else {
            return false;
        };
        let mut ticket = wire::lifecycle(own);
        if taught(&mut ticket, word, state).is_err() {
            return false;
        }
        let Ok(text) = written_as(&LifecyclePiece::Lifecycle { lifecycle: ticket }).1 else {
            return false;
        };
        let path = kept_path(&store, &one.job);
        // The copy is read through the parser, so a Ticket it would refuse is not kept.
        if config::parse_lifecycle(Path::new(&path), &text).is_err() {
            return false;
        }
        let piece = KeptPiece { kind: PieceKind::Lifecycle, piece_id: TICKET.into(), path, text };
        if !store.keep_job_copy_piece(&one.job, &piece).unwrap_or(false) {
            return false;
        }
        let mark = KeptCopyMark {
            lane: TICKET.into(),
            item: state.to_string(),
            text: format!("`{word}` taught by you"),
            by: "you".into(),
            at: self.now().as_str().to_string(),
        };
        let _ = store.keep_job_copy_mark(&one.job, &mark);
        true
    }

    /// Each live ticket `one` holds whose tracker last said `word`, read again against what it reads now.
    async fn tickets_read_again(&self, one: &Reached, word: &str) {
        let held = self.store().lock().await.lifecycle_instances_of(&one.job).unwrap_or_default();
        let tracker = StateSource::Tracker.as_wire();
        for ticket in held.iter().filter(|held| {
            held.lifecycle == TICKET && held.ended_at.is_empty() && held.seen_source == tracker && held.seen == word
        }) {
            self.lifecycle_seen_since(&one.job, TICKET, &ticket.key, Observed::Tracker(word), Some(one.before.as_slice())).await;
        }
    }
}

/// Where the Job's copy says its Ticket came from, or where a Ticket added to it is said to be.
fn kept_path(store: &Store, job: &str) -> String {
    store
        .job_copy_of(job)
        .ok()
        .flatten()
        .and_then(|kept| {
            kept.pieces.into_iter().find(|piece| piece.kind == PieceKind::Lifecycle && piece.piece_id == TICKET)
        })
        .map_or_else(|| format!("(job copy)/lifecycles/{TICKET}.yml"), |piece| piece.path)
}
