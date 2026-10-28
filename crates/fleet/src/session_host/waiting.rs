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
//! **An agent item that asks him to walk is refused, and one kept is dropped.** The window is the
//! one record of a walk, so the agent's item is a call with nothing behind it before `show_window`
//! and a second one after, which outlives his Approve. `walk_asks.rs` reads it.
//!
//! **So is one that repeats a question that lapsed**, nobody answered it in time or it was kept for a walk:
//! it is already in his Needs you from the `AskUserQuestion` call and his answer comes as a message.
//! `lapsed_asks.rs` reads it, and a read leaves off one kept before the rule.
//!
//! **So is one that repeats the plan he has not approved**, which `plan_asks.rs` reads with the model.
//!
//! **And one he could not act on**, once `clarity::IN_FORCE`: a status, a pointer, or no verb aimed at him.
//!
//! **A question asked in the walk window waits until answered**, read off its row as `window_ask:<row>`.
//!
//! **A dismissed id never comes back**, save a walk the job shows again, which `show_window` lifts.
//! `dismiss_waiting` keeps the id; a derived item with it is
//! left off every read and an agent item with it is dropped from the kept list. **Removing an agent
//! item tells the agent** (`Owner removed "<item>"`, and his reason after it). **So does putting away a
//! walk or a pull request it asked about** (`Owner dismissed "<item>"`), so it does not show or ask
//! again for want of knowing. Any other derived item is only dropped.
//!
//! **A reply is a wait only where it asks.** The turn ending on a message that only reports makes the
//! Session read as `replied` (an unread mark under Idle) and raises no item. One whose close asks him
//! something raises `replied:<turn ended at>`, unless the agent already asked through a tool; dismissing
//! it tells the agent nothing. A done raises an item too, and stays.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AllDone, AnswerSessionAsk, AnswerWaiting, ConfirmWaiting, DismissWaiting, HelmCallAnswer, HelmCallInFlight, Instant,
    QuestionAnswer, SendSessionMessage, SessionRecord, SessionRow, SessionVoice, SetWaitingFor,
    DraftHow, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource, WireError,
};
use store::{KeptSession, KeptWaiting, Store};

use crate::daemon::Fleet;
use crate::session_host::asking_in_window::WINDOW_ASK_PREFIX;
use crate::session_host::drafting::{self, DRAFT_PREFIX, FIXED_NOT_A_DRAFT};
use crate::session_host::clarity::{first_unclear, unclear_item_refusal, IN_FORCE as CLARITY_IN_FORCE};
use crate::session_host::handoff_answers::lapsed_question;
use crate::session_host::lapsed_asks::{lapsed_item_refusal, repeats_lapsed};
use crate::session_host::walk_asks::{asks_to_walk, walk_item_refusal};
use crate::session_host::FIRST_MESSAGE_ITEM;

const SESSION_WAITING_UNNAMED: &str = "fleet.session_waiting_unnamed";
const SESSION_WAITING_EMPTY: &str = "fleet.session_waiting_empty";
const SESSION_WAITING_UNHELD: &str = "fleet.session_waiting_unheld";
const SESSION_WAITING_WALK: &str = "fleet.session_waiting_walk";
const SESSION_WAITING_ASKED: &str = "fleet.session_waiting_asked";
const SESSION_WAITING_UNCLEAR: &str = "fleet.session_waiting_unclear";

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
    /// The last Fleet tell it was sent whose refusal is read.
    tell: Option<store::KeptTell>,
    /// The version, recording time and walk of the plan the person has not approved.
    plan: Option<(u32, String, Option<String>)>,
    /// What the Session's judges raised, waiting and settled.
    concerns: Vec<store::KeptConcern>,
    /// Every sentence read from a reply that he marked Not a question, in any Session.
    misreads: Vec<String>,
    /// Every sentence read from a reply that he confirmed with thumbs up, in any Session.
    confirmed: Vec<String>,
    /// The questions about where taught words open that were put to this Session.
    word_asks: Vec<store::KeptWordAsk>,
    /// The notices a Fleet rule raised as calls on this Session.
    calls: Vec<store::KeptCall>,
    /// The Job waits on its start card.
    start_held: bool,
    /// The Job is paused, and raises nothing until it is resumed.
    paused: bool,
    /// The project tasks it finished and nobody has said are done.
    asked_tasks: Vec<store::KeptAsked>,
    /// Its watches that ended not ours.
    not_ours: Vec<store::KeptWatchWhole>,
    /// Every workflow change that is ready now: a proposal waits only while its change is among them.
    ready_changes: Vec<i64>,
}

