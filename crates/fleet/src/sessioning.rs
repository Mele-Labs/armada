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
use crate::build_keeping::{build_heard, build_read, BuildReads};
use crate::build_record::Observed;
use crate::session_host::waiting::{waiting_reads, WaitingReads};
use crate::daemon::Fleet;
use crate::session_host::asking_in_window::origin_of;
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
    /// The latest Loadout list and the latest turn's uses.
    loadout: crate::loadout_keeping::LoadoutReads,
    /// What the build record is derived from.
    build: BuildReads,
    /// The plan the Session recorded, and the files it and its subagents touched.
    plan: Option<store::KeptPlan>,
    touches: Vec<store::KeptTouch>,
    /// What the workflow rail and the judge's last reading are derived from.
    rail: crate::rail_keeping::RailReads,
    /// The wait of a Session still waiting on another, and the title of the one it waits on.
    wait: Option<(store::KeptWait, Option<String>, Option<String>)>,
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

/// Most steps one walk carries, and most the walk weighs once written. **It rides on the session's
/// record** as the window's artifact, which is published whole on every change.
const MOST_STEPS: usize = 200;
const MOST_STEP_BYTES: usize = 32 * 1024;
/// Longest a caption, a name, a typed text and a key may be, in characters.
const MOST_SAID: usize = 240;
const MOST_NAME: usize = 200;
const MOST_TYPED: usize = 1000;
const MOST_KEY: usize = 40;
/// How deep `within` may nest, and how far a drag may go, in pixels.
const MOST_NESTING: usize = 4;
const MOST_DRAG: i32 = 5000;

/// Why a target cannot be found on a page, or `None` where it can be looked for.
fn target_problem(target: &ipc::WalkTarget, depth: usize) -> Option<&'static str> {
    if target.role.trim().is_empty() || target.role.chars().any(char::is_whitespace) || target.role.chars().count() > MOST_KEY {
        return Some("to have a `role` of one word on every target, such as `button`, or `text`");
    }
    match target.name.as_deref() {
        Some(name) if name.chars().count() > MOST_NAME => return Some("to have a target `name` of no more than 200 characters"),
        Some(name) if name.trim().is_empty() => return Some("to have a target `name` with words in it, or none"),
        None if target.role == "text" => return Some("to have a `name` on a `text` target: the words it draws"),
        _ => {}
    }
    match target.within.as_deref() {
        Some(_) if depth >= MOST_NESTING => Some("to nest its targets no more than four deep"),
        Some(inner) => target_problem(inner, depth + 1),
        None => None,
    }
}

/// What is wrong with one step, said as what the report needs of it, or `None`.
fn step_problem(step: &ipc::WalkStep) -> Option<&'static str> {
    if step.say.trim().is_empty() || step.say.chars().count() > MOST_SAID {
        return Some("to say, in a line of no more than 240 characters, what it shows");
    }
    let verbs = [
        step.press.is_some(),
        step.look.is_some(),
        step.hover.is_some(),
        step.later.is_some(),
        step.typed.is_some(),
        step.key.is_some(),
        step.select.is_some(),
        step.paste.is_some(),
        step.drag.is_some(),
    ];
    if verbs.into_iter().filter(|one| *one).count() != 1 {
        return Some("to name exactly one verb of press, look, hover, later, type, key, select, paste or drag");
    }
    // The target is the verb's own, and the keys that belong to another verb are stray.
    let (target, stray) = if step.typed.is_some() {
        (step.into.as_ref(), [step.on.is_some(), step.within.is_some(), step.by.is_some()])
    } else if step.key.is_some() {
        (step.on.as_ref(), [step.into.is_some(), step.within.is_some(), step.by.is_some()])
    } else if step.select.is_some() {
        (step.within.as_ref(), [step.into.is_some(), step.on.is_some(), step.by.is_some()])
    } else if step.drag.is_some() {
        (step.drag.as_ref(), [step.into.is_some(), step.on.is_some(), step.within.is_some()])
    } else {
        let own = step.press.as_ref().or(step.look.as_ref()).or(step.hover.as_ref()).or(step.later.as_ref()).or(step.paste.as_ref());
        (own, [step.into.is_some() || step.on.is_some(), step.within.is_some(), step.by.is_some()])
    };
    if stray.into_iter().any(|one| one) || (step.drag.is_some() && step.by.is_none()) {
        return Some("to pair each verb with its own target: `type` with `into`, `key` with `on`, `select` with `in`, `drag` with `by`, and no other");
    }
    let Some(target) = target else {
        return Some("to have a target: `into` for `type`, `on` for `key`, `in` for `select`");
    };
    if let Some(problem) = target_problem(target, 0) {
        return Some(problem);
    }
    if step.typed.as_deref().is_some_and(|words| words.chars().count() > MOST_TYPED) {
        return Some("to type no more than 1000 characters");
    }
    if step.key.as_deref().is_some_and(|key| key.is_empty() || key.chars().count() > MOST_KEY) {
        return Some("to have a `key` such as `n`, `Tab` or `Meta+n`");
    }
    if step.select.as_deref().is_some_and(|words| words.is_empty() || words.chars().count() > MOST_NAME) {
        return Some("to select words of no more than 200 characters");
    }
    if step.by.is_some_and(|by| by.x.abs() > MOST_DRAG || by.y.abs() > MOST_DRAG) {
        return Some("to drag `by` no more than 5000 pixels each way");
    }
    None
}

