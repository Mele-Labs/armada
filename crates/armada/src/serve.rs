//! `armada serve` — Fleet, started by hand, serving no repository until one is added.
//!
//! # The order is the specification
//!
//! 1. Read any runtime file there: refuse over a live Fleet, replace a stale one.
//! 2. Read the folder given, if one was, as an add reads it, and **refuse
//!    before taking anything.** A folder found wrong after the bind costs a
//!    port and a runtime file, both given back. None given is none served: the
//!    working directory is not a repository by default.
//! 3. Open the store and **claim the listener's port out of it** —
//!    `fleet::listener`. After the refusal above, for step 2's reason.
//! 4. Bind the listener: loopback, at the port just claimed.
//! 5. Write the runtime file carrying the port **read back from the bound
//!    listener**. Publishing a number nobody listens on gives Bridge a socket
//!    that refuses and no way to tell that from a wedged Fleet.
//! 6. Assemble a Fleet serving nothing, on the store already open. Serve the
//!    folder given, then every remembered one.
//! 7. Serve the router, and reconcile every served repository against what this
//!    process can see in a task of its own. **Reads and health answer while it
//!    runs; a command waits for it**, because it moves Jobs and a command
//!    between two of its moves would race them. When it ends, the turn loop
//!    starts on the same `Arc` the router holds, since reconciliation can admit
//!    a queued Job that needs turning whether or not anything ever connects,
//!    and then commands are let through. A reconciliation that fails stops the
//!    server.
//! 8. Wait to be stopped. A stop during step 7 ends its task where it stands,
//!    as a crash would. The port goes back and the file's guard removes it.
//!
//! **`exit 0` on a permanent refusal is deliberately not implemented.**
//! `docs/concepts/fleet.md` requires it of a supervised Fleet; started by hand,
//! a refusal exiting `0` reads at the terminal as success. Starting over a live
//! Fleet is not a refusal — it exits `0` and names the pid holding the port.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapters::{ActionsWorkflows, GitVcs, HeadlessAgent, IssueLookup};
use adapter_traits::Model;
use config::settings::{self as keys, Key, Resolved};
use config::Roster;
use fleet::permitting::{self, PermissionHold, UnansweredAskLimit};
use fleet::repositories::Locating;
use fleet::runtime::{self, Presence, RuntimeFile, Staleness};
use fleet::{
    detect_ceiling, Allowance, AskedRuns, BindConnectProbe, Bytes, CheckBudget, Clock,
    CommandBudget, Concurrency, Fittings, Fleet, Headroom, Host, JudgeBudget, Liveness, Micros,
    Mint, Noticing, Polling, PortRange, Reclaiming, Spare, StepNorms, SystemClock, TheMachine,
    TheVolume, UlidMint,
};
use ipc::ProtocolId;
use store::Store;

use crate::agent::agent_binary_named_by;
use crate::AGENT_BINARY;

/// The store, beside the runtime file rather than inside the repository.
///
/// Job history is machine state, not repository setup: a database under
/// `.armada/` would be a file every repository Armada is pointed at has to
/// remember to ignore, and two checkouts of one repository would each have
/// their own.
pub const STORE_FILE: &str = "armada.db";

/// The strict MCP configuration every Drone is spawned against.
///
/// Outside the repository, deliberately — a Drone that could read its own MCP
/// configuration could read the address it reports evidence to, and one that
/// could write it could name a different server.
pub const MCP_FILE: &str = "mcp.json";

/// Every number Fleet runs by is a setting now, read out of settings.json
/// against `config::settings`' table: what ships, and the argument for each
/// number, sit beside its entry there. **Two readings of the file are handed
/// in.** A live dial goes in as what ships on this machine — the defaults and
/// any variable set — and Fleet puts what the file chose over it at assembly
/// and at every save or hand edit (`fleet::tuning`). A value read once at start
/// is the file's at this start, and a change to it waits for the next one.
fn whole(n: i64) -> u32 {
    u32::try_from(n).unwrap_or(0)
}

fn days(n: i64) -> Duration {
    Duration::from_secs(u64::from(whole(n)) * 24 * 60 * 60)
}

/// The range every port claim on this machine comes from: the base and the
/// granule settings.json held at start, the ceiling the platform's.
fn port_range_of(at_start: &Resolved) -> PortRange {
    PortRange::of(
        u16::try_from(at_start.get(keys::PORT_RANGE_BASE)).unwrap_or(40_000),
        detect_ceiling(),
        u16::try_from(at_start.get(keys::PORT_BLOCK_GRANULE)).unwrap_or(8),
    )
}

