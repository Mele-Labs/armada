//! How much of the machine Fleet takes, and what one Job or step may spend.
//! Each default carries the argument for its number; `settings.toml`'s row
//! carries the rest.

use super::{Applies, Entry, Integer, Kind, Section, Shipped};

pub const DRONES_AT_ONCE: Integer = Integer("limits.dronesAtOnce");
pub const CHECKS_AT_ONCE: Integer = Integer("limits.checksAtOnce");
pub const MEMORY_SPARE_PERCENT: Integer = Integer("limits.memorySparePercent");
pub const DISK_FLOOR_GIB: Integer = Integer("limits.diskFloorGib");
pub const COST_CAP_DOLLARS_PER_JOB: Integer = Integer("limits.costCapDollarsPerJob");
pub const TURN_CAP_PER_JOB: Integer = Integer("limits.turnCapPerJob");
pub const ASKED_RUNS_PER_STEP: Integer = Integer("limits.askedRunsPerStep");
pub const FIXES_PER_STEP: Integer = Integer("limits.fixesPerStep");
pub const TOOL_CALLS_PER_STEP: Integer = Integer("limits.toolCallsPerStep");
pub const CONFLICT_CLEARING_SENDS: Integer = Integer("limits.conflictClearingSends");
pub const JUDGE_READ_TURNS: Integer = Integer("limits.judgeReadTurns");
pub const STANDING_RULES_KIB: Integer = Integer("limits.standingRulesKib");
pub const SLOT_BUILD_TRIM_AFTER_DAYS: Integer = Integer("limits.slotBuildTrimAfterDays");
pub const SLOT_BUILD_CEILING_GIB: Integer = Integer("limits.slotBuildCeilingGib");
pub const PORT_RANGE_BASE: Integer = Integer("limits.portRangeBase");
pub const PORT_BLOCK_GRANULE: Integer = Integer("limits.portBlockGranule");

