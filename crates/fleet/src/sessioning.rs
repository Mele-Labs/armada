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

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    ManifestId, Owners, SessionFact, SessionList, SessionRecord, SessionReport, SessionState,
    WireError,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptSession, SessionSearch, Store};

use crate::adrift::Adrift;
use crate::session_host::waiting::{waiting_reads, WaitingReads};
use crate::daemon::Fleet;
use crate::slot_holders::{pool_backs, stand_in_slot};
use crate::timeline::{Act, Doer};

/// What a session's row reads from the store, taken under the lock.
pub(crate) struct LedgerReads {
    held: Vec<KeptAttachment>,
    hosting: Option<store::KeptHosting>,
    waiting: WaitingReads,
    /// When the limit the session hit resets, while it is held for one.
    limited: Option<String>,
    /// The breakdown of the context window, as the wire spells it.
    context: Option<String>,
}

/// A report that names nothing it can be kept against. A 422.
const SESSION_UNNAMED: &str = "fleet.session_unnamed";

/// Whether an address is one Bridge's window will load: any `http` or `https` address with a host
/// and no blank in it (`docs/practices/capture-window.md`).
fn shows_a_page(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    let name = match host.strip_prefix('[') {
        Some(inner) => inner.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    !name.is_empty() && !url.contains(char::is_whitespace)
}

/// The takes a call named, trimmed, or why they cannot be shown. **Each address is held to the one
/// `url` is**, and a call that names takes shows the first: `url` must be that same page.
fn takes_shown(url: &str, named: Vec<ipc::WindowTake>) -> Result<Vec<ipc::WindowTake>, &'static str> {
    let takes: Vec<_> = named
        .into_iter()
        .map(|one| ipc::WindowTake { name: one.name.trim().to_string(), url: one.url.trim().to_string() })
        .collect();
    if takes.iter().any(|one| one.name.is_empty()) {
        return Err("a name on every variation");
    }
    if takes.iter().any(|one| !shows_a_page(&one.url)) {
        return Err("an `http` or `https` address on every variation");
    }
    if takes.first().is_some_and(|first| first.url != url) {
        return Err("`url` to be the first variation's address");
    }
    Ok(takes)
}

/// The two kinds a session holds one of at a time. Every other kind is kept as
/// the open word it arrived as.
const ONE_AT_A_TIME: [&str; 2] = ["branch", "slot"];

/// Where a directory is, as far as Fleet can tell.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Placed {
    pub manifest: Option<String>,
    pub slot: Option<u32>,
}

