//! The daemon: scheduling, Drone lifecycle, worktrees, delivery, and the
//! Evidence MCP a Drone reports through.
//!
//! Each module says what it is for; there is no second copy of that here,
//! because the tour this replaces went stale a milestone at a time. What
//! follows binds the crate rather than one file.
//!
//! **Fleet is the only writer of Job state**, and it reaches an agent only
//! through `adapter-traits` — never a vendor CLI directly, anywhere.
//!
//! **The OS lifecycle is its own module, orthogonal to scheduling.** v1 mixed
//! the two and its `schedule.rs` alone reached 2,929 lines.
//!
//! **The harness renders and Fleet starts.** Nothing in `adapters` spawns, so
//! [`Detached`](detach::Detached) is the only way a process begins here, and
//! every confinement property is a value a test reads rather than a process a
//! test runs.
//!
//! **Fleet stops a Drone only at a cap.** Escalation pauses one with its
//! worktree held, and killing is otherwise a person's act — but a Drone
//! confirmed thrashing spends without converging, so Fleet ends that one
//! itself. One that has merely gone quiet is not that case and
//! [`silence`](mod@silence) does not end it: it costs nothing, and holding it
//! leaves a person a worktree to redispatch onto.

pub mod admitting;
pub mod adopting;
pub mod adrift;
/// What a person changes on a proposal, and the Job it leaves at the press.
/// Spike 022, slice 4.
/// Where an approved Job lands, set once after its approval. 23.22.
pub mod aiming;
pub mod allowance;
/// A form's edits to `armada.yml`, placed a key at a time and written through
/// `editing::save`.
mod amending;
/// What Fleet does with a proposal a person left: keeps, releases, lands and
/// serves it. Spike 022, slice 4.
mod approved;
pub mod approving;
pub mod asked;
pub mod asked_run;
pub mod asking;
/// A red Check in a merge line turn, asked of the base before the branch is blamed.
mod asking_the_base;
pub mod at_step;
/// The four narrowings of the board, and the one rule each is.
mod attention;
/// Saving a workflow definition, and reading the workflow folders again.
mod authoring;
pub mod basing;
/// A Job this boot interrupted has its step restarted.
mod boot_restart;
mod boundary;
pub mod briefing;
/// Racing a plain command's work against [`budget::CommandBudget`],
/// split out of `commanding` at the 900-line refusal, `#897`.
mod budget;
/// A redispatch, drawn on the Studios that dispatched the Job it replaced.
mod carrying_on;
mod check_history;
mod check_output;
mod checking;
mod checkouts;
/// One Manifest as Fleet resolved it, for the caller asking what a Job here
/// will be held to.
mod clearing;
pub mod clock;
/// `api::Commands`, implemented over a real Fleet — the write half of the seam
/// `serving` holds the read half of. Three traits, three impl blocks, three
/// files, and no delegating signature between them.
pub mod commanding;
mod configured;
/// Whether a gate's red is the work's or the machine's, before it is ruled on.
mod confirming;
/// What an upstream's terminal status does to the Job waiting behind it — the
/// one place a dependency edge is weighed, for both admission and the Board.
pub mod conflict_resolution;
pub mod converging;
mod coupling;
/// A Job's Drones beside the one it keeps. Spike 022, slice 5.
pub mod crew;
pub mod crossing;
pub mod currency;
pub mod daemon;
pub mod delivery;
pub mod detach;
/// A person picks comments off a pull request, and they reach a Drone as its
/// opening brief. **The one place text somebody outside this machine wrote
/// enters a prompt.**
mod dismissing;
pub mod dispatch;
mod dispatched;
pub mod drafting;
/// Whether the repository still has what `armada.yml` names. **A read of the
/// repository**, where `daemon::rereading` is a read of the file — a `run`
/// line naming a deleted script parses perfectly and says nothing.
mod drifting;
pub mod drone;
mod drone_moves;
mod drones_had;
/// Which task wrote which file, off its Drone's edit calls. Slice 5.
pub mod edit_calls;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with. Save stops at the bytes.
mod editing;
pub mod ending;
pub mod evidence;
/// Going and looking at a Job now, because somebody suspects it is wedged.
/// **The rung below intervene**, and it costs no model call.
mod examining;
/// Asking a cheap model what one blocked command does, for the person deciding
/// whether to allow it. It decides nothing and moves nothing.
pub mod explaining;
/// Walking the checkout for `search_files`, the `@` mention popup's read.
mod files;
/// A Drone asking for the fix to a test broken on main. #999.
pub mod fixing;
/// A running Check's log, read for `observe_check_output` as it grows.
mod following;
mod following_up;
pub mod footprint;
mod framing;
mod freezing;
mod gate;
mod gated;
mod gating;
mod group;
mod grouping;
pub mod headroom;
pub mod helm;
/// What Fleet is holding disk for, and the five tests that decide whether it
/// may give one back without asking anybody.
pub mod holding;
/// The issue a Job came from, asked about again on its own cursor. Spike 022,
/// answer 5.
mod issue_noticing;
/// What a person changes on one Job from its detail: the model its later
/// steps run as, and the commands they allowed it.
mod job_settings;
/// A press to merge under `merge_by: push`, joining the merge line.
mod joining_the_line;
/// A Job's own log, read back and served. **The other side of the file every
/// `transcript::note` call writes**, and the third voice the activity log was
/// designed around.
pub mod journal;
pub mod judging;
pub mod keeping;
mod keeping_gates;
mod kept_reply;
/// Kit's MCP servers, resolved for one Manifest. `docs/concepts/kit.md`, `#1275`.
pub mod kit;
/// Kit's allowlist, in `~/.armada`. `docs/concepts/kit.md`.
pub mod kit_allowlist;
mod landing;
/// A Job's worktree is the pool slot it leased.
mod leasing;
mod ledgering;
/// The Drones-at-once bound, memory share and disk floor a person saves, and
/// how a save reaches admission without a restart.
pub mod limits;
pub mod listener;
/// Noticing what became of a Job's pull request. **Fleet may merge, and the
/// decision is what stays a person's** — a press from Bridge is
/// `crate::merging` and reaches the same four things this module does about a
/// merge it noticed. What this module is for is the other way one settles:
/// somebody merged it on the forge, and that is only ever knowable by asking.
/// An open one is asked a second question on the same rotation —
/// `crate::under_review`.
pub mod main_ci;
mod main_fix;
pub(crate) mod main_hub;
/// The merge line `armada land` keeps in each served repository, read and published.
mod manifest_checks;
/// A possible `armada.yml` per workspace, from Scan, and the Write that ends it.
pub mod manifest_proposal;
/// The one act that writes into a repository Fleet did not make: a person
/// presses, and Fleet merges the pull request their Job opened.
mod mending;
pub mod merge_lines;
mod merging;
pub mod mint;
/// What each Job is called on disk, answerable without a lock. **Every path
/// under `.armada/` is named by the handle**, and half the places that write a
/// Job's log line hold only its id.
mod naming;
/// What a Job says it needs on a file, and the order its landing takes. `#1059`.
mod needing;
pub mod noticing;
/// Where two Jobs claim the same paths, worked out at read time. **A
/// warning and nothing else** — no dispatch path reaches it.
/// A message or a stop addressed to one Drone of a Job. #1666.
mod one_drone;
/// Which of a step's Checks starts first, from this repository's past runs.
mod ordering;
pub mod overlap;
pub mod overruling;
/// Evidence a restart found still waiting for the gate, ruled on at boot. #796.
mod pausing;
pub mod peer;
/// What a working Drone is told about other Jobs writing where it writes. #998.
pub mod peers;
mod pending_evidence;
pub mod permitting;
/// The machine's places for Checks, one line for every Job and repository. #1063.
pub mod places;
mod plan_acts;
pub mod policy;
pub mod ports;
mod precedent;
/// A person's Bridge preferences, `limits`'s shape one table over.
mod preferences;
pub mod preparing;
/// The probes Fleet can run on itself, and the Doctor modules it cannot.
/// **Not Doctor**, whose grid is ten modules and is not built.
mod probing;
pub mod process;
/// Starting a scout on a Studio, recording what it read, and stopping it.
mod promoting;
pub mod proposal;
pub mod proposals;
mod proposing;
/// Running the repository's Checks against the tree a merge left behind, and
/// the record that is keyed by the commit rather than by a Job.
mod proving;
/// A pull request by repository and number, and the acts a Session takes on one.
/// Since 23.48.
mod pull_requesting;
/// `merge_by: push` over a base that moved: brought up, gated again, pushed.
mod pushing_onto_base;
pub mod questioning;
/// Giving one Job more money than the tier above it allows, and the ceiling on
/// which surface may give it. **The act `over_budget` has always pointed at.**
pub mod raising;
/// Reading a Link in: the source, fetched by Fleet. `#1293`.
mod reading_in;
pub mod readmitting;
pub mod readopting;
/// A person running a stopped step's Checks again, with no Drone — `#1105`.
mod rechecking;
mod reclaiming;
/// Links already on a Studio, read as what their addresses name — `#1394`.
mod recognising;
/// What the boot read found and what the reconciliation did about it.
mod reconciled;
/// A proposal sent back to the proposer with a note. 23.25.
pub mod reconsidering;
/// Where one repository's records live, off the checkout — the per-repository
/// key and the one-time move of what an older Fleet wrote under it.
pub mod records;
pub mod redaction;
pub mod redispatch;
mod refusing;
mod regating;
/// A person's run of one Manifest entry in a Job's worktree, and Undo from
/// the snapshot taken before it. **A rehearsal, never a verdict.**
mod rehearsing;
/// A parked Job paused by Fleet when work waits for a full pool.
mod releasing;
pub mod remarks;
pub mod reporting;
/// The repositories one Fleet serves, and adding one by folder.
pub mod repositories;
mod rerun_settles;
mod rerunning;
mod rescuing;
/// What one Job holds on this machine — its processes, what they are burning,
/// and the disk its worktree has taken. **Read on demand, never on the turn.**
pub mod resources;
pub mod resume;
/// A Job's retro: what got in the way while it ran. `docs/concepts/retro.md`.
mod retro;
mod reuse;
mod review;
mod review_term;
pub mod reviewing;
/// The Drones Fleet is holding, read without taking a working slot.
mod rostered;
mod ruling;
pub mod runtime;
mod saving;
pub mod saying;
/// Scan: reading a repository nobody set up for Armada. **It writes nothing,
/// because a [`scanning::Tree`] has no write on it.**
pub mod scanning;
pub mod scope;
mod scoping;
/// The read-only agent a person starts from a Studio. `#1292`.
pub mod scout;
mod scouting;
pub mod seeding;
mod servers;
pub mod serving;
pub mod session;
/// The session ledger: what a harness says of an agent session. `docs/concepts/session.md`.
pub mod session_host;
mod sessioning;
mod settling;
/// Running the repository's own harness, and keeping what it produced.
pub mod showing;
/// A person asking a Job to show its work, off the turn loop — `#603`.
mod showing_again;
pub mod silence;
pub mod slots;
/// The Manifest a Job actually sees — what was snapshotted at its creation,
/// resolved by every reader `#650` named rather than by `Fleet::manifest`
/// directly.
mod snapshotting;
pub mod spawning;
/// What the fleet has spent, and which Jobs a ceiling is holding.
mod spending;
mod stuck;
/// A Picture a person pastes onto a Studio, kept as a Note's frame is.
mod studio_pictures;
/// Runs a Studio holds, and what a node keeps of one past retention. `#1289`.
pub mod studio_runs;
/// Servers a Studio holds: started from one, and kept when they end. `#1345`.
pub mod studio_servers;
mod studio_sketches;
/// A repository's Studios, through the store and onto the stream. `#1285`.
mod studios;
pub mod sub_dispatch;
mod summarising;
mod superseding;
pub mod sweeping;
/// Edit this task, and a Job's tier map. Spike 022, slice 3.
/// The merge line Fleet runs for each repository, and its turn.
mod taking_turns;
pub mod task_edits;
/// A step whose plan is worked one task at a time, a Drone each. Spike 022, 1b.
pub mod tasking;
pub mod terms;
mod tooling;
pub mod transcript;
/// A step's tuning at the approval press. 23.20.
pub mod tuned;
pub mod turning;
/// The one vigil whose subject is a Job with no Drone to watch.
mod unattended;
/// What the forge says about a pull request nobody has merged yet, read on the
/// sweep `noticing` already runs.
mod under_review;
pub mod underway;
/// What a person pointed at while walking a Job's served mock, kept on the
/// Job and carried to the next Drone. Protocol 23.16.
mod walk_notes;
mod walking;
pub mod watch;
/// An Epic's wave: proposed, released by one press, and the members a parent
/// that finishes on their merges waits for. Spike 022, slice 6.
pub mod waving;
pub mod widening;
/// The redactions the `Queries` and `Commands` impls call by hand. Split out to
/// keep those files, rather than their helpers, the thing that grows.
mod wire;
mod work_plan;
pub mod working;
pub mod workspaces;

