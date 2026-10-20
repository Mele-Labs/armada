//! Which model each caller runs on, and how hard it thinks. Every default is
//! the adapter's, because a model's spelling is a vendor's; a Job's or a
//! task's own choice in Bridge wins over these.

use super::{Applies, Entry, Integer, Kind, List, Options, Section, Shipped, Words};

pub const MODELS_ROSTER: List = List("models.roster");
pub const MODELS_DEFAULT: Words = Words("models.default");
pub const MODELS_SESSION: Words = Words("models.session");
pub const MODELS_REVIEW: Words = Words("models.review");
pub const MODELS_JUDGE: Words = Words("models.judge");
pub const MODELS_PROPOSER: Words = Words("models.proposer");
pub const MODELS_RETRO: Words = Words("models.retro");
pub const MODELS_SECOND_OPINION: Words = Words("models.secondOpinion");
pub const MODELS_DISPUTE: Words = Words("models.dispute");

pub const PLAN_DOWNSHIFT_AT: Integer = Integer("plan.downshiftAt");
pub const PLAN_DOWNSHIFT_TO: Words = Words("plan.downshiftTo");
pub const PLAN_CONTEXT_HANDOFF: Words = Words("plan.contextHandoff");

/// What a Job whose context passes the Context rule's line is told to do: ask the owner on its card,
/// carry on in a new Job with no card, or nothing.
pub const HANDOFF_ASK: &str = "ask";
pub const HANDOFF_AUTOMATICALLY: &str = "carry on automatically";
pub const HANDOFF_NEVER: &str = "never";
pub const CONTEXT_HANDOFFS: &[&str] = &[HANDOFF_ASK, HANDOFF_AUTOMATICALLY, HANDOFF_NEVER];

pub const EFFORT_DEFAULT: Words = Words("effort.default");
pub const EFFORT_JUDGE: Words = Words("effort.judge");
pub const EFFORT_PROPOSER: Words = Words("effort.proposer");
pub const EFFORT_HELM: Words = Words("effort.helm");

/// Passing no effort at all, which is the harness picking — every call before
/// these settings existed.
pub const HARNESS_DEFAULT_EFFORT: &str = "harness default";
/// The efforts a call may be given.
pub const EFFORTS: &[&str] = &[HARNESS_DEFAULT_EFFORT, "low", "medium", "high"];

