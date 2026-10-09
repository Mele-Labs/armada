//! What a session waits on the person for: the agent's own list, kept whole, with
//! what Fleet already knows merged in on every read, and the one route that
//! settles an item. `docs/concepts/session.md`.
//!
//! **Derived items are never kept.** An open card, a permission or a walk window
//! is read off the session as it stands, so it cannot go stale.
//!
//! **Only a session's newest walk window can wait.** A newer `show_window` supersedes the older
//! ones, and a window stops waiting once it is approved or the person has sent the session a
//! message after it opened. A window closed without either still waits: that is his call.
//!
//! **A dismissed id never comes back.** `dismiss_waiting` keeps the id; a derived item with it is
//! left off every read and an agent item with it is dropped from the kept list.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AnswerSessionAsk, AnswerWaiting, DismissWaiting, HelmCallAnswer, HelmCallInFlight, Instant, QuestionAnswer,
    SendSessionMessage, SessionRecord, SessionRow, SessionVoice, SetWaitingFor, WaitingAct,
    WaitingActKind, WaitingItem, WaitingMode, WaitingOption, WaitingSource, WireError,
};
use store::{KeptSession, KeptWaiting, Store};

use crate::daemon::Fleet;

const SESSION_WAITING_UNNAMED: &str = "fleet.session_waiting_unnamed";
const SESSION_WAITING_EMPTY: &str = "fleet.session_waiting_empty";
const SESSION_WAITING_UNHELD: &str = "fleet.session_waiting_unheld";

/// A walk window a session showed, as the merge reads it.
pub(crate) struct Window {
    pub url: String,
    pub title: String,
    pub since: String,
}

/// What [`Fleet::waiting_from`] needs from the store: SQL only, taken under the lock.
pub(crate) struct WaitingReads {
    kept: Vec<KeptWaiting>,
    rows: Vec<String>,
    dismissed: Vec<String>,
}

pub(crate) fn waiting_reads(
    store: &Store,
    session: &KeptSession,
) -> Result<WaitingReads, store::WriteError> {
    Ok(WaitingReads {
        kept: store.waiting_for(&session.id)?,
        rows: store.session_rows(&session.id)?,
        dismissed: store.dismissed_waiting(&session.id)?,
    })
}

/// The line that tells the agent to settle an item itself.
pub(crate) fn mode_line(mode: WaitingMode) -> &'static str {
    match mode {
        WaitingMode::Best => "Decide this yourself. Think it through and go with the best option.",
        WaitingMode::Quick => "Decide this yourself. Take the reasonable path and keep moving.",
    }
}

fn offer_label(offer: &HelmCallAnswer) -> &'static str {
    match offer {
        HelmCallAnswer::AllowOnce => "Allow once",
        HelmCallAnswer::AllowAndRemember => "Allow and remember",
        HelmCallAnswer::Refuse => "Refuse",
    }
}

fn act_kind_text(kind: WaitingActKind) -> &'static str {
    match kind {
        WaitingActKind::Walk => "walk",
        WaitingActKind::Answer => "answer",
        WaitingActKind::ApprovePr => "approve_pr",
        WaitingActKind::Run => "run",
    }
}

fn act_kind_of(text: &str) -> Option<WaitingActKind> {
    match text {
        "walk" => Some(WaitingActKind::Walk),
        "answer" => Some(WaitingActKind::Answer),
        "approve_pr" => Some(WaitingActKind::ApprovePr),
        "run" => Some(WaitingActKind::Run),
        _ => None,
    }
}