/// The repository and pool slot `cwd` stands in, over `roots` of
/// `(manifest, repository root)`. **The deepest root wins**, so a repository
/// nested in another is its own.
pub(crate) fn placed(cwd: &str, roots: &[(String, String)]) -> Placed {
    let mut best: Option<(usize, &str, &str)> = None;
    for (manifest, root) in roots {
        let root = root.trim_end_matches('/');
        let Some(rest) = cwd.strip_prefix(root) else {
            continue;
        };
        if !(rest.is_empty() || rest.starts_with('/')) {
            continue;
        }
        if best.is_none_or(|(deepest, _, _)| root.len() > deepest) {
            best = Some((root.len(), manifest, rest));
        }
    }
    let Some((_, manifest, rest)) = best else {
        return Placed::default();
    };
    let slot = rest
        .strip_prefix(&format!("/{}/slot-", adapter_traits::SLOT_ROOT))
        .and_then(|after| {
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            let boundary =
                after[digits.len()..].is_empty() || after[digits.len()..].starts_with('/');
            boundary.then(|| digits.parse().ok()).flatten()
        });
    Placed {
        manifest: Some(manifest.to_string()),
        slot,
    }
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
    pub(crate) fn ledger_roots(&self) -> Vec<(String, String)> {
        self.repositories()
            .served()
            .iter()
            .map(|one| {
                (
                    one.manifest().id().as_str().to_string(),
                    one.root().to_string(),
                )
            })
            .collect()
    }

    fn ledger_unnamed(&self, what: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            SESSION_UNNAMED,
            format!("a session report needs {what}"),
            self.run_id(),
        ))
    }

    pub(crate) fn ledger_fault(&self, why: store::WriteError) -> Refusal {
        self.refusal(Adrift::Writing(why))
    }

    /// The row, with everything its holder holds, as the wire carries it.
    pub(crate) fn ledger_row(
        &self,
        store: &Store,
        session: &KeptSession,
    ) -> Result<SessionRecord, Refusal> {
        Ok(self.ledger_built(session, self.ledger_reads(store, session)?))
    }

    /// Everything [`ledger_built`](Self::ledger_built) needs from the store, and nothing else:
    /// a caller that holds the lock takes this and lets go before the row is drawn.
    pub(crate) fn ledger_reads(
        &self,
        store: &Store,
        session: &KeptSession,
    ) -> Result<LedgerReads, Refusal> {
        let fault = |why| self.ledger_fault(why);
        Ok(LedgerReads {
            held: store.attachments_of(&Holder::session(&session.id)).map_err(fault)?,
            hosting: if session.origin == "bridge" {
                store.hosting(&session.id).map_err(fault)?
            } else {
                None
            },
            waiting: waiting_reads(store, session).map_err(fault)?,
            limited: store.session_limit(&session.id).map_err(fault)?,
            context: store.session_context(&session.id).map_err(fault)?,
        })
    }

    /// What the session waits on, off what [`ledger_reads`](Self::ledger_reads) took, for a caller that
    /// wants no more of the row than that.
    pub(crate) fn ledger_waiting(
        &self,
        session: &KeptSession,
        reads: LedgerReads,
    ) -> Vec<ipc::WaitingItem> {
        let attachments: Vec<ipc::Attachment> = reads.held.iter().map(attachment_wire).collect();
        self.waiting_from(reads.waiting, session, &attachments).items
    }

    /// The row off what [`ledger_reads`](Self::ledger_reads) took: the disk is read here, not
    /// under the store lock.
    pub(crate) fn ledger_built(&self, session: &KeptSession, reads: LedgerReads) -> SessionRecord {
        let mut record = session_wire(session, &reads.held);
        // A subagent the mod never settles is done once its own transcript shows its turn ended.
        let home = &self.host().home;
        for one in record.attachments.iter_mut().filter(|one| {
            one.kind == "subagent" && one.state == ipc::AttachmentState::Standing
        }) {
            if adapters::terminal_thread::subagent_ended(home, &session.id, &one.target) {
                one.state = ipc::AttachmentState::Spent;
            }
        }
        if session.origin == "terminal" {
            let terminals = self.hosts().terminals();
            let listening =
                session.state == store::SessionState::Live && terminals.listening(&session.id);
            let asked = self.hosts().of(&session.id).state().asked.clone();
            record.terminal = match (terminals.facts_of(&session.id), listening, asked) {
                (None, false, None) => None,
                (facts, _, asked) => Some(ipc::TerminalFacts {
                    listening,
                    asked,
                    ..facts.unwrap_or_default()
                }),
            };
            record.mod_out_of_date = self.mod_out_of_date(session);
        }
        record.helm = reads.hosting.as_ref().is_some_and(|hosting| hosting.helm);
        if reads.hosting.is_some() && session.state == store::SessionState::Live {
            record.activity = self.activity_of(&session.id);
        }
        record.hosted = reads
            .hosting
            .map(|hosting| self.hosted_facts_of(&session.id, hosting));
        if session.state == store::SessionState::Live {
            record.done_since = reads.waiting.done.as_deref().map(wire_instant);
            record.held_until = reads.limited.as_deref().map(wire_instant);
        }
        record.context_breakdown = reads
            .context
            .as_deref()
            .and_then(|kept| ipc::decode("a context breakdown", kept.as_bytes()).ok());
        let waited = self.waiting_from(reads.waiting, session, &record.attachments);
        record.waiting_for = waited.items;
        record.standing = (session.state == store::SessionState::Live).then_some(waited.standing);
        record
    }

    /// Keep the version a mod reported. A fact that carries none leaves what is kept.
    fn mod_said(session: &mut KeptSession, version: Option<String>) -> bool {
        let Some(version) = version.filter(|one| session.mod_version.as_ref() != Some(one)) else {
            return false;
        };
        session.mod_version = Some(version);
        true
    }

    /// Whether the mod this terminal session runs is older than the one its repository holds. Where
    /// Fleet cannot read the repository's version there is nothing to be older than.
    fn mod_out_of_date(&self, session: &KeptSession) -> bool {
        let Some(manifest) = session.manifest_id.as_deref() else {
            return false;
        };
        let roots = self.ledger_roots();
        let Some((_, root)) = roots.iter().find(|(id, _)| id == manifest) else {
            return false;
        };
        let Some(current) = mod_version_in(&self.host().home, root) else {
            return false;
        };
        mod_is_older(session.mod_version.as_deref(), &current)
    }

    /// A terminal Session wrote these files, named from its directory. **Only in a pool slot**, the
    /// one place Fleet can tell whose tree a file is in.
    fn session_wrote(&self, session: &KeptSession, roots: &[(String, String)], paths: &[String]) {
        let place = placed(&session.cwd, roots);
        let (Some(manifest), Some(slot)) = (place.manifest, place.slot) else {
            return;
        };
        let Some((_, root)) = roots.iter().find(|(id, _)| *id == manifest) else {
            return;
        };
        let worktree = adapter_traits::slot_path(root.trim_end_matches('/'), slot);
        let inside = crate::clashing::inside_worktree(&worktree, &session.cwd, paths);
        if !inside.is_empty() {
            let who = crate::clashing::Who::Session(session.id.clone());
            self.writes().wrote(&who, &manifest, inside, &self.now());
        }
    }
}

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
            return Err(self.ledger_unnamed("a session id"));
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
            }
            SessionFact::Settled { attachment, state } => {
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
                    if let Some(line) = said.as_deref().map(crate::session_host::standing::first_line).filter(|line| !line.is_empty()) {
                        store.keep_turn_end(&id, &now, &line).map_err(fault)?;
                    }
                }
                // A turn taken since the limit was hit: the wake is not owed.
                store.clear_session_limit(&id).map_err(fault)?;
                changed = true;
                // Whatever it was changing for the person is done for now.
                crate::session_host::readiness::windows_set(&mut store, &holder, &now, false).map_err(fault)?;
            }
            SessionFact::Wrote => {
                changed |= crate::session_host::readiness::windows_set(&mut store, &holder, &now, true).map_err(fault)?;
            }
            SessionFact::Limited { resets_at } => {
                changed |= store
                    .keep_session_limit(&id, resets_at.as_str())
                    .map_err(fault)?;
            }
            // For the check of who else writes these files; nothing on the ledger changes.
            SessionFact::Touched { paths } => {
                self.session_wrote(&session, &roots, &paths);
                // A hosted session is heard from its own process, which enters the same acts.
                if terminal {
                    self.timeline().used(Doer::of_session(&session), Act::Edit, paths.len(), paths.last().cloned());
                }
            }
            SessionFact::Read { path } if terminal => {
                self.timeline().used(Doer::of_session(&session), Act::Read, 1, Some(path));
            }
            SessionFact::Ran { command } if terminal => {
                self.timeline().used(Doer::of_session(&session), Act::Ran, 1, Some(command));
            }
            SessionFact::Read { .. } | SessionFact::Ran { .. } => {}
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

        session.last_seen_at = now;
        store.keep_session(&session).map_err(fault)?;
        let record = self.ledger_row(&store, &session)?;
        drop(store);
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
        Ok(record)
    }

    async fn start_another_session(
        &self,
        caller: Option<api::Caller>,
        start: ipc::StartAnotherSession,
    ) -> Result<ipc::SessionHeld, Refusal> {
        self.another_session_started(caller, start).await
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

    async fn get_session_check_output(&self, run: u64) -> Result<ipc::CheckOutput, Refusal> {
        self.session_check_output(run).await
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
        let hosted = caller.and_then(|caller| self.session_holding(&caller));
        let id = hosted
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.ledger_unnamed("a session to show it for"))?;
        let harness = {
            let store = self.store().lock().await;
            store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.ledger_unnamed("a session Fleet knows"))?
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
                        .collect(),
                    },
                },
            })
            .await?;
        self.row_put(
            &id,
            ipc::SessionRow::Window {
                id: self.row_id(&id),
                at: self.instant(),
                title,
                url,
                variations,
                ended_at: None,
            },
        )
        .await;
        Ok(record)
    }

    async fn show_sketch(
        &self,
        caller: Option<api::Caller>,
        show: ipc::ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        self.sketch_shown(caller, show).await
    }

    async fn ask_in_window(
        &self,
        caller: Option<api::Caller>,
        ask: ipc::AskInWindow,
    ) -> Result<SessionRecord, Refusal> {
        self.window_asked(caller, ask).await
    }

    async fn window_closed(&self, closed: ipc::WindowClosed) -> Result<SessionRecord, Refusal> {
        let id = closed.session_id.as_str().trim().to_string();
        let known = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.is_some()
        };
        if !known {
            return Err(self.ledger_unnamed("a session Fleet knows"));
        }
        self.walk_ended(&id, closed.url.trim()).await;
        let store = self.store().lock().await;
        let session = store
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.ledger_unnamed("a session Fleet knows"))?;
        self.ledger_row(&store, &session)
    }

    async fn waiting_for(
        &self,
        caller: Option<api::Caller>,
        set: ipc::SetWaitingFor,
    ) -> Result<SessionRecord, Refusal> {
        self.set_waiting(caller, set).await
    }

    async fn all_done(
        &self,
        caller: Option<api::Caller>,
        done: ipc::AllDone,
    ) -> Result<SessionRecord, Refusal> {
        self.say_done(caller, done).await
    }

    async fn finish_up(
        &self,
        caller: Option<api::Caller>,
        finish: ipc::FinishUp,
    ) -> Result<ipc::FinishedUp, Refusal> {
        self.finished_up(caller, finish).await
    }

    async fn start_session_run(
        &self,
        start: ipc::StartSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.begin_session_run(start).await
    }

    async fn stop_session_run(
        &self,
        stop: ipc::StopSessionRun,
    ) -> Result<ipc::SessionRunUnderway, Refusal> {
        self.end_session_run(stop)
    }

    async fn observe_session_run(&self, run_id: String) -> Result<api::ObservedSessionRun, Refusal> {
        self.watch_session_run(&run_id)
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
        let mut store = self.store().lock().await;
        let Some(mut session) = store
            .session(rename.session_id.as_str())
            .map_err(|why| self.ledger_fault(why))?
        else {
            return Err(self.ledger_unnamed("a session Fleet knows"));
        };
        let changed = session.title.as_deref() != Some(title.as_str());
        session.title = Some(title);
        store
            .keep_session(&session)
            .map_err(|why| self.ledger_fault(why))?;
        let record = self.ledger_row(&store, &session)?;
        drop(store);
        if changed {
            self.publish(ipc::Event::SessionChanged(record.clone()));
        }
        Ok(record)
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
        let mut sessions = Vec::with_capacity(found.len());
        for session in &found {
            sessions.push(self.ledger_row(&store, session)?);
        }
        Ok(SessionList { sessions })
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
        for row in &rows {
            let title = match row.holder.kind {
                store::HolderKind::Session => store
                    .session(&row.holder.id)
                    .map_err(|why| self.ledger_fault(why))?
                    .and_then(|session| session.title),
                store::HolderKind::Job => None,
            };
            holders.push(held_wire(row, title));
        }
        Ok(Owners { holders })
    }
}