/// A model the table holds, at the effort another setting names.
fn model_at(name: String, effort: String) -> Result<Model, Box<dyn Error>> {
    Model::named_at(&name, fleet::tuning::effort_of(&effort)).map_err(|refused| refused.said().into())
}

/// Serve until a signal says stop, adding `repository` first where one is given.
///
/// **The one argument is added, as `add_repository` would add it**, and kept
/// for the launchers that name one. Without it Fleet serves what it remembers,
/// which on a fresh install is nothing.
pub async fn serve(repository: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let path = runtime::machine_path()?;

    let presence = runtime::read(&path)?;
    if let Presence::Running(live) = &presence {
        // Not a timeout and not a guess: the pid in that file is held by the
        // process that wrote it, so there is a Fleet, and starting a second one
        // would leave two writers over one store.
        eprintln!(
            "Fleet is already running as pid {} on port {}.",
            live.pid, live.port
        );
        return Ok(());
    }

    let machine = path
        .parent()
        .expect("the runtime file has a directory")
        .to_path_buf();
    // settings.json, read for what Fleet is started as. Read again once the
    // store is open, in case the first start carries the store's rows into it.
    let early = crate::settings::in_machine(&machine);
    // Before the port and before the runtime file. A Manifest Armada will not
    // have is a refusal that costs nothing to discover here and costs a bound
    // socket and a published file to discover later.
    let machine_facts = machine_facts(early.at_start())?;
    // The roster workflows are checked against is the one the picker offers:
    // two lists would be two answers to "is this a model this machine has".
    let kit = crate::setup::kit(std::path::Path::new(&machine_facts.home))?;
    let roster = Roster::of(&machine_facts.models.models);
    // Reads a folder a person adds, and holds every served `armada.yml`'s watch.
    let kit_home = kit.to_string_lossy().to_string();
    let locator = Arc::new(crate::locating::Locator::at(&machine, kit, roster));
    let given = match repository {
        Some(folder) => Some(locator.located(&folder).map_err(|why| why.to_string())?),
        None => None,
    };
    println!(
        "{} — model {}",
        match &given {
            Some(located) => format!("adding {}", located.root),
            None => String::from("no repository given"),
        },
        machine_facts.models.default
    );

    let vacancy = presence
        .vacancy(&path)
        .expect("a presence that is not running yields a vacancy");
    match vacancy.replacing() {
        Some(Staleness::PidDead) => {
            println!("replacing a runtime file left by a Fleet that did not exit cleanly");
        }
        Some(Staleness::PidHeldByAnother { .. }) => {
            println!("replacing a runtime file whose pid now belongs to something else");
        }
        None => {}
    }

    // The store, opened here rather than inside `assemble`: the port this
    // process binds is claimed out of it, and a claim needs a store to be made
    // in. Everything above this line refuses without having taken anything.
    std::fs::create_dir_all(&machine)?;
    let mut store = Store::open(&machine.join(STORE_FILE))?;
    if !store.unknown_migrations().is_empty() {
        println!(
            "the database has migrations this build does not know, all additive, so it is opened: {}",
            store.unknown_migrations().join(", ")
        );
    }

    // The store's saved limits and preferences, carried into settings.json at
    // the first start that finds no file, and never read again after.
    match fleet::settings::migrated(&store, &machine.join(crate::settings::SETTINGS_FILE)) {
        Ok(moved) if moved.is_empty() => {}
        Ok(moved) => println!("settings.json written from the store's saved rows: {}", moved.join(", ")),
        Err(why) => eprintln!("{why}"),
    }
    let settings = crate::settings::in_machine(&machine);
    if let Some(refused) = settings.refused() {
        eprintln!("{} was not taken: {refused}", settings.path().display());
        eprintln!("Fleet starts on the shipped settings; correct the file and save again");
    }
    let at_start = settings.at_start().clone();
    let turn_interval = Duration::from_millis(u64::from(whole(at_start.get(keys::TURN_INTERVAL_MS))));
    let sweep_interval = at_start.get(keys::BUILD_SWEEP_SECONDS);
    let trim = adapters::leasing::Trim {
        older_than: days(at_start.get(keys::SLOT_BUILD_TRIM_AFTER_DAYS)),
        ceiling_bytes: u64::from(whole(at_start.get(keys::SLOT_BUILD_CEILING_GIB))) * 1024 * 1024 * 1024,
    };

    // One range for every claim on this machine — a Job's span, the main
    // checkout's, and this one. Its ceiling is detected from the platform's
    // ephemeral floor, so nothing here hands out a port the kernel will also
    // assign. See `fleet::ports`.
    let port_range = port_range_of(&at_start);
    let claimed = fleet::claimed_listener_port(
        &mut store,
        port_range,
        &BindConnectProbe,
        SystemClock::new().now(),
    )?;

    // Bound at the port just claimed. The port written into the file is still
    // read back from the listener rather than taken from the claim, so it is a
    // port something is listening on by construction.
    let listener = tokio::net::TcpListener::bind(runtime::listener_address(claimed)).await?;
    let bound = listener.local_addr()?;

    let published = RuntimeFile::publish(vacancy, bound.port(), ProtocolId::current())?;
    println!(
        "Fleet running: pid {}, port {}, protocol {} — {}",
        published.file().pid,
        published.file().port,
        published.file().protocol_id,
        published.path().display()
    );

    // Two things need this Fleet and both get it. The router serves it and the
    // loop below turns it; a Fleet only one of them could hold would be either
    // unserved or — as it was — dispatched and never settled.
    let fleet = assemble(
        &machine,
        store,
        bound.port(),
        machine_facts,
        kit_home,
        Arc::clone(&locator) as Arc<dyn Locating>,
        settings,
    )?;
    let fleet = Arc::new(fleet);
    // A save or a hand edit to settings.json, handed to Fleet once it settles.
    let _settings_watch = crate::settings::watched_by(
        Arc::clone(&fleet),
        machine.join(crate::settings::SETTINGS_FILE),
    );
    // Before any repository is served, so every watch hands its re-read to Fleet.
    locator.bind(&fleet);

    // **Nothing runs until this has.** A Job the store says was running is
    // asked about: a Drone is spawned into a session of its own, so it outlives
    // the Fleet that started it and may still be working. What is gone is
    // `interrupted`; what is still there is adopted, and the Job carries on
    // with a Drone nothing can speak to.
    // The folder given, then the repositories added before this start, so their
    // Jobs are reconciled too. Records move and watches start as each is served.
    if let Some(located) = given {
        if let Err(why) = fleet.served_at_start(located).await {
            eprintln!("  the repository given is not served: {why}");
        }
    }
    for why in fleet.served_again().await {
        eprintln!("  a remembered repository is not served: {why}");
    }
    // **Served before it has reconciled**: reconciliation can take as long as a
    // Job's gate, and the listener answered nothing for all of it. Reads and
    // health answer at once; a command waits on `reconciliation`, so nothing
    // races the Job moves below. `api::Reconciliation`.
    let reconciliation = api::Reconciliation::begun();
    let events = fleet.events();
    // The loops that move Jobs start inside the boot, after `reconcile`, and
    // only then are commands let through.
    let boot = {
        let fleet = Arc::clone(&fleet);
        let events = events.clone();
        let reconciliation = reconciliation.clone();
        async move {
            let reconciled = match fleet.reconcile().await {
                Ok(reconciled) => reconciled,
                Err(why) => return Err(why.to_string()),
            };
            println!(
                "reconciled: {} interrupted, {} restarted, {} adopted, {} repaired, {} unreadable, {} mended{}",
                reconciled.interrupted.len(),
                reconciled.restarted.len(),
                reconciled.adopted.len(),
                reconciled.repaired,
                reconciled.unreadable.len(),
                reconciled.mended.len(),
                match reconciled.admitted.as_slice() {
                    [] => String::new(),
                    admitted => format!(", admitted {}", admitted.len()),
                }
            );
            // **Sessions taken back, not restarted.** A hosted session's agent
            // runs under a keeper that outlives Fleet; reattaching replays what
            // it said while Fleet was down.
            let kept = fleet.reattached_sessions().await;
            if kept > 0 {
                println!("  {kept} hosted session(s) reattached to a keeper that outlived the last Fleet");
            }
            // Said only where it happened: every boot after the one that converted
            // them prints nought, and a line saying so every time would be noise.
            if reconciled.recognised > 0 {
                println!(
                    "  {} Link on a Studio is now the Issue, Pull request or Epic its address names",
                    reconciled.recognised
                );
            }
            for job in &reconciled.adopted {
                // Named rather than counted, because an adopted Drone is a Job whose
                // record has a hole in it: what it did while Fleet was away is not in
                // the transcript and never will be. The Job's own log says how wide.
                eprintln!(
                    "  a Drone outlived the last Fleet and was adopted: {}",
                    job.as_str()
                );
            }
            for job in &reconciled.mended {
                // Named for the same reason: a person reading this Job's own log gets
                // the why, and this line is where an operator watching boot sees that
                // one existed at all.
                eprintln!(
                    "  a Job the old `Agree` arm left stranded was moved to escalated: {}",
                    job.as_str()
                );
            }
            for unreadable in &reconciled.unreadable {
                // Carried out rather than dropped: a short list with nothing saying so
                // is the one answer the store refuses to give.
                eprintln!("  a row would not rebuild: {unreadable}");
            }

            // **Started before commands are let through, not after.**
            // Reconciliation admitted a queued Job on the way out, and that Job
            // is already dispatched: it needs turning whether or not anything
            // ever connects.
            let turning =
                fleet::keep_turning(Arc::clone(&fleet), turn_interval, |why| {
                    // Carried out on its own line, and the loop keeps going: one
                    // turn having failed is not a reason for every later Job to
                    // stop advancing silently.
                    eprintln!("a turn did not complete: {why}");
                });
            println!("turning every {}ms", turn_interval.as_millis());
            // Main and the open pull requests, on the merge notice's interval but not
            // behind a turn, which waits on every Job's Checks.
            fleet::main_ci::keep_reading_main(
                Arc::clone(&fleet),
                turn_interval,
                |why| eprintln!("main was not read: {why}"),
            );
            // Pull requests and issues, for the same reason: a landing is told to its Job
            // without waiting behind a Check.
            fleet::notice_loop::keep_noticing(
                Arc::clone(&fleet),
                turn_interval,
                |why| eprintln!("a pull request or issue was not read: {why}"),
            );
            // A failed Trigger with `repair` on is worked by a Drone off the
            // Job's own path, so the Job carries on while it does.
            fleet::notice_loop::keep_repairing(Arc::clone(&fleet), turn_interval);
            // Each served repository's merge line, published when its hub moves.
            fleet::merge_lines::keep_reading(
                Arc::clone(&fleet),
                events.clone(),
                fleet::merge_lines::EVERY,
            );
            // The mods folder, rescanned so a mod a session wrote reaches Bridge.
            fleet::mods::keep_reading(Arc::clone(&fleet), fleet::mods::EVERY);
            // Stale build output, trimmed in every checkout of each served repository.
            fleet::sweeping::keep_sweeping(
                Arc::clone(&fleet),
                sweep_interval,
                trim,
                {
                    let fleet = Arc::clone(&fleet);
                    move || {
                        let fleet = Arc::clone(&fleet);
                        async move { fleet.piloted_checkouts().await }
                    }
                },
                |said| eprintln!("{said}"),
            );
            // After adoption above, so a Drone taken back is already recorded and
            // only those of a Fleet that is gone are ended.
            fleet::keep_ending_orphans(
                fleet::orphans::EVERY,
                {
                    let fleet = Arc::clone(&fleet);
                    move || {
                        let fleet = Arc::clone(&fleet);
                        async move { fleet.recorded_drone_pids().await }
                    }
                },
                |said| eprintln!("{said}"),
            );
            reconciliation.finished();
            Ok(turning)
        }
    };
    // Each minute's events by kind and Job, to be read against `BACKLOG`. #1759.
    let clock = SystemClock::new();
    api::tally_every(events.clone(), api::TALLY_EVERY, move |tally| {
        eprintln!("{} {tally}", clock.now().as_str());
    });
    let run_id = ipc::RunId::carried(UlidMint::new().ulid().as_str());
    // The reader for a Job's own log, taken from the Fleet before it is handed
    // over. **Nothing else on this side knows where the logs are**, which is
    // why it comes from Fleet rather than from the root resolved above.
    let job_logs = Arc::new(fleet.job_logs());
    // Kept past the move below, for the main checkout's own port span: its
    // release happens here, once, after the turn loop has drained — not from
    // inside a Fleet method the way a Job's own release is, because there is
    // no Job whose transition would carry it. See `fleet::ports`.
    let fleet_for_shutdown = Arc::clone(&fleet);
    let app = api::router(
        api::Served::sharing(fleet, run_id, events)
            .reading(job_logs)
            .reconciling(&reconciliation),
    );
    // The boot's clone is the only one left, so a boot that fails or is ended
    // answers every command waiting on it. Kept here, a held command would
    // block the graceful shutdown that the failure asks for.
    drop(reconciliation);
    println!("serving {} on {bound}", api::SERVED.len());

    // **With connect info**, because a Drone's tool call is attributed by the
    // process on the other end of its connection and `ConnectInfo` is how that
    // peer reaches the handler. See `fleet::peer`.
    let turning =
        crate::booting::serve_while_booting(listener, app, stop_requested(), boot).await?;

    // Between turns, letting the one in flight finish. A stop that returned
    // mid-turn could leave a step moved and its Job not — so this waits, and
    // says it is waiting, because a turn running a Check can hold it for the
    // whole Check budget and a terminal that has gone quiet reads as a wedge.
    // Stopped while still reconciling, the task is ended where it stands: what
    // it leaves half done is what a crash leaves, and the next boot repairs it.
    if let Some(turning) = turning {
        println!("stopping: letting the turn in flight finish");
        turning.stopped().await;
    }

    // After teardown, never before it: the turn in flight has finished, and
    // every server Fleet holds — a Job's or the main checkout's — is stopped
    // here, since nothing after this process could hand one on or stop it.
    // `docs/concepts/fleet.md`, *Servers* — the main checkout's span is held
    // for as long as Fleet runs, and released once, here.
    fleet_for_shutdown.stopped_every_server().await;
    fleet_for_shutdown.released_checkout_ports().await;
    // Fleet's own listener port, given back beside the main checkout's span
    // and at the same moment. A release that does not happen — a crash — is
    // not a port lost: the next start reads the row and takes the port up
    // again once the probe agrees nothing is on it. See `fleet::listener`.
    fleet_for_shutdown.released_listener_port().await;

    // Dropping it removes the file, which is what makes this a clean exit. An
    // exit that skips the drop leaves the file stale, and the next start
    // replaces it — the two halves of the same rule.
    drop(published);
    println!("Fleet stopped");
    Ok(())
}

