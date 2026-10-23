//! What a taught mod's file says: `words.toml` and `fleet.toml`. `docs/concepts/mods.md`.
//!
//! **The file and the wire are one type.** Fleet reads and writes these through serde, and a
//! teaching proposal carries the same `FleetRule` Bridge shows the person, so the shape is
//! described once. `found` and `where` are one string each in the file and on the wire, so a
//! person can read and write them by hand.

use serde::{Deserialize, Serialize};

/// How an address came to be in a Words mod.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AddressFound {
    /// The repository's Manifest names a tracker site, and the adapter built the address from it.
    Settings,
    /// A recently merged pull request's text held an address with a match in it.
    PullRequest(u64),
    /// The person wrote it, in the teaching or in answer to a question.
    Person,
}

impl From<AddressFound> for String {
    fn from(found: AddressFound) -> String {
        match found {
            AddressFound::Settings => "settings".to_string(),
            AddressFound::PullRequest(number) => format!("pull_request #{number}"),
            AddressFound::Person => "person".to_string(),
        }
    }
}

impl TryFrom<String> for AddressFound {
    type Error = String;

    fn try_from(text: String) -> Result<AddressFound, String> {
        match text.as_str() {
            "settings" => Ok(AddressFound::Settings),
            "person" => Ok(AddressFound::Person),
            other => other
                .strip_prefix("pull_request #")
                .and_then(|number| number.parse::<u64>().ok())
                .filter(|number| *number > 0)
                .map(AddressFound::PullRequest)
                .ok_or_else(|| format!("`{text}` is not `settings`, `person` or `pull_request #<number>`")),
        }
    }
}

/// Where a pattern's match opens in one repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordsAddress {
    /// A Manifest id, or `*` for every repository without an address of its own.
    pub repository: String,
    /// An http or https address with `{match}` where the matched words go.
    pub template: String,
    pub found: AddressFound,
}

/// A `words.toml`: a pattern learned once, and an address for each repository that has one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordsFile {
    /// What the pattern finds, as a person would say it: `Jira key`.
    pub name: String,
    /// A regular expression Bridge runs as a JavaScript one.
    pub pattern: String,
    /// Words the pattern matches whole.
    pub example: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub address: Vec<WordsAddress>,
}

/// What Fleet does with a notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Does {
    /// Send it, as it is sent now.
    Tell,
    /// Do not send it.
    Quiet,
    /// Send what comes in a while as one.
    Batch,
    /// Raise it as a call the person answers, and not as a line to the Session.
    Call,
}

/// Which notices a Fleet rule covers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum RuleScope {
    /// One Session, by id.
    Job(String),
    /// Every Session in a repository, by Manifest id.
    Repo(String),
    Everywhere,
}

impl From<RuleScope> for String {
    fn from(scope: RuleScope) -> String {
        match scope {
            RuleScope::Job(id) => format!("job {id}"),
            RuleScope::Repo(id) => format!("repo {id}"),
            RuleScope::Everywhere => "everywhere".to_string(),
        }
    }
}

impl TryFrom<String> for RuleScope {
    type Error = String;

    fn try_from(text: String) -> Result<RuleScope, String> {
        let plain = |id: &str| {
            !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
        };
        let wrong = || format!("`{text}` is not `job <id>`, `repo <manifest>` or `everywhere`");
        match text.as_str() {
            "everywhere" => Ok(RuleScope::Everywhere),
            other => match other.split_once(' ') {
                Some(("job", id)) if plain(id) => Ok(RuleScope::Job(id.to_string())),
                Some(("repo", id)) if plain(id) => Ok(RuleScope::Repo(id.to_string())),
                _ => Err(wrong()),
            },
        }
    }
}

/// One notice kind, what Fleet does with it, and where.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetRule {
    /// A notice's word: what a thread row's `notice` carries.
    pub notice: String,
    pub does: Does,
    #[serde(rename = "where")]
    pub scope: RuleScope,
}

/// A `fleet.toml`: `[[rule]]` tables.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetFile {
    #[serde(default)]
    pub rule: Vec<FleetRule>,
}