/// The `version` the `armada` plugin declares, where it carries one: the installed copy under `home`
/// first, which is the one a session loads, and the repository's own where there is none.
fn mod_version_in(home: &str, root: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        version: Option<String>,
    }
    let installed = crate::runtime::mod_dir(home).join(adapters::MOD_INSTALLED_MANIFEST);
    let bytes = std::fs::read(installed)
        .or_else(|_| std::fs::read(std::path::Path::new(root).join(adapters::MOD_MANIFEST)))
        .ok()?;
    ipc::decode::<Manifest>("the armada plugin's manifest", &bytes)
        .ok()?
        .version
}

/// Whether `reported` is behind `current`. No version reported is behind; a version that does not
/// read as dotted numbers is behind unless it is the same text.
pub(crate) fn mod_is_older(reported: Option<&str>, current: &str) -> bool {
    let Some(reported) = reported else {
        return true;
    };
    let parts = |text: &str| -> Option<Vec<u64>> { text.split('.').map(|one| one.parse().ok()).collect() };
    match (parts(reported), parts(current)) {
        (Some(had), Some(want)) => had < want,
        _ => reported != current,
    }
}

fn origin_text(origin: ipc::SessionOrigin) -> &'static str {
    match origin {
        ipc::SessionOrigin::Terminal => "terminal",
        ipc::SessionOrigin::Bridge => "bridge",
    }
}

