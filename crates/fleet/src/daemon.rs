//! The daemon core: the [`Fleet`] a process is. It is assembled from
//! [`mod@fittings`], answers through [`mod@answering`], and hands what it holds
//! to the crate around it through [`mod@seams`].
//!
//! **A refused transition is not survivable.** Every move goes through
//! `Job::transition` or `Job::transition_step`, and no arm below logs an
//! `Adrift::IllegalMove` or `Adrift::IllegalStepMove` and continues: a refusal
//! means Fleet asked for what the edge table says cannot happen, a bug in Fleet.
//!
//! **The store is behind one lock**, [`crate::store_lock`], taken for as short a
//! time as a read or a write takes and never across a Check or a Judge call.
//! [`Fleet::merge_end`] is a second and sits outside it, for the reason its
//! field gives.

use std::sync::Arc;

use adapter_traits::{CiConfiguration, ModelClient, Trackers};
use core_model::Ulid;
use tokio::sync::Mutex;

use crate::clock::Clock;
use crate::headroom::{Headroom, Machine};
use crate::mint::Mint;
use crate::peer::PeerOf;

mod answering;
pub use answering::Reconciled;
mod fittings;
mod rereading;
mod seams;

pub(crate) use fittings::Local;
pub use fittings::{Fittings, Host, StartingIn};