pub(super) const MODEL: &[Entry] = &[
    // An override names a model this machine has, so `ARMADA_MODEL` joins the
    // roster and leads it rather than replacing it. The names switched on: absent is every
    // model the harness offers, which is what ships once Fleet has asked it.
    Entry {
        key: MODELS_ROSTER.0,
        section: Section::Model,
        title: "Models",
        description: "The models switched on for Jobs and for every other model setting to choose from. Absent, every model the agent offers is on.",
        kind: Kind::TextList,
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("kit-level-allowed-default-models-list"),
        env: None,
        tip: None,
    },
    Entry {
        key: MODELS_DEFAULT.0,
        section: Section::Model,
        title: "Default model",
        description: "The model a Job's Drones run on when none is named.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("default-model-per-job-type"),
        env: Some("ARMADA_MODEL"),
        tip: None,
    },
    // The strongest in the roster, and its own dial so the plan's step down
    // moves Sessions without moving what Drones run on.
    Entry {
        key: MODELS_SESSION.0,
        section: Section::Model,
        title: "Job model",
        description: "The model a Job runs on when none is chosen.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("session-model"),
        env: None,
        tip: None,
    },
    // Ships as the Session model, and is its own dial so a plan step down of Sessions leaves the
    // read of a pull request where the owner put it.
    Entry {
        key: MODELS_REVIEW.0,
        section: Section::Model,
        title: "Review model",
        description: "The model an agent run about a pull request in the Reviews tab runs on.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("review-model"),
        env: None,
        tip: None,
    },
    // The cheapest in the roster: the Judge is the cheap one checking the work,
    // and a machine that raised it by raising the Drone's would pay Drone
    // prices on every criterion.
    Entry {
        key: MODELS_JUDGE.0,
        section: Section::Model,
        title: "Judge model",
        description: "The model a step naming none of its own is judged on.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("judge-model"),
        env: Some("ARMADA_JUDGE_MODEL"),
        tip: None,
    },
    // Its own dial: it fires on every dispatch rather than every criterion.
    Entry {
        key: MODELS_PROPOSER.0,
        section: Section::Model,
        title: "Job proposer model",
        description: "The model that reads a request and proposes a Job.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("job-proposer-model"),
        env: Some("ARMADA_PROPOSER_MODEL"),
        tip: None,
    },
    // The cheapest: one short read of a few replies, made for every notice nobody disputed.
    Entry {
        key: MODELS_DISPUTE.0,
        section: Section::Model,
        title: "Dispute reader model",
        description: "The model that reads a Job's replies to a Fleet notice for whether it pushed back.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("fleet-notice-dispute-model"),
        env: None,
        tip: None,
    },
    // The middle tier: a retro on the cheap one was rejected as unreadable.
    Entry {
        key: MODELS_RETRO.0,
        section: Section::Model,
        title: "Retro model",
        description: "The model a Job's retro is written on, once the Job ends.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("retro-model"),
        env: Some("ARMADA_RETRO_MODEL"),
        tip: None,
    },
    // No variable: raising the first look is no reason to move the second.
    Entry {
        key: MODELS_SECOND_OPINION.0,
        section: Section::Model,
        title: "Second opinion model",
        description: "The model a judged gaming flag is read a second time on.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
];

pub(super) const EFFORT: &[Entry] = &[
    Entry {
        key: EFFORT_DEFAULT.0,
        section: Section::Effort,
        title: "Drone effort",
        description:
            "How hard a step's Drone thinks when neither the step nor the approval sets it.",
        kind: Kind::Choice(Options::Fixed(EFFORTS)),
        shipped: Shipped::Text(HARNESS_DEFAULT_EFFORT),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: EFFORT_JUDGE.0,
        section: Section::Effort,
        title: "Judge effort",
        description: "How hard a Judge call thinks.",
        kind: Kind::Choice(Options::Fixed(EFFORTS)),
        shipped: Shipped::Text(HARNESS_DEFAULT_EFFORT),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: EFFORT_PROPOSER.0,
        section: Section::Effort,
        title: "Job proposer effort",
        description: "How hard the Job proposer thinks reading a request.",
        kind: Kind::Choice(Options::Fixed(EFFORTS)),
        shipped: Shipped::Text(HARNESS_DEFAULT_EFFORT),
        applies: Applies::Live,
        row: None,
        env: None,
        tip: None,
    },
    Entry {
        key: EFFORT_HELM.0,
        section: Section::Effort,
        title: "Helm effort",
        description: "How hard Helm thinks answering a message.",
        kind: Kind::Choice(Options::Fixed(EFFORTS)),
        shipped: Shipped::Text(HARNESS_DEFAULT_EFFORT),
        applies: Applies::AtRestart,
        row: None,
        env: None,
        tip: None,
    },
];

pub(super) const PLAN: &[Entry] = &[
    // The line is the weekly window alone: the five-hour one resets before a
    // step down could matter.
    Entry {
        key: PLAN_DOWNSHIFT_AT.0,
        section: Section::Plan,
        title: "Offer a lower default at",
        description: "How much of the weekly plan window must be used before the Job model is offered a step down until the window resets.",
        kind: Kind::Integer { min: 1, max: 100, unit: "percent" },
        shipped: Shipped::Integer(80),
        applies: Applies::Live,
        row: Some("plan-downshift-line"),
        env: None,
        tip: None,
    },
    // The middle tier, as `models.retro` ships: the step is to a model that
    // still works a Job, not to the Judge's cheap one.
    Entry {
        key: PLAN_DOWNSHIFT_TO.0,
        section: Section::Plan,
        title: "Lower default model",
        description: "The model the Job model steps down to when you accept the offer.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: Some("plan-downshift-model"),
        env: None,
        tip: None,
    },
    // No sentence: the three words say it, and the tooltip names what they are about.
    Entry {
        key: PLAN_CONTEXT_HANDOFF.0,
        section: Section::Plan,
        title: "Large context",
        description: "",
        kind: Kind::Choice(Options::Fixed(CONTEXT_HANDOFFS)),
        shipped: Shipped::Text(HANDOFF_ASK),
        applies: Applies::Live,
        row: Some("context-handoff"),
        env: None,
        tip: Some("Context limit"),
    },
];
