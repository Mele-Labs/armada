//! `release_session_holds`: the person gives back everything a Session holds on the ledger, or
//! hands it to another live Session of the same repository, from Bridge, for when the agent cannot.
//! A slot is released by Bridge's own Release (`slot_pool_changed`), and handed by moving its lease.
//! `docs/concepts/session.md`, *Releasing what a Session holds*.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotHeld, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    ChangeSlotPool, ChosenHold, HoldKept, HoldMoved, ManifestId, ReleaseSessionHolds, SessionHoldsReleased,
    SessionTurn, SlotAct, StopSession, WireError,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptSession, SessionState};

use super::NO_SUCH_SESSION;
use crate::daemon::Fleet;
use crate::leasing::pool_of;

/// The ledger kinds a release acts on. Artifacts and the links between Sessions stay.
const HELD_KINDS: [&str; 3] = ["slot", "branch", "pr"];
/// A hand to the Session that holds them. A 409.
const HAND_TO_ITSELF: &str = "fleet.hand_to_itself";
/// A hand to a Session that has ended. A 409.
const HAND_TO_ENDED: &str = "fleet.hand_to_ended";
/// A hand to a Session of another repository. A 409.
const HAND_TO_OTHER_REPOSITORY: &str = "fleet.hand_to_other_repository";
/// A chosen row the Session does not hold standing. A 409.
const HOLD_NOT_STANDING: &str = "fleet.hold_not_standing";

