//! A Job started `held` waits on its start card: no agent runs until `begin_session`.
//! The person settles the title, workflow, model, effort and how the work lands, then presses Start.
//!
//! **Held in two places.** The hosting's `start_held` survives a restart; the runtime's
//! `proposal.start_held` is what `holds_back` reads. A Fleet that restarts loses it, so the first send
//! or begin after one sets it again (`start_hold_restored`) before anything else is done to the Job.
//!
//! **No turn reaches the thread while the Job is held.** The first message, note or other Job's word
//! to reach a held Job is the one the agent will read first, and the person's to edit in his message
//! box (`StartHeld.message`). Every turn after it waits behind it. Each is kept whole in the store with
//! its files and Fleet's lines (`held_first_turns`, `held_later_turns`), so a restart loses none of
//! them, and Start writes their rows in the order they came, then sends them, so the thread reads as
//! the agent does.
//!
//! **Never blank.** A held Job with no workflow takes the repository's `feature` workflow, else its
//! first, once the proposer has read it or at once where nothing will; and a card that asks how the
//! work lands always offers one, Auto land where nothing else names one.
//!
//! **The usual landing** is the most frequent of the last ten picks for the Job's workflow in its
//! repository, once there are three, a tie going to the latest.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use base64::Engine as _;
use config::ResolvedWorkflow;
use core_model::{Ulid, WorkflowId};
use ipc::{
    BeginSession, DeliveryChoice, DeliveryPicked, DeliveryPickedFrom, ManifestId, SentFile, SessionRecord, SessionRow, SessionTag,
    SessionVoice, StartHeld,
};
use serde::{Deserialize, Serialize};
use store::{KeptHosting, Store};

use super::delivering::{delivery_asked, first_message, named_delivery};
use super::serving::{NO_SUCH_SESSION, SESSION_UNSTARTED};
use super::proposing::Held;
use super::uploads::Kept;
use crate::daemon::Fleet;

/// How many of the latest picks the usual landing is read from.
const READ: u32 = 10;

/// The fewest picks that make one usual.
const FEWEST: usize = 3;

/// The workflow a held Job with none takes, where its repository holds it.
const DEFAULT_WORKFLOW: &str = "feature";

/// The landing a card offers where nothing names one and the person has no usual.
const DEFAULT_LANDING: DeliveryChoice = DeliveryChoice::AutoLand;

/// The id of the waiting item a held Job raises, followed by the Job's.
pub(crate) const FIRST_MESSAGE_ITEM: &str = "unsent:";

/// A held Job's first turn as the store keeps it: what its thread row will say, and what the agent
/// reads in place of the words where Fleet adds to them (`instead`) or puts a line ahead (`ahead`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FirstTurn {
    pub from: SessionVoice,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<SentFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<SessionTag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instead: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead: Option<String>,
}

impl FirstTurn {
    /// Words sent as `from`, as the agent will read them.
    pub(crate) fn said(from: SessionVoice, text: &str) -> FirstTurn {
        FirstTurn { from, text: text.to_string(), files: Vec::new(), tags: Vec::new(), instead: None, ahead: None }
    }

    /// What the agent reads of it, short of the lines a turn starts with.
    pub(crate) fn reads(&self) -> &str {
        self.instead.as_deref().unwrap_or(&self.text)
    }

    /// The turn with the words he left on the card. **His words, whole**: Fleet's lines and another
    /// Job's voice go, and the files stay. `None` where nothing is left to send.
    fn edited(self, words: &str) -> Option<FirstTurn> {
        if words == self.reads().trim() {
            return Some(self);
        }
        (!words.is_empty() || !self.files.is_empty()).then(|| FirstTurn {
            from: SessionVoice::You,
            text: words.to_string(),
            instead: None,
            ..self
        })
    }
}

/// The workflow a held Job with none takes: the repository's `feature`, else its first.
fn default_workflow(held: &std::collections::BTreeMap<WorkflowId, ResolvedWorkflow>) -> Option<String> {
    let feature = WorkflowId::carried(Ulid::carried(DEFAULT_WORKFLOW));
    held.get(&feature).or_else(|| held.values().next()).map(|one| one.id().as_str().to_string())
}

/// A choice as the store keeps it.
fn spelled(choice: DeliveryChoice) -> &'static str {
    match choice {
        DeliveryChoice::AutoLand => "auto_land",
        DeliveryChoice::OpenPr => "open_pr",
        DeliveryChoice::KeepLocal => "keep_local",
    }
}

fn chosen(word: &str) -> Option<DeliveryChoice> {
    DeliveryChoice::OFFERED.into_iter().find(|one| spelled(*one) == word)
}

