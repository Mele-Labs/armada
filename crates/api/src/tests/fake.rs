//! A daemon that is not Fleet.
//!
//! This exists to answer the operations Fleet serves so the transport can be
//! exercised, and it is written without `core-model` on purpose: if a fake
//! daemon can be built out of `ipc` alone, so can a Bridge.
//!
//! **This file is the record.** The answers are in [`shapes`](super::shapes)
//! and the surfaces are the modules below, one per trait.

mod accounts;
mod adversarial_review;
mod app_runs;
mod authoring;
mod autopilots;
mod clashing;
mod commands;
mod conversing;
mod hosted_sessions;
mod issues;
mod logbook;
mod mods;
mod projects;
mod settings;
mod shared_home;
mod pull_requests;
mod queries;
mod rehearsing;
mod requests;
mod retros;
mod sessions;
mod studios;
mod teaching;
mod watches;

pub use accounts::profile;
pub use app_runs::THE_RUN;
pub use conversing::SERVED_MANIFEST;
pub use studios::{the_studio, THE_FRAME, THE_STUDIO};
pub use watches::THE_WATCH;

use std::sync::Mutex;

use super::shapes;
use super::shapes::run_id;
use crate::{Broadcaster, Refusal};

/// The one repository the fake serves where an operation names one by root.
pub const SERVED_ROOT: &str = "/work/shop";

/// Fleet's refusal for any other root, code and status alike.
fn served_root(repository: &str) -> Result<(), Refusal> {
    match repository == SERVED_ROOT {
        true => Ok(()),
        false => Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.run_memory_repository_not_served",
            format!("Fleet does not serve {repository}"),
            run_id(),
        ))),
    }
}

pub struct FakeDaemon {
    /// The row every Session act answers with where a test sets one, in place of a thin echo.
    pub session_row: Mutex<Option<ipc::SessionRecord>>,
    /// The `seen` each `record_plan` was handed, for a test to read after the router has the daemon.
    pub plans_seen: std::sync::Arc<Mutex<Vec<bool>>>,
    /// What `list_servers` answers with, narrowed by Manifest. Set by a test.
    pub servers: Mutex<Vec<ipc::ServerState>>,
    /// When set, every call answers with a fault. The stream closing on a
    /// daemon that cannot answer is a behaviour worth a test.
    pub mute: Mutex<bool>,
    /// The one merge line Check's log `observe_land_check` resolves, by the
    /// root, branch and Check it answers to. Planted by a test.
    pub land: Mutex<Option<crate::LandOutput>>,
    /// The limits in force, which the fake's own saves change.
    limits: Mutex<ipc::FleetLimits>,
    /// Every restart asked for, so a route test can assert what arrived.
    builds: Mutex<Vec<ipc::ChangeFleetBuild>>,
    /// The preferences in force, which the fake's own saves change.
    preferences: Mutex<ipc::Preferences>,
    /// settings.json as the fake holds it, which its own saves change.
    settings: Mutex<ipc::SettingsList>,
    /// The profiles the fake holds, which its own sign-ins and sign-outs change.
    pub accounts: Mutex<ipc::Accounts>,
    /// Every rule a person always-allowed for the repository. Set by a test,
    /// and changed by the fake's own removes — `#836`.
    pub repository_allowed: Mutex<Vec<ipc::AllowedCommandRow>>,
    /// Kit's servers, as the fake's own acts leave them — `#1275`.
    pub kit_servers: Mutex<Vec<ipc::KitServerRow>>,
    /// Every message `ask_helm` was given, in order. The fake's Helm Session exists once there is
    /// one, and `start_helm_fresh` forgets them.
    pub helm_asked: Mutex<Vec<ipc::AskHelm>>,
    /// What `tell_session` and `record_thought_sent` were given, in order, as the words and where.
    pub helm_sent: Mutex<Vec<(String, ipc::ThoughtTarget)>>,
    /// Every permission question the door put to this daemon, in order.
    pub asked_to_run: Mutex<Vec<ipc::AskingToRun>>,
    /// What a Helm session's last poll was answered, as the door told this
    /// daemon — what `get_helm_debug_info` reports. `#1367`.
    pub helm_polled: Mutex<Option<ipc::EventsSince>>,
    /// The peer port a test says a Helm session holds, and what that session
    /// may call. **Fleet's placement is `fleet::peer`'s and tested there**;
    /// this is what the door does with the answer.
    pub helm_on: Mutex<Option<(u16, fn(&ipc::door::Reachable) -> bool)>>,
    /// The peer port a test says a session Fleet hosts for Bridge holds.
    pub session_on: Mutex<Option<u16>>,
    /// Every Studio held, starting with [`THE_STUDIO`]. `#1285`.
    pub studios: Mutex<Vec<ipc::Studio>>,
    /// Who each Studio act that says who acted — add a node, propose an edge,
    /// rename — was recorded against, in order.
    pub added_by: Mutex<Vec<crate::Redirector>>,
    /// The repository each `list_checkout_runs` named, in order. `#1288`.
    pub checkout_runs_named: Mutex<Vec<Option<ipc::ManifestId>>>,
    /// The door each retro read came through, as `api::via` named it.
    pub read_via: Mutex<Vec<Option<ipc::Via>>>,
    /// What `list_sessions` answers with, narrowed by the state asked. Set by a test.
    pub sessions_held: Mutex<Vec<ipc::SessionRecord>>,
    /// The state each `list_sessions` was asked for, in order.
    pub sessions_asked: Mutex<Vec<Option<ipc::SessionState>>>,
    /// Every message `send_session_message_aiming` was handed, with the aim beside it, in order.
    pub messages_aimed: Mutex<Vec<(ipc::SendSessionMessage, Option<ipc::ModAim>)>>,
}

impl FakeDaemon {
    pub fn new(_events: Broadcaster) -> FakeDaemon {
        FakeDaemon {
            session_row: Mutex::new(None),
            plans_seen: Default::default(),
            servers: Mutex::new(Vec::new()),
            mute: Mutex::new(false),
            land: Mutex::new(None),
            limits: Mutex::new(shapes::limits()),
            builds: Mutex::new(Vec::new()),
            preferences: Mutex::new(shapes::preferences()),
            settings: Mutex::new(settings::settings()),
            accounts: Mutex::new(accounts::signed_out()),
            repository_allowed: Mutex::new(Vec::new()),
            kit_servers: Mutex::new(Vec::new()),
            helm_asked: Mutex::new(Vec::new()),
            helm_sent: Mutex::new(Vec::new()),
            asked_to_run: Mutex::new(Vec::new()),
            helm_polled: Mutex::new(None),
            helm_on: Mutex::new(None),
            session_on: Mutex::new(None),
            studios: Mutex::new(vec![studios::the_studio()]),
            added_by: Mutex::new(Vec::new()),
            checkout_runs_named: Mutex::new(Vec::new()),
            read_via: Mutex::new(Vec::new()),
            sessions_held: Mutex::new(Vec::new()),
            sessions_asked: Mutex::new(Vec::new()),
            messages_aimed: Mutex::new(Vec::new()),
        }
    }

    fn fault(&self, message: &str) -> Refusal {
        Refusal::Fault(ipc::WireError::raised("fake.mute", message, run_id()))
    }
}

