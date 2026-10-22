//! Everything Fleet is assembled from, and the assembly itself.
//!
//! **Resolved by the composition root, above this crate.** Nothing below Fleet
//! reads configuration: every dial, every path and every seam arrives here as a
//! field somebody wrote out, which is what lets a test plant one and assert on
//! what Fleet did with it.
//!
//! **Two plain structs, public fields, and no `Default` on either.** A builder
//! would let a caller stop early and a default would answer a new dial before
//! anybody had decided what it should be. Written out, adding a field is a
//! compile error at the one call site that assembles a Fleet — and
//! [`Fleet::assembled`] below is the only place the two are ever read.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, CiConfiguration, Delivery, Model, ModelClient, Vcs, WorkProduct,
};
use config::{Manifest, ResolvedWorkflow};
use core_model::WorkflowId;
use store::Store;
use tokio::sync::Mutex;

use super::Fleet;
use crate::clock::Clock;
use crate::headroom::{Headroom, Machine, Polling};
use crate::judging::JudgeBudget;
use crate::limits::Limits;
use crate::mint::Mint;
use crate::peer::PeerOf;
use crate::slots::Concurrency;

/// What Fleet knows about the machine it runs on.
///
/// **Resolved once, by the composition root, and never read from this
/// process.** `crate::drone` gives the reason for the two paths: a Drone that
/// inherited Fleet's own `PATH` would find a different toolchain on two
/// machines, and a different one again after a shell profile changes. The same
/// argument makes every field here an argument rather than a lookup.
///
/// Public fields and no `Default`, so a caller writes each one out.
#[derive(Clone, Debug)]
pub struct Host {
    /// What an agent's `PATH` is set to. Fleet's choice, not Fleet's own.
    pub path: String,
    /// The home directory the agent CLI reads its credentials from. **The
    /// confinement's known floor** — see `crate::drone::HostPaths`.
    pub home: String,
    /// Who the operator is. The agent CLI will not authenticate without it,
    /// however readable its credentials are — see `crate::drone::environment`.
    pub user: String,
    /// The strict MCP configuration an agent is bound to.
    pub mcp_config: String,
    /// The headless agent CLI's program, resolved once by the composition
    /// root's `AGENT_BINARY` override or its default. **Held here for Helm's
    /// host and not for a Drone's** — a Drone's own is `harness`'s, a typed
    /// [`AgentHarness`]; this is the plain program name
    /// `crate::helm::ProcessHost` runs, so a machine that names an override
    /// names it for a Drone, the Judge and Helm alike rather than leaving
    /// Helm on whatever `PATH` happens to hold. `#943`.
    pub agent_binary: String,
    /// The loopback port Fleet is listening on.
    ///
    /// **Held because a connection to it is what names a Session** — see
    /// `crate::peer`, which matches a caller's port against this one as a pair. Resolved once by
    /// the composition root from the listener it actually bound.
    pub port: u16,
    /// Where Fleet keeps its own copy of a Job's attachments, outside every
    /// worktree. `drafted()` writes under `<attachments_dir>/<job_id>/`, and
    /// `dispatch` reads from there to seed the worktree a Drone actually sees.
    pub attachments_dir: String,
    /// Where each hosted session's keeper socket and spool are: short, because
    /// a unix socket's path may not be long.
    pub keepers_dir: String,
    /// Where Fleet keeps a Studio's frames, beside the Studio's own records
    /// rather than in the database — `#1290`. One directory per Studio under
    /// this, and a Note names the file inside it.
    pub studio_frames_dir: String,
    /// Kit's home, `~/.armada`, where Kit's own allowlist is kept as a plain
    /// file a person can edit. Since protocol 23.35.
    pub kit_home: String,
    /// Where the mods are: one folder each, written by a person or a session
    /// and never by Fleet but at `scaffold_mod`. `docs/concepts/mods.md`.
    pub mods_dir: String,
    /// How long one Helm reply may take before its process is ended.
    /// `timeouts.helmReplySeconds`, read at start.
    pub helm_reply_budget: std::time::Duration,
    /// How hard Helm thinks. `effort.helm`, read at start; `None` passes no
    /// effort, the harness's own.
    pub helm_effort: Option<adapter_traits::Effort>,
}

/// [`Host`] without the repository: what is true of the machine whichever
/// repository a Job is in. **What Fleet holds**, so a repository's root is
/// only ever read off the repository — `crate::repositories`.
#[derive(Clone, Debug)]
pub(crate) struct Local {
    pub(crate) path: String,
    pub(crate) home: String,
    pub(crate) user: String,
    pub(crate) port: u16,
    pub(crate) attachments_dir: String,
    pub(crate) keepers_dir: String,
    pub(crate) studio_frames_dir: String,
    pub(crate) kit_home: String,
    pub(crate) mods_dir: String,
}

