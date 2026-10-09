//! A person's Bridge preferences, kept the way [`crate::limits`] are kept —
//! Fleet-wide, surviving a relaunch, and read back whole after every save.
//!
//! **`value` is a plain `bool` because every preference but one is a switch.**
//! `theme` is a word and rides in [`SavePreference::text`]; a third shape is a
//! decision for whoever adds it, not a generality bought here on spec.

use serde::{Deserialize, Serialize};

/// Every preference Fleet knows, and what is in force for each.
///
/// **Flat, one field per known preference** — `store::Preferences`' shape,
/// carried across the wire rather than restated as a map: a name outside this
/// struct cannot be read, which is the closed set on this side of the seam.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub where_things_are_open: bool,
    /// Whether this machine offers a pull request as a draft unless the
    /// repository, the workflow or the Job says otherwise. **Since 23.68.**
    /// Absent is false, and from a Fleet before it, which offered every one ready.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft_pull_requests: bool,
    /// The theme Bridge draws with: a built-in's id or a mod's name. Fleet
    /// holds the word and Bridge decides what it names, so one that names
    /// nothing is Bridge falling back to the default. Left out while it is
    /// [`DEFAULT_THEME`], and absent reads as that from a Fleet before it.
    #[serde(default = "dark", skip_serializing_if = "is_default_theme")]
    pub theme: String,
}

/// The theme nobody has chosen away from.
pub const DEFAULT_THEME: &str = "dark";

fn dark() -> String {
    DEFAULT_THEME.to_string()
}

fn is_default_theme(theme: &String) -> bool {
    theme == DEFAULT_THEME
}

impl Default for Preferences {
    fn default() -> Preferences {
        Preferences {
            where_things_are_open: false,
            draft_pull_requests: false,
            theme: dark(),
        }
    }
}

/// A save — `POST /preferences/save`. **One preference, not the set.**
/// `SaveLimits` sends every field it has an opinion on in one request because
/// the three limits are always read and saved together; a preference is read
/// as a whole struct but saved one at a time; leaving the rest is stated
/// rather than achieved by omission.
///
/// **`name` is a plain string, not a closed wire type.** The set it must
/// belong to is `store`'s own `CHECK`, and a name outside it is refused by
/// name — `fleet.unknown_preference` — rather than failing to decode, which is
/// what a closed wire type here would do instead and would say nothing about
/// which name was sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePreference {
    pub name: String,
    pub value: bool,
    /// The value of a preference that is a word rather than a switch, which is
    /// `theme` alone. `value` is read for every other name and this is read for
    /// none of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