/// What [`Fleet::waiting_from`] read: the items, and how the Session stands.
pub(crate) struct Waited {
    pub items: Vec<WaitingItem>,
    /// The concerns the person has settled, which wait on nothing.
    pub settled: Vec<WaitingItem>,
    pub standing: ipc::SessionStanding,
    /// The windows holding back the plan he has not approved.
    pub plan_held: Vec<ipc::PlanHold>,
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
        tell: store.tell_of(&session.id)?,
        plan: store
            .session_plan(&session.id)?
            .filter(|plan| plan.approved_at.is_none() && plan.set_aside_at.is_none())
            .map(|plan| (plan.version, plan.recorded_at, plan.walk)),
        concerns: store.session_concerns(&session.id)?,
        misreads: store.misreads()?,
        confirmed: store.confirmed()?,
        word_asks: store.word_asks_of(&session.id)?,
        calls: store.calls_of(&session.id)?,
        start_held: store.hosting(&session.id)?.is_some_and(|one| one.start_held),
        paused: store.hosting(&session.id)?.is_some_and(|one| one.paused_at.is_some()),
        asked_tasks: store.asked_project_tasks(&session.id)?,
        not_ours: store.ended_watches_in(&session.id, core_model::WatchState::NotOurs.as_wire())?,
        ready_changes: store.ready_workflow_change_ids()?,
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
        HelmCallAnswer::AllowLikeThisForSession => "Allow calls like this for this job",
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
            confirmed: false,
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
        confirmed: false,
    }
}

