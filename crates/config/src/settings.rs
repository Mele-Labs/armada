//! The Machine's settings: every key `settings.json` may hold, what kind of
//! value each takes, what it ships at, and when a change reaches Fleet.
//!
//! **One table, and it is the only list.** [`entries`] is what the file is
//! checked against, what the wire describes to Bridge and what the composition
//! root reads a value from. `crates/config/settings.toml` stays the design
//! registry, and an entry names the row it realises.
//!
//! **A file that breaks one rule is refused whole**, `armada.yml`'s rule, and
//! the last good settings stay in force. Precedence is an environment variable,
//! then the file, then what ships; a default only known at start is
//! [`Shipped::Supplied`]. A key is a typed handle only this crate can make, so
//! a caller asking for a number gets one — `docs/practices/rust.md` section 2.

mod bridge;
mod check;
mod limits;
mod machine;
mod models;
mod prompts;
mod prompts_drone;
mod prompts_judge;
mod resolve;
mod timeouts;

use std::time::Duration;

use store::settings_file::SettingValue;

pub use bridge::*;
pub use check::{checked, Refused, Saved};
pub use limits::*;
pub use machine::*;
pub use models::*;
pub use prompts::*;
pub use prompts_drone::*;
pub use prompts_judge::*;
pub use resolve::{Env, Resolved, Supplied};
pub use timeouts::*;

/// Where a setting is drawn on Bridge's Settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Limits,
    Timeouts,
    Retention,
    Harness,
    Model,
    Effort,
    /// A prompt, under the sub-heading of its family.
    Prompt(PromptSection),
    Features,
    Editor,
    Terminal,
    Bridge,
}

/// The group a section sits in on Bridge's Settings page, so the index on the
/// left stays short. A section's group is a function of the section, so the
/// two cannot disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Agents,
    Prompts,
    Fleet,
    Features,
    Tools,
    Appearance,
}

impl Group {
    /// The heading Bridge draws the group under, in the order the index lists them.
    pub fn title(self) -> &'static str {
        match self {
            Group::Agents => "Agents",
            Group::Prompts => "Prompts",
            Group::Fleet => "Fleet",
            Group::Features => "Features",
            Group::Tools => "Tools",
            Group::Appearance => "Appearance",
        }
    }
}

impl Section {
    /// The group this section is a sub-heading of.
    pub fn group(self) -> Group {
        match self {
            Section::Harness | Section::Model | Section::Effort => Group::Agents,
            Section::Prompt(_) => Group::Prompts,
            Section::Limits | Section::Timeouts | Section::Retention => Group::Fleet,
            Section::Features => Group::Features,
            Section::Editor | Section::Terminal => Group::Tools,
            Section::Bridge => Group::Appearance,
        }
    }

    /// The heading Bridge draws the section under.
    pub fn title(self) -> &'static str {
        match self {
            Section::Limits => "Limits",
            Section::Timeouts => "Timeouts",
            Section::Retention => "Retention",
            Section::Harness => "Harness",
            Section::Model => "Model",
            Section::Effort => "Effort",
            Section::Prompt(section) => section.title(),
            Section::Features => "Features",
            Section::Editor => "Editor",
            Section::Terminal => "Terminal",
            Section::Bridge => "Bridge",
        }
    }
}

/// What a setting's value is, with the bounds or choices that make a value one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A whole number from `min` to `max`, both included, counted in `unit`.
    Integer {
        min: i64,
        max: i64,
        unit: &'static str,
    },
    /// Whole seconds from `min` to `max`. Bridge reads it out in minutes or hours.
    Seconds {
        min: i64,
        max: i64,
    },
    Boolean,
    /// One of a closed set of words.
    Choice(Options),
    /// One line of text, blank allowed.
    Text,
    /// Several lines of text whose default is a prompt Fleet ships, and the
    /// placeholders Fleet fills that a saved one must keep, written `{name}`.
    Prompt(&'static [&'static str]),
    /// A list of text, at least one item long.
    TextList,
    /// A value Bridge owns and draws on a pane of its own. Any JSON is taken.
    Json,
}

/// Where a choice's words come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Options {
    /// Written in the table.
    Fixed(&'static [&'static str]),
    /// The models in force: `models.roster`, led by an `ARMADA_MODEL` naming
    /// one outside it, as the picker has always offered.
    Models,
    /// The harnesses this build runs, which only `adapters` may spell.
    Harnesses,
}

