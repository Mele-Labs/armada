//! How much of the machine Fleet takes, and what one Job or step may spend.
//! Each default carries the argument for its number; `settings.toml`'s row
//! carries the rest.

use super::{Applies, Entry, Integer, Kind, Section, Shipped};

pub const DRONES_AT_ONCE: Integer = Integer("limits.dronesAtOnce");
pub const CHECKS_AT_ONCE: Integer = Integer("limits.checksAtOnce");
pub const MEMORY_SPARE_PERCENT: Integer = Integer("limits.memorySparePercent");
pub const DISK_FLOOR_GIB: Integer = Integer("limits.diskFloorGib");
pub const DISK_RECLAIM_FLOOR_GIB: Integer = Integer("limits.diskReclaimFloorGib");
pub const STANDING_RULES_KIB: Integer = Integer("limits.standingRulesKib");
pub const SLOT_BUILD_TRIM_AFTER_DAYS: Integer = Integer("limits.slotBuildTrimAfterDays");
pub const SLOT_BUILD_CEILING_GIB: Integer = Integer("limits.slotBuildCeilingGib");
pub const PORT_RANGE_BASE: Integer = Integer("limits.portRangeBase");
pub const PORT_BLOCK_GRANULE: Integer = Integer("limits.portBlockGranule");
pub const WATCHES_AT_ONCE: Integer = Integer("limits.watchesAtOnce");
pub const WATCHES_PER_JOB: Integer = Integer("limits.watchesPerJob");
pub const OPERATOR_PLAN_PAUSE: Integer = Integer("limits.operatorPlanPause");

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
        tip: None,
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
        tip: None,
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
        tip: None,
    },
    // **Thirty gibibytes, and that one is measured.** A parallel agent run filled a
    // volume at 220 GB across 74 worktrees and three agents died at zero bytes free
    // holding uncommitted work. On 13 Nov 2026 free space fell 17 GB in about 20
    // minutes, 50 GB an hour at the peak, and one cold native build writes over 10 GB:
    // ten was ten minutes of runway, thirty is about half an hour for the operator to
    // act. Bytes and not a share, which is why it is its own value — see `Headroom::of`.
    Entry {
        key: DISK_FLOOR_GIB.0,
        section: Section::Limits,
        title: "Disk to keep free",
        description: "The gibibytes that must be free on a Job's volume before its Drone starts. Zero accepts a full disk.",
        kind: Kind::Integer { min: 0, max: 100, unit: "GiB" },
        shipped: Shipped::Integer(30),
        applies: Applies::Live,
        row: Some("disk-headroom-floor-for-spawning"),
        env: None,
        tip: None,
    },
    // **A hundred gibibytes**: the hold floor's thirty is half an hour at 50 GB an hour,
    // and reclaiming has to finish before it. Dropping the oldest builds frees about 50
    // GB a pass on this machine, so it starts three times as far up. The owner's
    // decision of 13 Nov 2026, `.armada/documents/disk-reclaim-policy.html`.
    Entry {
        key: DISK_RECLAIM_FLOOR_GIB.0,
        section: Section::Limits,
        title: "Disk to start reclaiming at",
        description: "The gibibytes free on a repository's volume under which Fleet drops old build output, oldest first, until there are twenty more. Zero never reclaims.",
        kind: Kind::Integer { min: 0, max: 500, unit: "GiB" },
        shipped: Shipped::Integer(100),
        applies: Applies::Live,
        row: Some("disk-reclaim-floor"),
        env: None,
        tip: None,
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
        tip: None,
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
        tip: None,
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
        tip: None,
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
        tip: None,
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
        tip: None,
    },
    // **Two, as Drones at once.** A watch's command is a provider's command-line tool, mostly
    // waiting on the network, so it costs little; two bounds what Fleet runs on a clock nobody
    // is watching. Measured on nothing.
    Entry {
        key: WATCHES_AT_ONCE.0,
        section: Section::Limits,
        title: "Watches at once",
        description: "How many watches run their command at the same time on this machine. One that is due waits for a place.",
        kind: Kind::Integer { min: 1, max: 8, unit: "watches" },
        shipped: Shipped::Integer(2),
        applies: Applies::Live,
        row: Some("watches-at-once"),
        env: None,
        tip: None,
    },
    // Five: the Job spike 36 replayed needed two, a CI run and a deploy.
    Entry {
        key: WATCHES_PER_JOB.0,
        section: Section::Limits,
        title: "Watches a Job may hold",
        description: "How many live watches one Job may hold. Starting one more is refused, and the refusal says so.",
        kind: Kind::Integer { min: 1, max: 20, unit: "watches" },
        shipped: Shipped::Integer(5),
        applies: Applies::Live,
        row: Some("watches-per-job"),
        env: None,
        tip: None,
    },
    // **Eighty, as the plan's step down is.** The share of the week's plan the Operator leaves
    // alone: past it the Operator makes no call, so the week's last of the plan goes to Jobs.
    Entry {
        key: OPERATOR_PLAN_PAUSE.0,
        section: Section::Limits,
        title: "Pause the Operator at",
        description: "How much of the weekly plan window must be used before the Operator makes no more calls until the window resets. Everything that needed a call goes to you.",
        kind: Kind::Integer { min: 0, max: 100, unit: "percent" },
        shipped: Shipped::Integer(80),
        applies: Applies::Live,
        row: Some("operator-plan-pause"),
        env: None,
        tip: None,
    },
];
