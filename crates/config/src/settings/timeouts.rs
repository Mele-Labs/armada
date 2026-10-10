//! How long Fleet waits for each thing, how often it looks, and how long it
//! keeps what it wrote. A step's or a repository's own value in `armada.yml`
//! or a workflow wins over the machine's, as it always has.

use super::{Applies, Entry, Integer, Kind, Seconds, Section, Shipped};

pub const CHECK_SECONDS: Seconds = Seconds("timeouts.checkSeconds");
pub const JUDGE_SECONDS: Seconds = Seconds("timeouts.judgeSeconds");
pub const COMMAND_SECONDS: Seconds = Seconds("timeouts.commandSeconds");
pub const UNANSWERED_ASK_SECONDS: Seconds = Seconds("timeouts.unansweredAskSeconds");
pub const PROPOSER_SECONDS: Seconds = Seconds("timeouts.proposerSeconds");
pub const STEP_WALL_CLOCK_SECONDS: Seconds = Seconds("timeouts.stepWallClockSeconds");
pub const STEP_GRACE_SECONDS: Seconds = Seconds("timeouts.stepGraceSeconds");
pub const DRONE_QUIET_AFTER_SECONDS: Seconds = Seconds("timeouts.droneQuietAfterSeconds");
pub const DRONE_POKE_LIMIT: Integer = Integer("timeouts.dronePokeLimit");
pub const HELM_ASK_HOLD_SECONDS: Seconds = Seconds("timeouts.helmAskHoldSeconds");
pub const SESSION_QUIET_SECONDS: Seconds = Seconds("timeouts.sessionQuietSeconds");
pub const HELM_REPLY_SECONDS: Seconds = Seconds("timeouts.helmReplySeconds");
pub const MERGE_NOTICE_SECONDS: Seconds = Seconds("timeouts.mergeNoticeSeconds");
pub const RECLAIM_SWEEP_SECONDS: Seconds = Seconds("timeouts.reclaimSweepSeconds");
pub const RESOURCE_POLL_SECONDS: Seconds = Seconds("timeouts.resourcePollSeconds");
pub const BUILD_SWEEP_SECONDS: Seconds = Seconds("timeouts.buildSweepSeconds");
pub const TURN_INTERVAL_MS: Integer = Integer("timeouts.turnIntervalMs");
pub const BRIDGE_RECONNECT_MS: Integer = Integer("timeouts.bridgeReconnectMs");

