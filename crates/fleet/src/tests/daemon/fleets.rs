//! A Fleet assembled over fakes, and the [`Fittings`] it is assembled from.
//!
//! **One set of fittings, varied by one field.** Every builder here calls
//! [`fitted_with`] and then changes the single thing its cases are about — the
//! version control, the workflow, the Judge, the mint. A second full set would
//! be a second answer to what a fixture Fleet is, and the two would drift.
//!
//! **A dial that ships is planted at the number that ships; a dial that would
//! make an unrelated case flaky is planted out of reach.** Which of the two
//! each one is, and why, is written on the field in [`fitted_with`] rather than
//! here, because that is where somebody changing it is standing.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Model;
use store::Store;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use super::workflows::{manifest, one, two_steps};
use crate::tests::planted::{Counted, Ticking};
use crate::daemon::{Fittings, Fleet, Host, StartingIn};
use crate::CheckBudget;
use crate::headroom::{Bytes, Headroom, Polling, Spare};
use crate::judging::JudgeBudget;
use crate::noticing::Noticing;
use crate::ports::PortRange;
use crate::slots::Concurrency;
use crate::tests::tmp::TempDir;

/// Everything a Fleet is assembled from, over one temporary directory.
pub fn fittings(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    fitted_with(home, work, FakeHarness::that_listens())
}

pub fn fitted_with(
    home: &TempDir,
    work: FakeWorkProduct,
    harness: FakeHarness,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    fitted_over(home, work, harness, FakeVcs::new())
}

