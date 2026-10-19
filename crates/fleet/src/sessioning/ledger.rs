//! Where a session stands, and the ledger reads and writes Fleet does on its behalf.

use super::*;

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
        let held = store.attachments_of(&Holder::session(&session.id)).map_err(fault)?;
        let build = build_read(store, session, &held).map_err(fault)?;
        let hosting = if session.origin == "bridge" { store.hosting(&session.id).map_err(fault)? } else { None };
        let rail = match &hosting {
            Some(hosting) => {
                crate::rail_keeping::rail_read(store, session, hosting.workflow.is_some()).map_err(fault)?
            }
            None => Default::default(),
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
            plan: store.session_plan(&session.id).map_err(fault)?,
            touches: store.session_touches(&session.id).map_err(fault)?,
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
        if reads.hosting.is_some() && session.state == store::SessionState::Live {
            record.activity = self.activity_of(&session.id);
        }
        if let Some(hosting) = &reads.hosting {
            record.workflow = hosting.workflow.clone();
            let reading = self.hosts().of(&session.id).state().proposal.reading.is_some();
            let picked = crate::session_host::proposing::picked_of(&hosting.proposed);
            record.proposal = (reading || !picked.is_empty())
                .then_some(ipc::SessionProposal { reading, picked });
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
        record.loadout_summary = crate::loadout_keeping::summary(reads.loadout);
        let waited = self.waiting_from(reads.waiting, session, &record.attachments);
        record.waiting_for = waited.items;
        let settled = waited.settled;
        let plan_held = waited.plan_held;
        record.standing = (session.state == store::SessionState::Live).then_some(waited.standing);
        record.build = reads.build.record(record.standing == Some(ipc::SessionStanding::Stuck), workers_of(&record.attachments));
        record.plan = reads.plan.as_ref().map(|kept| ipc::SessionPlan { held_by: plan_held, ..crate::plan_record::wire(kept) });
        record.touches = crate::plan_keeping::touches_wire(reads.touches);
        record.settled = settled;
        // A Session still waiting on another carries the wait until it starts.
        record.waits = reads
            .wait
            .filter(|_| session.state == store::SessionState::Live)
            .map(|(wait, title, part)| crate::session_host::after::wait_wire(&wait, title, part));
        self.rail_drawn(session, &reads.rail, &mut record);
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

    /// A terminal Session wrote these files, named from its directory. **Only in a pool slot**, the
    /// one place Fleet can tell whose tree a file is in.
    pub(super) fn session_wrote(&self, session: &KeptSession, roots: &[(String, String)], paths: &[String]) {
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