pub const RUN_LOG_DAYS: Integer = Integer("retention.runLogDays");
pub const HELM_SESSION_DAYS: Integer = Integer("retention.helmSessionDays");

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
    },
    // Measured on nothing: an order of magnitude past the harness's own few
    // minutes of hold, and comfortably short of the overnight `#801` was filed
    // about. The Drone is ended, the step stopped, the Job escalated.
    Entry {
        key: UNANSWERED_ASK_SECONDS.0,
        section: Section::Timeouts,
        title: "Unanswered permission ask",
        description: "How long a Drone's permission ask may go unanswered before Fleet ends the Drone and the Job asks for a person.",
        kind: Kind::Seconds { min: 60, max: 86_400 },
        shipped: Shipped::Seconds(1_800),
        applies: Applies::Live,
        row: None,
        env: None,
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
    },
    // Down from an 1800s nothing had measured. Spike 009: nine steps in ten
    // finished inside 500s and the longest honest one took 1777s, but the floor
    // is 1337s — one Check at the Check budget plus a p90 step's own work,
    // because a step's clock runs through Fleet's Checks and does not restart on
    // a retry. What none of the three step wires catches is what stopped every
    // stuck step measured: they were quiet, not long, which is the silence
    // threshold's to catch.
    Entry {
        key: STEP_WALL_CLOCK_SECONDS.0,
        section: Section::Timeouts,
        title: "Step wall clock",
        description: "How long a step may run before the Judge is asked whether its Drone is going in circles. A step's own value wins.",
        kind: Kind::Seconds { min: 60, max: 86_400 },
        shipped: Shipped::Seconds(1_500),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    // The shortest of the three deliberately. Spike 4 measured an injected turn
    // consumed in 1.59s mid-task and 33s against a forty-second command, so two
    // minutes is a Drone that is not answering rather than one inside a call.
    Entry {
        key: STEP_GRACE_SECONDS.0,
        section: Section::Timeouts,
        title: "Grace to report",
        description: "How long a Drone told to report has before its step is looked at.",
        kind: Kind::Seconds { min: 10, max: 3_600 },
        shipped: Shipped::Seconds(120),
        applies: Applies::Live,
        row: None,
        env: None,
    },
    // **Two minutes, the bottom of the band spike 009 leaves open.** Inside an
    // honest step the longest silence was 79s, so none of 31 honest steps would
    // have been poked. Three of the eight stuck ones were quieter than that —
    // 147s, 409s, 1636s — and only the bottom of the band catches the first.
    // Firing early costs one injected turn; firing late cost 27 minutes of a
    // person watching a step that had already stopped. A step's own
    // `quiet_after_seconds` wins, `fleet::Liveness::at`.
    Entry {
        key: DRONE_QUIET_AFTER_SECONDS.0,
        section: Section::Timeouts,
        title: "Drone silence",
        description: "How long a Drone may say nothing before Fleet asks whether it is still there. A step's own value wins.",
        kind: Kind::Seconds { min: 10, max: 3_600 },
        shipped: Shipped::Seconds(120),
        applies: Applies::Live,
        row: Some("drone-silence-threshold-quiet-after"),
        env: None,
    },
    // **Two pokes.** What must not fire routinely is the escalation rather than
    // the poke, and it needs the silence to survive both — about six minutes,
    // four and a half times the longest silence any honest step produced.
    Entry {
        key: DRONE_POKE_LIMIT.0,
        section: Section::Timeouts,
        title: "Times a silent Drone is asked",
        description: "How many times Fleet asks a silent Drone before the Job stops as stalled. A step's own value wins.",
        kind: Kind::Integer { min: 0, max: 10, unit: "times" },
        shipped: Shipped::Integer(2),
        applies: Applies::Live,
        row: Some("poke-limit-poke-limit"),
        env: None,
    },
    // `fleet::helm::SHIPPED_ASK_HOLD` says what the five minutes is measured
    // against; the row carries it too.
    Entry {
        key: HELM_ASK_HOLD_SECONDS.0,
        section: Section::Timeouts,
        title: "Helm ask held open",
        description: "How long one call a Helm session made waits for a person in the dock before Fleet refuses it for them.",
        kind: Kind::Seconds { min: 30, max: 3_600 },
        shipped: Shipped::Seconds(300),
        applies: Applies::AtRestart,
        row: Some("helm-ask-hold"),
        env: None,
    },
    Entry {
        key: SESSION_QUIET_SECONDS.0,
        section: Section::Timeouts,
        title: "Session left idle",
        description: "How long a hosted session's process may sit with no turn before Fleet ends it. The next message resumes it.",
        kind: Kind::Seconds { min: 60, max: 86_400 },
        shipped: Shipped::Seconds(600),
        applies: Applies::AtRestart,
        row: Some("session-quiet-timeout"),
        env: None,
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
    },
    // Five minutes: a `git status` per worktree still held, and what filled a
    // disk was seventy-four worktrees over days, not five minutes of one.
    Entry {
        key: RECLAIM_SWEEP_SECONDS.0,
        section: Section::Timeouts,
        title: "Disk reclaim",
        description: "How often Fleet looks for worktrees whose disk it may give back.",
        kind: Kind::Seconds { min: 30, max: 86_400 },
        shipped: Shipped::Seconds(300),
        applies: Applies::Live,
        row: None,
        env: None,
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
    },
    Entry {
        key: HELM_SESSION_DAYS.0,
        section: Section::Retention,
        title: "Closed Helm sessions",
        description: "How long a closed Helm session is kept so it can be resumed.",
        kind: Kind::Integer {
            min: 1,
            max: 365,
            unit: "days",
        },
        shipped: Shipped::Integer(30),
        applies: Applies::Live,
        row: Some("helm-session-retention-expiry"),
        env: None,
    },
];
