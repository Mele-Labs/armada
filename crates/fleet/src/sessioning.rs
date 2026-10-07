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
use crate::daemon::Fleet;

/// A report that names nothing it can be kept against. A 422.
const SESSION_UNNAMED: &str = "fleet.session_unnamed";

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
        let held = store
            .attachments_of(&Holder::session(&session.id))
            .map_err(|why| self.ledger_fault(why))?;
        let mut record = session_wire(session, &held);
        if session.origin == "terminal" {
            let terminals = self.hosts().terminals();
            let listening =
                session.state == store::SessionState::Live && terminals.listening(&session.id);
            record.terminal = match (terminals.facts_of(&session.id), listening) {
                (None, false) => None,
                (facts, _) => Some(ipc::TerminalFacts {
                    listening,
                    ..facts.unwrap_or_default()
                }),
            };
            record.mod_out_of_date = self.mod_out_of_date(session);
        }
        if session.origin == "bridge" {
            record.hosted = self
                .hosted_facts(store, &session.id)
                .map_err(|why| self.ledger_fault(why))?;
        }
        Ok(record)
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

    /// Take the slot `placed` names for `holder`, or give back the one it was in
    /// where it stands in none. `true` where the ledger changed.
    fn stand_in_slot(
        store: &mut Store,
        holder: &Holder,
        placed: &Placed,
        now: &str,
    ) -> Result<bool, store::WriteError> {
        let (Some(manifest), Some(slot)) = (&placed.manifest, placed.slot) else {
            return store.give_back(holder, Some("slot"), now);
        };
        store.attach(
            &KeptAttachment {
                holder: holder.clone(),
                kind: "slot".into(),
                manifest_id: manifest.clone(),
                target: slot.to_string(),
                state: AttachmentState::Standing,
                detail: Default::default(),
                since: now.into(),
                changed_at: now.into(),
            },
            true,
        )
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
                Self::stand_in_slot(&mut store, &holder, &place, &now).map_err(fault)?;
                changed = true;
            }
            SessionFact::Moved { cwd } => {
                if session.cwd != cwd {
                    let place = placed(&cwd, &roots);
                    session.cwd = cwd;
                    session.manifest_id = place.manifest.clone();
                    Self::stand_in_slot(&mut store, &holder, &place, &now).map_err(fault)?;
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
            SessionFact::Measured { usage } => {
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
                    },
                );
            }
            SessionFact::TurnCompleted => {
                session.last_turn_at = Some(now.clone());
                changed = true;
            }
            // Fleet ends a session it hosts, by `close_session`: the harness in
            // it going away is a process ending, and the next message resumes.
            SessionFact::Ended { .. } if session.origin == "bridge" => {}
            SessionFact::Ended { reason } => {
                changed |= session.state != store::SessionState::Ended;
                session.state = store::SessionState::Ended;
                session.ended_at = Some(now.clone());
                session.end_reason = Some(reason);
                store.give_back(&holder, None, &now).map_err(fault)?;
            }
        }

        session.last_seen_at = now;
        store.keep_session(&session).map_err(fault)?;
        let record = self.ledger_row(&store, &session)?;
        drop(store);
        if changed {
            self.publish(ipc::Event::SessionChanged(record.clone()));
        }
        Ok(record)
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
    let installed = crate::runtime::mod_dir(home).join("armada/.claude-plugin/plugin.json");
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
    }
}
