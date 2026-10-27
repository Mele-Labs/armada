//! What an agent sets a watch up with before it starts one: a try that keeps nothing, the recipes
//! there are, and a recipe kept on this machine. `docs/concepts/watch.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::watches::{LoginShell, WatchReading};
use crate::SessionId;

/// What a try runs, as a start names it: a recipe with its values, or the agent's own command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchTries {
    Recipe { recipe: String, values: BTreeMap<String, String> },
    Command { command: Vec<String> },
}

/// `try_watch`. `POST /watches/try`. Runs once and keeps nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TryAsWritten", into = "TryAsWritten")]
pub struct TryWatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub tries: WatchTries,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TryAsWritten {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recipe: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<Vec<String>>,
}

impl TryFrom<TryAsWritten> for TryWatch {
    type Error = &'static str;

    fn try_from(written: TryAsWritten) -> Result<TryWatch, &'static str> {
        let tries = match (written.recipe, written.command) {
            (Some(recipe), None) => WatchTries::Recipe { recipe, values: written.values },
            (None, Some(_)) if !written.values.is_empty() => {
                return Err("`values` fill a recipe's command: give your own command whole")
            }
            (None, Some(command)) => WatchTries::Command { command },
            (Some(_), Some(_)) => return Err("name a `recipe` or give a `command`, not both"),
            (None, None) => return Err("name a `recipe`, or give a `command`"),
        };
        Ok(TryWatch { session_id: written.session_id, tries })
    }
}

impl From<TryWatch> for TryAsWritten {
    fn from(tried: TryWatch) -> TryAsWritten {
        let (recipe, values, command) = match tried.tries {
            WatchTries::Recipe { recipe, values } => (Some(recipe), values, None),
            WatchTries::Command { command } => (None, BTreeMap::new(), Some(command)),
        };
        TryAsWritten { session_id: tried.session_id, recipe, values, command }
    }
}

/// What `try_watch` read, and where and how its command ran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchTried {
    /// What the line said, or why it gave none with the end of what it printed.
    pub reading: WatchReading,
    /// Whole, every value filled.
    pub command: Vec<String>,
    /// The folder it ran in: the Job's slot.
    pub cwd: String,
    /// The `PATH` it was given.
    pub path: String,
    pub login_shell: LoginShell,
    /// This Job's own scratch folder, which the slot guard lets it write and run scripts in.
    pub scratch: String,
}

/// Where a recipe was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchRecipeSource {
    /// Shipped with Armada.
    Armada,
    /// The repository's `.armada/watches/`, as its base branch holds it.
    Repository,
    /// This Mac's `~/Library/Application Support/Armada/machine/watches/`.
    Machine,
}

/// One recipe a start may name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchRecipeListed {
    pub name: String,
    /// The values a start gives, in the order its command names them.
    pub values: Vec<String>,
    /// Its command as written, each `{value}` unfilled.
    pub command: Vec<String>,
    pub source: WatchRecipeSource,
    pub every_seconds: u32,
    /// Absent gives up after the machine's `timeouts.watchGiveUpMinutes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gives_up_after_seconds: Option<u32>,
    /// The Requests environment its command is given. A name, never a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

/// `list_watch_recipes`. `GET /watches/recipes?session_id=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchRecipeList {
    /// One per name, the most specific copy, in name order.
    pub recipes: Vec<WatchRecipeListed>,
    /// Each file set aside, and why.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_out: Vec<String>,
}

/// `save_watch_recipe`. `POST /watches/recipes/save`. Writes into this Mac's own recipes only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveWatchRecipe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub name: String,
    /// A script to keep beside the recipe, by its path now. It is copied, and run first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// The whole command, or with `script` the arguments after it. An item written `{sha}` is a
    /// value a start fills.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// A span, `60s`, `2m` or `2h`.
    pub every: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gives_up_after: Option<String>,
    /// A Requests environment whose variables the command is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Write over a recipe of this name in the repository or on this Mac.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace: bool,
}

/// What `save_watch_recipe` kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchRecipeSaved {
    pub recipe: WatchRecipeListed,
    /// The recipe's file.
    pub file: String,
    /// The script's copy, where one was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// It wrote over a recipe of the same name.
    pub replaced: bool,
}