/// The three things Fleet reads out of its own environment.
///
/// **Read before the bind**, with the repository's setup, so that a machine
/// that is not set up refuses without having taken a port or published a
/// runtime file. They are the only values below that come from the process
/// rather than from a repository or from a constant.
struct MachineFacts {
    /// The operator's home. The agent CLI reads its credentials from it, which
    /// is the confinement's known floor — see `fleet::HostPaths`.
    home: String,
    /// Who the operator is. **The agent CLI will not authenticate without it**,
    /// however readable its credentials are — measured against a live Drone,
    /// where `USER` was the only difference between working and
    /// `Not logged in`. Read here with the others so a machine missing it
    /// refuses before the bind rather than at spawn.
    user: String,
    /// What a Drone's `PATH` is set to. Assembled here, and handed both to the
    /// Drone and to the probe below, so the `PATH` a named binary is looked for
    /// on is the `PATH` it will be run from.
    path: String,
    /// The headless agent CLI. The settings default unless
    /// [`AGENT_BINARY`] names one — **an override, not a requirement.**
    agent: HeadlessAgent,
    /// The models a Job may name, and the one it gets when it names none. The
    /// same shape as `agent`, and the second half of the same missing piece:
    /// until this was read, a proposal with no model was stored and died at
    /// dispatch as "no model was named".
    models: ipc::ModelChoices,
}