/// The open card as an item, if one is open.
fn card_item(asked: &HelmCallInFlight) -> WaitingItem {
    let act = Some(WaitingAct {
        kind: WaitingActKind::Answer,
        target: asked.call.clone(),
    });
    if let Some(first) = asked.questions.first() {
        let single = asked.questions.len() == 1 && !first.multi_select;
        return WaitingItem {
            id: format!("ask:{}", asked.call),
            text: first.question.clone(),
            since: asked.asked_at.clone(),
            source: WaitingSource::AskCard,
            act,
            options: if single {
                first
                    .options
                    .iter()
                    .map(|one| WaitingOption {
                        label: one.label.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
        };
    }
    WaitingItem {
        id: format!("perm:{}", asked.call),
        text: format!("{} {}", asked.tool, asked.detail).trim().to_string(),
        since: asked.asked_at.clone(),
        source: WaitingSource::Permission,
        act,
        options: asked
            .offers
            .iter()
            .map(|one| WaitingOption {
                label: offer_label(one).to_string(),
            })
            .collect(),
    }
}

/// The agent's items, then Fleet's: the open card, then each walk window. **Where an agent item
/// names the same act and target as one of Fleet's, Fleet's stands** and the agent's is left off.
pub(crate) fn merged(
    kept: Vec<KeptWaiting>,
    asked: Option<&HelmCallInFlight>,
    windows: &[Window],
    dismissed: &[String],
    ended: bool,
) -> Vec<WaitingItem> {
    if ended {
        return Vec::new();
    }
    let mut derived: Vec<WaitingItem> = asked.map(card_item).into_iter().collect();
    derived.extend(windows.iter().map(|one| WaitingItem {
        id: format!("walk:{}", one.url),
        text: one.title.clone(),
        since: Instant::carried(&one.since),
        source: WaitingSource::Walk,
        act: Some(WaitingAct {
            kind: WaitingActKind::Walk,
            target: one.url.clone(),
        }),
        options: Vec::new(),
    }));
    let same = |item: &WaitingItem, act: &Option<(String, String)>| {
        matches!((&item.act, act), (Some(a), Some((kind, target)))
                if act_kind_text(a.kind) == kind && &a.target == target)
    };
    let mut items: Vec<WaitingItem> = kept
        .into_iter()
        .filter(|one| {
            !dismissed.contains(&one.item_id)
                && !derived
                    .iter()
                    .any(|fleet| fleet.id == one.item_id || same(fleet, &one.act))
        })
        .map(|one| WaitingItem {
            id: one.item_id,
            text: one.text,
            since: Instant::carried(&one.since),
            source: WaitingSource::Agent,
            act: one.act.and_then(|(kind, target)| {
                act_kind_of(&kind).map(|kind| WaitingAct { kind, target })
            }),
            options: one
                .options
                .into_iter()
                .map(|label| WaitingOption { label })
                .collect(),
        })
        .collect();
    items.extend(derived.into_iter().filter(|one| !dismissed.contains(&one.id)));
    items
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
    /// What the session waits on now: its agent's list with Fleet's own merged in, off what
    /// [`waiting_reads`] took, so decoding a thread's rows happens after the store lock is let go.
    pub(crate) fn waiting_from(
        &self,
        reads: WaitingReads,
        session: &KeptSession,
        attachments: &[ipc::Attachment],
    ) -> Vec<WaitingItem> {
        let WaitingReads { kept, rows, dismissed } = reads;
        let asked = self.hosts().of(&session.id).state().asked.clone();
        let rows: Vec<SessionRow> = rows
            .iter()
            .filter_map(|body| ipc::decode::<SessionRow>("a session row", body.as_bytes()).ok())
            .collect();
        let approved: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                SessionRow::Message {
                    from: SessionVoice::You,
                    text,
                    ..
                } => text.strip_prefix("Approved: ").map(|url| url.trim().to_string()),
                _ => None,
            })
            .collect();
        let spoke = rows.iter().filter_map(|row| match row {
            SessionRow::Message {
                from: SessionVoice::You,
                at,
                ..
            } => Some(at),
            _ => None,
        });
        let last_spoke = spoke.max();
        let newest = attachments
            .iter()
            .filter(|one| {
                one.kind == "artifact"
                    && one.detail.get("form").map(String::as_str) == Some("window")
                    && one.state == ipc::AttachmentState::Standing
            })
            .max_by(|a, b| a.since.cmp(&b.since));
        let windows: Vec<Window> = newest
            .filter(|one| !approved.contains(&one.target))
            .filter(|one| last_spoke.map_or(true, |at| at <= &one.since))
            .map(|one| Window {
                url: one.target.clone(),
                title: one
                    .detail
                    .get("title")
                    .cloned()
                    .unwrap_or_else(|| one.target.clone()),
                since: one.since.as_str().to_string(),
            })
            .into_iter()
            .collect();
        merged(
            kept,
            asked.as_ref(),
            &windows,
            &dismissed,
            session.state == store::SessionState::Ended,
        )
    }

    /// `waiting_for`: replace the agent's list. An item keeps the `since` its id first had.
    pub(crate) async fn set_waiting(
        &self,
        caller: Option<api::Caller>,
        set: SetWaitingFor,
    ) -> Result<SessionRecord, Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| set.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a session to set it for"))?;
        let mut seen = std::collections::HashSet::new();
        for item in &set.items {
            if item.id.trim().is_empty() || item.text.trim().is_empty() {
                return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "every item needs an id and its text"));
            }
            if !seen.insert(item.id.trim().to_string()) {
                return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "two items share an id"));
            }
        }
        let now = self.clock().now().as_str().to_string();
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a session Fleet knows"))?;
            let before = store.waiting_for(&id).map_err(|why| self.ledger_fault(why))?;
            let items: Vec<KeptWaiting> = set
                .items
                .into_iter()
                .map(|one| {
                    let item_id = one.id.trim().to_string();
                    KeptWaiting {
                        since: before
                            .iter()
                            .find(|old| old.item_id == item_id)
                            .map(|old| old.since.clone())
                            .unwrap_or_else(|| now.clone()),
                        item_id,
                        text: one.text.trim().to_string(),
                        act: one
                            .act
                            .map(|act| (act_kind_text(act.kind).to_string(), act.target)),
                        options: one.options.into_iter().map(|o| o.label).collect(),
                    }
                })
                .collect();
            store
                .replace_waiting_for(&id, &items)
                .map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// `answer_waiting`: route the answer by where the item came from.
    pub(crate) async fn settle_waiting(
        self: Arc<Self>,
        said: AnswerWaiting,
    ) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().to_string();
        let terminal = self.terminal_session(&id).await?.is_some();
        if terminal {
            self.terminal_ask_standing(&id).await;
        }
        // The lock is held for the reads and let go before anything is drawn.
        let (session, reads) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a session Fleet knows"))?;
            let reads = self.ledger_reads(&store, &session)?;
            (session, reads)
        };
        let item = self
            .ledger_waiting(&session, reads)
            .into_iter()
            .find(|one| one.id == said.item_id)
        .ok_or_else(|| {
            Refusal::IllegalMove(WireError::raised(
                SESSION_WAITING_UNHELD,
                "nothing is waiting under that item. It was settled already, or the session stopped waiting",
                self.run_id(),
            ))
        })?;
        let nothing = said.choice.is_none()
            && said.mode.is_none()
            && said.text.as_deref().map_or(true, |t| t.trim().is_empty());
        if nothing && item.source != WaitingSource::Walk {
            return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "an answer needs a choice, words or a mode"));
        }
        let words = said.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
        let picked = match said.choice {
            Some(at) => Some(
                item.options
                    .get(at as usize)
                    .map(|one| one.label.clone())
                    .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_EMPTY, "that item has no such choice"))?,
            ),
            None => None,
        };
        let line = said.mode.map(mode_line);
        let message = |body: String| SendSessionMessage {
            session_id: said.session_id.clone(),
            text: body,
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        match item.source {
            WaitingSource::AskCard | WaitingSource::Permission => {
                let asked = self
                    .hosts()
                    .of(&id)
                    .state()
                    .asked
                    .clone()
                    .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNHELD, "no card is open"))?;
                let answer = if item.source == WaitingSource::AskCard {
                    match (picked.as_deref().or(words), asked.questions.first()) {
                        (Some(chosen), Some(first)) => AnswerSessionAsk {
                            session_id: said.session_id.clone(),
                            call: asked.call.clone(),
                            answer: HelmCallAnswer::AllowOnce,
                            note: None,
                            answers: vec![QuestionAnswer {
                                question: first.question.clone(),
                                chosen: vec![chosen.to_string()],
                            }],
                        },
                        _ => self.refusal_of(&said.session_id, &asked, line, None),
                    }
                } else {
                    match said.choice.and_then(|at| asked.offers.get(at as usize)) {
                        Some(offer) => AnswerSessionAsk {
                            session_id: said.session_id.clone(),
                            call: asked.call.clone(),
                            answer: offer.clone(),
                            note: None,
                            answers: Vec::new(),
                        },
                        None => self.refusal_of(&said.session_id, &asked, line, words),
                    }
                };
                self.answer_session_ask(answer).await
            }
            WaitingSource::Walk => {
                let target = item.act.map(|act| act.target).unwrap_or_default();
                let body = if words.is_none() && line.is_none() {
                    format!("Approved: {target}")
                } else {
                    reply(&item.text, picked.as_deref().or(words), line)
                };
                self.send_known(terminal.then_some(session), message(body)).await
            }
            WaitingSource::Agent => {
                let body = reply(&item.text, picked.as_deref().or(words), line);
                self.send_known(terminal.then_some(session), message(body)).await
            }
        }
    }

    /// `dismiss_waiting`: drop one item for good. Nothing is sent to the agent.
    pub(crate) async fn dismiss_waited(&self, dismiss: DismissWaiting) -> Result<SessionRecord, Refusal> {
        let id = dismiss.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            self.terminal_ask_standing(&id).await;
        }
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a session Fleet knows"))?;
            let held = self
                .ledger_row(&store, &session)?
                .waiting_for
                .iter()
                .any(|one| one.id == dismiss.item_id);
            if !held {
                return Err(Refusal::IllegalMove(WireError::raised(
                    SESSION_WAITING_UNHELD,
                    "nothing is waiting under that item. It was settled already, or the session stopped waiting",
                    self.run_id(),
                )));
            }
            store
                .dismiss_waiting(&id, &dismiss.item_id)
                .map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// Skip a card, with what the person said or the mode line as the reason the agent reads.
    fn refusal_of(
        &self,
        session_id: &ipc::SessionId,
        asked: &HelmCallInFlight,
        line: Option<&str>,
        words: Option<&str>,
    ) -> AnswerSessionAsk {
        AnswerSessionAsk {
            session_id: session_id.clone(),
            call: asked.call.clone(),
            answer: HelmCallAnswer::Refuse,
            note: words.or(line).map(str::to_string),
            answers: Vec::new(),
        }
    }
}

/// The message an answer to an agent item is: the item, the answer, and the mode line.
fn reply(item: &str, answer: Option<&str>, line: Option<&str>) -> String {
    let mut text = format!("Re: {item}");
    if let Some(answer) = answer {
        text.push('\n');
        text.push_str(answer);
    }
    if let Some(line) = line {
        text.push('\n');
        text.push_str(line);
    }
    text
}