/// The usual landing among `picks`, newest first: the most frequent, the latest of those that tie.
/// None below three picks.
pub(crate) fn usual_of(picks: &[DeliveryChoice]) -> Option<DeliveryChoice> {
    if picks.len() < FEWEST {
        return None;
    }
    let count = |choice: DeliveryChoice| picks.iter().filter(|one| **one == choice).count();
    let mut best: Option<(DeliveryChoice, usize)> = None;
    for choice in picks.iter().copied() {
        let times = count(choice);
        if best.map_or(true, |(_, most)| times > most) {
            best = Some((choice, times));
        }
    }
    best.map(|(choice, _)| choice)
}

/// What the start card is drawn from, taken under the store lock.
pub(crate) struct StartReads {
    first: Option<FirstTurn>,
    usual: Option<DeliveryChoice>,
}

/// A kept first turn read back. One that will not read is none: the card shows an empty box.
fn first_turn_of(body: &str) -> Option<FirstTurn> {
    ipc::decode::<FirstTurn>("a held job's first turn", body.as_bytes()).ok()
}

/// The held Job's first turn and the usual landing for its workflow.
pub(crate) fn start_read(store: &Store, hosting: &KeptHosting) -> Result<StartReads, store::WriteError> {
    let first = store.held_first_turn(&hosting.session_id)?.as_deref().and_then(first_turn_of);
    let usual = match hosting.workflow.as_deref() {
        Some(workflow) => {
            let picks: Vec<DeliveryChoice> =
                store.delivery_picks(&hosting.manifest_id, workflow, READ)?.iter().filter_map(|word| chosen(word)).collect();
            usual_of(&picks)
        }
        None => None,
    };
    Ok(StartReads { first, usual })
}