fn machine_facts(settings: &Resolved) -> Result<MachineFacts, Box<dyn Error>> {
    let home = std::env::var("HOME")?;
    let user = std::env::var("USER")?;
    let path = drone_path_over(
        &home,
        &std::env::var("PATH").unwrap_or_default(),
        &settings.get(keys::HARNESS_DRONE_PATH),
    );
    // Unnamed is the ordinary case and the adapter's default answers it. Named
    // and wrong is somebody having tried to point Fleet at something, and is
    // refused here — before the port, before the runtime file.
    let binary = config::settings::entry(keys::HARNESS_BINARY_PATH.name()).expect("in the table");
    let named = match (settings.overridden_by_env(binary), settings.chosen(keys::HARNESS_BINARY_PATH)) {
        (Some(_), _) => Some((settings.get(keys::HARNESS_BINARY_PATH), AGENT_BINARY)),
        (None, Some(chosen)) if !chosen.trim().is_empty() => Some((chosen, "harness.binaryPath in settings.json")),
        _ => None,
    };
    let agent = agent_binary_named_by(named, &path)?;
    // Not probed. Whether a model name is one this account may use is a
    // question only the vendor answers, and asking it would put a network call
    // before the bind.
    let models = ipc::ModelChoices {
        models: settings.models(),
        default: settings.get(keys::MODELS_DEFAULT),
        harnesses: settings.options_of(config::settings::entry(keys::HARNESS_AGENT.name()).expect("in the table")),
    };
    Ok(MachineFacts {
        home,
        user,
        path,
        agent,
        models,
    })
}