/// The daemon core: **the only writer of Job state.**
pub struct Fleet<H, V, W> {
    store: crate::store_lock::StoreLock,
    harness: Arc<H>,
    vcs: Arc<V>,
    work: W,
    clock: Arc<dyn Clock>,
    mint: Arc<dyn Mint>,
    /// Every repository this Fleet serves. See [`crate::repositories`].
    repositories: Arc<crate::repositories::Repositories>,
    /// Reading a folder a person adds. The composition root's.
    locating: Arc<dyn crate::repositories::Locating>,
    /// Reading the setup a person already has, to show it — `crate::kit`. It
    /// reads and cannot write, and nothing it returns reaches a Drone. `#1491`.
    setup: Arc<dyn adapter_traits::HarnessSetup + Send + Sync>,
    host: Local,
    /// The range a Job's port span is claimed from, and the granule its width
    /// rounds up to. See [`crate::ports`].
    port_range: crate::ports::PortRange,
    /// Every live dial in force: what the composition root shipped, overlaid
    /// by settings.json. A `std` lock for `drones`' reason — never held across
    /// an `.await`. See [`crate::tuning`].
    tuning: std::sync::RwLock<crate::tuning::Tuning>,
    /// The dials as handed in, which a key removed from the file falls back to.
    shipped_tuning: crate::tuning::Tuning,
    /// settings.json: the file, what was in force at start, and the last good
    /// read. See [`crate::settings`].
    settings: crate::settings::MachineSettings,
    /// Signing in to an account, and the profiles joined. See [`crate::signing_in`].
    signing_in: crate::signing_in::SigningIn,
    /// The central store's other half: a repository's forge address and the
    /// store a profile is enrolled with. See [`crate::red_claim`].
    reaching: Arc<dyn crate::red_claim::Reaching>,
    judge: Arc<dyn ModelClient + Send + Sync>,
    trackers: Arc<dyn Trackers + Send + Sync>,
    /// The tracker, the issue lists last read from it, and when Fleet last
    /// wrote back. Never written down but for what was said — `crate::tracking`.
    tracking: crate::tracking::Tracking,
    ci_configuration: Arc<dyn CiConfiguration + Send + Sync>,
    models: ipc::ModelChoices,
    /// What the harness said it offers, and what asks it. See [`crate::models_read`].
    models_held: crate::models_read::ModelsHeld,
    /// What turns the Phone Gateway on and off. See [`crate::phone`].
    phone_held: crate::phone::PhoneHeld,
    /// The default model's step down, never held but for what the store keeps. See [`crate::model_downshifting`].
    downshift: crate::model_downshifting::Holding,
    events: api::Broadcaster,
    /// Every Helm conversation, one per repository, and the host that carries
    /// their messages. See [`crate::helm`].
    helm: crate::helm::Conversations,
    /// The sessions Fleet hosts for Bridge. See [`crate::session_host`].
    sessions: crate::session_host::Hosts,
    /// Shell blocks run from a Session's thread. Never written down — `crate::session_running`.
    session_runs: crate::session_running::SessionRuns,
    /// Session Checks that are running, and a stop asked of one — `crate::session_check_live`.
    session_check_live: crate::session_check_live::SessionCheckLive,
    /// The addresses a session signs in with, until Bridge opens its window. A secret, never written down — `crate::window_sign_in`.
    window_sign_ins: crate::window_sign_in::SignIns,
    /// Every scout running, and the host that starts one. Outside `slots`:
    /// a scout takes no place under the concurrency cap. See [`crate::scout`].
    scouts: crate::scout::Scouts,
    /// Everything that happened, entered as it did, for the Cockpit's timeline. Minted here: it
    /// holds the queue the writer reads. See [`mod@crate::timeline`].
    timeline: crate::timeline::Timeline,
    machine: Arc<dyn Machine>,
    /// The headroom in force: shipped, or what a person saved. A `std` lock
    /// for `drones`' reason — never held across an `.await`. See
    /// [`crate::limits`], which is the only writer.
    headroom: std::sync::Mutex<Headroom>,
    /// The bound a person saved, which the width of a build is read off. See [`crate::limits`].
    concurrency: std::sync::Mutex<crate::slots::Concurrency>,
    /// The machine's places for Checks, and the limit in force: shipped, or what
    /// a person saved. One line for every Job and repository — `crate::places`.
    places: crate::places::Places,
    /// What the composition root handed in, kept so a save can say what
    /// shipped and an omitted field can fall back to it.
    shipped: crate::limits::Limits,
    /// The last `GET /worktrees`, served from memory. See [`crate::worktrees_kept`].
    kept: crate::worktrees_kept::Kept,
    /// What the forge-reading loops last saw. **Never written down**: what each decides is
    /// re-derived from the forge and the store every time, so a cursor that outlived the process
    /// would name a position in a list that has since changed.
    sweeping: Mutex<crate::noticing::Sweep>,
    /// Which files each live Job and Session has written, and the clashes found between them.
    /// Never written down, for `sweeping`'s reason; an `Arc` because a Drone's tap holds it —
    /// `crate::clashing`.
    writes: Arc<crate::clashing::Writes>,
    /// What each repository holds and what each slot's holder changed, kept a while. Never written
    /// down, for `sweeping`'s reason; an `Arc` because a read is spawned — `crate::territory`.
    territory: Arc<crate::territory::Territory>,
    /// The agent runs about pull requests that are going right now, by key. Never written down:
    /// a row the store says is going that is not here died with the process that held it
    /// (`crate::review_runs`).
    review_going: Mutex<std::collections::BTreeSet<String>>,
    /// Whom each slot far behind its base was told to merge it. Never written down — `crate::behind_bay`.
    behind_told: crate::behind_bay::BehindTold,
    /// Which Jobs were told which token rule. Rebuilt from the trip rows when needed — `crate::usage_rules`.
    usage_watch: crate::usage_rules::Watch,
    /// Which Jobs have a person's run out in their worktree, and how to stop
    /// it. Never written down, for `pressing`'s reason — `crate::rehearsing`.
    rehearsals: crate::rehearsing::Rehearsals,
    /// Which servers Fleet holds, a Job's and the main checkout's, and the last
    /// of each that ended. Never written down, for `rehearsals`' reason —
    /// `crate::servers`.
    servers: crate::servers::Servers,
    peers: Arc<dyn PeerOf>,
    /// What sends a request and the secrets it is filled with — `crate::requesting`.
    requests: crate::requesting::Requesting,
    /// The rebase-and-push tail, held by one Job at a time.
    ///
    /// **Every worktree is cut from one `.git`**, and whether two of them can
    /// rebase and push into it concurrently is not established — `#50` accepted
    /// the serialisation rather than discovering git's ref lockfiles by way of
    /// a Job dying at its push, unattended. So dispatch and the work run
    /// N-wide and this one part does not.
    ///
    /// **It is at the tail and not at admission.** A lock taken when a Job
    /// starts would be the single working slot again under another name; this
    /// one is taken when a Job's branch is touched and released when it has
    /// been.
    merge_end: Mutex<()>,
    /// When `origin main` was last fetched, for `get_fleet_build`. Nothing else
    /// of the build is held: the files `scripts/restart` leaves are the record.
    building: crate::building::Building,
    /// The mod list as last published. Never written down: a rescan after a
    /// restart sets it again from the folder, and says nothing for doing so.
    mods: crate::mods::Told,
    /// The local language model once it is loaded — `crate::language_model`.
    language_model: crate::language_model::LanguageModel,
    /// The model's reads of Sessions' replies. Never written down — `crate::session_host::asks_model`.
    reply_reads: Arc<crate::session_host::asks_model::Reads>,
    /// The model's reads of whether a reply turns a Fleet tell down — `crate::session_host::tell_refusal`.
    refusal_reads: Arc<crate::session_host::asks_model::Reads>,
    /// The model's vectors of Sessions' waiting items, for folding ones that are one ask — `crate::session_host::dedup`.
    item_vectors: Arc<crate::session_host::dedup::Vectors>,
    /// The model's reads of each sentence of a reply's close — `crate::session_host::extracting`.
    extract_reads: Arc<crate::session_host::asks_model::Reads<crate::session_host::extracting::Extracted>>,
    /// **This process's** run id, minted once at assembly.
    ///
    /// It names the emitter rather than a record, which is the one id a
    /// process mints for itself — and it is why a Fleet restart is visible as
    /// this value changing rather than as nothing at all.
    run: Ulid,
}
