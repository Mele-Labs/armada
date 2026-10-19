//! The rules Fleet watches each Job's token usage against, each an on-off and a limit. A Job that
//! crosses one is told once (`docs/concepts/session.md`, *Where the tokens went*). The figures other
//! than these, such as the hour a rule looks back over, are `fleet::usage_rules`' own.

use super::{Applies, Entry, Flag, Integer, Kind, Section, Shipped};

pub const CONTEXT_WATCH: Flag = Flag("limits.contextWatch");
pub const CONTEXT_WATCH_KTOKENS: Integer = Integer("limits.contextWatchKtokens");
pub const SUBAGENT_WATCH: Flag = Flag("limits.subagentWatch");
pub const SUBAGENT_WATCH_PERCENT: Integer = Integer("limits.subagentWatchPercent");
pub const CACHE_WATCH: Flag = Flag("limits.cacheWatch");
pub const CACHE_WATCH_PERCENT: Integer = Integer("limits.cacheWatchPercent");
pub const OPUS_READING_WATCH: Flag = Flag("limits.opusReadingWatch");
pub const OPUS_READING_WATCH_TURNS: Integer = Integer("limits.opusReadingWatchTurns");

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
        description: "Thousands of tokens in the last turn's context. Half of it counts while the Job is doing more than 20 turns an hour.",
        kind: Kind::Integer { min: 50, max: 1_000, unit: "thousand tokens" },
        shipped: Shipped::Integer(300),
        applies: Applies::Live,
        row: Some("usage-rule-context"),
        env: None,
        tip: None,
    },
    Entry {
        key: SUBAGENT_WATCH.0,
        section: Section::Limits,
        title: "Tell a Job its subagents burn most",
        description: "Whether a Job is told once when subagents ran more than the share below of its last hour.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("usage-rule-subagents"),
        env: None,
        tip: None,
    },
    Entry {
        key: SUBAGENT_WATCH_PERCENT.0,
        section: Section::Limits,
        title: "Subagent share to tell a Job at",
        description: "The share of the Job's burn in the last 60 minutes that subagents ran. Only counted once the subagents' own burn passes 5 cost units and one of them ran on opus.",
        kind: Kind::Integer { min: 1, max: 99, unit: "percent" },
        shipped: Shipped::Integer(50),
        applies: Applies::Live,
        row: Some("usage-rule-subagents"),
        env: None,
        tip: None,
    },
    Entry {
        key: CACHE_WATCH.0,
        section: Section::Limits,
        title: "Tell a Job its cache misses",
        description: "Whether a Job is told once when the share of its context read from the cache falls under the limit below.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("usage-rule-cache"),
        env: None,
        tip: None,
    },
    Entry {
        key: CACHE_WATCH_PERCENT.0,
        section: Section::Limits,
        title: "Cache hit rate to tell a Job under",
        description: "The share of context read from the cache over the Job's last 10 turns.",
        kind: Kind::Integer { min: 1, max: 99, unit: "percent" },
        shipped: Shipped::Integer(70),
        applies: Applies::Live,
        row: Some("usage-rule-cache"),
        env: None,
        tip: None,
    },
    Entry {
        key: OPUS_READING_WATCH.0,
        section: Section::Limits,
        title: "Tell a Job it only reads on the top model",
        description: "Whether a Job is told once when its last turns on the top model only read and searched.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("usage-rule-opus-reading"),
        env: None,
        tip: None,
    },
    Entry {
        key: OPUS_READING_WATCH_TURNS.0,
        section: Section::Limits,
        title: "Reading turns on the top model to tell a Job at",
        description: "How many turns in a row, on the top model, that only read and searched and made no edit.",
        kind: Kind::Integer { min: 5, max: 100, unit: "turns" },
        shipped: Shipped::Integer(20),
        applies: Applies::Live,
        row: Some("usage-rule-opus-reading"),
        env: None,
        tip: None,
    },
];
