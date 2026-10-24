//! A Job started `held` waits on its start card: no agent runs until `begin_session`.
//! The person settles the title, workflow, model, effort and how the work lands, then presses Start.
//!
//! **Held in two places.** The hosting's `start_held` survives a restart; the runtime's
//! `proposal.start_held` is what `holds_back` reads, and `held` is the queue behind it. A Fleet that
//! restarts loses the queue, so the first send or begin after it reads the person's messages back off
//! the thread (`restored`) before anything else is done to the Job.
//!
//! **The usual landing** is the most frequent of the last ten picks for the Job's workflow in its
//! repository, once there are three, a tie going to the latest.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use base64::Engine as _;
use ipc::{
    BeginSession, DeliveryChoice, DeliveryPicked, DeliveryPickedFrom, SentFile, SessionRecord, SessionRow, SessionVoice, StartHeld,
};
use store::{KeptHosting, Store};

use super::delivering::{delivery_asked, first_message, named_delivery};
use super::proposing::Held;
use super::uploads::Kept;
use crate::daemon::Fleet;

/// How many of the latest picks the usual landing is read from.
const READ: u32 = 10;

/// The fewest picks that make one usual.
const FEWEST: usize = 3;

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
    first: Option<String>,
    usual: Option<DeliveryChoice>,
}

/// The first message of the person's and the usual landing for the Job's workflow.
pub(crate) fn start_read(store: &Store, hosting: &KeptHosting) -> Result<StartReads, store::WriteError> {
    let first = store.session_rows(&hosting.session_id)?.iter().find_map(|body| {
        match ipc::decode::<SessionRow>("a job row", body.as_bytes()) {
            Ok(SessionRow::Message { from: SessionVoice::You, text, .. }) => Some(text),
            _ => None,
        }
    });
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

    /// The card's facts. The landing the first message names beats the usual one.
    pub(crate) fn start_wire(&self, hosting: &KeptHosting, reads: StartReads) -> StartHeld {
        let named = reads.first.as_deref().and_then(named_delivery);
        let picked = named
            .map(|choice| DeliveryPicked { choice, from: DeliveryPickedFrom::Named })
            .or(reads.usual.map(|choice| DeliveryPicked { choice, from: DeliveryPickedFrom::Usual }));
        StartHeld { asks_delivery: self.asks_delivery_of(hosting), picked }
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

    /// Make the runtime agree with a Job held across a restart: it holds back, and the queue holds
    /// the person's messages as the thread has them. Once per Fleet, and before anything is written
    /// to the thread, so a message sent now is not read back as an old one.
    pub(crate) async fn start_hold_restored(self: &Arc<Self>, id: &str, hosting: &KeptHosting) -> Result<(), Refusal> {
        if !hosting.start_held {
            return Ok(());
        }
        let runtime = self.hosts().of(id);
        if runtime.state().proposal.start_synced {
            return Ok(());
        }
        let mut held = Vec::new();
        for row in self.rows_of(id).await? {
            if let SessionRow::Message { from: SessionVoice::You, text, files, tags, .. } = row {
                let addressed = self.addressed(&tags).await?;
                held.push(Held { text, kept: self.uploads_again(id, &files), addressed, ahead: None, instead: None });
            }
        }
        let mut state = runtime.state();
        if !state.proposal.start_synced {
            state.proposal.held.extend(held);
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

    /// `begin_session`: Start pressed on the start card. **Refused with nothing changed** where the Job
    /// is not held (the answer a delivery answer gives for one settled already), and where its
    /// workflow asks how the work lands and `delivery` is absent. Otherwise the landing is kept as the
    /// delivery question's answer, so the plan step asks nothing, the pick is recorded, the hold is
    /// lifted and the turns that waited are sent.
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
        let rows = self.rows_of(id).await?;
        // The press that claims the flag is the one that begins; a second finds it settled.
        {
            let mut store = self.store().lock().await;
            let mut now = store.hosting(id).map_err(|why| self.ledger_fault(why))?.filter(|one| one.start_held).ok_or_else(|| self.settled_already())?;
            now.start_held = false;
            store.keep_hosting(&now).map_err(|why| self.ledger_fault(why))?;
        }
        if let Some(choice) = delivery {
            if !delivery_asked(&rows) {
                let named = first_message(&rows).and_then(named_delivery) == Some(choice);
                self.row_put(id, SessionRow::Delivery { id: self.row_id(id), at: self.instant(), choice: Some(choice), named }).await;
            }
            self.auto_land_follows(id, Some(choice)).await;
            self.delivery_pick_kept(id, choice).await;
        }
        // A reading still out sends the turns itself, on its answer.
        let drain = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
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