/// The same fittings over any version control: a real one where git's own
/// answer is the subject — `crate::tests::cloning`.
pub fn fitted_over<V>(
    home: &TempDir,
    work: FakeWorkProduct,
    harness: FakeHarness,
    vcs: V,
) -> Fittings<FakeHarness, V, FakeWorkProduct> {
    let root = home.path().to_string_lossy().to_string();
    Fittings {
        store: Store::open(&home.path().join("armada.db")).expect("a store"),
        harness,
        vcs,
        work,
        clock: Arc::new(Ticking::from_nine()),
        mint: Arc::new(Counted::from_one()),
        // The real reader, pointed at the fixture's own home. A fixture that
        // read the machine's would report whoever ran the tests.
        setup: Arc::new(adapters::ExistingSetup::over(
            adapters::Home::at(home.path()),
            &root,
        )),
        starting_in: Some(StartingIn {
            root: root.clone(),
            // Same value as `root` in this fixture; `crate::tests::records`
            // plants a distinct one where the seam is the subject.
            records_root: root.clone(),
            workflows: one(two_steps()),
            left_out: Vec::new(),
            files: Vec::new(),
            manifest: manifest(),
        }),
        host: Host {
            user: String::from("someone"),
            path: "/usr/bin:/bin".to_string(),
            home: root,
            // Under the fixture's own home rather than `/etc`, since `#1275`:
            // a spawn writes this file's Manifest-resolved sibling, and a
            // fixture pointed at a path nothing can write would exercise the
            // fallback in every case rather than the path that ships.
            mcp_config: home.path().join("mcp.json").to_string_lossy().to_string(),
            // Never actually run: a fixture that asks Helm anything swaps this
            // Fleet's host with `hosting_helm_on` first — `crate::tests::helm_conversation`.
            agent_binary: "the-agent".to_string(),
            // A port nothing is listening on. Attribution is planted in these
            // fixtures, so this is the other half of a pair no fake ever
            // matches on.
            port: 12345,
            attachments_dir: home
                .path()
                .join("attachments")
                .to_string_lossy()
                .to_string(),
            keepers_dir: home.path().join("sessions").to_string_lossy().to_string(),
            studio_frames_dir: home.path().join("studios").to_string_lossy().to_string(),
            // Under the fixture's own home, so no case reads or writes a real
            // `~/.armada`.
            kit_home: home.path().join("kit").to_string_lossy().to_string(),
            mods_dir: home.path().join("mods").to_string_lossy().to_string(),
            // The shipped fifteen minutes; the cases about a reply running out
            // plant their own with `answering_within`.
            helm_reply_budget: Duration::from_secs(15 * 60),
            helm_effort: None,
        },
        // Reads no folder: the cases about adding a repository plant their own.
        locating: Arc::new(crate::tests::repositories::Planted::nothing()),
        // Wide, and far from any port a test fixture or this machine's own
        // services are likely to hold — no fixture here declares `ports:`, so
        // nothing claims from it, and the one suite that does plants its own.
        port_range: PortRange::of(41_000, 41_999, 8),
        // The production default. No fixture here asserts on the sweep
        // window, so this is the number that ships rather than one invented
        // for the test.
        run_log_retention: Duration::from_secs(30 * 24 * 60 * 60),
        // The setting's own default. No fixture here asserts on `ReadOnly`.
        helm_authority: crate::helm::Authority::Acting,
        // Two seconds, not the shipped five minutes: a case about nobody
        // answering has to outlive the hold, and `HelmAskHold` is a fitting
        // for exactly that.
        helm_ask_hold: crate::helm::HelmAskHold::of(Duration::from_secs(2)),
        // Nothing to place. A fake harness opens no sockets, so a fixture that
        // answered otherwise would be asserting against the machine rather than
        // against Fleet. `crate::tests::peer` plants one where the subject is
        // attribution.
        peers: Arc::new(crate::tests::peer::TheOnlyDrone),
        // **One, so every fixture but the concurrency cases behaves exactly as
        // it did when there was one slot.** A bound of two here would make every
        // test that queues a second Job assert a different thing than it was
        // written to.
        concurrency: Concurrency::of(1),
        // A machine with plenty of everything, so no fixture but `headroom`'s
        // own is ever held back by one. Reading the real machine here would
        // make every test in this crate pass or fail on what else is running.
        machine: Arc::new(crate::tests::headroom::Plentiful),
        // The production threshold, so the cases that trip it trip the one that
        // ships. See `armada::serve::PROVISIONAL_HEADROOM`.
        headroom: Headroom::of(Spare::percent(15), Bytes::gibibytes(10)),
        // The number that ships, so a gate here runs its Checks as a real one
        // does; the machine is `Plentiful`, so none is held back for room.
        checks_at_once: crate::ChecksAtOnce::of(4),
        check_slots: None,
        // No interval, so a fixture that moves the machine sees it move. The
        // one case about the interval sets its own.
        polling: Polling::every(Duration::ZERO),
        // Never due, so no test asks a fake forge anything unless it says to.
        noticing: Noticing::every(Duration::from_secs(86_400)),
        budget: CheckBudget::of(Duration::from_secs(5)),
        // A Judge that fails every call, because no step in these fixtures
        // declares a criterion. One that answered would let a cold-by-default
        // regression pass unseen.
        judge: Arc::new(FakeJudge::that_fails("a Judge that should never be asked")),
        judge_budget: JudgeBudget::of(Duration::from_secs(5)),
        // The same five seconds. A suite's proposals answer in milliseconds, so
        // the split that matters in production — a watched call outliving an
        // unwatched one — buys a test nothing but a slower failure when one
        // hangs.
        proposer_budget: JudgeBudget::of(Duration::from_secs(5)),
        // The same five seconds, for `judge_budget`'s reason: a fixture's own
        // commands answer in milliseconds, so this only slows a genuine hang's
        // failure rather than a passing suite.
        judge_model: Model::named("the-cheap-model").expect("a model name"),
        proposer_model: Model::named("the-cheap-model").expect("a model name"),
        retro_model: Model::named("the-retro-model").expect("a model name"),
        // Resolves no link, lists nothing and takes every write, so every
        // fixture but `proposing`'s own behaves exactly as it did before this
        // seam existed. The cases about it, and about writing back, plant their
        // own to read what was written.
        trackers: Arc::new(testkit::FakeTrackers::empty()),
        // The shipped reader: it only reads files, so a fixture runs what ships.
        ci_configuration: Arc::new(adapters::ActionsWorkflows),
        // Planted, not read. The composition root resolves these from the
        // environment and the adapter; a test that read the same sources would
        // be asserting against a machine rather than against Fleet.
        models: ipc::ModelChoices {
            models: vec!["a-model".to_string(), "another-model".to_string()],
            default: "a-model".to_string(),
            session: "a-model".to_string(),
            harnesses: vec!["a-harness".to_string()],
            offered: Vec::new(),
            read: ipc::ModelsRead::default(),
        },
        events: api::Broadcaster::new(),
        // Beside the store, and absent until a case saves or writes one.
        settings: crate::settings::MachineSettings::at(
            crate::tests::settings_json::settings_file_in(home),
            crate::tests::settings_json::supplied(),
            config::settings::Env::none(),
        ),
        // Reaches nothing: a forge nobody types a code for, and a store that
        // enrols into the fixture's own home. `crate::tests::signing_in` plants
        // its own where the subject is a sign-in.
        signing_in: crate::signing_in::SigningIn::new(
            home.path(),
            Arc::new(testkit::FakeSignIn::new()),
            Arc::new(crate::tests::signing_in::Enrolled::new()),
            Arc::new(Ticking::from_nine()),
        ),
        // Reaches nothing: a switch that is off never asks. `crate::tests::red_claim`
        // plants a store where the subject is a claim.
        reaching: Arc::new(crate::tests::red_claim::Planted::signed_out()),
        session_quiet: crate::session_host::SHIPPED_QUIET_TIMEOUT,
        // A real sender, reached only by a case that sends to a socket of its own, and cookies held
        // in memory: no fixture touches a keychain.
        requests: crate::requesting::Requesting {
            sender: Arc::new(adapters::HttpRequests::within(Duration::from_secs(5))),
            cookies: Arc::new(crate::tests::requesting::Held::default()),
        },
    }
}

/// A Fleet whose Drone holds its input open, so the gate has something to speak
/// to when a step advances.
pub fn a_fleet(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    Fleet::assembled(fittings(home, work))
}

impl<H, V, W> Fittings<H, V, W> {
    /// The repository a fixture starts in, for a case that varies it.
    pub fn starting(&mut self) -> &mut StartingIn {
        self.starting_in
            .as_mut()
            .expect("a fixture starts in a repository")
    }
}
