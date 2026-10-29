//! The session ledger: what a harness says of an agent session, kept against
//! the session and published whole. `docs/concepts/session.md`.
//!
//! **Fleet resolves what the sender cannot.** A harness knows its directory and
//! not which repository Fleet serves or which pool slot that is; `placed` reads
//! both off the path, so a harness reports a place and never a slot number it
//! would have to know the pool's layout to spell.
//!
//! **A fact that repeats what the ledger holds changes nothing and publishes
//! nothing**, so a harness may report on every turn without a client redrawing
//! on every one.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    ManifestId, Owners, SessionFact, SessionList, SessionRecord, SessionReport, SessionState,
    WireError,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptSession, SessionSearch, Store};

use crate::adrift::Adrift;
use crate::build_facts::workers_of;
use crate::build_keeping::{build_heard, build_read, BuildReads, Forecasts};
use crate::build_record::Observed;
use crate::session_host::waiting::{waiting_reads, WaitingReads};
use crate::session_issues::ISSUE;
use crate::daemon::Fleet;
use crate::session_host::asking_in_window::origin_of;
use crate::slot_holders::{pool_backs, stand_in_slot};
use crate::timeline::{Doer, FileAct};

/// What a session's row reads from the store, taken under the lock.
pub(crate) struct LedgerReads {
    held: Vec<KeptAttachment>,
    hosting: Option<store::KeptHosting>,
    waiting: WaitingReads,
    /// When the limit the session hit resets, while it is held for one.
    limited: Option<String>,
    /// The breakdown of the context window, as the wire spells it.
    context: Option<String>,
    /// The latest Loadout list and the latest turn's uses.
    loadout: crate::loadout_keeping::LoadoutReads,
    /// What the build record is derived from.
    build: BuildReads,
    /// The plan the Session recorded, and the files it and its subagents touched.
    plan: Option<store::KeptPlan>,
    /// The latest adversarial review of the plan, whatever version it read.
    adversarial: Option<store::KeptAdversarialReview>,
    touches: Vec<store::KeptTouch>,
    /// The pull requests the plan may land in: the Session's own and those of the Jobs its groups were handed to.
    pulls: Vec<crate::plan_record::PlanPull>,
    /// What the workflow rail and the judge's last reading are derived from.
    rail: crate::rail_keeping::RailReads,
    /// How the Job was started, and the Job that started it.
    started: Option<ipc::SessionStarted>,
    /// The wait of a Session still waiting on another, and the title of the one it waits on.
    wait: Option<(store::KeptWait, Option<String>, Option<String>)>,
    /// The follow-ups its agent keeps for itself, still open.
    later: Vec<store::KeptLater>,
    /// The name of the live watch that started first, if it holds one.
    watching: Option<String>,
    /// What the start card is drawn from, for a Job that waits on it.
    start: Option<crate::session_host::start_card::StartReads>,
}

/// A report that names nothing it can be kept against. A 422.
const SESSION_UNNAMED: &str = "fleet.session_unnamed";

/// A hosted agent's subagent, by its attachment's target: started by a call, with the attachment's detail, or settled.
enum SubagentRow {
    Started(String, String, std::collections::BTreeMap<String, String>),
    Settled(String, ipc::ToolSubagentState),
}

mod ledger;
mod shown;
mod wire;

pub(crate) use ledger::{placed, Placed};
pub(crate) use shown::steps_shown;
pub(crate) use wire::mod_is_older;
use shown::{origins_named, shows_a_page, sign_in_named, takes_shown, ONE_AT_A_TIME};
use wire::{
    attachment_state, attachment_wire, held_wire, mod_version_in, origin_text, session_state, session_wire,
    wire_instant,
};

impl<H, V, W> Sessions for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn report_session(&self, report: SessionReport) -> Result<SessionRecord, Refusal> {
        let id = report.session_id.as_str().trim().to_string();
        if id.is_empty() {
            return Err(self.ledger_unnamed("a job id"));
        }
        if report.harness.trim().is_empty() {
            return Err(self.ledger_unnamed("the harness's name"));
        }
        let now = self.clock().now().as_str().to_string();
        let roots = self.ledger_roots();
        let holder = Holder::session(&id);
        let fault = |why| self.ledger_fault(why);

        let mut store = self.store().lock().await;
        let kept = store.session(&id).map_err(fault)?;
        let mut changed = kept.is_none();
        let mut ended = false;
        let mut parking: Option<(String, u32)> = None;
        let mut plan_moved: Option<ipc::PlanUsage> = None;
        // A terminal turn's end and what it said, read for a claim once the store is let go.
        let mut replied: Option<(String, String)> = None;
        // A terminal turn that failed on a sign-in that ran out, or one that finished, which is the account working.
        let (mut signed_out, mut answered) = (false, false);
        // A browser reached for outside Bridge, written on the thread once the store is let go.
        let mut reminder: Option<(String, Option<String>)> = None;
        // A hosted agent's subagent started or settled, drawn on its call's row once the store is let go.
        let mut subagent_row: Option<SubagentRow> = None;
        let mut session = kept.unwrap_or_else(|| KeptSession {
            id: id.clone(),
            harness: report.harness.clone(),
            origin: "terminal".into(),
            manifest_id: None,
            cwd: String::new(),
            title: None,
            state: store::SessionState::Live,
            started_at: now.clone(),
            last_seen_at: now.clone(),
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            figures: Default::default(),
            mod_version: None,
        });
        let manifest_of = |session: &KeptSession| session.manifest_id.clone().unwrap_or_default();
        // A session in a terminal is read from what its mod tells: anything said while a turn runs
        // is that turn being heard from. A hosted one is heard from its own process.
        let terminal = session.origin != "bridge";
        // What may move the plan's marks, after which a terminal session's mod is handed what Fleet marked.
        let marks = terminal
            && matches!(
                report.fact,
                SessionFact::Tasks { .. } | SessionFact::Committed { .. } | SessionFact::Touched { .. } | SessionFact::Calls { .. }
            );
        // A hosted session's agent hears them at its next call.
        let owed = !terminal && matches!(report.fact, SessionFact::Tasks { .. } | SessionFact::Committed { .. });
        if terminal && !matches!(report.fact, SessionFact::TurnStarted | SessionFact::TurnCompleted { .. } | SessionFact::Ended { .. }) {
            self.hosts().terminals().turn_heard(&id);
        }