/// The steps a call named, trimmed, or what the report needs of them. **The first step that is
/// wrong is named by its place**, so the session fixes that one and not the whole list.
pub(crate) fn steps_shown(named: Vec<ipc::WalkStep>) -> Result<Vec<ipc::WalkStep>, String> {
    if named.len() > MOST_STEPS {
        return Err(String::from("no more than 200 steps"));
    }
    let steps: Vec<_> = named
        .into_iter()
        .map(|mut one| {
            one.say = one.say.trim().to_string();
            one
        })
        .collect();
    if let Some((at, problem)) = steps.iter().enumerate().find_map(|(at, one)| step_problem(one).map(|why| (at, why))) {
        return Err(format!("step {} {problem}", at + 1));
    }
    if ipc::WalkStep::as_detail(&steps).len() > MOST_STEP_BYTES {
        return Err(String::from("steps that weigh no more than 32 KiB written out"));
    }
    Ok(steps)
}

/// The origins a call named, trimmed, or why they cannot be passed through. **An origin only**:
/// `http` or `https`, a host and an optional port, and no path, query, fragment or sign-in of its own.
fn origins_named(named: Vec<String>) -> Result<Vec<String>, &'static str> {
    let origins: Vec<String> = named.into_iter().map(|one| one.trim().to_string()).collect();
    let bare = |one: &String| {
        shows_a_page(one) && one.split_once("://").is_some_and(|(_, rest)| !rest.contains(['/', '?', '#', '@']))
    };
    if origins.iter().all(bare) {
        Ok(origins)
    } else {
        Err("every `origins` entry to be an `http` or `https` origin with no path, such as https://login.example.com")
    }
}

