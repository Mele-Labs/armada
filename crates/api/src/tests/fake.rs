//! A daemon that is not Fleet.
//!
//! It holds Jobs in a `Vec` and moves them on request. **It asserts nothing
//! about the status machine** — that machine is `core-model`'s and is tested
//! there, against the edge table. This exists to answer the operations M1 serves
//! so the transport can be exercised, and it is written without `core-model` on
//! purpose: if a fake daemon can be built out of `ipc` alone, so can a Bridge.
//!
//! **This file is the record and the moves it makes.** The answers are in
//! [`shapes`](super::shapes) and the three surfaces are the three modules
//! below, one per trait — [`Queries`](crate::Queries),
//! [`Commands`](crate::Commands) and [`Tools`](crate::Tools). It was one impl
//! block at 856 lines while `api::Daemon` was one trait; the split is #434's
//! and nothing about what the fake answers moved with it.
//!
//! Every refusal a fake can honestly raise is here, beside the `Vec` it reads
//! to raise it. There are two, and the modules below hold none of their own.

mod authoring;
mod commands;
mod conversing;
mod hosted_sessions;
mod mods;
mod settings;
mod needs;
mod piloting;
mod pull_requests;
mod queries;
mod rehearsing;
mod retros;
mod sessions;
mod studios;
mod tools;

pub use conversing::SERVED_MANIFEST;
pub use studios::{the_studio, THE_FRAME, THE_STUDIO};

use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use ipc::mcp::{DeclareScope, DispatchJob, NotRecorded, SubmitEvidence};
use ipc::{Actor, Event, Instant, JobId, JobStateChanged, JobSummary, UnreadableJob};

use super::shapes;
use super::shapes::{run_id, status};
use crate::{Broadcaster, Feed, Refusal, Turns};

