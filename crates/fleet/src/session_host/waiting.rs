//! What a session waits on the person for: the agent's own list, kept whole, with
//! what Fleet already knows merged in on every read, and the one route that
//! settles an item. `docs/concepts/session.md`.
//!
//! **Derived items are never kept.** An open card, a permission or a walk window
//! is read off the session as it stands, so it cannot go stale.
//!
//! **Every walk window he has not approved since it was last shown waits.** It stops once he
//! approves it or any take of it, once a newer window shows one of its pages (on any port, since a
//! dev server started again takes another), or once its page cannot load (`window_failed` spends
//! it, and showing it again brings it back). A newer window of another page, a message, a note or
//! closing the window settles nothing: that is his call. A page changed after it was shown waits
//! again only once it is shown again.
//!
//! **An agent item that points at a page the session showed is dropped.** The window is the one
//! record of that page, whether it still waits or not, so the agent's `walk` item for the same
//! address (compared without a trailing slash) is never a second call, and does not return once
//! the window is approved.
//!
//! **A question asked in the walk window waits until answered**, read off its row as `window_ask:<row>`.
//!
//! **A dismissed id never comes back.** `dismiss_waiting` keeps the id; a derived item with it is
//! left off every read and an agent item with it is dropped from the kept list. **Removing an agent
//! item tells the agent** (`Owner removed "<item>"`, and his reason after it); a derived item is only
//! dropped, since the agent never stated it.
//!
//! **A reply is a wait only where it asks.** The turn ending on a message that only reports makes the
//! Session read as `replied` (an unread mark under Idle) and raises no item. One whose close asks him
//! something raises `replied:<turn ended at>`, unless the agent already asked through a tool; dismissing
//! it tells the agent nothing. A done raises an item too, and stays.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AllDone, AnswerSessionAsk, AnswerWaiting, DismissWaiting, HelmCallAnswer, HelmCallInFlight, Instant,
    QuestionAnswer, SendSessionMessage, SessionRecord, SessionRow, SessionVoice, SetWaitingFor,
    WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource, WireError,
};
use store::{KeptSession, KeptWaiting, Store};

use crate::daemon::Fleet;
use crate::session_host::asking_in_window::WINDOW_ASK_PREFIX;
use crate::session_host::UNSENT_ITEM;

const SESSION_WAITING_UNNAMED: &str = "fleet.session_waiting_unnamed";
const SESSION_WAITING_EMPTY: &str = "fleet.session_waiting_empty";
const SESSION_WAITING_UNHELD: &str = "fleet.session_waiting_unheld";

/// The last line of a message answering an agent item: it need not say the item again. It says the
/// item is settled outright, since a bare "took this off" read as a removal to undo, and agents
/// stated the item again. Bridge leaves the line off the person's row, so keep it in step there.
pub(crate) const TAKEN_OFF: &str = "This settles the item, so Fleet took it off your waiting list. Do not add it back.";

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
    /// When the agent said it was done, if that still stands.
    pub(crate) done: Option<String>,
    /// How the last turn ended while nothing has followed it: when, and the end of what the agent said.
    turn_end: Option<(String, String)>,
    /// The version, recording time and walk of the plan the person has not approved.
    plan: Option<(u32, String, Option<String>)>,
    /// What the Session's judges raised, waiting and settled.
    concerns: Vec<store::KeptConcern>,
}

/// What [`Fleet::waiting_from`] read: the items, and how the Session stands.
pub(crate) struct Waited {
    pub items: Vec<WaitingItem>,
    /// The concerns the person has settled, which wait on nothing.
    pub settled: Vec<WaitingItem>,
    pub standing: ipc::SessionStanding,
}

pub(crate) fn waiting_reads(
    store: &Store,
    session: &KeptSession,
) -> Result<WaitingReads, store::WriteError> {
    Ok(WaitingReads {
        kept: store.waiting_for(&session.id)?,
        rows: store.session_rows(&session.id)?,
        dismissed: store.dismissed_waiting(&session.id)?,
        done: store.done_since(&session.id)?,
        turn_end: store.turn_end(&session.id)?,
        plan: store
            .session_plan(&session.id)?
            .filter(|plan| plan.approved_at.is_none())
            .map(|plan| (plan.version, plan.recorded_at, plan.walk)),
        concerns: store.session_concerns(&session.id)?,
    })
}