/// Another Job's message, held for the agent to read as `said`.
pub(crate) fn job_turn(text: &str, said: String) -> Held {
    let kept = Kept { files: Vec::new(), pictures: Vec::new(), paths: Vec::new(), picture_paths: Vec::new() };
    Held { text: text.to_string(), kept, addressed: Vec::new(), ahead: None, instead: Some(said) }
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
    /// Whether the Job's workflow has a step that asks how the work lands.
    fn asks_delivery_of(&self, hosting: &KeptHosting) -> bool {
        self.workflows_on()
            && hosting
                .workflow
                .as_deref()
                .and_then(|workflow| self.rail_steps(&hosting.manifest_id, workflow))
                .is_some_and(|steps| steps.iter().any(|step| step.asks_delivery))
    }

    /// The card's facts. The landing the first message names beats the usual one, and Auto land
    /// stands in for both where the card asks.
    pub(crate) fn start_wire(&self, hosting: &KeptHosting, reads: StartReads) -> StartHeld {
        let asks_delivery = self.asks_delivery_of(hosting);
        let named = reads.first.as_ref().and_then(|first| named_delivery(&first.text));
        let picked = named
            .map(|choice| DeliveryPicked { choice, from: DeliveryPickedFrom::Named })
            .or(reads.usual.map(|choice| DeliveryPicked { choice, from: DeliveryPickedFrom::Usual }))
            .or(asks_delivery.then_some(DeliveryPicked { choice: DEFAULT_LANDING, from: DeliveryPickedFrom::Default }));
        StartHeld { asks_delivery, picked, message: reads.first.map(|first| first.reads().to_string()) }
    }

    /// Keep `turn` as the held Job's first, where it is held and has none yet. `false` where it is not
    /// the first, or the Job is not held: the caller writes and sends it as any other.
    pub(crate) async fn first_turn_kept(&self, id: &str, turn: &FirstTurn) -> Result<bool, Refusal> {
        self.held_turn_kept(id, turn, false).await
    }

    /// Keep `turn` off the thread, as the held Job's first where it has none and behind its others where
    /// it has. `false` only where the Job is not held: the caller writes and sends it as any other.
    pub(crate) async fn turn_kept_while_held(&self, id: &str, turn: &FirstTurn) -> Result<bool, Refusal> {
        self.held_turn_kept(id, turn, true).await
    }

    async fn held_turn_kept(&self, id: &str, turn: &FirstTurn, behind: bool) -> Result<bool, Refusal> {
        if !self.hosts().of(id).state().proposal.start_held {
            return Ok(false);
        }
        let body = ipc::encode(turn).map_err(|why| self.hosted_fault(SESSION_UNSTARTED, &why.to_string()))?;
        let kept = {
            let mut store = self.store().lock().await;
            // Read under the lock Start claims the hold in, so a turn that arrives as it is pressed queues.
            let held = store.hosting(id).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.start_held);
            if !held {
                false
            } else if store.keep_held_first_turn(id, &body).map_err(|why| self.ledger_fault(why))? {
                true
            } else if behind {
                store.keep_held_later_turn(id, &body).map_err(|why| self.ledger_fault(why))?;
                true
            } else {
                false
            }
        };
        if kept {
            self.held_defaulted(id).await;
        }
        Ok(kept)
    }

    /// Hold a Job that is not yet held, as a met wait's is, and keep `message` as its first turn, his
    /// to edit and send: an agent's start, a fork it made and a wait it set all wait for him so.
    pub(crate) async fn held_with(&self, id: &str, message: &str) -> Result<(), Refusal> {
        {
            let mut store = self.store().lock().await;
            let mut hosting = store
                .hosting(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_SUCH_SESSION, &format!("no job Fleet hosts is named {id}")))?;
            if !hosting.start_held {
                hosting.start_held = true;
                store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
            }
        }
        {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            state.proposal.start_held = true;
            state.proposal.start_synced = true;
        }
        self.first_turn_kept(id, &FirstTurn::said(SessionVoice::You, message)).await?;
        // As his send of it once did: the issues and links it names are the Job's from the start.
        self.issues_named(id, message, &[]).await;
        self.links_seen(id, message, crate::session_links::LinkedBy::You).await;
        Ok(())
    }

    /// A held Job with no workflow takes the repository's `feature` workflow, else its first: once,
    /// never over one the person chose (custom too), and not while a reading is out, whose end calls
    /// this again. Nothing with workflows off.
    pub(crate) async fn held_defaulted(&self, id: &str) {
        if !self.workflows_on() {
            return;
        }
        {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            let proposal = &mut state.proposal;
            if !proposal.start_held || proposal.defaulted || proposal.workflow_chosen || proposal.reading.is_some() {
                return;
            }
            proposal.defaulted = true;
        }
        let mut store = self.store().lock().await;
        let Ok(Some(mut hosting)) = store.hosting(id) else { return };
        if hosting.workflow.is_some() || !hosting.start_held {
            return;
        }
        let held = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map(|served| served.workflows())
            .unwrap_or_default();
        let Some(workflow) = default_workflow(&held) else { return };
        hosting.workflow = Some(workflow);
        if let Err(why) = store.keep_hosting(&hosting) {
            self.session_warned(id, "a held job's workflow was not set", format!("{why:?}"));
        }
    }

    /// Keep that the person chose `choice` for the Job's kind of work. A custom Job has no kind.
    pub(crate) async fn delivery_pick_kept(&self, id: &str, choice: DeliveryChoice) {
        let mut store = self.store().lock().await;
        let Ok(Some(hosting)) = store.hosting(id) else { return };
        let Some(workflow) = hosting.workflow.as_deref() else { return };
        if let Err(why) = store.keep_delivery_pick(&hosting.manifest_id, workflow, spelled(choice), self.now().as_str()) {
            self.session_warned(id, "how the work lands was not kept", format!("{why:?}"));
        }
    }

    /// Make the runtime agree with a Job held across a restart: it holds back. The turns that came to it
    /// are in the store, not the runtime, so nothing is read back. Once per Fleet, and before anything is
    /// done to the Job.
    pub(crate) async fn start_hold_restored(self: &Arc<Self>, id: &str, hosting: &KeptHosting) -> Result<(), Refusal> {
        if !hosting.start_held {
            return Ok(());
        }
        let runtime = self.hosts().of(id);
        let mut state = runtime.state();
        if !state.proposal.start_synced {
            state.proposal.start_held = true;
            state.proposal.start_synced = true;
        }
        Ok(())
    }

    /// The files a message carried, found again by the ids its row names. One no longer kept is left out.
    fn uploads_again(&self, id: &str, files: &[SentFile]) -> Kept {
        let directory = self.uploads_of(id);
        let mut kept = Kept { files: Vec::new(), pictures: Vec::new(), paths: Vec::new(), picture_paths: Vec::new() };
        for file in files {
            let prefix = format!("{}-", file.id);
            let found = std::fs::read_dir(&directory)
                .ok()
                .and_then(|entries| entries.flatten().find(|entry| entry.file_name().to_string_lossy().starts_with(&prefix)));
            let Some(found) = found else { continue };
            let path = found.path().to_string_lossy().into_owned();
            if file.media_type.starts_with("image/") {
                let Ok(bytes) = std::fs::read(found.path()) else { continue };
                kept.pictures.push((file.media_type.clone(), base64::engine::general_purpose::STANDARD.encode(bytes)));
                kept.picture_paths.push(path);
            } else {
                kept.paths.push(path);
            }
            kept.files.push(file.clone());
        }
        kept
    }

    /// A kept turn written to the thread as Start sends it, and the turn the agent is sent of it.
    async fn first_turn_written(&self, id: &str, first: FirstTurn) -> Result<Held, Refusal> {
        let FirstTurn { from, text, files, tags, instead, ahead } = first;
        let his = from == SessionVoice::You;
        let row = SessionRow::Message {
            id: self.row_id(id),
            at: self.instant(),
            from,
            text: text.clone(),
            files: files.clone(),
            tags: tags.clone(),
            notice: None,
        };
        let row_id = row.id().to_string();
        if his {
            self.hosts().of(id).state().activity.writing.person_said(&row_id, &text);
        }
        self.row_put(id, row).await;
        if his {
            // The artifacts drawn from here on follow it, as they follow any message of his.
            if let Err(why) = self.store().lock().await.keep_session_message(id, &row_id) {
                self.session_warned(id, "a job's latest message was not kept", format!("{why:?}"));
            }
        }
        let addressed = self.addressed(&tags).await?;
        Ok(Held { text, kept: self.uploads_again(id, &files), addressed, ahead, instead })
    }

    /// `begin_session`: Start pressed on the start card. **Refused with nothing changed** where the Job
    /// is not held (the answer a delivery answer gives for one settled already), and where its
    /// workflow asks how the work lands and `delivery` is absent. Otherwise the first turn is written
    /// to the thread, his edit of it where `message` says one, the landing is kept as the delivery
    /// question's answer, so the plan step asks nothing, the pick is recorded, the hold is lifted and
    /// the first turn goes, then the turns that waited.
    pub(crate) async fn session_begun(self: &Arc<Self>, begin: BeginSession) -> Result<SessionRecord, Refusal> {
        let id = begin.session_id.as_str();
        let (session, hosting) = self.session_and_hosting(id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        if !hosting.start_held {
            return Err(self.settled_already());
        }
        let delivery = match (self.asks_delivery_of(&hosting), begin.delivery) {
            (true, None) => {
                return Err(self.hosted_refusal("fleet.session_waiting_empty", "how the work is delivered is chosen before the job starts"));
            }
            (true, choice) => choice,
            (false, _) => None,
        };
        self.start_hold_restored(id, &hosting).await?;
        // The press that claims the flag is the one that begins; a second finds it settled.
        let (first, later) = {
            let mut store = self.store().lock().await;
            let mut now = store.hosting(id).map_err(|why| self.ledger_fault(why))?.filter(|one| one.start_held).ok_or_else(|| self.settled_already())?;
            now.start_held = false;
            store.keep_hosting(&now).map_err(|why| self.ledger_fault(why))?;
            let first = store.take_held_first_turn(id).map_err(|why| self.ledger_fault(why))?;
            (first, store.take_held_later_turns(id).map_err(|why| self.ledger_fault(why))?)
        };
        let edit = begin.message.as_deref().map(str::trim);
        let first = match (first.as_deref().and_then(first_turn_of), edit) {
            (first, None) => first,
            (Some(first), Some(words)) => first.edited(words),
            // A box he filled on a Job that held nothing: his words are its first turn.
            (None, Some(words)) => (!words.is_empty()).then(|| FirstTurn::said(SessionVoice::You, words)),
        };
        // The thread reads as the agent will: his first, then the rest in the order they came.
        let mut turns = Vec::new();
        if let Some(first) = first {
            turns.push(self.first_turn_written(id, first).await?);
        }
        for turn in later.iter().filter_map(|body| first_turn_of(body)) {
            turns.push(self.first_turn_written(id, turn).await?);
        }
        let rows = self.rows_of(id).await?;
        if let Some(choice) = delivery {
            if !delivery_asked(&rows) {
                let named = first_message(&rows).and_then(named_delivery) == Some(choice);
                self.row_put(id, SessionRow::Delivery { id: self.row_id(id), at: self.instant(), choice: Some(choice), named }).await;
            }
            self.auto_land_follows(id, Some(choice)).await;
            self.delivery_pick_kept(id, choice).await;
        }
        // A reading still out sends the turns itself, on its answer. They go ahead of any that raced the press.
        let drain = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            for held in turns.into_iter().rev() {
                state.proposal.held.push_front(held);
            }
            state.proposal.start_held = false;
            let drain = state.proposal.reading.is_none() && !state.proposal.draining;
            state.proposal.draining |= drain;
            drain
        };
        let record = self.published_hosted(id).await?;
        if drain {
            self.held_sent(id).await;
            return self.published_hosted(id).await;
        }
        Ok(record)
    }
}