pub struct FakeDaemon {
    /// Every Job held, which the fake's own acts move.
    pub jobs: Mutex<Vec<JobSummary>>,
    unreadable: Mutex<Vec<UnreadableJob>>,
    events: Broadcaster,
    /// The per-Job transcript channels, so a test can watch one Job while a
    /// Board client is on `/events` and prove neither reaches the other.
    pub turns: Turns,
    /// What `observe_job` answers with as the history, for whichever Job is
    /// asked. A Job with none is the ordinary case, not an error.
    pub history: Mutex<Vec<ipc::TranscriptRow>>,
    /// Older rows the history left out.
    pub skipped: Mutex<u64>,
    minted: AtomicU64,
    /// Every submission taken, so a test can assert that a refused call left
    /// nothing behind.
    pub submitted: Mutex<Vec<SubmitEvidence>>,
    /// Every scope declaration taken, in arrival order.
    pub declared: Mutex<Vec<DeclareScope>>,
    pub requested: Mutex<Vec<ipc::mcp::RequestScope>>,
    /// Every question taken, in arrival order.
    pub asked: Mutex<Vec<ipc::mcp::AskQuestion>>,
    /// Every Job a Drone asked to have created, in arrival order.
    pub dispatched: Mutex<Vec<DispatchJob>>,
    /// How many asked runs were asked for, so a test can assert that a refused
    /// call ran nothing.
    pub checked: AtomicU64,
    /// Every report filed, in filing order, so a test can assert that a
    /// refused filing left none behind.
    pub reports: Mutex<Vec<ipc::Report>>,
    /// What `list_worktrees` answers with. Set by a test, because nothing here
    /// has a repository to read a checkout out of — the derivation is Fleet's
    /// and the route only carries it.
    pub held: Mutex<Vec<ipc::WorktreeHeld>>,
    /// What `list_servers` answers with, narrowed by Manifest. Set by a test.
    pub servers: Mutex<Vec<ipc::ServerState>>,
    /// When set, every call answers with a fault. The stream closing on a
    /// daemon that cannot answer is a behaviour worth a test.
    pub mute: Mutex<bool>,
    /// The one running Check's log `observe_check_output` resolves, by the
    /// name it answers to. Planted by a test, whose reader it drives.
    pub live: Mutex<Option<(String, crate::LiveOutput)>>,
    /// The one merge line Check's log `observe_land_check` resolves, by the
    /// root, branch and Check it answers to. Planted by a test, `live`'s way.
    pub land: Mutex<Option<crate::LandOutput>>,
    /// The limits in force, which the fake's own saves change.
    limits: Mutex<ipc::FleetLimits>,
    /// Every restart asked for, so a route test can assert what arrived.
    builds: Mutex<Vec<ipc::ChangeFleetBuild>>,
    /// The preferences in force, which the fake's own saves change.
    preferences: Mutex<ipc::Preferences>,
    /// settings.json as the fake holds it, which its own saves change.
    settings: Mutex<ipc::SettingsList>,
    /// Every rule a person always-allowed for the repository. Set by a test,
    /// and changed by the fake's own removes — `#836`.
    pub repository_allowed: Mutex<Vec<ipc::AllowedCommandRow>>,
    /// Kit's servers, as the fake's own acts leave them — `#1275`.
    pub kit_servers: Mutex<Vec<ipc::KitServerRow>>,
    /// The one Helm conversation's channel, which a test can offer into.
    pub helm: crate::HelmFeed,
    /// What `observe_helm` answers with as the thread. Set by a test.
    pub helm_thread: Mutex<Vec<ipc::HelmMessage>>,
    /// Every permission question the door put to this daemon, in order.
    pub asked_to_run: Mutex<Vec<ipc::AskingToRun>>,
    /// What a Helm session's last poll was answered, as the door told this
    /// daemon — what `get_helm_debug_info` reports. `#1367`.
    pub helm_polled: Mutex<Option<ipc::EventsSince>>,
    /// The peer port a test says a Helm session holds, and what that session
    /// may call. **Fleet's placement is `fleet::peer`'s and tested there**;
    /// this is what the door does with the answer.
    pub helm_on: Mutex<Option<(u16, fn(&ipc::door::Reachable) -> bool)>>,
    /// The connection each `start_run` reached this daemon on, as
    /// `crate::asking` said it: the port, `None` where there was no peer.
    pub run_asked_on: Mutex<Vec<Option<u16>>>,
    /// Who each redirect that reached this daemon was recorded against.
    pub redirected_by: Mutex<Vec<crate::Redirector>>,
    /// Who each proposal that reached this daemon was recorded against.
    /// `#943`.
    pub proposed_by: Mutex<Vec<crate::Redirector>>,
    /// Every Studio held, starting with [`THE_STUDIO`]. `#1285`.
    pub studios: Mutex<Vec<ipc::Studio>>,
    /// Who each Studio act that says who acted — add a node, propose an edge,
    /// rename — was recorded against, in order.
    pub added_by: Mutex<Vec<crate::Redirector>>,
    /// The repository each `list_checkout_runs` named, in order. `#1288`.
    pub checkout_runs_named: Mutex<Vec<Option<ipc::ManifestId>>>,
    /// The door each retro read came through, as `api::via` named it.
    pub read_via: Mutex<Vec<Option<ipc::Via>>>,
}

impl FakeDaemon {
    pub fn new(events: Broadcaster) -> FakeDaemon {
        FakeDaemon {
            jobs: Mutex::new(Vec::new()),
            unreadable: Mutex::new(Vec::new()),
            events,
            turns: Turns::new(),
            history: Mutex::new(Vec::new()),
            skipped: Mutex::new(0),
            minted: AtomicU64::new(0),
            submitted: Mutex::new(Vec::new()),
            declared: Mutex::new(Vec::new()),
            requested: Mutex::new(Vec::new()),
            asked: Mutex::new(Vec::new()),
            dispatched: Mutex::new(Vec::new()),
            checked: AtomicU64::new(0),
            reports: Mutex::new(Vec::new()),
            held: Mutex::new(Vec::new()),
            servers: Mutex::new(Vec::new()),
            mute: Mutex::new(false),
            live: Mutex::new(None),
            land: Mutex::new(None),
            limits: Mutex::new(shapes::limits()),
            builds: Mutex::new(Vec::new()),
            preferences: Mutex::new(shapes::preferences()),
            settings: Mutex::new(settings::settings()),
            repository_allowed: Mutex::new(Vec::new()),
            kit_servers: Mutex::new(Vec::new()),
            helm: crate::HelmFeed::new(),
            helm_thread: Mutex::new(Vec::new()),
            asked_to_run: Mutex::new(Vec::new()),
            helm_polled: Mutex::new(None),
            helm_on: Mutex::new(None),
            run_asked_on: Mutex::new(Vec::new()),
            redirected_by: Mutex::new(Vec::new()),
            proposed_by: Mutex::new(Vec::new()),
            studios: Mutex::new(vec![studios::the_studio()]),
            added_by: Mutex::new(Vec::new()),
            checkout_runs_named: Mutex::new(Vec::new()),
            read_via: Mutex::new(Vec::new()),
        }
    }