        match report.fact {
            SessionFact::Started {
                cwd,
                title,
                origin,
                mod_version,
            } => {
                // Started is always a change, so whether the version moved is not asked.
                Self::mod_said(&mut session, mod_version);
                let place = placed(&cwd, &roots);
                // **A session Bridge hosts stays Bridge's** when the mod in it
                // reports its start, so it is one row and not two.
                if session.origin != "bridge" {
                    session.origin = origin_text(origin).into();
                }
                session.state = store::SessionState::Live;
                session.ended_at = None;
                session.end_reason = None;
                session.cwd = cwd;
                session.manifest_id = place.manifest.clone();
                if let Some(title) = title.filter(|_| session.title.is_none()) {
                    session.title = Some(title);
                }
                stand_in_slot(&mut store, &holder, &place, &roots, &now).map_err(fault)?;
                changed = true;
            }
            SessionFact::Moved { cwd } => {
                if session.cwd != cwd {
                    let place = placed(&cwd, &roots);
                    session.cwd = cwd;
                    session.manifest_id = place.manifest.clone();
                    stand_in_slot(&mut store, &holder, &place, &roots, &now).map_err(fault)?;
                    changed = true;
                }
            }
            // A name a person gave wins; the first prompt's line only fills a gap.
            SessionFact::Titled { title, named } => {
                if named || session.title.is_none() {
                    changed |= session.title.as_deref() != Some(title.as_str());
                    session.title = Some(title);
                }
            }
            // An issue the agent filed: `session_issues` says which it keeps, and keeps what the forge said.
            SessionFact::Attached { attachment } if attachment.kind == ISSUE => {
                changed |= self.issue_filed(&mut store, &holder, &manifest_of(&session), &attachment, &now).map_err(fault)?;
            }
            SessionFact::Attached { attachment } => {
                if attachment.kind.trim().is_empty() || attachment.target.trim().is_empty() {
                    return Err(self.ledger_unnamed("a kind and a target on an attachment"));
                }
                // A branch or pull request read from a slot the lease is another's is theirs, but a
                // pull request the session opened itself is its own wherever it stood: the mod read
                // its address off `gh pr create`, so no directory is being trusted.
                let opened = attachment.kind == "pr"
                    && attachment.detail.get("created").is_some_and(|one| one == "true");
                let backed = opened
                    || !matches!(attachment.kind.as_str(), "branch" | "pr")
                    || pool_backs(&store, &session, &roots);
                if !terminal && attachment.kind == "subagent" {
                    let call = attachment.detail.get("call").unwrap_or(&attachment.target).clone();
                    subagent_row = Some(SubagentRow::Started(attachment.target.clone(), call, attachment.detail.clone()));
                }
                if backed {
                    changed |= store
                        .attach(
                            &KeptAttachment {
                                holder: holder.clone(),
                                kind: attachment.kind.clone(),
                                manifest_id: manifest_of(&session),
                                target: attachment.target,
                                state: AttachmentState::Standing,
                                detail: attachment.detail,
                                since: now.clone(),
                                changed_at: now.clone(),
                            },
                            ONE_AT_A_TIME.contains(&attachment.kind.as_str()),
                        )
                        .map_err(fault)?;
                }
            }
            SessionFact::Settled { attachment, state }
                if attachment.kind == "subagent" && state == ipc::AttachmentState::Spent =>
            {
                changed |= store
                    .subagent_stopped(&holder, &manifest_of(&session), &attachment.target, &now)
                    .map_err(fault)?;
                changed |= self.activity_subagent_over(&id, &attachment.target);
                if !terminal {
                    subagent_row = Some(SubagentRow::Settled(attachment.target.clone(), ipc::ToolSubagentState::Finished));
                }
            }
            SessionFact::Settled { attachment, state } => {
                if !terminal && attachment.kind == "subagent" && state == ipc::AttachmentState::GivenBack {
                    subagent_row = Some(SubagentRow::Settled(attachment.target.clone(), ipc::ToolSubagentState::CutOff));
                }
                changed |= store
                    .settle(
                        &holder,
                        &attachment.kind,
                        &manifest_of(&session),
                        &attachment.target,
                        attachment_state(state),
                        &now,
                    )
                    .map_err(fault)?;
            }
            SessionFact::Measured { usage, plan, context_breakdown } => {
                plan_moved = self.hosts().plan_read(plan, &now);
                if let Some(kept) = plan_moved.as_ref().and_then(|usage| ipc::encode(usage).ok()) {
                    store.keep_plan_reading(&kept).map_err(fault)?;
                }
                if let Some(breakdown) = context_breakdown {
                    if let Ok(kept) = ipc::encode(&breakdown) {
                        changed |= store.keep_session_context(&id, &kept).map_err(fault)?;
                    }
                }
                let figures = store::SessionFigures {
                    context_tokens: usage.context_tokens.or(session.figures.context_tokens),
                    context_window: usage.context_window.or(session.figures.context_window),
                    cost_micros: usage.cost_micros.or(session.figures.cost_micros),
                };
                changed |= figures != session.figures;
                session.figures = figures;
            }
            SessionFact::TurnMeasured { turn } => {
                // Usage is read by its own operation and draws nothing on the record, so it
                // publishes no `session.changed`.
                crate::turn_usage_keeping::kept(&mut store, &id, &turn).map_err(fault)?;
            }
            SessionFact::Equipped { loadout } => {
                changed |= crate::loadout_keeping::equipped(&mut store, &id, &loadout).map_err(fault)?;
            }
            SessionFact::Used { used } => {
                changed |= crate::loadout_keeping::used(&mut store, &id, &used).map_err(fault)?;
            }
            SessionFact::Tuned {
                model,
                effort,
                mode,
                commands,
                mod_version,
            } => {
                changed |= Self::mod_said(&mut session, mod_version);
                changed |= self.hosts().terminals().tuned(
                    &id,
                    ipc::TerminalFacts {
                        model,
                        effort,
                        mode,
                        commands,
                        listening: false,
                        asked: None,
                    },
                );
            }
            SessionFact::TurnStarted => {
                // A turn begun again is work the agent had said was over, and ends the reply the turn
                // before left. A hosted session's own process says so, so only a terminal's is read here.
                if terminal {
                    self.hosts().terminals().turn_began(&id);
                    store.clear_done(&id).map_err(fault)?;
                    store.clear_turn_end(&id).map_err(fault)?;
                }
                changed = true;
            }
            SessionFact::TurnCompleted { said } => {
                session.last_turn_at = Some(now.clone());
                if terminal {
                    self.hosts().terminals().turn_over(&id);
                    self.timeline().close_bursts_of(&Doer::of_session(&session));
                    if let Some(tail) = said.as_deref().map(crate::session_host::standing::tail).filter(|tail| !tail.is_empty()) {
                        store.keep_turn_end(&id, &now, &tail).map_err(fault)?;
                    }
                    replied = said.map(|said| (now.clone(), said));
                    answered = true;
                }
                // A turn taken since the limit was hit: the wake is not owed.
                store.clear_session_limit(&id).map_err(fault)?;
                changed = true;
                // Whatever it was changing for the person is done for now, unless a subagent is still on it.
                let home = &self.host().home;
                let ended = |agent: &str| adapters::terminal_thread::subagent_ended(home, &id, agent);
                crate::session_host::readiness::turn_ended(&mut store, &holder, &now, ended).map_err(fault)?;
            }
            SessionFact::Wrote => {
                changed |= crate::session_host::readiness::windows_set(&mut store, &holder, &now, true).map_err(fault)?;
            }
            SessionFact::BrowserOutside { tool, address } => {
                let tool = tool.trim().to_string();
                if tool.is_empty() {
                    return Err(self.ledger_unnamed("the tool on a browser_outside"));
                }
                reminder = Some((tool, address.as_deref().and_then(shown::address_shown)));
            }
            // The harness's own prompt, waiting on a person at the terminal. A hosted session's is
            // Fleet's own ask, so only a terminal's is read here.
            SessionFact::PermissionAsked { tool, detail } if terminal => {
                let open = crate::session_host::PromptOpen { tool, detail, since: now.clone() };
                changed |= self.hosts().terminals().prompt_opened(&id, open);
            }
            SessionFact::PermissionSettled if terminal => {
                changed |= self.hosts().terminals().prompt_closed(&id);
            }
            SessionFact::PermissionAsked { .. } | SessionFact::PermissionSettled => {}
            SessionFact::Limited { resets_at } => {
                changed |= store
                    .keep_session_limit(&id, resets_at.as_str())
                    .map_err(fault)?;
            }
            // A hosted session's own process says so, so only a terminal's is read here.
            SessionFact::SignedOut => signed_out = terminal,
            // For the check of who else writes these files; nothing on the ledger changes.
            SessionFact::Touched { paths, by, act } => {
                self.session_wrote(&store, &session, &roots, &paths);
                // A hosted session is heard from its own process, which enters the same acts.
                if terminal {
                    let file = paths.last().map(|path| self.file_named(Some(&session.cwd), path));
                    self.timeline().touched(Doer::of_session(&session), FileAct::Edit, paths.len(), file);
                    // The Field and the plan: the subagent is named by the type its attachment carries.
                    let by = by.map(|agent| {
                        let held = store.attachments_of(&Holder::session(&id)).unwrap_or_default();
                        let kind = held
                            .iter()
                            .find(|one| one.kind == "subagent" && one.target == agent)
                            .and_then(|one| one.detail.get("type").cloned())
                            .unwrap_or_else(|| String::from("subagent"));
                        (kind, agent)
                    });
                    let act = match act {
                        Some(ipc::TouchAct::Write) => "write",
                        _ => "edit",
                    };
                    // Named from the worktree's root, as a hosted session's are: the Field asks a diff by it.
                    let worktree = self.worktree_of(&session.cwd);
                    let home = &self.host().home;
                    for path in &paths {
                        let Some(path) = crate::file_touching::rooted(&worktree, &session.cwd, home, path) else { continue };
                        changed |= crate::plan_keeping::touch_kept(&mut store, &id, by.clone(), &path, act, &now)
                            .map_err(fault)?;
                    }
                }
                if !paths.is_empty() {
                    changed |= build_heard(&mut store, &id, &Observed::Edited, &now).map_err(fault)?;
                }
            }
            // The build record hears a hosted Session's reads and commands too: its process says the
            // same, and a repeat changes nothing.
            SessionFact::Read { path } => {
                if terminal {
                    let file = self.file_named(Some(&session.cwd), &path);
                    self.timeline().touched(Doer::of_session(&session), FileAct::Read, 1, Some(file));
                }
                changed |= build_heard(&mut store, &id, &Observed::Worked, &now).map_err(fault)?;
            }
            SessionFact::Ran { command } => {
                if terminal {
                    let doer = Doer::of_session(&session);
                    match adapters::command_reach(&command) {
                        Some(reach) => self.timeline().called(doer, reach),
                        None => self.timeline().ran(doer, command),
                    }
                }
                changed |= build_heard(&mut store, &id, &Observed::Worked, &now).map_err(fault)?;
            }
            fact @ (SessionFact::Tasks { .. } | SessionFact::PlanLeft | SessionFact::Committed { .. } | SessionFact::Pushed { .. }) => {
                changed |= self.build_told(&mut store, &session, fact, &now).map_err(fault)?;
            }
            // A hosted session's own calls are counted from its process, so its mod's count is not read.
            SessionFact::Calls { calls } if terminal => {
                crate::plan_keeping::calls_counted(&mut store, &id, calls).map_err(fault)?;
            }
            SessionFact::Calls { .. } => {}
            // Fleet ends a session it hosts, by `close_session`: the harness in
            // it going away is a process ending, and the next message resumes.
            SessionFact::Ended { .. } if session.origin == "bridge" => {}
            SessionFact::Ended { reason } => {
                ended = true;
                self.timeline().close_bursts_of(&Doer::of_session(&session));
                changed |= session.state != store::SessionState::Ended;
                session.state = store::SessionState::Ended;
                session.ended_at = Some(now.clone());
                // Ended from Bridge, it reads as a hosted close does. Either way its slot is
                // parked, unless its process goes on under another id.
                let asked = self.hosts().terminals().was_ended_from_bridge(&id);
                if asked || crate::session_host::ending::parks_on_end(&reason) {
                    parking = store
                        .attachments_of(&holder)
                        .map_err(fault)?
                        .into_iter()
                        .find(|one| one.kind == "slot" && one.state == AttachmentState::Standing)
                        .and_then(|one| Some((one.manifest_id, one.target.parse().ok()?)));
                }
                self.hosts().terminals().turn_over(&id);
                session.end_reason = Some(if asked { String::from("closed") } else { reason });
                store.clear_session_limit(&id).map_err(fault)?;
                store.give_back(&holder, None, &now).map_err(fault)?;
            }
        }