fn attachment_state(state: ipc::AttachmentState) -> AttachmentState {
    match state {
        ipc::AttachmentState::Standing => AttachmentState::Standing,
        ipc::AttachmentState::Spent => AttachmentState::Spent,
        ipc::AttachmentState::GivenBack => AttachmentState::GivenBack,
    }
}

fn session_state(state: SessionState) -> store::SessionState {
    match state {
        SessionState::Live => store::SessionState::Live,
        SessionState::Ended => store::SessionState::Ended,
    }
}

fn wire_instant(at: &str) -> ipc::Instant {
    ipc::Instant::carried(at)
}

fn attachment_wire(row: &KeptAttachment) -> ipc::Attachment {
    ipc::Attachment {
        kind: row.kind.clone(),
        manifest_id: (!row.manifest_id.is_empty()).then(|| ManifestId::carried(&row.manifest_id)),
        target: row.target.clone(),
        state: match row.state {
            AttachmentState::Standing => ipc::AttachmentState::Standing,
            AttachmentState::Spent => ipc::AttachmentState::Spent,
            AttachmentState::GivenBack => ipc::AttachmentState::GivenBack,
        },
        detail: row.detail.clone(),
        since: wire_instant(&row.since),
        changed_at: wire_instant(&row.changed_at),
    }
}