/// The sign-in addresses a call named, trimmed, or why they cannot be loaded. **Each is a page
/// on the shown origin or on one of `origins`**, so a sign-in reaches nowhere the window may not.
fn sign_in_named(url: &str, origins: &[String], named: Vec<String>) -> Result<Vec<String>, &'static str> {
    let addresses: Vec<String> = named.into_iter().map(|one| one.trim().to_string()).collect();
    let allowed: Vec<String> = std::iter::once(url).chain(origins.iter().map(String::as_str)).filter_map(origin_of).collect();
    let on_one = |one: &String| shows_a_page(one) && origin_of(one).is_some_and(|origin| allowed.contains(&origin));
    if addresses.iter().all(on_one) {
        Ok(addresses)
    } else {
        Err("every `sign_in` entry to be an `http` or `https` address on the shown origin or one of `origins`")
    }
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
        record.standing = (session.state == store::SessionState::Live).then_some(waited.standing);
        record.build = reads.build.record(record.standing == Some(ipc::SessionStanding::Stuck), workers_of(&record.attachments));
        record.plan = reads.plan.as_ref().map(crate::plan_record::wire);
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
                }
                // A turn taken since the limit was hit: the wake is not owed.
                store.clear_session_limit(&id).map_err(fault)?;
                changed = true;
                // Whatever it was changing for the person is done for now, unless a subagent is still on it.
                crate::session_host::readiness::turn_ended(&mut store, &holder, &now).map_err(fault)?;
            }
            SessionFact::Wrote => {
                changed |= crate::session_host::readiness::windows_set(&mut store, &holder, &now, true).map_err(fault)?;
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
            // For the check of who else writes these files; nothing on the ledger changes.
            SessionFact::Touched { paths, by, act } => {
                self.session_wrote(&session, &roots, &paths);
                // A hosted session is heard from its own process, which enters the same acts.
                if terminal {
                    self.timeline().used(Doer::of_session(&session), Act::Edit, paths.len(), paths.last().cloned());
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
                    for path in &paths {
                        changed |= crate::plan_keeping::touch_kept(&mut store, &id, by.clone(), path, act, &now)
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
                    self.timeline().used(Doer::of_session(&session), Act::Read, 1, Some(path));
                }
                changed |= build_heard(&mut store, &id, &Observed::Worked, &now).map_err(fault)?;
            }
            SessionFact::Ran { command } => {
                if terminal {
                    let doer = Doer::of_session(&session);
                    match adapters::command_reach(&command) {
                        Some(reach) => self.timeline().called(doer, reach),
                        None => self.timeline().used(doer, Act::Ran, 1, Some(command)),
                    }
                }
                changed |= build_heard(&mut store, &id, &Observed::Worked, &now).map_err(fault)?;
            }
            fact @ (SessionFact::Tasks { .. } | SessionFact::PlanLeft | SessionFact::Committed { .. } | SessionFact::Pushed { .. }) => {
                changed |= self.build_told(&mut store, &session, fact, &now).map_err(fault)?;
            }
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
        Ok(record)
    }

    async fn take_window_sign_in(&self, take: ipc::TakeWindowSignIn) -> Result<ipc::WindowSignIn, Refusal> {
        let sign_in = self.window_sign_ins().take(take.session_id.as_str().trim(), take.url.trim());
        Ok(ipc::WindowSignIn { sign_in: ipc::SignIn(sign_in) })
    }

    async fn show_sketch(
        &self,
        caller: Option<api::Caller>,
        show: ipc::ShowSketch,
    ) -> Result<SessionRecord, Refusal> {
        self.sketch_shown(caller, show).await
    }

    async fn record_plan(
        &self,
        caller: Option<api::Caller>,
        record: ipc::RecordPlan,
    ) -> Result<SessionRecord, Refusal> {
        self.plan_recorded(caller, record).await
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
        let known = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.is_some()
        };
        if !known {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        // Bridge names a step it cannot play by its place, which is not a page that failed to load.
        let text = if failed.why.trim().starts_with("step ") {
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
        let told = self.told_by_fleet(&id, &text).await?;
        Ok(ipc::WalkTold { told })
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

    async fn mark_annotation_done(
        &self,
        caller: Option<api::Caller>,
        mark: ipc::MarkAnnotationDone,
    ) -> Result<ipc::AnnotationMarked, Refusal> {
        self.annotation_done(caller, mark).await
    }

    async fn start_session_run(
        self: Arc<Self>,
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
            return Err(self.ledger_unnamed("a job Fleet knows"));
        };
        let changed = session.title.as_deref() != Some(title.as_str());
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

    async fn get_session_loadout(&self, session_id: ipc::SessionId) -> Result<ipc::SessionLoadout, Refusal> {
        let id = session_id.as_str().trim();
        let store = self.store().lock().await;
        if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.ledger_unnamed("a job Fleet knows"));
        }
        crate::loadout_keeping::whole(&store, id).map_err(|why| self.ledger_fault(why))
    }

    async fn read_loadout_item(&self, asked: ipc::LoadoutItemAsked) -> Result<ipc::LoadoutItemBody, Refusal> {
        self.loadout_item(asked).await
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
        holder: ipc::Holder { kind: ipc::HolderKind::Session, id: row.holder.id.clone() },
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
        loadout_summary: None,
        helm: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judging: false,
        judge: None,
        settled: Vec::new(),
        waits: None,
    }
}
