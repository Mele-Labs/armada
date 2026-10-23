//! How long Fleet waits for each thing, how often it looks, and how long it
//! keeps what it wrote. A step's or a repository's own value in `armada.yml`
//! or a workflow wins over the machine's, as it always has.

use super::{Applies, Entry, Integer, Kind, Seconds, Section, Shipped};

pub const CHECK_SECONDS: Seconds = Seconds("timeouts.checkSeconds");
pub const JUDGE_SECONDS: Seconds = Seconds("timeouts.judgeSeconds");
pub const COMMAND_SECONDS: Seconds = Seconds("timeouts.commandSeconds");
pub const PROPOSER_SECONDS: Seconds = Seconds("timeouts.proposerSeconds");
pub const HELM_ASK_HOLD_SECONDS: Seconds = Seconds("timeouts.helmAskHoldSeconds");
pub const SESSION_QUIET_SECONDS: Seconds = Seconds("timeouts.sessionQuietSeconds");
pub const HELM_REPLY_SECONDS: Seconds = Seconds("timeouts.helmReplySeconds");
pub const MERGE_NOTICE_SECONDS: Seconds = Seconds("timeouts.mergeNoticeSeconds");
pub const RESOURCE_POLL_SECONDS: Seconds = Seconds("timeouts.resourcePollSeconds");
pub const BUILD_SWEEP_SECONDS: Seconds = Seconds("timeouts.buildSweepSeconds");
pub const WATCH_RUN_SECONDS: Seconds = Seconds("timeouts.watchRunSeconds");
pub const WATCH_EVERY_SECONDS: Seconds = Seconds("timeouts.watchEverySeconds");
pub const WATCH_GIVE_UP_MINUTES: Integer = Integer("timeouts.watchGiveUpMinutes");
pub const TURN_INTERVAL_MS: Integer = Integer("timeouts.turnIntervalMs");
pub const BRIDGE_RECONNECT_MS: Integer = Integer("timeouts.bridgeReconnectMs");

pub const RUN_LOG_DAYS: Integer = Integer("retention.runLogDays");

