//! The rules Fleet watches each Job's token usage against, each an on-off and a limit. A Job that
//! crosses one is told once (`docs/concepts/session.md`, *Where the tokens went*). The figures other
//! than these, such as the hour a rule looks back over, are `fleet::usage_rules`' own.

use super::{Applies, Entry, Flag, Integer, Kind, Section, Shipped};

pub const CONTEXT_WATCH: Flag = Flag("limits.contextWatch");
pub const CONTEXT_WATCH_KTOKENS: Integer = Integer("limits.contextWatchKtokens");
pub const DISPUTE_READ_MINUTES: Integer = Integer("limits.disputeReadMinutes");

pub(super) const WATCHING: &[Entry] = &[
    Entry {
        key: CONTEXT_WATCH.0,
        section: Section::Limits,
        title: "Tell a Job its context is large",
        description: "Whether a Job is told once when the context of its last turn passes the limit below.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("usage-rule-context"),
        env: None,
        tip: None,
    },
    Entry {
        key: CONTEXT_WATCH_KTOKENS.0,
        section: Section::Limits,
        title: "Context to tell a Job at",
        description: "Thousands of tokens in the last turn's context.",
        kind: Kind::Integer { min: 50, max: 1_000, unit: "thousand tokens" },
        shipped: Shipped::Integer(300),
        applies: Applies::Live,
        row: Some("usage-rule-context"),
        env: None,
        tip: None,
    },
    Entry {
        key: DISPUTE_READ_MINUTES.0,
        section: Section::Limits,
        title: "Read a Job's replies to a Fleet notice after",
        description: "Minutes after a Fleet notice that asks a Job something, with no dispute of it, before the Job's next replies are read for whether it pushed back.",
        kind: Kind::Integer { min: 1, max: 240, unit: "minutes" },
        shipped: Shipped::Integer(10),
        applies: Applies::Live,
        row: Some("fleet-notice-dispute-read"),
        env: None,
        tip: None,
    },
];