#[cfg(test)]
mod tests;

pub use adopting::{reattaching, Adopted, Gap, Reattachment, Session};
pub use adrift::Adrift;
pub use allowance::{Allowance, Micros, Overspent};
pub use asked::Asked;
pub use asked_run::{AskedRuns, ChecksReported, NotRun};
pub use at_step::AtStep;
pub use budget::CommandBudget;
pub use clock::{Clock, SystemClock};
pub use confirming::{Confirmed, ONE_BY_ONE};
pub use converging::{NoReport, ReportNow, Stage, StepNorms, Tripwire, Wandering, FORCED_REPORT};
pub use crossing::{Cleared, Crossed, Dispatched, Produced, Reconciling, Redirected, ThePlan};
pub use daemon::{Fittings, Fleet, Host, StartingIn};
pub use delivery::Delivered;
pub use detach::Detached;
pub use drone::{
    aftermath, environment, Aftermath, DroneNotStarted, Ending, HostPaths, Left, Started,
};
pub use evidence::{
    Call, Decline, EvidenceInbox, EvidenceTool, Landed, NotSubmitted, Recorded, Standing,
};
pub use gate::{apply, rule_on, Began, CheckBudget, CheckOutput, Ruling};
pub use headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Short, Spare, TheMachine};
pub use holding::{GaveBack, Held, Holding, Reclaiming};
pub use judging::{Aloft, CallFailed, JudgeBudget, Judging, Look, Marking};
pub use keeping::{deliverables_dir, kept_deliverables, Keeping};
pub use listener::claimed_listener_port;
pub use mint::{Mint, UlidMint};
pub use noticing::{Noticed, Noticing};
pub use overruling::Overruling;
pub use peer::{NotACaller, PeerOf};
pub use places::{Asking, ChecksAtOnce, Place, Places, Room, OVERTAKEN_AT_MOST};
pub use policy::{HeldBecause, Policies};
pub use ports::{detect_ceiling, BindConnectProbe, PortRange, PortsRefused};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use proposal::{proposed, Proposing};
pub use proposing::{Brief, NotProposed, Proposal, ProposedJob, Unresolved};
pub use questioning::{Answer, NotAnswered, NotAsked, Question, Told};
pub use readopting::Recovered;
pub use rechecking::Unrecheckable;
pub use reconciled::Reconciled;
pub use redaction::Redactor;
pub use redispatch::Replacement;
pub use rehearsing::verify_steps;
pub use releasing::{release_order, Parked};
pub use reporting::{Counted, Filed, NotFiled};
pub use resume::Roused;
pub use reuse::KeptAskedRun;
pub use runtime::{
    listener_address, machine_path, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME,
};
pub use scope::{Declared, Drifting, NotDeclared};
pub use seeding::{Cold, CopyOnWrite, NotCloned, Seeding, TheVolume};
pub use session::{DroneSession, LiveSession, Turn};
pub use settling::Settled;
pub use showing::{frames_dir, show, ComingUp, NotShown, Shown};
pub use showing_again::Unshowable;
pub use silence::{Liveness, Poke, Quiet, Vigil};
pub use slots::Concurrency;
pub use sub_dispatch::NotDispatched;
pub use transcript::{history, log_of, transcript_of, Live, Recording, Spine, Tap, Taps};
pub use turning::{keep_turning, Turned, Turning, Worked};
pub use underway::{Announcing, LiveLog, Underway};
pub use watch::{Drained, Progress, Watching};
pub use widening::{NotWidened, Widening};
pub use wire::left_out_workflow;
pub use work_plan::PlanChanged;
