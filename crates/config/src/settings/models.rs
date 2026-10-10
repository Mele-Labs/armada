//! Which model each caller runs on, and how hard it thinks. Every default is
//! the adapter's, because a model's spelling is a vendor's; a Job's or a
//! task's own choice in Bridge wins over these.

use super::{Applies, Entry, Kind, List, Options, Section, Shipped, Words};

pub const MODELS_ROSTER: List = List("models.roster");
pub const MODELS_DEFAULT: Words = Words("models.default");
pub const MODELS_JUDGE: Words = Words("models.judge");
pub const MODELS_PROPOSER: Words = Words("models.proposer");
pub const MODELS_RETRO: Words = Words("models.retro");
pub const MODELS_SECOND_OPINION: Words = Words("models.secondOpinion");

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
    // roster and leads it rather than replacing it.
    Entry {
        key: MODELS_ROSTER.0,
        section: Section::Model,
        title: "Models",
        description: "The models a Job may name and every other model setting chooses from.",
        kind: Kind::TextList,
        shipped: Shipped::Supplied,
        applies: Applies::AtRestart,
        row: Some("kit-level-allowed-default-models-list"),
        env: None,
    },
    Entry {
        key: MODELS_DEFAULT.0,
        section: Section::Model,
        title: "Default model",
        description: "The model a Job's Drones run on when the Job names none.",
        kind: Kind::Choice(Options::Models),
        shipped: Shipped::Supplied,
        applies: Applies::AtRestart,
        row: Some("default-model-per-job-type"),
        env: Some("ARMADA_MODEL"),
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
    },
];
