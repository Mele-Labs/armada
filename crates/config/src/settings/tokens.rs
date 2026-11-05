//! What a Job costs in tokens: when its conversation is compacted, how turn summaries are asked
//! for, and which of the person's skills and connectors a repository's Jobs start without.
//! Each is read when a Job starts, so a change reaches the Jobs started after it.

use super::{Applies, Entry, Flag, Integer, Json, Kind, Section, Shipped};

pub const COMPACTION_WINDOW: Integer = Integer("tokens.compactionWindow");
pub const SUMMARY_SHORT_CACHE: Flag = Flag("tokens.summaryShortCache");
pub const SUMMARY_THINKING: Flag = Flag("tokens.summaryThinking");
/// A trial, shipped off. `docs/concepts/session/where-the-tokens-went.md`, *A Job compacted while it waits*.
pub const COMPACT_WHILE_WAITING: Flag = Flag("tokens.compactWhileWaiting");
/// Repository id to the skills switched off there. Drawn in a skill's detail in Kit, never as a row.
pub const SKILLS_OFF: Json = Json("tokens.skillsOff");
/// Repository id to whether its Jobs start without the person's connectors. A repository with no
/// entry follows its `armada.yml`. Drawn as System, Repository.
pub const CONNECTORS_OFF: Json = Json("tokens.connectorsOff");

pub(super) const TOKENS: &[Entry] = &[
    // The CLI reads 100,000 to 1,000,000 and ignores a value outside it.
    Entry {
        key: COMPACTION_WINDOW.0,
        section: Section::TokenUse,
        title: "Compaction window",
        description: "A Job's conversation is compacted once it reaches this many tokens. Smaller saves tokens on every later request, and larger keeps more of the thread word for word.",
        kind: Kind::Integer { min: 100_000, max: 1_000_000, unit: "tokens" },
        shipped: Shipped::Integer(250_000),
        applies: Applies::Live,
        row: Some("token-compaction-window"),
        env: None,
        tip: None,
    },
    Entry {
        key: SUMMARY_SHORT_CACHE.0,
        section: Section::TokenUse,
        title: "Short cache for turn summaries",
        description: "A five-minute cache costs less to write than an hour-long one, and a turn summary's prompt is rarely read again after five minutes. Off, turn summaries keep their cache for an hour.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(true),
        applies: Applies::Live,
        row: Some("token-summary-short-cache"),
        env: None,
        tip: None,
    },
    Entry {
        key: SUMMARY_THINKING.0,
        section: Section::TokenUse,
        title: "Thinking for turn summaries",
        description: "Off, turn summaries kept their quality on 60 measured summaries and their output fell from 210 to 148 tokens. Turn it on if turn summaries start missing what a turn did.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: Some("token-summary-thinking"),
        env: None,
        tip: None,
    },
    Entry {
        key: COMPACT_WHILE_WAITING.0,
        section: Section::TokenUse,
        title: "Compact a Job waiting on you",
        description: "Fleet compacts a Job that is waiting on you shortly before its prompt cache runs out. A Job that waits past its cache starts again from its whole context, and compacting first means it starts from a summary.",
        kind: Kind::Boolean,
        shipped: Shipped::Boolean(false),
        applies: Applies::Live,
        row: Some("token-compact-while-waiting"),
        env: None,
        tip: None,
    },
    Entry {
        key: SKILLS_OFF.0,
        section: Section::TokenUse,
        title: "Skills switched off for Jobs",
        description: "Kit, Skills draws this in each skill's detail, as a switch per repository.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: Some("token-skills-off"),
        env: None,
        tip: None,
    },
    Entry {
        key: CONNECTORS_OFF.0,
        section: Section::TokenUse,
        title: "Connectors switched off",
        description: "System, Repository draws this as a switch per repository.",
        kind: Kind::Json,
        shipped: Shipped::Nothing,
        applies: Applies::Live,
        row: Some("token-connectors-off"),
        env: None,
        tip: None,
    },
];