/// One repository a Fleet is assembled already serving.
#[derive(Clone, Debug)]
pub struct StartingIn {
    /// Absolute. Every worktree of its Jobs is added here.
    pub root: String,
    /// Where its Job records live, never under `root`. `crate::records::root`
    /// resolves it.
    pub records_root: String,
    /// Every workflow a Job there may run, keyed by `workflow_id`, each
    /// resolved against `manifest`.
    pub workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    /// The Kit and carried definitions left out of `workflows`.
    pub left_out: Vec<ipc::LeftOutWorkflow>,
    /// Every definition file read, as `config`'s catalogue kept them.
    pub files: Vec<config::WorkflowFile>,
    pub manifest: Manifest,
}

/// Everything Fleet is assembled from.
///
/// A plain struct with public fields rather than a builder, for the reason
/// `NewJob` gives: there is no `Default`, so a caller writes every field out
/// and cannot forget one, and adding a field is a compile error at the one call
/// site that matters.
pub struct Fittings<H, V, W> {
    pub store: Store,
    pub harness: H,
    pub vcs: V,
    pub work: W,
    pub clock: Arc<dyn Clock>,
    pub mint: Arc<dyn Mint>,
    /// The repository Fleet is assembled serving, if any. **None at the
    /// composition root**: a Fleet starts with nothing and a person adds the
    /// first. A test starts in one, so its cases need no add.
    pub starting_in: Option<StartingIn>,
    pub host: Host,
    /// Reading a folder a person adds. See [`crate::repositories::Locating`].
    pub locating: Arc<dyn crate::repositories::Locating>,
    /// Reading the setup a person already has, to show it. **A seam for
    /// `machine`'s reason, and one more**: the shipped answer reads a real home
    /// directory, and a fixture that used it would report whoever ran the
    /// tests. `#1491`.
    pub setup: Arc<dyn adapter_traits::HarnessSetup + Send + Sync>,
    /// The range a Job's port span is claimed from. `settings.port-range-base`,
    /// `settings.port-range-ceiling` (`crate::ports::detect_ceiling` supplies
    /// the composition root's default) and `settings.port-block-granule`.
    pub port_range: crate::ports::PortRange,
    /// `settings.ad-hoc-run-log-retention`. How long a run fired by hand from
    /// the Manifest surface keeps its log — see [`mod@crate::rehearsing`].
    pub run_log_retention: std::time::Duration,
    /// `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only`,
    /// resolved here like every other Machine setting. `#943`.
    pub helm_authority: crate::helm::Authority,
    /// How long one Helm call is held open for a person before Fleet answers
    /// `deny` for them. `#1389`, and `crate::helm::HelmAskHold` says why it is
    /// a fitting.
    pub helm_ask_hold: crate::helm::HelmAskHold,
    /// How a caller is placed: which process holds the connection a tool call
    /// arrived on. **A seam so a test can plant one** — the shipped answer is
    /// [`peer::Kernel`](crate::peer::Kernel), and a fixture has no sockets to
    /// ask about.
    pub peers: Arc<dyn PeerOf>,
    /// How many Jobs Fleet may work at once **where nobody has saved another**.
    /// The `settings.concurrency-cap` row, enforced — see [`Concurrency`], which
    /// has no default for the reason none of the four dials above it does, and
    /// `crate::limits` for how a saved value replaces it.
    pub concurrency: Concurrency,
    /// What the machine has left, asked rather than assumed. **A seam so a
    /// test can plant one**, exactly as `peers` is — the shipped answer is
    /// [`TheMachine`](crate::headroom::TheMachine) and a fixture has no machine
    /// it can hold still.
    pub machine: Arc<dyn Machine>,
    /// How much memory and disk must be free before another Drone starts,
    /// **where nobody has saved another**. The
    /// `settings.cpu-mem-headroom-threshold-for-spawning` and
    /// `settings.disk-headroom-floor-for-spawning` rows, enforced — see
    /// [`Headroom`], which has no default for [`Concurrency`]'s reason.
    pub headroom: Headroom,
    /// How many Checks may run at once on this machine, **where nobody has
    /// saved another**. The `settings.checks-at-once` row, enforced — see
    /// [`ChecksAtOnce`](crate::ChecksAtOnce), which has no default for
    /// [`Concurrency`]'s reason. #284, #1063.
    pub checks_at_once: crate::ChecksAtOnce,
    /// The Check slots every process on the machine takes from, `armada check`
    /// included. **`None` in a test**: a fixture shares no machine.
    pub check_slots: Option<checks_runner::CheckSlots>,
    /// How stale a machine reading may be. **The
    /// `settings.fleet-health-check-resource-poll-interval` row** — see
    /// [`Polling`] for why it is a freshness bound rather than a second timer.
    pub polling: Polling,
    /// How often the forge is asked about one pull request. **The
    /// `settings.merge-notice-interval` row** — `crate::noticing`.
    pub noticing: crate::noticing::Noticing,
    /// What makes a Judge call. **A pointer rather than a type parameter**: the
    /// seam renders and cannot fail, so nothing about it needs to be generic.
    pub judge: Arc<dyn ModelClient + Send + Sync>,
    /// How long one Judge call may take. See [`JudgeBudget`] for why it has no
    /// default.
    pub judge_budget: JudgeBudget,
    /// How long one proposer or pull request review call may take. **Not `judge_budget`** — the
    /// call is the same call and the wait is not: a Judge's is the only thing that can end a
    /// call nobody is watching, and a proposal has somebody who can end it themselves.
    pub proposer_budget: JudgeBudget,
    /// How long one Check may run. See [`crate::budget::CheckBudget`].
    pub budget: crate::budget::CheckBudget,
    /// What a step naming no model of its own is judged by. **Resolved by the
    /// composition root**, like every other input here — which model is cheap
    /// is a vendor's fact, and nothing below Fleet may spell one.
    pub judge_model: Model,
    /// What a Session's request is read by. **Its own dial and not the Judge's** — this call
    /// fires on every dispatch rather than on every criterion, so the two are raised for
    /// different reasons and at different prices.
    pub proposer_model: Model,
    /// What a Job's retro is written on. **Its own dial**: a retro is one call
    /// per ended Job, read by a person, and raised for neither of the others'
    /// reasons.
    pub retro_model: Model,
    /// The tracker a repository's open issues are read from and written back
    /// on, and what a request's own link in it resolves to before it becomes a
    /// Job's `facts`; asked per repository with what its Manifest chose.
    /// **A pointer rather than a type parameter**, for `judge`'s reason:
    /// rendering cannot fail, so nothing about it needs to be generic.
    /// `crate::tracking`. #91.
    pub trackers: Arc<dyn adapter_traits::Trackers + Send + Sync>,
    /// What reads a repository's CI configuration for Scan. **A seam so Fleet
    /// names no provider**, and so a test can plant what a reader answers.
    pub ci_configuration: Arc<dyn CiConfiguration + Send + Sync>,
    /// The models a Job may name, and the one it gets when it names none.
    ///
    /// **Resolved by the composition root, like every other input here.**
    /// Nothing below reads configuration, which is what lets a test plant a
    /// roster and assert on what a proposal with no model was given. Where the
    /// default is blank, a proposal that names no model is refused at creation
    /// rather than at spawn.
    pub models: ipc::ModelChoices,
    pub events: api::Broadcaster,
    /// settings.json, already read. Every live dial above is what ships, and
    /// what the file chose is put over it here and at every save or hand edit.
    pub settings: crate::settings::MachineSettings,
    /// Signing in to an account, through the forge and the store, in the machine
    /// directory. **A seam so a test can plant both** — see [`crate::signing_in`].
    pub signing_in: crate::signing_in::SigningIn,
    /// A repository's forge address and the store a profile is enrolled with,
    /// for a claim on a red main. **A seam so a test can plant both** — see
    /// [`crate::red_claim`].
    pub reaching: Arc<dyn crate::red_claim::Reaching>,
    /// How long a hosted session's process may sit with no turn before Fleet
    /// ends it. `timeouts.sessionQuietSeconds`, read at start.
    pub session_quiet: std::time::Duration,
    /// What sends a request from Requests and the Keychain its secrets are kept in. **A seam so
    /// a test plants both** and never touches the login keychain — [`crate::requesting`].
    pub requests: crate::requesting::Requesting,
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
    pub fn assembled(fittings: Fittings<H, V, W>) -> Fleet<H, V, W> {
        let shipped = Limits {
            concurrency: fittings.concurrency,
            headroom: fittings.headroom,
            checks_at_once: fittings.checks_at_once,
        };
        // **settings.json, not the store's row**, since the file replaced it:
        // a file that was refused at start left nothing saved, so this is the
        // shipped limits and the refusal is served.
        let settings_now = fittings.settings.now();
        let in_force = shipped.overlaid_by(&settings_now);
        // A Check's width, which the agents' own builds take too
        // (`crate::drone::HostPaths::width`). Read once for the hosts: a saved
        // Jobs bound moves it for them at the next start.
        let width = checks_runner::CheckWidth::read(in_force.concurrency.jobs());
        let run = fittings.mint.ulid();
        let helm = crate::helm::Conversations::holding_for(fittings.helm_ask_hold);
        let sessions = crate::session_host::Hosts::hosted_by(
            Arc::new(crate::session_host::ProcessHost::on_this_machine(
                &fittings.host,
                width,
            )),
            fittings.session_quiet,
            fittings.helm_ask_hold,
        );
        let scouts = crate::scout::Scouts::hosted_by(Arc::new(
            crate::scout::ScoutHost::on_this_machine(&fittings.host, width),
        ));
        let shipped_tuning = fittings.shipped_tuning();
        let tuning = shipped_tuning.overlaid_by(&settings_now);
        let timeline = crate::timeline::Timeline::new(Arc::clone(&fittings.clock));
        Fleet {
            store: crate::store_lock::StoreLock::new(fittings.store),
            harness: Arc::new(fittings.harness),
            vcs: Arc::new(fittings.vcs),
            work: fittings.work,
            clock: fittings.clock,
            mint: fittings.mint,
            repositories: Arc::new(match fittings.starting_in {
                Some(first) => crate::repositories::Repositories::starting_in(
                    first.root,
                    first.records_root,
                    crate::repositories::SetUp::of(first.manifest, first.workflows)
                        .leaving_out(first.left_out)
                        .with_files(first.files),
                ),
                None => crate::repositories::Repositories::none(),
            }),
            locating: fittings.locating,
            setup: fittings.setup,
            host: Local {
                path: fittings.host.path,
                home: fittings.host.home,
                user: fittings.host.user,
                port: fittings.host.port,
                attachments_dir: fittings.host.attachments_dir,
                keepers_dir: fittings.host.keepers_dir.clone(),
                studio_frames_dir: fittings.host.studio_frames_dir,
                kit_home: fittings.host.kit_home,
                mods_dir: fittings.host.mods_dir,
            },
            port_range: fittings.port_range,
            tuning: std::sync::RwLock::new(tuning),
            shipped_tuning,
            settings: fittings.settings,
            signing_in: fittings.signing_in,
            reaching: fittings.reaching,
            judge: fittings.judge,
            trackers: fittings.trackers,
            tracking: crate::tracking::Tracking::new(),
            ci_configuration: fittings.ci_configuration,
            models: fittings.models,
            models_held: Default::default(),
            downshift: Default::default(),
            events: fittings.events,
            helm,
            sessions,
            session_runs: Default::default(),
            session_check_live: Default::default(),
            window_sign_ins: Default::default(),
            scouts,
            timeline,
            machine: fittings.machine,
            headroom: std::sync::Mutex::new(in_force.headroom),
            concurrency: std::sync::Mutex::new(in_force.concurrency),
            places: crate::places::Places::on_the_machine(
                in_force.checks_at_once,
                width,
                fittings.check_slots,
            ),
            shipped,
            kept: crate::worktrees_kept::Kept::default(),
            sweeping: Mutex::new(crate::noticing::Sweep::default()),
            writes: Arc::default(),
            territory: Arc::default(),
            review_going: Mutex::new(std::collections::BTreeSet::new()),
            behind_told: Default::default(),
            usage_watch: Default::default(),
            rehearsals: crate::rehearsing::Rehearsals::default(),
            servers: crate::servers::Servers::default(),
            peers: fittings.peers,
            requests: fittings.requests,
            merge_end: Mutex::new(()),
            building: crate::building::Building::default(),
            mods: crate::mods::Told::default(),
            language_model: Default::default(),
            reply_reads: Arc::default(),
            refusal_reads: Arc::default(),
            extract_reads: Arc::default(),
            run,
        }
    }

    /// The same Fleet with its scouts started by `host` — a stand-in agent in
    /// a test.
    pub fn scouting_on(mut self, host: crate::scout::ScoutHost) -> Fleet<H, V, W> {
        self.scouts = crate::scout::Scouts::hosted_by(Arc::new(host));
        self
    }

    /// The same Fleet with its hosted sessions' processes started by
    /// `processes`, and ended after `quiet` — a stand-in agent in a test.
    pub fn hosting_sessions_on(
        mut self,
        processes: Arc<dyn crate::session_host::Processes>,
        quiet: std::time::Duration,
    ) -> Fleet<H, V, W> {
        self.sessions = crate::session_host::Hosts::hosted_by(processes, quiet, self.helm.hold());
        self
    }

    pub(crate) fn session_check_live(&self) -> &crate::session_check_live::SessionCheckLive {
        &self.session_check_live
    }

    pub(crate) fn session_runs(&self) -> &crate::session_running::SessionRuns {
        &self.session_runs
    }

    pub(crate) fn window_sign_ins(&self) -> &crate::window_sign_in::SignIns {
        &self.window_sign_ins
    }

    pub(crate) fn scouts(&self) -> &crate::scout::Scouts {
        &self.scouts
    }
}