/// What a setting is when nobody has saved one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shipped {
    Integer(i64),
    Seconds(i64),
    Boolean(bool),
    Text(&'static str),
    TextList(&'static [&'static str]),
    /// No value at all: a [`Kind::Json`] Bridge has nothing for.
    Nothing,
    /// Known only when Fleet starts, and handed in through [`Supplied`].
    Supplied,
}

/// When a change reaches what it governs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applies {
    /// From the next time it is read — the next admission, the next Check.
    Live,
    /// Read once when Fleet starts. A change waits for the next start.
    AtRestart,
}

/// One setting.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    /// Dotted camelCase, as the file writes it.
    pub key: &'static str,
    pub section: Section,
    pub title: &'static str,
    /// One or two sentences, as Bridge draws them under the title.
    pub description: &'static str,
    pub kind: Kind,
    pub shipped: Shipped,
    pub applies: Applies,
    /// The `crates/config/settings.toml` row this realises, where there is one.
    pub row: Option<&'static str>,
    /// The environment variable that wins over the file, where one does.
    pub env: Option<&'static str>,
}

/// Every setting, in the order Bridge draws them: group by group.
pub fn entries() -> impl Iterator<Item = &'static Entry> {
    [machine::HARNESS, models::MODEL, models::EFFORT]
        .into_iter()
        .flatten()
        .chain(prompt_entries())
        .chain(
            [
                limits::LIMITS,
                timeouts::TIMEOUTS,
                timeouts::RETENTION,
                machine::FEATURES,
                machine::EDITOR,
                machine::TERMINAL,
                bridge::BRIDGE,
            ]
            .into_iter()
            .flatten(),
        )
}

/// Every prompt, sorted by its section's place in [`PromptSection::ORDER`]
/// whichever table holds it: Bridge draws a group's sections in the order the
/// list first names them. **A sort, not a filter per section**, so a prompt
/// can never fall out of the list for want of a section in the order.
fn prompt_entries() -> impl Iterator<Item = &'static Entry> {
    static SORTED: std::sync::LazyLock<Vec<&'static Entry>> = std::sync::LazyLock::new(|| {
        let mut all: Vec<&'static Entry> =
            [prompts::PROMPTS, prompts_drone::DRONE, prompts_judge::JUDGE]
                .into_iter()
                .flatten()
                .collect();
        all.sort_by_key(|entry| match entry.section {
            Section::Prompt(section) => section.position(),
            _ => usize::MAX,
        });
        all
    });
    SORTED.iter().copied()
}

/// The entry for `key`, or `None` where the table has none.
pub fn entry(key: &str) -> Option<&'static Entry> {
    entries().find(|entry| entry.key == key)
}

/// A handle on one setting whose value reads as `Value`. Every implementing
/// type has a private field, so a handle is one of this module's constants.
pub trait Key: Copy {
    type Value;
    fn name(self) -> &'static str;
    /// The value, from one the table's checks already passed as this kind.
    fn read(value: &SettingValue) -> Self::Value;
}

macro_rules! key {
    ($(#[$doc:meta])* $name:ident => $value:ty, |$v:ident| $read:expr) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(&'static str);

        impl Key for $name {
            type Value = $value;
            fn name(self) -> &'static str {
                self.0
            }
            fn read($v: &SettingValue) -> $value {
                $read
            }
        }
    };
}

key!(
    /// A [`Kind::Integer`].
    Integer => i64, |value| match value { SettingValue::Integer(n) => *n, _ => 0 }
);
key!(
    /// A [`Kind::Seconds`].
    Seconds => Duration, |value| match value {
        SettingValue::Integer(n) => Duration::from_secs(u64::try_from(*n).unwrap_or(0)),
        _ => Duration::ZERO,
    }
);
key!(
    /// A [`Kind::Boolean`].
    Flag => bool, |value| matches!(value, SettingValue::Bool(true))
);
key!(
    /// A [`Kind::Text`], a [`Kind::Choice`] or a [`Kind::Prompt`].
    Words => String, |value| match value { SettingValue::Text(text) => text.clone(), _ => String::new() }
);
key!(
    /// A [`Kind::TextList`].
    List => Vec<String>, |value| match value {
        SettingValue::List(items) => items
            .iter()
            .filter_map(|item| match item {
                SettingValue::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
);
key!(
    /// A [`Kind::Json`]. `None` is Bridge having nothing saved.
    Json => Option<SettingValue>, |value| match value {
        SettingValue::Null => None,
        other => Some(other.clone()),
    }
);