/// One slot the pool holds for the Session, and who the pool says holds it.
struct Leased {
    slot: u32,
    holder: SlotHeld,
    branch: Option<String>,
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
    /// Give back or hand over the standing rows of [`HELD_KINDS`] the person chose, every one where he
    /// named none. **The target and the choice are checked before anything moves**, so a refusal
    /// changes nothing. A slot is moved before its rows, takes the `branch` row of what it has checked
    /// out with it, and where the pool would not move it keeps both.
    pub(crate) async fn session_holds_released(
        &self,
        asked: ReleaseSessionHolds,
    ) -> Result<SessionHoldsReleased, Refusal> {
        let id = asked.session_id.as_str().to_string();
        let source = self.known_session(&id).await?;
        let to = match &asked.to_session_id {
            Some(to) => Some(self.hand_target(&source, to.as_str()).await?),
            None => None,
        };
        let standing: Vec<KeptAttachment> = self
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(&id))
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|row| row.state == AttachmentState::Standing && HELD_KINDS.contains(&row.kind.as_str()))
            .collect();
        let manifest = source.manifest_id.clone();
        let served = match &manifest {
            Some(manifest) => self.served_named(Some(&ManifestId::carried(manifest))).ok(),
            None => None,
        };
        let leased_all = match &served {
            Some(served) => self.leased_for(served, &id).await,
            None => Vec::new(),
        };
        let mut chosen: Vec<(String, String)> = match &asked.rows {
            None => standing
                .iter()
                .map(|row| (row.kind.clone(), row.target.clone()))
                .chain(leased_all.iter().map(|one| (String::from("slot"), one.slot.to_string())))
                .collect(),
            Some(rows) => {
                // A slot the pool holds for the Session counts as held though its ledger has no row for it.
                let held = |row: &ChosenHold| {
                    standing.iter().any(|one| one.kind == row.kind && one.target == row.target)
                        || (row.kind == "slot" && leased_all.iter().any(|one| one.slot.to_string() == row.target))
                };
                if let Some(not) = rows.iter().find(|row| !held(row)) {
                    return Err(Refusal::IllegalMove(WireError::raised(
                        HOLD_NOT_STANDING,
                        format!("the job does not hold {} {}", not.kind, not.target),
                        self.run_id(),
                    )));
                }
                rows.iter().map(|row| (row.kind.clone(), row.target.clone())).collect()
            }
        };
        let picked = |chosen: &[(String, String)], kind: &str, target: &str| chosen.iter().any(|(k, t)| k == kind && t == target);
        let leased: Vec<Leased> = leased_all.into_iter().filter(|one| picked(&chosen, "slot", &one.slot.to_string())).collect();
        for one in &leased {
            if let Some(branch) = &one.branch {
                if standing.iter().any(|row| row.kind == "branch" && row.target == *branch) && !picked(&chosen, "branch", branch) {
                    chosen.push((String::from("branch"), branch.clone()));
                }
            }
        }
        let rows: Vec<KeptAttachment> = standing.into_iter().filter(|row| picked(&chosen, &row.kind, &row.target)).collect();
        let hosted = self.store().lock().await.hosting(&id).map_err(|why| self.ledger_fault(why))?;
        // Only a slot moved from under the agent needs its turn stopped.
        let slot_chosen = chosen.iter().any(|(kind, _)| kind == "slot");
        if slot_chosen && hosted.is_some() && source.state != SessionState::Ended {
            let working = matches!(self.hosts().of(&id).state().turn, SessionTurn::Working { .. });
            if working {
                self.stop_session(StopSession { session_id: asked.session_id.clone() }).await?;
            }
        }
        let mut answer = SessionHoldsReleased {
            to_session_id: to.as_ref().map(|one| ipc::SessionId::carried(one.id.clone())),
            ..SessionHoldsReleased::default()
        };
        let mut staying: Vec<(String, String)> = Vec::new();
        let mut handed_slots: Vec<(u32, Option<String>)> = Vec::new();
        if let Some(served) = &served {
            for leased in leased {
                let moved = match &to {
                    None => self.slot_released(served, &id, hosted.is_some(), &leased).await,
                    Some(to) => self.slot_handed(served, &leased, &to.id).await,
                };
                match moved {
                    Ok(released) => {
                        if to.is_some() {
                            handed_slots.push((leased.slot, leased.branch.clone()));
                        }
                        answer.moved.push(HoldMoved {
                            kind: String::from("slot"),
                            target: leased.slot.to_string(),
                            released,
                        });
                    }
                    Err(why) => {
                        staying.push((String::from("slot"), leased.slot.to_string()));
                        if let Some(branch) = &leased.branch {
                            staying.push((String::from("branch"), branch.clone()));
                        }
                        answer.kept.push(HoldKept {
                            kind: String::from("slot"),
                            target: leased.slot.to_string(),
                            why,
                        });
                    }
                }
            }
        }
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let fault = |why| self.ledger_fault(why);
            for row in &rows {
                if staying.iter().any(|(kind, target)| *kind == row.kind && *target == row.target) {
                    continue;
                }
                store
                    .settle(&row.holder, &row.kind, &row.manifest_id, &row.target, AttachmentState::GivenBack, &now)
                    .map_err(fault)?;
                if let Some(to) = &to {
                    let mut detail: BTreeMap<String, String> = row.detail.clone();
                    detail.insert(String::from("handed"), format!("session {id}"));
                    store
                        .attach(
                            &KeptAttachment {
                                holder: Holder::session(&to.id),
                                detail,
                                state: AttachmentState::Standing,
                                since: now.clone(),
                                changed_at: now.clone(),
                                ..row.clone()
                            },
                            false,
                        )
                        .map_err(fault)?;
                }
                let slot_already_named = row.kind == "slot" && answer.moved.iter().any(|one| one.kind == "slot" && one.target == row.target);
                if !slot_already_named {
                    answer.moved.push(HoldMoved {
                        kind: row.kind.clone(),
                        target: row.target.clone(),
                        released: None,
                    });
                }
            }
        }
        if let (Some(to), Some(served)) = (&to, &served) {
            // The central registry names a pull request's owner, and would still name the Job that gave it up.
            for one in answer.moved.iter().filter(|one| one.kind == "pr") {
                if let Ok(number) = one.target.parse::<u64>() {
                    let _ = self.pull_request_registered(served.root(), number, &crate::red_claim::Taker::Session(to.id.clone())).await;
                }
            }
        }
        if let Some(to) = &to {
            self.hostings_handed(&id, &to.id, &handed_slots).await;
            for (slot, _) in &handed_slots {
                self.session_slot(&id, source.title.clone(), manifest.clone(), *slot, false);
                self.session_slot(&to.id, to.title.clone(), manifest.clone(), *slot, true);
            }
        }
        if answer.moved.iter().any(|one| one.kind == "slot") {
            self.hosts().slot_freed();
        }
        let _ = self.published_hosted(&id).await;
        if let Some(to) = &to {
            let _ = self.published_hosted(&to.id).await;
        }
        Ok(answer)
    }

    async fn known_session(&self, id: &str) -> Result<KeptSession, Refusal> {
        self.store()
            .lock()
            .await
            .session(id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {id}")))
    }

    /// The Session a hand goes to: another, live, of the same repository.
    async fn hand_target(&self, source: &KeptSession, to: &str) -> Result<KeptSession, Refusal> {
        let refused = |code: &str, said: String| Refusal::IllegalMove(WireError::raised(code, said, self.run_id()));
        if to == source.id {
            return Err(refused(HAND_TO_ITSELF, String::from("a job cannot be handed what it already holds")));
        }
        let target = self.known_session(to).await?;
        let name = target.title.clone().unwrap_or_else(|| to.to_string());
        if target.state == SessionState::Ended {
            return Err(refused(HAND_TO_ENDED, format!("{name} has ended")));
        }
        if target.manifest_id != source.manifest_id {
            return Err(refused(HAND_TO_OTHER_REPOSITORY, format!("{name} works in another repository")));
        }
        Ok(target)
    }

    /// Every slot of `served`'s pool whose lease is the Session's, with who the pool names.
    async fn leased_for(&self, served: &crate::repositories::Served, id: &str) -> Vec<Leased> {
        let slots = crate::slot_holders::slots_of(served, id);
        if slots.is_empty() {
            return Vec::new();
        }
        let (vcs, pool) = (Arc::clone(self.vcs()), pool_of(served));
        let readings = tokio::task::spawn_blocking(move || vcs.slot_pool(&pool)).await.unwrap_or_default();
        readings
            .into_iter()
            .filter(|reading| slots.contains(&reading.slot))
            .map(|reading| Leased {
                slot: reading.slot,
                branch: adapters::leasing::checked_out(std::path::Path::new(&reading.path)).or(reading.branch),
                holder: reading.held,
            })
            .collect()
    }

    /// Release one slot as Bridge's Release does: commit what is uncommitted, then free it. A
    /// terminal Session holding as a Job of its own id, as a hand leaves it, is parked as a hosted
    /// one is.
    async fn slot_released(
        &self,
        served: &crate::repositories::Served,
        id: &str,
        hosted: bool,
        leased: &Leased,
    ) -> Result<Option<ipc::SlotReleased>, String> {
        let manifest = ManifestId::from(served.manifest().id());
        let holder = match &leased.holder {
            SlotHeld::Hosted(by) if by == id && !hosted => {
                let (vcs, pool, slot, by) = (Arc::clone(self.vcs()), pool_of(served), leased.slot, by.clone());
                let parked = tokio::task::spawn_blocking(move || vcs.park_hosted_slot(&pool, slot, &by))
                    .await
                    .map_err(|why| why.to_string())?
                    .map_err(|why| why.said())?;
                return Ok(crate::saving::released_answer(served, leased.slot, parked).released);
            }
            SlotHeld::Hosted(by) => by.clone(),
            SlotHeld::Session(said) => said.clone(),
            _ => return Err(String::from("No job holds it now.")),
        };
        let asked = ChangeSlotPool { act: SlotAct::Release, slot: Some(leased.slot), holder: Some(holder) };
        self.slot_pool_changed(asked, Some(&manifest))
            .await
            .map(|changed| changed.released)
            .map_err(|refusal| said_of(&refusal))
    }

    /// Move one slot's lease to `to` as it stands.
    async fn slot_handed(
        &self,
        served: &crate::repositories::Served,
        leased: &Leased,
        to: &str,
    ) -> Result<Option<ipc::SlotReleased>, String> {
        let from = match &leased.holder {
            SlotHeld::Hosted(by) => format!("job {by}"),
            SlotHeld::Session(said) => said.clone(),
            _ => return Err(String::from("No job holds it now.")),
        };
        let (vcs, pool, slot, to) = (Arc::clone(self.vcs()), pool_of(served), leased.slot, to.to_string());
        tokio::task::spawn_blocking(move || vcs.hand_session_slot(&pool, slot, &from, &to))
            .await
            .map_err(|why| why.to_string())?
            .map_err(|why| why.said())?;
        Ok(None)
    }

    /// A hosted source no longer works in a slot it handed, and a hosted target with no slot of its
    /// own works in the first it was handed. Each idle process is let go, so its next start is in
    /// the directory it now holds.
    async fn hostings_handed(&self, from: &str, to: &str, slots: &[(u32, Option<String>)]) {
        let mut idle = Vec::new();
        {
            let mut store = self.store().lock().await;
            if let Ok(Some(mut hosting)) = store.hosting(from) {
                if hosting.lease_slot.is_some_and(|slot| slots.iter().any(|(one, _)| *one == slot)) {
                    hosting.lease_slot = None;
                    hosting.lease_branch = None;
                    let _ = store.keep_hosting(&hosting);
                    idle.push(from);
                }
            }
            if let (Ok(Some(mut hosting)), Some((slot, branch))) = (store.hosting(to), slots.first()) {
                if hosting.lease_slot.is_none() {
                    hosting.lease_slot = Some(*slot);
                    hosting.lease_branch = branch.clone();
                    let _ = store.keep_hosting(&hosting);
                    idle.push(to);
                }
            }
        }
        for id in idle {
            self.let_go_if_idle(id);
        }
    }

    fn let_go_if_idle(&self, id: &str) {
        let runtime = self.hosts().of(id);
        let mut state = runtime.state();
        // A line written whose turn has not started would be lost with the process.
        if matches!(state.turn, SessionTurn::Idle) && state.queued == 0 {
            Self::let_go(&mut state);
        }
    }
}

/// A refusal's words, for the row that stayed.
pub(super) fn said_of(refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotFound(why) | Refusal::IllegalMove(why) | Refusal::Unacceptable(why) | Refusal::Fault(why) => {
            why.message.clone()
        }
    }
}