fn offer_label(offer: &HelmCallAnswer) -> &'static str {
    match offer {
        HelmCallAnswer::AllowOnce => "Allow once",
        HelmCallAnswer::AllowAndRemember => "Allow and remember",
        HelmCallAnswer::Refuse => "Refuse",
        HelmCallAnswer::AlwaysAllow => "Always allow",
        HelmCallAnswer::AlwaysBlock => "Always block",
        HelmCallAnswer::AllowForSession => "Always allow for this job",
    }
}

fn act_kind_text(kind: WaitingActKind) -> &'static str {
    match kind {
        WaitingActKind::Walk => "walk",
        WaitingActKind::Answer => "answer",
        WaitingActKind::ApprovePr => "approve_pr",
        WaitingActKind::Run => "run",
        WaitingActKind::Message => "message",
    }
}

fn act_kind_of(text: &str) -> Option<WaitingActKind> {
    match text {
        "walk" => Some(WaitingActKind::Walk),
        "answer" => Some(WaitingActKind::Answer),
        "approve_pr" => Some(WaitingActKind::ApprovePr),
        "run" => Some(WaitingActKind::Run),
        "message" => Some(WaitingActKind::Message),
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
                        description: None,
                    })
                    .collect()
            } else {
                Vec::new()
            },
            steps: Vec::new(),
            place: None,
            concern: None,
            cites: Vec::new(),
        };
    }
    WaitingItem {
        id: format!("perm:{}", asked.call),
        text: format!("{} {}", asked.tool, asked.detail)
            .trim()
            .to_string(),
        since: asked.asked_at.clone(),
        source: WaitingSource::Permission,
        act,
        options: asked
            .offers
            .iter()
            .map(|one| WaitingOption {
                label: offer_label(one).to_string(),
                description: None,
            })
            .collect(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
    }
}