/// The agent's items, then Fleet's: the open card, then each walk window. **Where an agent item
/// names the same act and target as one of Fleet's, Fleet's stands** and the agent's is left off,
/// and so is one that asks him to walk (`walk_asks.rs`), kept before `waiting_for` refused it.
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
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    }));
    let same = |item: &WaitingItem, act: &Option<(String, String)>| {
        matches!((&item.act, act), (Some(a), Some((kind, target)))
                if act_kind_text(a.kind) == kind && &a.target == target)
    };
    let mut items: Vec<WaitingItem> = kept
        .into_iter()
        .filter(|one| {
            !dismissed.contains(&one.item_id)
                && !asks_to_walk(&one.item_id, &one.text, one.act.as_ref().and_then(|(kind, _)| act_kind_of(kind)))
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
            confirmed: false,
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
        let WaitingReads { kept, rows, dismissed, done, turn_end, tell, plan, concerns, misreads, confirmed, word_asks, calls, start_held, paused, asked_tasks, not_ours, ready_changes } = reads;
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
        let ended = session.state == store::SessionState::Ended;
        let holds = plan.as_ref().map(|(_, recorded, walk)| {
            super::planning::plan_holds(recorded, walk.as_deref(), attachments, &rows, &dismissed)
        });
        let mut items = merged(kept, asked.as_ref(), &windows, &dismissed, ended);
        if let Some(lapsed) = lapsed_question(&rows) {
            items.retain(|one| {
                let labels: Vec<&str> = one.options.iter().map(|option| option.label.as_str()).collect();
                one.source != WaitingSource::Agent || !repeats_lapsed(&one.text, &labels, &lapsed)
            });
        }
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
                    confirmed: false,
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
        // Nothing until the model is loaded and has read the reply, which publishes the Session again.
        let refused = |said: &str| {
            let model = self.language_model().model()?;
            let read = self.refusal_reads().read_of(super::tell_refusal::read, model, &session.id, said)?;
            super::tell_refusal::refusal(&read, said)
        };
        // Each sentence of the close, the same way: nothing until the model has read the reply.
        let extracted = |said: &str| {
            let model = self.language_model().model()?;
            let read = self.extract_reads().read_of(super::extracting::read, model, &session.id, said)?;
            Some(if windows.is_empty() { read } else { read.beside_a_walk() })
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
            asks: &|said| {
                let line = self.reply_reads().asking_line(super::asks_model::SHIPPED, self.language_model().model(), &session.id, said);
                // A walk already waiting on him asked for his approval; a line that only says what follows it asks nothing more.
                line.filter(|line| windows.is_empty() || !super::asks::only_approving(line))
            },
            extracted: super::extracting::IN_FORCE.then_some(&extracted),
            // Only the first reply after the tell is read for a refusal of it.
            tell: tell
                .as_ref()
                .filter(|tell| super::tell_refusal::IN_FORCE && tell.first_end.as_deref() == turn_end.as_ref().map(|(at, _)| at.as_str()))
                .map(|tell| super::standing::Told { at: tell.at.as_str(), refused: &refused }),
        };
        // A terminal's own prompt holds the turn on the person, as an open card does.
        let seen = super::standing::Seen { held: seen.held || prompt.is_some(), ..seen };
        let standing = if ended || paused {
            ipc::SessionStanding::Idle
        } else {
            super::standing::standing(&seen, self.hosts().stuck_after())
        };
        if !ended {
            // A Job on its start card waits for the person to send its first message, until Start.
            let id = format!("{FIRST_MESSAGE_ITEM}{}", session.id);
            if start_held && !dismissed.contains(&id) {
                items.push(WaitingItem {
                    id,
                    text: String::from("First message"),
                    since: ipc::Instant::carried(&session.started_at),
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
                    confirmed: false,
                });
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
                plan.as_ref()
                    .zip(holds.as_ref())
                    .and_then(|((version, ..), holds)| super::planning::plan_called(&session.id, *version, holds))
                    .filter(|item| !dismissed.contains(&item.id)),
            );
            items.extend(
                concerns
                    .iter()
                    .filter(|one| one.settled.is_none())
                    .map(|one| crate::rail_keeping::concern_item(&session.id, one)),
            );
            // Taught words that open nowhere Fleet could find in this repository: asked once.
            items.extend(self.words_ask_items(&word_asks, &dismissed));
            items.extend(asked_tasks.iter().map(|asked| crate::project_holds::task_done_item(asked)));
            // Notices a Fleet mod's rule raised as calls, until he dismisses each.
            items.extend(calls.iter().filter(|one| !dismissed.contains(&one.id)).map(super::fleet_calls::call_item));
            // A watch that ended not ours, until he ignores it or asks for a fix.
            items.extend(not_ours.iter().filter_map(crate::watches::not_ours_item).filter(|one| !dismissed.contains(&one.id)));
            items.extend(rows.into_iter().filter_map(|row| match row {
                SessionRow::Delivery { at, choice: None, .. } => Some(super::delivering::delivery_item(at)),
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
                        confirmed: false,
                    })
                }
                SessionRow::Draft { id, at, title, settled: None, change, .. } => {
                    let id = format!("{DRAFT_PREFIX}{id}");
                    let open = change.map_or(true, |one| ready_changes.contains(&one));
                    let mut options = vec![WaitingOption { label: String::from("Approve"), description: None }];
                    if change.is_some() {
                        options.push(WaitingOption { label: String::from("Dismiss"), description: None });
                    }
                    (open && !dismissed.contains(&id)).then(|| WaitingItem {
                        act: Some(WaitingAct {
                            kind: WaitingActKind::Answer,
                            target: id.clone(),
                        }),
                        id,
                        text: title,
                        since: at,
                        source: WaitingSource::Draft,
                        options,
                        steps: Vec::new(),
                        place: None,
                        concern: None,
                        cites: Vec::new(),
                        confirmed: false,
                    })
                }
                _ => None,
            }));
            // The done and a reply that asks are read off the turn, so a Session whose agent never
            // called a tool still reaches the person. Not when it already asked through one, while
            // the sentence readers are off: in force they read the reply whatever is listed, and a
            // repeat of a listed item is folded below.
            let called = !super::extracting::IN_FORCE && items.iter().any(|one| {
                matches!(
                    one.source,
                    WaitingSource::Agent
                        | WaitingSource::AskCard
                        | WaitingSource::Walk
                        | WaitingSource::Permission
                        | WaitingSource::WindowAsk
                        | WaitingSource::Draft
                )
            });
            items.extend(super::standing::derived(&super::standing::Seen { called, ..seen }));
            // A sentence read from the reply near one he marked Not a question is left off, by the
            // vectors the fold below reads, once the model is loaded.
            if !misreads.is_empty() && items.iter().any(|one| one.source == WaitingSource::Replied) {
                if let Some(model) = self.language_model().model() {
                    let texts: Vec<&str> = items.iter().map(|one| one.text.as_str()).chain(misreads.iter().map(String::as_str)).collect();
                    let known = self.item_vectors().known(model, &session.id, &texts);
                    items = super::dedup::left_off_as_misread(items, &misreads, |text| known.get(text).cloned(), super::dedup::FOLDS_AT);
                }
            }
            // Items that are one ask by more than one door are one row, once the model is loaded.
            // An item it has not embedded yet is kept this read; `standing_tick` publishes the Session
            // again once it is.
            if super::dedup::IN_FORCE && items.len() > 1 {
                if let Some(model) = self.language_model().model() {
                    let texts: Vec<&str> = items.iter().map(|one| one.text.as_str()).collect();
                    let known = self.item_vectors().known(model, &session.id, &texts);
                    items = super::dedup::folded(items, |text| known.get(text).cloned(), super::dedup::FOLDS_AT);
                }
            }
        }
        // A sentence he confirmed reads so wherever a reply says it again.
        for one in items.iter_mut().filter(|one| one.source == WaitingSource::Replied) {
            one.confirmed = confirmed.contains(&one.text);
        }
        let settled = concerns
            .iter()
            .filter(|one| one.settled.is_some())
            .map(|one| crate::rail_keeping::concern_item(&session.id, one))
            .collect();
        // **A paused Job raises nothing**: its items come back after Resume, if they are still owed.
        let items = if paused { Vec::new() } else { items };
        Waited { items, settled, standing, plan_held: holds.map(|holds| holds.held_by).unwrap_or_default() }
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
            if asks_to_walk(&item.id, &item.text, item.act.as_ref().map(|act| act.kind)) {
                return Err(self.hosted_refusal(SESSION_WAITING_WALK, &walk_item_refusal(item.id.trim())));
            }
        }
        if let Some(lapsed) = self.rows_of(&id).await.ok().and_then(|rows| lapsed_question(&rows)) {
            for item in &set.items {
                let labels: Vec<&str> = item.options.iter().map(|option| option.label.as_str()).collect();
                if repeats_lapsed(&item.text, &labels, &lapsed) {
                    return Err(self.hosted_refusal(SESSION_WAITING_ASKED, &lapsed_item_refusal(item.id.trim())));
                }
            }
        }
        let unclear = set.items.iter().map(|item| {
            let runs = item.act.as_ref().is_some_and(|act| act.kind == WaitingActKind::Run && !act.target.trim().is_empty());
            (item.id.trim(), item.text.trim(), runs)
        });
        if let Some((item, text, why)) = first_unclear(unclear, CLARITY_IN_FORCE) {
            return Err(self.hosted_refusal(SESSION_WAITING_UNCLEAR, &unclear_item_refusal(item, text, why)));
        }
        let said = set.items.iter().map(|item| (item.id.trim().to_string(), item.text.trim().to_string())).collect();
        self.plan_repeated(&id, said).await?;
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
        let asked;
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            if session.state == store::SessionState::Ended {
                return Err(self.closed(&id));
            }
            self.later_holds_done(&store, &id)?;
            store
                .keep_done(&id, &now)
                .map_err(|why| self.ledger_fault(why))?;
            // Done is one of the two ways a held task is asked about: the other is the merge.
            asked = store.ask_held_project_tasks(&id, &now).map_err(|why| self.ledger_fault(why))? > 0;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        if asked {
            self.projects_published().await;
        }
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
        if said.fixed.is_some() && item.source != WaitingSource::Draft {
            return Err(self.hosted_refusal(FIXED_NOT_A_DRAFT, "fixed text is an answer to a draft only"));
        }
        let fixed = said.fixed.as_deref().map(str::trim).filter(|t| !t.is_empty());
        let nothing = said.choice.is_none()
            && said.mode.is_none()
            && fixed.is_none()
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
        let says = match (item.source, fixed, said.choice, words) {
            (WaitingSource::Draft, Some(_), ..) => "Approved with fixes".to_string(),
            (WaitingSource::Draft, None, Some(_), _) => "Approved".to_string(),
            (WaitingSource::Draft, None, None, Some(note)) => format!("Sent back: {note}"),
            _ => match (picked.as_deref().or(words), said.mode) {
            (Some(chosen), _) => format!("Answered: {chosen}"),
            (None, Some(ipc::WaitingMode::Best)) => "Answered: think it through".to_string(),
            (None, Some(ipc::WaitingMode::Quick)) => "Answered: get it done".to_string(),
            (None, None) => "Approved".to_string(),
            },
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
                            kind: None,
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
                            kind: asked.kind.clone(),
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
            // A call a Fleet rule raised is the same: his words are a message, and dismissing is what settles it.
            WaitingSource::Done | WaitingSource::FleetCall => {
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
                "the first message is settled by Start, from the Job's message box",
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
            WaitingSource::TaskDone => self.task_done_chosen(&id, &item.id, said.choice).await,
            // Fleet raised it, so Ignore tells the agent nothing; Fix it, or his words, are a message.
            WaitingSource::WatchNotOurs => {
                let watch = item.act.as_ref().map(|act| act.target.clone()).unwrap_or_default();
                let body = match said.choice {
                    Some(crate::watches::IGNORE) => words.map(|words| reply(&item.text, Some(&format!("Ignore\n{words}")), line)),
                    Some(crate::watches::FIX_IT) => {
                        let kept = self.store().lock().await.watch_whole(&watch).map_err(|why| self.ledger_fault(why))?;
                        let mut fix = kept.as_ref().map(crate::watches::fix_it).ok_or_else(|| self.settled_already())?;
                        for more in [words, line].into_iter().flatten() {
                            fix.push('\n');
                            fix.push_str(more);
                        }
                        Some(fix)
                    }
                    Some(_) => return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "that item has no such choice")),
                    None => Some(reply(&item.text, words, line)),
                };
                if let Some(body) = body {
                    Arc::clone(&self).send_known(terminal.then_some(session), message(body)).await?;
                }
                self.drop_answered(&id, &item.id).await
            }
            WaitingSource::Delivery => self.delivery_chosen(&id, said.choice).await,
            WaitingSource::WordsAddress => self.words_address_chosen(&id, &item, picked.as_deref(), words).await,
            // Settled only once the message is sent, so a failed send leaves the draft standing.
            WaitingSource::Draft => {
                let row = item.id.strip_prefix(DRAFT_PREFIX).unwrap_or_default();
                // A proposed workflow: Approve saves it and Dismiss drops it, before the Job is told.
                if let (Some(change), Some(choice)) = (self.draft_change(&id, row).await, said.choice) {
                    let change = ipc::WorkflowChangeRef { change };
                    let (body, how) = if choice == 0 {
                        self.workflow_change_approved(change).await?;
                        (drafting::approved(&item.text, None), DraftHow::Approved)
                    } else {
                        self.workflow_change_dismissed(change).await?;
                        (drafting::withdrawn(&item.text, None), DraftHow::Withdrawn)
                    };
                    Arc::clone(&self).send_known(terminal.then_some(session), message(body)).await?;
                    return self.draft_settled(&id, row, how, None).await;
                }
                let (body, how, kept) = match (fixed, said.choice, words) {
                    (Some(fixed), ..) => (drafting::fixed(&item.text, fixed), DraftHow::Fixed, Some(fixed)),
                    (None, Some(_), words) => (drafting::approved(&item.text, words), DraftHow::Approved, words),
                    (None, None, Some(note)) => (drafting::sent_back(&item.text, note), DraftHow::SentBack, Some(note)),
                    (None, None, None) => {
                        return Err(self.hosted_refusal(
                            SESSION_WAITING_EMPTY,
                            "a draft is approved, approved with fixed text, or sent back with a note",
                        ))
                    }
                };
                let body = [Some(body.as_str()), line].into_iter().flatten().collect::<Vec<_>>().join("\n");
                Arc::clone(&self).send_known(terminal.then_some(session), message(body)).await?;
                let row = item.id.strip_prefix(DRAFT_PREFIX).unwrap_or_default();
                self.draft_settled(&id, row, how, kept.map(str::to_string)).await
            }
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
    /// the agent does not state it again for want of knowing. **A draft is withdrawn**: the session is
    /// sent `Withdrawn: <title>`, not to propose it again, and the row is settled `withdrawn` with the
    /// reason. **A walk, or a sentence read from the agent's reply, is told too**: the session is sent
    /// `Owner dismissed "<item text>"`. Any other item Fleet derived is only dropped.
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
        if item.source == WaitingSource::Delivery {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "how the work is delivered is answered, not dismissed",
            ));
        }
        if item.source == WaitingSource::SlotChoice {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "whether a job waits for a slot or has one added is answered, not dismissed",
            ));
        }
        if item.source == WaitingSource::TaskDone {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "whether the tasks are done is answered, not dismissed",
            ));
        }
        if dismiss.misread && item.source != WaitingSource::Replied {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "only a question read from a reply is marked as not a question",
            ));
        }
        let _settling = self
            .hosts()
            .settling(&id, &item.id)
            .ok_or_else(|| self.settled_already())?;
        // The agent is told first, so a send that fails leaves the item standing to try again. "Not yet"
        // is the plan put away to keep talking.
        let told = match item.source {
            WaitingSource::Agent => Some(removed(&item.text, dismiss.reason.as_deref())),
            WaitingSource::Plan => Some(super::planning::NOT_YET_TOLD.to_string()),
            WaitingSource::Draft => Some(drafting::withdrawn(&item.text, dismiss.reason.as_deref())),
            WaitingSource::Walk | WaitingSource::Replied if !dismiss.misread => Some(dismissed_by_owner(&item.text)),
            _ => None,
        };
        if let Some(text) = told {
            let sent = SendSessionMessage {
                session_id: dismiss.session_id.clone(),
                text,
                attachments: Vec::new(),
                mentions: Vec::new(),
            };
            Arc::clone(&self).send_known(terminal, sent).await?;
        }
        if item.source == WaitingSource::Plan {
            self.plan_set_aside(&id, &item.id).await?;
        }
        // Settled on its row, as an answer settles it, so the thread says it was withdrawn.
        if item.source == WaitingSource::Draft {
            let row = item.id.strip_prefix(DRAFT_PREFIX).unwrap_or_default();
            if let Some(change) = self.draft_change(&id, row).await {
                // Already settled elsewhere is the same end, so that refusal is not the person's.
                let _ = self.workflow_change_dismissed(ipc::WorkflowChangeRef { change }).await;
            }
            let reason = dismiss.reason.as_deref().map(str::trim).filter(|why| !why.is_empty());
            return self.draft_settled(&id, row, DraftHow::Withdrawn, reason.map(str::to_string)).await;
        }
        // Not a question: the sentence is kept for good, so a near one is not raised again and the
        // trainer reads it as asking nothing.
        if dismiss.misread {
            let now = self.clock().now().as_str().to_string();
            self.store().lock().await.keep_misread(&id, &item.text, &now).map_err(|why| self.ledger_fault(why))?;
        }
        self.drop_answered(&id, &item.id).await
    }

    /// `confirm_waiting`: thumbs up on a question read from a reply. Its sentence is kept for good as one
    /// that asks, the item stays, and every read from then on carries it as confirmed. The agent is told
    /// nothing: the item was never its own.
    pub(crate) async fn confirm_waited(
        self: Arc<Self>,
        confirm: ConfirmWaiting,
    ) -> Result<SessionRecord, Refusal> {
        let id = confirm.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            self.terminal_ask_standing(&id).await;
        }
        let item = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            self.ledger_row(&store, &session)?
                .waiting_for
                .into_iter()
                .find(|one| one.id == confirm.item_id)
                .ok_or_else(|| self.settled_already())?
        };
        if item.source != WaitingSource::Replied {
            return Err(self.hosted_refusal(
                SESSION_WAITING_EMPTY,
                "only a question read from a reply is confirmed as one",
            ));
        }
        let _settling = self
            .hosts()
            .settling(&id, &item.id)
            .ok_or_else(|| self.settled_already())?;
        let now = self.clock().now().as_str().to_string();
        let record = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            store.keep_confirmed(&id, &item.text, &now).map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
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
    /// has none to settle. Whether any was taken off. **An item with steps stays**: its command is one
    /// step of several, and Bridge ticks that step where the run was pressed.
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
                        && one.steps.is_empty()
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
            kind: None,
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

/// What the agent is told when the person puts away a walk or a question read from its reply.
fn dismissed_by_owner(item: &str) -> String {
    format!("Owner dismissed \"{item}\"")
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