        let marked = if marks { crate::plan_keeping::marks_untold(&mut store, &id).map_err(fault)? } else { None };
        if owed {
            self.hosts().of(&id).state().marks_owed = true;
        }
        changed |= marked.is_some();
        session.last_seen_at = now;
        store.keep_session(&session).map_err(fault)?;
        let record = self.ledger_row(&store, &session)?;
        drop(store);
        if let Some(marked) = marked {
            self.terminal_marks_told(&session.id, marked);
        }
        if let Some((manifest, slot)) = parking {
            self.park_once_gone(&session.id, &manifest, slot);
        }
        if ended {
            // Its terminal prompt went with it, so Bridge must not offer the card.
            self.terminal_ask_closed(&session.id, ipc::SessionAskState::Unanswered).await;
            self.session_wound_up(&session.id).await;
            self.writes().left(&crate::clashing::Who::Session(session.id.clone()));
        }
        if changed {
            self.publish(ipc::Event::SessionChanged(record.clone()));
        }
        if let Some(usage) = plan_moved {
            self.publish(ipc::Event::PlanUsageChanged(usage.clone()));
            self.downshift_read(&usage).await;
        }
        if signed_out {
            self.signed_out_heard(&self.hosts().of(&session.id));
        }
        if answered {
            self.signed_in_by(&session.id);
        }
        match subagent_row {
            Some(SubagentRow::Started(target, call, detail)) => self.subagent_started(&session.id, &target, &call, &detail).await,
            Some(SubagentRow::Settled(target, state)) => self.subagent_settled(&session.id, &target, state).await,
            None => {}
        }
        if let Some((tool, address)) = reminder {
            let row = ipc::SessionRow::BrowserReminder { id: self.row_id(&session.id), at: self.instant(), tool, address };
            self.row_put(&session.id, row).await;
        }
        if let Some((ended, said)) = replied {
            if let Some(claim) = self.unbacked_claim(&session.id, &ended, Some(&said)).await {
                self.terminal_claim_reminded(&session.id, claim).await;
            }
        }
        Ok(record)
    }

    async fn start_another_session(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        start: ipc::StartAnotherSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        self.another_session_started(caller, start).await
    }

    async fn change_waiting(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        change: ipc::ChangeWaiting,
    ) -> Result<ipc::SessionWait, Refusal> {
        self.waiting_changed(caller, change).await
    }

    async fn fork_this_session(
        &self,
        caller: Option<api::Caller>,
        fork: ipc::ForkThisSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        self.this_session_forked(caller, fork).await
    }

    async fn claim_pull_request(
        &self,
        caller: Option<api::Caller>,
        claim: ipc::ClaimPullRequest,
    ) -> Result<ipc::PullRequestClaimed, Refusal> {
        self.pull_request_claimed(caller, claim).await
    }

    async fn claim_issue(
        &self,
        caller: Option<api::Caller>,
        claim: ipc::ClaimIssue,
    ) -> Result<ipc::IssueClaimed, Refusal> {
        self.issue_claimed(caller, claim).await
    }

    async fn take_main_red(
        &self,
        caller: Option<api::Caller>,
        take: ipc::TakeMainRed,
    ) -> Result<ipc::MainRedTaken, Refusal> {
        self.main_red_taken(caller, take).await
    }

    async fn report_session_check(
        &self,
        call: ipc::SessionCheckCall,
    ) -> Result<ipc::SessionCheckAnswer, Refusal> {
        self.session_check_reported(call).await
    }

    async fn ask_for_slot(&self, call: ipc::AskForSlot) -> Result<ipc::SlotAsked, Refusal> {
        Ok(self.slot_asked(call).await)
    }

    async fn get_session_check_output(&self, run: u64) -> Result<ipc::CheckOutput, Refusal> {
        self.session_check_output(run).await
    }

    async fn observe_session_check(&self, run: u64) -> Result<api::SessionCheckLive, Refusal> {
        self.session_check_observed(run).await
    }

    async fn stop_session_check(&self, run: u64, force: bool) -> Result<ipc::SessionCheckAnswer, Refusal> {
        self.session_check_stopped(run, force).await
    }

    async fn show_window(
        &self,
        caller: Option<api::Caller>,
        show: ipc::ShowWindow,
    ) -> Result<SessionRecord, Refusal> {
        let url = show.url.trim().to_string();
        if !shows_a_page(&url) {
            return Err(self.ledger_unnamed("an `http` or `https` address to show"));
        }
        let variations = takes_shown(&url, show.variations).map_err(|what| self.ledger_unnamed(what))?;
        let steps = steps_shown(show.steps).map_err(|what| self.ledger_unnamed(&what))?;
        let origins = origins_named(show.origins).map_err(|what| self.ledger_unnamed(what))?;
        let sign_in = sign_in_named(&url, &origins, show.sign_in.0).map_err(|what| self.ledger_unnamed(what))?;
        let hosted = caller.and_then(|caller| self.session_holding(&caller));
        let id = hosted
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.ledger_unnamed("a job to show it for"))?;
        let harness = {
            let store = self.store().lock().await;
            store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.ledger_unnamed("a job Fleet knows"))?
                .harness
        };
        let title = show
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or(&url)
            .to_string();
        let record = self
            .report_session(SessionReport {
                harness,
                session_id: ipc::SessionId::carried(&id),
                fact: SessionFact::Attached {
                    attachment: ipc::AttachmentReport {
                        kind: String::from("artifact"),
                        target: url.clone(),
                        detail: [
                            ("form".to_string(), "window".to_string()),
                            ("title".to_string(), title.clone()),
                        ]
                        .into_iter()
                        .chain((!variations.is_empty()).then(|| ("variations".to_string(), ipc::WindowTake::as_detail(&variations))))
                        .chain((!steps.is_empty()).then(|| ("steps".to_string(), ipc::WalkStep::as_detail(&steps))))
                        .chain((!origins.is_empty()).then(|| ("origins".to_string(), ipc::origins_as_detail(&origins))))
                        .collect(),
                    },
                },
            })
            .await?;
        self.walk_noted(&id, &record, &title);
        // The row is published below, carrying it.
        let _ = self.live_page_seen(&id, &url, &title);
        // A walk dismissed, by him or by withdraw_window, waits again once the job shows it again.
        {
            let mut store = self.store().lock().await;
            store.restore_waiting(&id, &format!("walk:{url}")).map_err(|why| self.ledger_fault(why))?;
        }
        // Held before the row goes out, which is what Bridge opens the window on and asks for it by.
        self.window_sign_ins().hold(&id, &url, sign_in);
        self.row_put(
            &id,
            ipc::SessionRow::Window {
                id: self.row_id(&id),
                at: self.instant(),
                title,
                url,
                variations,
                steps,
                origins,
                ended_at: None,
            },
        )
        .await;
        // Read again past the row: a window it replaces stops waiting by it.
        let record = {
            let store = self.store().lock().await;
            match store.session(&id).map_err(|why| self.ledger_fault(why))? {
                Some(session) => self.ledger_row(&store, &session)?,
                None => record,
            }
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    async fn take_window_sign_in(&self, take: ipc::TakeWindowSignIn) -> Result<ipc::WindowSignIn, Refusal> {
        let sign_in = self.window_sign_ins().take(take.session_id.as_str().trim(), take.url.trim());
        Ok(ipc::WindowSignIn { sign_in: ipc::SignIn(sign_in) })
    }

    async fn withdraw_window(
        &self,
        caller: Option<api::Caller>,
        withdraw: ipc::WithdrawWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.window_withdrawn(caller, withdraw).await
    }

    async fn show_sketch(
        &self,
        caller: Option<api::Caller>,
        show: ipc::ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        self.sketch_shown(caller, show).await
    }

    async fn answer_sketch(self: Arc<Self>, answer: ipc::AnswerSketch) -> Result<SessionRecord, Refusal> {
        self.sketch_answered(answer).await
    }

    async fn show_file(
        &self,
        caller: Option<api::Caller>,
        show: ipc::ShowFile,
    ) -> Result<SessionRecord, Refusal> {
        self.file_shown(caller, show).await
    }

    async fn get_shown_file(&self, id: ipc::SessionId, path: String) -> Result<api::ShownFile, Refusal> {
        self.shown_file_read(id.as_str(), &path).await
    }

    async fn record_plan(
        &self,
        caller: Option<api::Caller>,
        record: ipc::RecordPlan,
        seen: bool,
    ) -> Result<SessionRecord, Refusal> {
        self.plan_recorded(caller, record, seen).await
    }

    async fn approve_plan(self: Arc<Self>, approve: ipc::ApprovePlan) -> Result<SessionRecord, Refusal> {
        self.plan_approved(approve).await
    }

    async fn send_plan_changes(
        self: Arc<Self>,
        sent: ipc::SendPlanChanges,
    ) -> Result<SessionRecord, Refusal> {
        self.plan_changes_sent(sent).await
    }

    async fn judge_session(self: Arc<Self>, judge: ipc::JudgeSession) -> Result<SessionRecord, Refusal> {
        self.judge_asked(judge).await
    }

    async fn send_concern(self: Arc<Self>, send: ipc::SendConcern) -> Result<SessionRecord, Refusal> {
        self.concern_sent(send).await
    }

    async fn dismiss_concern(self: Arc<Self>, dismiss: ipc::DismissConcern) -> Result<SessionRecord, Refusal> {
        self.concern_dismissed(dismiss).await
    }

    async fn ask_in_window(
        &self,
        caller: Option<api::Caller>,
        ask: ipc::AskInWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.window_asked(caller, ask).await
    }

    async fn propose_draft(
        &self,
        caller: Option<api::Caller>,
        draft: ipc::ProposeDraft,
    ) -> Result<SessionRecord, Refusal> {
        self.draft_proposed(caller, draft).await
    }

    async fn withdraw_draft(
        &self,
        caller: Option<api::Caller>,
        withdraw: ipc::WithdrawDraft,
    ) -> Result<SessionRecord, Refusal> {
        self.draft_withdrawn(caller, withdraw).await
    }

    async fn window_closed(&self, closed: ipc::WindowClosed) -> Result<SessionRecord, Refusal> {
        let id = closed.session_id.as_str().trim().to_string();
        let known = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.is_some()
        };
        if !known {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        self.walk_ended(&id, closed.url.trim()).await;
        let store = self.store().lock().await;
        let session = store
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.ledger_unnamed("a job Fleet knows"))?;
        self.ledger_row(&store, &session)
    }

    async fn window_failed(
        self: Arc<Self>,
        failed: ipc::WindowFailed,
    ) -> Result<ipc::WalkTold, Refusal> {
        let id = failed.session_id.as_str().trim().to_string();
        let harness = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.map(|session| session.harness)
        };
        let Some(harness) = harness else {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        };
        // Nothing answers on its address, so the walk waits on nothing he can do: its window is spent
        // until the job shows it again, which attaches it standing.
        if crate::session_host::walking::could_not_load(&failed.why) {
            let rows = self.rows_of(&id).await?;
            if let Some(url) = crate::session_host::walking::window_of(&rows, failed.url.trim()) {
                self.report_session(SessionReport {
                    harness,
                    session_id: ipc::SessionId::carried(&id),
                    fact: SessionFact::Settled {
                        attachment: ipc::AttachmentNamed { kind: String::from("artifact"), target: url.to_string() },
                        state: ipc::AttachmentState::Spent,
                    },
                })
                .await?;
            }
        }
        // Bridge names a step it cannot play by its place, which is not a page that failed to load,
        // and says `steps set aside` where the page plays its own walk, which wins over the steps.
        let text = if failed.why.trim().starts_with("steps set aside") {
            format!(
                "Your walk window on {} plays the page's own walk, so the steps you passed were set \
                 aside and the person walks the page's. Show it again without steps.",
                failed.url.trim()
            )
        } else if failed.why.trim().starts_with("step ") {
            format!(
                "Your walk window on {} is held at {}. The person cannot walk past it until your \
                 steps match the page. Fix the steps, then call show_window again with them.",
                failed.url.trim(),
                failed.why.trim()
            )
        } else {
            format!(
                "Your walk window could not show {}: {}. The person cannot walk it until it loads. \
                 Fix it, then call show_window again.",
                failed.url.trim(),
                failed.why.trim()
            )
        };
        let told = self.told_by_fleet(&id, &text, ipc::Notice::WalkFailed).await?;
        Ok(ipc::WalkTold { told })
    }

    async fn waiting_for(
        &self,
        caller: Option<api::Caller>,
        set: ipc::SetWaitingFor,
    ) -> Result<SessionRecord, Refusal> {
        self.set_waiting(caller, set).await
    }

    async fn list_closed_waiting(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<ipc::SessionId>,
    ) -> Result<ipc::ClosedWaitingList, Refusal> {
        self.closed_waiting_listed(caller, session_id).await
    }

    async fn all_done(
        &self,
        caller: Option<api::Caller>,
        done: ipc::AllDone,
    ) -> Result<SessionRecord, Refusal> {
        self.say_done(caller, done).await
    }

    async fn defer_later(
        &self,
        caller: Option<api::Caller>,
        defer: ipc::DeferLater,
    ) -> Result<ipc::LaterDeferred, Refusal> {
        self.later_deferred(caller, defer).await
    }

    async fn settle_later(
        &self,
        caller: Option<api::Caller>,
        settle: ipc::SettleLater,
    ) -> Result<ipc::LaterSettled, Refusal> {
        self.later_settled(caller, settle).await
    }

    async fn dispute_fleet(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        dispute: ipc::DisputeFleet,
    ) -> Result<ipc::FleetDisputed, Refusal> {
        self.fleet_disputed(caller, dispute).await
    }

    async fn finish_up(
        &self,
        caller: Option<api::Caller>,
        finish: ipc::FinishUp,
    ) -> Result<ipc::FinishedUp, Refusal> {
        self.finished_up(caller, finish).await
    }

    async fn mark_annotation_done(
        &self,
        caller: Option<api::Caller>,
        mark: ipc::MarkAnnotationDone,
    ) -> Result<ipc::AnnotationMarked, Refusal> {
        self.annotation_done(caller, mark).await
    }

    async fn resolve_pull_request_note(
        &self,
        caller: Option<api::Caller>,
        resolve: ipc::ResolvePullRequestNote,
    ) -> Result<ipc::PullRequestNote, Refusal> {
        self.pull_request_note_resolved(caller, resolve).await
    }

    async fn start_session_run(
        self: Arc<Self>,
        start: ipc::StartSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.begin_session_run(start).await
    }

    async fn run_session_shell(
        self: Arc<Self>,
        run: ipc::RunSessionShell,
    ) -> Result<ipc::SessionShellRan, Refusal> {
        self.shell_action_run(run).await
    }

    async fn stop_session_run(
        &self,
        stop: ipc::StopSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.end_session_run(stop)
    }

    async fn write_session_run(
        &self,
        write: ipc::WriteSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.type_into_session_run(write).await
    }

    async fn resize_session_run(
        &self,
        resize: ipc::ResizeSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.resize_session_run_terminal(resize)
    }

    async fn observe_session_run(&self, run_id: String) -> Result<api::ObservedSessionRun, Refusal> {
        self.watch_session_run(&run_id)
    }

    async fn open_terminal(
        self: Arc<Self>,
        open: ipc::OpenTerminal,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        let run_id = self.open_job_shell(open.session_id.as_str(), open.cols, open.rows).await?;
        Ok(ipc::SessionRunUnderway { run_id })
    }

    async fn start_terminal_run(
        self: Arc<Self>,
        start: ipc::StartTerminalRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.begin_terminal_run(start).await
    }

    async fn type_in_terminal(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        typed: ipc::TypeInTerminal,
    ) -> Result<ipc::TerminalTyped, Refusal> {
        self.typed_in_job_shell(caller, typed).await
    }

    async fn read_terminal(
        &self,
        caller: Option<api::Caller>,
        asked: ipc::ReadTerminal,
    ) -> Result<ipc::TerminalScreen, Refusal> {
        self.job_shell_read(caller, asked).await
    }

    async fn drive_browser(
        &self,
        caller: Option<api::Caller>,
        session_id: Option<ipc::SessionId>,
        act: ipc::BrowserAct,
    ) -> Result<ipc::BrowserAnswer, Refusal> {
        self.browser_driven(caller, session_id, act).await
    }

    async fn answer_browser_request(
        &self,
        answer: ipc::AnswerBrowserRequest,
    ) -> Result<ipc::BrowserRequestAnswered, Refusal> {
        self.browser_request_answered(answer).await
    }

    async fn rename_session(&self, rename: ipc::RenameSession) -> Result<SessionRecord, Refusal> {
        let title = rename
            .title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if title.is_empty() {
            return Err(self.ledger_unnamed("a title"));
        }
        // Read before the store is held: the proposer learns his names against the request.
        let rows = self.rows_of(rename.session_id.as_str()).await.unwrap_or_default();
        let request = crate::session_host::delivering::first_message(&rows).map(str::to_string);
        let mut store = self.store().lock().await;
        let Some(mut session) = store
            .session(rename.session_id.as_str())
            .map_err(|why| self.ledger_fault(why))?
        else {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        };
        let changed = session.title.as_deref() != Some(title.as_str());
        if let (true, Some(request), Some(was), Some(repository)) =
            (changed, request, session.title.clone(), session.manifest_id.clone())
        {
            let kept = store::KeptRename {
                session_id: session.id.clone(),
                repository,
                request,
                was,
                title: title.clone(),
                renamed_at: self.now().as_str().to_string(),
            };
            // A rename that is not learned from is still a rename.
            if let Err(why) = store.keep_rename(&kept) {
                self.session_warned(&session.id, "a job's rename was not kept for naming", format!("{why:?}"));
            }
        }
        session.title = Some(title);
        store
            .keep_session(&session)
            .map_err(|why| self.ledger_fault(why))?;
        if changed {
            // A name the person gave is theirs, and a proposal still being read leaves it be.
            self.person_changed(&mut store, &session.id, ipc::ProposedField::Title);
        }
        let record = self.ledger_row(&store, &session)?;
        drop(store);
        if changed {
            self.publish(ipc::Event::SessionChanged(record.clone()));
        }
        Ok(record)
    }

    async fn name_session(&self, name: ipc::NameSession) -> Result<SessionRecord, Refusal> {
        self.named_by_armada(name).await
    }

    async fn list_sessions(
        &self,
        manifest_id: Option<ManifestId>,
        text: Option<String>,
        state: Option<SessionState>,
    ) -> Result<SessionList, Refusal> {
        let manifest = match manifest_id {
            Some(named) => Some(
                self.served_named(Some(&named))?
                    .manifest()
                    .id()
                    .as_str()
                    .to_string(),
            ),
            None => None,
        };
        let store = self.store().lock().await;
        let found = store
            .find_sessions(&SessionSearch {
                manifest_id: manifest.as_deref(),
                state: state.map(session_state),
                text: text.as_deref(),
            })
            .map_err(|why| self.ledger_fault(why))?;
        // Every row's reads are taken under the lock and it is let go before any is drawn: drawing
        // reads transcripts and the slot pool from disk, and every other Fleet command waits meanwhile.
        let mut forecasts = Forecasts::default();
        let mut taken = Vec::with_capacity(found.len());
        for session in &found {
            taken.push(self.ledger_reads_with(&store, session, &mut forecasts)?);
        }
        drop(store);
        let sessions = found.iter().zip(taken).map(|(session, reads)| self.ledger_built(session, reads)).collect();
        Ok(SessionList { sessions })
    }

    async fn recall(
        &self,
        manifest_id: Option<ManifestId>,
        query: String,
        section: Option<String>,
    ) -> Result<ipc::Recalled, Refusal> {
        self.logbook_recalled(manifest_id.as_ref(), &query, section.as_deref()).await
    }

    async fn get_logbook_for_subagent(&self, session_id: String, prompt: String) -> Result<ipc::ForSubagent, Refusal> {
        Ok(ipc::ForSubagent { text: self.logbook_for_subagent(&session_id, &prompt).await })
    }

    async fn who_owns(
        &self,
        kind: String,
        target: String,
        manifest_id: Option<ManifestId>,
    ) -> Result<Owners, Refusal> {
        let (kind, target) = (kind.trim(), target.trim().trim_start_matches('#'));
        if kind.is_empty() || target.is_empty() {
            return Err(self.ledger_unnamed("a kind and a target to ask about"));
        }
        let manifest = match manifest_id {
            Some(named) => Some(
                self.served_named(Some(&named))?
                    .manifest()
                    .id()
                    .as_str()
                    .to_string(),
            ),
            None => None,
        };
        let target = target
            .strip_prefix("slot-")
            .filter(|_| kind == "slot")
            .unwrap_or(target);
        let store = self.store().lock().await;
        let rows = store
            .attachments_at(kind, target, manifest.as_deref())
            .map_err(|why| self.ledger_fault(why))?;
        let mut holders = Vec::with_capacity(rows.len());
        // A Job's row is left out: the wire names no Job as a holder.
        for row in rows.iter().filter(|row| row.holder.kind == store::HolderKind::Session) {
            let title = store
                .session(&row.holder.id)
                .map_err(|why| self.ledger_fault(why))?
                .and_then(|session| session.title);
            holders.push(held_wire(row, title));
        }
        Ok(Owners { holders })
    }

    async fn get_session_system(self: Arc<Self>, session_id: ipc::SessionId) -> Result<ipc::SessionSystem, Refusal> {
        self.system_of(session_id).await
    }

    async fn stop_process(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        stop: ipc::StopProcess,
    ) -> Result<ipc::ProcessStopped, Refusal> {
        self.process_stopped(caller, stop).await
    }

    async fn get_session_usage(
        &self,
        session_id: ipc::SessionId,
        since: Option<String>,
        until: Option<String>,
    ) -> Result<ipc::SessionTurnUsage, Refusal> {
        let id = session_id.as_str().trim();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        crate::turn_usage_keeping::window(&store, id, since.as_deref(), until.as_deref())
            .map_err(|why| self.ledger_fault(why))
    }

    async fn get_usage_summary(
        &self,
        since: Option<String>,
        until: Option<String>,
        tz_offset_minutes: i32,
    ) -> Result<ipc::UsageSummary, Refusal> {
        let now = self.now().as_str().to_string();
        let store = self.store().lock().await;
        let read = store
            .usage_summary(since.as_deref(), until.as_deref(), &now, tz_offset_minutes)
            .map_err(|why| self.ledger_fault(why))?;
        let Some(read) = read else {
            return Err(self.ledger_unnamed("a time for each bound of the window"));
        };
        Ok(crate::usage_summarising::wire(read, tz_offset_minutes))
    }

    async fn get_usage_rules(&self, since: Option<String>, until: Option<String>) -> Result<ipc::UsageRules, Refusal> {
        let now = self.now().as_str().to_string();
        let dials = crate::usage_rules::dials_of(&self.machine_settings().now());
        let store = self.store().lock().await;
        let read = store
            .usage_rule_trips_between(since.as_deref(), until.as_deref(), &now)
            .map_err(|why| self.ledger_fault(why))?;
        let Some((since, until, trips)) = read else {
            return Err(self.ledger_unnamed("a time for each bound of the window"));
        };
        Ok(crate::usage_rules::report(&since, &until, &dials, &trips))
    }

    async fn get_session_loadout(&self, session_id: ipc::SessionId) -> Result<ipc::SessionLoadout, Refusal> {
        let id = session_id.as_str().trim();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        crate::loadout_keeping::whole(&store, id).map_err(|why| self.ledger_fault(why))
    }

    async fn get_attachment_history(&self, session_id: ipc::SessionId) -> Result<ipc::AttachmentHistory, Refusal> {
        let id = session_id.as_str().trim();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        let versions = store.attachment_versions(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
        Ok(ipc::AttachmentHistory { versions: versions.iter().map(wire::version_wire).collect() })
    }

    async fn read_loadout_item(&self, asked: ipc::LoadoutItemAsked) -> Result<ipc::LoadoutItemBody, Refusal> {
        self.loadout_item(asked).await
    }
}