/// The Fleet `serve` assembles.
type Served = Fleet<HeadlessAgent, GitVcs, GitVcs>;

/// Everything Fleet is made of, resolved once, here.
///
/// The clock, the mint, the two host paths and the agent binary are all
/// resolved at this one point and handed down. Nothing below reads its own
/// inputs from the process — which is what lets `fleet` be driven by a test
/// that plants a fixed instant and a countable id.
fn assemble(
    machine: &std::path::Path,
    store: Store,
    port: u16,
    facts: MachineFacts,
    kit_home: String,
    locator: Arc<dyn Locating>,
    settings: fleet::settings::MachineSettings,
) -> Result<Served, Box<dyn Error>> {
    // What ships on this machine, for every live dial; the file's own values
    // are Fleet's to put over them. `at_start` is for what is read once.
    let shipped = settings.shipped();
    let at_start = settings.at_start().clone();
    let port_range = port_range_of(&at_start);
    let MachineFacts {
        home,
        user,
        path,
        agent,
        models,
    } = facts;

    // The machine directory was created before the store was opened in it.
    // Where Fleet keeps its own copy of a Job's attachments, outside every
    // worktree — `drafted()` writes here at proposal time and `dispatch`
    // copies from here into the worktree a Drone can see.
    let attachments_dir = machine.join("attachments");
    std::fs::create_dir_all(&attachments_dir)?;
    let keepers_dir = machine.join("sessions");
    std::fs::create_dir_all(&keepers_dir)?;

    // A Studio's frames, beside the Studio's records rather than inside the
    // database — a screenshot in a row is read on every graph read. `#1290`.
    let studio_frames_dir = machine.join("studios");
    std::fs::create_dir_all(&studio_frames_dir)?;

    // A walk note's frame, one directory per Job, outside every worktree so it
    // outlives the checkout it was taken of. Protocol 23.18.
    let walk_frames_dir = machine.join("walks");
    std::fs::create_dir_all(&walk_frames_dir)?;

    // The mods, one folder each. Made here so a person has somewhere to look; a
    // session that scaffolds one makes it again if it is gone.
    let mods_dir = machine.join("mods");
    std::fs::create_dir_all(&mods_dir)?;

    // The Evidence server alone, for a spawn that cannot name the Manifest it
    // is serving — `fleet::spawning` writes this file's Manifest-resolved
    // sibling for every spawn that can. The path is `api`'s own constant rather
    // than a literal: this address is in no route table a gate rule reads, so
    // the one thing standing between a typo and a Drone that can never report
    // is that the address written here and the address routed there are one
    // value.
    let mcp_config = machine.join(MCP_FILE);
    adapters::the_drones_servers(
        &mcp_config,
        &format!("http://127.0.0.1:{port}{}", api::MCP_PATH),
        &[],
    )?;

    // The Judge and Helm's host both run the program the Drone runs, so a
    // machine that named one through the override names all three — a second
    // variable would let any pair disagree about which binary is installed.
    let judge_binary = agent.program().to_string();
    let judge_model = model_at(shipped.get(keys::MODELS_JUDGE), shipped.get(keys::EFFORT_JUDGE))?;
    let second_opinion_model =
        model_at(shipped.get(keys::MODELS_SECOND_OPINION), shipped.get(keys::EFFORT_JUDGE))?;
    let proposer_model = model_at(shipped.get(keys::MODELS_PROPOSER), shipped.get(keys::EFFORT_PROPOSER))?;
    let retro_model = model_at(shipped.get(keys::MODELS_RETRO), String::new())?;
    let fleet = Fleet::assembled(Fittings {
        store,
        harness: agent,
        vcs: GitVcs::new(),
        work: GitVcs::new(),
        clock: Arc::new(SystemClock::new()),
        mint: Arc::new(UlidMint::new()),
        starting_in: None,
        host: Host {
            path,
            home: home.clone(),
            user,
            mcp_config: mcp_config.to_string_lossy().to_string(),
            attachments_dir: attachments_dir.to_string_lossy().to_string(),
            keepers_dir: keepers_dir.to_string_lossy().to_string(),
            studio_frames_dir: studio_frames_dir.to_string_lossy().to_string(),
            walk_frames_dir: walk_frames_dir.to_string_lossy().to_string(),
            mods_dir: mods_dir.to_string_lossy().to_string(),
            kit_home,
            // `judge_binary`'s reason: the same override reaches Helm's host.
            // `#943`.
            agent_binary: judge_binary.clone(),
            helm_reply_budget: at_start.get(keys::HELM_REPLY_SECONDS),
            helm_effort: fleet::tuning::effort_of(&at_start.get(keys::EFFORT_HELM)),
            // The port the listener actually bound, which is the same one
            // written into `mcp.json` above — one value, so a Drone's
            // connection to the address it was given is the connection Fleet
            // matches its port against. See `fleet::peer`.
            port,
        },
        locating: locator,
        // The setup a person already has, read from their own home to be shown
        // in Kit — `#1491`. **It reads and cannot write**, and nothing it
        // returns can reach a Drone: `adapters::mcp` writes a Drone's servers
        // from `core_model::a_drone_resolves` and from nothing else.
        setup: Arc::new(adapters::ExistingSetup::over(
            adapters::Home::at(&home),
            &home,
        )),
        port_range,
        run_log_retention: days(shipped.get(keys::RUN_LOG_DAYS)),
        helm_authority: match shipped.get(keys::HELM_CAN_ACT) {
            true => fleet::helm::Authority::Acting,
            false => fleet::helm::Authority::ReadOnly,
        },
        helm_session_retention: days(shipped.get(keys::HELM_SESSION_DAYS)),
        helm_ask_hold: fleet::helm::HelmAskHold::of(at_start.get(keys::HELM_ASK_HOLD_SECONDS)),
        // The kernel, because the question is which process holds a socket.
        // `fleet::peer` holds the measurement that chose it over `lsof`.
        peers: Arc::new(fleet::peer::Kernel),
        concurrency: Concurrency::of(whole(shipped.get(keys::DRONES_AT_ONCE)) as usize),
        // The shell, not a platform crate: `fleet::headroom` carries the
        // argument, which is `fleet::process`'s and is about one spelling on
        // both platforms rather than about convenience.
        // The operator's home, for this one bundled reading. A Job's own
        // repository is read at admission, on its own volume — `fleet::admitting`.
        machine: Arc::new(TheMachine::watching(&home)),
        copy_on_write: Arc::new(TheVolume),
        headroom: Headroom::of(
            Spare::percent(whole(shipped.get(keys::MEMORY_SPARE_PERCENT))),
            Bytes::gibibytes(u64::from(whole(shipped.get(keys::DISK_FLOOR_GIB)))),
        ),
        checks_at_once: fleet::ChecksAtOnce::of(whole(shipped.get(keys::CHECKS_AT_ONCE)) as usize),
        check_slots: crate::declared::machine_slots_for_fleet(),
        polling: Polling::every(shipped.get(keys::RESOURCE_POLL_SECONDS)),
        noticing: Noticing::every(shipped.get(keys::MERGE_NOTICE_SECONDS)),
        reclaiming: Reclaiming::every(shipped.get(keys::RECLAIM_SWEEP_SECONDS)),
        allowance: Allowance::of(
            Micros::dollars(u64::from(whole(shipped.get(keys::COST_CAP_DOLLARS_PER_JOB)))),
            u64::from(whole(shipped.get(keys::TURN_CAP_PER_JOB))),
        ),
        budget: CheckBudget::of(shipped.get(keys::CHECK_SECONDS)),
        norms: StepNorms::of(
            whole(shipped.get(keys::TOOL_CALLS_PER_STEP)),
            shipped.get(keys::STEP_WALL_CLOCK_SECONDS),
            shipped.get(keys::STEP_GRACE_SECONDS),
        ),
        liveness: Liveness::of(
            shipped.get(keys::DRONE_QUIET_AFTER_SECONDS),
            whole(shipped.get(keys::DRONE_POKE_LIMIT)),
        ),
        asked_runs: AskedRuns::of(whole(shipped.get(keys::ASKED_RUNS_PER_STEP))),
        fixes: fleet::fixing::Fixes::of(whole(shipped.get(keys::FIXES_PER_STEP))),
        // The same CLI, invoked as a call rather than as a session. The
        // spelling of the model is the adapter's; this crate never learns it.
        judge: Arc::new(HeadlessAgent::at(judge_binary)),
        judge_budget: JudgeBudget::of(shipped.get(keys::JUDGE_SECONDS)),
        proposer_budget: JudgeBudget::of(shipped.get(keys::PROPOSER_SECONDS)),
        command_budget: CommandBudget::of(shipped.get(keys::COMMAND_SECONDS)),
        // Not a setting beside the others: the four minutes is derived from
        // what the agent CLI itself waits, so it stays spelled beside that
        // derivation and this line only names it.
        permission_hold: PermissionHold::of(permitting::HOLD),
        unanswered_ask_limit: UnansweredAskLimit::of(shipped.get(keys::UNANSWERED_ASK_SECONDS)),
        judge_model,
        proposer_model,
        retro_model,
        second_opinion_model,
        // The one link shape resolved before dispatch. See
        // `adapters::IssueLookup` for why it is the only one.
        links: Arc::new(IssueLookup),
        // The one CI provider Scan follows; the rest read as not followed.
        ci_configuration: Arc::new(ActionsWorkflows),
        models,
        events: api::Broadcaster::new(),
        session_quiet: at_start.get(keys::SESSION_QUIET_SECONDS),
        settings,
    });
    Ok(fleet)
}