fn held_wire(row: &KeptAttachment, title: Option<String>) -> ipc::Ownership {
    ipc::Ownership {
        holder: ipc::Holder {
            kind: match row.holder.kind {
                store::HolderKind::Session => ipc::HolderKind::Session,
                store::HolderKind::Job => ipc::HolderKind::Job,
            },
            id: row.holder.id.clone(),
        },
        attachment: attachment_wire(row),
        title,
    }
}

fn session_wire(session: &KeptSession, held: &[KeptAttachment]) -> SessionRecord {
    SessionRecord {
        id: ipc::SessionId::carried(&session.id),
        harness: session.harness.clone(),
        origin: if session.origin == "bridge" {
            ipc::SessionOrigin::Bridge
        } else {
            ipc::SessionOrigin::Terminal
        },
        manifest_id: session.manifest_id.as_deref().map(ManifestId::carried),
        cwd: session.cwd.clone(),
        title: session.title.clone(),
        state: match session.state {
            store::SessionState::Live => SessionState::Live,
            store::SessionState::Ended => SessionState::Ended,
        },
        started_at: wire_instant(&session.started_at),
        last_seen_at: wire_instant(&session.last_seen_at),
        last_turn_at: session.last_turn_at.as_deref().map(wire_instant),
        ended_at: session.ended_at.as_deref().map(wire_instant),
        end_reason: session.end_reason.clone(),
        usage: ipc::SessionUsage {
            context_tokens: session.figures.context_tokens,
            context_window: session.figures.context_window,
            cost_micros: session.figures.cost_micros,
        },
        attachments: held.iter().map(attachment_wire).collect(),
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        waiting_for: Vec::new(),
        done_since: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        helm: false,
        activity: None,
    }
}