/// Two addresses of one page: equal once a trailing slash is let go.
fn same_page(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

/// The agent's items, then Fleet's: the open card, then each walk window. **Where an agent item
/// names the same act and target as one of Fleet's, Fleet's stands** and the agent's is left off,
/// and so is a `walk` item for any address in `shown`, the pages this session has shown in a window.
pub(crate) fn merged(
    kept: Vec<KeptWaiting>,
    asked: Option<&HelmCallInFlight>,
    windows: &[Window],
    shown: &[&str],
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
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
    }));
    let same = |item: &WaitingItem, act: &Option<(String, String)>| {
        matches!((&item.act, act), (Some(a), Some((kind, target)))
                if act_kind_text(a.kind) == kind && &a.target == target)
    };
    let mut items: Vec<WaitingItem> = kept
        .into_iter()
        .filter(|one| {
            !dismissed.contains(&one.item_id)
                && !matches!(&one.act, Some((kind, target))
                    if kind == "walk" && shown.iter().any(|url| same_page(url, target)))
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
                .map(|label| WaitingOption {
                    label,
                    description: None,
                })
                .collect(),
            steps: one.steps,
            place: None,
            concern: None,
            cites: Vec::new(),
        })
        .collect();
    items.extend(
        derived
            .into_iter()
            .filter(|one| !dismissed.contains(&one.id)),
    );
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
    ) -> Waited {
        let WaitingReads { kept, rows, dismissed, done, turn_end, plan, concerns } = reads;
        let asked = self.hosts().of(&session.id).state().asked.clone();
        let rows: Vec<SessionRow> = rows
            .iter()
            .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
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
        let spoke_at = last_spoke.map(|at| at.as_str().to_string());
        // Every window he has not approved since it was last shown waits, until he approves it or a
        // newer window shows the same page. A message or a note does not settle it. One changed after
        // it was shown waits again only when the Job shows it again, however long it has been ready.
        let windows: Vec<Window> = super::walking::walk_windows(attachments)
            .filter(|one| !super::walking::is_working(one) && !super::walking::is_changed(one))
            .filter(|one| super::walking::walk_approval(&rows, &one.target).is_none())
            .filter(|one| !super::walking::replaced(&rows, &one.target))
            .map(|one| Window {
                url: one.target.clone(),
                title: one
                    .detail
                    .get("title")
                    .cloned()
                    .unwrap_or_else(|| one.target.clone()),
                since: one.changed_at.as_str().max(one.since.as_str()).to_string(),
            })
            .collect();
        let shown: Vec<&str> = attachments
            .iter()
            .filter(|one| {
                one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("window")
            })
            .map(|one| one.target.as_str())
            .collect();
        let ended = session.state == store::SessionState::Ended;
        let mut items = merged(kept, asked.as_ref(), &windows, &shown, &dismissed, ended);
        // The harness's own permission prompt in a terminal. Only the terminal can answer it, so its
        // one act opens the Session and it offers nothing to choose.
        let prompt = (session.origin != "bridge" && asked.is_none() && !ended)
            .then(|| self.hosts().terminals().prompt_of(&session.id))
            .flatten();
        if let Some(open) = &prompt {
            let id = format!("perm:{}", open.since);
            if !dismissed.contains(&id) {
                items.push(WaitingItem {
                    id,
                    text: format!("{} {}", open.tool, open.detail).trim().to_string(),
                    since: Instant::carried(&open.since),
                    source: WaitingSource::Permission,
                    act: Some(WaitingAct {
                        kind: WaitingActKind::Message,
                        target: session.id.clone(),
                    }),
                    options: Vec::new(),
                    steps: Vec::new(),
                    place: None,
                    concern: None,
                    cites: Vec::new(),
                });
            }
        }
        // What Fleet saw of the turn: running or not, and how the last one ended.
        let (helm, running) = {
            let runtime = self.hosts().of(&session.id);
            let state = runtime.state();
            let running = if session.origin == "bridge" {
                matches!(state.turn, ipc::SessionTurn::Working { .. }).then(|| state.last_active.elapsed())
            } else {
                self.hosts().terminals().quiet_for(&session.id)
            };
            (state.helm, running)
        };
        let seen = super::standing::Seen {
            helm,
            running,
            subagents: attachments
                .iter()
                .any(|one| one.kind == "subagent" && one.state == ipc::AttachmentState::Standing),
            held: asked.is_some(),
            called: false,
            turn_end: turn_end.as_ref().map(|(at, said)| (at.as_str(), said.as_str())),
            spoke: spoke_at.as_deref(),
            done: done.as_deref(),
            dismissed: &dismissed,
        };
        // A terminal's own prompt holds the turn on the person, as an open card does.
        let seen = super::standing::Seen { held: seen.held || prompt.is_some(), ..seen };
        let standing = if ended {
            ipc::SessionStanding::Idle
        } else {
            super::standing::standing(&seen, self.hosts().stuck_after())
        };
        if !ended {
            // The first message another Session's agent wrote for the person, until he sends it.
            let id = format!("{UNSENT_ITEM}{}", session.id);
            if !dismissed.contains(&id) {
                items.extend(
                    attachments
                        .iter()
                        .filter(|one| one.kind == "unsent" && one.state == ipc::AttachmentState::Standing)
                        .map(|one| WaitingItem {
                            id: id.clone(),
                            text: String::from("First message"),
                            since: one.since.clone(),
                            source: WaitingSource::FirstMessage,
                            act: Some(WaitingAct {
                                kind: WaitingActKind::Message,
                                target: session.id.clone(),
                            }),
                            options: Vec::new(),
                            steps: Vec::new(),
                            place: None,
                            concern: None,
                            cites: Vec::new(),
                        }),
                );
            }
            // Every slot is held and it is in the line for one: Wait or Add, until he says.
            items.extend(
                self.hosts()
                    .slot_queue()
                    .pending(&session.id)
                    .map(super::queueing::slot_choice_item),
            );
            // A plan he has not approved is one of the Session's calls; nothing waits on it.
            items.extend(
                super::planning::plan_called(&session.id, plan, attachments, &rows, &dismissed)
                    .filter(|item| !dismissed.contains(&item.id)),
            );
            items.extend(
                concerns
                    .iter()
                    .filter(|one| one.settled.is_none())
                    .map(|one| crate::rail_keeping::concern_item(&session.id, one)),
            );
            items.extend(rows.into_iter().filter_map(|row| match row {
                SessionRow::WindowAsk {
                    id,
                    at,
                    question,
                    options,
                    place,
                    window,
                    answer: None,
                } => {
                    let id = format!("{WINDOW_ASK_PREFIX}{id}");
                    (!dismissed.contains(&id)).then_some(WaitingItem {
                        id,
                        text: question,
                        since: at,
                        source: WaitingSource::WindowAsk,
                        act: Some(WaitingAct {
                            kind: WaitingActKind::Walk,
                            target: window,
                        }),
                        options,
                        steps: Vec::new(),
                        place: Some(place),
                        concern: None,
                        cites: Vec::new(),
                    })
                }
                _ => None,
            }));
            // The done and a reply that asks are read off the turn, so a Session whose agent never
            // called a tool still reaches the person. Not when it already asked through one.
            let called = items.iter().any(|one| {
                matches!(
                    one.source,
                    WaitingSource::Agent
                        | WaitingSource::AskCard
                        | WaitingSource::Walk
                        | WaitingSource::Permission
                        | WaitingSource::WindowAsk
                )
            });
            items.extend(super::standing::derived(&super::standing::Seen { called, ..seen }));
        }
        let settled = concerns
            .iter()
            .filter(|one| one.settled.is_some())
            .map(|one| crate::rail_keeping::concern_item(&session.id, one))
            .collect();
        Waited { items, settled, standing }
    }

    /// `waiting_for`: replace the agent's list. An item keeps the `since` its id first had.
    pub(crate) async fn set_waiting(
        &self,
        caller: Option<api::Caller>,
        set: SetWaitingFor,
    ) -> Result<SessionRecord, Refusal> {
        let id = self.waiting_session(caller, set.session_id.as_ref())?;
        let mut seen = std::collections::HashSet::new();
        for item in &set.items {
            if item.id.trim().is_empty() || item.text.trim().is_empty() {
                return Err(self
                    .hosted_refusal(SESSION_WAITING_EMPTY, "every item needs an id and its text"));
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
                .ok_or_else(|| {
                    self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows")
                })?;
            let before = store
                .waiting_for(&id)
                .map_err(|why| self.ledger_fault(why))?;
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
                        steps: one
                            .steps
                            .into_iter()
                            .map(|line| line.trim().to_string())
                            .filter(|line| !line.is_empty())
                            .collect(),
                    }
                })
                .collect();
            store
                .replace_waiting_for(&id, &items)
                .map_err(|why| self.ledger_fault(why))?;
            // Something is owed again, so nothing is left to work on.
            if !items.is_empty() {
                store.clear_done(&id).map_err(|why| self.ledger_fault(why))?;
            }
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// The session a call is about: the hosted one holding the connection, else the named one.
    pub(super) fn waiting_session(
        &self,
        caller: Option<api::Caller>,
        named: Option<&ipc::SessionId>,
    ) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job to set it for"))
    }

    /// `all_done`: the agent has nothing left to work on. A second call keeps the first instant.
    pub(crate) async fn say_done(
        &self,
        caller: Option<api::Caller>,
        done: AllDone,
    ) -> Result<SessionRecord, Refusal> {
        let id = self.waiting_session(caller, done.session_id.as_ref())?;
        let now = self.clock().now().as_str().to_string();
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            if session.state == store::SessionState::Ended {
                return Err(self.closed(&id));
            }
            store
                .keep_done(&id, &now)
                .map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// The agent's word that it was done no longer stands. Whether it had stood.
    pub(crate) async fn done_cleared(&self, id: &str) -> bool {
        self.store().lock().await.clear_done(id).unwrap_or(false)
    }

    /// A turn ended at `at`, on `said` where the agent said something to the person: its end is kept, so
    /// the Session reads as having replied, and as asking where the close asks. A turn that ended on tools alone leaves what stood as it was: the turn that
    /// began it already forgot the reply before.
    pub(crate) async fn turn_end_kept(&self, id: &str, at: &str, said: Option<&str>) {
        let tail = said.map(super::standing::tail).filter(|tail| !tail.is_empty());
        if let Some(tail) = tail {
            let _ = self.store().lock().await.keep_turn_end(id, at, &tail);
        }
    }

    /// The reply the last turn left no longer stands: he wrote, or another turn began. Whether it had.
    pub(crate) async fn turn_end_cleared(&self, id: &str) -> bool {
        self.store().lock().await.clear_turn_end(id).unwrap_or(false)
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
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            let reads = self.ledger_reads(&store, &session)?;
            (session, reads)
        };
        let item = self
            .ledger_waiting(&session, reads)
            .into_iter()
            .find(|one| one.id == said.item_id)
        .ok_or_else(|| self.settled_already())?;
        let _settling = self
            .hosts()
            .settling(&id, &item.id)
            .ok_or_else(|| self.settled_already())?;
        let nothing = said.choice.is_none()
            && said.mode.is_none()
            && said.text.as_deref().map_or(true, |t| t.trim().is_empty());
        if nothing && item.source != WaitingSource::Walk {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "an answer needs a choice, words or a mode",
            ));
        }
        let words = said
            .text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty());
        let picked = match said.choice {
            Some(at) => Some(
                item.options
                    .get(at as usize)
                    .map(|one| one.label.clone())
                    .ok_or_else(|| {
                        self.hosted_refusal(SESSION_WAITING_EMPTY, "that item has no such choice")
                    })?,
            ),
            None => None,
        };
        let line = said.mode.map(mode_line);
        // A pick with words says both: the words go with the pick, never in place of it.
        let both = match (picked.as_deref(), words) {
            (Some(pick), Some(words)) => Some(format!("{pick}\n{words}")),
            (Some(one), None) | (None, Some(one)) => Some(one.to_string()),
            (None, None) => None,
        };
        let message = |body: String| SendSessionMessage {
            session_id: said.session_id.clone(),
            text: body,
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        // The card routes answer through `answer_session_ask`, which enters its own; the rest are
        // a message to the agent, entered here once it is sent.
        let (source, fleet) = (item.source, Arc::clone(&self));
        let says = match (picked.as_deref().or(words), said.mode) {
            (Some(chosen), _) => format!("Answered: {chosen}"),
            (None, Some(ipc::WaitingMode::Best)) => "Answered: think it through".to_string(),
            (None, Some(ipc::WaitingMode::Quick)) => "Answered: get it done".to_string(),
            (None, None) => "Approved".to_string(),
        };
        let settled = match item.source {
            WaitingSource::AskCard | WaitingSource::Permission => {
                let asked = self.hosts().of(&id).state().asked.clone().ok_or_else(|| {
                    let why = if self.hosts().terminals().prompt_of(&id).is_some() {
                        "the terminal's own permission prompt is answered in the terminal"
                    } else {
                        "no card is open"
                    };
                    self.hosted_refusal(SESSION_WAITING_UNHELD, why)
                })?;
                let answer = if item.source == WaitingSource::AskCard {
                    match (picked.as_deref().or(words), asked.questions.first()) {
                        (Some(chosen), Some(first)) => AnswerSessionAsk {
                            session_id: said.session_id.clone(),
                            call: asked.call.clone(),
                            answer: HelmCallAnswer::AllowOnce,
                            note: picked.as_ref().and(words).map(str::to_string),
                            answers: vec![QuestionAnswer {
                                question: first.question.clone(),
                                chosen: vec![chosen.to_string()],
                            }],
                            folder: None,
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
                            // A word from Needs you carries no picker: an "always" is for the path itself.
                            folder: asked.reaches.clone(),
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
                    reply(&item.text, both.as_deref(), line)
                };
                self.send_known(terminal.then_some(session), message(body)).await
            }
            WaitingSource::WindowAsk => {
                let answer = [both.as_deref(), line].into_iter().flatten().collect::<Vec<_>>().join("\n");
                if let Some(row) = item.id.strip_prefix(WINDOW_ASK_PREFIX) {
                    self.window_ask_answered(&id, row, answer.clone()).await;
                }
                let body = format!("Re: {}\n{answer}", item.text);
                self.send_known(terminal.then_some(session), message(body)).await
            }
            WaitingSource::Agent => {
                let mut body = reply(&item.text, both.as_deref(), line);
                body.push('\n');
                body.push_str(TAKEN_OFF);
                Arc::clone(&self).send_known(terminal.then_some(session), message(body)).await?;
                // Delivered, so the item goes now: a failed send above leaves it standing.
                self.drop_answered(&id, &item.id).await
            }
            // Derived from the turn and `done_since`, never kept: the person's words are a message to
            // the Session, which is what makes either stop standing, so there is nothing to drop.
            WaitingSource::Done => {
                let body = both.clone().unwrap_or_default();
                self.send_known(terminal.then_some(session), message(body)).await
            }
            // The same, and a mode alone is an answer to a question.
            WaitingSource::Replied => {
                let body = [both.as_deref(), line].into_iter().flatten().collect::<Vec<_>>().join("\n");
                self.send_known(terminal.then_some(session), message(body)).await
            }
            WaitingSource::FirstMessage => Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "the first message is settled by sending it, from the Job's message box",
            )),
            WaitingSource::Plan => Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "a plan is approved, or sent changes, from its own card",
            )),
            WaitingSource::Judge => Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "a concern is sent to the agent, or dismissed, from its own card",
            )),
            WaitingSource::SlotChoice => self.slot_chosen(&id, said.choice).await,
        };
        if let (Ok(record), false) = (
            &settled,
            matches!(source, WaitingSource::AskCard | WaitingSource::Permission),
        ) {
            fleet.session_answered(record, says);
        }
        settled
    }

    /// `dismiss_waiting`: drop one item for good. **An item the agent stated is told so**: the session
    /// is sent `Owner removed "<item text>"`, with `: <reason>` after it where the person gave one, so
    /// the agent does not state it again for want of knowing. An item Fleet derived is only dropped.
    /// The message goes first, so a send that fails leaves the item standing to try again.
    pub(crate) async fn dismiss_waited(
        self: Arc<Self>,
        dismiss: DismissWaiting,
    ) -> Result<SessionRecord, Refusal> {
        let id = dismiss.session_id.as_str().to_string();
        let terminal = self.terminal_session(&id).await?;
        if terminal.is_some() {
            self.terminal_ask_standing(&id).await;
        }
        let item = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows")
                })?;
            self.ledger_row(&store, &session)?
                .waiting_for
                .into_iter()
                .find(|one| one.id == dismiss.item_id)
                .ok_or_else(|| self.settled_already())?
        };
        if item.source == WaitingSource::Judge {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "a concern is dismissed from its own card, which keeps it on the record as settled",
            ));
        }
        if item.source == WaitingSource::SlotChoice {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "whether a job waits for a slot or has one added is answered, not dismissed",
            ));
        }
        let _settling = self
            .hosts()
            .settling(&id, &item.id)
            .ok_or_else(|| self.settled_already())?;
        if item.source == WaitingSource::Agent {
            let sent = SendSessionMessage {
                session_id: dismiss.session_id.clone(),
                text: removed(&item.text, dismiss.reason.as_deref()),
                attachments: Vec::new(),
                mentions: Vec::new(),
            };
            Arc::clone(&self).send_known(terminal, sent).await?;
        }
        self.drop_answered(&id, &item.id).await
    }

    /// An item no longer waiting, or one another answer or removal is being delivered for.
    pub(crate) fn settled_already(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SESSION_WAITING_UNHELD,
            "nothing is waiting under that item. It was settled already, or the job stopped waiting",
            self.run_id(),
        ))
    }

    /// Take an agent item off the list for good and tell Bridge, once the person's word on it has been
    /// delivered. It does not refuse an item the agent has since restated away.
    async fn drop_answered(&self, id: &str, item_id: &str) -> Result<SessionRecord, Refusal> {
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            store
                .dismiss_waiting(id, item_id)
                .map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// A run the person made ended clean: take off the agent's `run` items for that command, so Bridge
    /// has none to settle. Whether any was taken off.
    pub(crate) async fn run_item_settled(&self, id: &str, command: &str) -> bool {
        let record = {
            let mut store = self.store().lock().await;
            let Ok(Some(session)) = store.session(id) else {
                return false;
            };
            let Ok(kept) = store.waiting_for(id) else {
                return false;
            };
            let items: Vec<String> = kept
                .into_iter()
                .filter(|one| {
                    matches!(&one.act, Some((kind, target))
                        if kind == "run" && target.trim() == command.trim())
                })
                .map(|one| one.item_id)
                .collect();
            if items.is_empty() || items.iter().any(|one| store.dismiss_waiting(id, one).is_err()) {
                return false;
            }
            let Ok(record) = self.ledger_row(&store, &session) else {
                return false;
            };
            record
        };
        self.publish(ipc::Event::SessionChanged(record));
        true
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
            folder: None,
        }
    }
}

/// What the agent is told when the person removes an item it stated: the item, and his reason where
/// he gave one.
fn removed(item: &str, reason: Option<&str>) -> String {
    match reason.map(str::trim).filter(|why| !why.is_empty()) {
        Some(why) => format!("Owner removed \"{item}\": {why}"),
        None => format!("Owner removed \"{item}\""),
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

/// The line that tells the agent to settle an item itself.
fn mode_line(mode: ipc::WaitingMode) -> &'static str {
    match mode {
        ipc::WaitingMode::Best => "Decide this yourself. Think it through and go with the best option.",
        ipc::WaitingMode::Quick => "Just get it done. Take the reasonable path, finish the work and land it. Skip the formalities.",
    }
}