/// The two per-user directories on a Drone's `PATH`, before the system ones.
///
/// `.cargo/bin` is where a repository's Checks find their toolchain.
/// `.local/bin` is **where the agent CLI's own native installer puts it** — a
/// Drone spawned without it died with *no such file or directory* on a machine
/// where the CLI was installed the ordinary way, because none of the six system
/// directories below is where that installer writes.
const PER_USER_DRONE_PATH: &[&str] = &[".cargo/bin", ".local/bin"];

/// The `PATH` a Drone gets: the per-user directories above, then the standard
/// system locations.
///
/// Assembled rather than inherited. Adding an entry is a deliberate edit here,
/// which is the point — the list is a diff, not a default.
#[cfg(test)]
pub(crate) fn drone_path(home: &str) -> String {
    drone_path_with(home, "")
}

/// [`drone_path_over`] with the system directories settings.json ships.
#[cfg(test)]
pub(crate) fn drone_path_with(home: &str, inherited: &str) -> String {
    let shipped = config::settings::Resolved::default().get(keys::HARNESS_DRONE_PATH);
    drone_path_over(home, inherited, &shipped)
}

/// The per-user directories, Fleet's own `PATH`, then `system` —
/// `harness.dronePath` in settings.json. **The owner's toolchain, as his shell has it** (8 Oct 2026):
/// a Session started without nvm, pnpm or mise on its `PATH` went looking with
/// `find /`, and its context7 and gitnexus servers did not start. He chose that
/// Drones load all his user settings; their `PATH` follows. Fleet's `PATH` is
/// the one `scripts/restart` wrote from his shell. Relative entries and
/// repeats are dropped.
pub(crate) fn drone_path_over(home: &str, inherited: &str, system: &[String]) -> String {
    let mut entries: Vec<String> = PER_USER_DRONE_PATH
        .iter()
        .map(|dir| format!("{home}/{dir}"))
        .collect();
    for dir in inherited.split(':').filter(|dir| dir.starts_with('/')) {
        if !entries.iter().any(|had| had == dir) && !system.iter().any(|one| one == dir) {
            entries.push(dir.to_string());
        }
    }
    entries.extend(system.iter().cloned());
    entries.join(":")
}

/// Wait for either of the two signals that mean stop.
///
/// `SIGTERM` because that is what a supervisor sends, `SIGINT` because that is
/// what the terminal this is started from sends. `SIGKILL` cannot be waited on,
/// which is exactly why the unclean-exit path has to be the one that needs no
/// code.
async fn stop_requested() {
    use tokio::signal::unix::{signal, SignalKind};

    // A signal handler that will not install is a daemon that cannot be asked
    // to stop, which is worse than one that stops now.
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM can be waited on");
    let mut interrupt = signal(SignalKind::interrupt()).expect("SIGINT can be waited on");
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
}