pub(super) const LIMITS: &[Entry] = &[
    // **Two, and the ceiling is not what bounds it.** Spike 012 ran five Drones
    // against one listener and told every one apart, so attribution is measured
    // well above two. What is not measured is everything else about five: `#47`,
    // two Drones writing one file with no write-scope reservation, and `#44`,
    // whether the machine has the memory and quota. Two is `#50`'s definition of
    // done, makes `#215`'s deadlock impossible, and is the smallest step that is
    // still a step. More buys throughput and costs a longer wait at the merge
    // end, where `Fleet::merge_end` serialises every push.
    Entry {
        key: DRONES_AT_ONCE.0,
        section: Section::Limits,
        title: "Drones at once",
        description: "How many Jobs have a Drone working at the same time. A lower number stops nothing already running; new starts wait until enough finish.",
        kind: Kind::Integer { min: 1, max: 8, unit: "Drones" },
        shipped: Shipped::Integer(2),
        applies: Applies::Live,
        row: Some("concurrency-cap"),
        env: None,
    },
    // **Half the cores, from one to eight** — `fleet::ChecksAtOnce::for_cores`,
    // which is why it is supplied. Measured under one Job on ten cores: this
    // repository's six Checks took 28.5s one at a time against 16.5s at four,
    // with two to six within noise, because the slowest Check and one Cargo
    // target lock set the floor. Four was per gate; the limit is the machine's
    // now (#1063), and half leaves the rest to Drones.
    Entry {
        key: CHECKS_AT_ONCE.0,
        section: Section::Limits,
        title: "Checks at once",
        description: "How many Checks run at the same time on this machine, across every Job and every repository. One already running finishes.",
        kind: Kind::Integer { min: 1, max: 8, unit: "Checks" },
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("checks-at-once"),
        env: None,
    },
    // **15% of memory, a floor rather than a measurement.** Nothing has measured
    // what a Drone costs in memory; the number refuses work on a machine that is
    // already full. CPU has no threshold at all: the operating system schedules it.
    Entry {
        key: MEMORY_SPARE_PERCENT.0,
        section: Section::Limits,
        title: "Memory to keep free",
        description: "The share of memory that must be free before another Drone starts. A Job approved onto a machine short of it waits.",
        kind: Kind::Integer { min: 0, max: 50, unit: "percent" },
        shipped: Shipped::Integer(15),
        applies: Applies::Live,
        row: Some("cpu-mem-headroom-threshold-for-spawning"),
        env: None,
    },
    // **Ten gibibytes, and that one is measured.** A parallel agent run filled a
    // volume at 220 GB across 74 worktrees — three gigabytes each, cut worktree
    // plus build output — and three agents died at zero bytes free holding
    // uncommitted work. Ten is about three of those: enough that the Job being
    // started can finish and the operator has warning before the next one. Bytes
    // and not a share, which is why it is its own value — see `Headroom::of`.
    Entry {
        key: DISK_FLOOR_GIB.0,
        section: Section::Limits,
        title: "Disk to keep free",
        description: "The gibibytes that must be free on a Job's volume before its Drone starts. Zero accepts a full disk.",
        kind: Kind::Integer { min: 0, max: 100, unit: "GiB" },
        shipped: Shipped::Integer(10),
        applies: Applies::Live,
        row: Some("disk-headroom-floor-for-spawning"),
        env: None,
    },
    // **Deliberately wide.** A small feature Job measured a mean of $0.099 over
    // three identical runs whose prices spread 2.31x on cache warmth alone, so a
    // cap near the mean would refuse a healthy Job for having started cold. Ten
    // dollars is about a hundred such Jobs: one that has stopped making progress
    // and kept paying. It was five until 9 Sep 2026, when a Job was refused its
    // last step at $5.28 having spent three of six Drones on Armada's own
    // defects. **Notional dollars**: spike 5 found `total_cost_usd` is list
    // price, and this account is not billed per token, so it is a runaway
    // detector rather than an invoice. A Job's column and `armada.yml` each win
    // over it — `fleet::Allowance::at`.
    Entry {
        key: COST_CAP_DOLLARS_PER_JOB.0,
        section: Section::Limits,
        title: "Spend per Job",
        description: "What one Job may spend, in notional dollars, before Fleet starts no more Drones on it. A Drone already working is not stopped. armada.yml and the Job's own cap win over it.",
        kind: Kind::Integer { min: 1, max: 1_000, unit: "dollars" },
        shipped: Shipped::Integer(10),
        applies: Applies::Live,
        row: Some("budget-cost-cap-per-job"),
        env: None,
    },
    // **The ceiling that actually catches something.** The same three runs
    // turned 7, 7 and 4 times, so turns are the steady signal the price is not.
    // A four-step Job at a generous thirty turns a step is 120; three hundred
    // leaves room for a workflow twice that long and still stops a Drone going
    // in circles for hours. It had no tier under it until 9 Sep 2026, when a Job
    // stopped at 393 having passed every Check, with a cheap `summarise` unrun.
    Entry {
        key: TURN_CAP_PER_JOB.0,
        section: Section::Limits,
        title: "Turns per Job",
        description: "How many turns one Job's Drones may take in all before Fleet starts no more. armada.yml and the Job's own cap win over it.",
        kind: Kind::Integer { min: 1, max: 10_000, unit: "turns" },
        shipped: Shipped::Integer(300),
        applies: Applies::Live,
        row: Some("budget-turn-cap-per-job"),
        env: None,
    },
    // **Three, derived from what it stands in for**, and measured on nothing. A
    // Drone that could run the Checks itself would run them about once per
    // attempt at green, and spike 009 puts a step's p90 at 437s of work — not
    // room for many workspace builds on top. One makes the tool a single shot
    // saved for the end, when it is worth least; more than three stops being a
    // check on the work and starts being the work. **A cost bound, not a
    // convergence one**: `fleet::asked_run` suspends both clocks while a run is
    // out, and a Drone that spends all three is still caught by the tool-call
    // tripwire, since each ask is one of its calls.
    Entry {
        key: ASKED_RUNS_PER_STEP.0,
        section: Section::Limits,
        title: "Check runs a step may ask for",
        description: "How many times one step's Drone may ask Fleet to run its Checks.",
        kind: Kind::Integer { min: 0, max: 20, unit: "runs" },
        shipped: Shipped::Integer(3),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    // One, a cost bound for the asked runs' reason: each is a run against main.
    // A step that meets a second test broken on main says so in its evidence. #999.
    Entry {
        key: FIXES_PER_STEP.0,
        section: Section::Limits,
        title: "Fixes a step may ask for",
        description: "How many fixes to main one step's Drone may ask for.",
        kind: Kind::Integer { min: 0, max: 10, unit: "fixes" },
        shipped: Shipped::Integer(1),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    // Measured on one repository rather than none — spike 009, 31 steps, two
    // workflows, one model, a warm cache. Median 18 calls, p90 68, so sixty sits
    // just under the widest ordinary step and four of the 31 would have bought a
    // look. Left there rather than raised to the p95: 31 steps is not enough to
    // move a tripwire towards firing less. **A trip spends the step's only
    // look**, so a ceiling low enough to catch a stuck Drone early burns the
    // attention a later, real thrash would need — `fleet::converging`.
    Entry {
        key: TOOL_CALLS_PER_STEP.0,
        section: Section::Limits,
        title: "Tool calls before a step is looked at",
        description: "How many tool calls a step's Drone makes before the Judge is asked whether it is going in circles.",
        kind: Kind::Integer { min: 1, max: 1_000, unit: "calls" },
        shipped: Shipped::Integer(60),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    // A base that keeps moving while a Drone clears it would send the Job round
    // for ever, so Fleet's own sends are bounded and a person's are not.
    Entry {
        key: CONFLICT_CLEARING_SENDS.0,
        section: Section::Limits,
        title: "Conflict clearing passes",
        description: "How many times in a row Fleet sends a pull request's branch back to clear a conflict before the Job asks for a person.",
        kind: Kind::Integer { min: 1, max: 10, unit: "passes" },
        shipped: Shipped::Integer(3),
        applies: Applies::Live,
        row: Some("conflict-clearing-send-cap"),
        env: None,
    },
    // Sixteen turns, a Judge's reads and its answer together; the row holds why.
    Entry {
        key: JUDGE_READ_TURNS.0,
        section: Section::Limits,
        title: "Judge read turns",
        description: "How many turns one Judge call may take reading the repository before it answers.",
        kind: Kind::Integer { min: 1, max: 64, unit: "turns" },
        shipped: Shipped::Integer(16),
        applies: Applies::Live,
        row: Some("judge-read-turns"),
        env: None,
    },
    Entry {
        key: STANDING_RULES_KIB.0,
        section: Section::Limits,
        title: "Standing rules shown to the Judge",
        description: "How much of a repository's standing rules file every Judge call is given. The rest is cut, and the Judge is told so.",
        kind: Kind::Integer { min: 1, max: 64, unit: "KiB" },
        shipped: Shipped::Integer(4),
        applies: Applies::Live,
        row: Some("standing-rules-cap"),
        env: None,
    },
    Entry {
        key: SLOT_BUILD_TRIM_AFTER_DAYS.0,
        section: Section::Limits,
        title: "Build output kept",
        description: "How old a file in a checkout's build output may grow before the sweep removes it.",
        kind: Kind::Integer { min: 1, max: 365, unit: "days" },
        shipped: Shipped::Integer(14),
        applies: Applies::AtRestart,
        row: Some("slot-build-trim-after-days"),
        env: None,
    },
    Entry {
        key: SLOT_BUILD_CEILING_GIB.0,
        section: Section::Limits,
        title: "Build output ceiling",
        description: "How large one checkout's build output may stay after a sweep before it is removed whole.",
        kind: Kind::Integer { min: 1, max: 1_000, unit: "GiB" },
        shipped: Shipped::Integer(20),
        applies: Applies::AtRestart,
        row: Some("slot-build-ceiling-gib"),
        env: None,
    },
    // Measured on nothing — chosen to sit below every platform's ephemeral floor
    // and above the ports a repository's own tooling claims (3000, 5432, 8080).
    Entry {
        key: PORT_RANGE_BASE.0,
        section: Section::Limits,
        title: "First port Fleet hands out",
        description: "The low end of the range a Job's ports, and Fleet's own, are claimed from.",
        kind: Kind::Integer { min: 1_024, max: 60_000, unit: "port" },
        shipped: Shipped::Integer(40_000),
        applies: Applies::AtRestart,
        row: Some("port-range-base"),
        env: None,
    },
    // Provisional: `docs/concepts/machine.md` names the signal to move it, a
    // repository where mid-Job widenings routinely fail to extend in place.
    Entry {
        key: PORT_BLOCK_GRANULE.0,
        section: Section::Limits,
        title: "Port block size",
        description: "The unit a Job's claim of ports is rounded up to.",
        kind: Kind::Integer { min: 1, max: 64, unit: "ports" },
        shipped: Shipped::Integer(8),
        applies: Applies::AtRestart,
        row: Some("port-block-granule"),
        env: None,
    },
];