    /// A row the store could not read back, which the list must still carry.
    pub fn with_unreadable(self, fault: &str) -> FakeDaemon {
        self.unreadable
            .lock()
            .expect("not poisoned")
            .push(UnreadableJob {
                job_id: None,
                fault: fault.to_string(),
            });
        self
    }

    /// A Drone writing this Job's rows. Dropping what comes back ends it, the
    /// same as a Drone exiting under Fleet.
    pub fn dispatching(&self, job_id: &JobId) -> Feed {
        self.turns.feeding(job_id)
    }

    /// The Job's channel, as Fleet's roster holds it from admission to the
    /// Job's end. Dropping it, with no Drone feeding, ends the watching.
    pub fn admitted(&self, job_id: &JobId) -> crate::Channel {
        self.turns.opening(job_id)
    }

    fn fault(&self, message: &str) -> Refusal {
        Refusal::Fault(ipc::WireError::raised("fake.mute", message, run_id()))
    }

    fn no_such_job(&self, job_id: &JobId) -> Refusal {
        Refusal::NoSuchJob(
            ipc::WireError::raised("fake.no_such_job", "no Job by that id", run_id())
                .about_job(job_id.clone()),
        )
    }

    /// Move a Job, publish the transition, and answer with where it now is.
    fn move_to(
        &self,
        job_id: &JobId,
        from: &str,
        to: &str,
        actor: &str,
    ) -> Result<JobSummary, Refusal> {
        let mut jobs = self.jobs.lock().expect("not poisoned");
        let Some(job) = jobs.iter_mut().find(|job| job.id == *job_id) else {
            return Err(self.no_such_job(job_id));
        };
        if job.status.as_wire() != from {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                "fake.illegal_move",
                format!("a Job at {} does not go to {to}", job.status.as_wire()),
                run_id(),
            )));
        }
        job.status = status(to);
        job.reason = None;
        let moved = job.clone();
        self.events.publish(Event::JobStateChanged(JobStateChanged {
            job_id: job_id.clone(),
            from: status(from),
            to: status(to),
            reason: None,
            actor: Actor::from_wire(actor).expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }));
        Ok(moved)
    }
}

impl FakeDaemon {
    /// Every Drone tool refuses on the one thing this daemon can see: nothing
    /// is being worked. **One reading, five callers** — five copies of it drift
    /// into five different sentences about one condition.
    fn while_working(&self, about: &str) -> Result<(), NotRecorded> {
        let running = self
            .jobs
            .lock()
            .expect("not poisoned")
            .iter()
            .any(|job| job.status.as_wire() == "running");
        match running {
            true => Ok(()),
            false => Err(NotRecorded {
                because: format!("no Job is being worked, so there is no {about}"),
            }),
        }
    }
}

/// A Job already running, so the Drone kill has something to kill.
pub fn running(daemon: &FakeDaemon, id: &str) {
    at(daemon, id, "running");
}

/// A running Job another Manifest owns, for a case about scope.
pub fn owned_by(daemon: &FakeDaemon, id: &str, handle: &str, manifest_id: &str) {
    let mut job = shapes::job_at(id, "running");
    job.handle = handle.to_string();
    job.owner_manifest_id = ipc::ManifestId::carried(manifest_id);
    daemon.jobs.lock().expect("not poisoned").push(job);
}

/// A Job put straight into the record at any status the registry has, without
/// a transition to get it there. **Putting it there is this side's**; the row
/// it puts is [`shapes::job_at`].
pub fn at(daemon: &FakeDaemon, id: &str, spelling: &str) {
    daemon
        .jobs
        .lock()
        .expect("not poisoned")
        .push(shapes::job_at(id, spelling));
}