pub(super) const TIMEOUTS: &[Entry] = &[
    // Provisional: a cold workspace build is minutes, and nothing has measured
    // what the ceiling should be.
    Entry {
        key: CHECK_SECONDS.0,
        section: Section::Timeouts,
        title: "Check",
        description: "How long one Check may run before it counts as failed.",
        kind: Kind::Seconds { min: 10, max: 7_200 },
        shipped: Shipped::Seconds(900),
        applies: Applies::Live,
        row: Some("check-timeout"),
        env: None,
        tip: None,
    },
    // The Judge latency row reads `undecided`, so nothing has measured this. It
    // is short because the calls sit at a gate a person waits behind: latency is
    // what it bounds, not money. A Judge emits no result envelope, so nothing
    // could read what one cost and a dollar cap would be enforced by nothing.
    Entry {
        key: JUDGE_SECONDS.0,
        section: Section::Timeouts,
        title: "Judge call",
        description: "How long one Judge call may take before it is given up as unanswered.",
        kind: Kind::Seconds { min: 10, max: 1_800 },
        shipped: Shipped::Seconds(120),
        applies: Applies::Live,
        row: Some("modelclient-endpoint-and-latency-budget"),
        env: None,
        tip: None,
    },
    // A command that only reads and writes the store, or does a little local
    // work beside it such as `approve_review`'s occasional push. Bridge waits
    // this plus five seconds, so a Bridge timeout means Fleet gave up first and
    // said so — `fleet::budget::CommandBudget`, `#712`. Provisional, replacing
    // `#693`'s 18s `GET /jobs` under load, which stopped describing the system
    // once `#693` moved Fleet's blocking `git` and `gh` calls off the runtime.
    Entry {
        key: COMMAND_SECONDS.0,
        section: Section::Timeouts,
        title: "Command",
        description: "How long one plain command may take before Fleet answers that it gave up. Bridge waits five seconds longer.",
        kind: Kind::Seconds { min: 5, max: 120 },
        shipped: Shipped::Seconds(15),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    // **Longer than the Judge's, and the difference is who is waiting.** A Judge
    // runs with nobody watching, so its budget is the only thing that ends a
    // call that will not answer. A proposal is watched and `stop_proposal` kills
    // it, so a tight budget would take the decision from the person; ten minutes
    // is the outer bound on a call nobody is left to stop. Bridge asks after two,
    // `PROPOSAL_IS_SLOW` in `packages/screens`. Measured on nothing.
    Entry {
        key: PROPOSER_SECONDS.0,
        section: Section::Timeouts,
        title: "Job proposer call",
        description: "How long the Job proposer may take reading a request before Fleet gives up on it.",
        kind: Kind::Seconds { min: 30, max: 3_600 },
        shipped: Shipped::Seconds(600),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    // `fleet::helm::SHIPPED_ASK_HOLD` says what the fourteen minutes is measured
    // against; the row carries it too.
    Entry {
        key: HELM_ASK_HOLD_SECONDS.0,
        section: Section::Timeouts,
        title: "Helm ask held open",
        description: "How long one call a Helm job made waits for a person in the dock before Fleet refuses it for them.",
        kind: Kind::Seconds { min: 30, max: 3_600 },
        shipped: Shipped::Seconds(840),
        applies: Applies::AtRestart,
        row: Some("helm-ask-hold"),
        env: None,
        tip: None,
    },
    Entry {
        key: SESSION_QUIET_SECONDS.0,
        section: Section::Timeouts,
        title: "Job left idle",
        description: "How long a hosted job's process may sit with no turn before Fleet ends it. The next message resumes it.",
        kind: Kind::Seconds { min: 60, max: 86_400 },
        shipped: Shipped::Seconds(600),
        applies: Applies::AtRestart,
        row: Some("session-quiet-timeout"),
        env: None,
        tip: None,
    },
    // Generous, because a reply that reads a dozen Jobs is a dozen tool calls.
    // Not an idle timeout: one process per message, closed once written.
    Entry {
        key: HELM_REPLY_SECONDS.0,
        section: Section::Timeouts,
        title: "Helm reply",
        description: "How long one Helm reply may take before its process is ended.",
        kind: Kind::Seconds { min: 60, max: 7_200 },
        shipped: Shipped::Seconds(900),
        applies: Applies::AtRestart,
        row: None,
        env: None,
        tip: None,
    },
    // **One pull request a minute, not every Job on a turn.** The question is a
    // process, so asking about every unsettled Job four times a second would
    // spend more of the machine on the question than on the work; ten open pull
    // requests is each asked every ten minutes. A minute is the latency of a
    // merge reaching the Board, and nobody is waiting on it faster than that.
    Entry {
        key: MERGE_NOTICE_SECONDS.0,
        section: Section::Timeouts,
        title: "Pull request check",
        description: "How often Fleet asks the forge what became of one open pull request, in rotation.",
        kind: Kind::Seconds { min: 10, max: 3_600 },
        shipped: Shipped::Seconds(60),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    // A reading costs three short-lived processes and about 80ms, so one every
    // turn would be a measurable share of a core. Five seconds is twenty turns;
    // the staleness can cost one Job admitted onto a machine that filled since.
    Entry {
        key: RESOURCE_POLL_SECONDS.0,
        section: Section::Timeouts,
        title: "Machine reading",
        description: "How stale a reading of the machine's memory and disk may be before it is taken again.",
        kind: Kind::Seconds { min: 1, max: 300 },
        shipped: Shipped::Seconds(5),
        applies: Applies::Live,
        row: Some("fleet-health-check-resource-poll-interval"),
        env: None,
        tip: None,
    },
    // An hour: a stale file is a fortnight old, so nothing is gained by looking
    // more often than a person would notice.
    Entry {
        key: BUILD_SWEEP_SECONDS.0,
        section: Section::Timeouts,
        title: "Build output sweep",
        description: "How often Fleet trims every checkout's build output.",
        kind: Kind::Seconds { min: 60, max: 86_400 },
        shipped: Shipped::Seconds(3_600),
        applies: Applies::AtRestart,
        row: Some("build-sweep-interval-minutes"),
        env: None,
        tip: None,
    },
    // Measured on nothing. It is the latency of a ruling and of a start, and the
    // start cannot be taken away: every dispatch is this loop's, a task nobody's
    // browser owns (`#428`). It costs one store read a tick while idle, which
    // `fleet::turning` names as the reason to wake the loop rather than poll it.
    Entry {
        key: TURN_INTERVAL_MS.0,
        section: Section::Timeouts,
        title: "Turn",
        description: "How often Fleet moves every Job on: rulings, starts and the loops beside them.",
        kind: Kind::Integer { min: 50, max: 5_000, unit: "ms" },
        shipped: Shipped::Integer(250),
        applies: Applies::AtRestart,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: BRIDGE_RECONNECT_MS.0,
        section: Section::Timeouts,
        title: "Bridge reconnect",
        description: "How long Bridge waits before it tries Fleet again after losing it.",
        kind: Kind::Integer { min: 250, max: 60_000, unit: "ms" },
        shipped: Shipped::Integer(2_000),
        applies: Applies::Live,
        row: Some("bridge-to-fleet-reconnect-timeout-attempts"),
        env: None,
        tip: None,
    },
    // Measured on nothing: spike 36 replayed readings and never timed a live command. Thirty
    // seconds is half the minute it read at, so a hung run cannot overlap the next.
    Entry {
        key: WATCH_RUN_SECONDS.0,
        section: Section::Timeouts,
        title: "Watch run",
        description: "How long one run of a watch's command may take before it is stopped and read as broken.",
        kind: Kind::Seconds { min: 5, max: 600 },
        shipped: Shipped::Seconds(30),
        applies: Applies::Live,
        row: Some("watch-run-timeout"),
        env: None,
        tip: None,
    },
    // **The floor, not a cadence.** A recipe or an agent names how often; one asking for less
    // than this is refused. Spike 36 read once a minute and missed no moment that mattered.
    Entry {
        key: WATCH_EVERY_SECONDS.0,
        section: Section::Timeouts,
        title: "Fastest a watch may run",
        description: "The shortest wait between two runs of a watch's command. A watch asking to run more often is refused.",
        kind: Kind::Seconds { min: 10, max: 3_600 },
        shipped: Shipped::Seconds(30),
        applies: Applies::Live,
        row: Some("watch-every-floor"),
        env: None,
        tip: None,
    },
    // Two hours where nothing is named, and a day at most, which is also the most a watch may
    // ask for. Spike 36's three deploy runs each ended within 30 minutes of the push.
    Entry {
        key: WATCH_GIVE_UP_MINUTES.0,
        section: Section::Timeouts,
        title: "Watch gives up after",
        description: "How long a watch that names no time of its own runs before it gives up. None may ask for more than a day.",
        kind: Kind::Integer { min: 1, max: 1_440, unit: "minutes" },
        shipped: Shipped::Integer(120),
        applies: Applies::Live,
        row: Some("watch-give-up"),
        env: None,
        tip: None,
    },
];

pub(super) const RETENTION: &[Entry] = &[
    // Not the Job retention window: `settings.toml`'s row says why a run fired
    // by hand keeps its log on its own clock.
    Entry {
        key: RUN_LOG_DAYS.0,
        section: Section::Retention,
        title: "Hand-run logs",
        description: "How long a Check or Command run by hand from the Manifest keeps its log.",
        kind: Kind::Integer {
            min: 1,
            max: 365,
            unit: "days",
        },
        shipped: Shipped::Integer(30),
        applies: Applies::Live,
        row: Some("ad-hoc-run-log-retention"),
        env: None,
        tip: None,
    },
];
