//! Where a session stands, and the ledger reads and writes Fleet does on its behalf.

use std::collections::BTreeMap;

use super::*;
use crate::slot_holders::Claim;
use crate::tree_paths::Tree;

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
            .hosted()
            .iter()
            .map(|one| {
                (
                    one.manifest().id().as_str().to_string(),
                    one.root().to_string(),
                )
            })
            .collect()
    }

    /// The worktree `cwd` stands in: the pool slot's folder where it is in one, else `cwd` itself.
    /// The one place Fleet can tell where the tree starts, which a Job's touches are named from.
    pub(crate) fn worktree_of(&self, cwd: &str) -> String {
        let roots = self.ledger_roots();
        let place = placed(cwd, &roots);
        match (place.manifest, place.slot) {
            (Some(manifest), Some(slot)) => roots
                .iter()
                .find(|(id, _)| *id == manifest)
                .map(|(_, root)| adapter_traits::slot_path(root.trim_end_matches('/'), slot)),
            _ => None,
        }
        .unwrap_or_else(|| cwd.to_string())
    }

    pub(crate) fn ledger_unnamed(&self, what: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            SESSION_UNNAMED,
            format!("a job report needs {what}"),
            self.run_id(),
        ))
    }

    pub(crate) fn ledger_fault(&self, why: store::WriteError) -> Refusal {
        self.refusal(Adrift::Writing(why))
    }

    /// The reading the store kept, for a Fleet no session has reported to since it started: the
    /// windows not yet reset, taken as the one held so the next report is judged against it. Nothing
    /// is published or judged for it; only a session's own report does that.
    pub(crate) async fn plan_usage_restored(&self) -> ipc::PlanUsage {
        let Ok(Some(kept)) = self.store().lock().await.plan_reading() else {
            return self.hosts().plan_usage();
        };
        let Ok(mut usage) = ipc::decode::<ipc::PlanUsage>("the kept plan reading", kept.as_bytes()) else {
            return self.hosts().plan_usage();
        };
        let now = self.now().epoch_millis();
        usage.windows.retain(|window| {
            let reset = window.resets_at.as_ref().and_then(|at| at.to_domain().epoch_millis());
            !matches!((reset, now), (Some(reset), Some(now)) if reset <= now)
        });
        if usage.windows.is_empty() {
            return self.hosts().plan_usage();
        }
        self.hosts().plan_restored(usage)
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
        self.ledger_reads_with(store, session, &mut Forecasts::default())
    }

    /// [`ledger_reads`](Self::ledger_reads) for many rows under one hold of the lock, which share `forecasts`.
    pub(crate) fn ledger_reads_with(
        &self,
        store: &Store,
        session: &KeptSession,
        forecasts: &mut Forecasts,
    ) -> Result<LedgerReads, Refusal> {
        let fault = |why| self.ledger_fault(why);
        let held = store.attachments_of(&Holder::session(&session.id)).map_err(fault)?;
        let build = build_read(store, session, &held, forecasts).map_err(fault)?;
        let hosting = if session.origin == "bridge" { store.hosting(&session.id).map_err(fault)? } else { None };
        let start = match &hosting {
            Some(hosting) if hosting.start_held => Some(crate::session_host::start_card::start_read(store, hosting).map_err(fault)?),
            _ => None,
        };
        let rail = match &hosting {
            Some(hosting) => {
                crate::rail_keeping::rail_read(store, session, hosting.workflow.is_some()).map_err(fault)?
            }
            None => Default::default(),
        };
        let by = held.iter().find(|one| one.kind == "started_by").map(|link| ipc::SessionLink {
            title: store.session(&link.target).ok().flatten().and_then(|parent| parent.title).unwrap_or_default(),
            id: ipc::SessionId::carried(&link.target),
        });
        let started = match (store.session_started(&session.id).map_err(fault)?, by) {
            (None, None) => None,
            (kept, by) => Some(match kept {
                Some(kept) => ipc::SessionStarted {
                    workflow: kept.workflow,
                    delivery: kept.delivery.as_deref().and_then(crate::session_host::start_card::chosen),
                    model: kept.model,
                    effort: kept.effort,
                    mode: Some(crate::session_host::mode_of(&kept.mode)),
                    by,
                },
                None => ipc::SessionStarted { by, ..Default::default() },
            }),
        };
        let wait = store
            .wait_of(&session.id)
            .map_err(fault)?
            .filter(|one| one.started_at.is_none())
            .map(|one| {
                let title = store.session(&one.on_session).ok().flatten().and_then(|on| on.title);
                let part = (!one.part.is_empty())
                    .then(|| store.session_plan(&one.on_session).ok().flatten())
                    .flatten()
                    .and_then(|plan| crate::session_host::after::plan_part_title(&crate::plan_record::wire(&plan), &one.part));
                (one, title, part)
            });
        let plan = store.session_plan(&session.id).map_err(fault)?;
        let pulls = match &plan {
            Some(_) => crate::plan_keeping::pulls_read(store, &session.id, &held).map_err(fault)?,
            None => Vec::new(),
        };
        let stop = store.session_stop(&session.id).map_err(fault)?;
        let open_turn = store.open_turn(&session.id).map_err(fault)?;
        Ok(LedgerReads {
            wait,
            rail,
            held,
            build,
            hosting,
            waiting: waiting_reads(store, session).map_err(fault)?,
            limited: store.session_limit(&session.id).map_err(fault)?,
            context: store.session_context(&session.id).map_err(fault)?,
            loadout: crate::loadout_keeping::summary_read(store, &session.id).map_err(fault)?,
            plan,
            judge_readings: store.judge_readings(&session.id).map_err(fault)?,
            touches: store.session_touches(&session.id).map_err(fault)?,
            pulls,
            later: store.later_of(&session.id).map_err(fault)?,
            watching: store.live_watch_names(&session.id).map_err(fault)?,
            holding: self.lifecycles_holding(store, &session.id),
            repair_over: stop.repair_job.as_deref().is_some_and(|repair| {
                let ended = store.session(repair).ok().flatten().is_none_or(|one| one.state == store::SessionState::Ended);
                ended || store.done_since(repair).ok().flatten().is_some()
            }),
            away: open_turn
                .as_ref()
                .and_then(|turn| Some(store.fleet_outages_between(&turn.since, turn.ended_at.as_deref()?).unwrap_or_default()))
                .unwrap_or_default(),
            stop,
            open_turn,
            start,
            started,
        })
    }

    /// What the session waits on, off what [`ledger_reads`](Self::ledger_reads) took, for a caller that
    /// wants no more of the row than that. **The row's own list**, drawn whole, so the wait a step
    /// raises is answered and dismissed as any other item is.
    pub(crate) fn ledger_waiting(
        &self,
        session: &KeptSession,
        reads: LedgerReads,
    ) -> Vec<ipc::WaitingItem> {
        self.ledger_built(session, reads).waiting_for
    }

    /// The row off what [`ledger_reads`](Self::ledger_reads) took: the disk is read here, not
    /// under the store lock.
    pub(crate) fn ledger_built(&self, session: &KeptSession, reads: LedgerReads) -> SessionRecord {
        let mut record = session_wire(session, &reads.held);
        if session.state == store::SessionState::Live {
            self.slots_held_drawn(session, &mut record.attachments);
        }
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
        record.auto_land = reads.hosting.as_ref().is_some_and(|hosting| hosting.auto_land);
        record.r#mod = reads.hosting.as_ref().is_some_and(|hosting| hosting.mod_job);
        record.from_job = reads.hosting.as_ref().and_then(|hosting| hosting.from_job.clone());
        record.from_tab = reads.hosting.as_ref().and_then(|hosting| hosting.from_tab.clone());
        record.mod_of_bay = reads.hosting.as_ref().is_some_and(|hosting| hosting.mod_of_bay);
        if reads.hosting.is_some() && session.state == store::SessionState::Live {
            record.activity = self.activity_of(&session.id);
            record.tools_missing = self.hosts().of(&session.id).state().tools_missing.clone();
        }
        if let Some(hosting) = &reads.hosting {
            record.workflow = hosting.workflow.clone();
            let reading = self.hosts().of(&session.id).state().proposal.reading.is_some();
            let picked = crate::session_host::proposing::picked_of(&hosting.proposed);
            record.proposal = (reading || !picked.is_empty())
                .then_some(ipc::SessionProposal { reading, picked });
        }
        let picks = reads.hosting.as_ref().map(|hosting| (hosting.workflow.clone(), hosting.model.clone(), hosting.effort.clone()));
        record.hosted = reads
            .hosting
            .map(|hosting| self.hosted_facts_of(&session.id, hosting, reads.start));
        if session.state == store::SessionState::Live {
            record.done_since = reads.waiting.done.as_deref().map(wire_instant);
            record.done_summary = reads.waiting.done_summary.clone();
            record.so_far = reads.waiting.so_far.clone();
            record.held_until = reads.limited.as_deref().map(wire_instant);
        }
        record.context_breakdown = reads
            .context
            .as_deref()
            .and_then(|kept| ipc::decode("a context breakdown", kept.as_bytes()).ok());
        record.loadout_summary = crate::loadout_keeping::summary(reads.loadout);
        let reply_owed_since = reads.waiting.reply_owed_since();
        let waited = self.waiting_from(reads.waiting, session, &record.attachments);
        record.waiting_for = waited.items;
        record.closed = waited.closed;
        let parked = waited.start_held || waited.paused;
        let spoke = waited.spoke;
        let died_saying = waited.died_saying;
        let plan_held = waited.plan_held;
        let resting = waited.resting;
        record.standing = (session.state == store::SessionState::Live).then_some(waited.standing);
        record.build = reads.build.record(record.standing == Some(ipc::SessionStanding::Stuck), workers_of(&record.attachments));
        record.plan = reads
            .plan
            .as_ref()
            .map(|kept| ipc::SessionPlan { held_by: plan_held, ..crate::plan_record::wire_read(kept, &reads.touches, &reads.pulls) });
        if let (Some(kept), Some(activity)) = (&reads.plan, &mut record.activity) {
            crate::plan_record::mark_listed(kept, &mut activity.todos);
        }
        record.touches = crate::plan_keeping::touches_wire(reads.touches);
        record.judge_readings = self.judge_readings_on_record(&session.id, &reads.judge_readings);
        record.later = reads
            .later
            .into_iter()
            .filter(|_| session.state == store::SessionState::Live)
            .map(|one| ipc::SessionLater { id: one.id, sentence: one.sentence, said_at: wire_instant(&one.said_at), confirmed: one.confirmed })
            .collect();
        // A Session still waiting on another carries the wait until it starts.
        record.waits = reads
            .wait
            .filter(|_| session.state == store::SessionState::Live)
            .map(|(wait, title, part)| {
                let mut wire = crate::session_host::after::wait_wire(&wait, title, part);
                (wire.workflow, wire.model, wire.effort) = picks.unwrap_or_default();
                wire
            });
        record.started = reads.started;
        self.rail_drawn(session, &reads.rail, reply_owed_since.as_deref(), &mut record);
        // Held open only when what it waits on is all that is left, read once the rail has drawn its wait: a
        // turn running or an item owed says more than a watch or a pull request does. A command the agent
        // backgrounded does not: a dev server outlives the work, so what holds the Job reads over it.
        let live = session.state == store::SessionState::Live;
        let at_rest = |standing: Option<ipc::SessionStanding>| matches!(standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
        if live && at_rest(Some(resting)) && record.waiting_for.is_empty() {
            let pulls = record.attachments.iter().filter_map(pull_request_wait);
            let started = reads.holding.iter().filter(|one| one.lifecycle == JOB_LIFECYCLE).map(|one| ipc::HeldOpenBy::Job { name: one.label.clone() });
            record.held_open_by = reads.watching.into_iter().map(|name| ipc::HeldOpenBy::Watch { name }).chain(started).chain(pulls).collect();
            if !record.held_open_by.is_empty() {
                record.standing = Some(resting);
            }
        }
        let between = live && at_rest(record.standing) && record.waiting_for.is_empty();
        let excused = !between
            || parked
            || !record.held_open_by.is_empty()
            || record.helm
            // Its process died in the turn: Fleet is starting it again, or it reads as dead below.
            || reads.open_turn.as_ref().is_some_and(|turn| turn.ended_at.is_some())
            || record.waits.is_some()
            || record.held_until.is_some()
            // A ticket lives as long as its work: only what is under way elsewhere excuses a stop.
            || reads.holding.iter().any(|one| one.lifecycle != TICKET_LIFECYCLE)
            || record.attachments.iter().any(open_pull_request)
            || (session.origin != "bridge" && !self.hosts().terminals().listening(&session.id))
            || self.hosts().of(&session.id).state().moving.is_some();
        let stop = session.last_turn_at.as_deref().filter(|_| !excused);
        let read = stop.map(|stop| crate::session_host::stopping::read(stop, &reads.stop, spoke.as_deref(), self.now().epoch_millis().unwrap_or(0)));
        if read == Some(crate::session_host::stopping::Read::Stopped) {
            record.standing = Some(ipc::SessionStanding::Stopped);
            record.stopped = stop.map(|stop| crate::session_host::stopping::stopped(stop, &record));
        }
        let owed = stop.filter(|_| read == Some(crate::session_host::stopping::Read::Owed)).map(|stop| {
            let from = [Some(stop), spoke.as_deref()].into_iter().flatten().filter_map(|at| core_model::Timestamp::from_rfc3339(at).epoch_millis()).max();
            (stop, from.unwrap_or(0))
        });
        self.hosts().stops_owed().noted(&session.id, owed);
        // Dead with the restart spent: nothing ends a turn here, so no turn's end says it.
        let dead = session.state == store::SessionState::Live
            && !parked
            && reads.open_turn.as_ref().is_some_and(crate::session_host::stopping::restart_spent)
            && self.hosts().of(&session.id).state().process.is_none();
        if dead {
            record.standing = Some(ipc::SessionStanding::Stopped);
            record.stopped = reads.open_turn.as_ref().map(|turn| crate::session_host::stopping::dead(turn, &reads.stop, reads.repair_over, &reads.away, died_saying));
        }
        // Gone quiet with work left: the Operator is handed it on the next tick.
        self.operator_seat().stall_noted(&session.id, crate::operator::seen_of(&record, between && !parked && !record.helm));
        record.stop_told_by = owed.map(|(_, from)| wire_instant(&crate::clock::rfc3339_utc(from + crate::session_host::stopping::STOP_AFTER.as_millis() as i64)));
        // How long the running turn has been at it and when it was last heard, which Now says beside it.
        if session.origin == "bridge" {
            let runtime = self.hosts().of(&session.id);
            let state = runtime.state();
            if matches!(state.turn, ipc::SessionTurn::Working { .. }) {
                record.turn_since = state.turn_began.as_deref().map(wire_instant);
                let quiet = state.last_active.elapsed().as_millis() as i64;
                record.last_heard_at = self.now().epoch_millis().map(|now| wire_instant(&crate::clock::rfc3339_utc(now - quiet)));
            }
        }
        record
    }

    /// A row for every other slot the pool says the session holds: one its subagent leased has no
    /// row of its own, since the ledger keeps one slot a session works in. **A terminal session
    /// that moved out of a slot gave it up** though its shell still holds the lease, so a slot the
    /// ledger gave back for one stays given back.
    fn slots_held_drawn(&self, session: &KeptSession, attachments: &mut Vec<ipc::Attachment>) {
        let Some(manifest) = session.manifest_id.as_deref() else {
            return;
        };
        let Some(served) = self.repositories().serving(manifest) else {
            return;
        };
        let terminal = session.origin != "bridge";
        let drawn = |slot: &str| {
            attachments.iter().any(|one| {
                one.kind == "slot"
                    && one.target == slot
                    && one.manifest_id.as_ref().map(ManifestId::as_str) == Some(manifest)
                    && (terminal || one.state == ipc::AttachmentState::Standing)
            })
        };
        let more: Vec<String> = crate::slot_holders::slots_of(&served, &session.id)
            .into_iter()
            .map(|slot| slot.to_string())
            .filter(|slot| !drawn(slot))
            .collect();
        let at = wire_instant(&session.last_seen_at);
        attachments.extend(more.into_iter().map(|target| ipc::Attachment {
            kind: String::from("slot"),
            manifest_id: Some(ManifestId::carried(manifest)),
            target,
            state: ipc::AttachmentState::Standing,
            detail: Default::default(),
            since: at.clone(),
            changed_at: at.clone(),
        }));
    }

    /// Keep the version a mod reported. A fact that carries none leaves what is kept.
    pub(super) fn mod_said(session: &mut KeptSession, version: Option<String>) -> bool {
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

    /// A terminal Session wrote these files, named from its directory. Each is credited from the root
    /// of the worktree it lies in: one the Session stands in, a pool slot whose lease is not
    /// another's, or any other linked worktree. **A write to the main checkout is not credited**:
    /// Fleet cannot tell whose change in it a file is.
    pub(super) fn session_wrote(&self, store: &Store, session: &KeptSession, roots: &[(String, String)], paths: &[String]) {
        let Some(manifest) = placed(&session.cwd, roots).manifest else {
            return;
        };
        let Some((_, root)) = roots.iter().find(|(id, _)| *id == manifest) else {
            return;
        };
        let home = &self.host().home;
        let mut by_worktree: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for path in paths {
            let linked = &crate::tree_paths::linked_worktree;
            let Some(at) = crate::tree_paths::in_tree(root, Some(&session.cwd), home, path, linked) else {
                continue;
            };
            let stands_in = format!("{}/", session.cwd.trim_end_matches('/')).starts_with(&format!("{}/", at.worktree));
            let credited = match at.tree {
                Tree::Main => false,
                _ if stands_in => true,
                Tree::Slot(slot) => crate::slot_holders::claim_on(store, root, slot, &session.id) != Claim::Not,
                Tree::Linked => true,
            };
            if credited && !at.path.is_empty() {
                by_worktree.entry(at.worktree).or_default().push(at.path);
            }
        }
        let who = crate::clashing::Who::Session(session.id.clone());
        for (worktree, inside) in by_worktree {
            self.writes().wrote_in(&who, &manifest, &worktree, inside, &self.now());
        }
    }
}

/// What an own pull request waits on, read from its attachment's detail as Bridge reads a pull request: open,
/// its Checks not failed, and in the queue, set to merge or on its Checks. A merged, given back or draft one waits on nothing,
/// nor does one the person dismissed from the Waiting band.
/// The lifecycle a Job it started is held under.
const JOB_LIFECYCLE: &str = "job";
/// The lifecycle of the ticket a Job works.
const TICKET_LIFECYCLE: &str = "ticket";

/// A pull request of its own that has not merged, whatever it waits on.
fn open_pull_request(one: &ipc::Attachment) -> bool {
    one.kind == crate::pull_requesting::PR
        && one.state == ipc::AttachmentState::Standing
        && one.detail.get("state").map(String::as_str) != Some("merged")
}

fn pull_request_wait(one: &ipc::Attachment) -> Option<ipc::HeldOpenBy> {
    let detail = |key: &str| one.detail.get(key).map(String::as_str);
    if one.kind != crate::pull_requesting::PR
        || one.state != ipc::AttachmentState::Standing
        || matches!(detail("state"), Some("merged" | "draft"))
        || detail(crate::pull_requesting::WAIT_DISMISSED) == Some("true")
    {
        return None;
    }
    let number = one.target.parse().ok()?;
    let checks = detail("checks");
    let auto_merge = detail("auto_merge") == Some("true");
    let on = if checks == Some("failed") {
        return None;
    } else if detail("queued") == Some("true") {
        ipc::PullRequestWait::Queue
    } else if !matches!(checks, Some("passed")) {
        ipc::PullRequestWait::Checks { auto_merge }
    } else if auto_merge {
        ipc::PullRequestWait::AutoMerge
    } else {
        return None;
    };
    Some(ipc::HeldOpenBy::PullRequest { number, on })
}
